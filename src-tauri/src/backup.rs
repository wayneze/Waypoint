//! Portable backups keep decrypted vault fields in memory only. The complete
//! SQLite snapshot is compressed and authenticated with a user-held password.
use std::io::{Read, Write};
use std::num::NonZeroU32;
use std::path::Path;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use ring::{aead, pbkdf2, rand::{SecureRandom, SystemRandom}};
use rusqlite::{backup::Backup, Connection, MAIN_DB};

const MAGIC: &[u8; 8] = b"WPBACK01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const HEADER_LEN: usize = MAGIC.len() + SALT_LEN + NONCE_LEN;
const ITERATIONS: u32 = 600_000;
const MAX_DATABASE_BYTES: u64 = 1024 * 1024 * 1024;

pub(crate) fn snapshot(conn: &Connection) -> Result<Connection, String> {
    let mut copy = Connection::open_in_memory().map_err(|e| e.to_string())?;
    Backup::new(conn, &mut copy)
        .and_then(|backup| backup.run_to_completion(256, std::time::Duration::from_millis(1), None))
        .map_err(|e| format!("生成数据库快照失败: {e}"))?;
    Ok(copy)
}

/// Validate both protected columns, without hydrating or dropping unknown fields.
/// All changes target an isolated snapshot, never the running database.
fn transform_vault(conn: &Connection, seal: bool) -> Result<(), String> {
    let rows = {
        let mut stmt = conn.prepare("SELECT id, notes, payload FROM vault_items").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| Ok((
            r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?, r.get::<_, Option<String>>(2)?
        ))).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    };
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (id, notes, payload) in rows {
        let convert = |value: Option<String>| -> Result<Option<String>, String> {
            value.map(|v| {
                let plain = crate::secrets::unseal(&v).map_err(|_| {
                    "备份中的账号在这台电脑上无法解密，未替换当前数据。请在原电脑生成密码加密备份，或先恢复可读的金库记录。".to_string()
                })?;
                Ok(plain)
            }).transpose()
        };
        let notes = convert(notes)?;
        let payload = convert(payload)?;
        if let Some(raw) = payload.as_deref().filter(|v| !v.trim().is_empty()) {
            let value: serde_json::Value = serde_json::from_str(raw)
                .map_err(|_| "账号内容已损坏，未替换当前数据".to_string())?;
            if !value.is_object() {
                return Err("账号内容不是有效对象，未替换当前数据".into());
            }
            if let Some(characters) = value.get("characters") {
                serde_json::from_value::<Vec<crate::vault::VaultCharacter>>(characters.clone())
                    .map_err(|_| "角色名册已损坏，未替换当前数据".to_string())?;
            }
            if let Some(assets) = value.get("assets") {
                serde_json::from_value::<Vec<crate::vault::VaultAsset>>(assets.clone())
                    .map_err(|_| "账号资产已损坏，未替换当前数据".to_string())?;
            }
        }
        let notes = if seal { notes.as_deref().map(crate::secrets::seal).transpose()? } else { notes };
        let payload = if seal { payload.as_deref().map(crate::secrets::seal).transpose()? } else { payload };
        tx.execute("UPDATE vault_items SET notes=?1, payload=?2 WHERE id=?3",
            rusqlite::params![notes, payload, id]).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())
}

pub(crate) fn validate_and_seal_vault(conn: &Connection) -> Result<(), String> {
    transform_vault(conn, true)
}

fn key(password: &str, salt: &[u8]) -> Result<aead::LessSafeKey, String> {
    let mut bytes = [0u8; 32];
    pbkdf2::derive(pbkdf2::PBKDF2_HMAC_SHA256, NonZeroU32::new(ITERATIONS).unwrap(),
        salt, password.as_bytes(), &mut bytes);
    let key = aead::UnboundKey::new(&aead::AES_256_GCM, &bytes);
    bytes.fill(0);
    key.map(aead::LessSafeKey::new).map_err(|_| "无法初始化备份加密".into())
}

fn image_entry(path: &str) -> bool {
    let Some((dir, file)) = path.split_once('/') else { return false };
    let Some((stem, ext)) = file.split_once('.') else { return false };
    matches!(dir, "poster-cache" | "wiki-cache") && stem.len() == 16
        && stem.bytes().all(|b| b.is_ascii_hexdigit())
        && matches!(ext, "jpg" | "png" | "webp" | "gif")
}

pub(crate) fn encode(conn: &Connection, password: &str, data_dir: &Path) -> Result<Vec<u8>, String> {
    if password.chars().count() < 10 {
        return Err("请设置至少 10 个字符的备份密码，并在换机时使用同一密码".into());
    }
    let mut copy = snapshot(conn)?;
    copy.pragma_update(None, "secure_delete", true).map_err(|e| e.to_string())?;
    transform_vault(&copy, false)?;
    // Machine credentials have always lived outside the database. Do not export
    // historical plaintext values or imply that their local markers are portable.
    copy.execute("DELETE FROM app_settings WHERE key IN ('webdav_password','steam_web_api_key')", [])
        .map_err(|e| e.to_string())?;
    copy.execute_batch("VACUUM").map_err(|e| e.to_string())?;
    let data = copy.serialize(MAIN_DB).map_err(|e| e.to_string())?;
    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o600);
    header.set_cksum();
    archive.append_data(&mut header, "waypoint.db", &data[..]).map_err(|e| e.to_string())?;
    let mut size = data.len() as u64;
    for folder in ["poster-cache", "wiki-cache"] {
        let dir = data_dir.join(folder);
        if !dir.exists() { continue; }
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() { continue; }
            let name = format!("{folder}/{}", entry.file_name().to_string_lossy());
            if !image_entry(&name) { continue; }
            size += entry.metadata().map_err(|e| e.to_string())?.len();
            if size > MAX_DATABASE_BYTES / 2 {
                return Err("数据库与图片超过 512 MB，请先清理可重新下载的图片缓存".into());
            }
            archive.append_path_with_name(entry.path(), name).map_err(|e| e.to_string())?;
        }
    }
    seal_archive(archive.into_inner().and_then(|encoder| encoder.finish()).map_err(|e| e.to_string())?, password, MAGIC)
}

pub(crate) fn seal_archive(mut encrypted: Vec<u8>, password: &str, magic: &[u8; 8]) -> Result<Vec<u8>, String> {
    if password.chars().count() < 10 { return Err("备份密码至少需要 10 个字符".into()); }
    let mut header = [0u8; HEADER_LEN];
    header[..magic.len()].copy_from_slice(magic);
    SystemRandom::new().fill(&mut header[magic.len()..]).map_err(|_| "生成备份随机数失败")?;
    let salt = &header[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce = aead::Nonce::assume_unique_for_key(header[HEADER_LEN - NONCE_LEN..].try_into().unwrap());
    key(password, salt)?.seal_in_place_append_tag(nonce, aead::Aad::from(&header), &mut encrypted)
        .map_err(|_| "备份加密失败")?;
    Ok([header.as_slice(), encrypted.as_slice()].concat())
}

pub(crate) fn open_archive(bytes: &[u8], password: &str, magic: &[u8; 8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(magic) || bytes.len() < HEADER_LEN + aead::AES_256_GCM.tag_len() { return Err("备份格式不正确或文件不完整".into()); }
    if password.is_empty() { return Err("请输入创建备份时使用的密码".into()); }
    let header = &bytes[..HEADER_LEN];
    let mut encrypted = bytes[HEADER_LEN..].to_vec();
    let salt = &header[MAGIC.len()..MAGIC.len() + SALT_LEN];
    let nonce = aead::Nonce::assume_unique_for_key(header[HEADER_LEN - NONCE_LEN..].try_into().unwrap());
    let data = key(password, salt)?.open_in_place(nonce, aead::Aad::from(header), &mut encrypted)
        .map_err(|_| "备份密码不正确或文件已损坏；当前数据未修改".to_string())?;
    Ok(data.to_vec())
}

fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    GzDecoder::new(bytes).take(MAX_DATABASE_BYTES + 1).read_to_end(&mut out)
        .map_err(|e| format!("备份解压失败: {e}"))?;
    if out.len() as u64 > MAX_DATABASE_BYTES {
        return Err("备份解压后超过 1 GB，无法恢复".into());
    }
    Ok(out)
}

/// Returns SQLite bytes. Legacy .db and .db.gz are retained for existing backups;
/// their vault must still be readable before a restore can replace live data.
pub(crate) struct Contents {
    pub database: Vec<u8>,
    pub images: Vec<(String, Vec<u8>)>,
}

fn unpack(bytes: &[u8]) -> Result<Contents, String> {
    let mut database = None;
    let mut images = Vec::new();
    let mut names = std::collections::HashSet::new();
    let mut archive = tar::Archive::new(bytes);
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let name = entry.path().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
        if !entry.header().entry_type().is_file() || !names.insert(name.clone())
            || (name != "waypoint.db" && !image_entry(&name)) {
            return Err("备份包含不支持的文件路径或重复条目".into());
        }
        let mut content = Vec::new();
        entry.read_to_end(&mut content).map_err(|e| e.to_string())?;
        if name == "waypoint.db" { database = Some(content); } else { images.push((name, content)); }
    }
    Ok(Contents { database: database.ok_or("备份中缺少数据库")?, images })
}

pub(crate) fn decode(bytes: &[u8], password: &str) -> Result<Contents, String> {
    if bytes.starts_with(MAGIC) {
        return unpack(&gunzip(&open_archive(bytes, password, MAGIC)?)?);
    }
    if bytes.starts_with(b"SQLite format 3\0") {
        return Ok(Contents { database: bytes.to_vec(), images: vec![] });
    }
    if bytes.starts_with(&[0x1f, 0x8b]) {
        return Ok(Contents { database: gunzip(bytes)?, images: vec![] });
    }
    Err("不是支持的 Waypoint 备份文件".into())
}

/// Cache images are content-addressed. Restore missing files without replacing
/// images already available on this device.
pub(crate) fn restore_images(data_dir: &Path, images: &[(String, Vec<u8>)]) -> Result<(), String> {
    for (name, bytes) in images {
        if !image_entry(name) { return Err("图片路径不合法".into()); }
        let dest = data_dir.join(name);
        let parent = dest.parent().ok_or("图片路径缺少目录")?;
        if std::fs::symlink_metadata(parent).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("图片缓存目录不能是符号链接".into());
        }
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        if dest.exists() { continue; }
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        file.write_all(bytes).and_then(|_| file.as_file().sync_all()).map_err(|e| e.to_string())?;
        file.persist_noclobber(dest).map_err(|e| format!("恢复图片失败: {e}"))?;
    }
    Ok(())
}

pub(crate) fn open_bytes(bytes: &[u8]) -> Result<Connection, String> {
    if bytes.len() < 100 || !bytes.starts_with(b"SQLite format 3\0") || bytes.len() as u64 > MAX_DATABASE_BYTES {
        return Err("备份中没有有效数据库".into());
    }
    // sqlite3_deserialize requires rollback-journal format. SQLite documents
    // setting header bytes 18/19 to 1 for a complete snapshot of a WAL database.
    let mut image = bytes.to_vec();
    image[18] = 1;
    image[19] = 1;
    let mut conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
    conn.deserialize_read_exact(MAIN_DB, image.as_slice(), image.len(), false).map_err(|e| e.to_string())?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_backup_contains_roster_and_progress_and_authenticates_everything() {
        let conn = Connection::open_in_memory().unwrap();
        crate::init_database(&conn).unwrap();
        let payload = r#"{"characters":[{"id":"hero","name":"测试角色","assets":[]}],"future_field":true}"#;
        conn.execute("INSERT INTO vault_items (id,platform,account_name,notes,payload) VALUES ('a','sky','test',?1,?2)",
            rusqlite::params![crate::secrets::seal("note").unwrap(), crate::secrets::seal(payload).unwrap()]).unwrap();
        // Every page persists through these tables; preserve arbitrary scopes
        // and unknown settings rather than maintaining a backup field whitelist.
        conn.execute_batch(
            "INSERT INTO tasks (id,character_id,title,category,reset_rule,source_type,done,catalog_kind,catalog_id,schedule_time)
                VALUES ('t','hero','副本','weekly','weekly','manual',1,'dungeon','1','20:00');
             INSERT INTO game_records (id,title,category,status,progress,rating,save_path,notes)
                VALUES ('game','示例游戏','rpg','playing','chapter 2',4.5,'D:/Saves','guide');
             INSERT INTO custom_memos (id,title,tag,cycle_type,note,created_at,character_id,pinned)
                VALUES ('memo','记事','test','none','正文','2026-01-01','hero',1);
             INSERT INTO steam_library (vault_item_id,appid,name,playtime_forever)
                VALUES ('a',1,'示例游戏',123);
             INSERT INTO daily_checks (scope,day,item_id,checked_at) VALUES ('sky.daily','2026-01-01','task','2026-01-01 12:00:00');
             INSERT INTO app_settings (key,value) VALUES ('future_setting','{\"unknown\":true}');"
        ).unwrap();
        for scope in ["sky.atlas.lights", "sky.collection", "jx3.achievements", "jx3.reputations", "jx3.adventures", "jx3.collect"] {
            conn.execute("INSERT INTO progress_items (scope,character_id,item_id,state,updated_at) VALUES (?1,'hero','item','{\"value\":true}',1)", [scope]).unwrap();
        }
        conn.execute("INSERT INTO app_settings (key,value) VALUES ('webdav_password','old-service-secret')", []).unwrap();
        let cover = "data:image/jpeg;base64,/9j/2Q==";
        conn.execute("INSERT INTO app_settings (key,value) VALUES ('waypoint_game_cover_game',?1)", [cover]).unwrap();
        let details = r#"{"guide_id":"guide","poster_position":{"x":25,"y":75},"library":{"installations":[{"root":"D:/Games","directory":"D:/Games/One","executable":"D:/Games/One/game.exe","candidates":[]}],"metadata":{"source":"vndb","source_id":"v2002","title":"STEINS;GATE","screenshots":["https://example.org/still.jpg"]},"background":"https://example.org/still.jpg"}}"#;
        conn.execute("INSERT INTO app_settings (key,value) VALUES ('waypoint_game_details_game',?1)", [details]).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("poster-cache")).unwrap();
        std::fs::write(dir.path().join("poster-cache/0123456789abcdef.jpg"), b"test image").unwrap();
        let encrypted = encode(&conn, "a backup password", dir.path()).unwrap();
        assert!(!encrypted.windows(payload.len()).any(|s| s == payload.as_bytes()));
        let contents = decode(&encrypted, "a backup password").unwrap();
        assert_eq!(contents.images, vec![("poster-cache/0123456789abcdef.jpg".into(), b"test image".to_vec())]);
        let restored_dir = tempfile::tempdir().unwrap();
        restore_images(restored_dir.path(), &contents.images).unwrap();
        assert_eq!(std::fs::read(restored_dir.path().join(&contents.images[0].0)).unwrap(), b"test image");
        assert!(restore_images(restored_dir.path(), &[("../escape.jpg".into(), vec![])]).is_err());
        let raw = contents.database;
        assert!(!raw.windows(18).any(|s| s == b"old-service-secret"));
        let copy = open_bytes(&raw).unwrap();
        assert_eq!(copy.query_row("SELECT value FROM app_settings WHERE key='waypoint_game_cover_game'", [], |r| r.get::<_, String>(0)).unwrap(), cover);
        assert_eq!(copy.query_row("SELECT value FROM app_settings WHERE key='waypoint_game_details_game'", [], |r| r.get::<_, String>(0)).unwrap(), details);
        let portable: String = copy.query_row("SELECT payload FROM vault_items", [], |r| r.get(0)).unwrap();
        assert_eq!(portable, payload); // No source-machine DPAPI remains.
        validate_and_seal_vault(&copy).unwrap();
        let stored: String = copy.query_row("SELECT payload FROM vault_items", [], |r| r.get(0)).unwrap();
        assert!(crate::secrets::is_sealed(&stored));
        assert_eq!(crate::secrets::unseal(&stored).unwrap(), payload);
        for table in ["tasks", "game_records", "custom_memos", "steam_library", "daily_checks", "progress_items"] {
            let query = format!("SELECT * FROM {table} ORDER BY rowid");
            let rows = |db: &Connection| {
                let mut stmt = db.prepare(&query).unwrap();
                let cols = stmt.column_count();
                stmt.query_map([], |row| (0..cols).map(|i| row.get::<_, rusqlite::types::Value>(i)).collect::<rusqlite::Result<Vec<_>>>())
                    .unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()
            };
            assert_eq!(rows(&copy), rows(&conn), "{table}");
        }
        assert_eq!(copy.query_row("SELECT value FROM app_settings WHERE key='future_setting'", [], |r| r.get::<_, String>(0)).unwrap(), r#"{"unknown":true}"#);
        assert!(decode(&encrypted, "wrong password").is_err());
        let mut damaged = encrypted.clone();
        damaged[HEADER_LEN + 2] ^= 1;
        assert!(decode(&damaged, "a backup password").is_err());
        conn.execute("UPDATE vault_items SET payload='dpapi:00'", []).unwrap();
        assert!(encode(&conn, "a backup password", dir.path()).is_err());
    }

    #[test]
    fn snapshot_captures_committed_wal_with_a_reader_holding_old_pages() {
        let dir = std::env::temp_dir().join(format!("wp-wal-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("live.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE t(x); INSERT INTO t VALUES(1);").unwrap();
        let reader = Connection::open(&path).unwrap();
        reader.execute_batch("BEGIN; SELECT * FROM t;").unwrap();
        conn.execute("INSERT INTO t VALUES(2)", []).unwrap();
        let mut copy = snapshot(&conn).unwrap();
        assert_eq!(copy.query_row("SELECT sum(x) FROM t", [], |r| r.get::<_, i64>(0)).unwrap(), 3);
        let raw = copy.serialize(MAIN_DB).unwrap().to_vec();
        let reopened = open_bytes(&raw).unwrap();
        assert_eq!(reopened.query_row("SELECT sum(x) FROM t", [], |r| r.get::<_, i64>(0)).unwrap(), 3);
        drop(reader);
        drop(conn);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
