// 光遇日历补充源：网易专题页与 Fandom 活动页。
use super::super::{SkyCalendarItem, SkyCalendarPreview, SkyEventEdition};
use super::super::wiki::*;
use super::super::weibo::*;
use super::super::fandom::*;

use super::reprints::*;
use super::news::*;
use super::merge::*;

pub(crate) fn netease_calendar_item(
    page: &str,
    title: &str,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
    today: chrono::NaiveDate,
    promo: Option<&str>,
    note: String,
) -> SkyCalendarItem {
    SkyCalendarItem {
        id: format!("cn-netease-{}", netease_page_key(page).replace(' ', "-")),
        kind: "event".into(),
        region: "cn".into(),
        title: title.to_string(),
        names: vec![title.to_string()],
        season: String::new(),
        start: ymd(start),
        end: ymd(end),
        status: event_window_status(Some(start), Some(end), today),
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

pub(crate) fn attach_fandom_event_media(item: &mut SkyCalendarItem, wt: &str) {
    if item.icon.is_empty() {
        if let Some(promo) = parse_fandom_promo_file(wt) {
            item.icon = format!("File:{}", promo);
        }
    }
    if item.note.is_empty() {
        item.note = fandom_lead_note(wt);
    }
    let files: Vec<(String, String)> = parse_fandom_gallery_entries(wt)
        .into_iter()
        .filter(|(file, caption)| {
            !caption.is_empty() && !fandom_gallery_is_sticker(file, caption)
        })
        .collect();
    for (file, caption) in files.into_iter().take(8) {
        let icon = format!("File:{}", file);
        let probe = preview("资讯", caption.clone(), String::new(), icon.clone(), String::new());
        if news_preview_is_cover_dup(item, &probe) {
            continue;
        }
        if item.previews.iter().any(|p| p.slot == "资讯" && p.icon == icon) {
            continue;
        }
        item.previews.push(probe);
    }
}

pub(crate) fn resolve_fandom_media_file(
    raw: &str,
    urls: &std::collections::HashMap<String, String>,
) -> String {
    if let Some(name) = raw.strip_prefix("File:") {
        return fandom_lookup_url(urls, name).unwrap_or_default();
    }
    if calendar_icon_usable(raw) || wiki_filename_from_url(raw).is_some() {
        return raw.to_string();
    }
    String::new()
}

pub(crate) fn resolve_fandom_media_urls(
    items: &mut [SkyCalendarItem],
    urls: &std::collections::HashMap<String, String>,
) {
    for it in items.iter_mut() {
        if it.icon.starts_with("File:") {
            it.icon = resolve_fandom_media_file(&it.icon, urls);
        }
        if !calendar_icon_usable(&it.icon) && wiki_filename_from_url(&it.icon).is_none() {
            it.icon.clear();
        }
        for p in it.previews.iter_mut() {
            p.icon = resolve_fandom_media_file(&p.icon, urls);
        }
        let cover = it.icon.clone();
        it.previews.retain(|p| {
            if p.slot != "资讯" {
                return true;
            }
            if !calendar_icon_usable(&p.icon) {
                return false;
            }
            if cover.is_empty() || p.icon.is_empty() {
                return true;
            }
            !same_media_file(&cover, &p.icon)
        });
        dedupe_news_previews(it);
    }
}

pub(crate) fn apply_wiki_preview_icons(
    item: &mut SkyCalendarItem,
    map: &std::collections::HashMap<String, String>,
) {
    if wiki_filename_from_url(&item.icon).is_some() {
        item.icon = resolve_wiki_file_url(&item.icon, map);
    }
    for p in item.previews.iter_mut() {
        if wiki_filename_from_url(&p.icon).is_some() {
            p.icon = resolve_wiki_file_url(&p.icon, map);
        }
    }
}

pub(crate) fn collect_fandom_media_files(items: &[SkyCalendarItem]) -> Vec<String> {
    let mut files = Vec::new();
    for it in items {
        if let Some(name) = it.icon.strip_prefix("File:") {
            if !files.iter().any(|s| s == name) {
                files.push(name.to_string());
            }
        }
        for p in &it.previews {
            if let Some(name) = p.icon.strip_prefix("File:") {
                if !files.iter().any(|s| s == name) {
                    files.push(name.to_string());
                }
            }
        }
    }
    files
}

pub(crate) async fn fetch_netease_calendar_events(
    client: &reqwest::Client,
    today: chrono::NaiveDate,
) -> (Vec<SkyCalendarItem>, Vec<String>) {
    let (special, cat) = tokio::join!(
        fandom_wikitext(client, "NetEase/Special Events"),
        fandom_category_pages(client, "Category:NetEase"),
    );
    let mut pages: Vec<(String, String, bool)> =
        parse_netease_special_event_links(&special.unwrap_or_default());
    for t in cat {
        if is_netease_event_page(&t)
            && !pages
                .iter()
                .any(|(p, _, _)| netease_page_key(p) == netease_page_key(&t))
        {
            pages.push((t, String::new(), false));
        }
    }
    let titles: Vec<String> = pages.iter().map(|(p, _, _)| p.clone()).collect();
    let wts = fandom_wikitext_many(client, &titles).await;
    let mut items = Vec::new();
    let mut weibo_ids = Vec::new();
    for (page, zh, collab) in &pages {
        let Some(wt) = wts
            .get(page)
            .or_else(|| wts.get(&page.replace(' ', "_")))
            .or_else(|| wts.get(&page.replace('_', " ")))
        else {
            continue;
        };
        let Some((start, end)) = parse_fandom_latest_span(wt) else {
            continue;
        };
        let status = event_window_status(Some(start), Some(end), today);
        if status != "live" && status != "upcoming" {
            continue;
        }
        let title = canonical_event_title(&netease_event_title(page, zh, wt, *collab));
        let promo = parse_fandom_promo_file(wt);
        let mut item = netease_calendar_item(
            page,
            &title,
            start,
            end,
            today,
            promo.as_deref(),
            fandom_lead_note(wt),
        );
        attach_fandom_event_media(&mut item, wt);
        if note_is_noise(&item.note) {
            item.note.clear();
        }
        items.push(item);
        for mid in parse_weibo_status_ids(wt) {
            if !weibo_ids.iter().any(|s| s == &mid) {
                weibo_ids.push(mid);
            }
        }
    }
    let files = collect_fandom_media_files(&items);
    let urls = if files.is_empty() {
        std::collections::HashMap::new()
    } else {
        fandom_file_urls(&files).await
    };
    resolve_fandom_media_urls(&mut items, &urls);
    (items, weibo_ids)
}

pub(crate) async fn fetch_fandom_calendar_events(
    client: &reqwest::Client,
    today: chrono::NaiveDate,
) -> (Vec<SkyCalendarItem>, Vec<SkyEventEdition>) {
    let year = today.format("%Y").to_string();
    let prev = (date_year(today) - 1).to_string();
    let (roots, recurring) = tokio::join!(
        fandom_category_pages(client, "Category:Events"),
        fandom_category_pages(client, "Category:Recurring Events"),
    );
    let mut titles: Vec<String> = Vec::new();
    for root in roots {
        titles.push(fandom_year_page(&root, &year));
        titles.push(fandom_year_page(&root, &prev));
        if recurring.contains(&root) {
            for ago in 2..=3 {
                titles.push(fandom_year_page(&root, &(date_year(today) - ago).to_string()));
            }
        }
        titles.push(root);
    }
    titles.sort();
    titles.dedup();
    let mut items: Vec<SkyCalendarItem> = Vec::new();
    let mut history: Vec<SkyEventEdition> = Vec::new();
    for chunk in titles.chunks(40) {
        let joined = chunk.join("|");
        let Some(val) = fandom_post(
            client,
            &[
                ("action", "query"),
                ("titles", joined.as_str()),
                ("prop", "langlinks|revisions"),
                ("lllimit", "max"),
                ("rvprop", "content"),
                ("rvslots", "main"),
                ("redirects", "1"),
                ("format", "json"),
            ],
        )
        .await
        else {
            continue;
        };
        for page in fandom_pages_map(&val) {
            if page.get("missing").is_some() {
                continue;
            }
            let en = page.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if en.is_empty() {
                continue;
            }
            let wt = page
                .pointer("/revisions/0/slots/main/*")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let year_page = en
                .rsplit_once('/')
                .map(|(_, tail)| tail.len() == 4 && tail.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            // 总目录中的历史日期不属于某场活动；年页才允许从正文读取档期。
            let window = parse_fandom_event_page_window(en, wt);
            let Some((start, end)) = window else {
                continue;
            };
            let status = event_window_status(Some(start), end, today);
            let zh = fandom_zh_names_from_page(page);
            let title = fandom_event_title_zh(en, wt, &zh);
            if recurring.contains(&fandom_event_root(en)) {
                if let Some(end) = end {
                    let edition = SkyEventEdition {
                        title: title.clone(), region: "intl".into(),
                        start: ymd(start), end: ymd(end),
                        url: format!("https://sky-children-of-the-light.fandom.com/wiki/{}", wiki_encode(&en.replace(' ', "_"))),
                    };
                    if let Some(existing) = history.iter_mut().find(|e| e.title == title && e.start == edition.start) {
                        // 年页限定这一届的物品；总目录会随新公告变化，不优先用作往届内容来源。
                        if year_page { *existing = edition; }
                    } else {
                        history.push(edition);
                    }
                }
            }
            if status != "live" && status != "upcoming" {
                if date_year(start) >= date_year(today) - 1 {
                    items.push(history_marker(&title));
                }
                continue;
            }
            let promo = parse_fandom_promo_file(wt);
            let mut item = fandom_calendar_item(
                en,
                &title,
                start,
                end,
                today,
                promo.as_deref(),
                fandom_lead_note(wt),
                "intl",
            );
            attach_fandom_event_media(&mut item, wt);
            if note_is_noise(&item.note) {
                item.note.clear();
            }
            if let Some(existing) = items.iter_mut().find(|e| e.id == item.id) {
                if item.start < existing.start {
                    continue;
                }
                if item.start > existing.start {
                    *existing = item;
                    continue;
                }
                if year_page {
                    let news: Vec<SkyCalendarPreview> = item
                        .previews
                        .iter()
                        .filter(|p| p.slot == "资讯")
                        .cloned()
                        .collect();
                    if !news.is_empty() {
                        existing.previews.retain(|p| p.slot != "资讯");
                        existing.previews.extend(news);
                    }
                    if !item.icon.is_empty() {
                        existing.icon = item.icon.clone();
                    }
                    if !item.note.is_empty() {
                        existing.note = item.note.clone();
                    }
                } else if existing.previews.iter().all(|p| p.slot != "资讯") {
                    existing.previews.extend(
                        item.previews
                            .iter()
                            .filter(|p| p.slot == "资讯")
                            .cloned(),
                    );
                }
            } else {
                items.push(item);
            }
        }
    }
    let files = collect_fandom_media_files(&items);
    let urls = if files.is_empty() {
        std::collections::HashMap::new()
    } else {
        fandom_file_urls(&files).await
    };
    resolve_fandom_media_urls(&mut items, &urls);
    (items, history)
}
