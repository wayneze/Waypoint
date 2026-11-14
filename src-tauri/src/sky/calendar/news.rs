// 光遇国服资讯：官网公告与微博。
use super::super::SkyCalendarItem;
use super::super::wiki::*;
use super::super::weibo::*;

use super::merge::*;

pub(crate) fn take_year(chars: &[char]) -> Option<(usize, i32)> {
    if chars.len() >= 5 && chars[0].is_ascii_digit() && chars[1].is_ascii_digit()
        && chars[2].is_ascii_digit() && chars[3].is_ascii_digit() && chars[4] == '年'
    {
        let y: String = chars[..4].iter().collect();
        return Some((5, y.parse().ok()?));
    }
    None
}

pub(crate) fn take_month_day(chars: &[char]) -> Option<(usize, u32, u32)> {
    let mut i = 0;
    let mut month = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        month.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || chars[i] != '月' || month.is_empty() {
        return None;
    }
    i += 1;
    let mut day = String::new();
    while i < chars.len() && chars[i].is_ascii_digit() {
        day.push(chars[i]);
        i += 1;
    }
    if day.is_empty() {
        return None;
    }
    if i < chars.len() && chars[i] == '日' {
        i += 1;
    }
    Some((i, month.parse().ok()?, day.parse().ok()?))
}

pub(crate) fn cn_notice_is_event(title: &str, comment: &str) -> bool {
    let title = title.trim();
    if title.is_empty() {
        return false;
    }
    let blob = format!("{} {}", title, comment);
    const REJECT: &[&str] = &[
        "打击",
        "非法",
        "辅助软件",
        "外挂",
        "封停",
        "封禁",
        "违规账号",
        "处罚公示",
        "奖励发放公示",
        "兑换指引",
        "同人创作分享",
        "创作激励",
        "创作活动",
        "维护",
        "停机",
        "补偿发放",
    ];
    if REJECT.iter().any(|k| blob.contains(k)) {
        return false;
    }
    // 正文模板里常年带「季」「节」，不能单凭评论把往年公告再收进来。
    const TITLE_KEEP: &[&str] = &[
        "节",
        "联动",
        "飞行日",
        "纪念日",
        "季",
        "演唱会",
        "嘉年华",
        "我的世界",
        "Minecraft",
        "活动开启",
        "活动",
        "正式上线",
        "更新内容公告",
    ];
    if !TITLE_KEEP.iter().any(|k| title.contains(k)) {
        return false;
    }
    const KEEP: &[&str] = &[
        "节",
        "联动",
        "飞行日",
        "纪念日",
        "季",
        "演唱会",
        "嘉年华",
        "活动开启",
        "活动",
        "正式上线",
        "即将开启",
        "更新内容公告",
        "我的世界",
        "Minecraft",
    ];
    KEEP.iter().any(|k| blob.contains(k))
}

pub(crate) fn note_is_noise(note: &str) -> bool {
    let t = note.trim();
    if t.is_empty() {
        return true;
    }
    if t == "国服专页" {
        return true;
    }
    let letters = t.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let cjk = t.chars().filter(|c| (0x4E00..=0x9FFF).contains(&(*c as u32))).count();
    cjk == 0 && letters >= 8
}

pub(crate) fn strip_html_text(raw: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in raw.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    html_unescape(&out)
        .replace('\u{a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn html_unescape(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

pub(crate) fn cn_notice_short_title(title: &str) -> String {
    let t = title.replace(['丨', '|'], "｜");
    if let Some(name) = prose_event_title(&t) {
        return name;
    }
    let core = t
        .split('｜')
        .map(|s| s.trim())
        .rfind(|s| !s.is_empty())
        .unwrap_or(title.trim());
    let core = core
        .trim_start_matches('《')
        .trim_start_matches("光·遇")
        .trim_start_matches("光•遇")
        .trim_start_matches('》')
        .trim_start_matches('：')
        .trim_start_matches(':')
        .trim();
    let core = core
        .trim_end_matches("更新内容公告")
        .trim_end_matches("内容公告")
        .trim_end_matches("活动公告")
        .trim();
    if core.chars().count() >= 2 {
        core.to_string()
    } else {
        title.trim().to_string()
    }
}

/// 正文没写年份时，用公告发布日定年。发布日之后 21 天内仍算今年；再早的月日不再推到明年。
pub(crate) fn notice_date_year(anchor: chrono::NaiveDate, month: u32, day: u32) -> Option<i32> {
    let year = date_year(anchor);
    let this_year = chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    if this_year + chrono::TimeDelta::days(21) >= anchor {
        return Some(year);
    }
    None
}

pub(crate) fn published_date_from_url(url: &str) -> Option<chrono::NaiveDate> {
    let bytes = url.as_bytes();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if bytes[i..i + 8].iter().all(|c| c.is_ascii_digit()) {
            let y: i32 = std::str::from_utf8(&bytes[i..i + 4]).ok()?.parse().ok()?;
            let m: u32 = std::str::from_utf8(&bytes[i + 4..i + 6]).ok()?.parse().ok()?;
            let d: u32 = std::str::from_utf8(&bytes[i + 6..i + 8]).ok()?.parse().ok()?;
            if (2020..=2035).contains(&y) {
                if let Some(date) = chrono::NaiveDate::from_ymd_opt(y, m, d) {
                    return Some(date);
                }
            }
        }
        i += 1;
    }
    None
}

pub(crate) fn parse_cn_open_date(text: &str, anchor: chrono::NaiveDate) -> Option<chrono::NaiveDate> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = compact.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let tail: String = chars[i..].iter().collect();
        let hit = tail.starts_with("将于")
            || tail.starts_with("定于")
            || tail.starts_with("预计")
            || tail.starts_with("开启时间");
        if !hit {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut seen_date = false;
        while j < chars.len() && j < i + 24 {
            if let Some((len, month, day)) = take_month_day(&chars[j..]) {
                seen_date = true;
                if let Some(year) = notice_date_year(anchor, month, day) {
                    if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, month, day) {
                        if date + chrono::TimeDelta::days(2) >= anchor {
                            return Some(date);
                        }
                    }
                }
                j += len;
                continue;
            }
            j += 1;
        }
        if seen_date {
            return None;
        }
        i += 1;
    }
    None
}

/// 完整年月日区间优先。没有区间时，用「将于X月X日开启」补国服开启日。
pub(crate) fn parse_cn_notice_window(
    text: &str,
    anchor: chrono::NaiveDate,
) -> Option<(chrono::NaiveDate, Option<chrono::NaiveDate>)> {
    if let Some((start, end)) = parse_cn_notice_span(text) {
        return Some((start, Some(end)));
    }
    if let Some((start, end)) = parse_cn_undated_span(text, anchor) {
        return Some((start, Some(end)));
    }
    parse_cn_open_date(text, anchor).map(|start| (start, None))
}

/// 官博常写「9月24日-10月14日」而不写年份。用今天给这段区间定年，不和另一句的月日配对。
pub(crate) fn parse_cn_undated_span(
    text: &str,
    anchor: chrono::NaiveDate,
) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let chars: Vec<char> = compact.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let Some((alen, am, ad)) = take_month_day(&chars[i..]) else {
            i += 1;
            continue;
        };
        let mut j = i + alen;
        let limit = (j + 8).min(chars.len());
        while j < limit {
            if take_year(&chars[j..]).is_some() {
                break;
            }
            if let Some((_, bm, bd)) = take_month_day(&chars[j..]) {
                let year = notice_date_year(anchor, am, ad)?;
                let end_year = if (bm, bd) < (am, ad) { year + 1 } else { year };
                if let (Some(a), Some(b)) = (
                    chrono::NaiveDate::from_ymd_opt(year, am, ad),
                    chrono::NaiveDate::from_ymd_opt(end_year, bm, bd),
                ) {
                    if (1..=60).contains(&(b - a).num_days()) {
                        return Some((a, b));
                    }
                }
                break;
            }
            j += 1;
        }
        i += alen;
    }
    None
}

pub(crate) fn parse_cn_notice_span(text: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let re_full = regex_lite_ymd_span(&compact);
    if re_full.is_some() {
        return re_full;
    }
    // 年份可能写在日期前面，也可能落款写在后面。只认同一段里的两个月日，避免和另一句配成区间。
    let chars: Vec<char> = compact.chars().collect();
    let mut years: Vec<(usize, i32)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some((len, y)) = take_year(&chars[i..]) {
            years.push((i, y));
            i += len;
            continue;
        }
        i += 1;
    }
    i = 0;
    while i < chars.len() {
        let Some((alen, am, ad)) = take_month_day(&chars[i..]) else {
            i += 1;
            continue;
        };
        let mut j = i + alen;
        let limit = (j + 16).min(chars.len());
        while j < limit {
            if let Some((blen, bm, bd)) = take_month_day(&chars[j..]) {
                let year = years
                    .iter()
                    .rev()
                    .find(|(yp, _)| *yp < i)
                    .map(|(_, y)| *y)
                    .or_else(|| years.iter().find(|(yp, _)| *yp > j + blen).map(|(_, y)| *y))
                    .unwrap_or(0);
                if year >= 2020 {
                    if let (Some(a), Some(b)) = (
                        chrono::NaiveDate::from_ymd_opt(year, am, ad),
                        chrono::NaiveDate::from_ymd_opt(year, bm, bd),
                    ) {
                        if (1..=60).contains(&(b - a).num_days()) {
                            return Some((a, b));
                        }
                    }
                }
                break;
            }
            j += 1;
        }
        i += alen;
    }
    None
}

pub(crate) fn regex_lite_ymd_span(s: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 10 <= bytes.len() {
        if bytes[i].is_ascii_digit()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && (bytes[i + 4] == b'-' || bytes[i + 4] == b'.' || bytes[i + 4] == b'/')
        {
            if let Some(a) = parse_ymd_prefix(&s[i..]) {
                let rest = &s[i + 8..];
                if let Some(rel) = rest.find(|c: char| c.is_ascii_digit()) {
                    if let Some(b) = parse_ymd_prefix(&rest[rel..]) {
                        if (1..=60).contains(&(b - a).num_days()) {
                            return Some((a, b));
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

pub(crate) fn cn_notice_clip(text: &str) -> String {
    let t = strip_html_text(text);
    let skip = ["由网易游戏代理", "凭借清新唯美", "荣膺", "亲爱的旅人们"];
    let parts: Vec<&str> = t
        .split(['。', '！', '!', '？', '?'])
        .map(|s| s.trim())
        .filter(|s| {
            !s.is_empty()
                && s.chars().count() >= 8
                && !skip.iter().any(|k| s.contains(k))
        })
        .collect();
    let joined = if parts.is_empty() { t } else { parts.join("。") };
    if joined.chars().count() <= 90 {
        joined
    } else {
        format!("{}…", joined.chars().take(90).collect::<String>())
    }
}

pub(crate) fn cn_notice_absolute_url(url: &str) -> String {
    let u = url.trim();
    if u.starts_with("http://") || u.starts_with("https://") {
        u.to_string()
    } else if u.starts_with("//") {
        format!("https:{}", u)
    } else if u.starts_with('/') {
        format!("https://sky.163.com{}", u)
    } else {
        format!("https://sky.163.com/{}", u.trim_start_matches('/'))
    }
}

pub(crate) fn cn_notice_images(item: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(u) = item.get("imageUrl").and_then(|v| v.as_str()) {
        if u.starts_with("http") {
            out.push(u.to_string());
        }
    }
    if let Some(raw) = item.get("imageUrls").and_then(|v| v.as_str()) {
        for part in raw.split([';', ',']) {
            let u = part.trim();
            if u.starts_with("http") && !out.iter().any(|s| s == u) {
                out.push(u.to_string());
            }
        }
    }
    out
}

pub(crate) fn millis_to_date(ms: i64) -> Option<chrono::NaiveDate> {
    chrono::DateTime::from_timestamp_millis(ms).map(|d| d.date_naive())
}

pub(crate) fn cn_notice_to_item(item: &serde_json::Value, today: chrono::NaiveDate) -> Option<SkyCalendarItem> {
    let raw_title = item.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
    if raw_title.is_empty() {
        return None;
    }
    let comment = item.get("comment").and_then(|v| v.as_str()).unwrap_or("");
    if !cn_notice_is_event(raw_title, comment) {
        return None;
    }
    let blob = format!("{} {}", raw_title, comment);
    let title = canonical_event_title(&cn_notice_short_title(raw_title));
    let url = item
        .get("url")
        .and_then(|v| v.as_str())
        .map(cn_notice_absolute_url)
        .unwrap_or_default();
    let anchor = published_date_from_url(&url).unwrap_or(today);
    let (start, end, status) = if let Some((start, end)) = parse_cn_notice_window(&blob, anchor) {
        let status = event_window_status(Some(start), end, today);
        (ymd(start), end.map(ymd).unwrap_or_default(), status)
    } else {
        ("".into(), "".into(), String::new())
    };
    let images = cn_notice_images(item);
    let icon = images.first().cloned().unwrap_or_default();
    let mut previews = Vec::new();
    for (i, img) in images.into_iter().skip(1).take(8).enumerate() {
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
    let code = item.get("code").and_then(|v| v.as_str()).unwrap_or("");
    Some(SkyCalendarItem {
        id: format!("cn-notice-{}", if code.is_empty() { title.clone() } else { code.into() }),
        kind: "event".into(),
        region: "cn".into(),
        title,
        names: vec![raw_title.to_string()],
        season: String::new(),
        start,
        end,
        status,
        note: cn_notice_clip(comment),
        url,
        cosmetics: String::new(),
        icon,
        previews,
    })
}

pub(crate) async fn sixhorse_news_page(
    client: &reqwest::Client,
    kind: &str,
    start: u32,
    keyword: &str,
) -> Vec<serde_json::Value> {
    let mut form = vec![
        ("start", start.to_string()),
        ("limit", "20".into()),
        ("contentkind", kind.to_string()),
    ];
    if !keyword.is_empty() {
        form.push(("keyword", keyword.to_string()));
    }
    let res = client
        .post("https://sixhorse.game.163.com/news/outer/newslist_search.do")
        .header(reqwest::header::REFERER, "https://sky.163.com/")
        .form(&form)
        .send()
        .await
        .ok();
    let Some(res) = res else {
        return Vec::new();
    };
    let Ok(val) = res.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    val.get("result")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub(crate) fn date_year(d: chrono::NaiveDate) -> i32 {
    ymd(d)[..4].parse().unwrap_or(0)
}

pub(crate) fn sixhorse_row_year(row: &serde_json::Value) -> Option<i32> {
    let url = row.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let bytes = url.as_bytes();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        if bytes[i..i + 8].iter().all(|c| c.is_ascii_digit()) {
            if let Ok(y) = std::str::from_utf8(&bytes[i..i + 4])
                .unwrap_or("")
                .parse::<i32>()
            {
                if (2020..=2035).contains(&y) {
                    return Some(y);
                }
            }
        }
        i += 1;
    }
    millis_to_date(row.get("showtime").and_then(|v| v.as_i64()).unwrap_or(0))
        .map(date_year)
        .filter(|y| (2020..=2035).contains(y))
}

pub(crate) fn keep_cn_notice_for_year(row: &serde_json::Value, today: chrono::NaiveDate) -> bool {
    match sixhorse_row_year(row) {
        Some(y) => y == date_year(today),
        None => true,
    }
}

pub(crate) fn json_http_client(secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .map_err(|e| e.to_string())
}

pub(crate) async fn fetch_cn_official_notices(today: chrono::NaiveDate) -> Vec<SkyCalendarItem> {
    let Ok(client) = json_http_client(20) else {
        return Vec::new();
    };
    let pages: Vec<(String, u32)> = ["26689", "26690", "26691"]
        .into_iter()
        .flat_map(|kind| [1u32, 21, 41].into_iter().map(move |start| (kind.to_string(), start)))
        .collect();
    let mut rows = Vec::new();
    for (kind, start) in pages {
        rows.extend(sixhorse_news_page(&client, &kind, start, "").await);
    }
    let mut items: Vec<SkyCalendarItem> = Vec::new();
    for row in rows {
        if !keep_cn_notice_for_year(&row, today) {
            continue;
        }
        if let Some(it) = cn_notice_to_item(&row, today) {
            if let Some(existing) = items.iter_mut().find(|e| {
                e.id == it.id || calendar_event_family(&e.title) == calendar_event_family(&it.title)
            }) {
                overlay_event_media(existing, &it);
            } else {
                items.push(it);
            }
        }
    }
    merge_calendar_events(&mut items, Vec::new());
    items
}

pub(crate) fn weibo_event_from_mblog(
    mblog: &serde_json::Value,
    today: chrono::NaiveDate,
) -> Option<SkyCalendarItem> {
    let raw = mblog.get("text").and_then(|t| t.as_str()).unwrap_or("");
    let title = weibo_card_event_title(raw)?;
    let posted = mblog
        .get("created_at")
        .and_then(|v| v.as_str())
        .and_then(parse_weibo_created_at);
    let anchor = posted.unwrap_or(today);
    let window = parse_cn_notice_window(raw, anchor);
    let (start, end, status) = if let Some((start, end)) = window {
        (
            ymd(start),
            end.map(ymd).unwrap_or_default(),
            event_window_status(Some(start), end, today),
        )
    } else {
        (String::new(), String::new(), String::new())
    };
    if !status.is_empty() && status != "live" && status != "upcoming" {
        return None;
    }
    let images: Vec<String> = weibo_mblog_images(mblog)
        .into_iter()
        .filter(|u| !u.to_ascii_lowercase().contains(".gif"))
        .collect();
    if images.is_empty() {
        return None;
    }
    let mut previews = Vec::new();
    for (i, img) in images.iter().enumerate() {
        previews.push(preview(
            "资讯",
            if i == 0 {
                "活动资讯".into()
            } else {
                format!("活动资讯 {}", i + 1)
            },
            String::new(),
            img.clone(),
            String::new(),
        ));
    }
    Some(SkyCalendarItem {
        id: format!("cn-weibo-{}", title),
        kind: "event".into(),
        region: "cn".into(),
        title,
        names: vec![],
        season: String::new(),
        start,
        end,
        status,
        note: cn_notice_clip(raw),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews,
    })
}

pub(crate) async fn weibo_status_mblog(
    client: &reqwest::Client,
    cookie: &str,
    mid: &str,
) -> Option<serde_json::Value> {
    let url = format!("https://m.weibo.cn/statuses/show?id={}", mid);
    let mut req = client
        .get(&url)
        .header("Referer", "https://m.weibo.cn/")
        .header("Accept", "application/json, text/plain, */*");
    if !cookie.is_empty() {
        req = req.header("Cookie", cookie);
    }
    let val = req.send().await.ok()?.json::<serde_json::Value>().await.ok()?;
    val.get("data")
        .cloned()
        .or_else(|| val.get("mblog").cloned())
        .filter(|v| v.is_object())
}

pub(crate) async fn weibo_mblogs_by_ids(mids: &[String]) -> Vec<serde_json::Value> {
    if mids.is_empty() {
        return Vec::new();
    }
    let Some(client) = weibo_http_client(20) else {
        return Vec::new();
    };
    let cookie = weibo_m_visitor_cookie(&client, "6355968578")
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut i = 0;
    while i < mids.len() {
        let a = mids.get(i);
        let b = mids.get(i + 1);
        let (ra, rb) = tokio::join!(
            async {
                match a {
                    Some(mid) => weibo_status_mblog(&client, &cookie, mid).await,
                    None => None,
                }
            },
            async {
                match b {
                    Some(mid) => weibo_status_mblog(&client, &cookie, mid).await,
                    None => None,
                }
            },
        );
        for mblog in [ra, rb].into_iter().flatten() {
            out.push(mblog);
        }
        i += 2;
    }
    out
}

pub(crate) fn collect_weibo_events(
    mblogs: Vec<serde_json::Value>,
    today: chrono::NaiveDate,
) -> Vec<SkyCalendarItem> {
    let mut items: Vec<SkyCalendarItem> = Vec::new();
    for mblog in mblogs {
        let Some(it) = weibo_event_from_mblog(&mblog, today) else {
            continue;
        };
        if let Some(existing) = items.iter_mut().find(|e| e.title == it.title) {
            overlay_event_media(existing, &it);
        } else {
            items.push(it);
        }
    }
    items
}

pub(crate) async fn fetch_cn_official_event_media(today: chrono::NaiveDate) -> Vec<SkyCalendarItem> {
    let Some(client) = weibo_http_client(20) else {
        return Vec::new();
    };
    let mblogs = weibo_uid_mblogs(&client, "6355968578").await;
    collect_weibo_events(mblogs, today)
}

