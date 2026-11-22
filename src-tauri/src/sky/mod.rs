use serde::{Deserialize, Serialize};

mod wiki;
mod weibo;
mod fandom;
mod daily;
mod calendar;
mod seasons;
mod events;
mod event_images;
mod dashen;
mod boost;
mod collection_media;
pub use collection_media::resolve_sky_collection_media;

pub(crate) fn beijing_today() -> chrono::NaiveDate {
    chrono::Utc::now()
        .with_timezone(&chrono::FixedOffset::east_opt(8 * 3600).unwrap())
        .date_naive()
}

pub use calendar::{fetch_sky_calendar, fetch_sky_event_edition, inspect_main};
pub use daily::{fetch_sky_daily_data, fetch_sky_quest_calendar};

/// 光遇 wiki / Fandom 的图标和实穿图走本地盘缓存。返回 `Some(referer)` 表示是这类图床；
/// Fandom 图床不带来源页会 403，B 站 wiki 不需要。
pub fn wiki_image_referer(url: &str) -> Option<Option<&'static str>> {
    let parsed = reqwest::Url::parse(url).ok()?;
    if parsed.scheme() == "http" && parsed.host_str() == Some("dp.mz997.com")
        && parsed.port() == Some(92) && parsed.path().starts_with("/exported_icons/")
        && parsed.path().ends_with(".png") && parsed.query().is_none()
    {
        return Some(None);
    }
    if parsed.scheme() != "https" {
        return None;
    }
    match parsed.host_str()?.to_ascii_lowercase().as_str() {
        "static.wikia.nocookie.net" => Some(Some("https://sky-children-of-the-light.fandom.com/")),
        "wiki.biligame.com" | "patchwiki.biligame.com" => Some(None),
        _ => None,
    }
}

/// 微博 / 微信图在网页里直接请求会 403。`img://localhost/?u=` 由桌面端带正确 Referer 代拉。
pub async fn fetch_hotlink_image(url: &str) -> Result<(String, Vec<u8>), String> {
    if !hotlink_image_host(url) {
        return Err("not a hotlink host".into());
    }
    let referer = if url.contains("sinaimg.cn") {
        "https://weibo.com/"
    } else if url.contains("mmbiz.qpic.cn") || url.contains("mmbiz.qlogo.cn") || url.contains("qpic.cn") {
        "https://mp.weixin.qq.com/"
    } else {
        "https://wiki.biligame.com/sky/"
    };
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(25))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .header(reqwest::header::REFERER, referer)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("image {}", resp.status()));
    }
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .split(';')
        .next()
        .unwrap_or("image/jpeg")
        .trim()
        .to_string();
    if !mime.starts_with("image/") {
        return Err("not an image".into());
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.len() < 32 || bytes.len() > 12 * 1024 * 1024 {
        return Err("image size".into());
    }
    Ok((mime, bytes.to_vec()))
}

fn hotlink_image_host(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" && parsed.scheme() != "http" {
        return false;
    }
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    host.ends_with("sinaimg.cn")
        || host.ends_with("qpic.cn")
        || host.ends_with("qlogo.cn")
        || host == "nie.res.netease.com"
        || host.ends_with(".166.net")
}
pub use events::fetch_sky_events;
pub use event_images::fetch_sky_event_guide_images;
pub use seasons::fetch_sky_seasons;
pub use dashen::fetch_sky_role_wings;
pub use boost::fetch_sky_boost_feedback;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyDailyResult {
    pub success: bool,
    pub date_str: String,
    pub post_text: String,
    pub all_images: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyQuestCalendarItem {
    pub date: String,
    pub quest_id: String,
    pub quest_cn: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyQuestCalendar {
    pub items: Vec<SkyQuestCalendarItem>,
    pub image: Option<String>,
    #[serde(rename = "seasonImage")]
    pub season_image: Option<String>,
    #[serde(rename = "eventImage")]
    pub event_image: Option<String>,
    #[serde(rename = "shardImage")]
    pub shard_image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyCalendarPreview {
    pub slot: String,
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub wear: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyCalendarItem {
    pub id: String,
    pub kind: String,
    pub region: String,
    pub title: String,
    pub names: Vec<String>,
    pub season: String,
    pub start: String,
    pub end: String,
    pub status: String,
    pub note: String,
    pub url: String,
    #[serde(default)]
    pub cosmetics: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub previews: Vec<SkyCalendarPreview>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyEventEdition {
    pub title: String,
    pub region: String,
    pub start: String,
    pub end: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyCalendarFeed {
    pub as_of: String,
    pub season: Option<SkyCalendarItem>,
    pub events: Vec<SkyCalendarItem>,
    pub cn_now: Vec<SkyCalendarItem>,
    pub cn_next: Vec<SkyCalendarItem>,
    pub intl_now: Vec<SkyCalendarItem>,
    pub intl_next: Vec<SkyCalendarItem>,
    #[serde(default)]
    pub event_history: Vec<SkyEventEdition>,
    /// 档期拉到了、但某个子数据源失败（图片查询之类），给状态条显示
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkySeasonWishItem {
    pub id: String,
    pub name: String,
    pub spirit: String,
    pub kind: String,
    pub slot: String,
    pub cost_desc: String,
    pub candles: u32,
    pub ascended: u32,
    pub hearts: u32,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub wear: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkySeasonCatalogItem {
    pub id: String,
    pub title: String,
    pub slogan: String,
    pub realm: String,
    pub loc: String,
    pub start: String,
    pub end: String,
    pub status: String,
    pub url: String,
    pub icon: String,
    pub quests: String,
    /// 节日页上的整张兑换图。情报卡已有单件列表时不用它代替实穿。
    #[serde(default)]
    pub sheet: String,
    pub items: Vec<SkySeasonWishItem>,
    /// 先祖 → 买齐这位先祖整棵树的季蜡。只有带兑换图的季节有。
    #[serde(default)]
    pub tree_candles: std::collections::HashMap<String, u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkySeasonCatalog {
    pub as_of: String,
    pub seasons: Vec<SkySeasonCatalogItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkyEventCatalog {
    pub as_of: String,
    pub events: Vec<SkySeasonCatalogItem>,
}

pub(crate) const COSMETIC_KINDS: &[&str] = &[
    "大型家具",
    "小型家具",
    "发型",
    "发饰",
    "面饰",
    "面具",
    "斗篷",
    "裤子",
    "鞋子",
    "手持",
    "颈饰",
    "耳饰",
    "背饰",
];

#[cfg(test)]
mod tests;
