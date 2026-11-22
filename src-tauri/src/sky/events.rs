// 节日活动目录。
use super::{COSMETIC_KINDS, SkyCalendarItem, SkyEventCatalog, SkySeasonCatalogItem, SkySeasonWishItem};
use super::wiki::*;
use super::fandom::*;


pub(crate) fn infer_cosmetic_kind(name: &str) -> Option<&'static str> {
    let mut best: Option<&'static str> = None;
    for k in COSMETIC_KINDS {
        if name.ends_with(k) {
            let better = best
                .map(|b| k.chars().count() > b.chars().count())
                .unwrap_or(true);
            if better {
                best = Some(*k);
            }
        }
    }
    if best.is_some() {
        return best;
    }
    const ALIASES: &[(&str, &str)] = &[
        ("短发", "发型"),
        ("长发", "发型"),
        ("头巾", "发型"),
        ("长袍", "裤子"),
        ("裤装", "裤子"),
        ("耳坠", "耳饰"),
        ("耳环", "耳饰"),
        ("阳伞", "手持"),
        ("帽", "发饰"),
        ("伞", "手持"),
        ("风铃", "手持"),
        ("平底鞋", "鞋子"),
        ("鞋", "鞋子"),
        ("肩饰", "背饰"),
        ("泳圈", "背饰"),
        ("冲浪板", "手持"),
    ];
    let mut hit: Option<&'static str> = None;
    let mut hit_len = 0usize;
    for (suffix, kind) in ALIASES {
        if name.ends_with(suffix) && suffix.chars().count() > hit_len {
            hit = Some(*kind);
            hit_len = suffix.chars().count();
        }
    }
    hit
}

pub(crate) fn parse_smw_event_rows(val: &serde_json::Value) -> Vec<(String, String, String, String)> {
    let Some(results) = val.pointer("/query/results").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (title, row) in results {
        let printouts = row.get("printouts").cloned().unwrap_or(serde_json::json!({}));
        let start_raw = smw_first_text(&printouts, "开启时间");
        let end_raw = smw_first_text(&printouts, "结束时间");
        let start_d = parse_ymd_prefix(&start_raw);
        let end_d = parse_ymd_prefix(&end_raw);
        let url = row
            .get("fullurl")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        rows.push((
            title.to_string(),
            url,
            start_d.map(ymd).unwrap_or_else(|| start_raw.chars().take(10).collect()),
            end_d.map(ymd).unwrap_or_else(|| end_raw.chars().take(10).collect()),
        ));
    }
    rows
}

pub(crate) fn pick_latest_event_rows(mut rows: Vec<(String, String, String, String)>) -> Vec<(String, String, String, String)> {
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for row in rows {
        let family = event_family_of(&row.0);
        if family.is_empty() || !seen.insert(family) {
            continue;
        }
        out.push(row);
    }
    out
}

pub(crate) fn parse_festival_info(wt: &str) -> (String, String, String) {
    let inner = match find_template_inners(wt, "节日信息").into_iter().next() {
        Some(v) => v,
        None => return (String::new(), String::new(), String::new()),
    };
    let fields = parse_template_fields_nested(&inner);
    let start = fields.get("开启时间").cloned().unwrap_or_default();
    let end = fields.get("结束时间").cloned().unwrap_or_default();
    let name = fields.get("节日名称").cloned().unwrap_or_default();
    (start, end, name)
}

pub(crate) fn parse_page_wear_sheets(wt: &str) -> Vec<String> {
    let mut out = Vec::new();
    let marker = "兑换图.png";
    let mut from = 0;
    while let Some(rel) = wt[from..].find(marker) {
        let end = from + rel;
        let prefix = &wt[..end];
        let start = prefix
            .rfind(|c: char| c == ':' || c == '/' || c == '[' || c == '|' || c == '=' || c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let name = format!("{}{}", prefix[start..].trim(), marker);
        from = end + marker.len();
        if !name.ends_with(marker) || out.iter().any(|s| s == &name) {
            continue;
        }
        out.push(name);
    }
    out
}

pub(crate) fn exhibit_wear_url(name: &str, kind: &str) -> String {
    let file_kind = cosmetic_file_kind(kind);
    if file_kind.is_empty() {
        return String::new();
    }
    if name.ends_with(&file_kind) {
        wiki_file_url(&format!("{}展示图.png", name))
    } else {
        cosmetic_wear_url(&file_kind, name)
    }
}

pub(crate) fn parse_exhibit_from_wear_files(wt: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let marker = "展示图.png";
    let mut from = 0;
    while let Some(rel) = wt[from..].find(marker) {
        let end = from + rel;
        let prefix = &wt[..end];
        let start = prefix
            .rfind(|c: char| c == ':' || c == '/' || c == '[' || c == '|' || c == '=' || c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        let stem = prefix[start..].trim();
        from = end + marker.len();
        if stem.is_empty() {
            continue;
        }
        let Some(kind) = infer_cosmetic_kind(stem) else {
            continue;
        };
        let pair = (stem.to_string(), kind.to_string());
        if !out.iter().any(|x| x == &pair) {
            out.push(pair);
        }
    }
    out
}

pub(crate) fn parse_exhibit_from_headings(wt: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in wt.lines() {
        let t = line.trim();
        if !t.starts_with("==") || !t.ends_with("==") {
            continue;
        }
        let name = t.trim_matches('=').trim();
        let name = name.strip_suffix("展示").unwrap_or(name).trim();
        if name.is_empty()
            || name.contains("物品兑换")
            || name.contains("礼包")
            || name.contains("活动")
            || name.contains("介绍")
            || name.contains("代币")
            || name.contains("任务")
            || name.contains("魔法")
        {
            continue;
        }
        let Some(kind) = infer_cosmetic_kind(name) else {
            continue;
        };
        let pair = (name.to_string(), kind.to_string());
        if !out.iter().any(|x| x == &pair) {
            out.push(pair);
        }
    }
    out
}

pub(crate) fn parse_event_exhibits(wt: &str) -> Vec<(String, String)> {
    let mut out = parse_exhibit_items(wt);
    for pair in parse_exhibit_from_wear_files(wt)
        .into_iter()
        .chain(parse_exhibit_from_headings(wt))
    {
        if !out.iter().any(|x| x == &pair) {
            out.push(pair);
        }
    }
    out
}

pub(crate) fn event_item_id(family: &str, name: &str, kind: &str, items: &[SkySeasonWishItem]) -> String {
    let n = items
        .iter()
        .filter(|i| i.spirit == family && i.kind == kind && i.name == name)
        .count();
    if n == 0 {
        format!("{}|{}|{}", family, name, kind)
    } else {
        format!("{}|{}|{}|{}", family, name, kind, n)
    }
}



pub(crate) fn parse_event_packs(packs: &serde_json::Value, family: &str, items: &mut Vec<SkySeasonWishItem>) {
    let Some(results) = packs.pointer("/query/results").and_then(|v| v.as_object()) else {
        return;
    };
    for (title, row) in results {
        let printouts = row.get("printouts").cloned().unwrap_or(serde_json::json!({}));
        let goods = smw_text_list(&printouts, "物品");
        let extra = smw_text_list(&printouts, "额外物品");
        let price = smw_first_text(&printouts, "价格");
        let stem = title.replace("礼包", "");
        let kinds: Vec<String> = goods
            .iter()
            .chain(extra.iter())
            .filter(|k| !season_skip_kind(k) && !is_furniture_kind(k))
            .cloned()
            .collect();
        let kind = kinds.first().cloned().unwrap_or_else(|| goods.first().cloned().unwrap_or_default());
        if kind.is_empty() && stem.is_empty() {
            continue;
        }
        let file_kind = cosmetic_file_kind(&kind);
        let owner = if stem.is_empty() { title.clone() } else { stem };
        if items.iter().any(|i| i.slot == "商城" && i.name == *title) {
            continue;
        }
        items.push(SkySeasonWishItem {
            id: event_item_id(family, title, "商城", items),
            name: title.clone(),
            spirit: family.to_string(),
            kind: if kinds.is_empty() { kind } else { kinds.join("、") },
            slot: "商城".into(),
            cost_desc: if price.is_empty() {
                String::new()
            } else {
                format!("{}元", price)
            },
            candles: 0,
            hearts: 0,
            ascended: 0,
            icon: if file_kind.is_empty() {
                String::new()
            } else {
                cosmetic_icon_url(&file_kind, &owner)
            },
            wear: String::new(),
        });
    }
}

/// 活动玩法只取正文栏目；兑换树、展示图和注释中的往年玩法不混入攻略。
pub(crate) fn parse_event_gameplay(wt: &str) -> String {
    let mut source = wt.to_string();
    for (open, close) in [("<!--", "-->"), ("<gallery", "</gallery>"), ("{{", "}}")] {
        while let Some(start) = source.find(open) {
            let Some(end) = source[start + open.len()..].find(close) else {
                source.truncate(start);
                break;
            };
            source.replace_range(start..start + open.len() + end + close.len(), "");
        }
    }
    let mut paragraphs = Vec::new();
    let mut selected = false;
    for line in source.lines() {
        let line = line.trim();
        if line.starts_with("==") && line.ends_with("==") {
            let heading = line.trim_matches('=').trim();
            if !line.starts_with("===") {
                selected = ["代币", "任务", "玩法", "攻略", "限定地图", "活动地图"]
                    .iter().any(|word| heading.contains(word));
            }
            continue;
        }
        if !selected { continue; }
        let mut text = line.to_string();
        while let Some(start) = text.find("[[") {
            let Some(end) = text[start + 2..].find("]]") else { break };
            let end = start + 2 + end;
            let link = &text[start + 2..end];
            let label = if link.starts_with("文件:") || link.starts_with("File:") || link.starts_with("分类:") {
                ""
            } else {
                link.rsplit('|').next().unwrap_or("")
            }.to_string();
            text.replace_range(start..end + 2, &label);
        }
        let html = scraper::Html::parse_fragment(&text);
        let text = html.root_element().text().collect::<String>().replace("'''", "").replace("''", "");
        let text = text.trim().trim_start_matches(['*', '#', ':']).trim();
        if !text.is_empty() { paragraphs.push(text.to_string()); }
    }
    paragraphs.join("\n\n")
}

pub(crate) fn parse_event_page(
    page: &str,
    url: &str,
    wt: &str,
    start: &str,
    end: &str,
    today: chrono::NaiveDate,
    packs: &serde_json::Value,
) -> SkySeasonCatalogItem {
    let family = event_family_of(page);
    let (info_start, info_end, info_name) = parse_festival_info(wt);
    let mut start = start.to_string();
    let mut end = end.to_string();
    if start.is_empty() {
        if let Some(s) = parse_ymd_prefix(&info_start) {
            start = ymd(s);
        }
    }
    if end.is_empty() {
        if let Some(e) = parse_ymd_prefix(&info_end) {
            end = ymd(e);
        }
    }
    let title = if family.is_empty() {
        info_name
    } else {
        family.clone()
    };
    let mut items = Vec::new();
    for (name, kind) in parse_event_exhibits(wt) {
        if season_skip_kind(&kind) {
            continue;
        }
        let file_kind = cosmetic_file_kind(&kind);
        items.push(SkySeasonWishItem {
            id: event_item_id(&title, &name, &kind, &items),
            name: name.clone(),
            spirit: title.clone(),
            kind: kind.clone(),
            slot: "兑换".into(),
            cost_desc: String::new(),
            candles: 0,
            hearts: 0,
            ascended: 0,
            icon: if file_kind.is_empty() {
                String::new()
            } else {
                cosmetic_icon_url(&file_kind, &name)
            },
            wear: exhibit_wear_url(&name, &kind),
        });
    }
    parse_event_packs(packs, &title, &mut items);
    let start_d = parse_ymd_prefix(&start);
    let end_d = parse_ymd_prefix(&end);
    let loc = match (start_d.map(ymd), end_d.map(ymd)) {
        (Some(s), Some(e)) => format!("{} ~ {}", s, e),
        (Some(s), None) => format!("{} 起", s),
        _ => String::new(),
    };
    SkySeasonCatalogItem {
        id: format!("wiki-event-{}", title),
        title,
        slogan: page.to_string(),
        realm: String::new(),
        loc,
        start: start_d.map(ymd).unwrap_or(start),
        end: end_d.map(ymd).unwrap_or(end),
        status: event_window_status(start_d, end_d, today),
        url: if url.is_empty() {
            format!("https://wiki.biligame.com/sky/{}", wiki_encode(page))
        } else {
            url.to_string()
        },
        icon: wiki_file_url(&format!("UI-{}.png", event_family_of(page))),
        quests: parse_event_gameplay(wt),
        sheet: parse_page_wear_sheets(wt).into_iter().next().unwrap_or_default(),
        tree_candles: Default::default(),
        items,
    }
}

pub(crate) fn needs_catalog_looks(item: &SkyCalendarItem) -> bool {
    item.kind == "event" && item.region != "intl"
        && !calendar_event_family(&item.title).is_empty()
        && !item.previews.iter().any(|p| p.slot == "兑换" && (!p.wear.is_empty() || !p.icon.is_empty()))
}

/// 情报卡没有国服 wiki 当年页时，用节日目录里同一节日的图标和兑换实穿。日期仍以公告为准。
pub(crate) fn attach_catalog_looks(item: &mut SkyCalendarItem, catalog: &[SkySeasonCatalogItem]) {
    if item.kind != "event" || item.region == "intl" {
        return;
    }
    let family = calendar_event_family(&item.title);
    if family.is_empty() {
        return;
    }
    let Some(event) = catalog.iter().find(|e| calendar_event_family(&e.title) == family) else {
        return;
    };
    if item.icon.is_empty() && !event.icon.is_empty() {
        item.icon = event.icon.clone();
    }
    let catalog_has_looks = event.items.iter().any(|i| {
        (i.slot == "兑换" || i.slot == "商城") && (!i.wear.is_empty() || !i.icon.is_empty())
    });
    // 节日目录已经有兑换列表。活动海报和价格总图都不再贴到这张卡上。
    if catalog_has_looks {
        item.previews.retain(|p| p.slot != "资讯");
    }
    if !needs_catalog_looks(item) {
        item.previews.retain(|p| !(p.slot == "兑换" && p.name == "兑换图"));
        return;
    }
    let looks = event_catalog_as_previews(event);
    for p in looks.previews {
        if p.slot != "兑换" && p.slot != "商城" {
            continue;
        }
        if item.previews.iter().any(|x| x.slot == p.slot && x.name == p.name) {
            continue;
        }
        item.previews.push(p);
    }
}

pub(crate) fn event_catalog_as_previews(event: &SkySeasonCatalogItem) -> SkyCalendarItem {
    SkyCalendarItem {
        id: event.id.clone(),
        kind: "event".into(),
        region: "cn".into(),
        title: event.title.clone(),
        names: vec![],
        season: String::new(),
        start: event.start.clone(),
        end: event.end.clone(),
        status: event.status.clone(),
        note: String::new(),
        url: event.url.clone(),
        cosmetics: String::new(),
        icon: event.icon.clone(),
        previews: event
            .items
            .iter()
            .map(|i| {
                preview(
                    &i.slot,
                    i.name.clone(),
                    i.kind.clone(),
                    i.icon.clone(),
                    i.wear.clone(),
                )
            })
            .collect(),
    }
}

pub(crate) fn apply_event_wear_from_previews(event: &mut SkySeasonCatalogItem, item: &SkyCalendarItem) {
    for src in &item.previews {
        if src.wear.is_empty() {
            continue;
        }
        if let Some(dst) = event
            .items
            .iter_mut()
            .find(|i| i.name == src.name && i.slot == src.slot)
        {
            dst.wear = src.wear.clone();
        }
    }
}

pub(crate) fn apply_event_item_urls(
    events: &mut [SkySeasonCatalogItem],
    icon_map: &std::collections::HashMap<String, String>,
    wear_map: &std::collections::HashMap<String, String>,
) {
    for event in events.iter_mut() {
        event.icon = resolve_wiki_file_url(&event.icon, icon_map);
        for item in event.items.iter_mut() {
            item.icon = resolve_wiki_file_url(&item.icon, icon_map);
            if item.wear.is_empty() {
                continue;
            }
            if let Some(name) = wear_filename_from_url(&item.wear) {
                item.wear = wear_map.get(&name).cloned().unwrap_or_default();
            }
        }
    }
}

pub(crate) fn event_status_rank(status: &str) -> u8 {
    match status {
        "live" => 0,
        "upcoming" => 1,
        _ => 2,
    }
}

pub(crate) fn pack_owner_key(owner: &str) -> String {
    let title = canonical_event_title(&event_family_of(owner));
    let family = calendar_event_family(&title);
    if family.chars().count() >= 2 {
        family
    } else {
        title
    }
}

pub(crate) fn packs_for_event_title(
    groups: &std::collections::HashMap<String, serde_json::Value>,
    title: &str,
) -> serde_json::Value {
    let key = pack_owner_key(title);
    if let Some(v) = groups.get(&key) {
        return v.clone();
    }
    let canon = canonical_event_title(&event_family_of(title));
    groups
        .get(&canon)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}))
}

pub(crate) fn group_event_packs(val: &serde_json::Value) -> std::collections::HashMap<String, serde_json::Value> {
    let mut grouped: std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>> =
        std::collections::HashMap::new();
    let Some(results) = val.pointer("/query/results").and_then(|v| v.as_object()) else {
        return std::collections::HashMap::new();
    };
    for (title, row) in results {
        let printouts = row.get("printouts").cloned().unwrap_or(serde_json::json!({}));
        let owner = smw_first_text(&printouts, "所属");
        if owner.is_empty() {
            continue;
        }
        grouped
            .entry(pack_owner_key(&owner))
            .or_default()
            .insert(title.clone(), row.clone());
    }
    grouped
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                serde_json::json!({ "query": { "results": serde_json::Value::Object(v) } }),
            )
        })
        .collect()
}

pub(crate) async fn fandom_days_item_catalog() -> Vec<FandomDaysItem> {
    let Ok(client) = fandom_http_client_timeout(60) else {
        return Vec::new();
    };
    let Some(wt) = fandom_wikitext(&client, "Module:Days Item/data").await else {
        return Vec::new();
    };
    parse_fandom_days_item_data(&wt)
}

pub(crate) async fn fandom_netease_wear_files(client: &reqwest::Client) -> Vec<String> {
    let titles: Vec<String> = fandom_category_pages(client, "Category:NetEase")
        .await
        .into_iter()
        .filter(|t| {
            let l = t.to_ascii_lowercase();
            l.contains("qixi") || l.contains("dragon") || l.contains("cosmetics")
        })
        .collect();
    if titles.is_empty() {
        return Vec::new();
    }
    let images = fandom_titles_images(client, &titles).await;
    let mut files = Vec::new();
    for name in images.into_values().flatten() {
        if is_fandom_wear_file(&name) && !files.iter().any(|s| s == &name) {
            files.push(name);
        }
    }
    files
}

// 档期和节日页启动时同时要目录。持锁拉取让后到者等前一次结果，避免两遍 Fandom 请求互相限流。
static EVENT_CATALOG: tokio::sync::Mutex<Option<(std::time::Instant, SkyEventCatalog)>> =
    tokio::sync::Mutex::const_new(None);
const EVENT_CATALOG_TTL: std::time::Duration = std::time::Duration::from_secs(300);

#[tauri::command]
pub async fn fetch_sky_events() -> Result<SkyEventCatalog, String> {
    let mut slot = EVENT_CATALOG.lock().await;
    if let Some((at, catalog)) = slot.as_ref() {
        if at.elapsed() < EVENT_CATALOG_TTL {
            return Ok(catalog.clone());
        }
    }
    let catalog = build_sky_events().await?;
    *slot = Some((std::time::Instant::now(), catalog.clone()));
    Ok(catalog)
}

async fn build_sky_events() -> Result<SkyEventCatalog, String> {
    let client = sky_http_client()?;
    let today = super::beijing_today();
    let (ask_val, pack_val) = tokio::join!(
        wiki_ask(
            &client,
            "[[分类:节日活动]]|?开启时间|?结束时间|sort=开启时间|order=desc|limit=100",
        ),
        wiki_ask(
            &client,
            "[[分类:礼包]]|?所属|?名称|?物品|?价格|sort=首次上架时间|order=desc|limit=500",
        ),
    );
    let mut rows = ask_val
        .ok()
        .as_ref()
        .map(parse_smw_event_rows)
        .unwrap_or_default();
    rows = pick_latest_event_rows(rows);
    if rows.is_empty() {
        return Err("未能拉到节日目录".into());
    }
    let pack_val = pack_val.map_err(|e| format!("节日礼包目录读取失败：{e}"))?;
    let pack_groups = group_event_packs(&pack_val);
    let titles: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    let pages = wiki_wikitext_chunked(&client, &titles).await;
    let mut wt_map = std::collections::HashMap::new();
    for (title, wt) in pages {
        wt_map.insert(title, wt);
    }
    let missing: Vec<_> = titles.iter()
        .filter(|title| wt_map.get(*title).is_none_or(|wt| wt.trim().is_empty()))
        .map(String::as_str).collect();
    if !missing.is_empty() {
        return Err(format!("节日页面读取失败，保留上次目录：{}", missing.join("、")));
    }
    let mut events = Vec::new();
    for (page, url, start, end) in rows {
        let wt = wt_map.get(&page).cloned().unwrap_or_default();
        let packs = packs_for_event_title(&pack_groups, &page);
        events.push(parse_event_page(
            &page, &url, &wt, &start, &end, today, &packs,
        ));
    }
    events.sort_by(|a, b| {
        event_status_rank(&a.status)
            .cmp(&event_status_rank(&b.status))
            .then_with(|| b.start.cmp(&a.start))
    });
    resolve_event_catalog_media(&client, &mut events).await;
    Ok(SkyEventCatalog {
        as_of: ymd(today),
        events,
    })
}

pub(crate) async fn resolve_event_catalog_media(client: &reqwest::Client, events: &mut [SkySeasonCatalogItem]) {
    let mut files = Vec::new();
    let mut wear_files = Vec::new();
    for event in events.iter() {
        if let Some(name) = wiki_filename_from_url(&event.icon) {
            files.push(name);
        }
        if !event.sheet.is_empty() {
            files.push(event.sheet.clone());
        }
        for item in &event.items {
            if let Some(name) = wiki_filename_from_url(&item.icon) {
                files.push(name);
            }
            if let Some(name) = wear_filename_from_url(&item.wear) {
                wear_files.push(name);
            }
        }
    }
    let (icon_map, wear_map, days_catalog, netease_files) = tokio::join!(
        wiki_file_urls(&client, &files),
        wiki_file_urls(&client, &wear_files),
        fandom_days_item_catalog(),
        async {
            let Ok(fc) = fandom_http_client_timeout(40) else {
                return Vec::new();
            };
            fandom_netease_wear_files(&fc).await
        },
    );
    apply_event_item_urls(events, &icon_map, &wear_map);
    for event in events.iter_mut() {
        if event.sheet.is_empty() {
            continue;
        }
        event.sheet = icon_map.get(&event.sheet).cloned().unwrap_or_default();
    }
    let mut days_sets = FandomFileSets::empty();
    absorb_days_item_catalog(&mut days_sets, days_catalog);
    days_sets.event_page.extend(netease_files);
    let previews: Vec<_> = events.iter().map(event_catalog_as_previews).collect();
    resolve_event_icon_wear(&previews, &mut days_sets).await;
    let mut chosen_files: Vec<String> = Vec::new();
    let mut chosen_by_title: std::collections::HashMap<String, Vec<Option<String>>> =
        std::collections::HashMap::new();
    if !days_sets.event_page.is_empty() {
        for event in events.iter() {
            let preview = event_catalog_as_previews(event);
            let chosen = choose_fandom_wear_files(&preview, &days_sets);
            for file in chosen.iter().flatten() {
                if !chosen_files.iter().any(|s| s == file) {
                    chosen_files.push(file.clone());
                }
            }
            if chosen.iter().any(|f| f.is_some()) {
                chosen_by_title.insert(event.title.clone(), chosen);
            }
        }
    }
    let fandom_map = if chosen_files.is_empty() {
        std::collections::HashMap::new()
    } else {
        fandom_file_urls(&chosen_files).await
    };
    for event in events.iter_mut() {
        let Some(chosen) = chosen_by_title.get(&event.title) else {
            continue;
        };
        let mut preview = event_catalog_as_previews(event);
        if preview.previews.len() != chosen.len() {
            continue;
        }
        for (p, file) in preview.previews.iter_mut().zip(chosen.iter()) {
            if let Some(file) = file {
                if let Some(url) = fandom_lookup_url(&fandom_map, file) {
                    p.wear = url;
                }
            }
        }
        apply_event_wear_from_previews(event, &preview);
    }
}
