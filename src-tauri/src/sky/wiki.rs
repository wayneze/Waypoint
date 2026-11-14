// 光遇 wiki 编码、档期状态、装扮文件名与 bili wiki HTTP。
use super::{COSMETIC_KINDS, SkyCalendarItem, SkyCalendarPreview};


pub(crate) fn wiki_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

pub(crate) fn wiki_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn parse_ymd_prefix(s: &str) -> Option<chrono::NaiveDate> {
    let t = s.trim();
    if t == "未知" || t.is_empty() {
        return None;
    }
    let date_part = t.split([' ', 'T']).next().unwrap_or(t).trim();
    let mut parts = date_part.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    chrono::NaiveDate::from_ymd_opt(y, m, d)
}

pub(crate) fn ymd(d: chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn calendar_status(start: Option<chrono::NaiveDate>, end: Option<chrono::NaiveDate>, today: chrono::NaiveDate) -> String {
    match (start, end) {
        (Some(s), Some(e)) if s <= today && today <= e => "live".into(),
        (Some(s), None) if s <= today => "live".into(),
        (Some(s), _) if s > today => "upcoming".into(),
        _ => "ended".into(),
    }
}

pub(crate) fn event_window_status(
    start: Option<chrono::NaiveDate>,
    end: Option<chrono::NaiveDate>,
    today: chrono::NaiveDate,
) -> String {
    match (start, end) {
        (Some(s), Some(e)) if s <= today && today <= e => "live".into(),
        (Some(s), Some(_)) if s > today => "upcoming".into(),
        (Some(s), None) if s > today => "upcoming".into(),
        _ => "ended".into(),
    }
}

pub(crate) fn parse_template_fields(inner: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for part in inner.split('|') {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

pub(crate) fn wiki_file_url(file: &str) -> String {
    format!(
        "https://wiki.biligame.com/sky/Special:FilePath/{}",
        wiki_encode(file)
    )
}

pub(crate) fn wiki_redirect_file_url(file: &str) -> String {
    format!(
        "https://wiki.biligame.com/sky/Special:Redirect/file/{}",
        wiki_encode(file)
    )
}

pub(crate) fn spirit_icon_url(name: &str) -> String {
    if name.contains('《') {
        wiki_file_url(&format!("{}.png", name))
    } else {
        wiki_file_url(&format!("UI-{}.png", name))
    }
}

pub(crate) fn season_logo_url(season: &str) -> String {
    wiki_file_url(&format!("UI-{}logo.png", season))
}

pub(crate) fn cosmetic_file_kind(kind: &str) -> String {
    kind.replace("毕业", "").trim_end_matches(|c: char| c.is_ascii_digit()).to_string()
}

pub(crate) fn cosmetic_icon_url(kind: &str, owner: &str) -> String {
    wiki_file_url(&format!("UI-{}-{}-无框.png", kind, owner))
}

pub(crate) fn cosmetic_wear_url(kind: &str, owner: &str) -> String {
    wiki_file_url(&format!("{}{}展示图.png", owner, kind))
}

pub(crate) fn wiki_filename_from_url(url: &str) -> Option<String> {
    const MARKS: &[&str] = &["Special:FilePath/", "Special:Redirect/file/"];
    let mut enc = None;
    for mark in MARKS {
        if let Some(i) = url.find(mark) {
            enc = url[i + mark.len()..].split(['?', '#']).next();
            break;
        }
    }
    let enc = enc?;
    let name = wiki_decode(enc);
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

pub(crate) fn wear_filename_from_url(url: &str) -> Option<String> {
    let name = wiki_filename_from_url(url)?;
    if name.contains("展示图") {
        Some(name)
    } else {
        None
    }
}

pub(crate) async fn wiki_file_urls_chunk(
    client: &reqwest::Client,
    chunk: &[String],
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if chunk.is_empty() {
        return out;
    }
    let titles = chunk
        .iter()
        .map(|f| format!("文件:{}", f))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(res) = client
        .post("https://wiki.biligame.com/sky/api.php")
        .form(&[
            ("action", "query"),
            ("titles", titles.as_str()),
            ("prop", "imageinfo"),
            ("iiprop", "url"),
            ("format", "json"),
        ])
        .send()
        .await
    else {
        return out;
    };
    let Ok(val) = res.json::<serde_json::Value>().await else {
        return out;
    };
    let Some(pages) = val.pointer("/query/pages").and_then(|v| v.as_object()) else {
        return out;
    };
    for page in pages.values() {
        let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("");
        // wiki 明确说没有这个文件：记成空串，调用方据此不再拼链接；请求失败的文件不在表里，仍按原链接试
        if page.get("missing").is_some() {
            if let Some(name) = title.strip_prefix("文件:") {
                out.push((name.to_string(), String::new()));
            }
            continue;
        }
        let url = page
            .get("imageinfo")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|info| info.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if url.is_empty() {
            continue;
        }
        let name = title.strip_prefix("文件:").unwrap_or(title).to_string();
        out.push((name, url.to_string()));
    }
    out
}

pub(crate) async fn wiki_file_urls(
    client: &reqwest::Client,
    files: &[String],
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut uniq = files.to_vec();
    uniq.sort();
    uniq.dedup();
    uniq.retain(|s| !s.is_empty());
    let chunks: Vec<Vec<String>> = uniq.chunks(40).map(|c| c.to_vec()).collect();
    let mut i = 0;
    while i < chunks.len() {
        let a = chunks.get(i).cloned();
        let b = chunks.get(i + 1).cloned();
        let c = chunks.get(i + 2).cloned();
        let d = chunks.get(i + 3).cloned();
        let (ra, rb, rc, rd) = tokio::join!(
            async {
                match a {
                    Some(ch) => wiki_file_urls_chunk(client, &ch).await,
                    None => Vec::new(),
                }
            },
            async {
                match b {
                    Some(ch) => wiki_file_urls_chunk(client, &ch).await,
                    None => Vec::new(),
                }
            },
            async {
                match c {
                    Some(ch) => wiki_file_urls_chunk(client, &ch).await,
                    None => Vec::new(),
                }
            },
            async {
                match d {
                    Some(ch) => wiki_file_urls_chunk(client, &ch).await,
                    None => Vec::new(),
                }
            },
        );
        for (name, url) in ra.into_iter().chain(rb).chain(rc).chain(rd) {
            out.insert(name, url);
        }
        i += 4;
    }
    out
}

pub(crate) fn is_furniture_kind(kind: &str) -> bool {
    kind.contains("家具")
}

pub(crate) fn preview_stem(p: &SkyCalendarPreview) -> &str {
    p.name.split(" · ").next().unwrap_or(p.name.as_str()).trim()
}

pub(crate) fn parse_spirit_en_names(wt: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut last_cn = String::new();
    for line in wt.lines() {
        let line = line.trim().trim_start_matches('|').trim();
        if let Some(v) = line.strip_prefix("先祖中文名") {
            last_cn = v.trim_start_matches('=').trim().to_string();
        } else if let Some(v) = line.strip_prefix("先祖英文名") {
            let en = v.trim_start_matches('=').trim().to_string();
            if !last_cn.is_empty() && !en.is_empty() {
                out.insert(std::mem::take(&mut last_cn), en);
            } else {
                last_cn.clear();
            }
        }
    }
    out
}

pub(crate) fn fold_zh_event_name(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '節' => '节',
            '慶' => '庆',
            '週' => '周',
            '陽' => '阳',
            '運' => '运',
            '搗' => '捣',
            '愛' => '爱',
            '華' => '华',
            '樂' => '乐',
            _ => c,
        })
        .filter(|c| !c.is_whitespace() && *c != '·' && *c != '-')
        .collect()
}

pub(crate) fn canonical_event_title(title: &str) -> String {
    let folded = fold_zh_event_name(title);
    if folded.contains("秋宵") || folded.contains("月华") {
        return "秋宵节".into();
    }
    if folded.contains("我的世界") {
        return "我的世界联动".into();
    }
    let low = title.to_ascii_lowercase();
    if low.contains("moonlight") {
        return "秋宵节".into();
    }
    if low.contains("minecraft") {
        return "我的世界联动".into();
    }
    title.trim().to_string()
}

pub(crate) fn has_cjk(s: &str) -> bool {
    s.chars().any(|c| (0x4E00..=0x9FFF).contains(&(c as u32)))
}

pub(crate) fn zh_event_stem(s: &str) -> String {
    let folded = fold_zh_event_name(s);
    for suf in ["周年庆", "嘉年华", "节日", "庆典", "之日", "之节", "节", "日"] {
        if let Some(stem) = folded.strip_suffix(suf) {
            if stem.chars().count() >= 2 {
                return stem.to_string();
            }
        }
    }
    folded
}

pub(crate) fn calendar_event_family(title: &str) -> String {
    let mut folded = fold_zh_event_name(title)
        .replace("联动", "")
        .replace("活动", "");
    // 来源页会给返场加“第 N 期”，它属于档次说明，不是另一个活动家族。
    if let Some(index) = folded.rfind('第') {
        if let Some(edition) = folded[index + '第'.len_utf8()..].strip_suffix('期') {
            if !edition.is_empty() && edition.chars().all(|c| c.is_ascii_digit()
                || "一二三四五六七八九十百零〇两".contains(c))
            {
                folded.truncate(index);
            }
        }
    }
    let stem = zh_event_stem(&folded);
    let stem = if stem.contains("月华") || stem.contains("秋宵") {
        "秋宵".into()
    } else if stem.contains("我的世界") {
        "我的世界".into()
    } else {
        stem
    };
    if stem.chars().count() >= 2 {
        stem
    } else {
        folded
    }
}

pub(crate) fn calendar_cover_filename(url: &str) -> String {
    let base = url.split(['?', '#']).next().unwrap_or(url);
    if base.contains("mmbiz.qpic.cn") || base.contains("mmbiz.qlogo.cn") {
        let segs: Vec<&str> = base.rsplit('/').filter(|s| !s.is_empty()).collect();
        if segs.len() >= 2 {
            let last = segs[0];
            if last.chars().all(|c| c.is_ascii_digit()) {
                return segs[1].to_ascii_lowercase();
            }
            return last.to_ascii_lowercase();
        }
    }
    let t = base.rsplit(['/', ':']).next().unwrap_or(base).to_ascii_lowercase();
    t.to_string()
}

pub(crate) fn same_media_file(a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let ka = calendar_cover_filename(a);
    let kb = calendar_cover_filename(b);
    !ka.is_empty() && ka == kb
}

pub(crate) fn parse_template_first_args(wt: &str, marker: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = wt;
    while let Some(i) = rest.find(marker) {
        rest = &rest[i + marker.len()..];
        let end = rest.find("}}").unwrap_or(rest.len());
        let inner = &rest[..end];
        rest = if end < rest.len() { &rest[end + 2..] } else { "" };
        let first = inner.split('|').next().unwrap_or("").trim();
        if !first.is_empty() && !out.iter().any(|s| s == first) {
            out.push(first.to_string());
        }
    }
    out
}

pub(crate) fn preview(slot: &str, name: String, kind: String, icon: String, wear: String) -> SkyCalendarPreview {
    SkyCalendarPreview {
        slot: slot.into(),
        name,
        kind,
        icon,
        wear,
    }
}

pub(crate) fn split_cosmetics(raw: &str) -> Vec<String> {
    raw.split(['、', '，', ',', '/'])
        .map(|s| s.trim().to_string())
        .filter(|s| COSMETIC_KINDS.iter().any(|k| s == *k || s.starts_with(*k)))
        .collect()
}

pub(crate) fn smw_first_text(printouts: &serde_json::Value, key: &str) -> String {
    let Some(first) = printouts.get(key).and_then(|v| v.as_array()).and_then(|a| a.first()) else {
        return String::new();
    };
    if let Some(s) = first.as_str() {
        return s.to_string();
    }
    first
        .get("raw")
        .or_else(|| first.get("timestamp"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub(crate) async fn wiki_wikitext(client: &reqwest::Client, page: &str) -> Result<String, String> {
    let res = client
        .post("https://wiki.biligame.com/sky/api.php")
        .form(&[
            ("action", "parse"),
            ("page", page),
            ("prop", "wikitext"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let val: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(val
        .pointer("/parse/wikitext/*")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

pub(crate) async fn wiki_ask(client: &reqwest::Client, query: &str) -> Result<serde_json::Value, String> {
    let res = client
        .post("https://wiki.biligame.com/sky/api.php")
        .form(&[("action", "ask"), ("query", query), ("format", "json")])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    res.json().await.map_err(|e| e.to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct ExchangeTreeItem {
    pub(crate) slot: String,
    pub(crate) kind: String,
    pub(crate) cost_desc: String,
    pub(crate) candles: u32,
    pub(crate) hearts: u32,
    pub(crate) ascended: u32,
}

pub(crate) struct ExchangeTree {
    pub(crate) spirit: String,
    pub(crate) items: Vec<ExchangeTreeItem>,
    /// 买齐整棵树要的季蜡，含染料、动作、季心这些不进清单的格子。
    pub(crate) candles_total: u32,
}

/// 兑换图模板：命名参数之外每个位置参数是一格，格内 `种类;备注;货币;数量;季卡`。
/// 各季字段个数不统一（有的多一个空位），货币和季卡标记按值识别，不按位置。
pub(crate) fn parse_exchange_trees(wt: &str) -> Vec<ExchangeTree> {
    let mut trees = Vec::new();
    for inner in find_template_inners(wt, "兑换图") {
        let parts = split_template_parts(&inner);
        let mut spirit = String::new();
        let mut items: Vec<ExchangeTreeItem> = Vec::new();
        let mut candles_total = 0u32;
        for part in &parts {
            if let Some((k, v)) = part.split_once('=') {
                if k.trim() == "先祖" {
                    spirit = v.trim().to_string();
                }
                continue;
            }
            let bits: Vec<&str> = part.split(';').map(|s| s.trim()).collect();
            if let Some(i) = bits.iter().position(|b| *b == "季蜡") {
                candles_total += bits.get(i + 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            }
            let kind = bits[0];
            if season_skip_kind(kind) || kind == "动作" {
                continue;
            }
            if !COSMETIC_KINDS.iter().any(|c| kind.starts_with(*c)) && kind != "乐谱" && !kind.starts_with("毕业") {
                continue;
            }
            let slot = if bits[1..].contains(&"季卡") { "季卡" } else { "兑换" };
            if items.iter().any(|it| it.slot == slot && it.kind == kind) {
                continue;
            }
            let (cost_desc, candles, hearts, ascended) = parse_tree_cost(&bits[1..]);
            items.push(ExchangeTreeItem {
                slot: slot.to_string(),
                kind: kind.to_string(),
                cost_desc,
                candles,
                hearts,
                ascended,
            });
        }
        if !spirit.is_empty() {
            trees.push(ExchangeTree { spirit, items, candles_total });
        }
    }
    trees
}

/// 格内货币名后面紧跟数量：`季蜡;18`、`季心;1`。
fn parse_tree_cost(bits: &[&str]) -> (String, u32, u32, u32) {
    let (mut candles, mut hearts, mut ascended) = (0u32, 0u32, 0u32);
    let mut desc = String::new();
    for (i, b) in bits.iter().enumerate() {
        let slot = match *b {
            "季蜡" | "白蜡" | "蜡烛" => &mut candles,
            "季心" | "爱心" => &mut hearts,
            "升华" | "红蜡" => &mut ascended,
            _ => continue,
        };
        let amt = bits.get(i + 1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        *slot = amt;
        if amt > 0 {
            desc = format!("{}{}", amt, b);
        }
    }
    (desc, candles, hearts, ascended)
}

pub(crate) fn smw_text_list(printouts: &serde_json::Value, key: &str) -> Vec<String> {
    printouts
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_exhibit_items(wt: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = wt;
    let marker = "{{展示图|物品|";
    while let Some(i) = rest.find(marker) {
        rest = &rest[i + marker.len()..];
        let end = rest.find("}}").unwrap_or(rest.len());
        let inner = &rest[..end];
        rest = if end < rest.len() { &rest[end + 2..] } else { "" };
        let parts: Vec<&str> = inner.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            continue;
        }
        let name = parts[0].to_string();
        let kind = parts[1].to_string();
        if name.is_empty() || !COSMETIC_KINDS.iter().any(|k| kind == *k) {
            continue;
        }
        if !out.iter().any(|(n, k)| n == &name && k == &kind) {
            out.push((name, kind));
        }
    }
    out
}

pub(crate) fn calendar_event_year(event: &SkyCalendarItem) -> String {
    if event.start.len() >= 4 && event.start.as_bytes()[..4].iter().all(|c| c.is_ascii_digit()) {
        return event.start[..4].to_string();
    }
    if let Some(id) = event.id.strip_prefix("event-") {
        if id.len() >= 4 && id.as_bytes()[..4].iter().all(|c| c.is_ascii_digit()) {
            return id[..4].to_string();
        }
    }
    String::new()
}

pub(crate) fn sky_http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert(reqwest::header::REFERER, "https://wiki.biligame.com/sky/".parse().unwrap());
            h
        })
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

pub(crate) fn find_template_inners(wt: &str, name: &str) -> Vec<String> {
    let marker = format!("{{{{{}", name);
    let mut out = Vec::new();
    let mut from = 0;
    let bytes = wt.as_bytes();
    while let Some(rel) = wt[from..].find(&marker) {
        let start = from + rel;
        let mut depth: i32 = 0;
        let mut i = start;
        let mut end = None;
        while i + 1 < bytes.len() {
            if bytes[i] == b'{' && bytes[i + 1] == b'{' {
                depth += 1;
                i += 2;
                continue;
            }
            if bytes[i] == b'}' && bytes[i + 1] == b'}' {
                depth -= 1;
                i += 2;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
                continue;
            }
            i += 1;
        }
        let Some(e) = end else { break };
        let inner = &wt[start + 2..e - 2];
        let inner = inner.strip_prefix(name).unwrap_or(inner);
        out.push(inner.to_string());
        from = e;
    }
    out
}

pub(crate) fn parse_template_fields_nested(inner: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for part in split_template_parts(inner) {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.trim().to_string(), strip_wiki_comment(v.trim()));
        }
    }
    map
}

/// 按顶层 `|` 切模板参数，嵌套模板里的 `|` 不切。
pub(crate) fn split_template_parts(inner: &str) -> Vec<String> {
    let bytes = inner.as_bytes();
    let mut depth: i32 = 0;
    let mut cur = String::new();
    let mut parts = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            depth += 1;
            cur.push_str("{{");
            i += 2;
            continue;
        }
        if bytes[i] == b'}' && i + 1 < bytes.len() && bytes[i + 1] == b'}' {
            depth = depth.saturating_sub(1);
            cur.push_str("}}");
            i += 2;
            continue;
        }
        if bytes[i] == b'|' && depth == 0 {
            parts.push(std::mem::take(&mut cur));
            i += 1;
            continue;
        }
        let ch = inner[i..].chars().next().unwrap_or('\0');
        cur.push(ch);
        i += ch.len_utf8();
    }
    if !cur.trim().is_empty() {
        parts.push(cur);
    }
    parts
}

pub(crate) fn strip_wiki_comment(s: &str) -> String {
    let mut out = s.to_string();
    while let Some(i) = out.find("<!--") {
        if let Some(j) = out[i..].find("-->") {
            out.replace_range(i..i + j + 3, "");
        } else {
            out.truncate(i);
            break;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn season_skip_kind(kind: &str) -> bool {
    kind.is_empty()
        || kind == "占位"
        || kind == "任务"
        || kind == "传送"
        || kind == "邀请"
        || kind == "爱心"
        || kind == "季心"
        || kind == "好感任务"
        || kind == "随机魔法"
        || kind.contains("logo")
        || kind.contains("魔法")
        || kind.contains("染料")
}

pub(crate) async fn wiki_wikitext_chunked(client: &reqwest::Client, titles: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < titles.len() {
        let a = titles.get(i).cloned();
        let b = titles.get(i + 1).cloned();
        let c = titles.get(i + 2).cloned();
        let d = titles.get(i + 3).cloned();
        let e = titles.get(i + 4).cloned();
        let f = titles.get(i + 5).cloned();
        let (ra, rb, rc, rd, re, rf) = tokio::join!(
            async {
                match a {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
            async {
                match b {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
            async {
                match c {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
            async {
                match d {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
            async {
                match e {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
            async {
                match f {
                    Some(t) => Some((t.clone(), wiki_wikitext(client, &t).await.unwrap_or_default())),
                    None => None,
                }
            },
        );
        for row in [ra, rb, rc, rd, re, rf].into_iter().flatten() {
            out.push(row);
        }
        i += 6;
    }
    out
}

pub(crate) fn resolve_wiki_file_url(url: &str, map: &std::collections::HashMap<String, String>) -> String {
    let Some(name) = wiki_filename_from_url(url) else {
        return url.to_string();
    };
    map.get(&name)
        .cloned()
        .unwrap_or_else(|| wiki_redirect_file_url(&name))
}

pub(crate) fn event_family_of(title: &str) -> String {
    let t = title.trim();
    let mut chars = t.chars();
    for _ in 0..4 {
        match chars.next() {
            Some(c) if c.is_ascii_digit() => {}
            _ => return t.to_string(),
        }
    }
    if chars.next() == Some('年') {
        let rest: String = chars.collect();
        if !rest.is_empty() {
            return rest;
        }
    }
    t.to_string()
}

pub(crate) fn parse_season_wikitext(wt: &str, today: chrono::NaiveDate) -> Vec<SkyCalendarItem> {
    let mut items = Vec::new();
    let lines: Vec<&str> = wt.lines().map(|l| l.trim()).collect();
    let mut i = 0;
    while i + 2 < lines.len() {
        let line = lines[i];
        let name = if let Some(inner) = line.strip_prefix("|[[") {
            inner.split("]]").next().unwrap_or("").split('|').next().unwrap_or("").trim()
        } else if let Some(inner) = line.strip_prefix("[[") {
            inner.split("]]").next().unwrap_or("").split('|').next().unwrap_or("").trim()
        } else {
            ""
        };
        if name.ends_with('季') && !name.contains(':') {
            let start_line = lines[i + 1].trim_start_matches('|').trim();
            let end_line = lines[i + 2].trim_start_matches('|').trim();
            if parse_ymd_prefix(start_line).is_some() && (end_line == "未知" || parse_ymd_prefix(end_line).is_some()) {
                let start_d = parse_ymd_prefix(start_line);
                let end_d = parse_ymd_prefix(end_line);
                items.push(SkyCalendarItem {
                    id: format!("season-{}", name),
                    kind: "season".into(),
                    region: "cn".into(),
                    title: name.to_string(),
                    names: vec![name.to_string()],
                    season: name.to_string(),
                    start: start_line[..10.min(start_line.len())].to_string(),
                    end: if end_line == "未知" { String::new() } else { end_line[..10.min(end_line.len())].to_string() },
                    status: calendar_status(start_d, end_d, today),
                    note: String::new(),
                    url: format!("https://wiki.biligame.com/sky/{}", wiki_encode(name)),
                    cosmetics: String::new(),
                    icon: String::new(),
                    previews: Vec::new(),
                });
                i += 3;
                continue;
            }
        }
        i += 1;
    }
    items
}

pub(crate) fn parse_smw_pages(val: &serde_json::Value, kind: &str, today: chrono::NaiveDate) -> Vec<SkyCalendarItem> {
    let Some(results) = val.pointer("/query/results").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    for (title, row) in results {
        let printouts = row.get("printouts").cloned().unwrap_or(serde_json::json!({}));
        let start_raw = smw_first_text(&printouts, "开启时间");
        let end_raw = smw_first_text(&printouts, "结束时间");
        let start_d = parse_ymd_prefix(&start_raw);
        let end_d = parse_ymd_prefix(&end_raw);
        let url = row.get("fullurl").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let display = title.strip_prefix("2026年").or_else(|| title.strip_prefix("2025年")).unwrap_or(title);
        items.push(SkyCalendarItem {
            id: format!("{}-{}", kind, title),
            kind: kind.into(),
            region: "cn".into(),
            title: display.to_string(),
            names: vec![display.to_string()],
            season: String::new(),
            start: start_d.map(ymd).unwrap_or_else(|| start_raw.chars().take(10).collect()),
            end: end_d.map(ymd).unwrap_or_default(),
            status: if kind == "event" {
                event_window_status(start_d, end_d, today)
            } else {
                calendar_status(start_d, end_d, today)
            },
            note: String::new(),
            url,
            cosmetics: String::new(),
            icon: String::new(),
            previews: Vec::new(),
        });
    }
    items
}
