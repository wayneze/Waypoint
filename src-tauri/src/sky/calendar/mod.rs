// 光遇档期：国服/国际复刻、季节窗口、节日资讯与 inspect。
use super::{SkyCalendarFeed, SkyCalendarItem, SkyEventEdition};
use super::wiki::*;
use super::fandom::*;
use super::events::{
    attach_catalog_looks, fetch_sky_events, group_event_packs, packs_for_event_title, needs_catalog_looks,
    parse_event_exhibits,
};

mod reprints;
mod news;
mod merge;
mod articles;
mod extras;

pub(crate) use reprints::*;
pub(crate) use news::*;
pub(crate) use merge::*;
pub(crate) use articles::*;
pub(crate) use extras::*;

// 光遇档期：国服/国际复刻表 + 季节/节日。国际未公布的窗口按上一档 +14 天推断，不编名字。
#[tauri::command]
pub async fn fetch_sky_calendar() -> Result<SkyCalendarFeed, String> {
    let client = sky_http_client()?;
    let today = chrono::Local::now().date_naive();
    let today_s = ymd(today);
    let cn_url = "https://wiki.biligame.com/sky/%E6%97%85%E8%A1%8C%E5%85%88%E7%A5%96%E5%9B%9E%E5%BD%92%E8%AE%B0%E5%BD%95";
    let intl_url = "https://wiki.biligame.com/sky/%E6%97%85%E8%A1%8C%E5%85%88%E7%A5%96%E5%9B%9E%E5%BD%92%E8%AE%B0%E5%BD%95_%E5%9B%BD%E9%99%85%E6%9C%8D";

    let event_q = format!(
        "[[分类:节日活动]][[开启时间::>{}-01-01]]|?开启时间|?结束时间|?Category|sort=开启时间|order=desc|limit=300",
        date_year(today) - 4
    );
    let (cn_wt, intl_wt, season_ask, event_ask, extra_events) = tokio::join!(
        wiki_wikitext(&client, "旅行先祖回归记录"),
        wiki_wikitext(&client, "旅行先祖回归记录 国际服"),
        wiki_ask(
            &client,
            "[[分类:奇妙之旅]]|?开启时间|?结束时间|sort=开启时间|order=desc|limit=4",
        ),
        wiki_ask(&client, &event_q),
        fetch_calendar_extra(today),
    );
    let cn_wt = cn_wt.unwrap_or_default();
    let intl_wt = intl_wt.unwrap_or_default();
    let season_ask = season_ask.ok();
    let event_ask = event_ask.ok();
    let (extra_events, weibo_ids, weibo_timeline, mut event_history) = extra_events;
    if let Some(value) = &event_ask {
        event_history.extend(parse_smw_event_history(value));
    }

    let cn_rows = extract_reprint_templates(&cn_wt);
    let intl_rows = extract_reprint_templates(&intl_wt);
    let mut reprint_en = reprint_spirit_names(&cn_rows);
    reprint_en.extend(reprint_spirit_names(&intl_rows));
    let cn_items = grouped_reprints(cn_rows, "cn", cn_url, today);
    let intl_items = grouped_reprints(intl_rows, "intl", intl_url, today);

    let mut seasons = season_ask
        .as_ref()
        .map(|v| parse_smw_pages(v, "season", today))
        .unwrap_or_default();
    if seasons.is_empty() {
        if let Ok(wt) = wiki_wikitext(&client, "奇妙之旅").await {
            seasons = parse_season_wikitext(&wt, today);
        }
    }
    let season = pick_current_season(seasons);

    let mut events: Vec<SkyCalendarItem> = event_ask
        .as_ref()
        .map(|v| parse_smw_pages(v, "event", today))
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.status == "live" || e.status == "upcoming")
        .collect();
    merge_calendar_events(&mut events, extra_events);
    // 国服 wiki 没有当年页时会落到往年重定向。往年窗口不能再标成今年进行中。
    events.retain(|e| {
        e.region == "intl" || e.status == "live" || e.status == "upcoming" || e.status == "history"
    });
    promote_ended_cn_to_history(&mut events);
    // 历史标记只用于国服/国际档期对照，不展示物品，也不参与图片查询。
    let (history, mut events): (Vec<_>, Vec<_>) = events.into_iter().partition(|e| e.status == "history");

    let (mut cn_now, mut cn_next) = split_now_next(cn_items.clone());
    let (mut intl_now, mut intl_next) = current_intl_reprints(intl_items, &cn_items, today);
    let visible_spirits: std::collections::HashSet<&str> = cn_now.iter()
        .chain(&cn_next).chain(&intl_now).chain(&intl_next)
        .flat_map(|item| item.names.iter().map(String::as_str))
        .collect();
    reprint_en.retain(|cn, _| visible_spirits.contains(cn.as_str()));

    if cn_now.is_empty() && cn_next.is_empty() {
        let named_cn: std::collections::HashSet<chrono::NaiveDate> = cn_items
            .iter()
            .filter_map(|it| parse_ymd_prefix(&it.start))
            .collect();
        if let Some(last) = last_regular(&cn_items) {
            for (s, e) in infer_regular_windows(last, today, &named_cn, 1) {
                cn_next.push(SkyCalendarItem {
                    id: format!("cn-inferred-{}", ymd(s)),
                    kind: "reprint".into(),
                    region: "cn".into(),
                    title: "旅行先祖（未公布）".into(),
                    names: vec![],
                    season: String::new(),
                    start: ymd(s),
                    end: ymd(e),
                    status: "inferred".into(),
                    note: "国服惯例约每两周窗口，名字尚未进 wiki".into(),
                    url: cn_url.into(),
                    cosmetics: String::new(),
                    icon: String::new(),
                    previews: Vec::new(),
                });
            }
        }
    }

    let season_title = season.as_ref().map(|s| s.title.clone());
    let mut wiki_event_pages: Vec<String> = Vec::new();
    for event in &events {
        for page in calendar_event_wiki_pages(event) {
            if !wiki_event_pages.iter().any(|p| p == &page) {
                wiki_event_pages.push(page);
            }
        }
    }
    let (season_wt, event_pages, pack_val, weibo_id_mblogs) = tokio::join!(
        async {
            if let Some(title) = season_title.as_ref() {
                wiki_wikitext(&client, title).await.ok()
            } else {
                None
            }
        },
        async {
            if wiki_event_pages.is_empty() {
                Vec::new()
            } else {
                wiki_wikitext_chunked(&client, &wiki_event_pages).await
            }
        },
        wiki_ask(
            &client,
            "[[分类:礼包]]|?所属|?名称|?物品|?价格|sort=首次上架时间|order=desc|limit=500",
        ),
        weibo_mblogs_by_ids(&weibo_ids),
    );

    let season_trees = season_wt.as_deref().map(parse_exchange_trees).unwrap_or_default();
    let spirit_en = season_wt.as_deref().map(parse_spirit_en_names).unwrap_or_default();
    let pack_groups = pack_val.ok().as_ref().map(group_event_packs).unwrap_or_default();

    let fill_list = |list: &mut Vec<SkyCalendarItem>| {
        for it in list.iter_mut() {
            fill_reprint_media(it);
        }
    };
    fill_list(&mut cn_now);
    fill_list(&mut cn_next);
    fill_list(&mut intl_now);
    fill_list(&mut intl_next);

    let mut season = season;
    if let Some(season_mut) = season.as_mut() {
        let packs = packs_for_event_title(&pack_groups, &season_mut.title);
        fill_season_media(season_mut, season_trees, &packs);
    }

    let mut event_wt_map = std::collections::HashMap::new();
    for (page, wt) in event_pages {
        event_wt_map.insert(page, wt);
    }
    for event in events.iter_mut() {
        if event.kind != "event" {
            continue;
        }
        let wiki_wt = calendar_event_wikitext(event, &event_wt_map);
        let exhibits = wiki_wt.map(parse_event_exhibits).unwrap_or_default();
        let packs = packs_for_event_title(&pack_groups, &event.title);
        fill_event_media(event, exhibits, &packs);
    }

    let mut wiki_files = Vec::new();
    let mut collect_wiki = |item: &SkyCalendarItem| {
        if let Some(name) = wiki_filename_from_url(&item.icon) {
            wiki_files.push(name);
        }
        for p in &item.previews {
            if let Some(name) = wiki_filename_from_url(&p.icon) {
                wiki_files.push(name);
            }
            if let Some(name) = wear_filename_from_url(&p.wear) {
                wiki_files.push(name);
            }
        }
    };
    for it in cn_now.iter().chain(cn_next.iter()).chain(intl_now.iter()).chain(intl_next.iter()) {
        collect_wiki(it);
    }
    if let Some(s) = season.as_ref() {
        collect_wiki(s);
    }
    for it in events.iter() {
        collect_wiki(it);
    }
    let event_queries: Vec<(String, String)> = {
        let mut q = Vec::new();
        for e in events.iter().filter(|e| e.kind == "event") {
            let year = calendar_event_year(e);
            let hit = fandom_en_for_cn_event(&e.title)
                .map(|s| s.to_string())
                .unwrap_or_else(|| e.title.clone());
            if !q.iter().any(|(t, y)| t == &hit && y == &year) {
                q.push((hit, year));
            }
        }
        q
    };
    let promo_files = collect_fandom_media_files(&events);
    let (wear_map, fandom_pair) = tokio::join!(
        wiki_file_urls(&client, &wiki_files),
        async {
            let sets = fandom_fetch_sets(&spirit_en, &reprint_en, &event_queries).await;
            let mut files = sets.all_files();
            for f in promo_files {
                if !files.iter().any(|s| s == &f) {
                    files.push(f);
                }
            }
            let map = fandom_file_urls(&files).await;
            (sets, map)
        },
    );
    let (mut fandom_sets, fandom_map) = fandom_pair;
    let mut warnings = fandom_sets.warnings.clone();
    if !wiki_files.is_empty() && wear_map.is_empty() {
        warnings.push("B 站 wiki 图片查询失败，图标和实穿图可能缺".to_string());
    }
    if !fandom_sets.all_files().is_empty() && fandom_map.is_empty() {
        warnings.push("Fandom 图片查询失败，部分实穿图缺".to_string());
    }
    for it in cn_now.iter_mut().chain(cn_next.iter_mut()).chain(intl_now.iter_mut()).chain(intl_next.iter_mut()) {
        apply_wear_urls(it, &wear_map);
        apply_wiki_preview_icons(it, &wear_map);
        apply_fandom_wear(it, &fandom_sets, &fandom_map);
    }
    if let Some(s) = season.as_mut() {
        apply_wear_urls(s, &wear_map);
        apply_wiki_preview_icons(s, &wear_map);
        apply_fandom_wear(s, &fandom_sets, &fandom_map);
    }
    for it in events.iter_mut() {
        fill_event_from_fandom(it, &fandom_sets);
        apply_wear_urls(it, &wear_map);
        apply_wiki_preview_icons(it, &wear_map);
    }
    resolve_event_icon_wear(&events, &mut fandom_sets).await;
    for it in events.iter_mut() {
        apply_fandom_wear(it, &fandom_sets, &fandom_map);
    }
    resolve_fandom_media_urls(&mut events, &fandom_map);
    for it in events.iter_mut() {
        apply_wiki_preview_icons(it, &wear_map);
    }
    merge_calendar_events(&mut events, weibo_timeline);
    merge_calendar_events(&mut events, collect_weibo_events(weibo_id_mblogs, today));
    events.extend(history);
    promote_ended_cn_to_history(&mut events);
    annotate_intl_against_cn(&mut events);
    events.retain(|e| e.status != "history");
    // 已有当期物品时无需等待所有历史活动的图片；目录仅为缺少兑换列表的公告补充外观。
    if events.iter().any(needs_catalog_looks) {
        if let Ok(catalog) = fetch_sky_events().await {
            for it in events.iter_mut() {
                attach_catalog_looks(it, &catalog.events);
            }
        }
    }
    for it in events.iter_mut() {
        dedupe_news_previews(it);
    }

    Ok(SkyCalendarFeed {
        as_of: today_s,
        season,
        events,
        cn_now,
        cn_next,
        intl_now,
        intl_next,
        event_history,
        warnings,
    })
}

pub(crate) fn parse_smw_event_history(value: &serde_json::Value) -> Vec<SkyEventEdition> {
    super::events::parse_smw_event_rows(value).into_iter()
        .filter(|(title, _, start, end)| {
            let categories = value["query"]["results"][title]["printouts"]["分类"].as_array();
            // 联动可能连续举办且名称不含“联动”；以来源分类决定是否可用于节日预测。
            let Some(categories) = categories else { return false };
            categories.iter().any(|c| c["fulltext"] == "分类:节日活动")
                && !categories.iter().any(|c| c["fulltext"] == "分类:联动活动")
                && parse_ymd_prefix(start).is_some() && parse_ymd_prefix(end).is_some()
        })
        .map(|(title, url, start, end)| SkyEventEdition {
            title: canonical_event_title(&event_family_of(&title)),
            region: "cn".into(), start, end, url,
        })
        .collect()
}

/// 预测只引用明确的一届内容；保持来源服务器和真实日期，不把往届物品当成本届公告。
#[tauri::command]
pub async fn fetch_sky_event_edition(edition: SkyEventEdition) -> Result<SkyCalendarItem, String> {
    let url = reqwest::Url::parse(&edition.url).map_err(|_| "活动来源无效")?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
        || url.port().is_some() || url.query().is_some()
    { return Err("活动来源无效".into()); }
    if edition.region == "cn" && url.host_str() == Some("wiki.biligame.com") && url.path().starts_with("/sky/") {
        let client = sky_http_client()?;
        let page = wiki_decode(url.path().trim_start_matches("/sky/"));
        let (wt, packs) = tokio::join!(
            wiki_wikitext(&client, &page),
            wiki_ask(&client, "[[分类:礼包]]|?所属|?名称|?物品|?价格|sort=首次上架时间|order=desc|limit=500"),
        );
        let wt = wt?;
        let (start, end, _) = super::events::parse_festival_info(&wt);
        if parse_ymd_prefix(&start).map(ymd).as_deref() != Some(&edition.start)
            || parse_ymd_prefix(&end).map(ymd).as_deref() != Some(&edition.end)
        { return Err("来源页面已更新，已停止引用不一致的往届内容".into()); }
        let packs = packs_for_event_title(&group_event_packs(&packs?), &edition.title);
        let mut event = super::events::parse_event_page(&page, &edition.url, &wt, &edition.start,
            &edition.end, chrono::Local::now().date_naive(), &packs);
        super::events::resolve_event_catalog_media(&client, std::slice::from_mut(&mut event)).await;
        let mut item = super::events::event_catalog_as_previews(&event);
        item.note = event.quests.clone();
        return Ok(item);
    }
    if edition.region != "intl" || url.host_str() != Some("sky-children-of-the-light.fandom.com")
        || !url.path().starts_with("/wiki/")
    { return Err("活动来源与服务器不符".into()); }
    let page = wiki_decode(url.path().trim_start_matches("/wiki/")).replace('_', " ");
    let client = fandom_http_client_timeout(40)?;
    let (wt, catalog) = tokio::join!(
        fandom_wikitext(&client, &page),
        fandom_wikitext(&client, "Module:Days Item/data"),
    );
    let wt = wt.ok_or("往届活动页面读取失败")?;
    let catalog = catalog.ok_or("往届物品图片目录读取失败")?;
    let (start, end) = parse_fandom_event_page_window(&page, &wt).ok_or("往届活动日期无法确认")?;
    if ymd(start) != edition.start || end.map(ymd).as_deref() != Some(&edition.end) {
        return Err("来源页面已更新，已停止引用不一致的往届内容".into());
    }
    let promo = parse_fandom_promo_file(&wt);
    let mut item = fandom_calendar_item(&page, &edition.title, start, end,
        chrono::Local::now().date_naive(), promo.as_deref(), String::new(), "intl");
    item.previews = fandom_edition_previews(&wt, &parse_fandom_days_item_data(&catalog));
    attach_fandom_event_media(&mut item, &wt);
    let mut files = collect_fandom_media_files(std::slice::from_ref(&item));
    files.extend(item.previews.iter().filter_map(|p| p.wear.strip_prefix("File:").map(str::to_owned)));
    let urls = fandom_file_urls(&files).await;
    for p in &mut item.previews {
        if let Some(file) = p.wear.strip_prefix("File:") { p.wear = fandom_lookup_url(&urls, file).unwrap_or_default(); }
    }
    resolve_fandom_media_urls(std::slice::from_mut(&mut item), &urls);
    if item.previews.is_empty() { return Err("来源暂未提供这一届活动的物品图片".into()); }
    Ok(item)
}

pub(crate) fn fandom_edition_previews(wt: &str, catalog: &[FandomDaysItem]) -> Vec<super::SkyCalendarPreview> {
    let normalize = |s: &str| s.trim().replace('_', " ").to_ascii_lowercase();
    let names: std::collections::HashSet<_> = parse_fandom_item_names(wt).iter().map(|n| normalize(n)).collect();
    catalog.iter().filter(|row| names.contains(&normalize(&row.name))
        || row.aliases.iter().any(|name| names.contains(&normalize(name))))
        .map(|row| preview(if row.iap { "商城" } else { "兑换" },
            fandom_cn_item_name(&row.name).unwrap_or_else(|| row.name.clone()),
            fandom_kind_from_en_name(&row.name).unwrap_or_default(),
            if row.icon.is_empty() { String::new() } else { format!("File:{}", row.icon) },
            row.files.first().map(|f| format!("File:{f}")).unwrap_or_default()))
        .collect()
}

pub(crate) async fn fetch_calendar_extra(
    today: chrono::NaiveDate,
) -> (Vec<SkyCalendarItem>, Vec<String>, Vec<SkyCalendarItem>, Vec<SkyEventEdition>) {
    let Ok(fc) = fandom_http_client_timeout(40) else {
        let (notices, weibo_timeline, articles) = tokio::join!(
            fetch_cn_official_notices(today),
            fetch_cn_official_event_media(today),
            fetch_cn_official_articles(today),
        );
        let mut out = notices;
        merge_calendar_events(&mut out, articles);
        return (out, Vec::new(), weibo_timeline, Vec::new());
    };
    let (notices, netease_pair, fandom, weibo_timeline, articles) = tokio::join!(
        fetch_cn_official_notices(today),
        fetch_netease_calendar_events(&fc, today),
        fetch_fandom_calendar_events(&fc, today),
        fetch_cn_official_event_media(today),
        fetch_cn_official_articles(today),
    );
    let (netease, weibo_ids) = netease_pair;
    let mut out = notices;
    merge_calendar_events(&mut out, netease);
    let (fandom, history) = fandom;
    merge_calendar_events(&mut out, fandom);
    merge_calendar_events(&mut out, articles);
    (out, weibo_ids, weibo_timeline, history)
}

pub(crate) fn inspect_item_slots(it: &SkyCalendarItem) -> serde_json::Value {
    let mut slots = serde_json::Map::new();
    for p in &it.previews {
        let n = slots.get(&p.slot).and_then(|v| v.as_u64()).unwrap_or(0);
        slots.insert(p.slot.clone(), serde_json::json!(n + 1));
    }
    serde_json::Value::Object(slots)
}

pub(crate) fn inspect_item_json(it: &SkyCalendarItem) -> serde_json::Value {
    serde_json::json!({
        "id": it.id,
        "kind": it.kind,
        "title": it.title,
        "status": it.status,
        "start": it.start,
        "end": it.end,
        "note": it.note,
        "icon": it.icon,
        "slots": inspect_item_slots(it),
        "previews": it.previews.iter().map(|p| serde_json::json!({
            "slot": p.slot,
            "name": p.name,
            "icon": p.icon,
            "wear": p.wear,
        })).collect::<Vec<_>>(),
    })
}

pub(crate) fn inspect_feed_json(feed: &SkyCalendarFeed, title: Option<&str>) -> serde_json::Value {
    let family = title.map(calendar_event_family).unwrap_or_default();
    let mut items = Vec::new();
    if let Some(s) = feed.season.as_ref() {
        if family.is_empty() || calendar_event_family(&s.title) == family {
            items.push(inspect_item_json(s));
        }
    }
    for list in [&feed.events, &feed.cn_now, &feed.cn_next, &feed.intl_now, &feed.intl_next] {
        for it in list {
            if family.is_empty() || calendar_event_family(&it.title) == family {
                items.push(inspect_item_json(it));
            }
        }
    }
    serde_json::json!({
        "ok": true,
        "surface": "calendar",
        "as_of": feed.as_of,
        "title": title,
        "count": items.len(),
        "items": items,
        "event_history": feed.event_history.iter()
            .filter(|e| family.is_empty() || calendar_event_family(&e.title) == family)
            .collect::<Vec<_>>(),
    })
}

pub(crate) async fn inspect_run(args: &[String]) -> Result<serde_json::Value, String> {
    let surface = args.first().map(|s| s.as_str()).unwrap_or("");
    let mut title: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--title" {
            title = args.get(i + 1).cloned();
            i += 2;
            continue;
        }
        i += 1;
    }
    match surface {
        "collection-media" => {
            let input = args.get(1).ok_or("需要外观图标 JSON 文件")?;
            let dir = args.get(2).ok_or("需要图片缓存目录")?;
            let items = serde_json::from_slice(&std::fs::read(input).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            serde_json::to_value(super::collection_media::resolve_collection_media(std::path::Path::new(dir), items, false).await?)
                .map_err(|e| e.to_string())
        }
        "seasons" => {
            let catalog = super::seasons::fetch_sky_seasons().await?;
            serde_json::to_value(catalog).map_err(|e| e.to_string())
        }
        "calendar" => {
            let feed = fetch_sky_calendar().await?;
            Ok(inspect_feed_json(&feed, title.as_deref()))
        }
        "weibo" => {
            let today = chrono::Local::now().date_naive();
            let items = fetch_cn_official_event_media(today).await;
            Ok(inspect_items_json("weibo", title.as_deref(), items))
        }
        "notices" => {
            let today = chrono::Local::now().date_naive();
            let items = fetch_cn_official_notices(today).await;
            Ok(inspect_items_json("notices", title.as_deref(), items))
        }
        "weixin" => {
            let urls = discover_weixin_article_urls().await;
            let items = fetch_weixin_articles(&urls).await;
            Ok(serde_json::json!({
                "ok": true,
                "surface": "weixin",
                "title": title,
                "discovered": urls,
                "count": items.len(),
                "items": items.iter().map(inspect_item_json).collect::<Vec<_>>(),
            }))
        }
        "dashen" => {
            let items = fetch_dashen_square_events().await;
            Ok(inspect_items_json("dashen", title.as_deref(), items))
        }
        "extra" => {
            let today = chrono::Local::now().date_naive();
            let (items, weibo_ids, weibo_timeline, history) = fetch_calendar_extra(today).await;
            let family = title
                .as_deref()
                .map(calendar_event_family)
                .unwrap_or_default();
            let items: Vec<serde_json::Value> = items
                .iter()
                .filter(|it| family.is_empty() || calendar_event_family(&it.title) == family)
                .map(inspect_item_json)
                .collect();
            let weibo: Vec<serde_json::Value> = weibo_timeline
                .iter()
                .filter(|it| family.is_empty() || calendar_event_family(&it.title) == family)
                .map(inspect_item_json)
                .collect();
            Ok(serde_json::json!({
                "ok": true,
                "surface": "extra",
                "title": title,
                "count": items.len(),
                "weibo_ids": weibo_ids,
                "weibo_count": weibo.len(),
                "items": items,
                "weibo": weibo,
                "event_history": history,
            }))
        }
        _ => Err(
            "inspect 用法: waypoint inspect calendar|seasons|weibo|notices|weixin|dashen|extra [--title 名称]；waypoint inspect collection-media <图标JSON> <图片缓存目录>"
                .into(),
        ),
    }
}

pub(crate) fn inspect_items_json(
    surface: &str,
    title: Option<&str>,
    items: Vec<SkyCalendarItem>,
) -> serde_json::Value {
    let family = title.map(calendar_event_family).unwrap_or_default();
    let items: Vec<serde_json::Value> = items
        .iter()
        .filter(|it| family.is_empty() || calendar_event_family(&it.title) == family)
        .map(inspect_item_json)
        .collect();
    serde_json::json!({
        "ok": true,
        "surface": surface,
        "title": title,
        "count": items.len(),
        "items": items,
    })
}

pub fn inspect_main(args: Vec<String>) -> i32 {
    match tauri::async_runtime::block_on(inspect_run(&args)) {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into()));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
