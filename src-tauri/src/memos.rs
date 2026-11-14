use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomMemo {
    pub id: String,
    pub title: String,
    pub tag: String,
    pub cycle_type: String, // "once", "daily", "weekly", "interval", "deadline"
    pub cycle_val: Option<String>,
    pub note: Option<String>,
    pub has_progress: bool,
    pub current_progress: i32,
    pub target_progress: i32,
    pub done: bool,
    pub last_completed_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub character_id: Option<String>,
    #[serde(default)]
    pub pinned: bool,
}

// Tauri Commands: 获取万象记事与周期追踪列表
#[tauri::command]
pub fn get_custom_memos(state: State<AppState>) -> Result<Vec<CustomMemo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, title, tag, cycle_type, cycle_val, note, has_progress, current_progress, target_progress, done, last_completed_at, created_at, character_id, pinned FROM custom_memos ORDER BY pinned DESC, done ASC, created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let has_prog_int: i32 = row.get(6)?;
            let done_int: i32 = row.get(9)?;
            let pinned_int: i32 = row.get(13).unwrap_or(0);
            Ok(CustomMemo {
                id: row.get(0)?,
                title: row.get(1)?,
                tag: row.get(2)?,
                cycle_type: row.get(3)?,
                cycle_val: row.get(4)?,
                note: row.get(5)?,
                has_progress: has_prog_int != 0,
                current_progress: row.get(7)?,
                target_progress: row.get(8)?,
                done: done_int != 0,
                last_completed_at: row.get(10)?,
                created_at: row.get(11)?,
                character_id: row.get(12).ok(),
                pinned: pinned_int != 0,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

// Tauri Commands: 新增万象记事
#[tauri::command]
pub fn add_custom_memo(app: AppHandle, memo: CustomMemo, state: State<AppState>) -> Result<CustomMemo, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO custom_memos (id, title, tag, cycle_type, cycle_val, note, has_progress, current_progress, target_progress, done, last_completed_at, created_at, character_id, pinned)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            memo.id,
            memo.title,
            memo.tag,
            memo.cycle_type,
            memo.cycle_val,
            memo.note,
            if memo.has_progress { 1 } else { 0 },
            memo.current_progress,
            memo.target_progress,
            if memo.done { 1 } else { 0 },
            memo.last_completed_at,
            memo.created_at,
            memo.character_id,
            if memo.pinned { 1 } else { 0 },
        ],
    )
    .map_err(|e| e.to_string())?;
    let _ = app.emit("memo-added", &memo);
    Ok(memo)
}

// Tauri Commands: 更新万象记事
#[tauri::command]
pub fn update_custom_memo(memo: CustomMemo, state: State<AppState>) -> Result<CustomMemo, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE custom_memos SET title = ?1, tag = ?2, cycle_type = ?3, cycle_val = ?4, note = ?5, has_progress = ?6, current_progress = ?7, target_progress = ?8, done = ?9, last_completed_at = ?10, character_id = ?11, pinned = ?12 WHERE id = ?13",
        rusqlite::params![
            memo.title,
            memo.tag,
            memo.cycle_type,
            memo.cycle_val,
            memo.note,
            if memo.has_progress { 1 } else { 0 },
            memo.current_progress,
            memo.target_progress,
            if memo.done { 1 } else { 0 },
            memo.last_completed_at,
            memo.character_id,
            if memo.pinned { 1 } else { 0 },
            memo.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(memo)
}

// Tauri Commands: 删除万象记事
#[tauri::command]
pub fn delete_custom_memo(id: String, state: State<AppState>) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM custom_memos WHERE id = ?1", [&id])
        .map_err(|e| e.to_string())?;
    Ok(true)
}

// Tauri Commands: 切换完成状态并记录时间戳
#[tauri::command]
pub fn toggle_custom_memo(id: String, state: State<AppState>) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let done_int: i32 = conn
        .query_row("SELECT done FROM custom_memos WHERE id = ?1", [&id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let new_done = if done_int == 0 { 1 } else { 0 };
    let now_str = chrono::Local::now().to_rfc3339();
    let completed_val: Option<String> = if new_done == 1 { Some(now_str) } else { None };
    conn.execute(
        "UPDATE custom_memos SET done = ?1, last_completed_at = ?2 WHERE id = ?3",
        rusqlite::params![new_done, completed_val, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(new_done == 1)
}
