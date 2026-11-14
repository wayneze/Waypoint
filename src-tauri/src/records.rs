use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct CoverPosition { x: f64, y: f64 }

#[derive(Default, Serialize, Deserialize)]
struct RecordDetails {
    guide_id: Option<String>,
    poster_position: Option<CoverPosition>,
    library: Option<crate::game_library::LibraryDetails>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameRecord {
    pub id: String,
    pub title: String,
    pub category: String,
    pub status: String,
    pub progress: String,
    pub rating: f64,
    pub save_path: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub poster: Option<String>,
    pub guide_id: Option<String>,
    pub poster_position: Option<CoverPosition>,
    pub library: Option<crate::game_library::LibraryDetails>,
}

// Tauri Commands: 获取单机与Galgame档案
#[tauri::command]
pub fn get_game_records(state: State<AppState>) -> Result<Vec<GameRecord>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    read_records(&conn)
}

pub(crate) fn read_records(conn: &rusqlite::Connection) -> Result<Vec<GameRecord>, String> {
    let mut stmt = conn
        .prepare("SELECT g.id, g.title, g.category, g.status, g.progress, g.rating, g.save_path, g.notes, s.value, d.value FROM game_records g LEFT JOIN app_settings s ON s.key = 'waypoint_game_cover_' || g.id LEFT JOIN app_settings d ON d.key = 'waypoint_game_details_' || g.id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let details: Option<String> = row.get(9)?;
            let details = details.map(|value| serde_json::from_str::<RecordDetails>(&value))
                .transpose().map_err(|error| rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(error)))?
                .unwrap_or_default();
            Ok(GameRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                category: row.get(2)?,
                status: row.get(3)?,
                progress: row.get(4)?,
                rating: row.get(5)?,
                save_path: row.get(6)?,
                notes: row.get(7)?,
                poster: row.get(8)?,
                guide_id: details.guide_id,
                poster_position: details.poster_position,
                library: details.library,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r.map_err(|e| e.to_string())?);
    }
    Ok(result)
}

fn validate_cover(poster: &str) -> Result<(), String> {
    if let Some(data) = poster.strip_prefix("data:image/jpeg;base64,") {
        if poster.len() <= 2_000_000 && data.starts_with("/9j/")
            && data.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        { return Ok(()); }
        return Err("封面图片无效或超过大小限制，请重新导入".into());
    }
    let url = reqwest::Url::parse(poster).map_err(|_| "封面需为有效的图片地址或导入的图片")?;
    if !matches!(url.scheme(), "http" | "https") || !url.username().is_empty() || url.password().is_some() {
        return Err("封面需为有效的图片地址或导入的图片".into());
    }
    Ok(())
}

fn save_record(conn: &mut rusqlite::Connection, record: &GameRecord, creating: bool) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    write_record(&tx, record, creating)?;
    tx.commit().map_err(|e| e.to_string())
}

/// Called inside the caller's transaction, including selective Steam imports.
pub(crate) fn write_record(conn: &rusqlite::Connection, record: &GameRecord, creating: bool) -> Result<(), String> {
    if record.title.trim().is_empty() || !record.rating.is_finite() || !(0.0..=10.0).contains(&record.rating) {
        return Err("请填写游戏名称，评分需在 0–10 之间".into());
    }
    if !matches!(record.category.as_str(), "galgame" | "single" | "steam")
        || !matches!(record.status.as_str(), "playing" | "completed" | "backlog" | "dropped") {
        return Err("游戏类别或状态无效".into());
    }
    if let Some(poster) = record.poster.as_deref().filter(|value| !value.is_empty()) {
        validate_cover(poster)?;
    }
    if let Some(position) = record.poster_position {
        if !position.x.is_finite() || !position.y.is_finite() || !(0.0..=100.0).contains(&position.x) || !(0.0..=100.0).contains(&position.y) {
            return Err("封面位置需在 0–100 之间".into());
        }
    }
    let changed = conn.execute(
        if creating {
            "INSERT INTO game_records (id, title, category, status, progress, rating, save_path, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        } else {
            "UPDATE game_records SET title=?2, category=?3, status=?4, progress=?5, rating=?6, save_path=?7, notes=?8 WHERE id=?1"
        },
        rusqlite::params![
            record.id,
            record.title.trim(),
            record.category,
            record.status,
            record.progress,
            record.rating,
            record.save_path,
            record.notes
        ],
    ).map_err(|e| e.to_string())?;
    if changed != 1 { return Err("游戏档案已不存在，请刷新书库".into()); }
    conn.execute("INSERT OR REPLACE INTO app_settings (key,value) VALUES (?1,?2)",
        rusqlite::params![format!("waypoint_game_cover_{}", record.id), record.poster.as_deref().unwrap_or("")])
        .map_err(|e| e.to_string())?;
    let details = serde_json::to_string(&RecordDetails { guide_id: record.guide_id.clone(), poster_position: record.poster_position, library: record.library.clone() }).map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_settings (key,value) VALUES (?1,?2)",
        rusqlite::params![format!("waypoint_game_details_{}", record.id), details])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn save_game_record(record: GameRecord, creating: bool, state: State<AppState>) -> Result<(), String> {
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    save_record(&mut conn, &record, creating)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editing_preserves_identity_and_commits_cover_with_record() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE game_records(id TEXT PRIMARY KEY,title TEXT,category TEXT,status TEXT,progress TEXT,rating REAL,save_path TEXT,notes TEXT); CREATE TABLE app_settings(key TEXT PRIMARY KEY,value TEXT);").unwrap();
        let mut record = GameRecord { id: "one".into(), title: "Game".into(), category: "single".into(), status: "playing".into(), progress: "第一章".into(), rating: 0.0, save_path: None, notes: None, poster: None, guide_id: None, poster_position: None, library: None };
        save_record(&mut conn, &record, true).unwrap();
        record.progress = "第二章".into();
        record.poster = Some("data:image/jpeg;base64,/9j/2Q==".into());
        record.guide_id = Some("guide-one".into());
        record.poster_position = Some(CoverPosition { x: 25.0, y: 75.0 });
        record.library = Some(crate::game_library::LibraryDetails { installations: vec![crate::game_library::Installation {
            root: r"\\server\games".into(), directory: r"\\server\games\one".into(), executable: r"\\server\games\one\game.exe".into(), candidates: vec![], steam_id: None,
        }], collection: Some(crate::game_library::GameCollection { name: "个人合集".into(), order: Some(2) }), preferred_launch: Some("local:\\\\server\\games\\one".into()),
            curation: Some(crate::game_library::GameCuration { added_tags: vec!["个人标签".into()], hidden_tags: vec!["PC".into()],
                developers: Some(vec!["自定义公司".into()]), publishers: Some(vec![]),
                staff: Some(vec![crate::game_library::GameStaff { name: "制作人".into(), role: "剧本".into(), source_id: None }]) }),
            scaling: vec![crate::scaling::ScalingProfile { directory: r"\\server\games\one".into(),
                preset: "anime".into(), capture: "auto".into(), mode: "fullscreen".into(), auto_start: true }],
            background: Some(String::new()), background_position: Some(CoverPosition { x: 40.0, y: 65.0 }), ..Default::default() });
        save_record(&mut conn, &record, false).unwrap();
        let loaded = read_records(&conn).unwrap().remove(0);
        assert_eq!(loaded.poster, record.poster);
        assert_eq!(loaded.guide_id, record.guide_id);
        assert_eq!(loaded.poster_position.unwrap().x, 25.0);
        let library = loaded.library.unwrap();
        assert_eq!(library.installations[0].executable, r"\\server\games\one\game.exe");
        assert_eq!(library.preferred_launch.as_deref(), Some("local:\\\\server\\games\\one"));
        assert_eq!(library.background.as_deref(), Some(""));
        assert_eq!(library.background_position.unwrap().y, 65.0);
        assert_eq!(library.collection.unwrap().name, "个人合集");
        assert_eq!(library.scaling.len(), 1);
        assert_eq!(library.scaling[0].preset, "anime");
        assert!(library.scaling[0].auto_start);
        let curation = library.curation.unwrap();
        assert_eq!(curation.added_tags, vec!["个人标签"]);
        assert_eq!(curation.hidden_tags, vec!["PC"]);
        assert_eq!(curation.developers.unwrap(), vec!["自定义公司"]);
        assert!(curation.publishers.unwrap().is_empty());
        assert_eq!(curation.staff.unwrap()[0].role, "剧本");
        assert_eq!(conn.query_row("SELECT count(*) FROM game_records", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        assert_eq!(conn.query_row("SELECT progress FROM game_records WHERE id='one'", [], |r| r.get::<_, String>(0)).unwrap(), "第二章");
        assert!(validate_cover("data:image/svg+xml,<svg/>").is_err());
        assert!(validate_cover("file:///C:/cover.jpg").is_err());
        conn.execute_batch("CREATE TRIGGER fail_cover BEFORE INSERT ON app_settings BEGIN SELECT RAISE(FAIL, 'disk failure'); END;").unwrap();
        record.progress = "不能落库".into();
        assert!(save_record(&mut conn, &record, false).is_err());
        assert_eq!(conn.query_row("SELECT progress FROM game_records WHERE id='one'", [], |r| r.get::<_, String>(0)).unwrap(), "第二章");
    }
}
