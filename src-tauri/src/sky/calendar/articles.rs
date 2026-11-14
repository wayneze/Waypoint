// 光遇国服长文：微信公众号与网易大神广场。
use super::super::SkyCalendarItem;
use super::super::wiki::*;
use super::super::weibo::*;

use super::news::*;
use super::merge::*;

/// 公众号历史列表要微信登录，不能当发现入口。身份只用来认出官网/微博里已经发出的文章。
pub(crate) const WEIXIN_OFFICIAL_BIZ: &str = "MzU3MTE3MzY2NA==";
pub(crate) const DASHEN_GY_SQUARE_ID: &str = "5cb546a0d5456870b97d9424";

pub(crate) fn weixin_http_client(secs: u64) -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                "zh-CN,zh;q=0.9".parse().unwrap(),
            );
            h
        })
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .ok()
}

pub(crate) fn dashen_http_client(secs: u64) -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(
                reqwest::header::HeaderName::from_static("gl-clienttype"),
                "60".parse().unwrap(),
            );
            h.insert(
                reqwest::header::HeaderName::from_static("gl-version"),
                "1".parse().unwrap(),
            );
            h.insert(reqwest::header::ORIGIN, "https://h5.ds.163.com".parse().unwrap());
            h.insert(reqwest::header::REFERER, "https://h5.ds.163.com/".parse().unwrap());
            h
        })
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .ok()
}

pub(crate) fn html_attr_after<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
    let rest = html.split(marker).nth(1)?;
    let rest = rest.trim_start_matches(['=', ' ', '"', '\'']);
    let end = rest.find(['"', '\'', '<', '\n']).unwrap_or(rest.len());
    let v = rest[..end].trim();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

pub(crate) fn html_js_string(html: &str, key: &str) -> Option<String> {
    let marker = format!("{} = ", key);
    let rest = html.split(&marker).nth(1)?;
    let rest = rest.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = rest.get(1..)?;
    let end = body.find(quote)?;
    Some(html_unescape(&body[..end]))
}

pub(crate) fn weixin_js_content_head(html: &str) -> &str {
    let Some(start_tag) = html.find("id=\"js_content\"") else {
        return "";
    };
    let rest = &html[start_tag..];
    let Some(gt) = rest.find('>') else {
        return "";
    };
    let body = &rest[gt + 1..];
    if let Some(cut) = body.find("往期精彩回顾") {
        return &body[..cut];
    }
    if let Some(end) = body.find("</div>") {
        return &body[..end];
    }
    body
}

pub(crate) fn weixin_img_url(raw: &str) -> Option<String> {
    let u = html_unescape(raw.trim()).replace("http://", "https://");
    if !u.starts_with("https://") {
        return None;
    }
    if !u.contains("mmbiz.qpic.cn") && !u.contains("mmbiz.qlogo.cn") {
        return None;
    }
    let low = u.to_ascii_lowercase();
    if low.contains("wx_fmt=gif") || low.contains("mmbiz_gif") || low.contains(".gif") {
        return None;
    }
    Some(u)
}

pub(crate) fn weixin_img_ratio(tag: &str) -> Option<f32> {
    let rest = tag.split("data-ratio=\"").nth(1)?;
    let end = rest.find('"')?;
    rest[..end].parse().ok()
}

pub(crate) fn weixin_head_images(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("<img") {
        let tag_end = rest[i..].find('>').unwrap_or(rest.len() - i);
        let tag = &rest[i..i + tag_end + 1];
        rest = &rest[i + tag_end + 1..];
        let Some(src) = tag.split("data-src=\"").nth(1) else {
            continue;
        };
        let end = src.find('"').unwrap_or(0);
        let Some(u) = weixin_img_url(&src[..end]) else {
            continue;
        };
        if weixin_img_ratio(tag).unwrap_or(1.0) < 0.1 {
            continue;
        }
        if !out.iter().any(|s| same_media_file(s, &u)) {
            out.push(u);
        }
        if out.len() >= 8 {
            break;
        }
    }
    out
}

pub(crate) fn normalize_weixin_url(raw: &str) -> Option<String> {
    let u = html_unescape(raw).replace('\\', "");
    let start = u.find("mp.weixin.qq.com/")?;
    let tail = &u[start + "mp.weixin.qq.com/".len()..];
    let end = tail
        .find(|c: char| {
            matches!(c, '"' | '\'' | '<' | '>' | ' ' | ')' | ']' | '#' | '\\')
                || c == '\n'
                || c == '\r'
        })
        .unwrap_or(tail.len());
    let path = tail[..end].trim_end_matches(['.', ',', ';', '，', '。']);
    if let Some(slug) = path.strip_prefix("s/") {
        let slug_end = slug
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
            .unwrap_or(slug.len());
        let slug = &slug[..slug_end];
        if slug.len() < 8 {
            return None;
        }
        return Some(format!("https://mp.weixin.qq.com/s/{}", slug));
    }
    if !path.starts_with("s?") {
        return None;
    }
    let query = &path[2..];
    let biz = query_param(query, "__biz");
    let mid = query_param(query, "mid");
    let idx = query_param(query, "idx").unwrap_or_else(|| "1".into());
    let sn = query_param(query, "sn");
    if biz.as_deref() != Some(WEIXIN_OFFICIAL_BIZ) || mid.is_none() || sn.is_none() {
        return None;
    }
    Some(format!(
        "https://mp.weixin.qq.com/s?__biz={}&mid={}&idx={}&sn={}",
        biz.unwrap_or_default(),
        mid.unwrap_or_default(),
        idx,
        sn.unwrap_or_default()
    ))
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for part in query.split(['&', ';']) {
        let part = part.trim().trim_start_matches("amp;");
        let (k, v) = part.split_once('=')?;
        if k == key && !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

pub(crate) fn collect_weixin_article_urls(blobs: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for blob in blobs {
        let mut from = 0;
        while let Some(rel) = blob[from..].find("mp.weixin.qq.com/") {
            let abs = from + rel;
            if let Some(url) = normalize_weixin_url(&blob[abs..]) {
                if !out.iter().any(|s| s == &url) {
                    out.push(url);
                }
            }
            from = abs + "mp.weixin.qq.com/".len();
        }
    }
    out
}

pub(crate) fn official_article_item(
    id: String,
    title: String,
    url: String,
    note: String,
    images: Vec<String>,
) -> Option<SkyCalendarItem> {
    let event_title = weibo_card_event_title(&format!("{} {}", title, note))?;
    let mut previews = Vec::new();
    for (i, img) in images.into_iter().enumerate() {
        previews.push(preview(
            "资讯",
            if i == 0 {
                "活动资讯".into()
            } else {
                format!("活动资讯 {}", i + 1)
            },
            String::new(),
            img,
            String::new(),
        ));
    }
    if previews.is_empty() {
        return None;
    }
    Some(SkyCalendarItem {
        id,
        kind: "event".into(),
        region: "cn".into(),
        title: event_title,
        names: vec![title],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: String::new(),
        note,
        url,
        cosmetics: String::new(),
        icon: String::new(),
        previews,
    })
}

pub(crate) fn parse_weixin_article(html: &str, url: &str) -> Option<SkyCalendarItem> {
    let title = html_attr_after(html, "property=\"og:title\" content")
        .map(html_unescape)
        .or_else(|| html_js_string(html, "msg_title"))
        .unwrap_or_default();
    if title.is_empty() {
        return None;
    }
    let head = weixin_js_content_head(html);
    if head.is_empty() {
        return None;
    }
    let note = cn_notice_clip(head);
    let images = weixin_head_images(head);
    let mut item = official_article_item(
        format!("cn-weixin-{}", event_title_id(&title)),
        title.clone(),
        url.to_string(),
        note,
        images,
    )?;
    let blob = format!("{} {}", title, strip_html_text(head));
    if let Some((start, end)) = parse_cn_notice_window(&blob, chrono::Local::now().date_naive()) {
        let status = event_window_status(Some(start), end, chrono::Local::now().date_naive());
        if status == "live" || status == "upcoming" {
            item.start = ymd(start);
            item.end = end.map(ymd).unwrap_or_default();
            item.status = status;
        }
    }
    Some(item)
}

pub(crate) fn event_title_id(title: &str) -> String {
    let t = canonical_event_title(title);
    if t == "我的世界联动" {
        "minecraft".into()
    } else if t == "秋宵节" {
        "qiuxiao".into()
    } else {
        calendar_event_family(&t)
    }
}

pub(crate) async fn fetch_weixin_articles(urls: &[String]) -> Vec<SkyCalendarItem> {
    let Some(client) = weixin_http_client(30) else {
        return Vec::new();
    };
    let urls: Vec<String> = urls.iter().take(8).cloned().collect();
    let mut items: Vec<SkyCalendarItem> = Vec::new();
    let mut i = 0;
    while i < urls.len() {
        let a = urls.get(i).cloned();
        let b = urls.get(i + 1).cloned();
        let (ra, rb) = tokio::join!(
            async {
                match a.as_deref() {
                    Some(url) => client.get(url).send().await.ok()?.text().await.ok(),
                    None => None,
                }
            },
            async {
                match b.as_deref() {
                    Some(url) => client.get(url).send().await.ok()?.text().await.ok(),
                    None => None,
                }
            },
        );
        for (html, url) in [(ra, a), (rb, b)] {
            let Some(url) = url else { continue };
            let Some(html) = html else { continue };
            let Some(it) = parse_weixin_article(&html, &url) else {
                continue;
            };
            if let Some(existing) = items.iter_mut().find(|e| {
                calendar_event_family(&e.title) == calendar_event_family(&it.title)
            }) {
                adopt_calendar_window(existing, &it);
                overlay_event_media(existing, &it);
            } else {
                items.push(it);
            }
        }
        i += 2;
    }
    items
}

pub(crate) fn dashen_feed_ids(facade: &serde_json::Value) -> Vec<String> {
    let mut ids = Vec::new();
    let lists = [
        facade
            .pointer("/result/gameDetailPageInfo/feedList")
            .and_then(|v| v.as_array()),
        facade
            .pointer("/result/gameInfo/gameDetailPageInfo/feedList")
            .and_then(|v| v.as_array()),
    ];
    for list in lists.into_iter().flatten() {
        for row in list {
            if let Some(id) = row.get("feedId").and_then(|v| v.as_str()) {
                if !id.is_empty() && !ids.iter().any(|s| s == id) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    ids
}

pub(crate) fn dashen_media_urls(body: &serde_json::Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(media) = body.get("media").and_then(|v| v.as_array()) {
        for m in media {
            let Some(u) = m.get("url").and_then(|v| v.as_str()) else {
                continue;
            };
            let low = u.to_ascii_lowercase();
            if !u.starts_with("http") || low.contains(".gif") || low.contains("wx_fmt=gif") {
                continue;
            }
            if !out.iter().any(|s| same_media_file(s, u)) {
                out.push(u.to_string());
            }
        }
    }
    out
}

pub(crate) fn dashen_event_from_feed(feed: &serde_json::Value) -> Option<SkyCalendarItem> {
    let parsed = if let Some(s) = feed.get("content").and_then(|v| v.as_str()) {
        serde_json::from_str::<serde_json::Value>(s).ok()
    } else {
        None
    };
    let content = feed
        .get("feedContent")
        .cloned()
        .or(parsed)
        .or_else(|| feed.get("content").cloned())?;
    let body = content.get("body").cloned().unwrap_or(content);
    let text = body.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let title = body.get("title").and_then(|v| v.as_str()).unwrap_or("");
    let blob = format!("{} {}", title, strip_html_text(text));
    let images = dashen_media_urls(&body);
    let id = feed.get("id").and_then(|v| v.as_str()).unwrap_or("feed");
    official_article_item(
        format!("cn-dashen-{}", id),
        if title.is_empty() { blob.clone() } else { title.to_string() },
        format!("https://ds.163.com/feed/{}", id),
        cn_notice_clip(&blob),
        images,
    )
}

pub(crate) async fn fetch_dashen_square_events() -> Vec<SkyCalendarItem> {
    let Some(client) = dashen_http_client(20) else {
        return Vec::new();
    };
    let Ok(facade_res) = client
        .get(format!(
            "https://inf.ds.163.com/v1/web/square/basic/facade?squareId={}",
            DASHEN_GY_SQUARE_ID
        ))
        .send()
        .await
    else {
        return Vec::new();
    };
    let Ok(facade) = facade_res.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let ids = dashen_feed_ids(&facade);
    if ids.is_empty() {
        return Vec::new();
    }
    let payload: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| serde_json::json!({ "id": id }))
        .collect();
    let Ok(detail_res) = client
        .post("https://god.gameyw.netease.com/v1/app/feed/detail")
        .json(&payload)
        .send()
        .await
    else {
        return Vec::new();
    };
    let Ok(detail) = detail_res.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let mut items: Vec<SkyCalendarItem> = Vec::new();
    if let Some(feeds) = detail.pointer("/result/feeds").and_then(|v| v.as_array()) {
        for feed in feeds {
            let Some(it) = dashen_event_from_feed(feed) else {
                continue;
            };
            if let Some(existing) = items.iter_mut().find(|e| {
                calendar_event_family(&e.title) == calendar_event_family(&it.title)
            }) {
                overlay_event_media(existing, &it);
            } else {
                items.push(it);
            }
        }
    }
    items
}

pub(crate) fn sixhorse_row_blob(row: &serde_json::Value) -> String {
    [
        row.get("title").and_then(|v| v.as_str()).unwrap_or(""),
        row.get("comment").and_then(|v| v.as_str()).unwrap_or(""),
        row.get("url").and_then(|v| v.as_str()).unwrap_or(""),
        row.get("redirectUrl").and_then(|v| v.as_str()).unwrap_or(""),
        row.get("link").and_then(|v| v.as_str()).unwrap_or(""),
    ]
    .join(" ")
}

pub(crate) fn weibo_mblog_blob(mblog: &serde_json::Value) -> String {
    let mut parts = vec![
        mblog.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        mblog
            .pointer("/page_info/page_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        mblog
            .pointer("/page_info/content1")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    ];
    if let Some(urls) = mblog.pointer("/url_struct").and_then(|v| v.as_array()) {
        for row in urls {
            if let Some(u) = row
                .get("long_url")
                .or_else(|| row.get("ori_url"))
                .or_else(|| row.get("short_url"))
                .and_then(|v| v.as_str())
            {
                parts.push(u.to_string());
            }
        }
    }
    parts.join(" ")
}

pub(crate) async fn discover_weixin_article_urls() -> Vec<String> {
    let Ok(client) = json_http_client(20) else {
        return Vec::new();
    };
    let mut blobs = Vec::new();
    for kind in ["26689", "26690", "26691"] {
        for start in [1u32, 21, 41] {
            for row in sixhorse_news_page(&client, kind, start, "").await {
                if keep_cn_notice_for_year(&row, chrono::Local::now().date_naive()) {
                    blobs.push(sixhorse_row_blob(&row));
                }
            }
        }
    }
    if let Some(wb) = weibo_http_client(20) {
        for mblog in weibo_uid_mblogs(&wb, "6355968578").await {
            blobs.push(weibo_mblog_blob(&mblog));
        }
    }
    collect_weixin_article_urls(&blobs)
}

pub(crate) async fn fetch_cn_official_articles(_today: chrono::NaiveDate) -> Vec<SkyCalendarItem> {
    let (urls, ds) = tokio::join!(discover_weixin_article_urls(), fetch_dashen_square_events());
    let wx = fetch_weixin_articles(&urls).await;
    let mut items = wx;
    merge_calendar_events(&mut items, ds);
    items
}
