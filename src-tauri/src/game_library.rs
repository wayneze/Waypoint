//! 本地安装发现与公开游戏资料。扫描只读；档案仍由 records 的单事务入口保存。
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{path::{Path, PathBuf}, sync::{OnceLock, atomic::{AtomicU64, Ordering}}, time::{Duration, Instant}};
use tauri::State;
use crate::AppState;

static SCAN_GENERATION: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Installation {
    pub root: String,
    pub directory: String,
    pub executable: String,
    #[serde(default)]
    pub candidates: Vec<String>,
    #[serde(default)]
    pub steam_id: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub source: String,
    pub source_id: String,
    pub title: String,
    #[serde(default)]
    pub original_title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cover: String,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub developer: String,
    #[serde(default)]
    pub developers: Vec<String>,
    #[serde(default)]
    pub publishers: Vec<String>,
    #[serde(default)]
    pub staff: Vec<GameStaff>,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub score: f64,
    pub url: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub info: Vec<MetadataField>,
    #[serde(default)]
    pub sources: Vec<MetadataSource>,
    #[serde(default)]
    pub videos: Vec<GameVideo>,
    #[serde(default)]
    pub music: Vec<GameMusic>,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub enriched: bool,
    #[serde(default)]
    pub media_version: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataField {
    pub label: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStaff {
    pub name: String,
    pub role: String,
    pub source_id: Option<String>,
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GameCuration {
    #[serde(default)]
    pub added_tags: Vec<String>,
    #[serde(default)]
    pub hidden_tags: Vec<String>,
    pub developers: Option<Vec<String>>,
    pub publishers: Option<Vec<String>>,
    pub staff: Option<Vec<GameStaff>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataSource {
    pub source: String,
    pub source_id: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVideo {
    pub title: String,
    pub url: String,
    pub thumbnail: String,
    pub page_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameMusic {
    pub id: String,
    pub title: String,
    pub relation: String,
    pub cover: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraMedia {
    pub kind: String,
    pub title: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataMatch {
    pub query: String,
    pub error: String,
    pub candidates: Vec<Metadata>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GameCollection {
    pub name: String,
    pub order: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LibraryDetails {
    #[serde(default)]
    pub installations: Vec<Installation>,
    pub collection: Option<GameCollection>,
    /// "steam" or "local:<normalized directory>"; never an unstable installation index.
    pub preferred_launch: Option<String>,
    #[serde(default)]
    pub save_sources: Vec<crate::game_saves::SaveSource>,
    pub metadata: Option<Metadata>,
    pub curation: Option<GameCuration>,
    #[serde(default)]
    pub scaling: Vec<crate::scaling::ScalingProfile>,
    pub background: Option<String>,
    pub background_position: Option<crate::records::CoverPosition>,
    #[serde(default)]
    pub extra_media: Vec<ExtraMedia>,
    pub matching: Option<MetadataMatch>,
}

#[derive(Debug, Serialize)]
pub struct ScanGame {
    pub title: String,
    pub installation: Installation,
    pub steam_id: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub games: Vec<ScanGame>,
    pub warnings: Vec<String>,
    pub complete: bool,
    pub visited: usize,
}

fn ignored_directory(name: &str) -> bool {
    let n = name.to_lowercase();
    matches!(n.as_str(), "data" | "savedata" | "save" | "saves" | "cg" | "bgm" | "voice" | "movie" | "movies" | "lib" | "renpy" | "engine" | "redist" | "_commonredist" | "directx" | "support" | "plugins" | "node_modules")
        || n.starts_with('.') || n.ends_with("_data")
        || ["backup", "patch", "update", "补丁", "備份", "备份", "原版文件", "修正"].iter().any(|s| n.contains(s))
}
pub(crate) fn game_executable(name: &str) -> bool {
    let n = name.to_lowercase();
    if !n.ends_with(".exe") { return false; }
    !["unins", "uninst", "setup", "install", "config", "userconf", "setting", "patch", "update", "redist", "dxweb", "vcredist", "crash", "report", "python", "zsync", "filecheck", "sdacti", "sdrt", "kmsdll", "support", "opensave", "reset", "3dconfig", "補丁", "补丁", "設定", "设置", "インストール", "チェック", "チェッカ", "認証", "验证", "修正"].iter().any(|s| n.contains(s))
        && n != "menu.exe"
}
fn search_name(name: &str) -> String {
    let mut depth = 0;
    let clean: String = name.chars().filter(|c| {
        if matches!(c, '[' | '【') { depth += 1; return false; }
        if matches!(c, ']' | '】') { depth = (depth - 1).max(0); return false; }
        depth == 0
    }).collect();
    let clean = clean.trim();
    // 仅剥离显式的年份/序号前缀，保留 11eyes、428 等正式标题里的数字。
    let clean = clean.split_once('-').filter(|(prefix, _)| {
        let prefix = prefix.trim();
        !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit() || c == '.')
    }).map(|(_, title)| title.trim()).unwrap_or(clean);
    clean.to_string()
}
fn steam_manifest(path: &Path) -> Option<(String, String, String)> {
    let text = std::fs::read_to_string(path).ok()?;
    let value = |key: &str| text.lines().find_map(|line| {
        let parts: Vec<&str> = line.split('"').collect();
        (parts.len() >= 4 && parts[1] == key).then(|| parts[3].to_string())
    });
    Some((value("appid")?, value("name")?, value("installdir")?))
}
fn path_text(path: &Path) -> String { path.to_string_lossy().into_owned() }

pub(crate) fn scan(root: &str, generation: u64) -> Result<ScanResult, String> {
    let root_path = PathBuf::from(root);
    if !root_path.is_absolute() { return Err("请选择完整的本地目录或网络共享目录".into()); }
    let first = std::fs::read_dir(&root_path).map_err(|e| format!("无法读取游戏目录：{e}"))?;
    drop(first);
    let mut result = ScanResult { games: vec![], warnings: vec![], complete: true, visited: 0 };
    let start = Instant::now();
    let mut queue = vec![(root_path.clone(), 0usize, String::new(), None::<String>)];
    while let Some((dir, depth, parent_title, inherited_id)) = queue.pop() {
        if SCAN_GENERATION.load(Ordering::Relaxed) != generation || start.elapsed() > Duration::from_secs(60) || result.visited >= 6000 {
            result.complete = false;
            result.warnings.push("扫描已停止或达到时间/目录上限；当前结果不完整，可选择更小的子目录继续扫描。".into());
            break;
        }
        result.visited += 1;
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => { result.complete = false; result.warnings.push(format!("{}：{e}", dir.display())); continue; }
        };
        let mut dirs = vec![];
        let mut exes = vec![];
        let mut manifests = vec![];
        for entry in entries {
            let entry = match entry { Ok(e) => e, Err(e) => { result.complete = false; result.warnings.push(e.to_string()); continue; } };
            let name = entry.file_name().to_string_lossy().into_owned();
            let Ok(kind) = entry.file_type() else { result.complete = false; continue; };
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if entry.metadata().map(|m| m.file_attributes() & 0x400 != 0).unwrap_or(true) { continue; }
            }
            if kind.is_symlink() { continue; }
            if kind.is_dir() && !ignored_directory(&name) { dirs.push(entry.path()); }
            if kind.is_file() && game_executable(&name) { exes.push(path_text(&entry.path())); }
            if kind.is_file() && name.starts_with("appmanifest_") && name.ends_with(".acf") {
                if let Some(m) = steam_manifest(&entry.path()) { manifests.push(m); }
            }
        }
        let raw_name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let name = search_name(raw_name);
        let normalized = |s: &str| s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
        let wrapper = depth >= 2 && name.len() <= 8
            && name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
            && name.chars().any(|c| c.is_ascii_digit());
        let numbered = raw_name.starts_with(|c: char| c.is_ascii_digit()) && raw_name.contains(" - ");
        let title = if name.is_empty() || (!parent_title.is_empty() && (wrapper || inherited_id.is_some())) { parent_title.clone() }
            else if !parent_title.is_empty() && !parent_title.contains("部曲")
                && !normalized(&name).starts_with(&normalized(&parent_title))
                && (numbered || name.to_lowercase().starts_with("vol.") || name.to_lowercase() == "extra") {
                format!("{parent_title} {name}")
            } else { name };
        if !exes.is_empty() {
            exes.sort_by_key(|path| {
                let n = Path::new(path).file_name().unwrap().to_string_lossy().to_lowercase();
                (!n.contains("_cn") && !n.contains("chs") && !n.contains("启动游戏") && !n.contains("汉化"), n)
            });
            let steam_id = inherited_id.or_else(|| std::fs::read_to_string(dir.join("steam_appid.txt")).ok()
                .map(|s| s.trim().to_owned()).filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())));
            result.games.push(ScanGame { title, steam_id: steam_id.clone(), installation: Installation {
                root: root.to_string(), directory: path_text(&dir), executable: exes[0].clone(), candidates: exes,
                steam_id,
            }});
            // 一个安装根下面的引擎/工具目录不再产生第二份游戏。
            continue;
        }
        if depth >= 7 && !dirs.is_empty() {
            result.complete = false;
            result.warnings.push(format!("目录层级过深，未继续：{}", dir.display()));
            continue;
        }
        if !manifests.is_empty() {
            dirs.retain(|p| p.file_name().and_then(|n| n.to_str()) != Some("common"));
            for (id, name, installed) in manifests {
                let path = dir.join("common").join(installed);
                if path.is_dir() { queue.push((path, depth + 1, name, Some(id))); }
            }
        }
        dirs.sort();
        for child in dirs.into_iter().rev() {
            queue.push((child, depth + 1, title.clone(), inherited_id.clone()));
        }
    }
    result.games.sort_by(|a, b| a.installation.directory.cmp(&b.installation.directory));
    Ok(result)
}

#[tauri::command]
pub async fn scan_game_directory(root: String) -> Result<ScanResult, String> {
    let generation = SCAN_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn_blocking(move || scan(&root, generation)).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn cancel_game_scan() { SCAN_GENERATION.fetch_add(1, Ordering::SeqCst); }

fn string(v: &Value, key: &str) -> String { v[key].as_str().unwrap_or("").to_string() }
pub(crate) fn client() -> Result<reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::builder().user_agent("Waypoint/0.1 (personal desktop game library)")
        .connect_timeout(crate::PUBLIC_CONNECT_TIMEOUT).timeout(crate::PUBLIC_REQUEST_TIMEOUT).build().map_err(|e| e.to_string())).clone()
}
const BANGUMI_TOKEN: &str = "bangumi_access_token";
pub(crate) fn bangumi_request(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match crate::secrets::load_secret(BANGUMI_TOKEN, Some(crate::secrets::STORED_MARK.into())) {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}
#[tauri::command]
pub fn bangumi_connected() -> bool {
    crate::secrets::load_secret(BANGUMI_TOKEN, Some(crate::secrets::STORED_MARK.into())).is_some()
}
#[tauri::command]
pub async fn connect_bangumi(token: String) -> Result<(), String> {
    let token = token.trim();
    if token.is_empty() { return Err("请填写 Bangumi Access Token".into()); }
    response(client()?.get("https://api.bgm.tv/v0/me").bearer_auth(token)).await
        .map_err(|_| "Bangumi 连接未通过，请检查令牌是否有效及网络连接。".to_string())?;
    crate::secrets::store_secret(BANGUMI_TOKEN, token)?;
    Ok(())
}
pub(crate) async fn response(request: reqwest::RequestBuilder) -> Result<Value, String> {
    let response = crate::public_http::send(request).await.map_err(|e| {
        let host = e.url().and_then(|url| url.host_str()).unwrap_or("资料服务").to_string();
        let phase = if e.is_timeout() && e.is_connect() { format!("建立连接超时（{} 秒）", crate::PUBLIC_CONNECT_TIMEOUT.as_secs()) }
            else if e.is_timeout() { format!("请求超时（{} 秒）", crate::PUBLIC_REQUEST_TIMEOUT.as_secs()) }
            else if e.is_connect() { "连接失败".into() } else { "请求失败".into() };
        let mut details = Vec::new();
        let mut cause = std::error::Error::source(&e);
        while let Some(error) = cause {
            let detail = error.to_string();
            if !details.contains(&detail) { details.push(detail); }
            cause = error.source();
        }
        if details.is_empty() { details.push(e.without_url().to_string()); }
        format!("{host} {phase}：{}", details.join("；"))
    })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("资料服务拒绝请求 ({status})：{}", body.chars().take(400).collect::<String>()));
    }
    response.json().await.map_err(|e| format!("资料格式无法读取：{e}"))
}
fn vndb_metadata(v: &Value) -> Metadata {
    let id = string(v, "id");
    let titles = v["titles"].as_array();
    let chinese = ["zh-Hans", "zh-Hant"].iter().find_map(|lang| {
        titles?.iter().find(|t| t["lang"].as_str() == Some(lang))?.get("title")?.as_str()
    });
    Metadata {
        source: "vndb".into(), source_id: id.clone(), title: chinese.map(str::to_owned).unwrap_or_else(|| string(v, "title")), original_title: string(v, "alttitle"),
        description: string(v, "description"), cover: string(&v["image"], "url"),
        screenshots: v["screenshots"].as_array().into_iter().flatten()
            .filter(|s| s["sexual"].as_f64().unwrap_or(2.0) < 0.5 && s["violence"].as_f64().unwrap_or(2.0) < 0.5)
            .map(|s| string(s, "url")).take(12).collect(),
        developer: v["developers"].as_array().into_iter().flatten().map(|d| string(d, "name")).collect::<Vec<_>>().join(" / "),
        developers: v["developers"].as_array().into_iter().flatten().map(|d| string(d, "name")).collect(),
        // eid=null names the original work. Translation/port-specific staff must not silently
        // become the authors of every installation of this work.
        staff: v["staff"].as_array().into_iter().flatten().filter(|s| s["eid"].is_null()).map(|s| {
            let original = string(s, "original");
            GameStaff { name: if original.is_empty() { string(s, "name") } else { original },
                role: string(s, "role"), source_id: Some(string(s, "id")) }
        }).collect(),
        release_date: string(v, "released"), score: v["rating"].as_f64().unwrap_or(0.0) / 10.0,
        url: format!("https://vndb.org/{id}"),
        aliases: v["aliases"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string)
            .chain(v["titles"].as_array().into_iter().flatten().map(|t| string(t, "title"))).collect(),
        tags: v["tags"].as_array().into_iter().flatten().filter(|t| t["spoiler"].as_f64().unwrap_or(1.0) == 0.0)
            .map(|t| string(t, "name")).take(24).collect(),
        sources: v["extlinks"].as_array().into_iter().flatten().filter_map(|link| source_link(&string(link, "url"))).collect(),
        ..Default::default()
    }
}
fn field_values(value: &Value) -> Vec<String> {
    match value {
        Value::String(s) => vec![s.clone()],
        Value::Array(items) => items.iter().filter_map(|i| i["v"].as_str().map(str::to_string)).collect(),
        _ => vec![],
    }
}
fn source_link(raw: &str) -> Option<MetadataSource> {
    let url = reqwest::Url::parse(raw).ok()?;
    let parts: Vec<_> = url.path_segments()?.filter(|p| !p.is_empty()).collect();
    let (source, id) = match url.host_str()? {
        "vndb.org" | "www.vndb.org" if parts.first()?.starts_with('v') => ("vndb", *parts.first()?),
        "store.steampowered.com" if parts.first() == Some(&"app") => ("steam", *parts.get(1)?),
        "bgm.tv" | "bangumi.tv" | "chii.in" if parts.first() == Some(&"subject") => ("bangumi", *parts.get(1)?),
        _ => return None,
    };
    let digits = if source == "vndb" { id.strip_prefix('v')? } else { id };
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) { return None; }
    Some(MetadataSource { source: source.into(), source_id: id.into(), url: raw.into() })
}
pub(crate) fn bgm_metadata(v: &Value) -> Metadata {
    let id = v["id"].to_string();
    let cn = string(v, "name_cn");
    let info: Vec<_> = v["infobox"].as_array().into_iter().flatten().map(|i| MetadataField {
        label: string(i, "key"), value: field_values(&i["value"]).join(" / "),
    }).filter(|i| !i.value.is_empty()).collect();
    let aliases = info.iter().filter(|i| matches!(i.label.as_str(), "别名" | "英文名" | "中文名"))
        .flat_map(|i| i.value.split(" / ").map(str::to_string)).collect();
    let sources = v["infobox"].as_array().into_iter().flatten().flat_map(|i| field_values(&i["value"]))
        .filter_map(|value| source_link(&value)).collect();
    Metadata { source: "bangumi".into(), source_id: id.clone(), title: if cn.is_empty() { string(v, "name") } else { cn },
        original_title: string(v, "name"), description: string(v, "summary"), cover: string(&v["images"], "large"),
        screenshots: vec![], developer: v["infobox"].as_array().into_iter().flatten()
            .filter(|i| matches!(i["key"].as_str(), Some("开发" | "开发商"))).filter_map(|i| i["value"].as_str()).collect::<Vec<_>>().join(" / "),
        release_date: string(v, "date"), score: v["rating"]["score"].as_f64().unwrap_or(0.0), url: format!("https://bgm.tv/subject/{id}"),
        aliases, sources,
        tags: v["tags"].as_array().into_iter().flatten().map(|t| string(t, "name")).take(30).collect(),
        platforms: info.iter().filter(|i| i.label == "平台").flat_map(|i| i.value.split(" / ").map(str::to_string)).collect(),
        info, ..Default::default()
    }
}

#[tauri::command]
pub async fn search_game_metadata(source: String, query: String) -> Result<Vec<Metadata>, String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 300 { return Err("请输入游戏名称或来源 ID".into()); }
    let client = client()?;
    match source.as_str() {
        "vndb" => {
            let id = query.starts_with('v') && query[1..].bytes().all(|b| b.is_ascii_digit()) && query.len() > 1;
            let v = response(client.post("https://api.vndb.org/kana/vn").json(&json!({
                "filters": [if id { "id" } else { "search" }, "=", query],
                "fields":"title,alttitle,aliases,titles.title,titles.lang,description,image.url,screenshots.url,screenshots.sexual,screenshots.violence,developers.name,staff.id,staff.name,staff.original,staff.role,staff.eid,released,rating,tags.name,tags.spoiler,extlinks.url",
                "results":8
            }))).await?;
            Ok(v["results"].as_array().into_iter().flatten().map(vndb_metadata).collect())
        }
        "bangumi" => {
            if query.bytes().all(|b| b.is_ascii_digit()) {
                let v = response(bangumi_request(client.get(format!("https://api.bgm.tv/v0/subjects/{query}")))).await?;
                if v["type"].as_i64() != Some(4) { return Err("此 Bangumi 条目不是游戏".into()); }
                return Ok(vec![bgm_metadata(&v)]);
            }
            let v = response(bangumi_request(client.post("https://api.bgm.tv/v0/search/subjects?limit=8"))
                .json(&json!({"keyword":query,"sort":"match","filter":{"type":[4],"nsfw":null}}))).await?;
            Ok(v["data"].as_array().into_iter().flatten().map(bgm_metadata).collect())
        }
        "steam" => {
            if query.bytes().all(|b| b.is_ascii_digit()) { return Ok(vec![steam_details(&client, query).await?]); }
            let v = response(client.get("https://store.steampowered.com/api/storesearch/").query(&[("term",query),("l","schinese"),("cc","cn")])).await?;
            Ok(v["items"].as_array().into_iter().flatten().take(6).map(|item| {
                let id = item["id"].to_string();
                Metadata { source:"steam".into(), source_id:id.clone(), title:string(item,"name"), original_title:String::new(),
                    description:String::new(), cover:string(item,"tiny_image"), screenshots:vec![], developer:String::new(), release_date:String::new(), score:0.0,
                    url:format!("https://store.steampowered.com/app/{id}/"), ..Default::default() }
            }).collect())
        }
        _ => Err("不支持的资料来源".into()),
    }
}
async fn steam_details(client: &reqwest::Client, id: &str) -> Result<Metadata, String> {
    let v = response(client.get("https://store.steampowered.com/api/appdetails").query(&[("appids",id),("l","schinese")])).await?;
    if v[id]["success"] != true { return Err("Steam 暂无此游戏资料".into()); }
    let d = &v[id]["data"];
    Ok(Metadata { source:"steam".into(), source_id:id.into(), title:string(d,"name"), original_title:String::new(),
        description:string(d,"short_description"), cover:string(d,"header_image"),
        screenshots:d["screenshots"].as_array().into_iter().flatten().take(12).map(|s| string(s,"path_full")).collect(),
        developer:d["developers"].as_array().into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(" / "),
        developers:d["developers"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect(),
        publishers:d["publishers"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect(),
        release_date:string(&d["release_date"],"date"), score:0.0, url:format!("https://store.steampowered.com/app/{id}/"),
        tags: d["genres"].as_array().into_iter().flatten().map(|g| string(g,"description")).collect(),
        background: string(d,"background_raw"),
        videos: d["movies"].as_array().into_iter().flatten().filter_map(|m| {
            let url = m["hls_h264"].as_str().or(m["mp4"]["max"].as_str()).or(m["mp4"]["480"].as_str())
                .or(m["webm"]["max"].as_str()).or(m["webm"]["480"].as_str())?;
            Some(GameVideo { title:string(m,"name"), url:url.replace("http://","https://"),
                thumbnail:string(m,"thumbnail"), page_url:format!("https://store.steampowered.com/app/{id}/") })
        }).collect(),
        ..Default::default() })
}

fn add_sources(meta: &mut Metadata, sources: Vec<MetadataSource>) {
    for source in sources {
        if !meta.sources.iter().any(|s| s.source == source.source && s.source_id == source.source_id) {
            meta.sources.push(source);
        }
    }
}
fn merge_media(meta: &mut Metadata, extra: Metadata) {
    add_sources(meta, extra.sources);
    for developer in extra.developers {
        if !meta.developers.contains(&developer) { meta.developers.push(developer); }
    }
    for publisher in extra.publishers {
        if !meta.publishers.contains(&publisher) { meta.publishers.push(publisher); }
    }
    for staff in extra.staff {
        if !meta.staff.iter().any(|s| s.name == staff.name && s.role == staff.role) { meta.staff.push(staff); }
    }
    for shot in extra.screenshots {
        if !meta.screenshots.contains(&shot) { meta.screenshots.push(shot); }
    }
    for video in extra.videos {
        if !meta.videos.iter().any(|v| v.url == video.url) { meta.videos.push(video); }
    }
    if meta.background.is_empty() { meta.background = extra.background; }
}
/// A release must represent one complete work; patches, trials and bundles cannot supply its identity.
fn release_vn(release: &Value) -> Option<String> {
    if release["official"] != true || release["patch"] != false { return None; }
    let vns = release["vns"].as_array()?;
    if vns.len() != 1 || vns[0]["rtype"] != "complete" { return None; }
    let id = string(&vns[0], "id");
    source_link(&format!("https://vndb.org/{id}")).map(|s| s.source_id)
}
async fn releases(client: &reqwest::Client, filter: Value) -> Result<Vec<Value>, String> {
    let mut items = Vec::new();
    let mut page = 1;
    loop {
        let result = response(client.post("https://api.vndb.org/kana/release").json(&json!({
            "filters":["and", filter, ["official","=",1], ["patch","!=",1], ["rtype","=","complete"]],
            "fields":"official,patch,vns.id,vns.rtype,extlinks.url,extlinks.name", "results":100, "page":page
        }))).await?;
        items.extend(result["results"].as_array().into_iter().flatten().cloned());
        if result["more"] != true { return Ok(items); }
        page += 1;
    }
}
fn release_steam_sources(items: &[Value], vn_id: &str) -> Vec<MetadataSource> {
    items.iter().filter(|release| release_vn(release).as_deref() == Some(vn_id))
        .flat_map(|release| release["extlinks"].as_array().into_iter().flatten())
        .filter_map(|link| source_link(&string(link, "url"))).filter(|s| s.source == "steam").collect()
}
/// Only explicit work/release links establish identity; a name search is accepted only with a VNDB backlink.
#[tauri::command]
pub async fn load_game_metadata(source: String, source_id: String) -> Result<Metadata, String> {
    let mut meta = search_game_metadata(source.clone(), source_id.clone()).await?.into_iter()
        .find(|m| m.source_id == source_id).ok_or("此来源没有返回已选作品")?;
    let primary = MetadataSource { source:source.clone(), source_id:source_id.clone(), url:meta.url.clone() };
    add_sources(&mut meta, vec![primary]);
    let client = client()?;
    let mut official_pages = Vec::new();
    if source == "steam" {
        match releases(&client, json!(["extlink","=",["steam",source_id]])).await {
            Ok(items) => {
                let mut ids: Vec<_> = items.iter().filter_map(release_vn).collect();
                ids.sort(); ids.dedup();
                if ids.len() == 1 {
                    let id = ids.remove(0);
                    add_sources(&mut meta, vec![MetadataSource { source:"vndb".into(), url:format!("https://vndb.org/{id}"), source_id:id }]);
                }
            }
            Err(e) => meta.warnings.push(format!("VNDB 发行关联未取得：{e}")),
        }
    }
    if let Some(vn) = meta.sources.iter().find(|s| s.source == "vndb").cloned() {
        if source != "vndb" {
            match search_game_metadata(vn.source.clone(), vn.source_id.clone()).await {
                Ok(items) => if let Some(extra) = items.into_iter().find(|m| m.source_id == vn.source_id) { merge_media(&mut meta, extra); },
                Err(e) => meta.warnings.push(format!("VNDB 画面未取得：{e}")),
            }
        }
        match releases(&client, json!(["vn","=",["id","=",vn.source_id]])).await {
            Ok(items) => {
                official_pages.extend(items.iter().filter(|r|release_vn(r).as_deref()==Some(&vn.source_id))
                    .flat_map(|r|r["extlinks"].as_array().into_iter().flatten())
                    .filter(|link|link["name"]=="website").map(|link|string(link,"url")));
                let sources = release_steam_sources(&items, &vn.source_id); add_sources(&mut meta, sources);
            }
            Err(e) => meta.warnings.push(format!("Steam 发行关联未取得：{e}")),
        }
        if !meta.sources.iter().any(|s| s.source == "bangumi") {
            let query = if meta.original_title.is_empty() { &meta.title } else { &meta.original_title };
            match search_game_metadata("bangumi".into(), query.clone()).await {
                Ok(items) => {
                    let mut linked = items.into_iter().filter(|m| m.sources.iter().any(|s| s.source == "vndb" && s.source_id == vn.source_id));
                    if let (Some(extra), None) = (linked.next(), linked.next()) {
                        add_sources(&mut meta, vec![MetadataSource { source:"bangumi".into(), source_id:extra.source_id, url:extra.url }]);
                    }
                }
                Err(e) => meta.warnings.push(format!("Bangumi 作品关联未取得：{e}")),
            }
        }
    }
    let bgm = meta.sources.iter().find(|s| s.source == "bangumi").cloned();
    if let Some(bgm) = &bgm {
        if source != "bangumi" {
            match search_game_metadata(bgm.source.clone(), bgm.source_id.clone()).await {
                Ok(items) => if let Some(extra) = items.into_iter().find(|m| m.source_id == bgm.source_id) {
                    if extra.title != extra.original_title { meta.title = extra.title.clone(); }
                    if !extra.description.is_empty() { meta.description = extra.description.clone(); }
                    if meta.info.is_empty() { meta.info = extra.info.clone(); }
                    merge_media(&mut meta, extra);
                },
                Err(e) => meta.warnings.push(format!("Bangumi 中文资料未取得：{e}")),
            }
        }
    }
    let steam: Vec<_> = meta.sources.iter().filter(|s| s.source == "steam" && !(source == "steam" && s.source_id == source_id)).cloned().collect();
    let (steam_result, music_result) = tokio::join!(
        async {
            let mut results = Vec::new();
            for s in steam { results.push(steam_details(&client, &s.source_id).await); }
            results
        },
        async { match bgm { Some(s) => response(bangumi_request(client.get(format!("https://api.bgm.tv/v0/subjects/{}/subjects",s.source_id)))).await.map(Some), None => Ok(None) } },
    );
    for result in steam_result {
        match result {
            Ok(extra) => merge_media(&mut meta, extra),
            Err(e) => meta.warnings.push(format!("Steam 宣传片未取得：{e}")),
        }
    }
    if meta.videos.is_empty() {
        official_pages.extend(meta.info.iter().filter(|i|matches!(i.label.to_lowercase().as_str(),"website"|"官方网站"|"官网"))
            .flat_map(|i|i.value.split(" / ").map(str::to_string)));
        let (videos,warnings)=crate::game_trailers::official_trailers(&client,official_pages).await;
        meta.videos=videos; meta.warnings.extend(warnings);
    }
    match music_result {
        Ok(Some(items)) => {
            meta.music = items.as_array().into_iter().flatten().filter(|v| v["type"] == 3).map(|v| {
                let id = v["id"].to_string();
                GameMusic { id:id.clone(), title:string(v,"name"), relation:string(v,"relation"),
                    cover:string(&v["images"],"common"), url:format!("https://bgm.tv/subject/{id}") }
            }).collect();
            meta.music.sort_by_key(|m| match m.relation.as_str() { "原声集" => 0, "片头曲" => 1, "片尾曲" => 2, _ => 3 });
        }
        Err(e) => meta.warnings.push(format!("音乐关联未取得：{e}")),
        _ => {}
    }
    meta.enriched = true;
    meta.media_version = 3;
    Ok(meta)
}

#[tauri::command]
pub async fn load_game_album(subject_id: String) -> Result<Value, String> {
    if subject_id.is_empty() || !subject_id.bytes().all(|b| b.is_ascii_digit()) { return Err("音乐条目 ID 无效".into()); }
    let client = client()?;
    let album = response(bangumi_request(client.get(format!("https://api.bgm.tv/v0/subjects/{subject_id}")))).await?;
    if album["type"] != 3 { return Err("此条目不是音乐专辑".into()); }
    let mut tracks = Vec::new();
    // 单张原声集可能有多张 CD；按 API 分页读取，不把前 100 首冒充全部。
    let mut offset = 0;
    loop {
        let page = response(bangumi_request(client.get("https://api.bgm.tv/v0/episodes")
            .query(&[("subject_id",subject_id.as_str()),("limit","100"),("offset",&offset.to_string())]))).await?;
        let items = page["data"].as_array().ok_or("音乐曲目格式无法读取")?;
        tracks.extend(items.iter().map(|t| json!({"title":t["name"],"duration":t["duration"],"disc":t["disc"],"number":t["sort"]})));
        offset += items.len();
        if items.is_empty() || offset >= page["total"].as_u64().unwrap_or(offset as u64) as usize { break; }
        if offset >= 1000 { return Err("音乐曲目过多，请在 Bangumi 查看完整专辑".into()); }
    }
    let info = bgm_metadata(&album).info;
    Ok(json!({"info":info,"tracks":tracks}))
}

#[tauri::command]
pub async fn game_installation_available(directory: String) -> bool {
    tauri::async_runtime::spawn_blocking(move || Path::new(&directory).is_dir()).await.unwrap_or(false)
}

#[tauri::command]
pub async fn launch_library_game(record_id: String, installation_index: usize, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let record = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        crate::records::read_records(&conn)?.into_iter().find(|r| r.id == record_id).ok_or("游戏档案不存在")?
    };
    let _operation = crate::game_operations::GameOperation::start(&record)?;
    let installation = record.library.as_ref().and_then(|l| l.installations.get(installation_index)).ok_or("未设置启动程序")?;
    let directory = std::fs::canonicalize(&installation.directory).map_err(|_| "安装目录不可用，请检查磁盘或网络共享")?;
    let executable = std::fs::canonicalize(&installation.executable).map_err(|_| "启动程序不存在，请重新扫描或选择程序")?;
    if !executable.starts_with(&directory) || !game_executable(executable.file_name().and_then(|n| n.to_str()).unwrap_or("")) { return Err("启动程序不属于此游戏目录或不是游戏程序".into()); }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::playtime::launch(&conn, &record_id, &directory, &executable)?;
    drop(conn);
    crate::scaling::auto_start(&app, &record, &installation.directory);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_staff_excludes_edition_specific_translators_and_keeps_original_credit_names() {
        let metadata = vndb_metadata(&json!({
            "id":"v2002","title":"STEINS;GATE","developers":[{"name":"Nitro+"}],
            "staff":[{"id":"s219","name":"Hayashi Naotaka","original":"林 直孝","role":"scenario","eid":null},
                {"id":"s100","name":"Translator","role":"translator","eid":1}]
        }));
        assert_eq!(metadata.developers, vec!["Nitro+"]);
        assert_eq!(metadata.staff.len(), 1);
        assert_eq!(metadata.staff[0].name, "林 直孝");
        assert_eq!(metadata.staff[0].role, "scenario");
        assert_eq!(metadata.staff[0].source_id.as_deref(), Some("s219"));
    }
    #[test]
    fn scan_groups_versions_and_skips_tools_and_backups() {
        assert_eq!(search_name("11eyes"), "11eyes");
        assert_eq!(search_name("428 ～封鎖された渋谷で～"), "428 ～封鎖された渋谷で～");
        let dir = tempfile::tempdir().unwrap();
        for file in ["2020-Series[通关]/01 - Vol. 1/Game.exe","2020-Series[通关]/01 - Vol. 1/Game_CN.exe","2020-Series[通关]/01 - Vol. 1/unins000.exe",
            "2020-Series[通关]/01 - Vol. 1/原版备份/Game.exe","2020-Series[通关]/02 - Extra/Game.exe","2020-Series[通关]/02 - Extra/lib/windows/python.exe"] {
            let p = dir.path().join(file); std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p, []).unwrap();
        }
        let result = scan(&path_text(dir.path()), SCAN_GENERATION.load(Ordering::Relaxed)).unwrap();
        assert!(result.complete);
        assert_eq!(result.games.len(), 2);
        assert_eq!(result.games[0].title, "Series Vol. 1");
        assert_eq!(result.games[0].installation.candidates.len(), 2);
        assert!(result.games[0].installation.executable.ends_with("Game_CN.exe"));
        let collection = scan(&path_text(&dir.path().join("2020-Series[通关]")), SCAN_GENERATION.load(Ordering::Relaxed)).unwrap();
        assert_eq!(collection.games[0].title, result.games[0].title);
    }
    #[test]
    #[ignore = "real read-only library acceptance; requires WAYPOINT_SCAN_ROOT"]
    fn real_library_scan() {
        let root = std::env::var("WAYPOINT_SCAN_ROOT").unwrap();
        let result = scan(&root, SCAN_GENERATION.load(Ordering::Relaxed)).unwrap();
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        assert!(result.complete);
        assert!(!result.games.is_empty());
        if let Ok(output) = std::env::var("WAYPOINT_SAVE_INSPECTION_OUTPUT") {
            let folders = crate::game_saves::SaveFolders {
                documents: std::env::var_os("WAYPOINT_DOCUMENTS").map(PathBuf::from).unwrap_or_default(),
                roaming: std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default(),
                local: std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default(),
                profile: std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default(),
                ..Default::default()
            };
            let inspection: Vec<Value> = result.games.iter().enumerate().map(|(index, game)| {
                let record: crate::records::GameRecord = serde_json::from_value(json!({
                    "id":format!("inspection-{index}"),"title":game.title,"category":"galgame","status":"backlog","progress":"","rating":0,
                    "library":{"installations":[game.installation]}
                })).unwrap();
                json!({"directory":game.installation.directory,
                    "saves":crate::game_saves::discover(&record,&folders,true),
                    "versions":crate::game_versions::inspect(Path::new(&game.installation.directory)).map_err(|e| e.to_string())})
            }).collect();
            std::fs::write(output,serde_json::to_vec_pretty(&inspection).unwrap()).unwrap();
        }
    }
    #[tokio::test]
    #[ignore = "live metadata acceptance requires network"]
    async fn live_metadata_sources() {
        let bgm = load_game_metadata("bangumi".into(), "3154".into()).await.unwrap();
        assert_eq!(bgm.source_id, "3154");
        assert!(!bgm.cover.is_empty());
        assert!(bgm.tags.iter().any(|t| t == "时间旅行"));
        assert!(bgm.info.iter().any(|i| i.label == "音乐"));
        assert!(bgm.sources.iter().any(|s| s.source == "steam" && s.source_id == "412830"));
        assert!(!bgm.music.is_empty());
        println!("Bangumi {}: {} tags, {} fields, {} videos, {} albums; warnings={:?}", bgm.title, bgm.tags.len(), bgm.info.len(), bgm.videos.len(), bgm.music.len(), bgm.warnings);
        let album = load_game_album(bgm.music[0].id.clone()).await.unwrap();
        assert!(!album["tracks"].as_array().unwrap().is_empty());
        println!("Album {}: {} tracks", bgm.music[0].title, album["tracks"].as_array().unwrap().len());
        if let Ok(path) = std::env::var("WAYPOINT_METADATA_OUTPUT") {
            std::fs::write(path, serde_json::to_string_pretty(&bgm).unwrap()).unwrap();
        }
        let neko = load_game_metadata("vndb".into(), "v15538".into()).await.unwrap();
        assert!(neko.sources.iter().any(|s| s.source == "steam" && s.source_id == "333600"));
        assert!(!neko.sources.iter().any(|s| s.source_id == "947650"), "DLC must not supply the trailer");
        assert!(!neko.videos.is_empty(), "NEKOPARA trailers missing: {:?}", neko.warnings);
        println!("VNDB {}: {} videos, {} albums; sources={:?}; warnings={:?}", neko.title, neko.videos.len(), neko.music.len(), neko.sources, neko.warnings);
        if let Ok(path) = std::env::var("WAYPOINT_MEDIA_OUTPUT") {
            std::fs::write(path, serde_json::to_string_pretty(&neko).unwrap()).unwrap();
        }
        let steam_neko = load_game_metadata("steam".into(), "333600".into()).await.unwrap();
        assert!(steam_neko.sources.iter().any(|s| s.source == "vndb" && s.source_id == "v15538"));
        assert!(!steam_neko.music.is_empty(), "Steam entry must also load linked Bangumi albums");
        assert_eq!(steam_neko.videos.len(), neko.videos.len(), "Primary Steam trailers must not be duplicated");
        println!("Steam reverse link: {} videos, {} albums; warnings={:?}", steam_neko.videos.len(), steam_neko.music.len(), steam_neko.warnings);
        match search_game_metadata("steam".into(), "281990".into()).await {
            Ok(items) => println!("Steam: {} results", items.len()),
            Err(e) => println!("Steam isolated failure: {e}"),
        }
    }
    #[test]
    fn release_media_requires_a_complete_official_single_work() {
        let complete = json!({"official":true,"patch":false,"vns":[{"id":"v15538","rtype":"complete"}],
            "extlinks":[{"url":"https://store.steampowered.com/app/333600/"}]});
        let mut patch = complete.clone(); patch["patch"] = json!(true);
        let mut trial = complete.clone(); trial["vns"][0]["rtype"] = json!("trial");
        let mut bundle = complete.clone(); bundle["vns"].as_array_mut().unwrap().push(json!({"id":"v17763","rtype":"complete"}));
        let mut unofficial = complete.clone(); unofficial["official"] = json!(false);
        let mut other = complete.clone(); other["vns"][0]["id"] = json!("v2002");
        let sources = release_steam_sources(&[patch,trial,bundle,unofficial,other,complete], "v15538");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_id, "333600");
    }
    #[test]
    fn bangumi_fields_keep_aliases_sources_and_production_details() {
        let m = bgm_metadata(&json!({"id":1,"name":"作品","name_cn":"中文作品","tags":[{"name":"科幻"}],"infobox":[
            {"key":"别名","value":[{"v":"Alias"}]},{"key":"音乐","value":"Composer"},
            {"key":"平台","value":[{"v":"PC"},{"v":"PSV"}]},
            {"key":"链接","value":[{"v":"https://vndb.org/v2002"},{"v":"https://store.steampowered.com/app/412830/"}]}
        ]}));
        assert_eq!(m.aliases, ["Alias"]);
        assert_eq!(m.platforms, ["PC","PSV"]);
        assert_eq!(m.tags, ["科幻"]);
        assert!(m.info.iter().any(|i| i.label == "音乐" && i.value == "Composer"));
        assert_eq!(m.sources.len(),2);
        assert!(source_link("https://vndb.org.evil.test/v2002").is_none());
    }
    #[tokio::test]
    #[ignore = "read-only whole-library metadata acceptance; requires fixture and output paths"]
    async fn real_library_metadata() {
        let fixture: Value = serde_json::from_slice(&std::fs::read(std::env::var("WAYPOINT_SCAN_FIXTURE").unwrap()).unwrap()).unwrap();
        let titles: Vec<String> = fixture["games"].as_array().unwrap().iter().map(|g| string(g,"title")).collect();
        let mut results = Vec::new();
        for pair in titles.chunks(2) {
            let query = |title: String| async move {
                let bgm = search_game_metadata("bangumi".into(), title.clone()).await;
                let vn = search_game_metadata("vndb".into(), title.clone()).await;
                json!({"title":title,"candidates":bgm.as_ref().ok(),"error":bgm.as_ref().err(),
                    "alternatives":vn.as_ref().ok(),"alternative_error":vn.as_ref().err()})
            };
            let (a,b) = tokio::join!(query(pair[0].clone()), async { if pair.len() > 1 { Some(query(pair[1].clone()).await) } else { None } });
            results.push(a); if let Some(b)=b { results.push(b); }
            println!("Searched {} / {}",results.len(),titles.len());
        }
        std::fs::write(std::env::var("WAYPOINT_METADATA_OUTPUT").unwrap(), serde_json::to_string_pretty(&results).unwrap()).unwrap();
        assert_eq!(results.len(),titles.len());
    }
}
