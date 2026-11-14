use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskItem {
    pub id: String,
    pub character_id: String,
    pub title: String,
    pub category: String, // "daily", "weekly", "custom"
    pub reset_rule: String,
    pub source_type: String, // "api_jx3", "fixed", "custom"
    pub api_key: Option<String>,
    pub tag: Option<String>,
    pub done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_time: Option<String>,
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskItem> {
    let done_int: i32 = row.get(8)?;
    Ok(TaskItem {
        id: row.get(0)?,
        character_id: row.get(1)?,
        title: row.get(2)?,
        category: row.get(3)?,
        reset_rule: row.get(4)?,
        source_type: row.get(5)?,
        api_key: row.get(6)?,
        tag: row.get(7)?,
        done: done_int != 0,
        catalog_kind: row.get(9)?,
        catalog_id: row.get(10)?,
        schedule_time: row.get(11)?,
    })
}

// Tauri Commands: 获取指定角色或所有角色的任务列表
#[tauri::command]
pub fn get_tasks(character_id: String, state: State<AppState>) -> Result<Vec<TaskItem>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut tasks = Vec::new();
    if character_id == "all" || character_id.is_empty() {
        let mut stmt = conn
            .prepare("SELECT id, character_id, title, category, reset_rule, source_type, api_key, tag, done, catalog_kind, catalog_id, schedule_time FROM tasks")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], task_from_row).map_err(|e| e.to_string())?;
        for row in rows {
            tasks.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let mut stmt = conn
            .prepare("SELECT id, character_id, title, category, reset_rule, source_type, api_key, tag, done, catalog_kind, catalog_id, schedule_time FROM tasks WHERE character_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&character_id], task_from_row)
            .map_err(|e| e.to_string())?;
        for row in rows {
            tasks.push(row.map_err(|e| e.to_string())?);
        }
    }
    Ok(tasks)
}

// Tauri Commands: 切换任务状态
#[tauri::command]
pub fn toggle_task(task_id: String, done: bool, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let done_int = if done { 1 } else { 0 };
    conn.execute(
        "UPDATE tasks SET done = ?1 WHERE id = ?2",
        rusqlite::params![done_int, task_id],
    )
    .map_err(|e| e.to_string())?;
    crate::cycle::record_task_check(&conn, &task_id, done, crate::cycle::now_local()).map_err(|e| e.to_string())?;
    Ok(())
}

// Tauri Commands: 新增任务
#[tauri::command]
pub fn add_task(task: TaskItem, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let done_int = if task.done { 1 } else { 0 };
    conn.execute(
        "INSERT INTO tasks (id, character_id, title, category, reset_rule, source_type, api_key, tag, done, catalog_kind, catalog_id, schedule_time) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            task.id,
            task.character_id,
            task.title,
            task.category,
            task.reset_rule,
            task.source_type,
            task.api_key,
            task.tag,
            done_int,
            task.catalog_kind,
            task.catalog_id,
            task.schedule_time
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// Tauri Commands: 删除任务
#[tauri::command]
pub fn delete_task(task_id: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", [&task_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_task_title(task_id: String, title: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET title = ?1 WHERE id = ?2",
        rusqlite::params![title, task_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_task_schedule_time(task_id: String, schedule_time: Option<String>, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET schedule_time = ?1 WHERE id = ?2",
        rusqlite::params![schedule_time, task_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn update_task_category(task_id: String, category: String, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET category = ?1 WHERE id = ?2",
        rusqlite::params![category, task_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}


