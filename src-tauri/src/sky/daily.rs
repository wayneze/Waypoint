// 微博每日任务实景图 + 国服 gyapi 今日任务。
use super::{SkyDailyResult, SkyQuestCalendar, SkyQuestCalendarItem};
use super::weibo::*;


// Tauri Commands: 获取微博 @今天游离翻车了吗 每日光遇权威图文实景数据
#[tauri::command]
pub async fn fetch_sky_daily_data() -> Result<SkyDailyResult, String> {
    let client = weibo_http_client(8).ok_or_else(|| "weibo client".to_string())?;
    let mblogs = weibo_uid_mblogs(&client, "7360748659").await;
    if mblogs.is_empty() {
        return Err("No cards in Weibo response".into());
    }
    let today = super::beijing_today();
    for mblog in mblogs {
        let posted = mblog.get("created_at")
            .and_then(|value| value.as_str())
            .and_then(parse_weibo_created_at);
        if posted != Some(today) {
            continue;
        }
        let raw_text = mblog.get("text").and_then(|t| t.as_str()).unwrap_or("");
        if raw_text.contains("每日任务") || raw_text.contains("季蜡") {
            let clean_text = weibo_mblog_text(&mblog);
            // 第几张图是什么由前端按正文"图N - …"认领；博主每天的图序不固定，这里不按位置猜
            let all_images = weibo_numbered_images(&mblog);
            return Ok(SkyDailyResult {
                success: true,
                date_str: today.format("%Y-%m-%d").to_string(),
                post_text: clean_text,
                all_images,
            });
        }
    }
    Err("No matching daily task post found for today".to_string())
}

pub(crate) fn gyapi_image_for_today(val: &serde_json::Value, today: &str) -> Option<String> {
    let date = val.get("date").and_then(|s| s.as_str()).unwrap_or("");
    if date != today {
        return None;
    }
    let image_prefix = format!("https://api.qmkjcm.cn/gyapi/images/{}/", today.replace('-', ""));
    ["tcurl", "image"].into_iter()
        .filter_map(|key| val.get(key).and_then(|value| value.as_str()))
        .find(|url| url.starts_with(&image_prefix))
        .map(str::to_string)
}

/// 国服每日任务条目：社区日历 JSON；季蜡配图走分类后的 jlwz，不依赖微博正文。
#[tauri::command]
pub async fn fetch_sky_quest_calendar() -> Result<SkyQuestCalendar, String> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let today = super::beijing_today().format("%Y-%m-%d").to_string();
    let fetch_json = |url| {
        let request = client.get(url);
        async move {
            let response = request.send().await?.error_for_status()?;
            response.json::<serde_json::Value>().await
        }
    };
    let (list, quests, candles, event, shard) = tokio::join!(
        fetch_json("https://api.qmkjcm.cn/api/gy_mr_rw/"),
        fetch_json("https://api.qmkjcm.cn/gyapi/rwt/mrrw/"),
        fetch_json("https://api.qmkjcm.cn/gyapi/rwt/jlwz/"),
        fetch_json("https://api.qmkjcm.cn/gyapi/rwt/db/"),
        fetch_json("https://api.qmkjcm.cn/gyapi/rwt/lsd/"),
    );
    let val = list.map_err(|e| e.to_string())?;
    if val.get("code").and_then(|c| c.as_i64()) != Some(200) {
        return Err(val
            .get("msg")
            .and_then(|m| m.as_str())
            .unwrap_or("quest calendar failed")
            .to_string());
    }

    let mut items = Vec::new();
    if let Some(list) = val.pointer("/data/task_list").and_then(|v| v.as_array()) {
        for v in list {
            let date = v
                .get("date")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            if date != today {
                continue;
            }
            let quest_cn = v
                .get("quest_cn")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if quest_cn.is_empty() {
                continue;
            }
            items.push(SkyQuestCalendarItem {
                date,
                quest_id: v
                    .get("quest_id")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                quest_cn,
            });
        }
    }
    if items.is_empty() {
        return Err("quest calendar has no items for today".to_string());
    }

    let image = |result: Result<serde_json::Value, reqwest::Error>| {
        result.ok().and_then(|img| gyapi_image_for_today(&img, &today))
    };
    Ok(SkyQuestCalendar {
        items,
        image: image(quests),
        season_image: image(candles),
        event_image: image(event),
        shard_image: image(shard),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_previous_image_even_when_the_endpoint_reports_today() {
        let day = "2026-11-22";
        let endpoint = serde_json::json!({
            "date": day,
            "tcurl": "https://api.qmkjcm.cn/gyapi/images/20261121/04.webp",
            "image": "https://api.qmkjcm.cn/gyapi/images/20261122/04.webp?v=1795276929"
        });
        assert_eq!(gyapi_image_for_today(&endpoint, day).as_deref(), endpoint["image"].as_str());
        let endpoint = serde_json::json!({ "date": day, "image": endpoint["tcurl"] });
        assert!(gyapi_image_for_today(&endpoint, day).is_none());
    }
}
