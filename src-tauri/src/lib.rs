use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, RunEvent, WindowEvent};
use tauri_plugin_notification::NotificationExt;

mod cache;
mod cycle;
mod db;
mod backup;
mod save_backups;
mod game_saves;
mod game_operations;
mod game_versions;
mod explore;
mod resources;
mod library_removal;
mod juij;
mod hotkeys;
mod dialogs;
mod guides;
mod jx3;
mod jx3_exterior;
mod memos;
mod mingyi;
mod posters;
mod public_http;
mod progress;
mod projection;
mod records;
mod game_library;
mod game_trailers;
mod discovery;
mod secrets;
mod settings;
mod sky;
mod steam;
mod playtime;
mod scaling;
mod tasks;
mod vault;
mod webdav;

// Public metadata, guide pages and images may traverse a slow proxy or overseas origin.
pub(crate) const PUBLIC_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
pub(crate) const PUBLIC_REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

pub use sky::inspect_main;
use db::{export_database, get_db_info, import_database, undo_last_restore};
use dialogs::{open_in_explorer, pick_folder, pick_open_file, pick_save_file};
use explore::search_explore_site;
use juij::read_juij_resource;
use guides::{
    open_in_obsidian, read_obsidian_file, scan_obsidian_game_guides, scan_obsidian_vault,
    update_guide_frontmatter,
};
use jx3::{
    fetch_jx3_box_achievement_search,
    fetch_jx3_box_reputations,
    fetch_jx3_box_fb_map, fetch_jx3_box_item_search,
    fetch_jx3_box_items_by_ids,
    fetch_jx3_box_serendipities, fetch_jx3_celebs, fetch_jx3_daily,
};
use jx3_exterior::{fetch_jx3_goods_by_name, fetch_jx3_goods_quote, scan_jx3_exterior, search_jx3_goods};
use hotkeys::{apply_hotkeys, apply_hotkeys_command, hotkey_status, on_shortcut};
use memos::{
    add_custom_memo, delete_custom_memo, get_custom_memos, toggle_custom_memo, update_custom_memo,
};
use mingyi::scan_mingyi_roles;
use projection::export_waypoint_projection;
use records::{save_game_record, get_game_records};
use settings::{delete_setting, get_all_settings, get_setting, save_setting};
use sky::{
    fetch_sky_calendar, fetch_sky_daily_data, fetch_sky_events, fetch_sky_quest_calendar,
    fetch_sky_role_wings, fetch_sky_seasons, fetch_sky_boost_feedback, fetch_sky_event_guide_images, fetch_sky_event_edition, resolve_sky_collection_media,
};
use steam::{get_steam_library, refresh_steam_library};
use tasks::{add_task, delete_task, get_tasks, toggle_task, update_task_title, update_task_schedule_time, update_task_category};
use vault::{add_vault_item, delete_vault_item, get_vault_items, update_vault_item};
use webdav::{
    download_webdav_backup, get_webdav_status, save_webdav_config, test_webdav_connection,
    upload_webdav_backup, cancel_webdav_backup,
};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
}

struct PendingWindowPersist {
    last_schedule: Mutex<Instant>,
    armed: AtomicBool,
}

/// 表结构版本，记在 `PRAGMA user_version`。改表时在 `migrate_schema` 末尾追加一级，不要改已发布的级。
const SCHEMA_VERSION: i64 = 3;

pub(crate) fn init_database(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    init_database_schema(conn)?;
    // 同平台同账号的重复金库项每次启动都合并，界面允许重复添加。
    crate::vault::migrate_vault_payload(conn)
}

pub(crate) fn init_database_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < SCHEMA_VERSION {
        let tx = conn.unchecked_transaction()?;
        migrate_schema(&tx, version)?;
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        tx.commit()?;
    }
    Ok(())
}

/// 表结构要升级时，先把旧库整份拷成 `waypoint.db.pre-v{旧版本}`，迁移出问题可以手动换回。
fn snapshot_before_upgrade(conn: &rusqlite::Connection, db_path: &std::path::Path) {
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap_or(0);
    let has_data: bool = conn
        .query_row("SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'table'", [], |r| r.get(0))
        .unwrap_or(false);
    if version >= SCHEMA_VERSION || !has_data {
        return;
    }
    let dest = std::path::PathBuf::from(format!("{}.pre-v{}", db_path.to_string_lossy(), version));
    if dest.exists() {
        return;
    }
    if let Err(e) = conn.execute("VACUUM INTO ?1", [dest.to_string_lossy()]) {
        eprintln!("升级前快照失败: {e}");
    }
}

fn add_column_if_missing(
    conn: &rusqlite::Connection,
    table: &str,
    column: &str,
    decl: &str,
) -> rusqlite::Result<()> {
    let exists: bool = conn.query_row(
        &format!("SELECT COUNT(*) > 0 FROM pragma_table_info('{table}') WHERE name = ?1"),
        [column],
        |row| row.get(0),
    )?;
    if !exists {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"), [])?;
    }
    Ok(())
}

fn migrate_schema(conn: &rusqlite::Connection, from: i64) -> rusqlite::Result<()> {
    if from < 1 {
        migrate_v1(conn)?;
    }
    if from < 2 {
        progress::migrate_v2(conn)?;
    }
    if from < 3 {
        secrets::migrate_v3(conn)?;
    }
    Ok(())
}

/// v1：版本号出现前的全部表结构。旧库可能缺后加的列，逐列补齐。
fn migrate_v1(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute("DROP TABLE IF EXISTS characters", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            character_id TEXT NOT NULL,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            reset_rule TEXT NOT NULL,
            source_type TEXT NOT NULL,
            api_key TEXT,
            tag TEXT,
            done INTEGER DEFAULT 0,
            catalog_kind TEXT,
            catalog_id TEXT,
            schedule_time TEXT
        )",
        [],
    )?;
    add_column_if_missing(conn, "tasks", "catalog_kind", "TEXT")?;
    add_column_if_missing(conn, "tasks", "catalog_id", "TEXT")?;
    add_column_if_missing(conn, "tasks", "schedule_time", "TEXT")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS vault_items (
            id TEXT PRIMARY KEY,
            platform TEXT NOT NULL,
            account_name TEXT NOT NULL,
            server TEXT,
            notes TEXT,
            payload TEXT
        )",
        [],
    )?;
    add_column_if_missing(conn, "vault_items", "notes", "TEXT")?;
    add_column_if_missing(conn, "vault_items", "payload", "TEXT")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS game_records (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            category TEXT NOT NULL,
            status TEXT NOT NULL,
            progress TEXT NOT NULL,
            rating REAL DEFAULT 0.0,
            save_path TEXT,
            notes TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS steam_library (
            vault_item_id TEXT NOT NULL,
            appid INTEGER NOT NULL,
            name TEXT NOT NULL,
            playtime_forever INTEGER NOT NULL DEFAULT 0,
            playtime_2weeks INTEGER,
            rtime_last_played INTEGER,
            img_icon_url TEXT,
            PRIMARY KEY (vault_item_id, appid)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS custom_memos (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            tag TEXT NOT NULL,
            cycle_type TEXT NOT NULL,
            cycle_val TEXT,
            note TEXT,
            has_progress INTEGER DEFAULT 0,
            current_progress INTEGER DEFAULT 0,
            target_progress INTEGER DEFAULT 0,
            done INTEGER DEFAULT 0,
            last_completed_at TEXT,
            created_at TEXT NOT NULL
        )",
        [],
    )?;
    add_column_if_missing(conn, "custom_memos", "character_id", "TEXT")?;
    add_column_if_missing(conn, "custom_memos", "pinned", "INTEGER DEFAULT 0")?;
    Ok(())
}

#[cfg(test)]
mod schema_tests {
    use super::*;

    fn columns(conn: &rusqlite::Connection, table: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}')"))
            .unwrap();
        stmt.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
    }

    #[test]
    fn legacy_db_gains_missing_columns_and_version() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE characters (id TEXT);
             CREATE TABLE tasks (id TEXT PRIMARY KEY, character_id TEXT NOT NULL, title TEXT NOT NULL,
                 category TEXT NOT NULL, reset_rule TEXT NOT NULL, source_type TEXT NOT NULL,
                 api_key TEXT, tag TEXT, done INTEGER DEFAULT 0);
             INSERT INTO tasks VALUES ('t1','c1','日常','daily','daily','manual',NULL,NULL,0);
             CREATE TABLE vault_items (id TEXT PRIMARY KEY, platform TEXT NOT NULL,
                 account_name TEXT NOT NULL, server TEXT);",
        )
        .unwrap();

        init_database(&conn).unwrap();
        init_database(&conn).unwrap();

        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, SCHEMA_VERSION);
        assert!(columns(&conn, "characters").is_empty());
        let task_cols = columns(&conn, "tasks");
        for c in ["catalog_kind", "catalog_id", "schedule_time"] {
            assert!(task_cols.iter().any(|x| x == c), "tasks.{c}");
        }
        assert!(columns(&conn, "vault_items").iter().any(|x| x == "payload"));
        assert!(columns(&conn, "custom_memos").iter().any(|x| x == "pinned"));
        assert!(columns(&conn, "progress_items").iter().any(|x| x == "character_id"));
        let kept: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0)).unwrap();
        assert_eq!(kept, 1);
    }
}

// Tauri Commands: 切换窗口置顶模式
#[tauri::command]
fn toggle_always_on_top(app: AppHandle, enabled: bool) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        window.set_always_on_top(enabled).map_err(|e| e.to_string())?;
        Ok(enabled)
    } else {
        Err("Main window not found".to_string())
    }
}

// Tauri Commands: 获取窗口置顶状态
#[tauri::command]
fn is_always_on_top(app: AppHandle) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        window.is_always_on_top().map_err(|e| e.to_string())
    } else {
        Err("Main window not found".to_string())
    }
}

// Tauri Commands: 发送原生通知 (通过 tauri-plugin-notification)
#[tauri::command]
fn trigger_notification(title: String, body: String, app: AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show().map_err(|error| error.to_string())
}

const WINDOW_STATE_KEY: &str = "window_state";
const WINDOW_MIN_WIDTH: u32 = 860;
const WINDOW_MIN_HEIGHT: u32 = 520;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WindowState {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

fn read_window_state(conn: &rusqlite::Connection) -> Option<WindowState> {
    let raw: String = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [WINDOW_STATE_KEY],
            |row| row.get(0),
        )
        .ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_window_state(conn: &rusqlite::Connection, state: &WindowState) {
    if let Ok(raw) = serde_json::to_string(state) {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
            rusqlite::params![WINDOW_STATE_KEY, raw],
        );
    }
}

fn capture_main_window_state(app: &AppHandle) -> Option<WindowState> {
    let window = app.get_webview_window("main")?;
    if window.is_minimized().unwrap_or(false) {
        return None;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let size = window.outer_size().ok()?;
    let pos = window.outer_position().ok()?;
    if !maximized && (size.width < WINDOW_MIN_WIDTH || size.height < WINDOW_MIN_HEIGHT) {
        return None;
    }
    Some(WindowState {
        x: pos.x,
        y: pos.y,
        width: size.width.max(WINDOW_MIN_WIDTH),
        height: size.height.max(WINDOW_MIN_HEIGHT),
        maximized,
    })
}

fn persist_main_window_state(app: &AppHandle) {
    let Some(next) = capture_main_window_state(app) else {
        return;
    };
    let app_state = app.state::<AppState>();
    if let Ok(conn) = app_state.db.lock() {
        write_window_state(&conn, &next);
    };
}

pub(crate) fn show_main_window(window: &tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn restore_main_window_state(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let saved = {
        let app_state = app.state::<AppState>();
        let Ok(conn) = app_state.db.lock() else {
            return;
        };
        read_window_state(&conn)
    };
    let Some(state) = saved else {
        show_main_window(&window);
        return;
    };
    let width = state.width.max(WINDOW_MIN_WIDTH);
    let height = state.height.max(WINDOW_MIN_HEIGHT);
    if !state.maximized {
        let _ = window.set_size(PhysicalSize::new(width, height));
        if let Some(monitor) = window
            .current_monitor()
            .ok()
            .flatten()
            .or_else(|| window.primary_monitor().ok().flatten())
        {
            let mpos = monitor.position();
            let msize = monitor.size();
            let max_x = mpos.x + (msize.width as i32 - width as i32).max(0);
            let max_y = mpos.y + (msize.height as i32 - height as i32).max(0);
            let x = state.x.clamp(mpos.x, max_x);
            let y = state.y.clamp(mpos.y, max_y);
            let _ = window.set_position(PhysicalPosition::new(x, y));
        } else {
            let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
        }
        show_main_window(&window);
    } else {
        show_main_window(&window);
        let _ = window.maximize();
    }
}

fn schedule_persist_main_window_state(app: &AppHandle) {
    let pending = app.state::<PendingWindowPersist>();
    {
        let mut last = pending.last_schedule.lock().unwrap_or_else(|e| e.into_inner());
        *last = Instant::now();
    }
    if pending.armed.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let pending = handle.state::<PendingWindowPersist>();
            let elapsed = pending
                .last_schedule
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .elapsed();
            if elapsed < Duration::from_millis(250) {
                continue;
            }
            pending.armed.store(false, Ordering::SeqCst);
            persist_main_window_state(&handle);
            break;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(resources::ResourceBrowser::default())
        .register_asynchronous_uri_scheme_protocol("img", |ctx, request, responder| {
            let query = request.uri().query().unwrap_or("").to_string();
            let data_dir = ctx.app_handle().path().app_data_dir().ok();
            // ?poster=1：书库海报，走本地盘缓存
            let poster_dir = query
                .split('&')
                .any(|p| p == "poster=1")
                .then(|| data_dir.as_ref().map(|d| d.join("poster-cache")))
                .flatten();
            let wiki_dir = data_dir.map(|d| d.join("wiki-cache"));
            let target = request
                .uri()
                .query()
                .and_then(|q| {
                    q.split('&').find_map(|pair| {
                        let mut parts = pair.splitn(2, '=');
                        let key = parts.next()?;
                        let val = parts.next().unwrap_or("");
                        if key == "u" {
                            Some(urlencoding::decode(val).ok()?.into_owned())
                        } else {
                            None
                        }
                    })
                });
            tauri::async_runtime::spawn(async move {
                let fetched = match (&target, &poster_dir) {
                    (Some(url), Some(dir)) => Some(posters::load_poster(dir, url, None).await),
                    (Some(url), None) => match (sky::wiki_image_referer(url), &wiki_dir) {
                        (Some(referer), Some(dir)) => Some(posters::load_poster(dir, url, referer).await),
                        _ => Some(sky::fetch_hotlink_image(url).await),
                    },
                    _ => None,
                };
                let response = match fetched {
                    Some(result) => match result {
                        Ok((mime, bytes)) => tauri::http::Response::builder()
                            .header(tauri::http::header::CONTENT_TYPE, mime)
                            .header(tauri::http::header::CACHE_CONTROL, "public, max-age=86400")
                            .body(bytes)
                            .unwrap_or_else(|_| {
                                tauri::http::Response::builder()
                                    .status(500)
                                    .body(Vec::new())
                                    .unwrap()
                            }),
                        Err(_) => tauri::http::Response::builder()
                            .status(404)
                            .header(tauri::http::header::CACHE_CONTROL, "no-store")
                            .body(Vec::new())
                            .unwrap(),
                    },
                    None => tauri::http::Response::builder()
                        .status(400)
                        .body(Vec::new())
                        .unwrap(),
                };
                responder.respond(response);
            });
        })
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                show_main_window(&window);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| on_shortcut(app, shortcut, event.state()))
                .build(),
        )
        .setup(|app| {
            let hotkeys = apply_hotkeys(app.handle());

            // 确保 AppData 路径存在
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let _ = std::fs::create_dir_all(&app_data_dir);
            // 光遇图标和实穿图会随活动不断累积，超过 300MB 就删掉最旧的一半
            let wiki_cache = app_data_dir.join("wiki-cache");
            std::thread::spawn(move || posters::prune_dir(&wiki_cache, 300 * 1024 * 1024));
            let db_path = app_data_dir.join("waypoint.db");

            let conn = rusqlite::Connection::open(&db_path).expect("Failed to initialize SQLite database");
            snapshot_before_upgrade(&conn, &db_path);
            init_database(&conn).expect("Failed to create database tables");

            app.manage(AppState {
                db: Mutex::new(conn),
            });
            app.manage(webdav::BackupUpload::default());
            playtime::start_monitor(app.handle().clone());
            let cache_conn = cache::open(&app_data_dir.join("cache.db"))
                .or_else(|_| cache::open(std::path::Path::new(":memory:")))
                .expect("Failed to open cache database");
            app.manage(cache::CacheDb(Mutex::new(cache_conn)));
            app.manage(juij::JuijClient::new()?);
            app.manage(PendingWindowPersist {
                last_schedule: Mutex::new(Instant::now()),
                armed: AtomicBool::new(false),
            });
            // 窗口在数据库就绪、状态注册之后才创建，否则首屏 IPC 会先于 manage 到达。
            let window_configs: Vec<_> = app.config().app.windows.clone();
            for window_config in &window_configs {
                tauri::WebviewWindowBuilder::from_config(app.handle(), window_config)?.build()?;
            }

            // 配置系统托盘与托盘菜单
            let quit_item = MenuItem::with_id(app, "quit", "退出 Waypoint", true, None::<&str>)?;
            let show_item = MenuItem::with_id(
                app,
                "show",
                format!("显示主面板 ({})", hotkeys.panel),
                true,
                None::<&str>,
            )?;
            let today_item = MenuItem::with_id(app, "today", "今日日课…", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&today_item, &show_item, &quit_item])?;
            app.manage(cycle::TrayToday(today_item));

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            show_main_window(&window);
                        }
                    }
                    "today" => {
                        if let Some(window) = app.get_webview_window("main") {
                            show_main_window(&window);
                        }
                        let _ = tauri::Emitter::emit(app, "navigate", "today");
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        }
                        | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        } => {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                show_main_window(&window);
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            restore_main_window_state(app.handle());

            // 先同步换一次日，前端首次读任务时勾选已经是今天的；之后后台每分钟一次
            cycle::run_once(app.handle());
            cycle::spawn(app.handle().clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            game_library::scan_game_directory,
            game_library::cancel_game_scan,
            game_library::search_game_metadata,
            discovery::market_connected,
            discovery::connect_market,
            discovery::discover_games,
            discovery::discover_deals,
            discovery::game_prices,
            game_library::load_game_metadata,
            game_library::load_game_album,
            game_trailers::search_game_videos,
            game_library::bangumi_connected,
            game_library::connect_bangumi,
            game_library::game_installation_available,
            game_library::launch_library_game,
            playtime::get_local_playtime,
            scaling::get_game_scaling,
            scaling::start_game_scaling,
            scaling::stop_game_scaling,
            steam::get_steam_activity,
            steam::resolve_steam_title,
            steam::import_steam_games,
            steam::unlink_steam_game,
            steam::refresh_steam_achievements,
            steam::launch_steam_game,
            get_tasks,
            toggle_task,
            add_task,
            delete_task,
            update_task_title,
            update_task_schedule_time,
            update_task_category,
            get_vault_items,
            add_vault_item,
            update_vault_item,
            delete_vault_item,
            get_game_records,
            save_game_record,
            fetch_jx3_daily,
            scan_jx3_exterior,
            fetch_jx3_goods_quote,
            fetch_jx3_goods_by_name,
            search_jx3_goods,
            fetch_jx3_celebs,
            fetch_jx3_box_fb_map,
            fetch_jx3_box_item_search,
            fetch_jx3_box_serendipities,
            fetch_jx3_box_reputations,
            fetch_jx3_box_achievement_search,
            fetch_jx3_box_items_by_ids,
            fetch_sky_daily_data,
            fetch_sky_quest_calendar,
            fetch_sky_calendar,
            fetch_sky_seasons,
            resolve_sky_collection_media,
            fetch_sky_events,
            fetch_sky_event_guide_images,
            fetch_sky_event_edition,
            fetch_sky_role_wings,
            fetch_sky_boost_feedback,
            trigger_notification,
            toggle_always_on_top,
            is_always_on_top,
            open_in_explorer,
            pick_folder,
            scan_mingyi_roles,
            scan_obsidian_vault,
            read_obsidian_file,
            scan_obsidian_game_guides,
            search_explore_site,
            resources::read_resource_article,
            resources::write_resource_markdown,
            library_removal::preview_library_removal,
            library_removal::remove_library_game,
            resources::open_resource_web,
            resources::resource_web_current,
            resources::set_sky_benefits_view,
            resources::navigate_sky_benefits,
            read_juij_resource,
            open_in_obsidian,
            export_waypoint_projection,
            get_db_info,
            get_setting,
            save_setting,
            delete_setting,
            get_all_settings,
            export_database,
            import_database,
            undo_last_restore,
            progress::apply_progress_ops,
            cache::cache_get_all,
            cycle::refresh_cycle_now,
            steam::save_steam_web_api_key,
            vault::export_vault_plain,
            vault::import_vault_plain,
            steam::steam_web_api_key_configured,
            cycle::rebase_task_cycles,
            cache::cache_put,
            progress::get_progress_snapshot,
            progress::get_activity_stats,
            pick_save_file,
            pick_open_file,
            get_steam_library,
            refresh_steam_library,
            update_guide_frontmatter,
            get_custom_memos,
            add_custom_memo,
            apply_hotkeys_command,
            hotkey_status,
            update_custom_memo,
            delete_custom_memo,
            toggle_custom_memo,
            test_webdav_connection,
            save_webdav_config,
            get_webdav_status,
            upload_webdav_backup,
            cancel_webdav_backup,
            download_webdav_backup,
            save_backups::list_game_save_backups,
            game_saves::discover_game_saves,
            game_versions::inspect_game_versions,
            game_versions::switch_game_resources,
            save_backups::create_game_save_backup,
            save_backups::restore_game_save_backup,
            save_backups::upload_game_save_backup,
            save_backups::download_game_save_backup
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| match event {
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Focused(false),
                ..
            } if label == "quick-memo" => {
                if let Some(window) = app.get_webview_window("quick-memo") {
                    let _ = window.hide();
                }
            }
            RunEvent::WindowEvent {
                label,
                event: WindowEvent::Moved(_) | WindowEvent::Resized(_),
                ..
            } if label == "main" => schedule_persist_main_window_state(app),
            RunEvent::ExitRequested { .. } | RunEvent::Exit => { scaling::shutdown(); persist_main_window_state(app); },
            _ => {}
        });
}
