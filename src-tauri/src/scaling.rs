//! Waypoint owns the headless scaler's lifetime. The scaler owns capture, focus and cursor restoration.
use serde::{Deserialize, Serialize};
use std::{io::{BufRead, BufReader, Write}, path::{Path, PathBuf}, sync::{mpsc, Mutex, OnceLock}, time::Duration};
use tauri::{Emitter, Manager, State};
use crate::{AppState, records::GameRecord};
use crate::game_saves::normalized as path_key;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingProfile {
    pub directory: String,
    pub preset: String,
    pub capture: String,
    pub mode: String,
    pub auto_start: bool,
}
impl ScalingProfile {
    fn validate(&self) -> Result<(), String> {
        if !["fit", "anime", "sharp", "pixel"].contains(&self.preset.as_str())
            || !["auto", "wgc", "gdi"].contains(&self.capture.as_str())
            || !["fullscreen", "window"].contains(&self.mode.as_str()) {
            return Err("缩放预设无效，请重新保存作品设置".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingStatus {
    pub record_id: Option<String>,
    pub directory: Option<String>,
    pub phase: String,
    pub message: String,
}
impl Default for ScalingStatus {
    fn default() -> Self { Self { record_id: None, directory: None, phase: "idle".into(), message: String::new() } }
}
enum Command { Targets(Vec<(u32, u64)>), Stop }
#[derive(Default)]
struct Controller { token: u64, sender: Option<mpsc::Sender<Command>>, status: ScalingStatus }
static CONTROLLER: OnceLock<Mutex<Controller>> = OnceLock::new();
fn controller() -> &'static Mutex<Controller> { CONTROLLER.get_or_init(Default::default) }
pub(crate) fn is_running() -> bool { controller().lock().map(|c| c.sender.is_some()).unwrap_or(false) }
fn publish(app: &tauri::AppHandle, token: u64, phase: &str, message: String) {
    if let Ok(mut c) = controller().lock() {
        if c.token != token { return; }
        c.status.phase = phase.into(); c.status.message = message;
        let _ = app.emit("game-scaling-updated", &c.status);
    }
}
pub(crate) fn sync_processes(processes: &[crate::playtime::Process]) {
    if let Ok(c) = controller().lock() {
        if let (Some(sender), Some(directory)) = (&c.sender, &c.status.directory) {
            let _ = sender.send(Command::Targets(crate::playtime::targets_in(processes, directory)));
        }
    }
}
pub(crate) fn shutdown() {
    if let Ok(c) = controller().lock() { if let Some(tx) = &c.sender { let _ = tx.send(Command::Stop); } }
}
#[tauri::command]
pub fn get_game_scaling() -> Result<ScalingStatus, String> {
    Ok(controller().lock().map_err(|e| e.to_string())?.status.clone())
}
#[tauri::command]
pub fn stop_game_scaling() -> Result<(), String> { shutdown(); Ok(()) }

#[cfg(all(windows, target_arch = "x86_64"))]
fn prepare_runtime(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    const PAYLOAD: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/scaling/payload.tar.gz"));
    let digest = ring::digest::digest(&ring::digest::SHA256, PAYLOAD);
    let revision = digest.as_ref()[..10].iter().map(|b| format!("{b:02x}")).collect::<String>();
    let parent = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("scaling");
    let directory = parent.join(revision);
    if !directory.join("waypoint-scaling.exe").is_file() {
        std::fs::create_dir_all(&parent).map_err(|e| e.to_string())?;
        let staging = tempfile::tempdir_in(&parent).map_err(|e| e.to_string())?;
        tar::Archive::new(flate2::read::GzDecoder::new(PAYLOAD)).unpack(staging.path()).map_err(|e| format!("缩放组件解包失败：{e}"))?;
        std::fs::rename(staging.path(), &directory).map_err(|e| format!("缩放组件准备失败：{e}"))?;
    }
    Ok(directory)
}
#[cfg(not(all(windows, target_arch = "x86_64")))]
fn prepare_runtime(_: &tauri::AppHandle) -> Result<PathBuf, String> { Err("内置缩放目前支持 Windows x64".into()) }

fn profile_for(record: &GameRecord, directory: &str) -> ScalingProfile {
    record.library.iter().flat_map(|l| &l.scaling).find(|p| path_key(Path::new(&p.directory)) == path_key(Path::new(directory)))
        .cloned().unwrap_or_else(|| ScalingProfile { directory: directory.into(), preset: "fit".into(), capture: "auto".into(), mode: "fullscreen".into(), auto_start: false })
}
fn start(app: &tauri::AppHandle, record: &GameRecord, directory: &str) -> Result<(), String> {
    let profile = profile_for(record, directory);
    profile.validate()?;
    let directory = std::fs::canonicalize(directory).map_err(|_| "游戏安装目录不可用")?;
    let targets = crate::playtime::targets_for_directory(&directory)?;
    let mut c = controller().lock().map_err(|e| e.to_string())?;
    if c.sender.is_some() { return Err("已有缩放任务，请先停止当前缩放".into()); }
    let runtime = prepare_runtime(app)?;
    let mut command = std::process::Command::new(runtime.join("waypoint-scaling.exe"));
    command.current_dir(&runtime).args([&profile.preset, &profile.capture, &profile.mode])
        .stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let mut child = command.spawn().map_err(|e| format!("缩放组件启动失败：{e}"))?;
    let mut stdin = child.stdin.take().ok_or("缩放控制通道不可用")?;
    let stdout = child.stdout.take().ok_or("缩放状态通道不可用")?;
    let (sender, receiver) = mpsc::channel();
    c.token += 1;
    let token = c.token;
    c.status = ScalingStatus { record_id: Some(record.id.clone()), directory: Some(path_key(&directory)), phase: "waiting".into(), message: String::new() };
    let _ = app.emit("game-scaling-updated", &c.status);
    sender.send(Command::Targets(targets)).map_err(|e| e.to_string())?;
    c.sender = Some(sender);
    let app = app.clone();
    std::thread::spawn(move || {
        let reader_app = app.clone();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break; };
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(phase) = value["phase"].as_str().filter(|p| ["waiting","starting","scaling","paused","idle","error"].contains(p)) {
                        publish(&reader_app, token, phase, value["message"].as_str().unwrap_or("").into());
                    }
                }
            }
        });
        let mut stopping = None;
        let mut last_targets = Vec::new();
        let exit = loop {
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Command::Targets(targets)) if targets != last_targets => {
                    let line = format!("targets {}\n", targets.iter().map(|(pid, created)| format!("{pid} {created}")).collect::<Vec<_>>().join(" "));
                    if stdin.write_all(line.as_bytes()).is_err() { break child.wait().ok(); }
                    last_targets = targets;
                }
                Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = stdin.write_all(b"stop\n");
                    if stopping.is_none() { stopping = Some(std::time::Instant::now()); }
                }
                _ => {}
            }
            if let Ok(Some(status)) = child.try_wait() { break Some(status); }
            // This process was launched by us; a stuck scaler may be terminated after its
            // normal restore path has had time to run. Game processes are never terminated.
            if stopping.is_some_and(|at| at.elapsed() > Duration::from_secs(8)) {
                let _ = child.kill(); break child.wait().ok();
            }
        };
        drop(stdin);
        let _ = reader.join();
        if let Ok(mut c) = controller().lock() {
            if c.token == token {
                c.sender = None;
                if c.status.phase != "error" {
                    c.status.phase = if exit.is_some_and(|s| s.success()) { "idle" } else { "error" }.into();
                    if c.status.phase == "error" { c.status.message = "缩放组件意外退出，可重新应用；游戏继续运行".into(); }
                }
                let _ = app.emit("game-scaling-updated", &c.status);
            }
        }
    });
    Ok(())
}
pub(crate) fn auto_start(app: &tauri::AppHandle, record: &GameRecord, directory: &str) {
    if !profile_for(record, directory).auto_start || is_running() { return; }
    if let Err(error) = start(app, record, directory) {
        // Launch already succeeded. Report scaling separately from launching the game.
        let status = ScalingStatus { record_id: Some(record.id.clone()), directory: Some(directory.into()), phase: "error".into(), message: error };
        if let Ok(mut c) = controller().lock() {
            if c.sender.is_none() { c.status = status.clone(); }
        }
        let _ = app.emit("game-scaling-updated", status);
    }
}
#[tauri::command]
pub fn start_game_scaling(record_id: String, installation_index: usize, app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    let record = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::records::read_records(&conn)?.into_iter().find(|r| r.id == record_id).ok_or("游戏档案不存在")?
    };
    let directory = &record.library.as_ref().and_then(|l| l.installations.get(installation_index)).ok_or("请先关联本地安装")?.directory;
    start(&app, &record, directory)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profiles_bind_to_an_installation_and_reject_unknown_native_options() {
        let record: GameRecord = serde_json::from_value(serde_json::json!({
            "id":"one","title":"one","category":"galgame","status":"backlog","progress":"","rating":0,
            "library":{"installations":[],"scaling":[{"directory":"C:/Games/One","preset":"anime","capture":"gdi","mode":"fullscreen","auto_start":true}]}
        })).unwrap();
        assert!(profile_for(&record, "c:\\games\\one\\").auto_start);
        assert!(!profile_for(&record, "c:\\games\\two").auto_start);
        let mut profile = profile_for(&record, "c:\\games\\one"); profile.preset = "arbitrary_shader".into();
        assert!(profile.validate().is_err());
        let mut network_record = record;
        network_record.library.as_mut().unwrap().scaling[0].directory = r"\\server\Games\One".into();
        assert!(profile_for(&network_record, r"\\?\UNC\server\games\one\").auto_start);
    }
}
