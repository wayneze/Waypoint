use serde::{Deserialize, Serialize};
use tauri::State;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use crate::AppState;
use crate::settings::read_app_setting;
use crate::vault::load_vault_item;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamLibraryGame {
    pub vault_item_id: String,
    pub appid: i64,
    pub name: String,
    pub playtime_forever: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playtime_2weeks: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtime_last_played: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub img_icon_url: Option<String>,
}

const STEAM_WEB_API_KEY: &str = "steam_web_api_key";

pub(crate) fn normalize_steamid64(raw: &str) -> Option<String> {
    let value = raw.trim().trim_end_matches('/');
    let id = value.strip_prefix("https://steamcommunity.com/profiles/")
        .or_else(|| value.strip_prefix("http://steamcommunity.com/profiles/"))
        .unwrap_or(value);
    (id.len() == 17 && id.starts_with("7656119") && id.bytes().all(|b| b.is_ascii_digit()))
        .then(|| id.to_owned())
}

fn replace_steam_library(
    conn: &rusqlite::Connection,
    vault_item_id: &str,
    games: &[SteamLibraryGame],
) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM steam_library WHERE vault_item_id = ?1",
        [vault_item_id],
    )?;
    for g in games {
        conn.execute(
            "INSERT INTO steam_library (vault_item_id, appid, name, playtime_forever, playtime_2weeks, rtime_last_played, img_icon_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                g.vault_item_id,
                g.appid,
                g.name,
                g.playtime_forever,
                g.playtime_2weeks,
                g.rtime_last_played,
                g.img_icon_url
            ],
        )?;
    }
    Ok(())
}

fn load_steam_library(
    conn: &rusqlite::Connection,
    vault_item_id: Option<&str>,
) -> rusqlite::Result<Vec<SteamLibraryGame>> {
    if let Some(id) = vault_item_id {
        let mut stmt = conn.prepare(
            "SELECT vault_item_id, appid, name, playtime_forever, playtime_2weeks, rtime_last_played, img_icon_url
             FROM steam_library WHERE vault_item_id = ?1
             ORDER BY playtime_forever DESC, name ASC",
        )?;
        let rows = stmt.query_map([id], map_steam_library_row)?;
        rows.collect()
    } else {
        let mut stmt = conn.prepare(
            "SELECT vault_item_id, appid, name, playtime_forever, playtime_2weeks, rtime_last_played, img_icon_url
             FROM steam_library
             ORDER BY vault_item_id ASC, playtime_forever DESC, name ASC",
        )?;
        let rows = stmt.query_map([], map_steam_library_row)?;
        rows.collect()
    }
}

fn map_steam_library_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SteamLibraryGame> {
    Ok(SteamLibraryGame {
        vault_item_id: row.get(0)?,
        appid: row.get(1)?,
        name: row.get(2)?,
        playtime_forever: row.get(3)?,
        playtime_2weeks: row.get(4)?,
        rtime_last_played: row.get(5)?,
        img_icon_url: row.get(6)?,
    })
}

#[tauri::command]
pub fn get_steam_library(
    vault_item_id: Option<String>,
    state: State<AppState>,
) -> Result<Vec<SteamLibraryGame>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let games = load_steam_library(&conn, vault_item_id.as_deref()).map_err(|e| e.to_string())?;
    // Old data for a changed account card must never be attributed to the new identity.
    let mut valid = BTreeMap::new();
    let mut visible = Vec::new();
    for game in games {
        if !valid.contains_key(&game.vault_item_id) {
            let usable = if let Ok(identity) = account_identity(&conn, &game.vault_item_id) {
                account_cache(&conn, &game.vault_item_id, &identity)?.updated_at.is_some()
            } else { false };
            valid.insert(game.vault_item_id.clone(), usable);
        }
        if valid[&game.vault_item_id] { visible.push(game); }
    }
    Ok(visible)
}

/// Steam Web API key 存凭据管理器，库里只记是否已配置。空串表示清除。
#[tauri::command]
pub fn save_steam_web_api_key(key: String, state: State<AppState>) -> Result<(), String> {
    let mark = crate::secrets::store_secret(STEAM_WEB_API_KEY, key.trim())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)", [STEAM_WEB_API_KEY, &mark])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn steam_web_api_key_configured(state: State<AppState>) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(crate::secrets::load_secret(STEAM_WEB_API_KEY, read_app_setting(&conn, STEAM_WEB_API_KEY)).is_some())
}

const ACCOUNT_PREFIX: &str = "waypoint_steam_account_";
const TITLE_PREFIX: &str = "waypoint_steam_title_";
pub(crate) const LINK_PREFIX: &str = "waypoint_steam_link_";
static SYNC: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub icon_gray: String,
    pub hidden: bool,
    pub unlocked: bool,
    pub unlocked_at: Option<i64>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AchievementCache {
    pub appid: i64,
    pub updated_at: Option<i64>,
    /// ready, unsupported, private, error. Errors retain the last successful items.
    pub status: String,
    pub error: Option<String>,
    pub items: Vec<Achievement>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AccountCache {
    pub account_id: String,
    pub steamid64: String,
    #[serde(default)]
    pub profile: String,
    pub updated_at: Option<i64>,
    pub error: Option<String>,
    pub achievements: BTreeMap<i64, AchievementCache>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteamLink { pub record_id: String, pub appid: i64 }

#[derive(Debug, Serialize)]
pub struct SteamActivity {
    pub links: Vec<SteamLink>,
    pub accounts: Vec<AccountCache>,
    pub installed: Vec<InstalledGame>,
    pub titles: BTreeMap<i64, SteamTitle>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SteamTitle {
    pub title: Option<String>,
    pub source: Option<String>,
    pub checked_at: i64,
    pub error: Option<String>,
}

fn load_titles(conn: &rusqlite::Connection) -> Result<BTreeMap<i64, SteamTitle>, String> {
    let mut stmt = conn.prepare("SELECT substr(key,length(?1)+1),value FROM app_settings WHERE substr(key,1,length(?1))=?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([TITLE_PREFIX], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))).map_err(|e| e.to_string())?;
    let mut titles = BTreeMap::new();
    for row in rows {
        let (id, value) = row.map_err(|e| e.to_string())?;
        if let Ok(id) = id.parse() { titles.insert(id, serde_json::from_str(&value).map_err(|e| format!("中文名称缓存无法读取：{e}"))?); }
    }
    Ok(titles)
}

fn chinese_title(value: &str) -> bool { value.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) }

fn linked_chinese_title(appid: i64, original: &str, candidates: &[crate::game_library::Metadata]) -> Option<(String, String)> {
    let id = appid.to_string();
    let localized: Vec<_> = candidates.iter().filter(|m| chinese_title(&m.title) && m.title != m.original_title).collect();
    let linked: Vec<_> = localized.iter().copied().filter(|m| m.sources.iter().any(|s| s.source == "steam" && s.source_id == id)).collect();
    let normalize = |s: &str| s.chars().filter(|c|c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let key = normalize(original);
    let matches = if !linked.is_empty() { linked } else {
        localized.into_iter().filter(|m| !key.is_empty() && normalize(&m.original_title) == key
            && !m.sources.iter().any(|s|s.source=="steam" && s.source_id!=id)).collect()
    };
    (matches.len() == 1).then(|| (matches[0].title.clone(), matches[0].url.clone()))
}

/// Public names are resolved independently of account sync; a slow name service never blocks the library.
async fn resolve_title(appid: i64, original: &str) -> SteamTitle {
    let (store, bangumi) = tokio::join!(
        crate::game_library::search_game_metadata("steam".into(), appid.to_string()),
        crate::game_library::search_game_metadata("bangumi".into(), original.into())
    );
    let official = store.as_ref().ok().and_then(|items| items.iter().find(|m| chinese_title(&m.title)))
        .map(|m| (m.title.clone(), m.url.clone()));
    let linked = bangumi.as_ref().ok().and_then(|items| linked_chinese_title(appid, original, items));
    let found = official.or(linked);
    let error = if found.is_some() { None } else {
        let failed: Vec<_> = [store.err(), bangumi.err()].into_iter().flatten().collect();
        (!failed.is_empty()).then(|| failed.join("；"))
    };
    SteamTitle { title: found.as_ref().map(|v| v.0.clone()), source: found.map(|v| v.1),
        checked_at: chrono::Utc::now().timestamp(), error }
}

#[tauri::command]
pub async fn resolve_steam_title(appid: i64, retry: bool, state: State<'_, AppState>) -> Result<SteamTitle, String> {
    let key = format!("{TITLE_PREFIX}{appid}");
    let (name, old) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let old: SteamTitle = read_json(&conn, &key)?;
        if !retry && old.checked_at > 0 { return Ok(old); }
        let name: String = conn.query_row("SELECT name FROM steam_library WHERE appid=?1 LIMIT 1", [appid], |r|r.get(0))
            .map_err(|_| "库存中没有这款游戏，请先刷新列表")?;
        (name, old)
    };
    let mut title = resolve_title(appid, &name).await;
    if title.title.is_none() && old.title.is_some() && title.error.is_some() {
        title.title = old.title; title.source = old.source;
    }
    write_json(&*state.db.lock().map_err(|e| e.to_string())?, &key, &title)?;
    Ok(title)
}
pub(crate) fn read_json<T: serde::de::DeserializeOwned + Default>(conn: &rusqlite::Connection, key: &str) -> Result<T, String> {
    let value: Option<String> = conn.query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r| r.get(0))
        .optional().map_err(|e| e.to_string())?;
    value.map(|v| serde_json::from_str(&v).map_err(|e| format!("读取游玩数据失败：{e}"))).unwrap_or_else(|| Ok(T::default()))
}

pub(crate) fn write_json<T: Serialize>(conn: &rusqlite::Connection, key: &str, value: &T) -> Result<(), String> {
    conn.execute("INSERT OR REPLACE INTO app_settings(key,value) VALUES (?1,?2)",
        rusqlite::params![key, serde_json::to_string(value).map_err(|e| e.to_string())?]).map_err(|e| e.to_string())?;
    Ok(())
}

fn account_identity(conn: &rusqlite::Connection, id: &str) -> Result<String, String> {
    let item = load_vault_item(conn, id).map_err(|_| "Steam 账号已不存在")?;
    if !item.platform.to_lowercase().contains("steam") { return Err("请选择 Steam 账号卡".into()); }
    let raw = item.steamid64.as_deref().unwrap_or("").trim();
    if let Some(id) = normalize_steamid64(raw) { return Ok(id); }
    if let Ok(url) = reqwest::Url::parse(raw) {
        let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
        if matches!(url.scheme(), "http" | "https") && url.host_str() == Some("steamcommunity.com")
            && parts.len() == 2 && parts[0] == "id" && !parts[1].is_empty() {
            return Ok(format!("https://steamcommunity.com/id/{}", parts[1]));
        }
    }
    Err("请在金库账号卡填写 17 位 SteamID64 或 steamcommunity.com 的 profiles / id 资料页链接".into())
}

fn credentials(conn: &rusqlite::Connection, id: &str) -> Result<(String, String), String> {
    let steamid = account_identity(conn, id)?;
    let key = crate::secrets::load_secret(STEAM_WEB_API_KEY, read_app_setting(conn, STEAM_WEB_API_KEY))
        .filter(|s| !s.trim().is_empty()).ok_or("请先在金库的 Steam 连接中填写 API key")?;
    Ok((key, steamid))
}

fn account_cache(conn: &rusqlite::Connection, id: &str, steamid: &str) -> Result<AccountCache, String> {
    let cache: AccountCache = read_json(conn, &format!("{ACCOUNT_PREFIX}{id}"))?;
    Ok(if cache.profile == steamid || (cache.profile.is_empty() && cache.steamid64 == steamid) { cache } else {
        AccountCache { account_id: id.into(), profile: steamid.into(), ..Default::default() }
    })
}

fn load_links(conn: &rusqlite::Connection) -> Result<Vec<SteamLink>, String> {
    let mut stmt = conn.prepare("SELECT s.value FROM app_settings s JOIN game_records g ON s.key=?1 || g.id").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([LINK_PREFIX], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub async fn get_steam_activity(state: State<'_, AppState>) -> Result<SteamActivity, String> {
    let (links, accounts, titles) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id FROM vault_items WHERE lower(platform) LIKE '%steam%'").map_err(|e| e.to_string())?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>().map_err(|e| e.to_string())?;
        let mut accounts = Vec::new();
        for id in ids {
            if let Ok(identity) = account_identity(&conn, &id) { accounts.push(account_cache(&conn, &id, &identity)?); }
        }
        (load_links(&conn)?, accounts, load_titles(&conn)?)
    };
    let installed = tauri::async_runtime::spawn_blocking(installed_games).await.map_err(|e| e.to_string())?;
    Ok(SteamActivity { links, accounts, installed, titles })
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().connect_timeout(crate::PUBLIC_CONNECT_TIMEOUT).timeout(crate::PUBLIC_REQUEST_TIMEOUT)
        .build().map_err(|e| e.without_url().to_string())
}

async fn api(client: &reqwest::Client, endpoint: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let res = client.get(format!("https://api.steampowered.com/{endpoint}/")).query(params).send().await
        .map_err(|e| format!("Steam 网络请求失败：{}", e.without_url()))?;
    if res.status() == reqwest::StatusCode::BAD_REQUEST {
        // Steam uses HTTP 400 for structured player-stat outcomes, including games without stats.
        let value: Value = res.json().await.map_err(|_| "Steam HTTP 400".to_owned())?;
        return if value["playerstats"]["success"] == false { Ok(value) } else { Err("Steam HTTP 400".into()) };
    }
    if !res.status().is_success() {
        return Err(match res.status().as_u16() {
            401 | 403 => "Steam 拒绝访问：请检查 Web API key 和账号隐私设置".into(),
            429 => "Steam 请求过于频繁，请稍后重试".into(),
            code => format!("Steam HTTP {code}"),
        });
    }
    res.json().await.map_err(|e| format!("Steam 响应无法解析：{}", e.without_url()))
}

async fn resolve_identity(client: &reqwest::Client, key: &str, profile: &str) -> Result<String, String> {
    if let Some(id) = normalize_steamid64(profile) { return Ok(id); }
    let vanity = profile.strip_prefix("https://steamcommunity.com/id/").ok_or("Steam 资料页地址无效")?;
    let value = api(client, "ISteamUser/ResolveVanityURL/v1", &[("key", key), ("vanityurl", vanity)]).await?;
    value["response"]["steamid"].as_str().and_then(normalize_steamid64).ok_or_else(|| "未找到这个 Steam 自定义资料页，请核对链接".into())
}

fn parse_library(json: &Value, id: &str) -> Result<Vec<SteamLibraryGame>, String> {
    let response = json.get("response").ok_or("Steam 未返回游戏库")?;
    let empty = Vec::new();
    let games = match response.get("games") {
        Some(v) => v.as_array().ok_or("Steam 游戏库格式异常，已保留缓存")?,
        None if response["game_count"].as_u64() == Some(0) => &empty,
        None => return Err("无法读取游戏库，请将 Steam 资料的「游戏详情」设为公开并检查账号与 key".into()),
    };
    games.iter().map(|g| {
        let appid = g["appid"].as_i64().filter(|v| *v > 0).ok_or("Steam 返回了无效 AppID")?;
        let name = g["name"].as_str().filter(|n| !n.trim().is_empty()).ok_or("Steam 返回了缺少名称的游戏")?;
        Ok(SteamLibraryGame { vault_item_id: id.into(), appid, name: name.into(),
            playtime_forever: g["playtime_forever"].as_i64().unwrap_or(0).max(0),
            playtime_2weeks: g["playtime_2weeks"].as_i64(),
            rtime_last_played: g["rtime_last_played"].as_i64().filter(|v| *v > 0),
            img_icon_url: g["img_icon_url"].as_str().map(str::to_owned) })
    }).collect()
}

#[tauri::command]
pub async fn refresh_steam_library(vault_item_id: String, state: State<'_, AppState>) -> Result<Vec<SteamLibraryGame>, String> {
    let _sync = SYNC.lock().await;
    let identity = { let conn = state.db.lock().map_err(|e| e.to_string())?; account_identity(&conn, &vault_item_id)? };
    let result = async {
        let key = { let conn = state.db.lock().map_err(|e| e.to_string())?; credentials(&conn, &vault_item_id)?.0 };
        let client = client()?;
        let resolved = resolve_identity(&client, &key, &identity).await?;
        let value = api(&client, "IPlayerService/GetOwnedGames/v1",
            &[("key", &key), ("steamid", &resolved), ("include_appinfo", "1"), ("include_played_free_games", "1")]).await?;
        Ok::<_, String>((resolved, parse_library(&value, &vault_item_id)?))
    }.await;
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    if account_identity(&conn, &vault_item_id)? != identity { return Err("同步期间账号已更换，请重新同步".into()); }
    let mut cache = account_cache(&conn, &vault_item_id, &identity)?;
    match result {
        Ok((resolved, games)) => {
            cache.steamid64 = resolved; cache.profile = identity.clone();
            let tx = conn.transaction().map_err(|e| e.to_string())?;
            replace_steam_library(&tx, &vault_item_id, &games).map_err(|e| e.to_string())?;
            cache.updated_at = Some(chrono::Utc::now().timestamp()); cache.error = None;
            write_json(&tx, &format!("{ACCOUNT_PREFIX}{vault_item_id}"), &cache)?;
            tx.commit().map_err(|e| e.to_string())?;
            Ok(games)
        }
        Err(error) => { cache.error = Some(error.clone()); write_json(&conn, &format!("{ACCOUNT_PREFIX}{vault_item_id}"), &cache)?; Err(error) }
    }
}

fn parse_achievements(schema: &Value, player: &Value, appid: i64) -> Result<AchievementCache, String> {
    let game = schema.get("game").filter(|g| g.is_object()).ok_or("Steam 未返回成就定义")?;
    let definitions = match game.pointer("/availableGameStats/achievements") {
        Some(v) => v.as_array().ok_or("Steam 成就定义格式异常")?,
        None => {
            // A successful schema without achievements is distinct from an empty/error response.
            let no_stats = player["playerstats"]["success"] == false
                && player["playerstats"]["error"].as_str() == Some("Requested app has no stats");
            if !no_stats && game.get("gameName").is_none() && game.get("gameVersion").is_none() { return Err("Steam 成就定义暂不可用".into()); }
            return Ok(AchievementCache { appid, status: "unsupported".into(), updated_at: Some(chrono::Utc::now().timestamp()), ..Default::default() });
        }
    };
    if definitions.is_empty() { return Ok(AchievementCache { appid, status: "unsupported".into(), updated_at: Some(chrono::Utc::now().timestamp()), ..Default::default() }); }
    let stats = &player["playerstats"];
    if stats["success"] != true { return Err("无法读取玩家成就：请检查游戏详情隐私与账号访问权限".into()); }
    let progress = stats["achievements"].as_array().ok_or("Steam 玩家成就格式异常")?;
    let mut items = Vec::new();
    for d in definitions {
        let id = d["name"].as_str().ok_or("Steam 成就缺少标识")?;
        let p = progress.iter().find(|p| p["apiname"].as_str() == Some(id)).ok_or("Steam 成就列表不完整，请稍后重试")?;
        let unlocked = p["achieved"].as_i64() == Some(1);
        items.push(Achievement { id: id.into(), name: d["displayName"].as_str().unwrap_or(id).into(),
            description: d["description"].as_str().unwrap_or("").into(), icon: d["icon"].as_str().unwrap_or("").into(),
            icon_gray: d["icongray"].as_str().unwrap_or("").into(), hidden: d["hidden"].as_i64() == Some(1),
            unlocked, unlocked_at: p["unlocktime"].as_i64().filter(|v| unlocked && *v > 0) });
    }
    Ok(AchievementCache { appid, items, status: "ready".into(), updated_at: Some(chrono::Utc::now().timestamp()), error: None })
}

#[tauri::command]
pub async fn refresh_steam_achievements(vault_item_id: String, appid: i64, state: State<'_, AppState>) -> Result<AchievementCache, String> {
    if appid <= 0 { return Err("无效 AppID".into()); }
    let _sync = SYNC.lock().await;
    let identity = { let conn = state.db.lock().map_err(|e| e.to_string())?; account_identity(&conn, &vault_item_id)? };
    let result = async {
        let key = { let conn = state.db.lock().map_err(|e| e.to_string())?; credentials(&conn, &vault_item_id)?.0 };
    let client = client()?;
    let resolved = resolve_identity(&client, &key, &identity).await?;
    let id = appid.to_string();
    let schema_params = [("key", key.as_str()), ("appid", id.as_str()), ("l", "schinese")];
    let player_params = [("key", key.as_str()), ("appid", id.as_str()), ("steamid", resolved.as_str()), ("l", "schinese")];
    let (schema, player) = tokio::join!(
        api(&client, "ISteamUserStats/GetSchemaForGame/v2", &schema_params),
        api(&client, "ISteamUserStats/GetPlayerAchievements/v1", &player_params)
    );
    // An empty schema needs the player's explicit "no stats" outcome; privacy and network failures stay errors.
    schema.and_then(|s| {
        match player {
            Ok(p) => parse_achievements(&s, &p, appid),
            Err(_) if s.pointer("/game/availableGameStats/achievements").is_none()
                || s.pointer("/game/availableGameStats/achievements").and_then(Value::as_array).is_some_and(Vec::is_empty)
                => parse_achievements(&s, &json!({}), appid),
            Err(error) => Err(error),
        }
    })
    }.await;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if account_identity(&conn, &vault_item_id)? != identity { return Err("同步期间账号已更换，请重新同步".into()); }
    let mut account = account_cache(&conn, &vault_item_id, &identity)?;
    let cache = match result {
        Ok(cache) => cache,
        Err(error) => {
            let mut old = account.achievements.remove(&appid).unwrap_or(AchievementCache { appid, ..Default::default() });
            old.status = if error.contains("隐私") || error.contains("访问权限") { "private" } else { "error" }.into();
            old.error = Some(error); old
        }
    };
    account.achievements.insert(appid, cache.clone());
    write_json(&conn, &format!("{ACCOUNT_PREFIX}{vault_item_id}"), &account)?;
    Ok(cache)
}

#[derive(Deserialize)]
pub struct ImportChoice {
    appid: i64,
    record_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    guide_id: Option<String>,
}

fn record_appids(record: &crate::records::GameRecord) -> Vec<i64> {
    let mut ids = Vec::new();
    if let Some(library) = &record.library {
        ids.extend(library.installations.iter().filter_map(|i| i.steam_id.as_deref()?.parse::<i64>().ok()));
        if let Some(m) = &library.metadata {
            if m.source == "steam" { ids.extend(m.source_id.parse::<i64>()); }
            ids.extend(m.sources.iter().filter(|s| s.source == "steam").filter_map(|s| s.source_id.parse::<i64>().ok()));
        }
    }
    ids
}

fn import_games(conn: &mut rusqlite::Connection, account: &str, choices: Vec<ImportChoice>) -> Result<Vec<String>, String> {
    let identity = account_identity(conn, account)?;
    let cache = account_cache(conn, account, &identity)?;
    if cache.updated_at.is_none() { return Err("请先成功同步当前 Steam 账号，再选择导入".into()); }
    let games = load_steam_library(conn, Some(account)).map_err(|e| e.to_string())?;
    let titles = load_titles(conn)?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut imported = Vec::new();
    for choice in choices {
        let game = games.iter().find(|g| g.appid == choice.appid).ok_or("选择的游戏已不在缓存中，请刷新库存")?;
        let records = crate::records::read_records(&tx)?;
        let links = load_links(&tx)?;
        let bound = links.iter().find(|l| l.appid == game.appid);
        let exact: Vec<_> = records.iter().filter(|r| record_appids(r).contains(&game.appid)).collect();
        let target = if let Some(bound) = bound {
            if choice.record_id.as_deref().is_some_and(|id| id != bound.record_id) { return Err("这款 Steam 游戏已关联其他档案，请先解除原关联".into()); }
            Some(bound.record_id.clone())
        } else if let Some(id) = choice.record_id { Some(id) }
        else if exact.len() > 1 { return Err(format!("{} 匹配到多个档案，请明确选择合并目标", game.name)); }
        else { exact.first().map(|r| r.id.clone()) };
        let record_id = match target {
            Some(id) => {
                if !records.iter().any(|r| r.id == id) { return Err("合并目标已不存在".into()); }
                if links.iter().any(|l| l.record_id == id && l.appid != game.appid) { return Err("该档案已关联其他 Steam 游戏".into()); }
                id
            }
            None => {
                let id = format!("steam_{}", game.appid);
                if records.iter().any(|r| r.id == id) { return Err("已有同名导入档案，请选择该档案关联".into()); }
                let title = choice.title.filter(|t| !t.trim().is_empty()).or_else(||titles.get(&game.appid).and_then(|t|t.title.clone())).unwrap_or_else(||game.name.clone());
                let record = crate::records::GameRecord { id: id.clone(), title, category: "steam".into(),
                    status: "backlog".into(), progress: String::new(), rating: 0.0, save_path: None, notes: None,
                    poster: Some(format!("https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/{}/library_600x900.jpg", game.appid)),
                    guide_id: choice.guide_id, poster_position: None, library: None };
                crate::records::write_record(&tx, &record, true)?;
                id
            }
        };
        write_json(&tx, &format!("{LINK_PREFIX}{record_id}"), &SteamLink { record_id: record_id.clone(), appid: game.appid })?;
        // Only an explicit import may restore a previously removed item.
        let mut removed: Vec<String> = read_json(&tx, "waypoint_removed_guides")?;
        removed.retain(|id| id != &record_id);
        write_json(&tx, "waypoint_removed_guides", &removed)?;
        imported.push(record_id);
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(imported)
}

#[tauri::command]
pub fn import_steam_games(vault_item_id: String, choices: Vec<ImportChoice>, state: State<AppState>) -> Result<Vec<String>, String> {
    import_games(&mut *state.db.lock().map_err(|e| e.to_string())?, &vault_item_id, choices)
}

#[tauri::command]
pub fn unlink_steam_game(record_id: String, state: State<AppState>) -> Result<(), String> {
    state.db.lock().map_err(|e| e.to_string())?.execute("DELETE FROM app_settings WHERE key=?1", [format!("{LINK_PREFIX}{record_id}")]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn launch_steam_game(record_id: String, app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let appid = load_links(&*state.db.lock().map_err(|e| e.to_string())?)?.into_iter()
        .find(|l| l.record_id == record_id).ok_or("此档案尚未关联 Steam")?.appid;
    if appid <= 0 { return Err("无效 AppID".into()); }
    app.opener().open_url(format!("steam://rungameid/{appid}"), None::<&str>).map_err(|e| format!("无法启动 Steam：{e}"))?;
    if let Some(installed) = installed_games().into_iter().find(|i| i.appid == appid) {
        let record = {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            crate::records::read_records(&conn)?.into_iter().find(|r| r.id == record_id)
        };
        if let Some(record) = record { crate::scaling::auto_start(&app, &record, &installed.directory); }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct InstalledGame { pub appid: i64, pub directory: String }

fn quoted_values(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' { continue; }
        let mut value = String::new();
        while let Some(c) = chars.next() {
            if c == '"' { break; }
            if c == '\\' { if let Some(next) = chars.next() { value.push(next); } } else { value.push(c); }
        }
        values.push(value);
    }
    values
}

fn installed_games() -> Vec<InstalledGame> {
    let mut roots = Vec::new();
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        if let Ok(output) = std::process::Command::new("reg").args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
            .creation_flags(0x08000000).output() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(path) = text.lines().find_map(|line| line.split_once("REG_SZ").map(|(_, p)| p.trim())) {
                roots.push(std::path::PathBuf::from(path));
            }
        }
    }
    if let Some(root) = roots.first() {
        if let Ok(text) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            let values = quoted_values(&text);
            roots.extend(values.windows(2).filter(|p| p[0] == "path").map(|p| std::path::PathBuf::from(&p[1])).collect::<Vec<_>>());
        }
    }
    roots.sort(); roots.dedup();
    let mut installed = Vec::new();
    for root in roots {
        let apps = root.join("steamapps");
        let Ok(entries) = std::fs::read_dir(&apps) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_prefix("appmanifest_").and_then(|n| n.strip_suffix(".acf")).and_then(|n| n.parse::<i64>().ok()) else { continue };
            let Ok(text) = std::fs::read_to_string(entry.path()) else { continue };
            let values = quoted_values(&text);
            let get = |key: &str| values.windows(2).find(|p| p[0].eq_ignore_ascii_case(key)).map(|p| p[1].as_str());
            if get("StateFlags").and_then(|v| v.parse::<u32>().ok()).is_some_and(|v| v & 4 != 0) {
                if let Some(dir) = get("installdir") {
                    let path = apps.join("common").join(dir);
                    if path.is_dir() { installed.push(InstalledGame { appid: id, directory: path.to_string_lossy().into() }); }
                }
            }
        }
    }
    installed
}

pub(crate) fn remove_account_cache(conn: &rusqlite::Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM app_settings WHERE key=?1", [format!("{ACCOUNT_PREFIX}{id}")]).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn import_is_atomic_idempotent_and_keeps_personal_fields() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::init_database_schema(&conn).unwrap();
        conn.execute("INSERT INTO vault_items(id,platform,account_name,payload) VALUES ('a','Steam','fixture',?1)",
            [r#"{"steamid64":"76561198000000000"}"#]).unwrap();
        let cache = AccountCache { account_id: "a".into(), steamid64: "76561198000000000".into(), profile: "76561198000000000".into(), updated_at: Some(1), ..Default::default() };
        write_json(&conn, &format!("{ACCOUNT_PREFIX}a"), &cache).unwrap();
        let games = parse_library(&json!({"response":{"game_count":2,"games":[
            {"appid":10,"name":"Original","playtime_forever":100},{"appid":20,"name":"Another","playtime_forever":200}]}}), "a").unwrap();
        replace_steam_library(&conn, "a", &games).unwrap();
        let record: crate::records::GameRecord = serde_json::from_value(json!({
            "id":"personal","title":"我的标题","category":"single","status":"playing","progress":"第三章","rating":8.5,
            "notes":"我的感想","poster":"https://example.com/custom.jpg","guide_id":"guide-one"
        })).unwrap();
        crate::records::write_record(&conn, &record, true).unwrap();
        for _ in 0..2 {
            import_games(&mut conn, "a", vec![ImportChoice { appid:10, record_id:Some("personal".into()), title:None, guide_id:None }]).unwrap();
        }
        let loaded = crate::records::read_records(&conn).unwrap();
        assert_eq!(loaded.len(),1);
        assert_eq!(serde_json::to_value(&loaded[0]).unwrap(),serde_json::to_value(record).unwrap());
        assert_eq!(load_links(&conn).unwrap().len(),1);
        assert!(import_games(&mut conn,"a",vec![ImportChoice{appid:20,record_id:None,title:None,guide_id:None},ImportChoice{appid:999,record_id:None,title:None,guide_id:None}]).is_err());
        assert_eq!(crate::records::read_records(&conn).unwrap().len(),1);
        conn.execute_batch("CREATE TRIGGER fail_library BEFORE INSERT ON steam_library BEGIN SELECT RAISE(FAIL,'disk full'); END;").unwrap();
        {
            let tx=conn.transaction().unwrap();
            assert!(replace_steam_library(&tx,"a",&games).is_err());
        }
        assert_eq!(load_steam_library(&conn,Some("a")).unwrap().len(),2);
        write_json(&conn, &format!("{TITLE_PREFIX}20"), &SteamTitle { title: Some("另一个游戏".into()), source: Some("https://bgm.tv/subject/1".into()), checked_at:1, error:None }).unwrap();
        import_games(&mut conn,"a",vec![ImportChoice{appid:20,record_id:None,title:None,guide_id:Some("guide-two".into())}]).unwrap();
        let localized = crate::records::read_records(&conn).unwrap().into_iter().find(|r|r.id=="steam_20").unwrap();
        assert_eq!(localized.title,"另一个游戏");
        assert_eq!(localized.guide_id.as_deref(),Some("guide-two"));
    }

    #[test]
    fn chinese_names_require_unique_steam_identity() {
        let base = crate::game_library::Metadata { title:"命运石之门".into(),url:"https://bgm.tv/subject/3154".into(),
            sources:vec![crate::game_library::MetadataSource {source:"steam".into(),source_id:"412830".into(),url:"https://store.steampowered.com/app/412830/".into()}],
            ..Default::default() };
        assert_eq!(linked_chinese_title(412830,"STEINS;GATE",&[base.clone()]).unwrap().0,"命运石之门");
        assert!(linked_chinese_title(825630,"STEINS;GATE 0",&[base.clone()]).is_none());
        assert!(linked_chinese_title(412830,"STEINS;GATE",&[base.clone(),base.clone()]).is_none());
        assert!(linked_chinese_title(412830,"STEINS;GATE",&[crate::game_library::Metadata{sources:vec![],..base}]).is_none());
        let exact = crate::game_library::Metadata {title:"群星".into(),original_title:"Stellaris".into(),url:"https://bgm.tv/subject/151871".into(),..Default::default()};
        assert_eq!(linked_chinese_title(281990,"Stellaris",&[exact.clone()]).unwrap().0,"群星");
        assert!(linked_chinese_title(281990,"Stellaris: Utopia",&[exact]).is_none());
    }

    #[test]
    fn distinguishes_private_unsupported_and_incomplete_achievements() {
        assert!(parse_library(&json!({"response":{}}),"a").is_err());
        assert!(parse_library(&json!({"response":{"games":"invalid"}}),"a").is_err());
        assert!(parse_achievements(&json!({"game":{}}),&json!({}),10).is_err());
        let no_stats = json!({"playerstats":{"success":false,"error":"Requested app has no stats"}});
        let unsupported = parse_achievements(&json!({"game":{}}), &no_stats, 702050).unwrap();
        assert_eq!(unsupported.status, "unsupported");
        assert!(unsupported.error.is_none());
        assert!(parse_achievements(&json!({"game":{}}), &json!({"playerstats":{"success":false,"error":"Profile is not public"}}), 702050).is_err());
        assert_eq!(parse_achievements(&json!({"game":{"gameName":"No achievements","gameVersion":"1"}}),&json!({}),10).unwrap().status,"unsupported");
        let schema=json!({"game":{"gameName":"fixture","availableGameStats":{"achievements":[
            {"name":"A","displayName":"First","description":"Description","hidden":0,"icon":"https://example.com/a","icongray":"https://example.com/b"},
            {"name":"B","displayName":"Secret ending","description":"Spoiler","hidden":1}]}}});
        assert!(parse_achievements(&schema, &no_stats, 10).is_err());
        assert!(parse_achievements(&schema,&json!({"playerstats":{"success":false}}),10).unwrap_err().contains("隐私"));
        assert!(parse_achievements(&schema,&json!({"playerstats":{"success":true,"achievements":[]}}),10).is_err());
        let result=parse_achievements(&schema,&json!({"playerstats":{"success":true,"achievements":[{"apiname":"B","achieved":0,"unlocktime":0},{"apiname":"A","achieved":1,"unlocktime":100}]}}),10).unwrap();
        assert!(result.items[0].unlocked);assert_eq!(result.items[0].unlocked_at,Some(100));
        assert!(result.items[1].hidden);assert!(!result.items[1].unlocked);
        assert_eq!(normalize_steamid64("https://steamcommunity.com/profiles/76561198000000000/"),Some("76561198000000000".into()));
        assert_eq!(normalize_steamid64("garbage76561198000000000"),None);
    }

}
