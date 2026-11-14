//! 网络数据缓存。单独放在 `cache.db`，不进用户库，导出和 WebDAV 备份都不带它；删掉只会让下次多拉一次网络。

use std::collections::HashMap;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::State;

pub struct CacheDb(pub Mutex<Connection>);

pub(crate) fn open(path: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS fetch_cache (
            key TEXT PRIMARY KEY,
            body TEXT NOT NULL,
            fetched_at INTEGER NOT NULL
        );",
    )?;
    // 按天的缓存（光遇今日任务等）会一直累积，启动时清掉一个月前的
    let cutoff = chrono::Local::now().timestamp_millis() - 30 * 86_400_000;
    conn.execute("DELETE FROM fetch_cache WHERE fetched_at < ?1", [cutoff])?;
    Ok(conn)
}

#[derive(Serialize)]
pub struct CacheEntry {
    body: String,
    /// 毫秒时间戳，和前端 Date.now() 同口径
    fetched_at: i64,
}

/// 启动时整表读入前端内存，之后读取都是同步的。
#[tauri::command]
pub fn cache_get_all(cache: State<CacheDb>) -> Result<HashMap<String, CacheEntry>, String> {
    let conn = cache.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT key, body, fetched_at FROM fetch_cache")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, CacheEntry { body: r.get(1)?, fetched_at: r.get(2)? })))
        .map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<_>>().map_err(|e| e.to_string())
}

pub(crate) fn put(conn: &Connection, key: &str, body: &str, fetched_at: i64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO fetch_cache (key, body, fetched_at) VALUES (?1, ?2, ?3)",
        params![key, body, fetched_at],
    )
    .map(|_| ())
}

#[tauri::command]
pub fn cache_put(key: String, body: String, fetched_at: i64, cache: State<CacheDb>) -> Result<(), String> {
    let conn = cache.0.lock().map_err(|e| e.to_string())?;
    put(&conn, &key, &body, fetched_at).map_err(|e| e.to_string())
}
