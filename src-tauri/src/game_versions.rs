//! Known resource switches use reversible file moves; distribution scripts are never executed.
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};
use serde::Serialize;
use tauri::State;
use crate::{game_operations::GameOperation, game_saves::no_link, AppState};

#[derive(Debug, Serialize)]
pub struct ResourceSwitch {
    pub id: String, pub label: String, pub current: Option<String>,
    pub original: String, pub modified: String, pub files: Vec<String>,
}
#[derive(Default, Serialize)]
pub struct GameVersions { pub switches: Vec<ResourceSwitch>, pub tools: Vec<String> }
fn scripts(root: &Path) -> Vec<(String,String)> {
    [root.to_path_buf(), root.join("_说明")].into_iter().flat_map(|dir| {
        fs::read_dir(dir).into_iter().flatten().filter_map(Result::ok).filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.contains("切换") || !name.to_lowercase().ends_with(".bat") || !e.metadata().is_ok_and(|m| m.len() <= 65_536) { return None; }
            let bytes = fs::read(e.path()).ok()?;
            let commands = String::from_utf8_lossy(&bytes).to_lowercase().replace([' ', '\t', '\r'], "");
            Some((e.path().display().to_string(), commands))
        }).collect::<Vec<_>>()
    }).collect()
}
fn paired_state(root: &Path, original: &[&str], modified: &[&str]) -> Option<String> {
    let originals = original.iter().all(|p| root.join(p).exists());
    let modifieds = modified.iter().all(|p| root.join(p).exists());
    if originals && !modified.iter().any(|p| root.join(p).exists()) { Some("original".into()) }
    else if modifieds && !original.iter().any(|p| root.join(p).exists()) { Some("modified".into()) }
    else { None }
}
pub(crate) fn inspect(root: &Path) -> Result<GameVersions, String> {
    no_link(root)?;
    let scripts = scripts(root);
    let mut out = GameVersions { switches: Vec::new(), tools: scripts.iter().map(|(p,_)| p.clone()).collect() };
    let commands = scripts.iter().map(|(_,s)| s.as_str()).collect::<Vec<_>>().join("\n");
    if commands.contains("renarc.nsa") && root.join("nscript.dat").is_file() {
        let current = paired_state(root, &["arc.nsa.m","arc1.nsa.m","arc2.nsa.m"], &["arc.nsa.Y","arc1.nsa.Y","arc2.nsa.Y"])
            .filter(|_| ["arc.nsa","arc1.nsa","arc2.nsa"].iter().all(|p| root.join(p).is_file()));
        out.switches.push(ResourceSwitch { id:"nsa-art".into(), label:"画面风格".into(), current, original:"原版".into(), modified:"萌化".into(),
            files:vec!["arc.nsa".into(),"arc1.nsa".into(),"arc2.nsa".into()] });
    }
    if commands.contains("movebmp_bakbmp") && commands.contains("movebmp2_bakbmp2") {
        out.switches.push(ResourceSwitch { id:"bmp-art".into(), label:"画面风格".into(),
            current:paired_state(root, &["bmp_bak","bmp2_bak"], &["bmp","bmp2"]), original:"原版".into(), modified:"萌化".into(), files:vec!["bmp".into(),"bmp2".into()] });
    }
    if commands.contains("gamedata\\data10.pack") && root.join("无码补丁备份/data10.pack").is_file() {
        let active = root.join("GameData/data10.pack").is_file();
        let held = root.join("GameData/data10.pack.waypoint-disabled").exists();
        out.switches.push(ResourceSwitch { id:"pack-patch".into(), label:"内容补丁".into(),
            current:if active && !held { Some("modified".into()) } else if !active { Some("original".into()) } else { None },
            original:"官方画面".into(), modified:"民间步兵补丁".into(), files:vec!["GameData/data10.pack".into()] });
    }
    Ok(out)
}
fn move_files(steps: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    // Reject static conflicts before moving the first file. Destinations that are
    // earlier sources are intentional swap steps (e.g. .m -> active after active -> .Y).
    for (index,(from,to)) in steps.iter().enumerate() {
        if !steps[..index].iter().any(|(_,dest)| dest == from) { no_link(from)?; }
        if to.exists() && !steps[..index].iter().any(|(source,_)| source == to) { return Err(format!("目标已存在，未覆盖：{}",to.display())); }
    }
    let mut completed: Vec<(PathBuf,PathBuf)> = Vec::new();
    for (from,to) in steps {
        let result = (|| -> Result<(), String> {
            no_link(from)?;
            if to.exists() { return Err(format!("目标已存在，未覆盖：{}", to.display())); }
            fs::rename(from,to).map_err(|e| e.to_string())
        })();
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            for (from,to) in completed.into_iter().rev() {
                if from.exists() { rollback_errors.push(format!("原位置被占用：{}", from.display())); }
                else if let Err(e) = fs::rename(to,from) { rollback_errors.push(e.to_string()); }
            }
            return Err(if rollback_errors.is_empty() { format!("切换失败，已回退：{error}") }
                else { format!("切换失败：{error}；回退未完成，文件保留在游戏目录，请检查：{}", rollback_errors.join("；")) });
        }
        completed.push((from.clone(),to.clone()));
    }
    Ok(())
}
pub(crate) fn apply(root: &Path, id: &str, target: &str) -> Result<GameVersions,String> {
    if !matches!(target,"original"|"modified") { return Err("切换选项无效".into()); }
    let state = inspect(root)?;
    let switch = state.switches.iter().find(|s| s.id == id).ok_or("未找到已适配的切换方案")?;
    let current = switch.current.as_deref().ok_or("资源文件状态不完整，请先检查游戏目录；未进行切换")?;
    if current == target { return Ok(state); }
    // Check every affected ancestor, including resources nested below the install root.
    for name in &switch.files {
        for ancestor in root.join(name).ancestors().filter(|p| p.exists()) { no_link(ancestor)?; }
    }
    match id {
        "nsa-art" => {
            let (incoming,outgoing) = if target == "modified" { ("m","Y") } else { ("Y","m") };
            let mut steps = Vec::new();
            for file in &switch.files {
                steps.push((root.join(file),root.join(format!("{file}.{outgoing}"))));
                steps.push((root.join(format!("{file}.{incoming}")),root.join(file)));
            }
            move_files(&steps)?;
        }
        "bmp-art" => {
            let steps = switch.files.iter().map(|file| if target == "modified" {
                (root.join(format!("{file}_bak")),root.join(file))
            } else { (root.join(file),root.join(format!("{file}_bak"))) }).collect::<Vec<_>>();
            move_files(&steps)?;
        }
        "pack-patch" => {
            let active = root.join("GameData/data10.pack");
            let held = root.join("GameData/data10.pack.waypoint-disabled");
            if target == "original" { move_files(&[(active,held)])?; }
            else if held.exists() { move_files(&[(held,active)])?; }
            else {
                let source = root.join("无码补丁备份/data10.pack");
                no_link(&source)?;
                let before = fs::metadata(&source).map_err(|e| e.to_string())?;
                let mut staged = tempfile::NamedTempFile::new_in(active.parent().ok_or("资源目录无效")?).map_err(|e| e.to_string())?;
                let copied = std::io::copy(&mut fs::File::open(&source).map_err(|e| e.to_string())?, &mut staged).map_err(|e| e.to_string())?;
                staged.flush().and_then(|_| staged.as_file().sync_all()).map_err(|e| e.to_string())?;
                let after = fs::metadata(&source).map_err(|e| e.to_string())?;
                if copied != before.len() || before.len() != after.len() || before.modified().ok() != after.modified().ok() { return Err("源补丁在复制期间发生变化，未启用".into()); }
                // Read back the staged copy before publishing it on a network share.
                let hash = |p: &Path| -> Result<ring::digest::Digest,String> {
                    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256); let mut file = fs::File::open(p).map_err(|e| e.to_string())?; let mut buf=[0u8;65536];
                    loop { let n=file.read(&mut buf).map_err(|e| e.to_string())?; if n==0 { break; } ctx.update(&buf[..n]); }
                    Ok(ctx.finish())
                };
                if hash(&source)?.as_ref() != hash(staged.path())?.as_ref() { return Err("补丁复制校验失败，未启用".into()); }
                staged.persist_noclobber(&active).map_err(|e| e.to_string())?;
            }
        }
        _ => return Err("切换方案无效".into()),
    }
    let after = inspect(root)?;
    if after.switches.iter().find(|s| s.id == id).and_then(|s| s.current.as_deref()) != Some(target) { return Err("切换后的文件状态未通过核验，请检查游戏目录".into()); }
    Ok(after)
}
#[tauri::command]
pub async fn inspect_game_versions(state: State<'_, AppState>, record_id: String, installation_index: usize) -> Result<GameVersions,String> {
    let record = crate::records::read_records(&*state.db.lock().map_err(|e| e.to_string())?)?.into_iter().find(|r| r.id==record_id).ok_or("作品已不存在")?;
    let root = record.library.and_then(|l| l.installations.get(installation_index).map(|i| PathBuf::from(&i.directory))).ok_or("安装已不存在")?;
    tauri::async_runtime::spawn_blocking(move || inspect(&root)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn switch_game_resources(state: State<'_, AppState>, record_id: String, installation_index: usize, switch_id: String, target: String) -> Result<GameVersions,String> {
    let record = crate::records::read_records(&*state.db.lock().map_err(|e| e.to_string())?)?.into_iter().find(|r| r.id==record_id).ok_or("作品已不存在")?;
    let operation = GameOperation::start(&record)?;
    crate::playtime::ensure_stopped(&*state.db.lock().map_err(|e| e.to_string())?, &record)?;
    let root = record.library.and_then(|l| l.installations.get(installation_index).map(|i| PathBuf::from(&i.directory))).ok_or("安装已不存在")?;
    tauri::async_runtime::spawn_blocking(move || { let _operation=operation; apply(&root,&switch_id,&target) }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nsa_and_pack_switches_round_trip_without_executing_scripts_or_losing_originals() {
        let temp=tempfile::tempdir().unwrap();let root=temp.path();
        fs::write(root.join("原版萌化切换.bat"),b"ren arc.nsa arc.nsa.Y\n").unwrap();
        fs::write(root.join("nscript.dat"),b"script").unwrap();
        for file in ["arc.nsa","arc1.nsa","arc2.nsa"] {
            fs::write(root.join(file),b"original").unwrap();
            fs::write(root.join(format!("{file}.m")),b"modified").unwrap();
        }
        assert_eq!(inspect(root).unwrap().switches[0].current.as_deref(),Some("original"));
        apply(root,"nsa-art","modified").unwrap();
        assert_eq!(fs::read(root.join("arc1.nsa")).unwrap(),b"modified");
        apply(root,"nsa-art","original").unwrap();
        assert_eq!(fs::read(root.join("arc1.nsa")).unwrap(),b"original");
        fs::create_dir(root.join("GameData")).unwrap();fs::create_dir(root.join("无码补丁备份")).unwrap();
        fs::write(root.join("无码补丁备份/data10.pack"),b"patch").unwrap();
        fs::write(root.join("内容切换.bat"),b"del GameData\\data10.pack\n").unwrap();
        apply(root,"pack-patch","modified").unwrap();
        apply(root,"pack-patch","original").unwrap();
        assert!(!root.join("GameData/data10.pack").exists());
        assert_eq!(fs::read(root.join("GameData/data10.pack.waypoint-disabled")).unwrap(),b"patch");
        apply(root,"pack-patch","modified").unwrap();
        assert_eq!(fs::read(root.join("GameData/data10.pack")).unwrap(),b"patch");
    }
    #[test]
    fn resource_moves_roll_back_a_partial_failure_and_refuse_ambiguous_state() {
        let temp=tempfile::tempdir().unwrap();let root=temp.path();
        fs::write(root.join("a"),b"original").unwrap();
        fs::create_dir(root.join("directory")).unwrap();
        // The second rename fails after the first succeeds: a directory cannot move inside itself.
        assert!(move_files(&[(root.join("a"),root.join("b")),(root.join("directory"),root.join("directory/inside"))]).is_err());
        assert_eq!(fs::read(root.join("a")).unwrap(),b"original");assert!(!root.join("b").exists());
        fs::write(root.join("画面切换.bat"),b"move bmp_bak bmp\nmove bmp2_bak bmp2\n").unwrap();
        for name in ["bmp","bmp_bak","bmp2_bak"] {fs::create_dir(root.join(name)).unwrap();}
        assert!(apply(root,"bmp-art","modified").is_err());
        assert!(root.join("bmp_bak").exists());
        fs::remove_dir(root.join("bmp")).unwrap();
        apply(root,"bmp-art","modified").unwrap();
        apply(root,"bmp-art","original").unwrap();
        assert!(root.join("bmp_bak").exists() && root.join("bmp2_bak").exists());
    }
}
