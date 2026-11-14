use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MingyiRoleSnapshot {
    pub uid: String,
    pub jx3_id: Option<i64>,
    pub name: String,
    pub server: String,
    pub region: String,
    pub edition: String,
    pub time: Option<i64>,
    pub time_str: Option<String>,
    pub version: Option<String>,
    pub acquired_count: usize,
    pub acquired_ids: Vec<String>,
    pub shot_path: String,
    pub info_path: String,
    pub shot_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct MingyiScanResult {
    pub data_root: String,
    pub roles: Vec<MingyiRoleSnapshot>,
    pub warnings: Vec<String>,
}

fn decode_jx3dat_text(bytes: &[u8]) -> Result<String, String> {
    if bytes.is_empty() {
        return Err("空文件".into());
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return Ok(s.to_string());
    }
    let (cow, _, had_errors) = encoding_rs::GBK.decode(bytes);
    if had_errors && !cow.contains("return") {
        let (cow18030, _, had18030) = encoding_rs::GB18030.decode(bytes);
        if !had18030 || cow18030.contains("return") {
            return Ok(cow18030.into_owned());
        }
        return Err("无法解码为 UTF-8 / GBK".into());
    }
    Ok(cow.into_owned())
}

fn lua_field_after<'a>(src: &'a str, key: &str) -> Option<&'a str> {
    let mut from = 0;
    while from < src.len() {
        let rest = &src[from..];
        let pos = rest.find(key)?;
        let abs = from + pos;
        let before_ok = abs == 0
            || src
                .as_bytes()
                .get(abs - 1)
                .map(|b| !b.is_ascii_alphanumeric() && *b != b'_')
                .unwrap_or(true);
        let after_key = abs + key.len();
        if before_ok {
            if let Some(rest) = src.get(after_key..) {
                let rest = rest.trim_start();
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(value.trim_start());
                }
            }
        }
        from = abs + key.len();
    }
    None
}

fn lua_string_field(src: &str, key: &str) -> Option<String> {
    let after = lua_field_after(src, key)?;
    if !after.starts_with('"') {
        return None;
    }
    let inner = &after[1..];
    let end = inner.find('"')?;
    Some(inner[..end].to_string())
}

fn lua_number_field(src: &str, key: &str) -> Option<i64> {
    let after = lua_field_after(src, key)?;
    let digits: String = after
        .chars()
        .take_while(|c| *c == '-' || c.is_ascii_digit())
        .collect();
    if digits.is_empty() || digits == "-" {
        return None;
    }
    digits.parse().ok()
}

type MingyiInfo = (String, Option<i64>, String, String, String, String, Option<i64>, Option<String>, Option<String>);

pub(crate) fn parse_mingyi_info_table(src: &str) -> Result<MingyiInfo, String> {
    let text = src.trim();
    if !text.contains("return") {
        return Err("info.jx3dat 不是明文 Lua 表".into());
    }
    let uid = lua_string_field(text, "uid").unwrap_or_default();
    let name = lua_string_field(text, "name").unwrap_or_default();
    if uid.is_empty() && name.is_empty() {
        return Err("info.jx3dat 缺少 uid / name".into());
    }
    Ok((
        uid,
        lua_number_field(text, "id"),
        name,
        lua_string_field(text, "server").unwrap_or_default(),
        lua_string_field(text, "region").unwrap_or_default(),
        lua_string_field(text, "edition").unwrap_or_default(),
        lua_number_field(text, "time"),
        lua_string_field(text, "time_str"),
        lua_string_field(text, "version"),
    ))
}

pub(crate) fn parse_mingyi_shot_ids(src: &str) -> Result<Vec<String>, String> {
    let text = src.trim_start_matches('\u{feff}').trim();
    if text.is_empty() {
        return Err("空内容".into());
    }
    let body = text
        .strip_prefix("return")
        .map(str::trim)
        .unwrap_or(text);
    let inner = if (body.starts_with('{') && body.ends_with('}'))
        || (body.starts_with('[') && body.ends_with(']'))
    {
        &body[1..body.len() - 1]
    } else {
        body
    };
    if inner.contains('=') {
        return Err("不是成就 ID 数组。请使用 achievement_acquire_shot.jx3dat（return {0,1,2,...}）".into());
    }
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for part in inner.split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        if !token.chars().all(|c| c.is_ascii_digit()) {
            return Err("快照含非数字内容，可能已加密或不是成就清单".into());
        }
        let id = token.parse::<u64>().map(|n| n.to_string()).map_err(|_| {
            "成就 ID 超出范围".to_string()
        })?;
        if seen.insert(id.clone()) {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn looks_like_mingyi_data_root(path: &std::path::Path) -> bool {
    path.is_dir()
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case("my#data"))
            .unwrap_or(false)
}

pub(crate) fn resolve_mingyi_data_root(hint: Option<&str>) -> Result<std::path::PathBuf, String> {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(raw) = hint.map(str::trim).filter(|s| !s.is_empty()) {
        let p = std::path::PathBuf::from(raw);
        candidates.push(p.clone());
        candidates.push(p.join("my#data"));
        candidates.push(p.join("interface").join("my#data"));
        candidates.push(p.join("bin").join("zhcn_hd").join("interface").join("my#data"));
        candidates.push(
            p.join("Game")
                .join("JX3")
                .join("bin")
                .join("zhcn_hd")
                .join("interface")
                .join("my#data"),
        );
        candidates.push(
            p.join("JX3")
                .join("bin")
                .join("zhcn_hd")
                .join("interface")
                .join("my#data"),
        );
    }
    for drive in ["E:", "D:", "C:"] {
        candidates.push(std::path::PathBuf::from(format!(
            r"{drive}\SeasunGame\Game\JX3\bin\zhcn_hd\interface\my#data"
        )));
        candidates.push(std::path::PathBuf::from(format!(
            r"{drive}\Program Files (x86)\SeasunGame\Game\JX3\bin\zhcn_hd\interface\my#data"
        )));
        candidates.push(std::path::PathBuf::from(format!(
            r"{drive}\Program Files\SeasunGame\Game\JX3\bin\zhcn_hd\interface\my#data"
        )));
    }
    for c in candidates {
        if looks_like_mingyi_data_root(&c) {
            return Ok(c);
        }
        if c.is_dir() {
            let nested = c.join("interface").join("my#data");
            if looks_like_mingyi_data_root(&nested) {
                return Ok(nested);
            }
        }
    }
    Err("未找到茗伊数据目录（interface/my#data）。请选择剑网3 安装目录或 my#data。".into())
}

pub(crate) fn is_mingyi_role_dir_name(name: &str) -> bool {
    let (uid, rest) = match name.split_once('@') {
        Some(v) => v,
        None => return false,
    };
    !uid.is_empty()
        && uid.chars().all(|c| c.is_ascii_digit())
        && !name.starts_with('!')
        && !name.starts_with('#')
        && !rest.is_empty()
}

pub(crate) fn scan_mingyi_data_root(root: &std::path::Path) -> Result<MingyiScanResult, String> {
    let mut result = MingyiScanResult {
        data_root: root.to_string_lossy().into_owned(),
        roles: Vec::new(),
        warnings: Vec::new(),
    };
    let entries = std::fs::read_dir(root).map_err(|e| format!("无法读取 {}: {}", root.display(), e))?;
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                result.warnings.push(format!("跳过目录项: {e}"));
                continue;
            }
        };
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !is_mingyi_role_dir_name(&name) {
            continue;
        }
        let info_path = path.join("info.jx3dat");
        let shot_path = path.join("userdata").join("achievement_acquire_shot.jx3dat");
        if !info_path.exists() {
            result
                .warnings
                .push(format!("{name} 没有 info.jx3dat，已跳过"));
            continue;
        }
        let info_bytes =
            std::fs::read(&info_path).map_err(|e| format!("读取 {} 失败: {e}", info_path.display()))?;
        let info_text = match decode_jx3dat_text(&info_bytes) {
            Ok(t) => t,
            Err(e) => {
                result.warnings.push(format!("{name} info.jx3dat: {e}"));
                continue;
            }
        };
        let (uid, jx3_id, role_name, server, region, edition, time, time_str, version) =
            match parse_mingyi_info_table(&info_text) {
                Ok(v) => v,
                Err(e) => {
                    result.warnings.push(format!("{name}: {e}"));
                    continue;
                }
            };
        let mut role = MingyiRoleSnapshot {
            uid: if uid.is_empty() {
                name.split('@').next().unwrap_or(&name).to_string()
            } else {
                uid
            },
            jx3_id,
            name: role_name,
            server,
            region,
            edition,
            time,
            time_str,
            version,
            acquired_count: 0,
            acquired_ids: Vec::new(),
            shot_path: shot_path.to_string_lossy().into_owned(),
            info_path: info_path.to_string_lossy().into_owned(),
            shot_error: None,
        };
        if shot_path.exists() {
            match std::fs::read(&shot_path) {
                Ok(bytes) => match decode_jx3dat_text(&bytes) {
                    Ok(text) => match parse_mingyi_shot_ids(&text) {
                        Ok(ids) => {
                            role.acquired_count = ids.len();
                            role.acquired_ids = ids;
                        }
                        Err(e) => role.shot_error = Some(e),
                    },
                    Err(e) => role.shot_error = Some(e),
                },
                Err(e) => role.shot_error = Some(format!("读取失败: {e}")),
            }
        } else {
            role.shot_error = Some("没有 achievement_acquire_shot.jx3dat（退出游戏后茗伊才会写出）".into());
        }
        result.roles.push(role);
    }
    result.roles.sort_by(|a, b| a.name.cmp(&b.name).then(a.uid.cmp(&b.uid)));
    Ok(result)
}

#[tauri::command]
pub fn scan_mingyi_roles(path: Option<String>) -> Result<MingyiScanResult, String> {
    let root = resolve_mingyi_data_root(path.as_deref())?;
    scan_mingyi_data_root(&root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mingyi_shot_parses_completed_id_array() {
        let ids = parse_mingyi_shot_ids("return {0,1,2,10,24,26,12274}").unwrap();
        assert_eq!(ids, vec!["0", "1", "2", "10", "24", "26", "12274"]);
    }

    #[test]
    fn mingyi_scan_reads_role_and_reports_missing_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        for (uid, name) in [("1001", "测试角色甲"), ("1002", "测试角色乙")] {
            let role = dir.path().join(format!("{uid}@test"));
            std::fs::create_dir_all(role.join("userdata")).unwrap();
            std::fs::write(role.join("info.jx3dat"), format!(
                "return {{ uid=\"{uid}\", id=100, name=\"{name}\", server=\"测试服\" }}"
            )).unwrap();
        }
        std::fs::write(dir.path().join("1001@test/userdata/achievement_acquire_shot.jx3dat"),
            "return {0,1,2,10,24,26,12274}").unwrap();
        std::fs::create_dir(dir.path().join("!not-a-role")).unwrap();
        let scan = scan_mingyi_data_root(dir.path()).unwrap();
        assert_eq!(scan.roles.len(), 2);
        let role = scan.roles.iter().find(|r| r.uid == "1001").unwrap();
        assert_eq!(role.uid, "1001");
        assert_eq!(role.jx3_id, Some(100));
        assert_eq!(role.name, "测试角色甲");
        assert_eq!(role.server, "测试服");
        assert_eq!(role.acquired_ids, vec!["0", "1", "2", "10", "24", "26", "12274"]);
        assert_eq!(role.acquired_count, 7);
        assert!(role.shot_error.is_none());
        assert!(scan.roles.iter().find(|r| r.uid == "1002").unwrap().shot_error.as_deref().unwrap().contains("没有 achievement"));
    }

    #[test]
    fn mingyi_shot_rejects_invented_achieve_table() {
        let err = parse_mingyi_shot_ids(
            r#"["achieve"] = { ["1001"] = true, ["3042"] = 1692800000 }"#,
        )
        .unwrap_err();
        assert!(err.contains("不是成就 ID 数组"));
    }

    #[test]
    fn mingyi_info_parses_identity_without_confusing_id_and_uid() {
        let src = r#"return {region="电信区",id=100,uid="1001",name="测试角色",server="测试服",edition="zhcn_hd",time=1785005928,time_str="20260726025848",version="1.5.0.9878"}"#;
        let (uid, jx3_id, name, server, region, edition, time, time_str, version) =
            parse_mingyi_info_table(src).unwrap();
        assert_eq!(uid, "1001");
        assert_eq!(jx3_id, Some(100));
        assert_eq!(name, "测试角色");
        assert_eq!(server, "测试服");
        assert_eq!(region, "电信区");
        assert_eq!(edition, "zhcn_hd");
        assert_eq!(time, Some(1785005928));
        assert_eq!(time_str.as_deref(), Some("20260726025848"));
        assert_eq!(version.as_deref(), Some("1.5.0.9878"));
    }

    #[test]
    fn mingyi_role_dir_name_skips_global_and_server() {
        assert!(is_mingyi_role_dir_name("1001@zhcn_hd"));
        assert!(!is_mingyi_role_dir_name("!all-users@zhcn_hd"));
        assert!(!is_mingyi_role_dir_name("#电信区_测试服@zhcn_hd"));
        assert!(!is_mingyi_role_dir_name("#cache"));
    }

}
