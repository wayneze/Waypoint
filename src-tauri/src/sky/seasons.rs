// 奇妙之旅季节目录。
use super::{SkySeasonCatalog, SkySeasonCatalogItem, SkySeasonWishItem};
use super::wiki::*;
use super::fandom::{fandom_http_client, fandom_wikitext_many, parse_fandom_spirit_wear, fandom_file_urls, fandom_lookup_url};


fn season_item_owner(season: &str, spirit: &str, kind: &str) -> String {
    if kind.starts_with("毕业") { format!("{}毕业礼", season) } else { spirit.to_string() }
}

pub(crate) fn season_item_icon_url(season: &str, spirit: &str, kind: &str) -> String {
    cosmetic_icon_url(&cosmetic_file_kind(kind), &season_item_owner(season, spirit, kind))
}

pub(crate) fn season_item_wear_url(season: &str, spirit: &str, kind: &str) -> String {
    cosmetic_wear_url(&cosmetic_file_kind(kind), &season_item_owner(season, spirit, kind))
}

pub(crate) fn split_wiki_list(raw: &str) -> Vec<String> {
    raw.split('、')
        .map(|s| strip_wiki_comment(s.trim()))
        .filter(|s| !s.is_empty() && !season_skip_kind(s))
        .collect()
}

pub(crate) fn season_item_id(title: &str, spirit: &str, slot: &str, kind: &str, items: &[SkySeasonWishItem]) -> String {
    let n = items
        .iter()
        .filter(|i| i.spirit == spirit && i.slot == slot && i.kind == kind)
        .count();
    format!("{}|{}|{}|{}|{}", title, spirit, slot, kind, n)
}

pub(crate) fn parse_season_page(title: &str, url: &str, wt: &str, start: &str, end: &str, today: chrono::NaiveDate) -> SkySeasonCatalogItem {
    let mut slogan = String::new();
    let mut quests = String::new();
    let mut start = start.to_string();
    let mut end = end.to_string();
    if let Some(inner) = find_template_inners(wt, "季节顶部").into_iter().next() {
        let fields = parse_template_fields_nested(&inner);
        slogan = fields.get("宣传语").cloned().unwrap_or_default();
        if let Some(n) = fields.get("有无季节任务") {
            if !n.is_empty() && n != "无" {
                quests = format!("季节任务 {}", n);
            }
        }
    }
    if let Some(inner) = find_template_inners(wt, "季节简介").into_iter().next() {
        let fields = parse_template_fields_nested(&inner);
        if start.is_empty() {
            if let Some(s) = parse_ymd_prefix(fields.get("开启时间").map(String::as_str).unwrap_or("")) {
                start = ymd(s);
            }
        }
        if end.is_empty() {
            if let Some(e) = parse_ymd_prefix(fields.get("结束时间").map(String::as_str).unwrap_or("")) {
                end = ymd(e);
            }
        }
        if quests.is_empty() {
            if let Some(n) = fields.get("先祖数量") {
                if !n.is_empty() {
                    quests = format!("先祖 {}", n);
                }
            }
        }
    }
    let trees: std::collections::HashMap<String, ExchangeTree> = parse_exchange_trees(wt)
        .into_iter()
        .map(|t| (t.spirit.clone(), t))
        .collect();
    let mut tree_candles = std::collections::HashMap::new();
    let mut realm = String::new();
    let mut loc = String::new();
    let mut items = Vec::new();
    for inner in find_template_inners(wt, "季节先祖板块") {
        let fields = parse_template_fields_nested(&inner);
        let spirit = fields.get("先祖中文名").cloned().unwrap_or_default();
        if spirit.is_empty() {
            continue;
        }
        let typ = fields.get("先祖类型").cloned().unwrap_or_default();
        let map = fields.get("所在地图").cloned().unwrap_or_default();
        let pos = fields.get("先祖位置").cloned().unwrap_or_default();
        if typ.contains("向导") {
            if !map.is_empty() {
                realm = map.clone();
            }
            loc = if pos.is_empty() { map } else { pos };
        } else {
            if realm.is_empty() && !map.is_empty() {
                realm = map;
            }
            if loc.is_empty() && !pos.is_empty() {
                loc = pos;
            }
        }
        if let Some(tree) = trees.get(&spirit).filter(|t| !t.items.is_empty()) {
            if tree.candles_total > 0 {
                tree_candles.insert(spirit.clone(), tree.candles_total);
            }
            for it in &tree.items {
                items.push(SkySeasonWishItem {
                    id: season_item_id(title, &spirit, &it.slot, &it.kind, &items),
                    name: format!("{} · {}", spirit, it.kind),
                    spirit: spirit.clone(),
                    kind: it.kind.clone(),
                    slot: it.slot.clone(),
                    cost_desc: it.cost_desc.clone(),
                    candles: it.candles,
                    hearts: it.hearts,
                    ascended: it.ascended,
                    icon: season_item_icon_url(title, &spirit, &it.kind),
                    wear: season_item_wear_url(title, &spirit, &it.kind),
                });
            }
            continue;
        }
        let goods = split_wiki_list(fields.get("物品").map(String::as_str).unwrap_or(""));
        let pass = split_wiki_list(fields.get("季卡物品").map(String::as_str).unwrap_or(""));
        for (slot, list) in [("兑换", goods), ("季卡", pass)] {
            for kind in list {
                if kind == "动作" || kind.starts_with("动作") {
                    continue;
                }
                items.push(SkySeasonWishItem {
                    id: season_item_id(title, &spirit, slot, &kind, &items),
                    name: format!("{} · {}", spirit, kind),
                    spirit: spirit.clone(),
                    kind: kind.clone(),
                    slot: slot.to_string(),
                    cost_desc: String::new(),
                    candles: 0,
                    hearts: 0,
                    ascended: 0,
                    icon: season_item_icon_url(title, &spirit, &kind),
                    wear: season_item_wear_url(title, &spirit, &kind),
                });
            }
        }
    }
    let start_d = parse_ymd_prefix(&start);
    let end_d = parse_ymd_prefix(&end);
    SkySeasonCatalogItem {
        id: format!("wiki-{}", title),
        title: title.to_string(),
        slogan,
        realm,
        loc,
        start: start_d.map(ymd).unwrap_or(start),
        end: end_d.map(ymd).unwrap_or(end),
        status: calendar_status(start_d, end_d, today),
        url: if url.is_empty() {
            format!("https://wiki.biligame.com/sky/{}", wiki_encode(title))
        } else {
            url.to_string()
        },
        icon: season_logo_url(title),
        quests,
        sheet: String::new(),
        items,
        tree_candles,
    }
}

#[tauri::command]
pub async fn fetch_sky_seasons() -> Result<SkySeasonCatalog, String> {
    let client = sky_http_client()?;
    let today = chrono::Local::now().date_naive();
    let ask_val = wiki_ask(
        &client,
        "[[分类:奇妙之旅]]|?开启时间|?结束时间|sort=开启时间|order=desc|limit=50",
    )
    .await
    .ok();
    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    if let Some(val) = ask_val.as_ref() {
        for it in parse_smw_pages(val, "season", today) {
            let title = it
                .id
                .strip_prefix("season-")
                .unwrap_or(&it.title)
                .to_string();
            rows.push((title, it.url, it.start, it.end));
        }
    }
    if rows.is_empty() {
        if let Ok(wt) = wiki_wikitext(&client, "奇妙之旅").await {
            for it in parse_season_wikitext(&wt, today) {
                rows.push((it.title.clone(), it.url, it.start, it.end));
            }
        }
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2));
    let titles: Vec<String> = rows.iter().map(|r| r.0.clone()).collect();
    // 目录接口一次返回全部原文，避免逐页 parse 请求部分失败后静默丢页。
    let response: serde_json::Value = client
        .post("https://wiki.biligame.com/sky/api.php")
        .form(&[
            ("action", "query"), ("titles", titles.join("|").as_str()),
            ("prop", "revisions"), ("rvprop", "content"), ("rvslots", "main"),
            ("format", "json"), ("formatversion", "2"),
        ])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    if let Some(error) = response.get("error") {
        return Err(format!("季节目录读取失败：{}", error["info"].as_str().unwrap_or("来源接口出错")));
    }
    let mut wt_map = std::collections::HashMap::new();
    if let Some(pages) = response.pointer("/query/pages").and_then(|v| v.as_array()) {
        for page in pages {
            if let (Some(title), Some(wt)) = (
                page["title"].as_str(),
                page.pointer("/revisions/0/slots/main/content").and_then(|v| v.as_str()),
            ) {
                wt_map.insert(title.to_string(), wt.to_string());
            }
        }
    }
    let missing: Vec<_> = titles.iter().filter(|title| wt_map.get(*title).is_none_or(|wt| wt.trim().is_empty())).cloned().collect();
    if !missing.is_empty() {
        return Err(format!("季节页面未完整载入（{}），已保留上次目录，请重试", missing.join("、")));
    }
    let mut seasons = Vec::new();
    for (title, url, start, end) in rows {
        let wt = &wt_map[&title];
        seasons.push(parse_season_page(&title, &url, wt, &start, &end, today));
    }
    if seasons.is_empty() {
        return Err("未能拉到季节目录".into());
    }
    let mut files = Vec::new();
    for season in &seasons {
        if let Some(name) = wiki_filename_from_url(&season.icon) {
            files.push(name);
        }
        for item in &season.items {
            if let Some(name) = wiki_filename_from_url(&item.icon) {
                files.push(name);
            }
        }
    }
    files.extend(seasons.iter().flat_map(|s| &s.items).filter_map(|i| wear_filename_from_url(&i.wear)));
    let spirit_en = wt_map.values().flat_map(|wt| parse_spirit_en_names(wt)).collect();
    let (wiki_map, fandom_wear) = tokio::join!(
        wiki_file_urls(&client, &files),
        async {
            let Ok(fc) = fandom_http_client() else { return Default::default() };
            let pages = fandom_wikitext_many(&fc, &["Module:Spirits/data".into(), "Module:Seasons/data".into()]).await;
            parse_fandom_spirit_wear(&pages.values().cloned().collect::<Vec<_>>().join("\n"), &spirit_en)
        },
    );
    apply_season_media_urls(&mut seasons, &wiki_map);
    let candidates = season_fandom_candidates(&seasons, &fandom_wear);
    let fandom_map = fandom_file_urls(&candidates.values().cloned().collect::<Vec<_>>()).await;
    for item in seasons.iter_mut().flat_map(|s| &mut s.items) {
        if let Some(file) = candidates.get(&item.id) {
            item.wear = fandom_lookup_url(&fandom_map, file).unwrap_or_default();
        }
    }
    Ok(SkySeasonCatalog {
        as_of: ymd(today),
        seasons,
    })
}

pub(crate) fn apply_season_media_urls(
    seasons: &mut [SkySeasonCatalogItem],
    map: &std::collections::HashMap<String, String>,
) {
    for season in seasons.iter_mut() {
        season.icon = resolve_wiki_file_url(&season.icon, map);
        for item in season.items.iter_mut() {
            item.icon = resolve_wiki_file_url(&item.icon, map);
            item.wear = wear_filename_from_url(&item.wear).and_then(|f| map.get(&f)).cloned().unwrap_or_default();
        }
    }
}

/// 只接受同一先祖、同一种类的唯一对应；编号家具不借用同组其他物品的图片。
pub(crate) fn season_fandom_candidates(
    seasons: &[SkySeasonCatalogItem],
    media: &std::collections::HashMap<String, std::collections::HashMap<String, String>>,
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for season in seasons {
        for item in &season.items {
            if !item.wear.is_empty() { continue; }
            if season.items.iter().filter(|i| i.spirit == item.spirit && i.kind == item.kind).count() != 1 { continue; }
            let Some(owner_media) = media.get(&item.spirit) else { continue };
            let Some(file) = owner_media.get(&item.kind) else { continue };
            // 两站的道具分类不同；同一张 prop 图同时落到手持、大小家具时没有唯一身份。
            if season.items.iter().any(|other| other.id != item.id
                && other.spirit == item.spirit && owner_media.get(&other.kind) == Some(file)) { continue; }
            out.insert(item.id.clone(), file.clone());
        }
    }
    out
}
