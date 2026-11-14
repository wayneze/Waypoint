// 光遇复刻表：国服/国际旅行先祖窗口。
use super::super::SkyCalendarItem;
use super::super::wiki::*;
use super::super::fandom::*;
use super::super::events::parse_event_packs;
use super::merge::calendar_icon_usable;

pub(crate) fn extract_reprint_templates(wt: &str) -> Vec<std::collections::HashMap<String, String>> {
    let mut out = Vec::new();
    let mut rest = wt;
    let marker = "{{复刻项|";
    while let Some(i) = rest.find(marker) {
        rest = &rest[i + marker.len()..];
        if let Some(end) = rest.find("}}") {
            out.push(parse_template_fields(&rest[..end]));
            rest = &rest[end + 2..];
        } else {
            break;
        }
    }
    out
}

pub(crate) fn reprint_spirit_names(
    rows: &[std::collections::HashMap<String, String>],
) -> std::collections::HashMap<String, String> {
    rows.iter().filter_map(|row| {
        let cn = row.get("先祖中文名")?.trim();
        let en = row.get("先祖英文名")?.trim();
        (!cn.is_empty() && !en.is_empty()).then(|| (cn.to_string(), en.to_string()))
    }).collect()
}

pub(crate) fn apply_wear_urls(item: &mut SkyCalendarItem, map: &std::collections::HashMap<String, String>) {
    if map.is_empty() {
        return;
    }
    for p in item.previews.iter_mut() {
        if p.wear.is_empty() {
            continue;
        }
        // 展示图文件名是按"名字+部位"拼的，wiki 上没有这张就清掉，交给 Fandom 补或退回图标
        if let Some(name) = wear_filename_from_url(&p.wear) {
            p.wear = map.get(&name).cloned().unwrap_or_default();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn fandom_calendar_item(
    page: &str,
    title: &str,
    start: chrono::NaiveDate,
    end: Option<chrono::NaiveDate>,
    today: chrono::NaiveDate,
    promo: Option<&str>,
    note: String,
    region: &str,
) -> SkyCalendarItem {
    let key = fandom_event_root(page)
        .to_ascii_lowercase()
        .replace(' ', "-");
    SkyCalendarItem {
        id: format!("{}-fandom-{}", region, key),
        kind: "event".into(),
        region: region.into(),
        title: title.to_string(),
        names: vec![title.to_string()],
        season: String::new(),
        start: ymd(start),
        end: end.map(ymd).unwrap_or_default(),
        status: event_window_status(Some(start), end, today),
        note,
        url: format!(
            "https://sky-children-of-the-light.fandom.com/wiki/{}",
            wiki_encode(&page.replace(' ', "_"))
        ),
        cosmetics: String::new(),
        icon: promo.map(|f| format!("File:{}", f)).unwrap_or_default(),
        previews: Vec::new(),
    }
}

pub(crate) fn netease_title_decorate(name: &str, collab: bool) -> String {
    let n = name.trim();
    if n.is_empty() {
        return String::new();
    }
    if !collab || n.contains('联') || n.contains('节') || n.contains('日') || n.contains('庆') {
        n.to_string()
    } else {
        format!("{}联动", n)
    }
}

pub(crate) fn netease_event_title(page: &str, zh_hint: &str, wt: &str, collab: bool) -> String {
    if !zh_hint.trim().is_empty() {
        return netease_title_decorate(zh_hint, collab);
    }
    if let Some(zh) = parse_displaytitle_zh(wt) {
        return netease_title_decorate(&zh, collab);
    }
    let mut run = String::new();
    for c in wt.chars().take(800) {
        if (0x4E00..=0x9FFF).contains(&(c as u32)) {
            run.push(c);
        } else if run.chars().count() >= 2 {
            return netease_title_decorate(&run, collab);
        } else {
            run.clear();
        }
    }
    if run.chars().count() >= 2 {
        return netease_title_decorate(&run, collab);
    }
    let en = page.rsplit('/').next().unwrap_or(page).trim();
    netease_title_decorate(en, collab)
}

pub(crate) fn parse_netease_special_event_links(wt: &str) -> Vec<(String, String, bool)> {
    let mut collab = false;
    let mut out = Vec::new();
    for line in wt.lines() {
        let t = line.trim();
        if t.contains("List of Special Collaboration") {
            collab = true;
            continue;
        }
        if t.contains("List of Special Events") {
            collab = false;
            continue;
        }
        if !t.starts_with('*') {
            continue;
        }
        let Some(start) = t.find("[[") else {
            continue;
        };
        let Some(end) = t[start + 2..].find("]]") else {
            continue;
        };
        let inner = &t[start + 2..start + 2 + end];
        let page = inner.split('|').next().unwrap_or("").trim();
        if page.is_empty() {
            continue;
        }
        let low = page.replace('_', " ").to_ascii_lowercase();
        if !low.starts_with("netease/") {
            continue;
        }
        let zh = t[start + 2 + end + 2..].trim().to_string();
        out.push((page.replace('_', " "), zh, collab));
    }
    out
}

pub(crate) fn netease_page_key(title: &str) -> String {
    title
        .rsplit('/')
        .next()
        .unwrap_or(title)
        .to_ascii_lowercase()
        .replace('_', " ")
}

pub(crate) fn fill_reprint_looks(item: &mut SkyCalendarItem, looks: &[(String, String)]) {
    for (name, cosmetics) in looks {
        if name.is_empty() {
            continue;
        }
        let url = spirit_icon_url(name);
        if item.icon.is_empty() {
            item.icon = url.clone();
        }
        item.previews.push(preview(
            "先祖",
            name.clone(),
            "先祖".into(),
            url,
            String::new(),
        ));
        for kind in split_cosmetics(cosmetics) {
            item.previews.push(preview(
                "兑换",
                format!("{} · {}", name, kind),
                kind.clone(),
                cosmetic_icon_url(&kind, name),
                cosmetic_wear_url(&kind, name),
            ));
        }
    }
}

pub(crate) fn fill_reprint_media(item: &mut SkyCalendarItem) {
    if item.previews.iter().any(|p| p.slot == "先祖") {
        return;
    }
    let looks: Vec<(String, String)> = item
        .names
        .iter()
        .map(|n| (n.clone(), item.cosmetics.clone()))
        .collect();
    fill_reprint_looks(item, &looks);
}

pub(crate) fn grouped_reprints(
    rows: Vec<std::collections::HashMap<String, String>>,
    region: &str,
    page_url: &str,
    today: chrono::NaiveDate,
) -> Vec<SkyCalendarItem> {
    #[derive(Clone)]
    struct Acc {
        kind: String,
        seq: String,
        start: String,
        end: String,
        season: String,
        names: Vec<String>,
        looks: Vec<(String, String)>,
    }
    let mut order: Vec<String> = Vec::new();
    let mut grouped: std::collections::HashMap<String, Acc> = std::collections::HashMap::new();
    for row in rows {
        let kind = row.get("复刻类型").cloned().unwrap_or_default();
        if kind.is_empty() {
            continue;
        }
        let start = row.get("开始时间").cloned().unwrap_or_default();
        let end = row.get("结束时间").cloned().unwrap_or_default();
        let seq = row.get("同类序号").cloned().unwrap_or_default();
        let season = row.get("所属季节").cloned().unwrap_or_default();
        let cosmetics = row.get("装扮").cloned().unwrap_or_default();
        let name = row.get("先祖中文名").cloned().unwrap_or_default();
        let key = format!("{}|{}|{}|{}", kind, seq, start, end);
        if !grouped.contains_key(&key) {
            order.push(key.clone());
            grouped.insert(key.clone(), Acc {
                kind,
                seq,
                start,
                end,
                season,
                names: Vec::new(),
                looks: Vec::new(),
            });
        }
        if let Some(acc) = grouped.get_mut(&key) {
            if !name.is_empty() && !acc.names.contains(&name) {
                acc.names.push(name.clone());
                acc.looks.push((name, cosmetics));
            }
        }
    }
    order.into_iter().filter_map(|key| {
        let acc = grouped.remove(&key)?;
        let start_d = parse_ymd_prefix(&acc.start);
        let end_d = parse_ymd_prefix(&acc.end);
        // 复刻必须有起止日。缺结束日不能当成一直进行中（wiki 有 2023-1-2 这种未补零写法）。
        let status = match (start_d, end_d) {
            (Some(s), Some(e)) if s <= today && today <= e => "live".into(),
            (Some(s), Some(_)) if s > today => "upcoming".into(),
            (Some(s), None) if s > today => "upcoming".into(),
            _ => "ended".into(),
        };
        let title = if acc.kind.contains("集体") {
            if acc.seq.is_empty() {
                format!("集体回归 · {}", acc.names.join("、"))
            } else {
                format!("集体回归 {} · {}", acc.seq, acc.names.join("、"))
            }
        } else {
            acc.names.first().cloned().unwrap_or_else(|| "旅行先祖".into())
        };
        let id = format!("{}-{}-{}", region, acc.seq, start_d.map(ymd).unwrap_or_else(|| acc.start.clone()));
        let cosmetics = acc
            .looks
            .iter()
            .flat_map(|(_, raw)| split_cosmetics(raw))
            .fold(Vec::new(), |mut out, kind| {
                if !out.contains(&kind) {
                    out.push(kind);
                }
                out
            })
            .join("、");
        let mut item = SkyCalendarItem {
            id,
            kind: "reprint".into(),
            region: region.into(),
            title,
            names: acc.names,
            season: acc.season,
            start: start_d.map(ymd).unwrap_or(acc.start),
            end: end_d.map(ymd).unwrap_or(acc.end),
            status,
            note: String::new(),
            url: page_url.into(),
            cosmetics,
            icon: String::new(),
            previews: Vec::new(),
        };
        fill_reprint_looks(&mut item, &acc.looks);
        Some(item)
    }).collect()
}

pub(crate) fn last_regular(items: &[SkyCalendarItem]) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    let mut best: Option<(chrono::NaiveDate, chrono::NaiveDate)> = None;
    for it in items {
        if it.title.contains("集体") {
            continue;
        }
        let Some(s) = parse_ymd_prefix(&it.start) else { continue };
        let Some(e) = parse_ymd_prefix(&it.end) else { continue };
        if best.map(|(bs, _)| s > bs).unwrap_or(true) {
            best = Some((s, e));
        }
    }
    best
}

pub(crate) fn infer_regular_windows(
    last: (chrono::NaiveDate, chrono::NaiveDate),
    today: chrono::NaiveDate,
    named_starts: &std::collections::HashSet<chrono::NaiveDate>,
    count: usize,
) -> Vec<(chrono::NaiveDate, chrono::NaiveDate)> {
    let dur = (last.1 - last.0).num_days();
    if dur <= 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = last.0 + chrono::Duration::days(14);
    for _ in 0..24 {
        if out.len() >= count {
            break;
        }
        if start >= today && !named_starts.contains(&start) {
            out.push((start, start + chrono::Duration::days(dur)));
        }
        start += chrono::Duration::days(14);
    }
    out
}

pub(crate) fn pick_current_season(mut items: Vec<SkyCalendarItem>) -> Option<SkyCalendarItem> {
    if let Some(live) = items.iter().find(|s| s.status == "live").cloned() {
        return Some(live);
    }
    items.sort_by(|a, b| b.start.cmp(&a.start));
    items.into_iter().next()
}

pub(crate) fn current_intl_reprints(
    mut intl: Vec<SkyCalendarItem>,
    cn: &[SkyCalendarItem],
    today: chrono::NaiveDate,
) -> (Vec<SkyCalendarItem>, Vec<SkyCalendarItem>) {
    let mut last = std::collections::HashMap::new();
    for it in cn {
        let Some(start) = parse_ymd_prefix(&it.start) else { continue };
        if start > today { continue; }
        for name in &it.names {
            last.entry(name.clone())
                .and_modify(|d| {
                    if start > *d {
                        *d = start;
                    }
                })
                .or_insert(start);
        }
    }
    intl.retain(|it| matches!(it.status.as_str(), "live" | "upcoming") && !it.names.is_empty());
    for it in &mut intl {
        if cn.is_empty() {
            it.note = "国服历史回归记录暂不可用".into();
            continue;
        }
        it.note = it.names.iter().map(|name| {
            let history = last.get(name)
                .map(|date| format!("国服上次回归：{}", ymd(*date)))
                .unwrap_or_else(|| "未查到国服历史回归记录".into());
            if it.names.len() > 1 { format!("{name} · {history}") } else { history }
        }).collect::<Vec<_>>().join("；");
    }
    split_now_next(intl)
}

pub(crate) fn fill_season_media(
    season: &mut SkyCalendarItem,
    trees: Vec<ExchangeTree>,
    packs: &serde_json::Value,
) {
    season.icon = season_logo_url(&season.title);
    for tree in trees {
        let spirit = tree.spirit;
        season.previews.push(preview(
            "先祖",
            spirit.clone(),
            "先祖".into(),
            spirit_icon_url(&spirit),
            String::new(),
        ));
        for ExchangeTreeItem { slot, kind, .. } in tree.items {
            let owner = if kind.starts_with("毕业") {
                format!("{}毕业礼", season.title)
            } else {
                spirit.clone()
            };
            let file_kind = cosmetic_file_kind(&kind);
            if file_kind.is_empty() {
                continue;
            }
            season.previews.push(preview(
                &slot,
                format!("{} · {}", spirit, kind),
                kind,
                cosmetic_icon_url(&file_kind, &owner),
                cosmetic_wear_url(&file_kind, &owner),
            ));
        }
    }
    if let Some(results) = packs.pointer("/query/results").and_then(|v| v.as_object()) {
        for (title, row) in results {
            let printouts = row.get("printouts").cloned().unwrap_or(serde_json::json!({}));
            let goods = smw_text_list(&printouts, "物品");
            let price = smw_first_text(&printouts, "价格");
            let stem = title.replace("礼包", "");
            let icon = goods
                .first()
                .map(|good| cosmetic_icon_url(good, &stem))
                .unwrap_or_default();
            season.previews.push(preview(
                "商城",
                if price.is_empty() { title.clone() } else { format!("{} · {}", title, price) },
                goods.join("、"),
                icon,
                String::new(),
            ));
        }
    }
}

pub(crate) fn fill_event_media(event: &mut SkyCalendarItem, exhibits: Vec<(String, String)>, packs: &serde_json::Value) {
    if !calendar_icon_usable(&event.icon) && !event.icon.starts_with("File:") {
        event.icon = wiki_file_url(&format!("UI-{}.png", event.title));
    }
    event.previews.retain(|p| p.slot != "兑换" && p.slot != "商城");
    for (name, kind) in exhibits {
        event.previews.push(preview(
            "兑换",
            format!("{} · {}", name, kind),
            kind.clone(),
            cosmetic_icon_url(&kind, &name),
            cosmetic_wear_url(&kind, &name),
        ));
    }
    let mut shop = Vec::new();
    parse_event_packs(packs, &event.title, &mut shop);
    for item in shop {
        event.previews.push(preview(
            "商城",
            if item.cost_desc.is_empty() {
                item.name.clone()
            } else {
                format!("{} · {}", item.name, item.cost_desc)
            },
            item.kind,
            item.icon,
            String::new(),
        ));
    }
}

pub(crate) fn split_now_next(items: Vec<SkyCalendarItem>) -> (Vec<SkyCalendarItem>, Vec<SkyCalendarItem>) {
    let mut now = Vec::new();
    let mut next = Vec::new();
    for it in items {
        match it.status.as_str() {
            "live" => now.push(it),
            "upcoming" | "inferred" => next.push(it),
            _ => {}
        }
    }
    next.sort_by(|a, b| a.start.cmp(&b.start));
    (now, next)
}

pub(crate) fn calendar_event_wiki_pages(event: &SkyCalendarItem) -> Vec<String> {
    let mut pages = Vec::new();
    if let Some(page) = event.id.strip_prefix("event-") {
        if !page.is_empty() {
            pages.push(page.to_string());
        }
    }
    let family = event_family_of(&event.title);
    if family.chars().count() >= 2 {
        let year = calendar_event_year(event);
        if year.len() == 4 {
            let dated = format!("{}年{}", year, family);
            if !pages.iter().any(|p| p == &dated) {
                pages.push(dated);
            }
        }
        if !pages.iter().any(|p| p == &family) {
            pages.push(family);
        }
    }
    pages
}

pub(crate) fn wiki_redirect_target(wt: &str) -> Option<String> {
    let t = wt.trim_start();
    let rest = t
        .strip_prefix("#重定向")
        .or_else(|| t.strip_prefix("#REDIRECT"))
        .or_else(|| t.strip_prefix("#redirect"))?;
    let i = rest.find("[[")?;
    let inner = rest[i + 2..].split("]]").next()?;
    let target = inner.split('|').next()?.trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

pub(crate) fn wiki_page_year(title: &str) -> Option<String> {
    let t = title.trim();
    let bytes = t.as_bytes();
    if bytes.len() >= 5 && bytes[..4].iter().all(|c| c.is_ascii_digit()) && t.contains('年') {
        Some(t[..4].to_string())
    } else {
        None
    }
}

pub(crate) fn calendar_event_wikitext<'a>(
    event: &SkyCalendarItem,
    map: &'a std::collections::HashMap<String, String>,
) -> Option<&'a str> {
    let want_year = calendar_event_year(event);
    for page in calendar_event_wiki_pages(event) {
        let Some(wt) = map.get(&page).filter(|s| !s.is_empty()) else {
            continue;
        };
        if let Some(target) = wiki_redirect_target(wt) {
            if let (true, Some(got)) = (!want_year.is_empty(), wiki_page_year(&target)) {
                if got != want_year {
                    continue;
                }
            }
            if let Some(real) = map.get(&target).filter(|s| !s.is_empty() && wiki_redirect_target(s).is_none()) {
                if let (true, Some(got)) = (!want_year.is_empty(), wiki_page_year(&target)) {
                    if got != want_year {
                        continue;
                    }
                }
                return Some(real.as_str());
            }
            continue;
        }
        if let (true, Some(got)) = (!want_year.is_empty(), wiki_page_year(&page)) {
            if got != want_year {
                continue;
            }
        }
        return Some(wt.as_str());
    }
    None
}
