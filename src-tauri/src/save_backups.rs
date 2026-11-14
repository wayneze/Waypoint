//! Per-game snapshots are immutable, encrypted files. Restores stage and validate
//! a whole directory before swapping it, with a fresh snapshot of the old saves.
use std::{collections::HashSet, fs, io::{Read, Write}, path::{Path, PathBuf}};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use ring::{digest, rand::{SecureRandom, SystemRandom}};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use crate::{records::GameRecord, AppState};
use crate::game_saves::{self, no_link, ResolvedSave, SaveKind, SaveSource};
use crate::game_operations::GameOperation;

const MAGIC: &[u8; 8] = b"WPSAVE01";
const MAX_BYTES: usize = 512 * 1024 * 1024;
const MAX_FILES: usize = 20_000;

#[derive(Serialize)]
pub struct SaveVersion { id: String, created_at: i64, bytes: u64, remote: bool }
#[derive(Serialize, Deserialize)]
struct Manifest { record_id: String, title: String, #[serde(default)] sources: Vec<SaveSource> }
#[derive(PartialEq)]
struct FileState { path: PathBuf, size: u64, modified: std::time::SystemTime, directory: bool }
fn game_key(id: &str) -> String { digest::digest(&digest::SHA256, id.as_bytes()).as_ref().iter().map(|b| format!("{b:02x}")).collect() }
fn version_time(id: &str) -> Result<i64, String> {
    let (time, nonce) = id.split_once('-').ok_or("备份版本无效")?;
    if time.len() != 13 || !time.bytes().all(|b| b.is_ascii_digit()) || nonce.len() != 16 || !nonce.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("备份版本无效".into()); }
    time.parse().map_err(|_| "备份时间无效".into())
}
fn new_id() -> Result<String, String> {
    let mut nonce = [0u8; 8];
    SystemRandom::new().fill(&mut nonce).map_err(|_| "生成版本号失败")?;
    Ok(format!("{}-{}", chrono::Utc::now().timestamp_millis(), nonce.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}
fn storage(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(crate::db::db_file_path(app)?.parent().ok_or("数据目录无效")?.join("save-backups").join(game_key(id)))
}
fn package_path(root: &Path, id: &str) -> Result<PathBuf, String> { version_time(id)?; Ok(root.join(format!("{id}.wpsave"))) }
fn cloud_name(record_id: &str, id: &str) -> Result<String, String> { version_time(id)?; Ok(format!("waypoint-save-{}-{id}.wpsave", game_key(record_id))) }
fn record(state: &State<AppState>, id: &str) -> Result<GameRecord, String> {
    crate::records::read_records(&*state.db.lock().map_err(|e| e.to_string())?)?.into_iter().find(|r| r.id == id).ok_or("作品已不存在".into())
}
fn collect(root: &Path) -> Result<Vec<FileState>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut total = 0u64;
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            let meta = no_link(&path)?;
            if !meta.is_file() && !meta.is_dir() { return Err("存档中包含不支持的文件类型".into()); }
            let relative = path.strip_prefix(root).map_err(|e| e.to_string())?.to_path_buf();
            safe_relative(&relative)?;
            if meta.is_dir() { pending.push(path); } else { total = total.saturating_add(meta.len()); }
            files.push(FileState { path: relative, size: if meta.is_dir() { 0 } else { meta.len() }, modified: meta.modified().map_err(|e| e.to_string())?, directory: meta.is_dir() });
            if total > MAX_BYTES as u64 || files.len() > MAX_FILES { return Err("存档超过 512 MB 或 20000 个文件，请缩小存档目录".into()); }
        }
    }
    files.sort_by(|a,b| a.path.cmp(&b.path));
    Ok(files)
}
fn collect_source(source: &ResolvedSave) -> Result<Vec<FileState>, String> {
    if !source.root.exists() { return Ok(Vec::new()); }
    if source.source.kind == SaveKind::Directory { return collect(&source.root); }
    let mut files = Vec::new();
    for entry in fs::read_dir(&source.root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !game_saves::nscripter_file(&entry.file_name().to_string_lossy()) { continue; }
        let meta = no_link(&entry.path())?;
        if !meta.is_file() { return Err("存档文件不是普通文件".into()); }
        files.push(FileState { path: entry.file_name().into(), size: meta.len(), modified: meta.modified().map_err(|e| e.to_string())?, directory: false });
    }
    files.sort_by(|a,b| a.path.cmp(&b.path));
    Ok(files)
}
fn safe_relative(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.components().any(|part| {
        let std::path::Component::Normal(name) = part else { return true };
        let Some(name) = name.to_str() else { return true };
        let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        name.contains([':', '\\']) || name.ends_with(['.', ' ']) || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || ["COM", "LPT"].iter().any(|prefix| base.strip_prefix(prefix).is_some_and(|n| n.len() == 1 && "123456789".contains(n)))
    }) { return Err("备份中包含无效的相对文件路径".into()); }
    Ok(())
}
fn append(archive: &mut tar::Builder<GzEncoder<Vec<u8>>>, path: &Path, bytes: &[u8], directory: bool) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64); header.set_mode(if directory { 0o700 } else { 0o600 });
    header.set_entry_type(if directory { tar::EntryType::Directory } else { tar::EntryType::Regular });
    header.set_cksum();
    archive.append_data(&mut header, path, bytes).map_err(|e| e.to_string())
}
fn snapshot(sources: &[ResolvedSave], record: &GameRecord, password: &str) -> Result<Vec<u8>, String> {
    let before = sources.iter().map(collect_source).collect::<Result<Vec<_>,_>>()?;
    if before.iter().flatten().map(|f| f.size).sum::<u64>() > MAX_BYTES as u64 || before.iter().map(Vec::len).sum::<usize>() > MAX_FILES {
        return Err("存档总计超过 512 MB 或 20000 个文件".into());
    }
    let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    append(&mut archive, Path::new("manifest.json"), &serde_json::to_vec(&Manifest { record_id: record.id.clone(), title: record.title.clone(), sources: sources.iter().map(|s| s.source.clone()).collect() }).map_err(|e| e.to_string())?, false)?;
    for (index, (source, files)) in sources.iter().zip(&before).enumerate() {
      for file in files {
        let mut bytes = Vec::new();
        if !file.directory {
            fs::File::open(source.root.join(&file.path)).map_err(|e| e.to_string())?.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            if bytes.len() as u64 != file.size { return Err("存档正在变化，请退出游戏后重试".into()); }
        }
        append(&mut archive, &PathBuf::from(format!("source-{index}")).join(&file.path), &bytes, file.directory)?;
      }
    }
    if sources.iter().map(collect_source).collect::<Result<Vec<_>,_>>()? != before { return Err("存档正在变化，请退出游戏后重试".into()); }
    let bytes = archive.into_inner().and_then(|e| e.finish()).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES - 64 { return Err("压缩备份超过 512 MB".into()); }
    crate::backup::seal_archive(bytes, password, MAGIC)
}
fn unpack(bytes: &[u8], password: &str, record_id: &str, dest: &Path) -> Result<Vec<SaveSource>, String> {
    let compressed = crate::backup::open_archive(bytes, password, MAGIC)?;
    let mut archive = tar::Archive::new(GzDecoder::new(compressed.as_slice()).take((MAX_BYTES + MAX_FILES * 1024) as u64));
    let mut names = HashSet::new();
    let mut manifest: Option<Manifest> = None;
    let mut total = 0u64;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        let kind = entry.header().entry_type();
        if !names.insert(path.to_string_lossy().to_lowercase()) || names.len() > MAX_FILES + 1 || (!kind.is_file() && !kind.is_dir()) { return Err("备份包含重复条目或不支持的链接".into()); }
        total = total.saturating_add(entry.size());
        if total > MAX_BYTES as u64 { return Err("存档解压后超过 512 MB".into()); }
        if path == Path::new("manifest.json") && kind.is_file() && entry.size() < 65_536 {
            let identity: Manifest = serde_json::from_reader(&mut entry).map_err(|e| e.to_string())?;
            if identity.record_id != record_id { return Err("此备份属于另一部作品，未修改当前存档".into()); }
            let mut slots = HashSet::new();
            if identity.sources.iter().any(|s| s.slot.is_empty() || !slots.insert(&s.slot)) { return Err("备份中的存档位置重复或无效".into()); }
            if manifest.is_some() { return Err("备份的作品信息重复".into()); }
            manifest = Some(identity);
        } else {
            let identity = manifest.as_ref().ok_or("备份缺少前置作品信息")?;
            let mut parts = path.components();
            let prefix = parts.next().ok_or("备份文件路径无效")?.as_os_str().to_string_lossy();
            let source = if identity.sources.is_empty() {
                if prefix != "files" { return Err("备份文件不在存档目录内".into()); }
                None
            } else {
                let index: usize = prefix.strip_prefix("source-").ok_or("备份存档位置无效")?.parse().map_err(|_| "备份存档位置无效")?;
                if prefix != format!("source-{index}") { return Err("备份存档位置无效".into()); }
                Some(identity.sources.get(index).ok_or("备份存档位置越界")?)
            };
            let relative = parts.as_path();
            safe_relative(relative)?;
            if source.is_some_and(|s| s.kind == SaveKind::Nscripter) && (kind.is_dir() || relative.components().count() != 1 || !game_saves::nscripter_file(&relative.to_string_lossy())) { return Err("备份包含存档范围外的文件".into()); }
            let target = dest.join(prefix.as_ref()).join(relative);
            if kind.is_dir() { fs::create_dir_all(&target).map_err(|e| e.to_string())?; }
            else {
                fs::create_dir_all(target.parent().ok_or("文件路径无效")?).map_err(|e| e.to_string())?;
                let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|e| e.to_string())?;
                std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
                file.sync_all().map_err(|e| e.to_string())?;
            }
        }
    }
    let manifest = manifest.ok_or("存档备份缺少作品信息")?;
    Ok(manifest.sources)
}
fn persist(root: &Path, id: &str, bytes: &[u8]) -> Result<SaveVersion, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(bytes).and_then(|_| file.as_file().sync_all()).map_err(|e| e.to_string())?;
    file.persist_noclobber(package_path(root, id)?).map_err(|e| format!("保存备份失败：{e}"))?;
    Ok(SaveVersion { id: id.into(), created_at: version_time(id)?, bytes: bytes.len() as u64, remote: false })
}
fn read_package(root: &Path, id: &str) -> Result<Vec<u8>, String> {
    let path = package_path(root, id)?;
    if !no_link(&path)?.is_file() { return Err("备份不是普通文件".into()); }
    let mut bytes = Vec::new();
    fs::File::open(path).map_err(|e| e.to_string())?.take(MAX_BYTES as u64 + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES { return Err("备份超过 512 MB".into()); }
    Ok(bytes)
}
fn cache_download(root: &Path, id: &str, bytes: &[u8]) -> Result<SaveVersion, String> {
    if package_path(root, id)?.exists() {
        if read_package(root, id)? != bytes { return Err("本地同名备份内容不同，未覆盖；请核对云端版本".into()); }
        return Ok(SaveVersion { id: id.into(), created_at: version_time(id)?, bytes: bytes.len() as u64, remote: false });
    }
    persist(root, id, bytes)
}
fn restore(sources: &[ResolvedSave], storage: &Path, record: &GameRecord, id: &str, password: &str) -> Result<SaveVersion, String> {
    let bytes = read_package(storage, id)?;
    let unpacked = tempfile::tempdir().map_err(|e| e.to_string())?;
    let archived = unpack(&bytes, password, &record.id, unpacked.path())?;
    if (!archived.is_empty() && (archived.len() != sources.len() || sources.iter().any(|s| !archived.iter().any(|a| a.slot == s.source.slot && a.kind == s.source.kind))))
        || (archived.is_empty() && (sources.len() != 1 || sources[0].source.kind != SaveKind::Directory)) {
        return Err("备份与当前存档位置不匹配，请先确认原存档位置；未修改任何文件".into());
    }
    let original_state = sources.iter().map(collect_source).collect::<Result<Vec<_>,_>>()?;
    let mut transactions = Vec::new();
    let mut swaps = Vec::new();
    for source in sources {
        let index = archived.iter().position(|s| s.slot == source.source.slot);
        let packed = unpacked.path().join(index.map(|i| format!("source-{i}")).unwrap_or_else(|| "files".into()));
        let parent = if source.source.kind == SaveKind::Directory { source.root.parent().ok_or("存档目录无效")? } else { &source.root };
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let transaction = tempfile::Builder::new().prefix(".waypoint-restore-").tempdir_in(parent).map_err(|e| e.to_string())?;
        let staged = transaction.path().join("restored");
        fs::create_dir(&staged).map_err(|e| e.to_string())?;
        if packed.exists() {
            for file in collect(&packed)? {
                let target = staged.join(&file.path);
                if file.directory { fs::create_dir_all(&target).map_err(|e| e.to_string())?; }
                else { fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?; fs::copy(packed.join(&file.path), target).map_err(|e| e.to_string())?; }
            }
        }
        if source.source.kind == SaveKind::Directory {
            swaps.push((source.root.clone(), Some(staged), transaction.path().join("previous")));
        } else {
            let mut files: HashSet<PathBuf> = collect_source(source)?.into_iter().map(|f| f.path).collect();
            files.extend(collect(&staged)?.into_iter().map(|f| f.path));
            for (i,file) in files.into_iter().enumerate() {
                let replacement = staged.join(&file);
                swaps.push((source.root.join(file), replacement.exists().then_some(replacement), transaction.path().join(format!("previous-{i}"))));
            }
        }
        transactions.push(transaction);
    }
    let previous = snapshot(sources, record, password)?;
    let before = persist(storage, &new_id()?, &previous)?;
    if sources.iter().map(collect_source).collect::<Result<Vec<_>,_>>()? != original_state { return Err("存档在恢复准备期间发生变化，未替换；请退出游戏后重试".into()); }
    let mut applied = Vec::new();
    for (target, replacement, previous) in &swaps {
        let existed = target.exists();
        let mut moved = false;
        let result = (|| -> Result<(), std::io::Error> {
            if existed { fs::rename(target, previous)?; moved = true; }
            if let Some(replacement) = replacement { fs::rename(replacement, target)?; }
            Ok(())
        })();
        applied.push((target, previous, moved, result.is_ok() && replacement.is_some()));
        if let Err(error) = result {
            let mut rollback_errors = Vec::new();
            for (target, previous, moved, installed) in applied.into_iter().rev() {
                let result = (|| -> Result<(), std::io::Error> {
                    if installed { if target.is_dir() { fs::remove_dir_all(target)?; } else { fs::remove_file(target)?; } }
                    if moved { fs::rename(previous, target)?; }
                    Ok(())
                })();
                if let Err(e) = result { rollback_errors.push(e.to_string()); }
            }
            if !rollback_errors.is_empty() {
                let locations: Vec<_> = transactions.into_iter().map(|t| t.keep().display().to_string()).collect();
                return Err(format!("恢复失败：{error}；回退未完成：{}。原文件保留在 {}，恢复前版本 {}", rollback_errors.join("；"), locations.join("；"), before.id));
            }
            return Err(format!("恢复失败，已回退原存档：{error}"));
        }
    }
    Ok(before)
}
fn prepare(app: &AppHandle, state: &State<AppState>, id: &str, restoring: bool) -> Result<(GameRecord, Vec<ResolvedSave>, PathBuf, GameOperation), String> {
    let record = record(state, id)?;
    let mut operation = GameOperation::start(&record)?;
    crate::playtime::ensure_stopped(&*state.db.lock().map_err(|e| e.to_string())?, &record)?;
    let data = crate::db::db_file_path(app)?.parent().ok_or("数据目录无效")?.to_path_buf();
    let sources = game_saves::resolve(&record, &game_saves::app_folders(app), &data, restoring)?;
    operation.include_paths(sources.iter().map(|s| s.root.as_path()))?;
    Ok((record, sources, storage(app, id)?, operation))
}
#[tauri::command]
pub async fn list_game_save_backups(app: AppHandle, state: State<'_, AppState>, record_id: String, remote: bool) -> Result<Vec<SaveVersion>, String> {
    record(&state, &record_id)?;
    let mut versions = Vec::new();
    if remote {
        let config = crate::webdav::get_config_from_db(&*state.db.lock().map_err(|e| e.to_string())?);
        let prefix = format!("waypoint-save-{}-", game_key(&record_id));
        for name in crate::webdav::list_files(&config).await? {
            if let Some(id) = name.strip_prefix(&prefix).and_then(|s| s.strip_suffix(".wpsave")) {
                if let Ok(created_at) = version_time(id) { versions.push(SaveVersion { id: id.into(), created_at, bytes: 0, remote: true }); }
            }
        }
    } else {
        let folder = storage(&app, &record_id)?;
        if !folder.exists() { return Ok(versions); }
        for file in fs::read_dir(folder).map_err(|e| e.to_string())? {
            let file = file.map_err(|e| e.to_string())?;
            if !file.file_type().map_err(|e| e.to_string())?.is_file() { continue; }
            let name = file.file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".wpsave") {
                if let Ok(created_at) = version_time(id) { versions.push(SaveVersion { id: id.into(), created_at, bytes: file.metadata().map_err(|e| e.to_string())?.len(), remote: false }); }
            }
        }
    }
    versions.sort_by(|a,b| b.created_at.cmp(&a.created_at));
    versions.dedup_by(|a,b| a.id == b.id);
    Ok(versions)
}
#[tauri::command]
pub async fn create_game_save_backup(app: AppHandle, state: State<'_, AppState>, record_id: String, password: String) -> Result<SaveVersion, String> {
    let (record, sources, folder, operation) = prepare(&app, &state, &record_id, false)?;
    tauri::async_runtime::spawn_blocking(move || { let _operation = operation; persist(&folder, &new_id()?, &snapshot(&sources, &record, &password)?) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn restore_game_save_backup(app: AppHandle, state: State<'_, AppState>, record_id: String, id: String, password: String) -> Result<SaveVersion, String> {
    let (record, sources, folder, operation) = prepare(&app, &state, &record_id, true)?;
    tauri::async_runtime::spawn_blocking(move || { let _operation = operation; restore(&sources, &folder, &record, &id, &password) }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn upload_game_save_backup(app: AppHandle, state: State<'_, AppState>, record_id: String, id: String) -> Result<(), String> {
    let _operation = GameOperation::start(&record(&state, &record_id)?)?;
    let config = crate::webdav::get_config_from_db(&*state.db.lock().map_err(|e| e.to_string())?);
    let bytes = read_package(&storage(&app, &record_id)?, &id)?;
    crate::webdav::upload_new_file(&config, &cloud_name(&record_id, &id)?, bytes).await
}
#[tauri::command]
pub async fn download_game_save_backup(app: AppHandle, state: State<'_, AppState>, record_id: String, id: String, password: String) -> Result<SaveVersion, String> {
    let operation = GameOperation::start(&record(&state, &record_id)?)?;
    let config = crate::webdav::get_config_from_db(&*state.db.lock().map_err(|e| e.to_string())?);
    let folder = storage(&app, &record_id)?;
    let bytes = crate::webdav::download_file(&config, &cloud_name(&record_id, &id)?, MAX_BYTES).await?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        unpack(&bytes, &password, &record_id, temp.path())?;
        cache_download(&folder, &id, &bytes)
    }).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources(root: &Path) -> Vec<ResolvedSave> {
        vec![ResolvedSave { source: SaveSource { slot: "test".into(), path: root.to_string_lossy().into_owned(), kind: SaveKind::Directory }, root: root.to_path_buf() }]
    }
    fn game() -> GameRecord { serde_json::from_value(serde_json::json!({"id":"test-game","title":"Test","category":"single","status":"playing","rating":0,"progress":""})).unwrap() }
    #[test]
    fn multiple_sources_restore_only_their_owned_files_and_keep_an_undo_snapshot() {
        let temp=tempfile::tempdir().unwrap();
        let game_root=temp.path().join("game");let docs=temp.path().join("documents-saves");let store=temp.path().join("backups");
        fs::create_dir(&game_root).unwrap();fs::create_dir(&docs).unwrap();
        fs::write(game_root.join("arc.nsa"),b"game assets").unwrap();
        fs::write(game_root.join("save01.dat"),b"first slot").unwrap();
        fs::write(game_root.join("gloval.sav"),b"first flags").unwrap();
        fs::write(game_root.join("NScrllog.dat"),b"first read history").unwrap();
        fs::write(docs.join("progress.dat"),b"first external").unwrap();
        let mut locations=sources(&docs);
        locations.push(ResolvedSave {source:SaveSource {slot:"root-files".into(),path:game_root.to_string_lossy().into_owned(),kind:SaveKind::Nscripter},root:game_root.clone()});
        let first=persist(&store,&new_id().unwrap(),&snapshot(&locations,&game(),"test-password").unwrap()).unwrap();
        fs::write(game_root.join("save01.dat"),b"second slot").unwrap();fs::write(game_root.join("save02.dat"),b"new slot").unwrap();
        fs::write(game_root.join("NScrllog.dat"),b"second read history").unwrap();
        fs::write(docs.join("progress.dat"),b"second external").unwrap();
        let undo=restore(&locations,&store,&game(),&first.id,"test-password").unwrap();
        assert_eq!(fs::read(game_root.join("arc.nsa")).unwrap(),b"game assets");
        assert_eq!(fs::read(game_root.join("save01.dat")).unwrap(),b"first slot");assert!(!game_root.join("save02.dat").exists());
        assert_eq!(fs::read(game_root.join("NScrllog.dat")).unwrap(),b"first read history");
        assert_eq!(fs::read(docs.join("progress.dat")).unwrap(),b"first external");
        restore(&locations,&store,&game(),&undo.id,"test-password").unwrap();
        assert_eq!(fs::read(game_root.join("save02.dat")).unwrap(),b"new slot");
        assert_eq!(fs::read(game_root.join("NScrllog.dat")).unwrap(),b"second read history");
        assert_eq!(fs::read(docs.join("progress.dat")).unwrap(),b"second external");
        let mut wrong=sources(&docs);wrong[0].source.slot="wrong".into();
        assert!(restore(&wrong,&store,&game(),&first.id,"test-password").is_err());
        assert_eq!(fs::read(docs.join("progress.dat")).unwrap(),b"second external");
    }
    #[test]
    fn existing_single_directory_packages_still_restore_through_the_same_boundary() {
        let temp=tempfile::tempdir().unwrap();let root=temp.path().join("saves");let store=temp.path().join("backups");
        fs::create_dir(&root).unwrap();fs::write(root.join("old.sav"),b"current").unwrap();
        let mut archive=tar::Builder::new(GzEncoder::new(Vec::new(),Compression::default()));
        append(&mut archive,Path::new("manifest.json"),br#"{"record_id":"test-game","title":"Test"}"#,false).unwrap();
        append(&mut archive,Path::new("files/old.sav"),b"previous format",false).unwrap();
        let bytes=crate::backup::seal_archive(archive.into_inner().unwrap().finish().unwrap(),"test-password",MAGIC).unwrap();
        let first=persist(&store,&new_id().unwrap(),&bytes).unwrap();
        restore(&sources(&root),&store,&game(),&first.id,"test-password").unwrap();
        assert_eq!(fs::read(root.join("old.sav")).unwrap(),b"previous format");
    }
    #[test]
    fn snapshot_restore_and_undo_preserve_exact_files_and_reject_wrong_password_or_identity() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("saves"); let store = dir.path().join("backups");
        fs::create_dir_all(root.join("子目录")).unwrap();
        fs::write(root.join("子目录/slot.sav"), b"first").unwrap();
        let password = "test-password";
        let bytes = snapshot(&sources(&root), &game(), password).unwrap();
        let first = persist(&store, &new_id().unwrap(), &bytes).unwrap();
        fs::write(root.join("子目录/slot.sav"), b"second").unwrap();
        fs::write(root.join("extra.sav"), b"extra").unwrap();
        assert!(restore(&sources(&root), &store, &game(), &first.id, "wrong").is_err());
        assert_eq!(fs::read(root.join("extra.sav")).unwrap(), b"extra");
        let mut other = game(); other.id = "another".into();
        assert!(restore(&sources(&root), &store, &other, &first.id, password).is_err());
        let undo = restore(&sources(&root), &store, &game(), &first.id, password).unwrap();
        assert_eq!(fs::read(root.join("子目录/slot.sav")).unwrap(), b"first");
        assert!(!root.join("extra.sav").exists());
        restore(&sources(&root), &store, &game(), &undo.id, password).unwrap();
        assert_eq!(fs::read(root.join("子目录/slot.sav")).unwrap(), b"second");
        assert_eq!(fs::read(root.join("extra.sav")).unwrap(), b"extra");
        let temp = tempfile::tempdir().unwrap();
        let mut corrupt = bytes; let index = corrupt.len()-1; corrupt[index] ^= 1;
        assert!(unpack(&corrupt, password, &game().id, temp.path()).is_err());
        for path in ["../escape", "/absolute", "C:/escape", "dir/file:stream", "NUL.txt", "dir/name."] { assert!(safe_relative(Path::new(path)).is_err(), "{path}"); }
        assert!(package_path(&store, "../other").is_err());
        assert!(persist(&store, &first.id, b"overwrite").is_err());
        assert!(cache_download(&store, &first.id, &read_package(&store, &first.id).unwrap()).is_ok());
        assert!(cache_download(&store, &first.id, b"overwrite").is_err());
    }

    #[test]
    fn cloud_versions_publish_list_download_and_decrypt_the_same_snapshot() {
        use std::{io::{BufRead, BufReader}, net::TcpListener, time::Duration};
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("slot.sav"), b"cloud save").unwrap();
        let password = "cloud-test-password";
        let bytes = snapshot(&sources(root.path()), &game(), password).unwrap();
        let id = new_id().unwrap();
        let name = cloud_name(&game().id, &id).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/backups/", listener.local_addr().unwrap());
        let expected = bytes.clone(); let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut staging = String::new();
            for (step, method) in ["HEAD", "PUT", "HEAD", "HEAD", "MOVE", "HEAD", "PROPFIND", "GET"].iter().enumerate() {
                let until = std::time::Instant::now() + Duration::from_secs(8);
                let mut socket = loop {
                    match listener.accept() {
                        Ok((socket, _)) => break socket,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < until => std::thread::sleep(Duration::from_millis(5)),
                        Err(e) => panic!("missing {method}: {e}"),
                    }
                };
                socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut reader = BufReader::new(socket.try_clone().unwrap());
                let mut line = String::new(); reader.read_line(&mut line).unwrap();
                let parts: Vec<_> = line.split_whitespace().collect();
                assert_eq!(parts[0], *method);
                if step == 1 { staging = parts[1].into(); assert!(staging.contains(".upload-")); }
                if matches!(step, 2 | 4) { assert_eq!(parts[1], staging); }
                let mut length = 0;
                loop {
                    let mut header = String::new(); reader.read_line(&mut header).unwrap();
                    if header == "\r\n" { break; }
                    if let Some((key, value)) = header.split_once(':') {
                        if key.eq_ignore_ascii_case("content-length") { length = value.trim().parse::<usize>().unwrap(); }
                    }
                }
                let mut body = vec![0; length]; reader.read_exact(&mut body).unwrap();
                if step == 1 { assert_eq!(body, expected); }
                let xml = format!("<d:multistatus xmlns:d=\"DAV:\"><d:response><d:href>/backups/{server_name}</d:href></d:response><d:response><d:href>/elsewhere/fake.wpsave</d:href></d:response></d:multistatus>");
                let (status, data): (u16, &[u8]) = match step { 0 | 3 => (404, b""), 1 | 4 => (201, b""), 6 => (207, xml.as_bytes()), 7 => (200, &expected), _ => (200, b"") };
                let size = if matches!(step, 2 | 5) { expected.len() } else { data.len() };
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Length: {size}\r\nETag: \"save-version\"\r\nConnection: close\r\n\r\n").unwrap();
                socket.write_all(data).unwrap();
            }
        });
        let downloaded = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
            let config = (url, "test-user".into(), "test-pass".into());
            crate::webdav::upload_new_file(&config, &name, bytes.clone()).await.unwrap();
            assert_eq!(crate::webdav::list_files(&config).await.unwrap(), vec![name.clone()]);
            crate::webdav::download_file(&config, &name, MAX_BYTES).await.unwrap()
        });
        server.join().unwrap();
        assert_eq!(downloaded, bytes);
        let target = tempfile::tempdir().unwrap();
        unpack(&downloaded, password, &game().id, target.path()).unwrap();
        assert_eq!(fs::read(target.path().join("source-0/slot.sav")).unwrap(), b"cloud save");
    }
}
