use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3DailyCalendar {
    pub date: String,
    pub war: String,
    pub mine: String,
    pub school: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rescue: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leader: Vec<String>,
    /// 今日秘境日常（三个副本）
    #[serde(default)]
    pub card: Vec<String>,
    /// 今日福缘宠物
    #[serde(default)]
    pub lucky: Vec<String>,
    /// 外观抽取，只有部分日子有
    #[serde(default)]
    pub draw: Vec<String>,
    /// 本周公共任务（武林通鉴）
    #[serde(default)]
    pub weekly_conn: Vec<String>,
    /// 本周团队秘境
    #[serde(default)]
    pub weekly_raid: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3CelebEvent {
    pub map: String,
    pub stage: String,
    pub site: String,
    pub desc: String,
    pub time: String,
}

fn jx3_json_text(v: Option<&serde_json::Value>) -> Option<String> {
    v.and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn jx3_strip_war_prefix(raw: &str) -> String {
    raw.strip_prefix("大战！")
        .or_else(|| raw.strip_prefix("大战!"))
        .unwrap_or(raw)
        .trim()
        .to_string()
}

fn jx3_json_ok<'a>(json: &'a serde_json::Value, label: &str) -> Result<&'a serde_json::Value, String> {
    let code = json
        .get("code")
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0);
    if code != 200 {
        let msg = json.get("msg").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("JX3API {} code {}: {}", label, code, msg));
    }
    json.get("data")
        .ok_or_else(|| format!("JX3API {} missing data", label))
}

fn jx3_string_list(v: Option<&serde_json::Value>) -> Vec<String> {
    match v {
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|x| x.as_str().map(str::trim).filter(|s| !s.is_empty()).map(|s| s.to_string()))
            .collect(),
        Some(serde_json::Value::String(s)) => s
            .split(['、', ',', '/', '，'])
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect(),
        _ => Vec::new(),
    }
}

fn parse_jx3_daily_calendar(json: &serde_json::Value) -> Result<Jx3DailyCalendar, String> {
    let data = jx3_json_ok(json, "calendar")?;
    let date = jx3_json_text(data.get("date")).ok_or_else(|| "JX3API calendar missing date".to_string())?;
    let war_raw = jx3_json_text(data.get("war")).ok_or_else(|| "JX3API calendar missing war".to_string())?;
    let orecar = jx3_json_text(data.get("orecar")).ok_or_else(|| "JX3API calendar missing orecar".to_string())?;
    let school = jx3_json_text(data.get("school")).ok_or_else(|| "JX3API calendar missing school".to_string())?;
    let war = jx3_strip_war_prefix(&war_raw);
    if war.is_empty() {
        return Err("JX3API calendar empty war".into());
    }
    Ok(Jx3DailyCalendar {
        date,
        war,
        mine: orecar,
        school,
        battle: jx3_json_text(data.get("battle")),
        rescue: jx3_json_text(data.get("rescue")),
        leader: jx3_string_list(data.get("leader")),
        card: jx3_string_list(data.get("card")),
        lucky: jx3_string_list(data.get("lucky")),
        draw: jx3_string_list(data.get("draw")),
        weekly_conn: jx3_string_list(data.pointer("/weekly/conn")),
        weekly_raid: jx3_string_list(data.pointer("/weekly/raid")),
    })
}

fn is_jx3_zhue(stage: &str, desc: &str) -> bool {
    stage.contains("诛恶") || desc.contains("诛恶")
}

fn parse_jx3_celebs(json: &serde_json::Value) -> Result<Vec<Jx3CelebEvent>, String> {
    let data = jx3_json_ok(json, "celebs")?;
    let items = data
        .as_array()
        .ok_or_else(|| "JX3API celebs data is not a list".to_string())?;
    if items.is_empty() {
        return Err("JX3API celebs empty".into());
    }
    let mut out = Vec::new();
    for item in items {
        let map = jx3_json_text(item.get("map")).unwrap_or_default();
        let stage = jx3_json_text(item.get("stage")).unwrap_or_default();
        let desc = jx3_json_text(item.get("desc")).unwrap_or_default();
        if map.is_empty() || stage.is_empty() || !is_jx3_zhue(&stage, &desc) {
            continue;
        }
        out.push(Jx3CelebEvent {
            map,
            stage,
            site: jx3_json_text(item.get("site")).unwrap_or_default(),
            desc,
            time: jx3_json_text(item.get("time")).unwrap_or_default(),
        });
    }
    Ok(out)
}

// 剑网3活动日历。权威路径 /active/calendar；num=0 当天，1 明天。矿车字段是 orecar。缺字段即失败，不回填旧名。
#[tauri::command]
pub async fn fetch_jx3_daily(num: Option<i32>) -> Result<Jx3DailyCalendar, String> {
    let num = num.unwrap_or(0);
    if num < 0 {
        return Err("JX3API calendar num cannot be negative".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .get("https://www.jx3api.com/active/calendar")
        .query(&[("num", num)])
        .send()
        .await
        .map_err(|e| format!("JX3API network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("JX3API HTTP {}", res.status().as_u16()));
    }
    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("JX3API JSON parse error: {}", e))?;
    parse_jx3_daily_calendar(&json)
}

#[tauri::command]
pub async fn fetch_jx3_celebs(name: String) -> Result<Vec<Jx3CelebEvent>, String> {
    let name = name.trim();
    if name != "云从社" && name != "披风会" && name != "楚天社" {
        return Err("JX3API celebs only accepts 云从社, 披风会 or 楚天社".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get("https://www.jx3api.com/active/celebs")
        .query(&[("name", name)])
        .send()
        .await
        .map_err(|e| format!("JX3API network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("JX3API HTTP {}", res.status().as_u16()));
    }
    let json: serde_json::Value = res
        .json()
        .await
        .map_err(|e| format!("JX3API JSON parse error: {}", e))?;
    parse_jx3_celebs(&json)
}

#[tauri::command]
pub async fn fetch_jx3_box_fb_map() -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;

    let urls = [
        "https://fastly.jsdelivr.net/gh/JX3BOX/jx3box-data@master/data/fb/fb_map.json",
        "https://cdn.jsdelivr.net/gh/JX3BOX/jx3box-data@master/data/fb/fb_map.json",
        "https://raw.githubusercontent.com/JX3BOX/jx3box-data/master/data/fb/fb_map.json",
    ];

    let mut last_err = String::new();
    for url in urls {
        match client.get(url).send().await {
            Ok(res) => {
                if res.status().is_success() {
                    match res.json::<serde_json::Value>().await {
                        Ok(json) => return Ok(json),
                        Err(e) => last_err = format!("JSON parse error: {}", e),
                    }
                } else {
                    last_err = format!("HTTP {}", res.status().as_u16());
                }
            }
            Err(e) => last_err = format!("Network error: {}", e),
        }
    }
    Err(format!("JX3BOX fb_map all mirrors failed: {}", last_err))
}

#[tauri::command]
pub async fn fetch_jx3_box_item_search(keyword: String) -> Result<serde_json::Value, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err("JX3BOX item search keyword empty".into());
    }
    if keyword.chars().count() > 40 {
        return Err("JX3BOX item search keyword too long".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get("https://node.jx3box.com/item/search")
        .query(&[("keyword", keyword), ("per", "50")])
        .send()
        .await
        .map_err(|e| format!("JX3BOX item search network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("JX3BOX item search HTTP {}", res.status().as_u16()));
    }
    res.json()
        .await
        .map_err(|e| format!("JX3BOX item search JSON parse error: {}", e))
}

fn jx3_box_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_jx3_box_serendipities(page: u32, per: u32) -> Result<serde_json::Value, String> {
    let page = page.max(1);
    let per = per.clamp(1, 50);
    let client = jx3_box_client(12)?;
    let res = client
        .get("https://node.jx3box.com/serendipities")
        .query(&[("page", page.to_string()), ("per", per.to_string())])
        .send()
        .await
        .map_err(|e| format!("JX3BOX serendipities network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("JX3BOX serendipities HTTP {}", res.status().as_u16()));
    }
    res.json()
        .await
        .map_err(|e| format!("JX3BOX serendipities JSON parse error: {}", e))
}

#[tauri::command]
pub async fn fetch_jx3_box_achievement_search(
    keyword: String,
    page: u32,
) -> Result<serde_json::Value, String> {
    let keyword = keyword.trim();
    if keyword.is_empty() {
        return Err("JX3BOX achievement search keyword empty".into());
    }
    if keyword.chars().count() > 40 {
        return Err("JX3BOX achievement search keyword too long".into());
    }
    let page = page.max(1);
    let client = jx3_box_client(12)?;
    let res = client
        .get("https://node.jx3box.com/achievement/search")
        .query(&[("keyword", keyword), ("page", &page.to_string())])
        .send()
        .await
        .map_err(|e| format!("JX3BOX achievement search network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "JX3BOX achievement search HTTP {}",
            res.status().as_u16()
        ));
    }
    res.json()
        .await
        .map_err(|e| format!("JX3BOX achievement search JSON parse error: {}", e))
}

#[tauri::command]
pub async fn fetch_jx3_box_reputations(page: u32) -> Result<serde_json::Value, String> {
    let page = page.max(1);
    let client = jx3_box_client(20)?;
    let res = client
        .get("https://node.jx3box.com/reputations")
        .query(&[
            ("client", "std"),
            ("page", &page.to_string()),
            ("per", "50"),
        ])
        .send()
        .await
        .map_err(|e| format!("JX3BOX reputations network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "JX3BOX reputations HTTP {}",
            res.status().as_u16()
        ));
    }
    res.json()
        .await
        .map_err(|e| format!("JX3BOX reputations JSON parse error: {}", e))
}

/// 物品 id（`5_87714` 这种「表_编号」）批量换名字，奇遇奖励用。
#[tauri::command]
pub async fn fetch_jx3_box_items_by_ids(ids: Vec<String>) -> Result<serde_json::Value, String> {
    let clean: Vec<&str> = ids
        .iter()
        .map(|s| s.trim())
        .filter(|s| {
            let mut parts = s.splitn(2, '_');
            let ok = |p: Option<&str>| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
            ok(parts.next()) && ok(parts.next())
        })
        .take(50)
        .collect();
    if clean.is_empty() {
        return Err("JX3BOX item ids empty".into());
    }
    let client = jx3_box_client(15)?;
    let res = client
        .get(format!("https://node.jx3box.com/item_merged/id/{}", clean.join(",")))
        .query(&[("per", "50")])
        .send()
        .await
        .map_err(|e| format!("JX3BOX item ids network error: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("JX3BOX item ids HTTP {}", res.status().as_u16()));
    }
    res.json()
        .await
        .map_err(|e| format!("JX3BOX item ids JSON parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jx3_daily_calendar_parses_current_payload() {
        let json = serde_json::json!({
            "code": 200,
            "msg": "success",
            "data": {
                "date": "2026-09-15",
                "week": "二",
                "war": "大战！英雄冰川宫宝库",
                "battle": "浮香丘",
                "orecar": "跨服·河西瀚漠",
                "rescue": "少林·乱世",
                "school": "长歌·笃学",
                "leader": ["白非人", "蚀毒巨尸"],
                "card": ["冷龙峰", "英雄鹿桥驿"],
                "lucky": ["顺顺"],
                "weekly": { "conn": ["五鼠作难白龙口"], "raid": ["洛阳之战", "雷域大泽"] }
            }
        });
        let cal = parse_jx3_daily_calendar(&json).unwrap();
        assert_eq!(cal.date, "2026-09-15");
        assert_eq!(cal.war, "英雄冰川宫宝库");
        assert_eq!(cal.mine, "跨服·河西瀚漠");
        assert_eq!(cal.school, "长歌·笃学");
        assert_eq!(cal.battle.as_deref(), Some("浮香丘"));
        assert_eq!(cal.rescue.as_deref(), Some("少林·乱世"));
        assert_eq!(cal.leader, vec!["白非人".to_string(), "蚀毒巨尸".to_string()]);
        assert_eq!(cal.card, vec!["冷龙峰".to_string(), "英雄鹿桥驿".to_string()]);
        assert_eq!(cal.lucky, vec!["顺顺".to_string()]);
        assert_eq!(cal.weekly_raid, vec!["洛阳之战".to_string(), "雷域大泽".to_string()]);
        assert_eq!(cal.weekly_conn, vec!["五鼠作难白龙口".to_string()]);
    }
    #[test]
    fn jx3_daily_calendar_parses_friday_leader() {
        let json = serde_json::json!({
            "code": 200,
            "data": {
                "date": "2026-09-18",
                "week": "五",
                "war": "大战！英雄獭山岛",
                "orecar": "跨服·河西瀚漠",
                "school": "长歌·笃学",
                "leader": ["尤珈罗摩", "炎晖"]
            }
        });
        let cal = parse_jx3_daily_calendar(&json).unwrap();
        assert_eq!(cal.date, "2026-09-18");
        assert_eq!(cal.leader, vec!["尤珈罗摩".to_string(), "炎晖".to_string()]);
    }
    #[test]
    fn jx3_celebs_parses_yuncang_and_rejects_empty() {
        let json = serde_json::json!({
            "code": 200,
            "msg": "success",
            "data": [
                {"map": "银霜口", "stage": "摸金倒斗", "site": "秦氏祖墓", "desc": "公共任务", "time": "09:01"},
                {"map": "银霜口", "stage": "诛恶事件(限时刷新)", "site": "隐者客栈东北", "desc": "诛恶事件。", "time": "09:40"}
            ]
        });
        let list = parse_jx3_celebs(&json).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].stage, "诛恶事件(限时刷新)");
        assert_eq!(list[0].time, "09:40");
        let ordinary = serde_json::json!({
            "code": 200,
            "data": [{"map": "河西瀚漠", "stage": "伏击吐蕃", "site": "那云寨西北", "desc": "协助轶漠军", "time": "11:26"}]
        });
        assert!(parse_jx3_celebs(&ordinary).unwrap().is_empty());
        let empty = serde_json::json!({ "code": 200, "data": [] });
        assert!(parse_jx3_celebs(&empty).is_err());
    }
    #[test]
    fn jx3_daily_calendar_rejects_dead_path_and_partial() {
        let dead = serde_json::json!({
            "code": 404,
            "msg": "请求的路径 [App.Data.Active_Calendar] 不存在",
            "data": {}
        });
        assert!(parse_jx3_daily_calendar(&dead).is_err());
        let partial = serde_json::json!({
            "code": 200,
            "data": { "date": "2026-09-15", "war": "大战！英雄幽月轮" }
        });
        assert!(parse_jx3_daily_calendar(&partial).is_err());
    }
}
