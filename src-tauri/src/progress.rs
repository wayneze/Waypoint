//! 收集/图鉴进度与每日打卡的正式表。前端启动时整表读入内存，之后按行增量写。
//! 以前这些数据整块 JSON 存在 app_settings 里，v2 迁移把它们拆成行。

use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::Value;
use tauri::State;

use crate::AppState;

pub(crate) fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS progress_items (
            scope TEXT NOT NULL,
            character_id TEXT NOT NULL DEFAULT '',
            item_id TEXT NOT NULL,
            state TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (scope, character_id, item_id)
        );
        CREATE TABLE IF NOT EXISTS daily_checks (
            scope TEXT NOT NULL,
            day TEXT NOT NULL,
            item_id TEXT NOT NULL,
            checked_at TEXT NOT NULL,
            PRIMARY KEY (scope, day, item_id)
        );",
    )
}

fn now_str() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ProgressOp {
    Put { scope: String, character_id: String, item_id: String, state: Value },
    Del { scope: String, character_id: String, item_id: String },
    Check { scope: String, day: String, item_id: String },
    Uncheck { scope: String, day: String, item_id: String },
}

pub(crate) fn apply_ops(conn: &Connection, ops: &[ProgressOp]) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    let now = now_str();
    for op in ops {
        match op {
            ProgressOp::Put { scope, character_id, item_id, state } => {
                tx.execute(
                    "INSERT INTO progress_items (scope, character_id, item_id, state, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(scope, character_id, item_id) DO UPDATE SET state = excluded.state, updated_at = excluded.updated_at",
                    params![scope, character_id, item_id, state.to_string(), now],
                )?;
            }
            ProgressOp::Del { scope, character_id, item_id } => {
                tx.execute(
                    "DELETE FROM progress_items WHERE scope = ?1 AND character_id = ?2 AND item_id = ?3",
                    params![scope, character_id, item_id],
                )?;
            }
            ProgressOp::Check { scope, day, item_id } => {
                tx.execute(
                    "INSERT OR IGNORE INTO daily_checks (scope, day, item_id, checked_at) VALUES (?1, ?2, ?3, ?4)",
                    params![scope, day, item_id, now],
                )?;
            }
            ProgressOp::Uncheck { scope, day, item_id } => {
                tx.execute(
                    "DELETE FROM daily_checks WHERE scope = ?1 AND day = ?2 AND item_id = ?3",
                    params![scope, day, item_id],
                )?;
            }
        }
    }
    tx.commit()
}

#[tauri::command]
pub fn apply_progress_ops(ops: Vec<ProgressOp>, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    apply_ops(&conn, &ops).map_err(|e| e.to_string())
}

/// 整表快照：items 按写入顺序（rowid）给出，保持列表原有排序。
#[tauri::command]
pub fn get_progress_snapshot(state: State<AppState>) -> Result<Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    let mut stmt = conn
        .prepare("SELECT scope, character_id, item_id, state FROM progress_items ORDER BY rowid")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (scope, ch, id, raw) = row.map_err(|e| e.to_string())?;
        let state: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        items.push(serde_json::json!([scope, ch, id, state]));
    }
    let mut checks = Vec::new();
    let mut stmt = conn
        .prepare("SELECT scope, day, item_id FROM daily_checks ORDER BY day, rowid")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (scope, day, id) = row.map_err(|e| e.to_string())?;
        checks.push(serde_json::json!([scope, day, id]));
    }
    Ok(serde_json::json!({ "items": items, "checks": checks }))
}

// ---- v2 迁移：app_settings 里的整块 JSON → 行 ----

/// 平铺的 `{ itemId: state }` 映射，全局进度。
const FLAT_SCOPES: &[(&str, &str)] = &[
    ("waypoint_jx3_tokens_v1", "jx3.tokens"),
    ("waypoint_jx3_pendants_v1", "jx3.pendants"),
    ("waypoint_sky_spirits_status_v2", "sky.spirits"),
    ("waypoint_sky_season_wish_v1", "sky.season_wish"),
    ("waypoint_sky_event_wish_v1", "sky.event_wish"),
    ("waypoint_game_custom_status", "guide.status"),
    ("waypoint_guide_progress", "guide.progress"),
    ("waypoint_completed_routes", "guide.routes"),
];

/// 按角色分组的 `{ characterId: { itemId: state } }`。
const CHAR_SCOPES: &[(&str, &str)] = &[("waypoint_jx3_char_reputations_v2", "jx3.reputation")];

/// 迁到 daily_checks 的按日键：前缀 → scope。
const DAILY_PREFIXES: &[(&str, &str)] = &[
    ("waypoint_sky_completed_", "sky.quest"),
    ("waypoint_sky_season_candles_", "sky.season_candle"),
    ("waypoint_sky_event_daily_", "sky.event_daily"),
];

/// 网络缓存和已无代码引用的旧键。缓存改放独立的 cache.db，不再进用户库和备份。
const DROP_PATTERNS: &[&str] = &[
    "waypoint_sky_news_feed_%",
    "waypoint_sky_event_catalog_%",
    "waypoint_sky_season_catalog_%",
    "waypoint_sky_calendar_%",
    "waypoint_sky_daily_%",
    "waypoint_sky_spirits_status_v1",
    "waypoint_sky_news_open",
    "waypoint_jx3_adventures_catalog_v1",
    "waypoint_jx3_adventures_fetched_at_v1",
    "waypoint_jx3_reputations_v1",
    "waypoint_jx3_reputations_catalog_v1",
    "waypoint_jx3_box_dungeon_cache_v1",
    "waypoint_jx3_exterior_quotes_%",
    "waypoint_ui_zoom",
    "waypoint_setting_%",
];

fn take_json(conn: &Connection, key: &str) -> rusqlite::Result<Option<Value>> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |r| r.get(0))
        .ok();
    conn.execute("DELETE FROM app_settings WHERE key = ?1", [key])?;
    Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
}

fn put(conn: &Connection, scope: &str, ch: &str, id: &str, state: &Value, now: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO progress_items (scope, character_id, item_id, state, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![scope, ch, id, state.to_string(), now],
    )?;
    Ok(())
}

fn put_map(conn: &Connection, scope: &str, ch: &str, map: &Value, now: &str) -> rusqlite::Result<()> {
    if let Some(obj) = map.as_object() {
        for (id, state) in obj {
            put(conn, scope, ch, id, state, now)?;
        }
    }
    Ok(())
}

fn id_text(v: &Value) -> String {
    v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
}

/// 收集清单 `{ items: [...], pinnedDungeons, collapsed }`：条目逐行，布局单独一行。
fn put_collect_store(conn: &Connection, ch: &str, store: &Value, now: &str) -> rusqlite::Result<()> {
    if let Some(items) = store.get("items").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                put(conn, "jx3.collect", ch, id, item, now)?;
            }
        }
    }
    let layout = serde_json::json!({
        "pinnedDungeons": store.get("pinnedDungeons").cloned().unwrap_or(Value::Array(vec![])),
        "collapsed": store.get("collapsed").cloned().unwrap_or(Value::Object(Default::default())),
    });
    put(conn, "jx3.collect.layout", ch, "layout", &layout, now)
}

pub(crate) fn migrate_v2(conn: &Connection) -> rusqlite::Result<()> {
    create_tables(conn)?;
    let now = now_str();

    for (key, scope) in FLAT_SCOPES {
        if let Some(v) = take_json(conn, key)? {
            put_map(conn, scope, "", &v, &now)?;
        }
    }
    for (key, scope) in CHAR_SCOPES {
        if let Some(Value::Object(by_char)) = take_json(conn, key)? {
            for (ch, map) in &by_char {
                put_map(conn, scope, ch, map, &now)?;
            }
        }
    }

    // 成就：按角色的 id 列表 → 每个已完成成就一行
    let mut has_char_achieves = false;
    if let Some(Value::Object(by_char)) = take_json(conn, "waypoint_jx3_completed_achieves_by_char_v1")? {
        for (ch, ids) in &by_char {
            for id in ids.as_array().into_iter().flatten() {
                put(conn, "jx3.achievement", ch, &id_text(id), &Value::Bool(true), &now)?;
                has_char_achieves = true;
            }
        }
    }
    // 角色化以前的全局成就列表：没有角色数据时才作为全局进度保留
    if let Some(Value::Array(ids)) = take_json(conn, "waypoint_jx3_completed_achieves_v1")? {
        if !has_char_achieves {
            for id in &ids {
                put(conn, "jx3.achievement", "", &id_text(id), &Value::Bool(true), &now)?;
            }
        }
    }

    if let Some(atlas) = take_json(conn, "waypoint_sky_atlas_v1")? {
        for kind in ["shrines", "lights"] {
            if let Some(m) = atlas.get(kind) {
                put_map(conn, &format!("sky.atlas.{kind}"), "", m, &now)?;
            }
        }
        if let Some(m) = atlas.get("actions").or_else(|| atlas.get("spirits")) {
            put_map(conn, "sky.atlas.actions", "", m, &now)?;
        }
    }

    if let Some(store) = take_json(conn, "waypoint_jx3_collect_v1")? {
        put_collect_store(conn, "", &store, &now)?;
    }
    if let Some(Value::Object(by_char)) = take_json(conn, "waypoint_jx3_char_collect_v2")? {
        for (ch, store) in &by_char {
            put_collect_store(conn, ch, store, &now)?;
        }
    }

    if let Some(Value::Array(seasons)) = take_json(conn, "waypoint_sky_season_custom_v1")? {
        for s in &seasons {
            if let Some(id) = s.get("id").and_then(|v| v.as_str()) {
                put(conn, "sky.season_custom", "", id, s, &now)?;
            }
        }
    }

    for (prefix, scope) in DAILY_PREFIXES {
        let keys: Vec<String> = {
            let mut stmt = conn.prepare("SELECT key FROM app_settings WHERE substr(key, 1, ?1) = ?2")?;
            let rows = stmt.query_map(params![prefix.len() as i64, prefix], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for key in keys {
            let day = key[prefix.len()..].to_string();
            let ids: Vec<String> = match take_json(conn, &key)? {
                Some(Value::Array(list)) => list.iter().map(id_text).collect(),
                // 节日日常存的是 `{ name }`
                Some(Value::Object(o)) => o.get("name").map(id_text).into_iter().collect(),
                _ => Vec::new(),
            };
            for id in ids {
                conn.execute(
                    "INSERT OR IGNORE INTO daily_checks (scope, day, item_id, checked_at) VALUES (?1, ?2, ?3, ?4)",
                    params![scope, day, id, now],
                )?;
            }
        }
    }

    for pat in DROP_PATTERNS {
        // `_` 在 LIKE 里是通配符，这里按字面量匹配
        let escaped = pat.replace('_', "\\_");
        conn.execute("DELETE FROM app_settings WHERE key LIKE ?1 ESCAPE '\\'", [escaped])?;
    }
    Ok(())
}

/// 统计页：最近 N 天每天每类打卡数，以及收集/图鉴进度的更新次数。只读。
#[tauri::command]
pub fn get_activity_stats(days: i64, state: State<AppState>) -> Result<Value, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    activity_stats(&conn, days, chrono::Local::now().date_naive()).map_err(|e| e.to_string())
}

pub(crate) fn activity_stats(conn: &Connection, days: i64, today: chrono::NaiveDate) -> rusqlite::Result<Value> {
    let since = (today - chrono::Duration::days(days.clamp(1, 366) - 1)).format("%Y-%m-%d").to_string();
    let mut stmt = conn.prepare(
        "SELECT day, scope, COUNT(*) FROM daily_checks WHERE day >= ?1 GROUP BY day, scope ORDER BY day",
    )?;
    let checks: Vec<Value> = stmt
        .query_map([&since], |r| Ok(serde_json::json!([r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?])))?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare(
        "SELECT substr(updated_at, 1, 10) AS d, scope, COUNT(*) FROM progress_items
         WHERE substr(updated_at, 1, 10) >= ?1 GROUP BY d, scope ORDER BY d",
    )?;
    let progress: Vec<Value> = stmt
        .query_map([&since], |r| Ok(serde_json::json!([r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?])))?
        .collect::<rusqlite::Result<_>>()?;
    let daily_tasks: i64 = conn.query_row("SELECT COUNT(*) FROM tasks WHERE category = 'daily'", [], |r| r.get(0))?;
    Ok(serde_json::json!({ "since": since, "checks": checks, "progress": progress, "daily_tasks": daily_tasks }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_stats_groups_by_day_and_scope() {
        let conn = Connection::open_in_memory().unwrap();
        crate::init_database(&conn).unwrap();
        for (day, id) in [("2026-10-28", "a"), ("2026-10-28", "b"), ("2026-10-29", "a"), ("2026-09-01", "z")] {
            conn.execute("INSERT INTO daily_checks VALUES ('task.daily', ?1, ?2, 'x')", [day, id]).unwrap();
        }
        let v = activity_stats(&conn, 7, chrono::NaiveDate::from_ymd_opt(2026, 10, 29).unwrap()).unwrap();
        assert_eq!(v["checks"], serde_json::json!([["2026-10-28", "task.daily", 2], ["2026-10-29", "task.daily", 1]]));
    }

    fn set(conn: &Connection, k: &str, v: &str) {
        conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)", [k, v]).unwrap();
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn v2_moves_kv_blobs_into_rows_and_drops_caches() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)", []).unwrap();
        set(&conn, "waypoint_jx3_tokens_v1", r#"{"s1":{"head":true,"body":false}}"#);
        set(&conn, "waypoint_jx3_char_reputations_v2", r#"{"c1":{"r1":{"currentLevel":"敬重"}}}"#);
        set(&conn, "waypoint_jx3_completed_achieves_by_char_v1", r#"{"c1":["11","12"],"c2":["13"]}"#);
        set(&conn, "waypoint_jx3_char_collect_v2", r#"{"c1":{"items":[{"id":"a","name":"x"},{"id":"b","name":"y"}],"pinnedDungeons":[],"collapsed":{}}}"#);
        set(&conn, "waypoint_sky_atlas_v1", r#"{"shrines":{},"lights":{"l1":true},"actions":{},"spirits":{}}"#);
        set(&conn, "waypoint_sky_completed_2026-10-23", r#"["q1","任务一"]"#);
        set(&conn, "waypoint_sky_season_candles_2026-10-23", "[1,3]");
        set(&conn, "waypoint_sky_event_daily_2026-10-23", r#"{"name":"恶作剧之日"}"#);
        set(&conn, "waypoint_sky_news_feed_v4", "{}");
        set(&conn, "waypoint_sky_daily_v10_2026-10-23", "{}");
        set(&conn, "waypoint_jx3_exterior_quotes_v1", "{}");
        set(&conn, "webdav_url", "https://x");
        set(&conn, "waypointXsky_daily", "keep");

        migrate_v2(&conn).unwrap();

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='jx3.tokens'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='jx3.reputation' AND character_id='c1'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='jx3.achievement'"), 3);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='jx3.collect' AND character_id='c1'"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='jx3.collect.layout'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items WHERE scope='sky.atlas.lights'"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM daily_checks WHERE scope='sky.quest' AND day='2026-10-23'"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM daily_checks WHERE scope='sky.season_candle'"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM daily_checks WHERE scope='sky.event_daily' AND item_id='恶作剧之日'"), 1);
        // 只剩真正的设置；字面量下划线不会误删相似键
        let mut stmt = conn.prepare("SELECT key FROM app_settings ORDER BY key").unwrap();
        let left: Vec<String> = stmt.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect();
        assert_eq!(left, vec!["waypointXsky_daily".to_string(), "webdav_url".to_string()]);
    }

    #[test]
    fn ops_put_delete_and_check_roundtrip() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        let ops: Vec<ProgressOp> = serde_json::from_value(serde_json::json!([
            { "op": "put", "scope": "s", "character_id": "", "item_id": "a", "state": { "x": 1 } },
            { "op": "put", "scope": "s", "character_id": "", "item_id": "b", "state": true },
            { "op": "del", "scope": "s", "character_id": "", "item_id": "a" },
            { "op": "check", "scope": "d", "day": "2026-10-29", "item_id": "q" },
            { "op": "check", "scope": "d", "day": "2026-10-29", "item_id": "q" },
        ]))
        .unwrap();
        apply_ops(&conn, &ops).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM progress_items"), 1);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM daily_checks"), 1);
    }
}
