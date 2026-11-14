//! Local playtime counts confirmed process lifetimes using an awake-time clock.
//! It never adds Steam's remote minutes, and never terminates game processes.
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{Emitter, Manager, State};
use crate::{steam::{read_json, write_json}, AppState};
use crate::game_saves::normalized;

pub(crate) const PLAY_PREFIX: &str = "waypoint_local_play_";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ProcessId { pid: u32, created: u64 }
#[derive(Debug, Clone)]
pub(crate) struct Process { id: ProcessId, parent: u32, path: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub seconds: f64,
    pub recovered: bool,
    pub end_reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    pub session: Session,
    #[serde(default)]
    pub tracking_error: Option<String>,
    directory: String,
    processes: Vec<ProcessId>,
    launched_after: u64,
    last_tick: u64,
    last_seen: i64,
    missing_since: Option<u64>,
    launcher_pid: Option<u32>,
    handed_off: bool,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LocalPlaytime {
    pub record_id: String,
    pub total_seconds: f64,
    pub last_played: Option<i64>,
    pub sessions: Vec<Session>,
    pub active: Option<ActiveSession>,
}

fn key(id: &str) -> String { format!("{PLAY_PREFIX}{id}") }
fn in_directory(path: &str, directory: &str) -> bool { path.starts_with(&format!("{directory}\\")) }
fn launcher(path: &Path) -> bool {
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
    ["launcher", "启动", "ランチャー"].iter().any(|part| name.contains(part))
}

fn load_all(conn: &rusqlite::Connection) -> Result<Vec<LocalPlaytime>, String> {
    let mut stmt = conn.prepare("SELECT s.value FROM app_settings s JOIN game_records g ON s.key=?1 || g.id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([PLAY_PREFIX], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect()
}

pub(crate) fn is_active(conn: &rusqlite::Connection, id: &str) -> Result<bool, String> {
    Ok(read_json::<LocalPlaytime>(conn, &key(id))?.active.is_some())
}
pub(crate) fn has_active(conn: &rusqlite::Connection) -> Result<bool, String> {
    Ok(load_all(conn)?.iter().any(|p| p.active.is_some()))
}
pub(crate) fn ensure_stopped(conn: &rusqlite::Connection, record: &crate::records::GameRecord) -> Result<(), String> {
    if is_active(conn, &record.id)? { return Err("游戏正在运行，请退出后再操作存档或版本".into()); }
    let directories: Vec<_> = record.library.iter().flat_map(|l| &l.installations).map(|i| normalized(Path::new(&i.directory))).collect();
    if !directories.is_empty() && processes()?.iter().any(|p| directories.iter().any(|d| in_directory(&p.path, d)) && is_game_process(p)) {
        return Err("安装目录中仍有游戏进程，请退出后重试".into());
    }
    Ok(())
}

#[tauri::command]
pub fn get_local_playtime(state: State<AppState>) -> Result<Vec<LocalPlaytime>, String> {
    load_all(&*state.db.lock().map_err(|e| e.to_string())?)
}

pub(crate) fn launch(conn: &rusqlite::Connection, record_id: &str, directory: &Path, executable: &Path) -> Result<(), String> {
    let directory_text = normalized(directory);
    if load_all(conn)?.iter().any(|p| p.active.as_ref().is_some_and(|a| p.record_id == record_id || a.directory == directory_text)) {
        return Err("这部游戏正在计时，请勿重复启动；退出后等待数秒结算".into());
    }
    if processes()?.iter().any(|p| in_directory(&p.path, &directory_text) && is_game_process(p)) {
        return Err("该目录已有游戏进程运行，请先退出后再从 Waypoint 启动".into());
    }
    let tick = awake_ticks()?;
    let launched_after = filetime_now();
    let child = std::process::Command::new(executable).current_dir(directory).spawn().map_err(|e| format!("启动失败：{e}"))?;
    let process = child_identity(&child)?;
    let now = chrono::Utc::now().timestamp();
    let mut stats: LocalPlaytime = read_json(conn, &key(record_id))?;
    stats.record_id = record_id.into();
    stats.last_played = Some(now);
    stats.active = Some(ActiveSession {
        session: Session { started_at: now, ended_at: None, seconds: 0.0, recovered: false, end_reason: None },
        tracking_error: None,
        directory: directory_text, processes: vec![process], launched_after, last_tick: tick, last_seen: now,
        missing_since: None, launcher_pid: launcher(executable).then_some(child.id()), handed_off: false,
    });
    write_json(conn, &key(record_id), &stats).map_err(|e| format!("游戏已启动，但无法保存计时：{e}"))
}

fn is_game_process(p: &Process) -> bool {
    crate::game_library::game_executable(p.path.rsplit('\\').next().unwrap_or(""))
}
pub(crate) fn targets_in(current: &[Process], directory: &str) -> Vec<(u32, u64)> {
    current.iter().filter(|p| p.id.created != 0 && in_directory(&p.path, directory) && is_game_process(p))
        .map(|p| (p.id.pid, p.id.created)).collect()
}
pub(crate) fn targets_for_directory(directory: &Path) -> Result<Vec<(u32, u64)>, String> {
    Ok(targets_in(&processes()?, &normalized(directory)))
}

fn advance(stats: &mut LocalPlaytime, current: &[Process], tick: u64, now: i64) {
    let Some(active) = stats.active.as_mut() else { return };
    if current.iter().any(|p| p.id.created == 0 && active.processes.iter().any(|known| known.pid == p.id.pid)) {
        active.tracking_error = Some("无法确认游戏进程身份，已暂停计时；请检查游戏是否以管理员权限运行".into());
        active.last_tick = tick;
        return;
    }
    active.tracking_error = None;
    // Creation time prevents a recycled PID (or a process on another boot) from resuming a session.
    let mut alive: Vec<_> = current.iter().filter(|p| active.processes.contains(&p.id)).collect();
    for p in current {
        if p.id.created >= active.launched_after && in_directory(&p.path, &active.directory) && is_game_process(p)
            && !alive.iter().any(|a| a.id == p.id) {
            alive.push(p);
        }
    }
    // Descendants outside the install folder are included only when their observed parent belongs to this session.
    loop {
        let extra: Vec<_> = current.iter().filter(|p| p.id.created >= active.launched_after && is_game_process(p)
            && alive.iter().any(|parent| parent.id.pid == p.parent && p.id.created >= parent.id.created)
            && !alive.iter().any(|a| a.id == p.id)).collect();
        if extra.is_empty() { break; }
        alive.extend(extra);
    }
    if active.launcher_pid.is_some() && alive.iter().any(|p| Some(p.id.pid) != active.launcher_pid) { active.handed_off = true; }
    let playing = alive.iter().any(|p| !active.handed_off || Some(p.id.pid) != active.launcher_pid);
    if playing && tick >= active.last_tick {
        if active.missing_since.is_none() {
            active.session.seconds += (tick - active.last_tick) as f64 / 10_000_000.0;
            if now - active.last_seen > 10 { active.session.recovered = true; }
        }
        active.last_seen = now;
        active.missing_since = None;
        // Retain only identities still alive, plus the launcher during a handoff.
        active.processes = alive.iter().map(|p| p.id.clone()).collect();
    } else {
        let missing = *active.missing_since.get_or_insert(tick);
        if tick < active.last_tick || tick.saturating_sub(missing) >= 100_000_000 {
            let mut session = active.session.clone();
            session.ended_at = Some(active.last_seen);
            session.end_reason = Some(if now - active.last_seen > 20 { "interrupted" } else { "exited" }.into());
            stats.total_seconds += session.seconds;
            stats.sessions.push(session);
            stats.active = None;
            return;
        }
    }
    if let Some(active) = stats.active.as_mut() { active.last_tick = tick; }
}

pub(crate) fn start_monitor(app: tauri::AppHandle) {
    let recovery = (|| -> Result<(), String> {
        let state = app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let entries = load_all(&conn)?;
        let current = if entries.iter().any(|p| p.active.is_some()) { processes()? } else { Vec::new() };
        for mut stats in entries {
            if let Some(active) = stats.active.as_mut() {
                if current.iter().any(|p| active.processes.contains(&p.id)
                    || (p.id.created == 0 && active.processes.iter().any(|known| known.pid == p.id.pid))) {
                    active.session.recovered = true;
                } else {
                    let mut session = active.session.clone();
                    session.ended_at = Some(active.last_seen); session.end_reason = Some("interrupted".into());
                    stats.total_seconds += session.seconds; stats.sessions.push(session); stats.active = None;
                }
                write_json(&conn, &key(&stats.record_id), &stats)?;
            }
        }
        Ok(())
    })();
    if let Err(error) = recovery { eprintln!("本地游玩记录恢复失败：{error}"); }
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let result = (|| -> Result<bool, String> {
            let state = app.state::<AppState>();
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            let mut entries = load_all(&conn)?;
            if !entries.iter().any(|p| p.active.is_some()) && !crate::scaling::is_running() { return Ok(false); }
            let processes = processes()?;
            crate::scaling::sync_processes(&processes);
            let tick = awake_ticks()?;
            let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
            for stats in entries.iter_mut().filter(|p| p.active.is_some()) {
                advance(stats, &processes, tick, chrono::Utc::now().timestamp());
                write_json(&tx, &key(&stats.record_id), stats)?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            Ok(true)
        })();
        match result {
            Ok(true) => { let _ = app.emit("game-activity-updated", ()); }
            Err(error) => { let _ = app.emit("game-activity-error", error); }
            _ => {}
        }
    });
}

/// A restored backup contains historical totals, not live process ownership.
pub(crate) fn settle_restored_sessions(conn: &rusqlite::Connection) -> Result<(), String> {
    for mut stats in load_all(conn)? {
        if let Some(active) = stats.active.take() {
            let mut session = active.session;
            session.ended_at = Some(active.last_seen); session.end_reason = Some("backup_restored".into());
            stats.total_seconds += session.seconds; stats.sessions.push(session);
            write_json(conn, &key(&stats.record_id), &stats)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn awake_ticks() -> Result<u64, String> {
    let mut tick = 0;
    if unsafe { windows_sys::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTime(&mut tick) } == 0 {
        return Err("无法读取系统计时器，已暂停计时".into());
    }
    Ok(tick)
}
#[cfg(windows)]
fn filetime_now() -> u64 {
    let mut time = std::mem::MaybeUninit::uninit();
    unsafe { windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime(time.as_mut_ptr()); filetime(time.assume_init()) }
}
#[cfg(windows)]
fn filetime(time: windows_sys::Win32::Foundation::FILETIME) -> u64 { ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64 }

#[cfg(windows)]
fn identity(handle: windows_sys::Win32::Foundation::HANDLE, pid: u32) -> Option<ProcessId> {
    use windows_sys::Win32::{Foundation::FILETIME, System::Threading::GetProcessTimes};
    let mut created = FILETIME::default(); let mut exit = FILETIME::default(); let mut kernel = FILETIME::default(); let mut user = FILETIME::default();
    (unsafe { GetProcessTimes(handle, &mut created, &mut exit, &mut kernel, &mut user) } != 0)
        .then(|| ProcessId { pid, created: filetime(created) })
}
#[cfg(windows)]
fn child_identity(child: &std::process::Child) -> Result<ProcessId, String> {
    use std::os::windows::io::AsRawHandle;
    identity(child.as_raw_handle(), child.id()).ok_or_else(|| "游戏已启动，但无法取得进程身份，未开启计时".into())
}
#[cfg(windows)]
fn processes() -> Result<Vec<Process>, String> {
    use windows_sys::Win32::{Foundation::{CloseHandle, INVALID_HANDLE_VALUE}, System::{Diagnostics::ToolHelp::*, Threading::*}};
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE { return Err("无法读取进程列表，已暂停计时".into()); }
        let mut entry = PROCESSENTRY32W::default(); entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry);
        let mut result = Vec::new();
        while more != 0 {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
            let before = result.len();
            if !handle.is_null() {
                let mut path = vec![0u16; 32768]; let mut size = path.len() as u32;
                if let Some(id) = identity(handle, entry.th32ProcessID) {
                    if QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut size) != 0 {
                        result.push(Process { id, parent: entry.th32ParentProcessID, path: normalized(Path::new(&String::from_utf16_lossy(&path[..size as usize]))) });
                    }
                }
                CloseHandle(handle);
            }
            if result.len() == before {
                result.push(Process { id: ProcessId { pid: entry.th32ProcessID, created: 0 }, parent: entry.th32ParentProcessID, path: String::new() });
            }
            more = Process32NextW(snapshot, &mut entry);
        }
        CloseHandle(snapshot);
        Ok(result)
    }
}
#[cfg(not(windows))]
fn processes() -> Result<Vec<Process>, String> { Err("本地自动计时目前仅支持 Windows".into()) }
#[cfg(not(windows))]
fn awake_ticks() -> Result<u64, String> { Err("本地自动计时目前仅支持 Windows".into()) }
#[cfg(not(windows))]
fn filetime_now() -> u64 { 0 }
#[cfg(not(windows))]
fn child_identity(_: &std::process::Child) -> Result<ProcessId, String> { Err("不支持的系统".into()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_handoff_sleep_recovery_and_pid_reuse() {
        let launcher = Process { id: ProcessId { pid:1,created:100 },parent:0,path:r"c:\game\launcher.exe".into() };
        let game = Process { id: ProcessId { pid:2,created:110 },parent:1,path:r"c:\game\game.exe".into() };
        let mut stats = LocalPlaytime { record_id:"fixture".into(), active:Some(ActiveSession {
            session:Session {started_at:1000,ended_at:None,seconds:0.0,recovered:false,end_reason:None}, tracking_error:None,
            directory:r"c:\game".into(),processes:vec![launcher.id.clone()],launched_after:99,last_tick:0,
            last_seen:1000,missing_since:None,launcher_pid:Some(1),handed_off:false
        }),..Default::default() };
        advance(&mut stats,&[launcher.clone(),game.clone()],20_000_000,1002);
        // Wall time moves one hour; awake time advances two seconds.
        advance(&mut stats,&[launcher.clone(),game.clone()],40_000_000,4604);
        assert_eq!(stats.active.as_ref().unwrap().session.seconds,4.0);
        // Simulates persistence and app restart while the game stays alive.
        stats=serde_json::from_str(&serde_json::to_string(&stats).unwrap()).unwrap();
        advance(&mut stats,&[launcher.clone(),game],240_000_000,4624);
        assert_eq!(stats.active.as_ref().unwrap().session.seconds,24.0);
        let reused=Process {id:ProcessId{pid:2,created:120},parent:0,path:r"c:\other\other.exe".into()};
        advance(&mut stats,&[launcher.clone(),reused.clone()],260_000_000,4626);
        advance(&mut stats,&[launcher,reused],360_000_000,4636);
        assert!(stats.active.is_none()); assert_eq!(stats.total_seconds,24.0); assert_eq!(stats.sessions.len(),1);
    }

    #[test]
    #[cfg(windows)]
    fn real_process_launch_duplicate_guard_and_exit_settlement() {
        let dir=tempfile::tempdir().unwrap();
        let executable=dir.path().join("WaypointTimerFixture.exe");
        std::fs::copy(std::path::Path::new(&std::env::var("WINDIR").unwrap()).join("System32/where.exe"),&executable).unwrap();
        let conn=rusqlite::Connection::open_in_memory().unwrap();
        crate::init_database_schema(&conn).unwrap();
        conn.execute("INSERT INTO game_records(id,title,category,status,progress) VALUES ('fixture','fixture','single','backlog','')",[]).unwrap();
        launch(&conn,"fixture",dir.path(),&executable).unwrap();
        assert!(launch(&conn,"fixture",dir.path(),&executable).unwrap_err().contains("重复启动"));
        let mut stats:LocalPlaytime=read_json(&conn,&key("fixture")).unwrap();
        let tick=awake_ticks().unwrap();
        // Real executable exits by itself, no user's process is killed.
        std::thread::sleep(std::time::Duration::from_millis(500));
        let live=processes().unwrap();
        let pid=stats.active.as_ref().unwrap().processes[0].clone();
        assert!(!live.iter().any(|p|p.id==pid));
        advance(&mut stats,&live,tick+10_000_000,chrono::Utc::now().timestamp());
        advance(&mut stats,&live,tick+110_000_000,chrono::Utc::now().timestamp());
        assert!(stats.active.is_none());assert_eq!(stats.sessions.len(),1);
        write_json(&conn,&key("fixture"),&stats).unwrap();
        assert!(!is_active(&conn,"fixture").unwrap());
    }
}
