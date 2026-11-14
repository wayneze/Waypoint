// Fandom 试穿文件名、画廊与节日英译对照。
use super::super::COSMETIC_KINDS;
use super::super::wiki::*;

use super::items::*;

pub(crate) fn parse_english_month(word: &str) -> Option<u32> {
    match word.to_ascii_lowercase().as_str() {
        "january" | "jan" => Some(1),
        "february" | "feb" => Some(2),
        "march" | "mar" => Some(3),
        "april" | "apr" => Some(4),
        "may" => Some(5),
        "june" | "jun" => Some(6),
        "july" | "jul" => Some(7),
        "august" | "aug" => Some(8),
        "september" | "sep" | "sept" => Some(9),
        "october" | "oct" => Some(10),
        "november" | "nov" => Some(11),
        "december" | "dec" => Some(12),
        _ => None,
    }
}

pub(crate) fn parse_fandom_event_span(raw: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let s = raw.split("<ref").next().unwrap_or(raw);
    let s = s.split('|').next().unwrap_or(s).trim();
    let tokens: Vec<&str> = s
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let year: i32 = tokens.iter().rev().find_map(|t| {
        if t.len() == 4 && t.starts_with("20") {
            t.parse().ok()
        } else {
            None
        }
    })?;
    let mut md = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some(m) = parse_english_month(tokens[i]) {
            if let Some(d) = tokens
                .get(i + 1)
                .and_then(|t| t.parse::<u32>().ok())
                .filter(|d| (1..=31).contains(d))
            {
                md.push((m, d));
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    let (m1, d1) = *md.first()?;
    let (m2, d2) = *md.get(1)?;
    let y1 = if m1 > m2 { year - 1 } else { year };
    Some((
        chrono::NaiveDate::from_ymd_opt(y1, m1, d1)?,
        chrono::NaiveDate::from_ymd_opt(year, m2, d2)?,
    ))
}

pub(crate) fn parse_fandom_event_date_field(wt: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let mut from = 0;
    while let Some(rel) = wt[from..].find('|') {
        let i = from + rel + 1;
        let rest = wt[i..].trim_start();
        let lower = rest.to_ascii_lowercase();
        if !lower.starts_with("date") {
            from = i;
            continue;
        }
        let after = &rest["date".len()..];
        if after.starts_with('d') || after.starts_with('s') {
            from = i;
            continue;
        }
        let line = rest.lines().next().unwrap_or("");
        if let Some((_, v)) = line.split_once('=') {
            if let Some(span) = parse_fandom_event_span(v) {
                return Some(span);
            }
        }
        from = i;
    }
    None
}

pub(crate) fn parse_day_token(t: &str) -> Option<u32> {
    let s = t.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    s.parse().ok().filter(|d| (1..=31).contains(d))
}

pub(crate) fn parse_fandom_dated_mentions(raw: &str) -> Vec<chrono::NaiveDate> {
    let s = raw.split("<ref").next().unwrap_or(raw);
    let tokens: Vec<&str> = s
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if let Some(m) = parse_english_month(tokens[i]) {
            if let Some(d) = tokens.get(i + 1).copied().and_then(parse_day_token) {
                let year = tokens[i + 2..].iter().take(6).find_map(|t| {
                    if t.len() == 4 && t.starts_with("20") {
                        t.parse::<i32>().ok()
                    } else {
                        None
                    }
                });
                if let Some(y) = year {
                    if let Some(date) = chrono::NaiveDate::from_ymd_opt(y, m, d) {
                        out.push(date);
                    }
                }
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub(crate) fn parse_fandom_prose_spans(raw: &str) -> Vec<(chrono::NaiveDate, chrono::NaiveDate)> {
    let mut compact = Vec::new();
    for d in parse_fandom_dated_mentions(raw) {
        if compact.last() != Some(&d) {
            compact.push(d);
        }
    }
    let mut spans = Vec::new();
    let mut i = 0;
    while i + 1 < compact.len() {
        let a = compact[i];
        let b = compact[i + 1];
        let days = (b - a).num_days();
        if (1..=60).contains(&days) {
            spans.push((a, b));
            i += 2;
            continue;
        }
        i += 1;
    }
    spans
}

pub(crate) fn parse_fandom_latest_span(raw: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let mut spans = parse_fandom_prose_spans(raw);
    if let Some(span) = parse_fandom_event_date_field(raw) {
        spans.push(span);
    }
    spans.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    spans.into_iter().next()
}

/// 活动信息框拥有本届档期；只公布开始日时保留未知结束日，不借用正文里的往年日期。
pub(crate) fn parse_fandom_calendar_window(wt: &str) -> Option<(chrono::NaiveDate, Option<chrono::NaiveDate>)> {
    let inner = find_template_inners(wt, "SkyEvent").into_iter().next()?;
    let fields = parse_template_fields_nested(&inner);
    let date = fields.get("date")?;
    if let Some((start, end)) = parse_fandom_event_span(date) {
        return Some((start, Some(end)));
    }
    let dates = parse_fandom_dated_mentions(date);
    (dates.len() == 1).then(|| (dates[0], None))
}

/// 年页可能只在正文记载档期；发现历史与读取内容必须使用同一规则。
pub(crate) fn parse_fandom_event_page_window(page: &str, wt: &str) -> Option<(chrono::NaiveDate, Option<chrono::NaiveDate>)> {
    parse_fandom_calendar_window(wt).or_else(|| {
        let (_, year) = page.rsplit_once('/')?;
        if year.len() != 4 || !year.chars().all(|c| c.is_ascii_digit()) { return None; }
        parse_fandom_latest_span(wt).map(|(start, end)| (start, Some(end)))
    })
}

pub(crate) fn is_fandom_image_name(name: &str) -> bool {
    let low = name.to_ascii_lowercase();
    low.ends_with(".jpg")
        || low.ends_with(".jpeg")
        || low.ends_with(".png")
        || low.ends_with(".webp")
        || low.ends_with(".gif")
}

pub(crate) fn strip_fandom_file_prefix(name: &str) -> String {
    name.trim()
        .strip_prefix("File:")
        .or_else(|| name.trim().strip_prefix("file:"))
        .or_else(|| name.trim().strip_prefix("Image:"))
        .unwrap_or(name.trim())
        .trim()
        .to_string()
}

pub(crate) fn parse_fandom_infobox_image(wt: &str) -> Option<String> {
    let mut from = 0;
    while let Some(rel) = wt[from..].find('|') {
        let i = from + rel + 1;
        let rest = wt[i..].trim_start();
        let lower = rest.to_ascii_lowercase();
        if !(lower.starts_with("image") || lower.starts_with("promo")) {
            from = i;
            continue;
        }
        let after = if lower.starts_with("image") {
            &rest["image".len()..]
        } else {
            &rest["promo".len()..]
        };
        if after.starts_with(|c: char| c.is_ascii_alphabetic()) {
            from = i;
            continue;
        }
        let line = rest.lines().next().unwrap_or("");
        if let Some((_, v)) = line.split_once('=') {
            let name = strip_fandom_file_prefix(v.split('|').next().unwrap_or("").trim());
            if is_fandom_image_name(&name) {
                return Some(name);
            }
        }
        from = i;
    }
    None
}

pub(crate) fn parse_fandom_promo_file(wt: &str) -> Option<String> {
    if let Some(name) = parse_fandom_infobox_image(wt) {
        return Some(name);
    }
    let mut rest = wt;
    while let Some(i) = rest.find("[[") {
        let inner_src = &rest[i + 2..];
        let Some(end) = inner_src.find("]]") else {
            break;
        };
        let inner = &inner_src[..end];
        rest = &inner_src[end + 2..];
        let first = inner.split('|').next().unwrap_or("").trim();
        let name = strip_fandom_file_prefix(first);
        if name.is_empty() {
            continue;
        }
        if is_fandom_image_name(&name) {
            return Some(name);
        }
    }
    None
}

pub(crate) fn fandom_gallery_caption_zh(caption: &str) -> String {
    let c = caption.trim().to_ascii_lowercase();
    if c.contains("date") {
        "活动日程".into()
    } else if c.contains("introduction") {
        "活动介绍".into()
    } else if c.contains("new cosmetic") {
        "新增物品".into()
    } else if c.contains("returning cosmetic") {
        "回归物品".into()
    } else if c.contains("returning") {
        "回归玩法".into()
    } else if c.contains("activit") {
        "活动玩法".into()
    } else if c.contains("guide") {
        "活动指南".into()
    } else if !caption.trim().is_empty() && has_cjk(caption) {
        caption.trim().to_string()
    } else {
        String::new()
    }
}

pub(crate) fn parse_fandom_gallery_entries(wt: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let lower = wt.to_ascii_lowercase();
    let mut from = 0;
    while let Some(rel) = lower[from..].find("<gallery") {
        let abs = from + rel;
        let after = wt[abs..].find('>').map(|i| abs + i + 1).unwrap_or(wt.len());
        let end = lower[after..]
            .find("</gallery>")
            .map(|i| after + i)
            .unwrap_or(wt.len());
        for line in wt[after..end].lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('<') {
                continue;
            }
            let mut parts = t.splitn(2, '|');
            let first = parts.next().unwrap_or("").trim();
            let caption = parts.next().unwrap_or("").trim();
            let name = strip_fandom_file_prefix(first);
            if is_fandom_image_name(&name) && !out.iter().any(|(s, _)| s == &name) {
                out.push((name, fandom_gallery_caption_zh(caption)));
            }
        }
        from = end + 10;
    }
    out
}

pub(crate) fn fandom_lead_note(wt: &str) -> String {
    let mut lines = Vec::new();
    for raw in wt.lines() {
        let t = raw.trim();
        if t.starts_with('=')
            || t.starts_with("{{")
            || t.starts_with('|')
            || t.starts_with("}}")
            || t.starts_with("[[File:")
            || t.starts_with("<")
        {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if t.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        let mut s = t.replace("'''", "").replace("''", "");
        while let Some(i) = s.find("[[") {
            let Some(j) = s[i..].find("]]") else {
                break;
            };
            let shown = s[i + 2..i + j]
                .split('|')
                .next_back()
                .unwrap_or("")
                .to_string();
            s.replace_range(i..i + j + 2, &shown);
        }
        while let Some(i) = s.find('{') {
            let Some(j) = s[i..].find('}') else {
                break;
            };
            s.replace_range(i..i + j + 1, "");
        }
        let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
        if s.chars().count() >= 12 {
            lines.push(s);
        }
        if lines.join("").chars().count() >= 80 {
            break;
        }
    }
    let joined = lines.join(" ");
    if joined.chars().count() <= 140 {
        joined
    } else {
        joined.chars().take(140).collect::<String>()
    }
}

pub(crate) fn fandom_event_title_zh(page: &str, wt: &str, zh_names: &[String]) -> String {
    if let Some(zh) = zh_names.iter().find(|s| !s.trim().is_empty()) {
        return canonical_event_title(&fold_zh_event_name(zh));
    }
    if let Some(zh) = parse_displaytitle_zh(wt) {
        return canonical_event_title(&fold_zh_event_name(&zh));
    }
    let root = fandom_event_root(page);
    canonical_event_title(&match root.as_str() {
        "Days of Moonlight" => "秋宵节".into(),
        "Days of Mischief" => "恶作剧之日".into(),
        "Days of Feast" => "宴会节".into(),
        "Days of Sunlight" => "夏之日".into(),
        "Days of Bloom" => "花憩节".into(),
        "Days of Fortune" => "春节".into(),
        "Days of Nature" => "自然日".into(),
        "Days of Color" => "缤纷飞行日".into(),
        "Sky Anniversary" => "周年庆".into(),
        _ => root,
    })
}

pub(crate) fn parse_displaytitle_zh(wt: &str) -> Option<String> {
    let marker = "{{DISPLAYTITLE:";
    let i = wt.find(marker)?;
    let rest = &wt[i + marker.len()..];
    let end = rest.find("}}")?;
    let inner = rest[..end].trim();
    let zh: String = inner
        .chars()
        .filter(|c| (0x4E00..=0x9FFF).contains(&(*c as u32)))
        .collect();
    if zh.chars().count() >= 2 {
        Some(zh)
    } else {
        None
    }
}

pub(crate) fn parse_fandom_item_names(wt: &str) -> Vec<String> {
    let mut names = parse_template_first_args(wt, "{{Icon Name|");
    for n in parse_template_first_args(wt, "{{Days Item|") {
        if !names.iter().any(|s| s == &n) {
            names.push(n);
        }
    }
    names
}

pub(crate) fn parse_expanded_pngs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split(|c: char| c.is_whitespace() || c == '[' || c == ']' || c == '{' || c == '}' || c == '|') {
        let name = raw.trim().trim_start_matches("File:");
        if name.len() > 4 && name.to_ascii_lowercase().ends_with(".png") && is_fandom_wear_file(name)
            && !out.iter().any(|s| s == name) {
                out.push(name.to_string());
            }
    }
    out
}

pub(crate) fn fandom_lua_field_at(block: &str, key: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = block[from..].find(key) {
        let i = from + rel;
        let prev_ok = i == 0 || {
            let p = block.as_bytes()[i - 1];
            !(p == b'_' || p.is_ascii_alphanumeric())
        };
        if prev_ok && block[i + key.len()..].trim_start().starts_with('=') {
            return Some(i);
        }
        from = i + key.len().max(1);
    }
    None
}

pub(crate) fn fandom_lua_quoted(block: &str, key: &str) -> Option<String> {
    let i = fandom_lua_field_at(block, key)?;
    let rest = block[i + key.len()..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let rest = &rest[1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 找到包住 `at` 的那层 Lua 表。条目里有 `alt_name = { ... }` 这种内层表，结尾 `}` 也带缩进，
/// 不能用最近的 `{` 和行首 `}` 当边界，否则会读到相邻条目的字段。
fn lua_enclosing_table(src: &str, at: usize) -> Option<(usize, usize)> {
    let bytes = src.as_bytes();
    let mut stack = Vec::new();
    let mut target = None;
    let mut quote = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
        } else if b == b'-' && bytes.get(i + 1) == Some(&b'-') {
            let start = i + 2;
            let mut opening = start + 1;
            while bytes.get(opening) == Some(&b'=') { opening += 1; }
            if bytes.get(start) == Some(&b'[') && bytes.get(opening) == Some(&b'[') {
                let closing = format!("]{}]", "=".repeat(opening - start - 1));
                i = src[opening + 1..].find(&closing).map(|offset| opening + 1 + offset + closing.len()).unwrap_or(bytes.len());
            } else {
                i += bytes[i..].iter().position(|b| *b == b'\n').unwrap_or(bytes.len() - i);
            }
            continue;
        } else {
            if i == at {
                target = stack.last().copied();
            }
            match b {
                b'"' | b'\'' => quote = Some(b),
                b'{' => stack.push(i),
                b'}' => {
                    let start = stack.pop()?;
                    if target == Some(start) {
                        return Some((start, i + 1));
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

pub(crate) fn parse_fandom_spirit_wear(
    wt: &str,
    spirit_en: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
    // Module:Spirits/data 是 Icon Name 的数据来源。键描述物品和视角，
    // 文件名本身不必包含先祖名、类型或 front 等词。
    const KINDS: &[(&str, &[&str])] = &[
        ("面具", &["mask"]), ("发型", &["hair"]), ("斗篷", &["cape"]),
        ("裤子", &["outfit"]), ("鞋子", &["footwear", "shoes"]),
        ("颈饰", &["neckpiece"]), ("发饰", &["hairpiece", "headpiece"]),
        ("耳饰", &["headpiece", "facepiece"]), ("面饰", &["facepiece"]),
        ("手持", &["instrument", "prop"]), ("背饰", &["prop"]),
        ("小型家具", &["prop"]), ("大型家具", &["prop"]),
    ];
    let normalize = |s: &str| s.trim().replace('_', " ").to_ascii_lowercase();
    let mut out = std::collections::HashMap::new();
    let mut from = 0;
    while let Some(rel) = fandom_lua_field_at(&wt[from..], "name") {
        let at = from + rel;
        from = at + "name".len();
        let Some((start, end)) = lua_enclosing_table(wt, at) else { continue };
        from = end;
        let block = &wt[start..end];
        let Some(name) = fandom_lua_quoted(block, "name") else { continue };
        let mut names = vec![normalize(&name)];
        if let Some(guide) = fandom_lua_quoted(block, "guide_name") { names.push(normalize(&guide)); }
        if let Some(alias_at) = fandom_lua_field_at(block, "alt_name") {
            let tail = block[alias_at + "alt_name".len()..].trim_start();
            if let Some(tail) = tail.strip_prefix('=').map(str::trim_start) {
                if tail.starts_with('{') {
                    let open = block.len() - tail.len();
                    if let Some((s, e)) = lua_enclosing_table(block, open + 1) {
                        names.extend(block[s..e].split('"').skip(1).step_by(2).map(normalize));
                    }
                }
            }
        }
        let owners: Vec<&String> = spirit_en.iter()
            .filter(|(_, en)| names.contains(&normalize(en)))
            .map(|(cn, _)| cn).collect();
        if owners.is_empty() {
            continue;
        }
        let fields: std::collections::HashMap<&str, String> = block.lines().filter_map(|line| {
            let (key, _) = line.trim().split_once('=')?;
            let key = key.trim();
            if !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return None;
            }
            Some((key, fandom_lua_quoted(line, key)?))
        }).collect();
        let mut items = std::collections::HashMap::new();
        for &(kind, prefixes) in KINDS {
          for graduation in [false, true] {
            let mut variants = Vec::new();
            for key in fields.keys() {
                for view in ["front", "real", "held", "back"] {
                    if let Some(base) = key.strip_suffix(&format!("_{view}")) {
                        if base.split('_').any(|part| part == "u") == graduation
                            && prefixes.iter().any(|p| base == *p || base.starts_with(&format!("{p}_"))) {
                            variants.push(base);
                        }
                    }
                }
            }
            variants.sort_unstable();
            variants.dedup();
            let mut candidates = Vec::new();
            for base in variants {
                if let Some(file) = ["front", "real", "held", "back"].iter().find_map(|view| {
                    let file = fields.get(format!("{base}_{view}").as_str())?.trim();
                    let ext = file.rsplit('.').next()?.to_ascii_lowercase();
                    (matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp")
                        && Some(file) != fields.get(base).map(|s| s.trim())).then_some(file)
                }) {
                    candidates.push(file.to_string());
                }
            }
            candidates.sort_by_key(|file| file.replace('_', " "));
            candidates.dedup_by(|a, b| a.replace('_', " ") == b.replace('_', " "));
            // 多件同类物品没有可靠编号对应时，不任意挑一件。
            if candidates.len() == 1 {
                items.insert(if graduation { format!("毕业{kind}") } else { kind.to_string() }, candidates.remove(0));
            }
          }
        }
        for owner in owners {
            out.insert(owner.clone(), items.clone());
        }
    }
    out
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FandomDaysItem {
    pub(crate) days: String,
    pub(crate) year: String,
    pub(crate) name: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) iap: bool,
    pub(crate) icon: String,
    pub(crate) files: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct FandomCosmetic {
    pub kind: String,
    pub icon: String,
    pub wear: String,
    pub season: Option<u32>,
}

/// 从同一物品块的 icon/real 或 cape/cape_front 等字段读取关联，不靠中英文译名。
pub(crate) fn parse_fandom_cosmetics(wt: &str) -> Vec<FandomCosmetic> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = fandom_lua_field_at(&wt[from..], "name") {
        let at = from + rel;
        from = at + 4;
        let Some((start, end)) = lua_enclosing_table(wt, at) else { continue };
        from = end;
        let block = &wt[start..end];
        let fields: std::collections::HashMap<_, _> = block.lines().filter_map(|line| {
            let (key, _) = line.trim().split_once('=')?;
            let key = key.trim();
            Some((key, fandom_lua_quoted(line, key)?.trim().to_string()))
        }).collect();
        let number = |field: &str| -> Option<u32> {
            let at = fandom_lua_field_at(block, field)?;
            block[at + field.len()..].trim_start().strip_prefix('=')?.trim_start()
                .split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
        };
        let season = number("sort").or_else(|| number("order").map(|n| n / 100)).filter(|n| *n > 0);
        for (&key, icon) in &fields {
            let prefix = key.split('_').next().unwrap_or(key);
            let kind = if key == "icon" {
                fields.get("item_type").map(String::as_str)
                    .unwrap_or(if fields.contains_key("days") { "*" } else { "" })
            } else { prefix };
            let kind = match kind {
                "cape" => "wing", "outfit" | "outift" => "body", "hair" => "hair",
                "mask" => "mask", "hairpiece" | "headpiece" => "horn",
                "facepiece" => "face", "neckpiece" | "necklace" => "neck",
                "footwear" | "shoes" => "feet", "prop" | "instrument" => "prop",
                "tail" => "tail", "*" => "*",
                _ => continue,
            };
            let wear = ["front", "real", "held", "back"].iter().find_map(|view| {
                let field = if key == "icon" { view.to_string() } else { format!("{key}_{view}") };
                fields.get(field.as_str()).filter(|s| !s.is_empty() && *s != icon && !is_fandom_rejected_wear(s))
            });
            if let Some(wear) = wear.filter(|_| !icon.is_empty()) {
                out.push(FandomCosmetic { kind: kind.into(), icon: icon.clone(), wear: wear.clone(), season });
            }
        }
    }
    out.sort_by(|a, b| (&a.kind, &a.icon).cmp(&(&b.kind, &b.icon)));
    out.dedup_by(|a, b| a.kind == b.kind && a.icon == b.icon);
    out
}

/// 衣柜页面也有直接配对的 Icon 单元格，补全没有放进 Lua 模块的家具。
pub(crate) fn parse_fandom_closet_cosmetics(wt: &str, kind: &str) -> Vec<FandomCosmetic> {
    let cells: Vec<_> = wt.lines().collect();
    let mut out = Vec::new();
    for pair in cells.windows(2) {
        let cell = |line: &str| -> Option<Vec<String>> {
            let body = line.trim().strip_prefix('|')?.trim().strip_prefix("{{Icon|")?.strip_suffix("}}")?;
            Some(body.split('|').map(|s| s.trim().to_string()).collect())
        };
        let (Some(icon), Some(wear)) = (cell(pair[0]), cell(pair[1])) else { continue };
        if !icon.iter().any(|s| s == "tab") || !wear.iter().any(|s| s == "closet_display") { continue; }
        let (Some(icon), Some(wear)) = (icon.first(), wear.first()) else { continue };
        if icon != wear && !is_fandom_rejected_wear(wear) {
            out.push(FandomCosmetic { kind: kind.into(), icon: icon.clone(), wear: wear.clone(), season: None });
        }
    }
    out
}

pub(crate) fn parse_fandom_days_item_data(wt: &str) -> Vec<FandomDaysItem> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = fandom_lua_field_at(&wt[from..], "days") {
        let abs = from + rel;
        from = abs + "days".len();
        let Some((start, end)) = lua_enclosing_table(wt, abs) else {
            continue;
        };
        let block = &wt[start..end];
        from = end;
        let days = fandom_lua_quoted(block, "days").unwrap_or_default();
        if days.is_empty() {
            continue;
        }
        let year = fandom_lua_quoted(block, "year").unwrap_or_default();
        let item_name = fandom_lua_quoted(block, "name").unwrap_or_default();
        let aliases = fandom_lua_field_at(block, "alt_name")
            .and_then(|at| block[at..].split_once('='))
            .map(|(_, tail)| tail.trim_start())
            .filter(|tail| tail.starts_with('{'))
            .and_then(|tail| tail.split_once('}'))
            .map(|(list, _)| list.split('"').skip(1).step_by(2).map(str::to_owned).collect())
            .unwrap_or_default();
        let iap = fandom_lua_quoted(block, "iap").is_some_and(|v| !v.is_empty());
        let icon = fandom_lua_quoted(block, "icon").unwrap_or_default();
        let mut files = Vec::new();
        for key in ["front", "real", "held"] {
            if let Some(name) = fandom_lua_quoted(block, key) {
                if name != icon && !is_fandom_rejected_wear(&name) && !files.iter().any(|s| s == &name) {
                    files.push(name);
                }
            }
        }
        if files.is_empty() {
            if let Some(name) = fandom_lua_quoted(block, "back") {
                if name != icon && !is_fandom_rejected_wear(&name) && !files.iter().any(|s| s == &name) {
                    files.push(name);
                }
            }
        }
        if !files.is_empty() || !icon.is_empty() {
            out.push(FandomDaysItem { days, year, name: item_name, aliases, iap, icon, files });
        }
    }
    out
}

pub(crate) fn fandom_kind_from_en_name(name: &str) -> Option<String> {
    let mut best: Option<&'static str> = None;
    for k in COSMETIC_KINDS {
        if is_furniture_kind(k) {
            continue;
        }
        if fandom_kind_match(k, name) {
            let better = best
                .map(|b| k.chars().count() > b.chars().count())
                .unwrap_or(true);
            if better {
                best = Some(*k);
            }
        }
    }
    best.map(|s| s.to_string())
}

pub(crate) fn fandom_en_for_cn_event(title: &str) -> Option<&'static str> {
    let family = canonical_event_title(&event_family_of(title));
    match family.as_str() {
        "秋宵节" => Some("Days of Moonlight"),
        "恶作剧之日" => Some("Days of Mischief"),
        "宴会节" => Some("Days of Feast"),
        "夏之日" => Some("Days of Sunlight"),
        "花憩节" => Some("Days of Bloom"),
        "春节" => Some("Days of Fortune"),
        "自然日" => Some("Days of Nature"),
        "缤纷飞行日" => Some("Days of Color"),
        "周年庆" => Some("Sky Anniversary"),
        "我的世界联动" | "我的世界" => Some("NetEase/Minecraft"),
        _ => None,
    }
}
