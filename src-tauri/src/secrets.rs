//! 敏感数据不再明文进库：
//! - WebDAV 密码、Steam Web API key 存 Windows 凭据管理器，库里只留一个"已配置"标记；
//! - 金库的备注和 payload 用 DPAPI 绑定当前 Windows 账户加密后再写库。
//!
//! 密文绑定本机；跨电脑使用设置页的密码加密完整备份。

/// 库里代替真实密钥的标记，表示"在凭据管理器里"。
pub(crate) const STORED_MARK: &str = "__credman__";
const TARGET_PREFIX: &str = "Waypoint/";
/// DPAPI 密文的前缀，后面是十六进制
const SEALED_PREFIX: &str = "dpapi:";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect()
}

#[cfg(windows)]
mod os {
    use windows_sys::Win32::Foundation::{LocalFree, FILETIME};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            let ok = CryptProtectData(&input, std::ptr::null(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out);
            if ok == 0 {
                return Err("DPAPI 加密失败".into());
            }
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            LocalFree(out.pbData as _);
            Ok(bytes)
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
            let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
            let ok = CryptUnprotectData(&input, std::ptr::null_mut(), std::ptr::null(), std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out);
            if ok == 0 {
                return Err("这台电脑或这个系统账户解不开金库密文（DPAPI 绑定原账户）".into());
            }
            let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            LocalFree(out.pbData as _);
            Ok(bytes)
        }
    }

    pub fn cred_write(target: &str, secret: &str) -> Result<(), String> {
        let mut name = wide(target);
        let blob = secret.as_bytes();
        let cred = CREDENTIALW {
            Flags: 0,
            Type: CRED_TYPE_GENERIC,
            TargetName: name.as_mut_ptr(),
            Comment: std::ptr::null_mut(),
            LastWritten: FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 },
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: std::ptr::null_mut(),
            UserName: std::ptr::null_mut(),
        };
        if unsafe { CredWriteW(&cred, 0) } == 0 {
            return Err("写入 Windows 凭据管理器失败".into());
        }
        Ok(())
    }

    pub fn cred_read(target: &str) -> Option<String> {
        let name = wide(target);
        let mut ptr: *mut CREDENTIALW = std::ptr::null_mut();
        unsafe {
            if CredReadW(name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut ptr) == 0 || ptr.is_null() {
                return None;
            }
            let c = &*ptr;
            let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize).to_vec();
            CredFree(ptr as _);
            String::from_utf8(bytes).ok()
        }
    }

    pub fn cred_delete(target: &str) {
        let name = wide(target);
        unsafe {
            CredDeleteW(name.as_ptr(), CRED_TYPE_GENERIC, 0);
        }
    }
}

/// 非 Windows 只用于开发编译：不加密，凭据存进程内存。
#[cfg(not(windows))]
mod os {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static CREDS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> { Ok(data.to_vec()) }
    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> { Ok(data.to_vec()) }
    pub fn cred_write(target: &str, secret: &str) -> Result<(), String> {
        CREDS.lock().unwrap().get_or_insert_with(HashMap::new).insert(target.into(), secret.into());
        Ok(())
    }
    pub fn cred_read(target: &str) -> Option<String> {
        CREDS.lock().unwrap().as_ref()?.get(target).cloned()
    }
    pub fn cred_delete(target: &str) {
        if let Some(m) = CREDS.lock().unwrap().as_mut() { m.remove(target); }
    }
}

/// 把密钥存进凭据管理器，返回写库用的标记；空值表示清除。
pub(crate) fn store_secret(name: &str, secret: &str) -> Result<String, String> {
    let target = format!("{TARGET_PREFIX}{name}");
    if secret.is_empty() {
        os::cred_delete(&target);
        return Ok(String::new());
    }
    os::cred_write(&target, secret)?;
    Ok(STORED_MARK.into())
}

/// 读密钥。库里存的是标记就去凭据管理器取；历史明文原样返回（迁移前的库）。
pub(crate) fn load_secret(name: &str, db_value: Option<String>) -> Option<String> {
    match db_value.as_deref() {
        Some(STORED_MARK) => os::cred_read(&format!("{TARGET_PREFIX}{name}")),
        Some("") | None => None,
        Some(_) => db_value,
    }
}

/// 加密写库用。空串原样保留，方便"是否填写"判断。
pub(crate) fn seal(plain: &str) -> Result<String, String> {
    if plain.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("{SEALED_PREFIX}{}", hex(&os::protect(plain.as_bytes())?)))
}

/// 解密。没有前缀的是加密以前的明文，直接返回。
pub(crate) fn unseal(stored: &str) -> Result<String, String> {
    let Some(body) = stored.strip_prefix(SEALED_PREFIX) else {
        return Ok(stored.to_string());
    };
    let bytes = unhex(body).ok_or("金库密文格式损坏")?;
    String::from_utf8(os::unprotect(&bytes)?).map_err(|_| "金库密文解出来不是文本".into())
}

#[cfg(test)]
pub(crate) fn is_sealed(stored: &str) -> bool {
    stored.starts_with(SEALED_PREFIX)
}

/// v3：把库里的明文密钥搬进凭据管理器，库里换成标记。
pub(crate) fn migrate_v3(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    for key in ["webdav_password", "steam_web_api_key"] {
        let value: Option<String> = conn
            .query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get(0))
            .ok();
        let Some(value) = value.filter(|v| !v.is_empty() && v != STORED_MARK) else { continue };
        let mark = store_secret(key, &value).map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        conn.execute("UPDATE app_settings SET value = ?1 WHERE key = ?2", [&mark, key])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_roundtrip_and_legacy_plaintext() {
        let sealed = seal("账号 密码 123").unwrap();
        assert!(is_sealed(&sealed));
        assert!(!sealed.contains("密码"));
        assert_eq!(unseal(&sealed).unwrap(), "账号 密码 123");
        assert_eq!(unseal("旧明文").unwrap(), "旧明文");
        assert_eq!(seal("").unwrap(), "");
    }

    #[test]
    fn credential_roundtrip() {
        let name = format!("test-{}", std::process::id());
        assert_eq!(store_secret(&name, "s3cret").unwrap(), STORED_MARK);
        assert_eq!(load_secret(&name, Some(STORED_MARK.into())).as_deref(), Some("s3cret"));
        assert_eq!(store_secret(&name, "").unwrap(), "");
        assert_eq!(load_secret(&name, Some(STORED_MARK.into())), None);
        assert_eq!(load_secret(&name, Some("legacy".into())).as_deref(), Some("legacy"));
    }
}
