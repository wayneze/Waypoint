use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const HOTKEY_PANEL_KEY: &str = "hotkey_panel";
pub const HOTKEY_CAPTURE_KEY: &str = "hotkey_capture";
const DEFAULT_PANEL: &str = "Alt+W";
const DEFAULT_CAPTURE: &str = "Alt+M";
const LEGACY_CAPTURE: &str = "Alt+Shift+M";

#[derive(Clone, serde::Serialize)]
pub struct HotkeyStatus {
    pub panel: String,
    pub capture: String,
    pub panel_error: Option<String>,
    pub capture_error: Option<String>,
}

fn read_hotkey(app: &AppHandle, key: &str, fallback: &str) -> String {
    let Some(state) = app.try_state::<crate::AppState>() else {
        return fallback.to_string();
    };
    let Ok(conn) = state.db.lock() else {
        return fallback.to_string();
    };
    let stored = crate::settings::read_app_setting(&conn, key)
        .filter(|value| value.parse::<Shortcut>().is_ok());
    match stored {
        Some(value) if key == HOTKEY_CAPTURE_KEY && value.eq_ignore_ascii_case(LEGACY_CAPTURE) => {
            fallback.to_string()
        }
        Some(value) => value,
        None => fallback.to_string(),
    }
}

fn toggle_main(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn show_capture(app: &AppHandle) {
    let Some(window) = app.get_webview_window("quick-memo") else {
        return;
    };
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let pos = monitor.position();
        let width = 640.0 * monitor.scale_factor();
        let x = pos.x + ((size.width as f64 - width) / 2.0).round() as i32;
        let y = pos.y + (size.height as f64 * 0.28).round() as i32;
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
    }
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn same_shortcut(pressed: &Shortcut, stored: &str) -> bool {
    stored
        .parse::<Shortcut>()
        .map(|parsed| parsed.id() == pressed.id())
        .unwrap_or(false)
}

fn register_one(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let shortcut = accelerator
        .parse::<Shortcut>()
        .map_err(|err| err.to_string())?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|err| err.to_string())
}

pub fn apply_hotkeys(app: &AppHandle) -> HotkeyStatus {
    let panel = read_hotkey(app, HOTKEY_PANEL_KEY, DEFAULT_PANEL);
    let capture = read_hotkey(app, HOTKEY_CAPTURE_KEY, DEFAULT_CAPTURE);
    let _ = app.global_shortcut().unregister_all();
    let panel_error = register_one(app, &panel).err();
    let capture_error = if capture.eq_ignore_ascii_case(&panel) {
        Some("记一笔不能和主面板用同一条热键".to_string())
    } else {
        register_one(app, &capture).err()
    };
    let status = HotkeyStatus {
        panel: panel.clone(),
        capture: capture.clone(),
        panel_error,
        capture_error,
    };
    let _ = app.emit("hotkeys-changed", &status);
    status
}

pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    if state != ShortcutState::Pressed {
        return;
    }
    let panel = read_hotkey(app, HOTKEY_PANEL_KEY, DEFAULT_PANEL);
    let capture = read_hotkey(app, HOTKEY_CAPTURE_KEY, DEFAULT_CAPTURE);
    if same_shortcut(shortcut, &panel) {
        toggle_main(app);
    } else if same_shortcut(shortcut, &capture) {
        show_capture(app);
    }
}

#[tauri::command]
pub fn apply_hotkeys_command(app: AppHandle) -> Result<HotkeyStatus, String> {
    Ok(apply_hotkeys(&app))
}

#[tauri::command]
pub fn hotkey_status(app: AppHandle) -> Result<HotkeyStatus, String> {
    let panel = read_hotkey(&app, HOTKEY_PANEL_KEY, DEFAULT_PANEL);
    let capture = read_hotkey(&app, HOTKEY_CAPTURE_KEY, DEFAULT_CAPTURE);
    let panel_error = if app
        .global_shortcut()
        .is_registered(panel.parse::<Shortcut>().map_err(|err| err.to_string())?)
    {
        None
    } else {
        Some(format!("{panel} 没有注册上"))
    };
    let capture_error = if app
        .global_shortcut()
        .is_registered(capture.parse::<Shortcut>().map_err(|err| err.to_string())?)
    {
        None
    } else {
        Some(format!("{capture} 没有注册上"))
    };
    Ok(HotkeyStatus {
        panel,
        capture,
        panel_error,
        capture_error,
    })
}
