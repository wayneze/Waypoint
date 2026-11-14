use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::io::Write;
use tauri::{AppHandle, Manager, State};
use crate::AppState;
#[cfg(test)]
use crate::init_database;

// Tauri Commands: 获取本地 SQLite 数据库物理存储状态
#[tauri::command]
pub fn get_db_info(app: AppHandle) -> Result<serde_json::Value, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let db_path = app_data_dir.join("waypoint.db");
    let exists = db_path.exists();
    let size = if exists {
        std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0)
    } else {
        0
    };
    let undo_at = std::fs::metadata(sidecar(&db_path, ".before-restore"))
        .and_then(|m| m.modified())
        .ok()
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string());
    Ok(serde_json::json!({
        "path": db_path.to_string_lossy(),
        "dir": app_data_dir.to_string_lossy(),
        "size_bytes": size,
        "exists": exists,
        "undo_restore_at": undo_at
    }))
}

pub(crate) fn db_file_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    Ok(app_data_dir.join("waypoint.db"))
}

#[tauri::command]
pub fn export_database(app: AppHandle, dest_path: String, password: String, state: State<AppState>) -> Result<String, String> {
    let dest = Path::new(&dest_path);
    if dest.canonicalize().ok() == db_file_path(&app)?.canonicalize().ok() {
        return Err("备份不能覆盖正在使用的数据库".into());
    }
    let bytes = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::backup::encode(&conn, &password, db_file_path(&app)?.parent().ok_or("数据目录不存在")?)?
    };
    let parent = dest.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(&bytes).and_then(|_| file.as_file().sync_all()).map_err(|e| e.to_string())?;
    file.persist(dest).map_err(|e| format!("保存备份失败: {e}"))?;
    Ok(dest_path)
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{}", path.to_string_lossy(), suffix))
}

fn remove_db_files(path: &Path) {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let _ = std::fs::remove_file(sidecar(path, suffix));
    }
}

/// 把 `src` 库完整拷成单文件 `dest`（不依赖它的 -wal 旁文件是否一起拷走）。
fn snapshot_db(src: &rusqlite::Connection, dest: &Path) -> Result<(), String> {
    remove_db_files(dest);
    src.execute("VACUUM INTO ?1", [dest.to_string_lossy()])
        .map(|_| ())
        .map_err(|e| format!("生成数据库快照失败: {}", e))
}

fn undo_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(sidecar(&db_file_path(app)?, ".before-restore"))
}

/// 用 `src` 替换当前库。顺序：先在临时文件里验证并迁移新库，再给当前库留回滚点
/// `waypoint.db.before-restore`，最后换文件；任何一步失败都回到原库。
pub(crate) fn restore_backup(app: &AppHandle, bytes: &[u8], password: &str, state: &State<AppState>) -> Result<(), String> {
    let contents = crate::backup::decode(bytes, password)?;
    let conn = crate::backup::open_bytes(&contents.database)?;
    replace_db_connection(&db_file_path(app)?, &conn, &state.db, &contents.images)
}

fn replace_db_file(dest: &Path, src: &Path, db: &Mutex<rusqlite::Connection>) -> Result<(), String> {
    if !src.exists() {
        return Err("备份文件不存在".into());
    }
    let probe = rusqlite::Connection::open_with_flags(src, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("无法打开备份: {e}"))?;
    replace_db_connection(dest, &probe, db, &[])
}

fn replace_db_connection(dest: &Path, source: &rusqlite::Connection, db: &Mutex<rusqlite::Connection>, images: &[(String, Vec<u8>)]) -> Result<(), String> {
    let mut guard = db.lock().map_err(|e| e.to_string())?;
    if crate::playtime::has_active(&guard)? { return Err("游戏正在计时，请退出游戏后再恢复备份".into()); }
    let probe = crate::backup::snapshot(source)?;
    let ok: i64 = probe
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('app_settings','tasks','vault_items')",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
    if ok < 1 {
        return Err("这不是 Waypoint 数据库备份".into());
    }
    let staged = sidecar(dest, ".restore-tmp");
    let verify = (|| -> Result<(), String> {
        let check: String = probe
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if check != "ok" {
            return Err(format!("备份文件已损坏: {}", check));
        }
        probe.pragma_update(None, "secure_delete", true).map_err(|e| e.to_string())?;
        // Credentials belong to this machine. A legacy import must not write its
        // service passwords into Credential Manager during schema validation.
        probe.execute("DELETE FROM app_settings WHERE key IN ('webdav_password','steam_web_api_key')", [])
            .map_err(|e| e.to_string())?;
        crate::init_database_schema(&probe).map_err(|e| format!("备份升级失败: {}", e))?;
        // Keep this machine's connection identity and credential markers
        // together. Restoring data must not disconnect an already working
        // account or pair its password with another machine's username.
        let local_webdav = crate::settings::read_app_setting(&guard, "webdav_url")
            .is_some_and(|url| !url.trim().is_empty());
        for key in [
            "webdav_url", "webdav_username", "webdav_password",
            "webdav_last_sync_time", "webdav_remote_stamp", "steam_web_api_key",
        ] {
            if key != "steam_web_api_key" && !local_webdav {
                continue;
            }
            probe.execute("DELETE FROM app_settings WHERE key = ?1", [key])
                .map_err(|e| e.to_string())?;
            if let Some(value) = crate::settings::read_app_setting(&guard, key) {
                probe.execute("INSERT INTO app_settings (key,value) VALUES (?1,?2)", [key, &value])
                    .map_err(|e| e.to_string())?;
            }
        }
        crate::playtime::settle_restored_sessions(&probe)?;
        crate::backup::validate_and_seal_vault(&probe)?;
        crate::vault::migrate_vault_payload(&probe).map_err(|e| e.to_string())?;
        crate::backup::restore_images(dest.parent().ok_or("数据目录不存在")?, images)?;
        // Only locally sealed data may reach a temporary file.
        snapshot_db(&probe, &staged)
    })();
    if let Err(e) = verify {
        remove_db_files(&staged);
        return Err(e);
    }

    let undo = sidecar(dest, ".before-restore");
    if let Err(e) = snapshot_db(&guard, &undo) {
        remove_db_files(&staged);
        return Err(e);
    }
    *guard = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;

    remove_db_files(dest);
    let swapped = std::fs::rename(&staged, dest)
        .map_err(|e| format!("写入数据库失败: {}", e))
        .and_then(|_| rusqlite::Connection::open(dest).map_err(|e| e.to_string()));
    match swapped {
        Ok(conn) => {
            *guard = conn;
            Ok(())
        }
        Err(e) => {
            // 换库失败：从回滚点还原原库，保证应用仍有可用数据库。
            remove_db_files(dest);
            remove_db_files(&staged);
            std::fs::copy(&undo, dest).map_err(|re| format!("{}；还原原库也失败: {}", e, re))?;
            *guard = rusqlite::Connection::open(dest).map_err(|re| format!("{}；重新打开原库失败: {}", e, re))?;
            Err(e)
        }
    }
}

#[tauri::command]
pub fn import_database(app: AppHandle, src_path: String, password: Option<String>, state: State<AppState>) -> Result<(), String> {
    let src = PathBuf::from(&src_path);
    let bytes = std::fs::read(&src).map_err(|e| e.to_string())?;
    if bytes.starts_with(b"SQLite format 3\0") {
        return replace_db_file(&db_file_path(&app)?, &src, &state.db);
    }
    restore_backup(&app, &bytes, password.as_deref().unwrap_or(""), &state)
}

/// 撤销上次导入/恢复：换回 `before-restore` 快照。撤销本身也会留下新的回滚点，可以再撤回来。
#[tauri::command]
pub fn undo_last_restore(app: AppHandle, state: State<AppState>) -> Result<(), String> {
    let undo = undo_path(&app)?;
    if !undo.exists() {
        return Err("没有可撤销的恢复记录".into());
    }
    let held = sidecar(&undo, ".src");
    std::fs::copy(&undo, &held).map_err(|e| format!("读取回滚点失败: {}", e))?;
    let res = replace_db_file(&db_file_path(&app)?, &held, &state.db);
    remove_db_files(&held);
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db_with(path: &Path, value: &str) -> rusqlite::Connection {
        let conn = rusqlite::Connection::open(path).unwrap();
        init_database(&conn).unwrap();
        conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES ('k', ?1)", [value]).unwrap();
        conn
    }

    fn read_k(db: &Mutex<rusqlite::Connection>) -> String {
        db.lock().unwrap().query_row("SELECT value FROM app_settings WHERE key = 'k'", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn restore_swaps_db_and_keeps_undo_point() {
        let dir = std::env::temp_dir().join(format!("wp-restore-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let dest = dir.join("waypoint.db");
        let backup = dir.join("backup.db");
        drop(db_with(&backup, "backup"));
        let db = Mutex::new(db_with(&dest, "current"));
        let local_settings = [
            ("webdav_url", "https://local.example/dav/"),
            ("webdav_username", "local-user"),
            ("webdav_password", crate::secrets::STORED_MARK),
            ("webdav_remote_stamp", "local-version"),
            ("webdav_last_sync_time", "2026-11-06 02:00"),
            ("steam_web_api_key", crate::secrets::STORED_MARK),
        ];
        for (key, value) in local_settings {
            db.lock().unwrap().execute("INSERT OR REPLACE INTO app_settings (key,value) VALUES (?1,?2)", [key, value]).unwrap();
        }
        let incoming = rusqlite::Connection::open(&backup).unwrap();
        for (key, value) in [
            ("webdav_url", "https://backup.example/dav/"),
            ("webdav_username", "backup-user"),
            ("webdav_password", "foreign-password-must-not-be-imported"),
            ("steam_web_api_key", "foreign-api-key-must-not-be-imported"),
        ] {
            incoming.execute("INSERT OR REPLACE INTO app_settings (key,value) VALUES (?1,?2)", [key, value]).unwrap();
        }
        drop(incoming);
        let assert_local_connections = || {
            // Reopen the physical database, as the next client launch does.
            let reopened = rusqlite::Connection::open(&dest).unwrap();
            for (key, value) in local_settings {
                assert_eq!(crate::settings::read_app_setting(&reopened, key).as_deref(), Some(value), "{key}");
            }
        };

        replace_db_file(&dest, &backup, &db).unwrap();
        assert_eq!(read_k(&db), "backup");
        assert_local_connections();

        // 回滚点是原库；再次替换回去即撤销
        let undo = sidecar(&dest, ".before-restore");
        let held = sidecar(&undo, ".src");
        std::fs::copy(&undo, &held).unwrap();
        replace_db_file(&dest, &held, &db).unwrap();
        assert_eq!(read_k(&db), "current");
        assert_local_connections();

        // A new machine may import the connection address, never another
        // machine's credential or a marker pointing at an unrelated secret.
        let fresh_dest = dir.join("fresh.db");
        let fresh = Mutex::new(db_with(&fresh_dest, "fresh"));
        replace_db_file(&fresh_dest, &backup, &fresh).unwrap();
        let fresh_conn = fresh.lock().unwrap();
        assert_eq!(crate::settings::read_app_setting(&fresh_conn, "webdav_url").as_deref(), Some("https://backup.example/dav/"));
        for key in ["webdav_password", "steam_web_api_key"] {
            assert_eq!(crate::settings::read_app_setting(&fresh_conn, key), None);
        }
        drop(fresh_conn);
        drop(fresh);

        // 不是 Waypoint 库：拒绝且原库不动
        let junk = dir.join("junk.db");
        rusqlite::Connection::open(&junk).unwrap().execute("CREATE TABLE x (a)", []).unwrap();
        assert!(replace_db_file(&dest, &junk, &db).is_err());
        assert_eq!(read_k(&db), "current");

        // A structurally valid foreign DPAPI backup must not replace the current
        // database or its undo point.
        let foreign = dir.join("foreign.db");
        let foreign_conn = db_with(&foreign, "foreign");
        foreign_conn.execute("INSERT INTO vault_items (id,platform,account_name,payload) VALUES ('a','sky','test','dpapi:00')", []).unwrap();
        let undo_before = std::fs::read(&undo).unwrap();
        assert!(replace_db_file(&dest, &foreign, &db).is_err());
        assert_eq!(read_k(&db), "current");
        assert_eq!(std::fs::read(&undo).unwrap(), undo_before);
        drop(foreign_conn);

        drop(db);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
