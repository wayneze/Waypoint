use std::path::{Path, PathBuf};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::State;
use crate::{AppState, records::read_records};

const REMOVED_GUIDES: &str = "waypoint_removed_guides";

#[derive(Clone, Deserialize)]
pub struct RemovalTarget {
    record_id: Option<String>,
    guide_id: String,
    guide_path: String,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct RemovalFile { path: String, kind: String, blocked: Option<String> }
#[derive(Serialize)]
pub struct RemovalPlan { title: String, files: Vec<RemovalFile> }

fn setting(conn: &Connection, key: &str) -> Result<Value, String> {
    let value: Option<String> = conn.query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r|r.get(0)).optional().map_err(|e|e.to_string())?;
    value.map(|s| serde_json::from_str(&s).map_err(|e|format!("设置 {key} 无法读取：{e}"))).unwrap_or(Ok(Value::Null))
}
fn put(conn: &Connection, key: &str, value: &Value) -> Result<(), String> {
    conn.execute("INSERT OR REPLACE INTO app_settings(key,value) VALUES (?1,?2)", rusqlite::params![key, value.to_string()]).map(|_|()).map_err(|e|e.to_string())
}
fn canonical(path: &str) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|e|format!("无法读取路径 {path}：{e}"))
}
fn path_text(path: &Path) -> String { path.to_string_lossy().trim_start_matches(r"\\?\").to_string() }
fn overlaps(a: &Path, b: &Path) -> bool { a.starts_with(b) || b.starts_with(a) }

fn plan(conn: &Connection, target: &RemovalTarget) -> Result<RemovalPlan, String> {
    let records = read_records(conn)?;
    let record = target.record_id.as_ref().map(|id| records.iter().find(|r| &r.id == id).ok_or("作品已不存在，请刷新书库")).transpose()?;
    let vault: Option<String> = conn.query_row("SELECT value FROM app_settings WHERE key='obsidian_vault_path'", [], |r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let mut title = record.map(|r|r.title.clone()).unwrap_or_default();
    let mut files = Vec::new();
    let mut guide_names = Vec::new();
    if !target.guide_path.is_empty() {
        let base = PathBuf::from(vault.as_deref().ok_or("攻略库未配置")?);
        let raw = PathBuf::from(&target.guide_path);
        let relative = raw.strip_prefix(&base).map_err(|_| "攻略不在当前笔记库中")?;
        if relative.components().any(|part| !matches!(part, std::path::Component::Normal(_))) {
            return Err("攻略路径包含无效层级".into());
        }
        let expected = format!("guide_{}", relative.to_string_lossy().replace(['/', '\\', ' '], "_").trim_end_matches(".md"));
        if expected != target.guide_id { return Err("攻略身份已变化，请重新打开删除窗口".into()); }
        let resolved = canonical(&target.guide_path).and_then(|path| {
            let root = canonical(&path_text(&base))?;
            if !path.starts_with(&root) { return Err("攻略链接指向笔记库之外，不能删除文件".into()); }
            Ok((path, root))
        });
        match resolved {
            Ok((path, root)) => {
                if let Some(guide) = crate::guides::parse_guide_file(&path, &root) {
                    if title.is_empty() { title = guide.title.clone(); }
                    guide_names.extend([guide.title, guide.original_title.unwrap_or_default()]);
                    guide_names.extend(guide.aliases);
                }
                files.push(RemovalFile { path: path_text(&path), kind: "guide".into(), blocked: None });
            },
            Err(error) => files.push(RemovalFile { path: target.guide_path.clone(), kind: "guide".into(), blocked: Some(error) }),
        }
    } else if record.is_none() { return Err("没有可移除的作品".into()); }
    else if target.record_id.as_deref() != Some(&target.guide_id) { return Err("作品身份不匹配".into()); }
    // Also protect legacy records whose guide is matched by name, before an explicit association is saved.
    let normalize = |name: &str| name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let guide_keys: Vec<_> = guide_names.iter().map(|s| normalize(s)).filter(|s| s.chars().count() >= 3).collect();
    let shared_guide = records.iter().filter(|r| Some(&r.id) != target.record_id.as_ref()).any(|r| {
        if r.guide_id.as_deref() == Some(&target.guide_id) { return true; }
        if r.guide_id.is_some() { return false; }
        let meta = r.library.as_ref().and_then(|l| l.metadata.as_ref());
        [Some(r.title.as_str()), meta.map(|m| m.title.as_str()), meta.map(|m| m.original_title.as_str())]
            .into_iter().flatten().map(normalize).any(|name| guide_keys.iter().any(|key| name.contains(key)))
    });
    if shared_guide { for file in &mut files { file.blocked = Some("其他作品也关联此攻略".into()); } }
    let ids: Vec<_> = [Some(target.guide_id.as_str()), target.record_id.as_deref()].into_iter().flatten().collect();
    let links = setting(conn, "waypoint_game_links")?;
    let mut protected: Vec<PathBuf> = ["USERPROFILE", "WINDIR", "ProgramFiles", "ProgramFiles(x86)", "APPDATA", "LOCALAPPDATA"]
        .iter().filter_map(|key|std::env::var(key).ok()).filter_map(|s|canonical(&s).ok()).collect();
    let system: Vec<PathBuf> = ["WINDIR"]
        .iter().filter_map(|key|std::env::var(key).ok()).filter_map(|s|canonical(&s).ok()).collect();
    if let Some(root) = vault.and_then(|v|canonical(&v).ok()) { protected.push(root); }
    if let Some(library) = record.and_then(|r|r.library.as_ref()) {
        for installation in &library.installations {
            if !Path::new(&installation.directory).exists() { continue; }
            let path = match canonical(&installation.directory) {
                Ok(path) => path,
                Err(error) => { files.push(RemovalFile { path: installation.directory.clone(), kind: "game".into(), blocked: Some(error) }); continue; }
            };
            let executable = canonical(&installation.executable).ok();
            let blocked = if path.parent().is_none() || protected.iter().any(|root|root.starts_with(&path)) || system.iter().any(|root|path.starts_with(root)) {
                Some("这是系统、用户或笔记库目录，不能作为游戏删除".into())
            } else if executable.is_none_or(|exe| !exe.starts_with(&path) || exe == path) {
                Some("启动程序不在该游戏目录内，无法确认删除边界".into())
            } else if records.iter().filter(|r|Some(&r.id) != target.record_id.as_ref()).flat_map(|r|r.library.iter()).flat_map(|l|&l.installations)
                .filter_map(|i|canonical(&i.directory).ok()).any(|other|overlaps(&path,&other)) {
                Some("此目录与其他作品共用，不能整体删除".into())
            } else { None };
            files.push(RemovalFile { path:path_text(&path), kind:"game".into(), blocked });
        }
    }
    for id in &ids {
        for entry in links.get(*id).and_then(Value::as_array).into_iter().flatten() {
            if let Some(raw) = entry.get("markdown_path").and_then(Value::as_str).filter(|p|Path::new(p).exists()) {
                let path = match canonical(raw) {
                    Ok(path) => path,
                    Err(error) => { files.push(RemovalFile { path: raw.into(), kind: "guide".into(), blocked: Some(error) }); continue; }
                };
                if path.extension().is_none_or(|ext|!ext.eq_ignore_ascii_case("md")) { continue; }
                let shared = links.as_object().into_iter().flatten().filter(|(owner,_)|!ids.contains(&owner.as_str()))
                    .flat_map(|(_,entries)|entries.as_array().into_iter().flatten())
                    .filter_map(|l|l.get("markdown_path").and_then(Value::as_str)).filter_map(|p|canonical(p).ok()).any(|p|p==path);
                files.push(RemovalFile { path:path_text(&path), kind:"guide".into(), blocked:(shared || shared_guide).then(||"其他作品也关联此攻略".into()) });
            }
        }
    }
    for file in &mut files {
        if file.kind != "game" || file.blocked.is_some() { continue; }
        let directory = Path::new(&file.path);
        let shared = links.as_object().into_iter().flatten().filter(|(owner,_)|!ids.contains(&owner.as_str()))
            .flat_map(|(_,entries)|entries.as_array().into_iter().flatten())
            .filter_map(|entry|entry.get("markdown_path").and_then(Value::as_str))
            .filter_map(|path|canonical(path).ok()).any(|path|Path::new(&path_text(&path)).starts_with(directory));
        if shared { file.blocked = Some("此目录内含其他作品关联的攻略".into()); }
    }
    files.sort_by(|a,b|a.path.cmp(&b.path));
    files.dedup_by(|a,b| {
        if a.path != b.path || a.kind != b.kind { return false; }
        if a.blocked.is_some() { b.blocked = a.blocked.clone(); }
        true
    });
    Ok(RemovalPlan { title, files })
}

#[tauri::command]
pub fn preview_library_removal(target: RemovalTarget, state: State<AppState>) -> Result<RemovalPlan, String> {
    let conn = state.db.lock().map_err(|e|e.to_string())?;
    plan(&conn, &target)
}

fn recycle(paths: &[PathBuf]) -> Result<(), String> {
    if paths.is_empty() { return Ok(()); }
    #[cfg(windows)]
    {
        use windows::{core::{HSTRING, PCWSTR}, Win32::{System::Com::*, UI::Shell::*}};
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().map_err(|e|e.to_string())?;
            let result = (|| -> windows::core::Result<()> {
                let operation: IFileOperation = CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER)?;
                operation.SetOperationFlags(FOFX_RECYCLEONDELETE | FOFX_ADDUNDORECORD | FOFX_EARLYFAILURE | FOF_WANTNUKEWARNING | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT)?;
                for path in paths {
                    let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(HSTRING::from(path_text(path)).as_ptr()), None)?;
                    operation.DeleteItem(&item, None)?;
                }
                operation.PerformOperations()?;
                if operation.GetAnyOperationsAborted()?.as_bool() { return Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x800704C7u32 as i32))); }
                Ok(())
            })();
            CoUninitialize();
            result.map_err(|e|format!("移入回收站未完成：{e}"))
        }
    }
    #[cfg(not(windows))]
    { Err("当前平台不支持移入回收站".into()) }
}

fn remove(conn: &mut Connection, target: &RemovalTarget, selected: &[RemovalFile]) -> Result<(), String> {
    if let Some(id) = &target.record_id {
        if crate::playtime::is_active(conn, id)? { return Err("游戏仍在运行，请退出游戏并等待计时结算后再删除作品".into()); }
    }
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let current = plan(&tx,target)?;
    // Recheck against current ownership; the UI cannot nominate arbitrary files.
    for file in selected {
        if file.blocked.is_some() || !current.files.contains(file) { return Err("删除范围已变化或包含共用目录，请重新打开确认窗口".into()); }
    }
    let mut paths: Vec<PathBuf> = selected.iter().map(|f|canonical(&f.path)).collect::<Result<_,_>>()?;
    // A guide inside a selected game directory is moved once with its parent.
    paths.retain(|path| !selected.iter().filter(|f|f.kind=="game").filter_map(|f|canonical(&f.path).ok()).any(|dir|dir!=*path && path.starts_with(dir)));
    recycle(&paths).map_err(|error|format!("{error}。作品记录保留；已移走的文件可在回收站恢复。"))?;
    if paths.iter().any(|p|p.exists()) { return Err("部分文件仍在原位置，作品记录保留，请检查文件占用".into()); }
    let ids: Vec<_> = [Some(target.guide_id.as_str()), target.record_id.as_deref()].into_iter().flatten().collect();
    {
        let mut removed = setting(&tx, REMOVED_GUIDES)?.as_array().cloned().unwrap_or_default();
        for id in &ids { if !removed.contains(&json!(id)) { removed.push(json!(id)); } }
        put(&tx, REMOVED_GUIDES, &json!(removed))?;
    }
    if let Some(id) = &target.record_id {
        tx.execute("DELETE FROM game_records WHERE id=?1", [id]).map_err(|e|e.to_string())?;
        for prefix in ["waypoint_game_cover_", "waypoint_game_details_", crate::steam::LINK_PREFIX, crate::playtime::PLAY_PREFIX] {
            tx.execute("DELETE FROM app_settings WHERE key=?1", [format!("{prefix}{id}")]).map_err(|e|e.to_string())?;
        }
    }
    for key in ["waypoint_game_links", "waypoint_game_recent", "waypoint_guide_route_focus"] {
        let mut value = setting(&tx, key)?;
        if let Some(map) = value.as_object_mut() { for id in &ids { map.remove(*id); } put(&tx,key,&value)?; }
    }
    let mut pinned = setting(&tx,"waypoint_pinned_games")?;
    if let Some(list) = pinned.as_array_mut() { list.retain(|v|v.as_str().is_none_or(|v|!ids.contains(&v))); put(&tx,"waypoint_pinned_games",&pinned)?; }
    for id in ids {
        tx.execute("DELETE FROM progress_items WHERE scope IN ('guide.routes','guide.progress','guide.status') AND (item_id=?1 OR substr(item_id,1,length(?2))=?2)",
            rusqlite::params![id, format!("resource:{id}:")]).map_err(|e|e.to_string())?;
    }
    tx.commit().map_err(|e|format!("文件处理后，书库记录未能保存：{e}"))
}

#[tauri::command]
pub async fn remove_library_game(app: tauri::AppHandle, target: RemovalTarget, selected: Vec<RemovalFile>) -> Result<(), String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let mut conn = state.db.lock().map_err(|e|e.to_string())?;
        remove(&mut conn,&target,&selected)
    }).await.map_err(|e|e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, Connection, RemovalTarget, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let vault = temp.path().join("vault");
        let game = temp.path().join("game");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::create_dir_all(&game).unwrap();
        let guide = vault.join("Removal fixture.md");
        std::fs::write(&guide, "---\ntitle: Removal fixture\ncategory: galgame\n---\n# Test guide").unwrap();
        let executable = game.join("test.exe");
        std::fs::write(&executable, "test fixture, not executable").unwrap();
        let conn = Connection::open_in_memory().unwrap();
        crate::init_database(&conn).unwrap();
        conn.execute("INSERT INTO game_records(id,title,category,status,progress) VALUES ('fixture','Removal fixture','galgame','backlog','')", []).unwrap();
        conn.execute("INSERT INTO app_settings(key,value) VALUES ('obsidian_vault_path',?1)", [path_text(&vault)]).unwrap();
        let target = RemovalTarget { record_id:Some("fixture".into()), guide_id:"guide_Removal_fixture".into(), guide_path:path_text(&guide) };
        put(&conn, "waypoint_game_details_fixture", &json!({"guide_id": target.guide_id, "library":{"installations":[{"root":path_text(temp.path()),"directory":path_text(&game),"executable":path_text(&executable)}]}})).unwrap();
        put(&conn, "waypoint_game_links", &json!({"fixture":[{"markdown_path":path_text(&guide)}], "keep":[{"url":"https://example.com/"}]})).unwrap();
        put(&conn, "waypoint_pinned_games", &json!(["fixture",target.guide_id,"keep"])).unwrap();
        conn.execute("INSERT INTO progress_items(scope,character_id,item_id,state,updated_at) VALUES ('guide.progress','','fixture','{}','now'),('guide.progress','','keep','{}','now')", []).unwrap();
        (temp,conn,target,game,guide)
    }

    #[test]
    fn library_only_keeps_files_and_excludes_removed_guide_even_when_file_is_missing() {
        let (_temp,mut conn,target,game,guide) = fixture();
        remove(&mut conn,&target,&[]).unwrap();
        assert!(game.exists() && guide.exists());
        assert!(read_records(&conn).unwrap().is_empty());
        assert_eq!(setting(&conn, REMOVED_GUIDES).unwrap(), json!([target.guide_id,"fixture"]));
        assert!(setting(&conn,"waypoint_game_links").unwrap().get("keep").is_some());
        assert_eq!(setting(&conn,"waypoint_pinned_games").unwrap(), json!(["keep"]));
        assert_eq!(conn.query_row("SELECT count(*) FROM progress_items", [], |r|r.get::<_,i64>(0)).unwrap(),1);

        let (_temp,mut conn,target,_,guide) = fixture();
        std::fs::remove_file(guide).unwrap();
        assert!(plan(&conn,&target).unwrap().files.iter().any(|f|f.kind=="guide" && f.blocked.is_some()));
        remove(&mut conn,&target,&[]).unwrap();
        assert!(read_records(&conn).unwrap().is_empty());
    }

    #[test]
    fn stale_or_arbitrary_file_selections_cannot_remove_records_or_shared_files() {
        let (_temp,mut conn,target,game,guide) = fixture();
        let selected = plan(&conn,&target).unwrap().files;
        conn.execute("INSERT INTO game_records(id,title,category,status,progress) VALUES ('other','Other','galgame','backlog','')", []).unwrap();
        put(&conn,"waypoint_game_details_other",&json!({"guide_id":target.guide_id,"library":{"installations":[{"root":path_text(&game),"directory":path_text(&game),"executable":path_text(&game.join("test.exe"))}]}})).unwrap();
        assert!(plan(&conn,&target).unwrap().files.iter().all(|f|f.blocked.is_some()));
        assert!(remove(&mut conn,&target,&selected).is_err());
        assert!(game.exists() && guide.exists());
        assert_eq!(read_records(&conn).unwrap().len(),2);
        let rogue = RemovalFile { path:path_text(&_temp.path().join("unrelated")),kind:"game".into(),blocked:None };
        assert!(remove(&mut conn,&target,&[rogue]).is_err());
    }

    #[test]
    #[ignore = "Windows Recycle Bin acceptance, creates and removes only temporary fixtures"]
    fn selected_game_and_guide_files_are_independent_and_recycled() {
        for kinds in [vec!["guide"],vec!["game"],vec!["guide","game"]] {
            let (_temp,mut conn,target,game,guide) = fixture();
            let selected: Vec<_> = plan(&conn,&target).unwrap().files.into_iter().filter(|f|kinds.contains(&f.kind.as_str())).collect();
            remove(&mut conn,&target,&selected).unwrap();
            assert_eq!(game.exists(),!kinds.contains(&"game"));
            assert_eq!(guide.exists(),!kinds.contains(&"guide"));
            assert!(read_records(&conn).unwrap().is_empty());
            println!("recycle acceptance selected={kinds:?}, game_exists={}, guide_exists={}",game.exists(),guide.exists());
        }
    }
}
