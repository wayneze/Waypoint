//! 换日与提醒。后台线程每分钟跑一次 `tick`：按各游戏复位钟把过期的日课/周常勾选清掉，
//! 再按设置发提醒。应用缩在托盘、窗口没开时也照常执行，前端只负责显示结果。

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::Value;

// ---- 设置键（和前端共用 app_settings）----
pub(crate) const NOTIFY_DAILY_KEY: &str = "waypoint_notify_daily";
pub(crate) const NOTIFY_WEEKLY_KEY: &str = "waypoint_notify_weekly";
pub(crate) const NOTIFY_EVENT_END_KEY: &str = "waypoint_notify_event_end";
pub(crate) const NOTIFY_DAILY_TIME_KEY: &str = "waypoint_notify_daily_time";
const RESET_OVERRIDE_KEY: &str = "waypoint_game_reset_v1";
const ENABLED_GAMES_KEY: &str = "enabled_games";
const TASK_CYCLE_KEY: &str = "waypoint_task_cycle_v2";
const LAST_DAILY_NOTIFY_KEY: &str = "waypoint_notify_daily_last";
const LAST_WEEKLY_KEY: &str = "waypoint_notify_weekly_last";
const EVENT_END_SENT_KEY: &str = "waypoint_notify_event_end_sent";
/// 前端"国服可能会来"里关注的先祖名，JSON 字符串数组
const SKY_WATCH_KEY: &str = "waypoint_sky_watch_spirits";
const SKY_WATCH_SENT_KEY: &str = "waypoint_sky_watch_sent";
/// cache.db 里光遇档期的键，和前端 skyCalendarService 一致
const SKY_CALENDAR_CACHE_KEY: &str = "sky.calendar";
const DEFAULT_DAILY_TIME: &str = "20:00";

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GameMeta {
    id: String,
    label: String,
    kind: String,
    daily_reset: String,
    weekly_reset: String,
    #[serde(default)]
    nav_label: Option<String>,
}

fn catalog() -> Vec<GameMeta> {
    serde_json::from_str(include_str!("../../src/content/games.json")).unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ResetClock {
    hour: u32,
    minute: u32,
    weekday: Option<u32>,
}

const WEEKDAYS: &[(&str, u32)] = &[
    ("星期日", 0), ("星期天", 0), ("周日", 0), ("周天", 0),
    ("星期一", 1), ("周一", 1), ("星期二", 2), ("周二", 2),
    ("星期三", 3), ("周三", 3), ("星期四", 4), ("周四", 4),
    ("星期五", 5), ("周五", 5), ("星期六", 6), ("周六", 6),
];

/// 第一个 `H:MM` / `HH:MM`。
fn parse_hm(src: &str) -> Option<(u32, u32)> {
    let chars: Vec<char> = src.chars().collect();
    (0..chars.len()).filter(|&i| chars[i] == ':').find_map(|i| {
        let h: String = chars[i.saturating_sub(2)..i].iter().skip_while(|c| !c.is_ascii_digit()).collect();
        let m: String = chars.get(i + 1..i + 3)?.iter().collect();
        if h.is_empty() || !h.chars().chain(m.chars()).all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some((h.parse().ok()?, m.parse().ok()?))
    })
}

/// 和前端 `parseResetClock` 同规则：先找星期，再取第一个 HH:MM，缺了用 fallback。
pub(crate) fn parse_reset_clock(rule: &str, fallback: &str) -> ResetClock {
    let src = if rule.trim().is_empty() { fallback } else { rule.trim() };
    let weekday = WEEKDAYS.iter().find(|(label, _)| src.contains(label)).map(|(_, d)| *d);
    let (hour, minute) = parse_hm(src).or_else(|| parse_hm(fallback)).unwrap_or((0, 0));
    ResetClock { hour: hour.min(23), minute: minute.min(59), weekday }
}

fn at(day: NaiveDate, c: ResetClock) -> NaiveDateTime {
    day.and_time(NaiveTime::from_hms_opt(c.hour, c.minute, 0).unwrap_or_default())
}

pub(crate) fn last_daily_boundary(now: NaiveDateTime, c: ResetClock) -> NaiveDateTime {
    let today = at(now.date(), c);
    if now >= today { today } else { today - Duration::days(1) }
}

pub(crate) fn last_weekly_boundary(now: NaiveDateTime, c: ResetClock) -> NaiveDateTime {
    let wd = c.weekday.unwrap_or(1) % 7;
    let mut cand = at(now.date(), c);
    let delta = (cand.weekday().num_days_from_sunday() + 7 - wd) % 7;
    cand -= Duration::days(delta as i64);
    if now < cand {
        cand -= Duration::days(7);
    }
    cand
}

/// 和前端 `periodStamp` 同格式，旧记录可直接沿用。
fn period_stamp(d: NaiveDateTime) -> String {
    d.format("%Y-%m-%dT%H:%M").to_string()
}

// ---- 游戏、角色与设置 ----

struct Settings(HashMap<String, String>);

impl Settings {
    fn load(conn: &Connection) -> rusqlite::Result<Self> {
        let mut stmt = conn.prepare("SELECT key, value FROM app_settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        Ok(Settings(rows.collect::<rusqlite::Result<_>>()?))
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }

    fn json(&self, key: &str) -> Option<Value> {
        self.get(key).and_then(|s| serde_json::from_str(s).ok())
    }

    /// 开关存成 JSON 布尔，缺省为开
    fn flag(&self, key: &str) -> bool {
        self.json(key).and_then(|v| v.as_bool()).unwrap_or(true)
    }
}

fn save(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)", [key, value])?;
    Ok(())
}

struct Games {
    list: Vec<GameMeta>,
    overrides: Value,
    enabled: Option<Vec<String>>,
}

impl Games {
    fn load(settings: &Settings) -> Self {
        let enabled = settings.json(ENABLED_GAMES_KEY).and_then(|v| {
            v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        });
        Games {
            list: catalog(),
            overrides: settings.json(RESET_OVERRIDE_KEY).unwrap_or(Value::Null),
            enabled,
        }
    }

    fn meta(&self, id: &str) -> Option<&GameMeta> {
        self.list.iter().find(|g| g.id == id)
    }

    fn label(&self, id: &str) -> String {
        self.meta(id).map(|g| g.label.clone()).unwrap_or_else(|| id.to_string())
    }

    fn clocks(&self, id: &str) -> (ResetClock, ResetClock) {
        let base = self.meta(id);
        let ov = self.overrides.get(id);
        let pick = |field: &str, fallback: &str| {
            ov.and_then(|o| o.get(field)).and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()).map(str::to_string)
                .or_else(|| base.map(|b| if field == "dailyReset" { b.daily_reset.clone() } else { b.weekly_reset.clone() }))
                .unwrap_or_else(|| fallback.to_string())
        };
        let daily = pick("dailyReset", "00:00");
        let weekly = pick("weeklyReset", "周一 00:00");
        (parse_reset_clock(&daily, "00:00"), parse_reset_clock(&weekly, &daily))
    }

    /// 和前端 `resolveVaultGameId` 同规则
    fn resolve(&self, platform: &str) -> String {
        let raw = platform.trim();
        if raw.is_empty() {
            return "other".into();
        }
        let p = raw.to_lowercase();
        if let Some(g) = self.list.iter().find(|g| p == g.id || p.contains(&g.id) || p.contains(&g.label.to_lowercase())) {
            return g.id.clone();
        }
        if raw.contains('剑') { return "jx3".into(); }
        if raw.contains("光遇") { return "sky".into(); }
        if raw.contains("逆水寒") { return "nsh".into(); }
        if p.contains("galgame") || p.contains("视觉小说") { return "galgame".into(); }
        if p.contains("steam") { return "steam".into(); }
        raw.to_string()
    }

    /// 和前端 `gameHasRoster` + `isGameEnabled` 一致：只有挂日常板的网游角色才计数。
    fn tracks(&self, id: &str) -> bool {
        let meta = self.meta(id);
        if meta.map(|m| m.kind == "reader").unwrap_or(false) {
            return false;
        }
        let key = format!("{} {}", id, meta.map(|m| m.label.as_str()).unwrap_or("")).to_lowercase();
        if key.contains("steam") || key.contains("galgame") || key.contains("视觉小说") {
            return false;
        }
        // 没有独立页面的游戏不受开关控制
        let toggleable = meta.map(|m| m.nav_label.is_some()).unwrap_or(false);
        !toggleable || self.enabled.as_ref().map(|e| e.iter().any(|x| x == id)).unwrap_or(true)
    }
}

struct Roster {
    /// 角色 id → 游戏 id（金库里所有角色）
    game_of: HashMap<String, String>,
    /// 挂在日常板上的角色
    tracked: HashSet<String>,
}

fn load_roster(conn: &Connection, games: &Games) -> rusqlite::Result<Roster> {
    let mut stmt = conn.prepare("SELECT id, platform, account_name, server, notes, payload FROM vault_items")?;
    let rows = stmt.query_map([], |r| {
        Ok(crate::vault::hydrate_vault_item(r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, None))
    })?;
    let mut roster = Roster { game_of: HashMap::new(), tracked: HashSet::new() };
    for item in rows {
        let item = item?;
        let game = games.resolve(&item.platform);
        let tracks = games.tracks(&game);
        for ch in item.characters {
            if tracks && ch.track_daily.unwrap_or(true) {
                roster.tracked.insert(ch.id.clone());
            }
            roster.game_of.insert(ch.id, game.clone());
        }
    }
    Ok(roster)
}

impl Roster {
    /// 和前端 `gameIdOf` 一致：找不到角色时取 id 前缀
    fn game(&self, character_id: &str) -> String {
        self.game_of.get(character_id).cloned()
            .unwrap_or_else(|| character_id.split('_').next().unwrap_or(character_id).to_string())
    }
}

// ---- 换日 ----

struct TaskRow {
    id: String,
    character_id: String,
    category: String,
    done: bool,
}

fn load_cycle_tasks(conn: &Connection) -> rusqlite::Result<Vec<TaskRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, character_id, category, done FROM tasks WHERE category IN ('daily', 'weekly')",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(TaskRow { id: r.get(0)?, character_id: r.get(1)?, category: r.get(2)?, done: r.get::<_, i64>(3)? != 0 })
    })?;
    rows.collect()
}

fn task_stamp(t: &TaskRow, now: NaiveDateTime, games: &Games, roster: &Roster) -> String {
    let (daily, weekly) = games.clocks(&roster.game(&t.character_id));
    if t.category == "weekly" {
        period_stamp(last_weekly_boundary(now, weekly))
    } else {
        period_stamp(last_daily_boundary(now, daily))
    }
}

/// 过了复位点的已勾任务清掉勾选。返回换日的"游戏日课/周常"标签。
/// 第一次见到的任务只记周期，不清勾选（和旧前端逻辑一致）。
fn reset_due_tasks(conn: &Connection, now: NaiveDateTime, settings: &Settings, games: &Games, roster: &Roster) -> rusqlite::Result<Vec<String>> {
    let prev: HashMap<String, String> = settings
        .json(TASK_CYCLE_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let mut next = serde_json::Map::new();
    let mut touched: Vec<String> = Vec::new();
    let tx = conn.unchecked_transaction()?;
    for t in load_cycle_tasks(&tx)? {
        let stamp = task_stamp(&t, now, games, roster);
        let changed = prev.get(&t.id).map(|p| p != &stamp).unwrap_or(false);
        if t.done && changed {
            tx.execute("UPDATE tasks SET done = 0 WHERE id = ?1", [&t.id])?;
            let label = format!(
                "{}{}",
                games.label(&roster.game(&t.character_id)),
                if t.category == "weekly" { "周常" } else { "日课" }
            );
            if !touched.contains(&label) {
                touched.push(label);
            }
        }
        next.insert(t.id, Value::String(stamp));
    }
    save(&tx, TASK_CYCLE_KEY, &Value::Object(next).to_string())?;
    tx.commit()?;
    Ok(touched)
}

/// 改了复位钟以后，把所有任务的周期改记成新钟下的当前周期，避免误清勾选。
pub(crate) fn rebase_cycles(conn: &Connection, now: NaiveDateTime) -> rusqlite::Result<()> {
    let settings = Settings::load(conn)?;
    let games = Games::load(&settings);
    let roster = load_roster(conn, &games)?;
    let mut next = serde_json::Map::new();
    for t in load_cycle_tasks(conn)? {
        next.insert(t.id.clone(), Value::String(task_stamp(&t, now, &games, &roster)));
    }
    save(conn, TASK_CYCLE_KEY, &Value::Object(next).to_string())
}

fn remaining(conn: &Connection, roster: &Roster, category: &str) -> rusqlite::Result<i64> {
    let mut stmt = conn.prepare("SELECT character_id FROM tasks WHERE done = 0 AND category = ?1")?;
    let rows = stmt.query_map([category], |r| r.get::<_, String>(0))?;
    let mut n = 0;
    for ch in rows {
        if roster.tracked.contains(&ch?) {
            n += 1;
        }
    }
    Ok(n)
}

// ---- 提醒 ----

pub(crate) struct Notice {
    pub title: String,
    pub body: String,
}

pub(crate) struct TickOutcome {
    /// 本次换日的"剑网3日课"之类标签
    pub reset: Vec<String>,
    /// 挂日常板角色的未完成日课数
    pub daily_left: i64,
    pub weekly_left: i64,
    pub notices: Vec<Notice>,
}

fn daily_reminder(conn: &Connection, now: NaiveDateTime, settings: &Settings, left: i64) -> rusqlite::Result<Option<Notice>> {
    if !settings.flag(NOTIFY_DAILY_KEY) {
        return Ok(None);
    }
    let clock = parse_reset_clock(settings.get(NOTIFY_DAILY_TIME_KEY).unwrap_or(DEFAULT_DAILY_TIME), DEFAULT_DAILY_TIME);
    let day = now.date().format("%Y-%m-%d").to_string();
    // 过了提醒时间且今天还没提醒过；跨过午夜就不再补昨天的
    if now < at(now.date(), clock) || settings.get(LAST_DAILY_NOTIFY_KEY) == Some(day.as_str()) {
        return Ok(None);
    }
    save(conn, LAST_DAILY_NOTIFY_KEY, &day)?;
    if left == 0 {
        return Ok(None);
    }
    Ok(Some(Notice {
        title: "Waypoint · 日常还没做完".into(),
        body: format!("挂在日常板上的角色还有 {left} 项日课没勾。"),
    }))
}

/// 各游戏周常复位后提醒一次。应用关着时错过的复位，超过 12 小时就不补发。
fn weekly_reminders(conn: &Connection, now: NaiveDateTime, settings: &Settings, games: &Games, roster: &Roster) -> rusqlite::Result<Vec<Notice>> {
    let mut last: HashMap<String, String> = settings
        .json(LAST_WEEKLY_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let first_run = last.is_empty();
    let tracked_games: HashSet<String> = roster.tracked.iter().map(|ch| roster.game(ch)).collect();
    let mut out = Vec::new();
    let mut changed = false;
    for game in &tracked_games {
        let (_, weekly) = games.clocks(game);
        let boundary = last_weekly_boundary(now, weekly);
        let stamp = period_stamp(boundary);
        if last.get(game) == Some(&stamp) {
            continue;
        }
        let fresh = now - boundary < Duration::hours(12);
        if !first_run && fresh && settings.flag(NOTIFY_WEEKLY_KEY) {
            out.push(Notice {
                title: format!("Waypoint · {}周常已刷新", games.label(game)),
                body: format!("{} 新的一周开始了，周常和团本 CD 已重置。", weekly_label(weekly)),
            });
        }
        last.insert(game.clone(), stamp);
        changed = true;
    }
    if changed {
        save(conn, LAST_WEEKLY_KEY, &serde_json::to_string(&last).unwrap_or_default())?;
    }
    Ok(out)
}

fn weekly_label(c: ResetClock) -> String {
    let wd = ["日", "一", "二", "三", "四", "五", "六"][(c.weekday.unwrap_or(1) % 7) as usize];
    format!("周{} {:02}:{:02}", wd, c.hour, c.minute)
}

/// cache.db 里前端拉过的光遇档期
fn sky_calendar(cache: Option<&Connection>) -> Option<Value> {
    let body: String = cache?
        .query_row("SELECT body FROM fetch_cache WHERE key = ?1", [SKY_CALENDAR_CACHE_KEY], |r| r.get(0))
        .ok()?;
    serde_json::from_str(&body).ok()
}

/// 国服在档活动明天或今天结束时提醒一次。
fn event_end_reminders(conn: &Connection, feed: Option<&Value>, now: NaiveDateTime, settings: &Settings) -> rusqlite::Result<Vec<Notice>> {
    if !settings.flag(NOTIFY_EVENT_END_KEY) {
        return Ok(Vec::new());
    }
    let Some(feed) = feed else { return Ok(Vec::new()) };
    let mut sent: Vec<String> = settings.json(EVENT_END_SENT_KEY).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let today = now.date();
    let mut items: Vec<&Value> = Vec::new();
    if let Some(season) = feed.get("season").filter(|v| !v.is_null()) {
        items.push(season);
    }
    for key in ["events", "cn_now"] {
        items.extend(feed.get(key).and_then(|v| v.as_array()).into_iter().flatten());
    }
    let mut out = Vec::new();
    for it in items {
        let text = |k: &str| it.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        if text("status") != "live" || text("region") == "intl" {
            continue;
        }
        let Ok(end) = NaiveDate::parse_from_str(text("end").get(..10).unwrap_or(""), "%Y-%m-%d") else { continue };
        let days = (end - today).num_days();
        if !(0..=1).contains(&days) {
            continue;
        }
        let mark = format!("{}@{}", text("id"), text("end"));
        if sent.contains(&mark) {
            continue;
        }
        let names: Vec<String> = it.get("names").and_then(|v| v.as_array()).into_iter().flatten()
            .filter_map(|v| v.as_str().map(str::to_string)).collect();
        let what = if names.is_empty() { text("title") } else { format!("{}（{}）", text("title"), names.join("、")) };
        out.push(Notice {
            title: format!("Waypoint · 光遇{}结束", if days == 0 { "今天" } else { "明天" }),
            body: format!("{what} {} 结束，要换的东西别忘了。", text("end")),
        });
        sent.push(mark);
    }
    if !out.is_empty() {
        // 只留最近的标记，免得无限增长
        let keep = sent.len().saturating_sub(50);
        save(conn, EVENT_END_SENT_KEY, &serde_json::to_string(&sent[keep..]).unwrap_or_default())?;
    }
    Ok(out)
}

/// 关注的先祖进国服档期时提醒：点名预告一次，开始当天一次。关注列表本身就是开关。
fn watch_reminders(conn: &Connection, feed: Option<&Value>, settings: &Settings) -> rusqlite::Result<Vec<Notice>> {
    let watched: Vec<String> = settings.json(SKY_WATCH_KEY).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let Some(feed) = feed.filter(|_| !watched.is_empty()) else { return Ok(Vec::new()) };
    let mut sent: Vec<String> = settings.json(SKY_WATCH_SENT_KEY).and_then(|v| serde_json::from_value(v).ok()).unwrap_or_default();
    let mut out = Vec::new();
    for key in ["cn_now", "cn_next"] {
        for it in feed.get(key).and_then(|v| v.as_array()).into_iter().flatten() {
            let text = |k: &str| it.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = text("status");
            if status != "live" && status != "upcoming" {
                continue;
            }
            let hits: Vec<&str> = it.get("names").and_then(|v| v.as_array()).into_iter().flatten()
                .filter_map(|v| v.as_str())
                .filter(|n| watched.iter().any(|w| w == n))
                .collect();
            let mark = format!("{}@{}", text("id"), status);
            if hits.is_empty() || sent.contains(&mark) {
                continue;
            }
            let who = hits.join("、");
            out.push(if status == "live" {
                Notice {
                    title: format!("Waypoint · {who}已回国服"),
                    body: format!("你关注的{who}正在国服复刻，{} 至 {}。", text("start"), text("end")),
                }
            } else {
                Notice {
                    title: format!("Waypoint · {who}要来国服了"),
                    body: format!("你关注的{who}已点名国服复刻，{} 开始，{} 结束。", text("start"), text("end")),
                }
            });
            sent.push(mark);
        }
    }
    if !out.is_empty() {
        let keep = sent.len().saturating_sub(50);
        save(conn, SKY_WATCH_SENT_KEY, &serde_json::to_string(&sent[keep..]).unwrap_or_default())?;
    }
    Ok(out)
}

pub(crate) fn tick(conn: &Connection, cache: Option<&Connection>, now: NaiveDateTime) -> rusqlite::Result<TickOutcome> {
    let settings = Settings::load(conn)?;
    let games = Games::load(&settings);
    let roster = load_roster(conn, &games)?;
    let reset = reset_due_tasks(conn, now, &settings, &games, &roster)?;
    let daily_left = remaining(conn, &roster, "daily")?;
    let weekly_left = remaining(conn, &roster, "weekly")?;
    let mut notices = Vec::new();
    notices.extend(daily_reminder(conn, now, &settings, daily_left)?);
    notices.extend(weekly_reminders(conn, now, &settings, &games, &roster)?);
    if games.tracks("sky") {
        let feed = sky_calendar(cache);
        notices.extend(event_end_reminders(conn, feed.as_ref(), now, &settings)?);
        notices.extend(watch_reminders(conn, feed.as_ref(), &settings)?);
    }
    Ok(TickOutcome { reset, daily_left, weekly_left, notices })
}

pub(crate) fn now_local() -> NaiveDateTime {
    Local::now().naive_local()
}

// ---- 运行时接线 ----

/// 托盘菜单里的"今日剩余"一行，每次 tick 更新文字。
pub struct TrayToday(pub tauri::menu::MenuItem<tauri::Wry>);

pub(crate) fn tray_text(daily_left: i64, weekly_left: i64) -> String {
    if daily_left == 0 && weekly_left == 0 {
        "今日日课已清".into()
    } else {
        format!("今日日课剩 {daily_left} 项 · 周常 {weekly_left} 项")
    }
}

pub(crate) fn run_once(app: &tauri::AppHandle) {
    use tauri::{Emitter, Manager};
    use tauri_plugin_notification::NotificationExt;

    let outcome = {
        let state = app.state::<crate::AppState>();
        let Ok(conn) = state.db.lock() else { return };
        let cache = app.try_state::<crate::cache::CacheDb>();
        let cache_conn = cache.as_ref().and_then(|c| c.0.lock().ok());
        tick(&conn, cache_conn.as_deref(), now_local())
    };
    let outcome = match outcome {
        Ok(o) => o,
        Err(e) => {
            eprintln!("cycle tick failed: {e}");
            return;
        }
    };
    if !outcome.reset.is_empty() {
        let _ = app.emit("cycle-reset", &outcome.reset);
    }
    for n in &outcome.notices {
        let _ = app.notification().builder().title(&n.title).body(&n.body).show();
    }
    if let Some(item) = app.try_state::<TrayToday>() {
        let _ = item.0.set_text(tray_text(outcome.daily_left, outcome.weekly_left));
    }
}

/// 档期不是今天拉的就在后台重拉，窗口一直没打开时活动结束和关注先祖的提醒也照样准。
/// 只在追踪光遇时拉；失败后隔一小时再试。拉到后推给前端，免得前端内存里还是旧的。
fn refresh_sky_calendar(app: &tauri::AppHandle, last_try: &mut Option<NaiveDateTime>) {
    use tauri::{Emitter, Manager};

    let now = now_local();
    if last_try.is_some_and(|t| now - t < Duration::hours(1)) {
        return;
    }
    let stale = {
        let state = app.state::<crate::AppState>();
        let Ok(conn) = state.db.lock() else { return };
        let tracks = Settings::load(&conn).map(|s| Games::load(&s).tracks("sky")).unwrap_or(false);
        let Some(cache) = app.try_state::<crate::cache::CacheDb>() else { return };
        let Ok(cache_conn) = cache.0.lock() else { return };
        let today = now.date().format("%Y-%m-%d").to_string();
        let fresh = sky_calendar(Some(&cache_conn))
            .is_some_and(|f| f.get("as_of").and_then(|v| v.as_str()) == Some(today.as_str()));
        tracks && !fresh
    };
    if !stale {
        return;
    }
    *last_try = Some(now);
    let feed = match tauri::async_runtime::block_on(crate::sky::fetch_sky_calendar()) {
        Ok(feed) => feed,
        Err(e) => {
            eprintln!("sky calendar refresh failed: {e}");
            return;
        }
    };
    let has_rows = feed.season.is_some()
        || [&feed.events, &feed.cn_now, &feed.cn_next, &feed.intl_now, &feed.intl_next].iter().any(|l| !l.is_empty());
    if !has_rows {
        return;
    }
    let Ok(body) = serde_json::to_string(&feed) else { return };
    let fetched_at = Local::now().timestamp_millis();
    {
        let Some(cache) = app.try_state::<crate::cache::CacheDb>() else { return };
        let Ok(cache_conn) = cache.0.lock() else { return };
        if let Err(e) = crate::cache::put(&cache_conn, SKY_CALENDAR_CACHE_KEY, &body, fetched_at) {
            eprintln!("sky calendar cache write failed: {e}");
            return;
        }
    }
    let _ = app.emit(
        "cache-updated",
        serde_json::json!({
            "key": SKY_CALENDAR_CACHE_KEY,
            "label": "光遇档期",
            "body": body,
            "fetched_at": fetched_at,
            "warning": feed.warnings.join("；"),
        }),
    );
}

/// 每分钟跑一次。用"已过时间且今天没做过"判断，电脑睡眠错过整点也会补上。
pub(crate) fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut sky_last_try = None;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            refresh_sky_calendar(&app, &mut sky_last_try);
            run_once(&app);
        }
    });
}

/// 勾选/取消日课周常时记进 daily_checks，给统计页用。日期按该游戏的复位钟算：
/// 剑三 07:00 换日，凌晨两点勾的算前一天。
pub(crate) fn record_task_check(conn: &Connection, task_id: &str, done: bool, now: NaiveDateTime) -> rusqlite::Result<()> {
    let row: Option<(String, String)> = conn
        .query_row("SELECT character_id, category FROM tasks WHERE id = ?1", [task_id], |r| Ok((r.get(0)?, r.get(1)?)))
        .ok();
    let Some((character_id, category)) = row else { return Ok(()) };
    if category != "daily" && category != "weekly" {
        return Ok(());
    }
    let settings = Settings::load(conn)?;
    let games = Games::load(&settings);
    let roster = load_roster(conn, &games)?;
    let (daily, _) = games.clocks(&roster.game(&character_id));
    let day = last_daily_boundary(now, daily).date().format("%Y-%m-%d").to_string();
    let scope = format!("task.{category}");
    let op = if done {
        crate::progress::ProgressOp::Check { scope, day, item_id: task_id.to_string() }
    } else {
        crate::progress::ProgressOp::Uncheck { scope, day, item_id: task_id.to_string() }
    };
    crate::progress::apply_ops(conn, &[op])
}

/// 前端勾选任务后立即刷新托盘计数，不等下一分钟。
#[tauri::command]
pub fn refresh_cycle_now(app: tauri::AppHandle) {
    run_once(&app);
}

/// 设置页改了复位钟后调用，见 `rebase_cycles`。
#[tauri::command]
pub fn rebase_task_cycles(state: tauri::State<crate::AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    rebase_cycles(&conn, now_local()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::init_database(&conn).unwrap();
        // 一个剑三账号（07:00 换日）挂两个角色，第二个不挂日常板
        let payload = r#"{"characters":[{"id":"c1","name":"甲","assets":[]},{"id":"c2","name":"乙","assets":[],"track_daily":false}]}"#;
        conn.execute(
            "INSERT INTO vault_items (id, platform, account_name, payload) VALUES ('v1', '剑网3', 'acc', ?1)",
            [payload],
        ).unwrap();
        for (id, ch, cat) in [("t1", "c1", "daily"), ("t2", "c1", "weekly"), ("t3", "c2", "daily")] {
            conn.execute(
                "INSERT INTO tasks (id, character_id, title, category, reset_rule, source_type, done) VALUES (?1, ?2, 'x', ?3, ?3, 'manual', 1)",
                [id, ch, cat],
            ).unwrap();
        }
        conn
    }

    fn done(conn: &Connection, id: &str) -> bool {
        conn.query_row("SELECT done FROM tasks WHERE id = ?1", [id], |r| r.get::<_, i64>(0)).unwrap() != 0
    }

    #[test]
    fn clock_parsing_matches_frontend() {
        assert_eq!(parse_reset_clock("周一 07:00", "00:00"), ResetClock { hour: 7, minute: 0, weekday: Some(1) });
        assert_eq!(parse_reset_clock("7:30", "00:00"), ResetClock { hour: 7, minute: 30, weekday: None });
        assert_eq!(parse_reset_clock("", "周日 00:00"), ResetClock { hour: 0, minute: 0, weekday: Some(0) });
        let mon7 = parse_reset_clock("周一 07:00", "00:00");
        // 2026-10-26 是周一
        assert_eq!(last_weekly_boundary(dt("2026-10-26 06:59"), mon7), dt("2026-10-19 07:00"));
        assert_eq!(last_weekly_boundary(dt("2026-10-26 07:00"), mon7), dt("2026-10-26 07:00"));
        assert_eq!(last_daily_boundary(dt("2026-10-29 06:00"), mon7), dt("2026-10-28 07:00"));
    }

    #[test]
    fn tick_resets_past_boundary_and_counts_tracked_only() {
        let conn = db();
        // 第一次只记周期
        let first = tick(&conn, None, dt("2026-10-29 06:00")).unwrap();
        assert!(first.reset.is_empty());
        assert!(done(&conn, "t1"));
        // 过了 07:00：日课清掉，周常（周一复位）不动
        let out = tick(&conn, None, dt("2026-10-29 07:01")).unwrap();
        assert_eq!(out.reset, vec!["剑网3日课".to_string()]);
        assert!(!done(&conn, "t1") && !done(&conn, "t3") && done(&conn, "t2"));
        // 只算挂日常板的 c1
        assert_eq!(out.daily_left, 1);
    }

    #[test]
    fn task_check_lands_on_game_day() {
        let conn = db();
        // 剑三 07:00 换日：凌晨两点勾的算前一天
        record_task_check(&conn, "t1", true, dt("2026-10-29 02:00")).unwrap();
        let day: String = conn.query_row("SELECT day FROM daily_checks WHERE scope = 'task.daily'", [], |r| r.get(0)).unwrap();
        assert_eq!(day, "2026-10-28");
        record_task_check(&conn, "t1", false, dt("2026-10-29 02:05")).unwrap();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM daily_checks", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn daily_reminder_fires_once_after_time_and_honors_toggle() {
        let conn = db();
        tick(&conn, None, dt("2026-10-29 06:00")).unwrap();
        tick(&conn, None, dt("2026-10-29 07:01")).unwrap();
        let has_daily = |o: &TickOutcome| o.notices.iter().any(|n| n.title.contains("日常"));
        assert!(!has_daily(&tick(&conn, None, dt("2026-10-29 19:59")).unwrap()));
        // 20:00 那分钟电脑在睡眠，20:07 才醒也要提醒
        assert!(has_daily(&tick(&conn, None, dt("2026-10-29 20:07")).unwrap()));
        assert!(!has_daily(&tick(&conn, None, dt("2026-10-29 20:08")).unwrap()));
        save(&conn, NOTIFY_DAILY_KEY, "false").unwrap();
        assert!(!has_daily(&tick(&conn, None, dt("2026-10-30 21:00")).unwrap()));
    }

    #[test]
    fn event_end_reminder_once_per_event() {
        let conn = db();
        let cache = crate::cache::open(std::path::Path::new(":memory:")).unwrap();
        let feed = r#"{"season":null,"events":[{"id":"e1","title":"恶作剧之日","region":"cn","status":"live","end":"2026-10-30","names":[]}],"cn_now":[]}"#;
        cache.execute("INSERT INTO fetch_cache (key, body, fetched_at) VALUES ('sky.calendar', ?1, ?2)",
            rusqlite::params![feed, chrono::Local::now().timestamp_millis()]).unwrap();
        let ends = |o: &TickOutcome| o.notices.iter().filter(|n| n.title.contains("结束")).count();
        assert_eq!(ends(&tick(&conn, Some(&cache), dt("2026-10-28 10:00")).unwrap()), 0);
        assert_eq!(ends(&tick(&conn, Some(&cache), dt("2026-10-29 10:00")).unwrap()), 1);
        assert_eq!(ends(&tick(&conn, Some(&cache), dt("2026-10-30 10:00")).unwrap()), 0);
    }

    #[test]
    fn watched_spirit_reminds_on_announce_and_start() {
        let conn = db();
        let cache = crate::cache::open(std::path::Path::new(":memory:")).unwrap();
        let put = |feed: &str| {
            cache.execute("INSERT OR REPLACE INTO fetch_cache (key, body, fetched_at) VALUES ('sky.calendar', ?1, 0)", [feed]).unwrap();
        };
        let watch = |o: &TickOutcome| o.notices.iter().filter(|n| n.title.contains("国服")).count();
        let upcoming = r#"{"cn_now":[],"cn_next":[{"id":"cn-210","status":"upcoming","names":["迟疑渔翁"],"start":"2026-11-12","end":"2026-11-16"},{"id":"cn-inf","status":"inferred","names":[]}]}"#;
        put(upcoming);
        assert_eq!(watch(&tick(&conn, Some(&cache), dt("2026-11-05 10:00")).unwrap()), 0);
        save(&conn, SKY_WATCH_KEY, r#"["迟疑渔翁"]"#).unwrap();
        assert_eq!(watch(&tick(&conn, Some(&cache), dt("2026-11-05 10:01")).unwrap()), 1);
        assert_eq!(watch(&tick(&conn, Some(&cache), dt("2026-11-05 10:02")).unwrap()), 0);
        put(r#"{"cn_now":[{"id":"cn-210","status":"live","names":["迟疑渔翁"],"start":"2026-11-12","end":"2026-11-16"}],"cn_next":[]}"#);
        let live = tick(&conn, Some(&cache), dt("2026-11-12 10:00")).unwrap();
        assert_eq!(watch(&live), 1);
        assert!(live.notices.iter().any(|n| n.title.contains("已回国服")));
        assert_eq!(watch(&tick(&conn, Some(&cache), dt("2026-11-12 10:01")).unwrap()), 0);
    }
}
