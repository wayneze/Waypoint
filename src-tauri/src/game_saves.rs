//! Save locations are resolved here for both individual and batch backups.
use std::{collections::HashSet, fs, path::{Path, PathBuf}};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use crate::{records::GameRecord, AppState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveKind { Directory, Nscripter }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveSource { pub slot: String, pub path: String, pub kind: SaveKind }
#[derive(Debug, Serialize)]
pub struct SaveCandidate {
    pub source: SaveSource, pub reason: String, pub status: String, pub automatic: bool,
}
#[derive(Debug, Default, Serialize)]
pub struct SaveDiscovery { pub candidates: Vec<SaveCandidate>, pub warnings: Vec<String> }
pub struct ResolvedSave { pub source: SaveSource, pub root: PathBuf }

pub(crate) fn normalized(path: &Path) -> String {
    let path = path.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    if let Some(network) = path.strip_prefix(r"\\?\unc\") { format!(r"\\{network}") }
    else { path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned() }
}
pub(crate) fn no_link(path: &Path) -> Result<fs::Metadata, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("无法读取 {}：{e}", path.display()))?;
    let mut linked = meta.file_type().is_symlink();
    #[cfg(windows)] { use std::os::windows::fs::MetadataExt; linked |= meta.file_attributes() & 0x400 != 0; }
    if linked { return Err(format!("目录包含链接或联接，不能操作：{}", path.display())); }
    Ok(meta)
}
pub(crate) fn nscripter_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(name.as_str(), "envdata" | "gloval.sav" | "glovaa.sav" | "global.sav" | "nscrllog.dat")
        || name.strip_prefix("save").and_then(|s| s.rsplit_once('.'))
            .is_some_and(|(n, ext)| !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) && matches!(ext, "dat" | "sav" | "ss"))
}
fn config_only(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "savepath.txt" | "savecheck" | "krenvprf.kep" | "systemdata.dat" | "thumbs.db" | "desktop.ini")
}
fn source_status(source: &SaveSource) -> String {
    let root = Path::new(&source.path);
    if !root.exists() {
        return if root.ancestors().last().is_some_and(|p| !p.exists()) { "unavailable".into() } else { "missing".into() };
    }
    if no_link(root).is_err() { return "unavailable".into(); }
    let mut pending = vec![(root.to_path_buf(),0)];
    let mut visited = 0;
    while let Some((dir,depth)) = pending.pop() {
        visited += 1;
        if visited > 100 { return "unavailable".into(); }
        let Ok(entries) = fs::read_dir(dir) else { return "unavailable".into(); };
        for entry in entries {
            let Ok(entry) = entry else { return "unavailable".into(); };
            let Ok(kind) = entry.file_type() else { return "unavailable".into(); };
            let name = entry.file_name().to_string_lossy().into_owned();
            if source.kind == SaveKind::Nscripter {
                if kind.is_file() && nscripter_file(&name) { return "ready".into(); }
            } else if kind.is_file() && !config_only(&name) {
                return "ready".into();
            } else if kind.is_dir() && depth < 3 && !matches!(name.to_lowercase().as_str(), "backup" | "screenshots" | "screenshot" | "cache") {
                if no_link(&entry.path()).is_err() { return "unavailable".into(); }
                pending.push((entry.path(),depth+1));
            }
        }
    }
    "empty".into()
}
fn candidate(out: &mut SaveDiscovery, source: SaveSource, reason: &str, automatic: bool) {
    if out.candidates.iter().any(|c| normalized(Path::new(&c.source.path)) == normalized(Path::new(&source.path)) && c.source.kind == source.kind) { return; }
    let status = source_status(&source);
    out.candidates.push(SaveCandidate { source, reason: reason.into(), status, automatic });
}
#[derive(Default)]
pub struct SaveFolders { pub documents: PathBuf, pub roaming: PathBuf, pub local: PathBuf, pub profile: PathBuf, pub public_documents: PathBuf }
impl SaveFolders {
    fn from_app(app: &AppHandle) -> Self {
        Self {
            documents: app.path().document_dir().unwrap_or_default(),
            roaming: std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default(),
            local: std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default(),
            profile: app.path().home_dir().unwrap_or_default(),
            public_documents: std::env::var_os("PUBLIC").map(|p| PathBuf::from(p).join("Documents")).unwrap_or_default(),
        }
    }
}
fn config_text(path: &Path, legacy_encoding: &'static encoding_rs::Encoding) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() > 65_536 { return None; }
    let bytes = fs::read(path).ok()?;
    if bytes.starts_with(&[0xff, 0xfe]) { return Some(encoding_rs::UTF_16LE.decode(&bytes[2..]).0.into_owned()); }
    Some(match String::from_utf8(bytes.clone()) {
        Ok(s) => s,
        Err(_) => legacy_encoding.decode(&bytes).0.into_owned(),
    })
}
fn configured_path(value: &str, install: &Path, folders: &SaveFolders) -> Option<PathBuf> {
    let mut value = value.trim().trim_matches('"').to_string();
    if value.eq_ignore_ascii_case("<none>") { return Some(install.join("SaveData")); }
    for (token, root) in [
        ("<MYDOCUMENTS>", &folders.documents), ("<APPDATA>", &folders.roaming),
        ("<COMMON_DOCUMENTS>", &folders.public_documents), ("%USERPROFILE%", &folders.profile),
        ("%APPDATA%", &folders.roaming), ("%LOCALAPPDATA%", &folders.local),
    ] {
        if value.to_uppercase().contains(token) {
            if root.as_os_str().is_empty() { return None; }
            value = value.replace(token, &root.to_string_lossy());
        }
    }
    if value.contains(['<', '>', '%']) { return None; }
    let path = PathBuf::from(value.replace('\\', std::path::MAIN_SEPARATOR_STR));
    Some(if path.is_absolute() { path } else { install.join(path) })
}
pub fn discover(record: &GameRecord, folders: &SaveFolders, automatic_only: bool) -> SaveDiscovery {
    let mut out = SaveDiscovery::default();
    if !automatic_only {
        let configured = record.library.as_ref().map(|l| l.save_sources.as_slice()).unwrap_or_default();
        if !configured.is_empty() {
            for source in configured { candidate(&mut out, source.clone(), "已确认的存档位置", true); }
            return out;
        }
        if let Some(path) = record.save_path.as_deref().filter(|s| !s.trim().is_empty()) {
            candidate(&mut out, SaveSource { slot: "manual".into(), path: path.into(), kind: SaveKind::Directory }, "手动设置的存档目录", true);
            return out;
        }
    }
    for (index, install) in record.library.iter().flat_map(|l| &l.installations).enumerate() {
        let root = Path::new(&install.directory);
        let entries = match fs::read_dir(root) {
            Ok(e) => e.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(e) => { out.warnings.push(format!("安装目录离线或无法读取 {}：{e}", root.display())); continue; }
        };
        let mut explicit = false;
        if let Some(text) = config_text(&root.join("SaveData/SavePath.txt"), encoding_rs::SHIFT_JIS) {
            if let Some(value) = text.lines().filter_map(|l| l.trim().split_once('=')).find(|(k,_)| k.trim().eq_ignore_ascii_case("SavePath")).map(|(_,v)| v) {
                if let Some(path) = configured_path(value, root, folders) {
                    candidate(&mut out, SaveSource { slot: format!("install-{index}:configured"), path: path.to_string_lossy().into_owned(), kind: SaveKind::Directory }, "SavePath.txt 指定的位置", true);
                    explicit = true;
                }
            }
        }
        if let Some(text) = config_text(&root.join("config/startup.xml"), encoding_rs::UTF_8) {
            if let Some(value) = text.split_once("<save>").and_then(|(_,s)| s.split_once("</save>")).map(|(s,_)| s) {
                if let Some(path) = configured_path(value, root, folders) {
                    candidate(&mut out, SaveSource { slot: format!("install-{index}:configured"), path: path.to_string_lossy().into_owned(), kind: SaveKind::Directory }, "游戏 startup.xml 指定的位置", true);
                    explicit = true;
                }
            }
        }
        if let Some(text) = config_text(&root.join("AliceStart.ini"), encoding_rs::GB18030) {
            let setting = |name: &str| text.lines().filter_map(|l| l.trim().split_once('=')).find(|(k,_)| k.trim().eq_ignore_ascii_case(name)).map(|(_,v)| v.trim().trim_matches('"'));
            if let (Some(name),Some(folder)) = (setting("GameName"),setting("SaveFolder")) {
                // This engine's distribution supplies this exact Documents layout.
                let relative = Path::new(name).join(folder);
                if !relative.is_absolute() && !relative.components().any(|p| matches!(p,std::path::Component::ParentDir))
                    && root.join("放到文档里/AliceSoft").join(&relative).is_dir() && !folders.documents.as_os_str().is_empty() {
                    candidate(&mut out, SaveSource {slot:format!("install-{index}:configured"),path:folders.documents.join("AliceSoft").join(relative).to_string_lossy().into_owned(),kind:SaveKind::Directory},
                        "AliceStart.ini 与游戏提供的文档目录结构",true);
                    explicit=true;
                }
            }
        }
        if entries.iter().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("STEINSGATE.exe"))
            && root.join("system.npa").is_file() && !folders.roaming.as_os_str().is_empty() {
            candidate(&mut out,SaveSource {slot:format!("install-{index}:configured"),path:folders.roaming.join("Nitroplus/STEINS;GATE").to_string_lossy().into_owned(),kind:SaveKind::Directory},
                "Nitroplus PC 版存档位置（游戏使用说明）",true);
            explicit=true;
        }
        if explicit { continue; }
        let nscripter = entries.iter().any(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case("nscript.dat"))
            || entries.iter().any(|e| nscripter_file(&e.file_name().to_string_lossy()));
        if nscripter {
            candidate(&mut out, SaveSource { slot: format!("install-{index}:nscripter"), path: install.directory.clone(), kind: SaveKind::Nscripter }, "游戏根目录中的存档文件（不含资源包）", true);
            continue;
        }
        let mut paths: Vec<PathBuf> = entries.iter().filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter(|e| { let n = e.file_name().to_string_lossy().to_lowercase(); matches!(n.as_str(), "save" | "saves" | "savedata") || n.ends_with("_savedata") })
            .map(|e| e.path()).collect();
        for relative in ["game/saves", "SteamEmu/Saves", "Profile/ALI213/Saves", "ELFIMAGE/save"] {
            let path = root.join(relative); if path.is_dir() { paths.push(path); }
        }
        // Packed distributions also contain full-CG saves. Only live engine locations are considered.
        paths.sort_by_key(|p| normalized(p));
        let ready: Vec<_> = paths.iter().filter(|p| source_status(&SaveSource { slot: String::new(), path: p.to_string_lossy().into_owned(), kind: SaveKind::Directory }) == "ready").cloned().collect();
        for path in paths {
            let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().replace('\\', "/").to_lowercase();
            let reliable = ready.len() <= 1;
            candidate(&mut out, SaveSource { slot: format!("install-{index}:{relative}"), path: path.to_string_lossy().into_owned(), kind: SaveKind::Directory },
                if reliable { "引擎存档位置" } else { "发现多个存档目录，请确认使用的位置" }, reliable);
        }
    }
    out
}
pub(crate) fn resolve(record: &GameRecord, folders: &SaveFolders, data: &Path, allow_missing: bool) -> Result<Vec<ResolvedSave>, String> {
    let discovery = discover(record, folders, false);
    if !discovery.warnings.is_empty() { return Err(discovery.warnings.join("；")); }
    let mut sources = Vec::new();
    let mut slots = HashSet::new();
    for candidate in discovery.candidates {
        if !candidate.automatic { return Err("发现多个存档位置，请先在存档管理中确认".into()); }
        if candidate.status == "unavailable" { return Err(format!("存档位置无法读取：{}", candidate.source.path)); }
        if candidate.status != "ready" && !allow_missing {
            if record.library.as_ref().is_some_and(|l| !l.save_sources.is_empty()) { return Err(format!("已确认的存档尚未生成或为空：{}", candidate.source.path)); }
            continue;
        }
        let source = candidate.source;
        if !slots.insert(source.slot.clone()) { return Err("存档位置标识重复，请重新识别".into()); }
        let raw = Path::new(&source.path);
        if !raw.is_absolute() || raw.components().any(|p| matches!(p, std::path::Component::ParentDir)) { return Err("存档路径必须为完整目录".into()); }
        for ancestor in raw.ancestors().filter(|p| p.exists()) { no_link(ancestor)?; }
        let ancestor = raw.ancestors().find(|p| p.exists()).ok_or("存档所在磁盘或共享目录不可用")?;
        let root = ancestor.canonicalize().map_err(|e| e.to_string())?.join(raw.strip_prefix(ancestor).map_err(|e| e.to_string())?);
        let protected = [&folders.documents, &folders.roaming, &folders.local, &folders.profile, &folders.public_documents, data];
        if root.parent().is_none() || protected.iter().filter(|p| !p.as_os_str().is_empty()).any(|p| {
            let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            p.starts_with(&root)
        }) || ["WINDIR", "ProgramFiles", "ProgramFiles(x86)"].into_iter().filter_map(std::env::var_os).filter_map(|p| Path::new(&p).canonicalize().ok()).any(|p| root.starts_with(p)) {
            return Err("请选择游戏专用存档，不能备份系统、文档根目录或应用数据根目录".into());
        }
        if data.canonicalize().is_ok_and(|p| root.starts_with(p)) { return Err("不能备份应用自身的数据目录".into()); }
        if source.kind == SaveKind::Directory && record.library.iter().flat_map(|l| &l.installations).any(|i| {
            Path::new(&i.directory).canonicalize().is_ok_and(|p| p.starts_with(&root))
        }) { return Err("不能把整个游戏安装目录作为存档目录".into()); }
        if sources.iter().any(|s: &ResolvedSave| root.starts_with(&s.root) || s.root.starts_with(&root)) { return Err("存档位置互相包含，请只保留一个".into()); }
        sources.push(ResolvedSave { source, root });
    }
    if sources.is_empty() { return Err("尚未找到已生成的存档，请运行游戏保存后重新识别，或手动选择目录".into()); }
    Ok(sources)
}
pub(crate) fn app_folders(app: &AppHandle) -> SaveFolders { SaveFolders::from_app(app) }

#[tauri::command]
pub async fn discover_game_saves(app: AppHandle, state: State<'_, AppState>, record_id: String, automatic_only: bool) -> Result<SaveDiscovery, String> {
    let record = crate::records::read_records(&*state.db.lock().map_err(|e| e.to_string())?)?.into_iter().find(|r| r.id == record_id).ok_or("作品已不存在")?;
    let folders = app_folders(&app);
    tauri::async_runtime::spawn_blocking(move || discover(&record, &folders, automatic_only)).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(root: &Path) -> GameRecord {
        serde_json::from_value(serde_json::json!({
            "id":"fixture","title":"Fixture","category":"galgame","status":"backlog","progress":"","rating":0,
            "library":{"installations":[{"root":root,"directory":root,"executable":root.join("game.exe"),"candidates":[]}]}
        })).unwrap()
    }
    #[test]
    fn configuration_respects_redirected_documents_and_shift_jis_before_local_templates() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("game"); let docs = temp.path().join("redirected-documents");
        fs::create_dir_all(root.join("SaveData")).unwrap(); fs::create_dir(&docs).unwrap();
        let text = "//SavePath=<none>\r\nSavePath=<MYDOCUMENTS>\\美少女万華鏡VOL2\\";
        fs::write(root.join("SaveData/SavePath.txt"), encoding_rs::SHIFT_JIS.encode(text).0).unwrap();
        fs::write(root.join("SaveData/save.dat"), b"bundled template").unwrap();
        let folders = SaveFolders { documents:docs.clone(), ..Default::default() };
        let game = record(&root);
        let scan = discover(&game,&folders,true);
        assert_eq!(scan.candidates.len(),1);
        assert_eq!(Path::new(&scan.candidates[0].source.path),docs.join("美少女万華鏡VOL2"));
        assert_eq!(scan.candidates[0].status,"missing");
        fs::create_dir(docs.join("美少女万華鏡VOL2")).unwrap();
        fs::write(docs.join("美少女万華鏡VOL2/save.dat"),b"my progress").unwrap();
        assert_eq!(discover(&game,&folders,true).candidates[0].status,"ready");
        fs::remove_file(root.join("SaveData/SavePath.txt")).unwrap();
        fs::create_dir_all(root.join("放到文档里/AliceSoft/妻みぐい３-汉化测试ver0.8/SaveData")).unwrap();
        let ini = "GameName = \"妻みぐい３-汉化测试ver0.8\"\r\nSaveFolder = \"SaveData\"";
        fs::write(root.join("AliceStart.ini"), encoding_rs::GB18030.encode(ini).0).unwrap();
        let scan=discover(&game,&folders,true);
        assert_eq!(scan.candidates.len(),1);
        assert_eq!(Path::new(&scan.candidates[0].source.path),docs.join("AliceSoft/妻みぐい３-汉化测试ver0.8/SaveData"));
    }
    #[test]
    fn nscripter_root_files_and_nested_engine_saves_exclude_assets_and_full_cg_archives() {
        let temp = tempfile::tempdir().unwrap(); let root=temp.path();
        fs::write(root.join("nscript.dat"),b"script").unwrap(); fs::write(root.join("gloval.sav"),b"flags").unwrap();
        fs::create_dir(root.join("savedata")).unwrap();
        let scan=discover(&record(root),&SaveFolders::default(),true);
        assert_eq!(scan.candidates.len(),1); assert_eq!(scan.candidates[0].source.kind,SaveKind::Nscripter);
        assert!(!nscripter_file("savestruct.dll")); assert!(!nscripter_file("001.ass"));
        fs::remove_file(root.join("nscript.dat")).unwrap(); fs::remove_file(root.join("gloval.sav")).unwrap();
        fs::create_dir_all(root.join("SteamEmu/Saves")).unwrap(); fs::write(root.join("SteamEmu/Saves/datasu.ksd"),b"flags").unwrap();
        fs::create_dir_all(root.join("全CG存档/savedata")).unwrap(); fs::write(root.join("全CG存档/savedata/datasu.ksd"),b"template").unwrap();
        fs::write(root.join("savedata/savecheck"),b"").unwrap();
        let scan=discover(&record(root),&SaveFolders::default(),true);
        assert_eq!(scan.candidates.iter().filter(|c| c.status=="ready").count(),1);
        assert!(scan.candidates.iter().any(|c| Path::new(&c.source.path).ends_with("SteamEmu/Saves") && c.automatic));
        assert!(!scan.candidates.iter().any(|c| c.source.path.contains("全CG")));
    }
}
