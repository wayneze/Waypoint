// 微博访客 Cookie 与博文图文。日常任务和图鉴资讯共用。
use super::wiki::same_media_file;


pub(crate) fn strip_html_tags(text: &str) -> String {
    let mut in_tag = false;
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

pub(crate) fn weibo_large_image(url: &str) -> String {
    url.replace("/orj360/", "/mw2000/")
        .replace("/bmiddle/", "/mw2000/")
        .replace("/large/", "/mw2000/")
}

pub(crate) fn weibo_mblog_text(mblog: &serde_json::Value) -> String {
    let raw = mblog.get("text").and_then(|t| t.as_str()).unwrap_or("");
    strip_html_tags(&raw.replace("<br />", "\n").replace("<br/>", "\n"))
}

pub(crate) fn weibo_push_image(out: &mut Vec<String>, url: &str) {
    let u = url.trim();
    if !u.starts_with("http") {
        return;
    }
    let large = weibo_large_image(u);
    if out.iter().any(|s| same_media_file(s, &large)) {
        return;
    }
    out.push(large);
}

pub(crate) fn weibo_mblog_images(mblog: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(pics) = mblog.get("pics").and_then(|p| p.as_array()) {
        for pic in pics {
            let url = pic
                .pointer("/large/url")
                .and_then(|u| u.as_str())
                .or_else(|| pic.pointer("/largest/url").and_then(|u| u.as_str()))
                .or_else(|| pic.get("url").and_then(|u| u.as_str()))
                .unwrap_or("");
            weibo_push_image(&mut out, url);
        }
    }
    if let Some(infos) = mblog.get("pic_infos").and_then(|v| v.as_object()) {
        for pic in infos.values() {
            let url = pic
                .pointer("/largest/url")
                .and_then(|u| u.as_str())
                .or_else(|| pic.pointer("/large/url").and_then(|u| u.as_str()))
                .or_else(|| pic.pointer("/mw2000/url").and_then(|u| u.as_str()))
                .unwrap_or("");
            weibo_push_image(&mut out, url);
        }
    }
    if let Some(ids) = mblog.get("pic_ids").and_then(|v| v.as_array()) {
        for id in ids {
            let Some(pid) = id.as_str() else { continue };
            if pid.len() < 8 {
                continue;
            }
            weibo_push_image(
                &mut out,
                &format!("https://wx1.sinaimg.cn/large/{}.jpg", pid),
            );
        }
    }
    if let Some(u) = mblog.get("original_pic").and_then(|v| v.as_str()) {
        weibo_push_image(&mut out, u);
    }
    out
}

pub(crate) fn parse_weibo_created_at(raw: &str) -> Option<chrono::NaiveDate> {
    let t = raw.trim();
    if t.len() >= 10 {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(&t[..10], "%Y-%m-%d") {
            return Some(date);
        }
    }
    let rest = t.split_once(' ').map(|(_, r)| r).unwrap_or(t);
    let mut month = 0u32;
    let mut day = 0u32;
    for (name, n) in [
        ("Jan", 1), ("Feb", 2), ("Mar", 3), ("Apr", 4), ("May", 5), ("Jun", 6),
        ("Jul", 7), ("Aug", 8), ("Sep", 9), ("Oct", 10), ("Nov", 11), ("Dec", 12),
    ] {
        if let Some(after) = rest.strip_prefix(name) {
            month = n;
            let digits: String = after
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect();
            day = digits.parse().ok()?;
            break;
        }
    }
    if month == 0 || day == 0 {
        return None;
    }
    let year = t
        .rsplit(|c: char| !c.is_ascii_digit())
        .find(|s| s.len() == 4)
        .and_then(|s| s.parse().ok())
        .filter(|y| (2020..=2035).contains(y))?;
    chrono::NaiveDate::from_ymd_opt(year, month, day)
}

pub(crate) fn parse_weibo_status_ids(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut from = 0;
    while from < text.len() {
        let rest = &text[from..];
        let hit = rest
            .find("weibo.com/")
            .map(|i| (i, 10))
            .or_else(|| rest.find("m.weibo.cn/detail/").map(|i| (i, 18)))
            .or_else(|| rest.find("m.weibo.cn/status/").map(|i| (i, 18)))
            .or_else(|| rest.find("m.weibo.cn/statuses/show?id=").map(|i| (i, 28)));
        let Some((rel, skip)) = hit else {
            break;
        };
        let abs = from + rel + skip;
        let tail = &text[abs..];
        let tail = tail
            .strip_prefix("status/")
            .or_else(|| tail.strip_prefix("detail/"))
            .unwrap_or(tail);
        let n1 = tail.chars().take_while(|c| c.is_ascii_digit()).count();
        let a = &tail[..n1];
        let rem = &tail[n1..];
        let mid = if let Some(after) = rem.strip_prefix('/') {
            let b = after
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();
            if b.len() >= 10 {
                b
            } else {
                String::new()
            }
        } else if a.len() >= 16 {
            a.to_string()
        } else {
            String::new()
        };
        if !mid.is_empty() && !out.iter().any(|s| s == &mid) {
            out.push(mid);
        }
        from = abs + 1;
    }
    out
}

pub(crate) fn weibo_card_event_title(text: &str) -> Option<String> {
    let t = strip_html_tags(text);
    if t.contains("版本更新") || t.contains("停机") || t.contains("维护") {
        return None;
    }
    // 话题标签就是活动名。多个话题时不猜是哪一个。
    let mut tags = Vec::new();
    let chars: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '#' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < chars.len() && chars[j] != '#' && chars[j] != '\n' {
            j += 1;
        }
        if j < chars.len() && chars[j] == '#' && j > start {
            let tag: String = chars[start..j].iter().collect();
            let tag = tag.trim().trim_start_matches("光遇").trim().to_string();
            if tag.chars().count() >= 2
                && tag != "光遇"
                && !tag.ends_with("的一天")
                && !tags.iter().any(|s| s == &tag)
            {
                tags.push(tag);
            }
            i = j + 1;
            continue;
        }
        i += 1;
    }
    if tags.len() == 1 {
        return Some(crate::sky::wiki::canonical_event_title(&tags[0]));
    }
    // 一条官博常挂多个话题。只采用正文里点到的那个，点不到就不猜。
    if tags.len() > 1 {
        let body = strip_hashtags(&t);
        if let Some(name) = tag_named_in_body(&tags, &body) {
            return Some(crate::sky::wiki::canonical_event_title(&name));
        }
        return prose_event_title(&body);
    }
    prose_event_title(&t)
}

fn strip_hashtags(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '#' && chars[j] != '\n' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '#' {
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn tag_stem(tag: &str) -> String {
    for suf in ["纪念日", "飞行日", "嘉年华", "演唱会", "联动", "节"] {
        if let Some(stem) = tag.strip_suffix(suf) {
            if stem.chars().count() >= 2 {
                return stem.to_string();
            }
        }
    }
    tag.to_string()
}

fn tag_named_in_body(tags: &[String], body: &str) -> Option<String> {
    let hits: Vec<&String> = tags
        .iter()
        .filter(|tag| {
            let stem = tag_stem(tag);
            body.contains(tag.as_str()) || (stem.chars().count() >= 2 && body.contains(&stem))
        })
        .collect();
    if hits.is_empty() {
        return None;
    }
    let festivals: Vec<&&String> = hits
        .iter()
        .filter(|tag| {
            tag.ends_with('节')
                || tag.ends_with("纪念日")
                || tag.ends_with("飞行日")
                || tag.ends_with("嘉年华")
                || tag.ends_with("演唱会")
        })
        .collect();
    if festivals.len() == 1 {
        return Some((*festivals[0]).clone());
    }
    if hits.len() == 1 {
        return Some(hits[0].clone());
    }
    None
}

pub(crate) fn prose_event_title(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if t.contains("我的世界") || t.to_ascii_lowercase().contains("minecraft") {
        return Some("我的世界联动".into());
    }
    for marker in ["联动", "节", "飞行日", "纪念日", "嘉年华", "演唱会"] {
        if let Some(end) = t.rfind(marker) {
            let head: String = t[..end].chars().rev().take(8).collect::<String>().chars().rev().collect();
            let start = head
                .rfind(['》', '｜', '|', '：', ':', '，', '。', ' ', '《'])
                .map(|i| i + c_len_at(&head, i))
                .unwrap_or(0);
            let name = format!("{}{}", head[start..].trim(), marker);
            if name.chars().count() >= marker.chars().count() + 2
                && name.chars().count() <= 12
                && !name.ends_with("的一天")
            {
                return Some(crate::sky::wiki::canonical_event_title(&name));
            }
        }
    }
    None
}

fn c_len_at(s: &str, byte: usize) -> usize {
    s[byte..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

pub(crate) async fn weibo_m_visitor_cookie(client: &reqwest::Client, uid: &str) -> Option<String> {
    let now_ts = chrono::Utc::now().timestamp();
    let target_url = format!(
        "https://m.weibo.cn/u/{}?luicode=10000011&lfid=231583&launchid=10000360-page_H5",
        uid
    );
    let visitor_url = format!(
        "https://visitor.passport.weibo.cn/visitor/visitor?entry=sinawap&a=enter&url=https%3A%2F%2Fm.weibo.cn%2Fu%2F{}&domain=.weibo.cn&sudaref=&ua=php-sso_sdk_client-0.6.36&_rand={}",
        uid, now_ts
    );
    let v_text = client
        .get(&visitor_url)
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let request_id = v_text
        .split("var request_id = \"")
        .nth(1)
        .and_then(|s| s.split('"').next())?;
    let gen_body = format!(
        "cb=visitor_gray_callback&ver=20250916&request_id={}&tid=&from=weibo&webdriver=false&rid={}&return_url=https%3A%2F%2Fm.weibo.cn%2Fu%2F{}",
        request_id, now_ts, uid
    );
    let gen_text = client
        .post("https://visitor.passport.weibo.cn/visitor/genvisitor2")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Referer", &visitor_url)
        .body(gen_body)
        .send()
        .await
        .ok()?
        .text()
        .await
        .ok()?;
    let sub = gen_text
        .split("\"sub\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("");
    let subp = gen_text
        .split("\"subp\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("");
    if sub.is_empty() {
        return None;
    }
    let cookie = format!("SUB={}; SUBP={};", sub, subp);
    let _ = client
        .get(&target_url)
        .header("Cookie", &cookie)
        .header("Referer", "https://m.weibo.cn/")
        .send()
        .await;
    Some(cookie)
}

pub(crate) async fn weibo_uid_mblogs(client: &reqwest::Client, uid: &str) -> Vec<serde_json::Value> {
    let Some(cookie) = weibo_m_visitor_cookie(client, uid).await else {
        return Vec::new();
    };
    let target_url = format!(
        "https://m.weibo.cn/u/{}?luicode=10000011&lfid=231583&launchid=10000360-page_H5",
        uid
    );
    let api_url = format!(
        "https://m.weibo.cn/api/container/getIndex?type=uid&value={}&containerid=107603{}",
        uid, uid
    );
    let Ok(json) = client
        .get(&api_url)
        .header("Cookie", &cookie)
        .header("Referer", &target_url)
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
    else {
        return Vec::new();
    };
    let Ok(val) = json.json::<serde_json::Value>().await else {
        return Vec::new();
    };
    let mut cards = val
        .pointer("/data/cards")
        .and_then(|c| c.as_array())
        .map(|cards| {
            cards
                .iter()
                .filter_map(|c| c.get("mblog").cloned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut since = cards
        .last()
        .and_then(|m| m.get("id").and_then(|v| v.as_str()).or_else(|| m.get("mid").and_then(|v| v.as_str())))
        .unwrap_or("")
        .to_string();
    let mut page = 0;
    while page < 2 && !since.is_empty() {
        let more = format!("{}&since_id={}", api_url, since);
        let Ok(res) = client
            .get(&more)
            .header("Cookie", &cookie)
            .header("Referer", &target_url)
            .send()
            .await
        else {
            break;
        };
        let Ok(next) = res.json::<serde_json::Value>().await else {
            break;
        };
        let extra = next
            .pointer("/data/cards")
            .and_then(|c| c.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|c| c.get("mblog").cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if extra.is_empty() {
            break;
        }
        let next_since = extra
            .last()
            .and_then(|m| m.get("id").and_then(|v| v.as_str()).or_else(|| m.get("mid").and_then(|v| v.as_str())))
            .unwrap_or("")
            .to_string();
        if next_since == since {
            break;
        }
        since = next_since;
        cards.extend(extra);
        page += 1;
    }
    cards
}

pub(crate) fn weibo_http_client(secs: u64) -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1")
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .ok()
}
