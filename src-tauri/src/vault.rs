use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VaultAsset {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qty: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct VaultCharacter {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(default)]
    pub assets: Vec<VaultAsset>,
    /// 挂到日常 · 周常。关掉后号仍留在金库。缺字段时 hydrate 再填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_daily: Option<bool>,
    /// 光遇查询ID。仅光遇角色使用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_id: Option<String>,
    /// 金库里标星的常用号：排在前面、整行高亮。
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub starred: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VaultItem {
    pub id: String,
    pub platform: String,
    pub account_name: String,
    pub server: Option<String>,
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steamid64: Option<String>,
    #[serde(default)]
    pub assets: Vec<VaultAsset>,
    #[serde(default)]
    pub characters: Vec<VaultCharacter>,
    #[serde(default, skip_serializing)]
    pub character_name: Option<String>,
    /// 密文解不开（换了电脑或系统账户）时的原因。此时条目只读，不会被写回覆盖。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<String>,
}

fn parse_identity(raw: &str) -> (String, Option<String>) {
    let text = raw.trim();
    let open = text.find('（').or_else(|| text.find('('));
    let close = text.rfind('）').or_else(|| text.rfind(')'));
    if let (Some(o), Some(c)) = (open, close) {
        if c > o {
            let name = text[..o].trim();
            let sect = text[o + text[o..].chars().next().map(|ch| ch.len_utf8()).unwrap_or(1)..c].trim();
            if !name.is_empty() {
                return (
                    name.to_string(),
                    if sect.is_empty() { None } else { Some(sect.to_string()) },
                );
            }
        }
    }
    (text.to_string(), None)
}

fn platform_has_roster(platform: &str) -> bool {
    let p = platform.to_lowercase();
    !(p.contains("steam") || p.contains("galgame") || p.contains("视觉小说"))
}

fn seal_err(e: String) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(e.into())
}

/// 写库前加密备注和 payload。
fn sealed_fields(item: &VaultItem) -> rusqlite::Result<(Option<String>, String)> {
    if let Some(why) = &item.locked {
        return Err(seal_err(format!("这条金库记录解不开，不能保存：{why}")));
    }
    let notes = match item.notes.as_deref() {
        Some(n) => Some(crate::secrets::seal(n).map_err(seal_err)?),
        None => None,
    };
    let payload = crate::secrets::seal(&vault_payload_json(item)).map_err(seal_err)?;
    Ok((notes, payload))
}

pub(crate) fn vault_payload_json(item: &VaultItem) -> String {
    serde_json::json!({
        "assets": item.assets,
        "characters": item.characters,
        "steamid64": item.steamid64,
    })
    .to_string()
}

fn insert_vault_row(conn: &rusqlite::Connection, item: &VaultItem) -> rusqlite::Result<()> {
    let (notes, payload) = sealed_fields(item)?;
    conn.execute(
        "INSERT INTO vault_items (id, platform, account_name, server, notes, payload) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![item.id, item.platform, item.account_name, item.server, notes, payload],
    )?;
    Ok(())
}

fn column_exists(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
    let mut stmt = match conn.prepare(&format!("PRAGMA table_info({})", table)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let rows = stmt.query_map([], |row| row.get::<_, String>(1));
    if let Ok(rows) = rows {
        for r in rows.flatten() {
            if r == column {
                return true;
            }
        }
    }
    false
}

pub(crate) fn hydrate_vault_item(
    id: String,
    platform: String,
    account_name: String,
    server: Option<String>,
    notes: Option<String>,
    payload: Option<String>,
    character_name: Option<String>,
) -> VaultItem {
    let mut locked = None;
    let mut open = |v: Option<String>| match v.as_deref().map(crate::secrets::unseal) {
        Some(Ok(plain)) => Some(plain),
        Some(Err(e)) => {
            locked = Some(e);
            None
        }
        None => None,
    };
    let notes = open(notes);
    let payload = open(payload);
    let mut item = VaultItem {
        id: id.clone(),
        platform,
        account_name,
        server,
        notes,
        steamid64: None,
        assets: vec![],
        characters: vec![],
        character_name: None,
        locked,
    };
    if let Some(raw) = payload.as_deref().filter(|s| !s.trim().is_empty()) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
            if let Ok(assets) = serde_json::from_value::<Vec<VaultAsset>>(v.get("assets").cloned().unwrap_or(serde_json::Value::Null)) {
                item.assets = assets;
            }
            if let Ok(mut chars) = serde_json::from_value::<Vec<VaultCharacter>>(v.get("characters").cloned().unwrap_or(serde_json::Value::Null)) {
                for ch in &mut chars {
                    if ch.server.as_deref().unwrap_or("").is_empty() {
                        ch.server = item.server.clone();
                    }
                    if ch.track_daily.is_none() {
                        ch.track_daily = Some(true);
                    }
                }
                item.characters = chars;
            }
            item.steamid64 = v
                .get("steamid64")
                .and_then(|s| s.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
        }
    }
    if item.characters.is_empty() {
        if let Some(legacy) = character_name.or(item.character_name.clone()).filter(|s| !s.trim().is_empty()) {
            if platform_has_roster(&item.platform) {
                let (name, sect) = parse_identity(&legacy);
                item.characters.push(VaultCharacter {
                    id: item.id.clone(),
                    name,
                    sect,
                    server: item.server.clone(),
                    assets: vec![],
                    track_daily: Some(true),
                    query_id: None,
                    starred: false,
                });
            }
        }
    }
    item
}

pub(crate) fn load_vault_items(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<VaultItem>> {
    let has_character_name = column_exists(conn, "vault_items", "character_name");
    if has_character_name {
        let mut stmt = conn.prepare(
            "SELECT id, platform, account_name, server, notes, payload, character_name FROM vault_items",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(hydrate_vault_item(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })?;
        rows.collect()
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, platform, account_name, server, notes, payload FROM vault_items",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(hydrate_vault_item(
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                None,
            ))
        })?;
        rows.collect()
    }
}

pub(crate) fn load_vault_item(conn: &rusqlite::Connection, item_id: &str) -> rusqlite::Result<VaultItem> {
    let has_character_name = column_exists(conn, "vault_items", "character_name");
    if has_character_name {
        conn.query_row(
            "SELECT id, platform, account_name, server, notes, payload, character_name FROM vault_items WHERE id = ?1",
            [item_id],
            |row| {
                Ok(hydrate_vault_item(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
    } else {
        conn.query_row(
            "SELECT id, platform, account_name, server, notes, payload FROM vault_items WHERE id = ?1",
            [item_id],
            |row| {
                Ok(hydrate_vault_item(
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    None,
                ))
            },
        )
    }
}

/// 启动时合并重复账号并把明文行加密。有解不开的行就整步跳过，避免覆盖密文。
pub(crate) fn migrate_vault_payload(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let items = load_vault_items(conn)?;
    if items.iter().any(|i| i.locked.is_some()) {
        return Ok(());
    }

    let mut merged: Vec<VaultItem> = Vec::new();
    for item in items {
        if let Some(existing) = merged.iter_mut().find(|m| {
            m.platform == item.platform && m.account_name == item.account_name && m.id != item.id
        }) {
            existing.characters.extend(item.characters);
            existing.assets.extend(item.assets);
            if existing.steamid64.is_none() {
                existing.steamid64 = item.steamid64;
            }
            continue;
        }
        merged.push(item);
    }

    for item in &merged {
        let (notes, payload) = sealed_fields(item)?;
        conn.execute(
            "UPDATE vault_items SET account_name = ?1, server = ?2, notes = ?3, payload = ?4 WHERE id = ?5",
            rusqlite::params![item.account_name, item.server, notes, payload, item.id],
        )?;
    }

    let keep: std::collections::HashSet<String> = merged.iter().map(|i| i.id.clone()).collect();
    let mut stmt = conn.prepare("SELECT id FROM vault_items")?;
    let ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    for id in ids {
        if !keep.contains(&id) {
            conn.execute("DELETE FROM vault_items WHERE id = ?1", [&id])?;
        }
    }
    Ok(())
}

// Tauri Commands: 获取账号金库
#[tauri::command]
pub fn get_vault_items(state: State<AppState>) -> Result<Vec<VaultItem>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    load_vault_items(&conn).map_err(|e| e.to_string())
}

// Tauri Commands: 新增账号金库项
#[tauri::command]
pub fn add_vault_item(item: VaultItem, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    insert_vault_row(&conn, &item).map_err(|e| e.to_string())?;
    Ok(())
}

// Tauri Commands: 删除账号金库项
#[tauri::command]
pub fn delete_vault_item(item_id: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM steam_library WHERE vault_item_id = ?1", [&item_id])
        .map_err(|e| e.to_string())?;
    crate::steam::remove_account_cache(&conn, &item_id)?;
    conn.execute("DELETE FROM vault_items WHERE id = ?1", [&item_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// Tauri Commands: 更新账号金库项 (全字段可编辑)
#[tauri::command]
pub fn update_vault_item(item: VaultItem, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if load_vault_item(&conn, &item.id).ok().and_then(|i| i.locked).is_some() {
        return Err("这条金库记录在本机解不开，不能覆盖。".into());
    }
    let (notes, payload) = sealed_fields(&item).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE vault_items SET platform = ?1, account_name = ?2, server = ?3, notes = ?4, payload = ?5 WHERE id = ?6",
        rusqlite::params![item.platform, item.account_name, item.server, notes, payload, item.id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// 换电脑用：把金库解密后写成明文 JSON。库里的密文绑定本机系统账户，别的机器解不开。
#[tauri::command]
pub fn export_vault_plain(dest_path: String, state: State<AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let items = load_vault_items(&conn).map_err(|e| e.to_string())?;
    if let Some(bad) = items.iter().find(|i| i.locked.is_some()) {
        return Err(format!("「{}」在本机解不开，先在原电脑导出", bad.account_name));
    }
    let body = serde_json::json!({ "kind": "waypoint-vault", "version": 1, "items": items });
    std::fs::write(&dest_path, serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?)
        .map_err(|e| format!("写出失败: {e}"))?;
    Ok(items.len())
}

/// 导入明文金库：按 id 覆盖或新增，导入后用本机账户重新加密。
#[tauri::command]
pub fn import_vault_plain(src_path: String, state: State<AppState>) -> Result<usize, String> {
    let raw = std::fs::read_to_string(&src_path).map_err(|e| format!("读取失败: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|_| "不是金库导出文件".to_string())?;
    if v.get("kind").and_then(|k| k.as_str()) != Some("waypoint-vault") {
        return Err("不是金库导出文件".into());
    }
    let items: Vec<VaultItem> = serde_json::from_value(v.get("items").cloned().unwrap_or_default())
        .map_err(|e| format!("金库文件格式不对: {e}"))?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for mut item in items.iter().cloned() {
        item.locked = None;
        tx.execute("DELETE FROM vault_items WHERE id = ?1", [&item.id]).map_err(|e| e.to_string())?;
        insert_vault_row(&tx, &item).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(items.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_payload_keeps_steamid_not_api_key() {
        let item = VaultItem {
            id: "v5".into(),
            platform: "Steam".into(),
            account_name: "example_player".into(),
            server: Some("Global".into()),
            notes: None,
            steamid64: Some("76561198000000000".into()),
            assets: vec![],
            characters: vec![],
            character_name: None,
            locked: None,
        };
        let payload = vault_payload_json(&item);
        assert!(payload.contains("76561198000000000"));
        assert!(!payload.contains("steam_web_api_key"));
        let hydrated = hydrate_vault_item(
            item.id.clone(),
            item.platform.clone(),
            item.account_name.clone(),
            item.server.clone(),
            item.notes.clone(),
            Some(payload),
            None,
        );
        assert_eq!(hydrated.steamid64.as_deref(), Some("76561198000000000"));
    }

    #[test]
    fn vault_missing_track_daily_defaults_on() {
        let payload = serde_json::json!({
            "assets": [],
            "characters": [{
                "id": "c1",
                "name": "测试角色",
                "assets": []
            }]
        })
        .to_string();
        let hydrated = hydrate_vault_item(
            "acct".into(),
            "剑网3".into(),
            "player@example.com".into(),
            None,
            None,
            Some(payload),
            None,
        );
        assert_eq!(hydrated.characters.len(), 1);
        assert_eq!(hydrated.characters[0].track_daily, Some(true));
        assert_eq!(hydrated.account_name, "player@example.com");
    }

    #[test]
    fn vault_payload_keeps_sky_query_id() {
        let item = VaultItem {
            id: "vsky".into(),
            platform: "光遇".into(),
            account_name: "sky@example.com".into(),
            server: None,
            notes: None,
            steamid64: None,
            assets: vec![],
            characters: vec![VaultCharacter {
                id: "c1".into(),
                name: "孩子".into(),
                sect: None,
                server: None,
                assets: vec![],
                track_daily: Some(true),
                query_id: Some("137106295".into()),
                starred: false,
            }],
            character_name: None,
            locked: None,
        };
        let payload = vault_payload_json(&item);
        assert!(payload.contains("137106295"));
        let hydrated = hydrate_vault_item(
            item.id.clone(),
            item.platform.clone(),
            item.account_name.clone(),
            None,
            None,
            Some(payload),
            None,
        );
        assert_eq!(hydrated.characters[0].query_id.as_deref(), Some("137106295"));
    }

    #[test]
    fn vault_rows_are_sealed_on_disk_and_locked_rows_are_never_overwritten() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::init_database(&conn).unwrap();
        conn.execute(
            "INSERT INTO vault_items (id, platform, account_name, notes, payload) VALUES ('v1', '光遇', 'acc', '密码 hunter2', '{\"characters\":[]}')",
            [],
        ).unwrap();
        // 启动迁移把旧明文加密
        migrate_vault_payload(&conn).unwrap();
        let (notes, payload): (String, String) = conn
            .query_row("SELECT notes, payload FROM vault_items WHERE id = 'v1'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert!(crate::secrets::is_sealed(&notes) && crate::secrets::is_sealed(&payload));
        assert!(!notes.contains("hunter2"));
        assert_eq!(load_vault_item(&conn, "v1").unwrap().notes.as_deref(), Some("密码 hunter2"));

        // 别的机器写的密文：解不开就标 locked，迁移不改动它
        conn.execute("UPDATE vault_items SET notes = 'dpapi:00ff' WHERE id = 'v1'", []).unwrap();
        let item = load_vault_item(&conn, "v1").unwrap();
        assert!(item.locked.is_some());
        migrate_vault_payload(&conn).unwrap();
        let notes: String = conn.query_row("SELECT notes FROM vault_items WHERE id = 'v1'", [], |r| r.get(0)).unwrap();
        assert_eq!(notes, "dpapi:00ff");
        assert!(sealed_fields(&item).is_err());
    }
}
