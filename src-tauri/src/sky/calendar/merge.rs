// 光遇档期合并：同场去重、国服/国际对齐、封面与图标挑选。
use super::super::{SkyCalendarItem, SkyCalendarPreview};
use super::super::wiki::*;
use super::super::events::event_status_rank;

use super::news::*;

pub(crate) fn calendar_item_dated(item: &SkyCalendarItem) -> bool {
    parse_ymd_prefix(&item.start).is_some()
}

pub(crate) fn calendar_same_server(a: &SkyCalendarItem, b: &SkyCalendarItem) -> bool {
    let a_cn = a.region != "intl";
    let b_cn = b.region != "intl";
    a_cn == b_cn
}

/// 国服窗口和国际服窗口是两条档期。同服、同节日才合并；国际日期不得写成国服已开。
pub(crate) fn calendar_items_same_event(a: &SkyCalendarItem, b: &SkyCalendarItem) -> bool {
    if a.id == b.id {
        return true;
    }
    if a.kind != "event" || b.kind != "event" || !calendar_same_server(a, b) {
        return false;
    }
    let family = calendar_event_family(&b.title);
    !family.is_empty() && calendar_event_family(&a.title) == family
}

pub(crate) fn adopt_calendar_window(dst: &mut SkyCalendarItem, src: &SkyCalendarItem) {
    if !calendar_item_dated(src) || !calendar_same_server(dst, src) {
        return;
    }
    let src_start = parse_ymd_prefix(&src.start);
    let dst_start = parse_ymd_prefix(&dst.start);
    let src_end = parse_ymd_prefix(&src.end);
    let dst_end = parse_ymd_prefix(&dst.end);
    let newer = match (src_start, dst_start) {
        (Some(s), Some(d)) => s > d || (s == d && src_end > dst_end),
        (Some(_), None) => true,
        _ => false,
    };
    if !newer {
        return;
    }
    dst.start = src.start.clone();
    dst.end = src.end.clone();
    if !src.status.is_empty() {
        dst.status = src.status.clone();
    }
}

pub(crate) fn cn_window_for_family<'a>(
    events: &'a [SkyCalendarItem],
    family: &str,
) -> Option<&'a SkyCalendarItem> {
    events.iter().find(|e| {
        e.region != "intl"
            && calendar_item_dated(e)
            && (e.status == "live" || e.status == "upcoming")
            && calendar_event_family(&e.title) == family
    })
}

fn family_of(item: &SkyCalendarItem) -> String {
    calendar_event_family(&item.title)
}

pub(crate) fn history_marker(title: &str) -> SkyCalendarItem {
    SkyCalendarItem {
        id: format!("cn-history-{}", calendar_event_family(title)),
        kind: "event".into(),
        region: "cn".into(),
        title: canonical_event_title(title),
        names: vec![],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: "history".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    }
}

/// 已经结束的国服活动只留下「今年办过」，不再占当前档期。
pub(crate) fn promote_ended_cn_to_history(events: &mut Vec<SkyCalendarItem>) {
    let mut i = 0;
    while i < events.len() {
        let ended = events[i].region != "intl" && events[i].status == "ended";
        if !ended {
            i += 1;
            continue;
        }
        let family = calendar_event_family(&events[i].title);
        let has_open = events.iter().any(|e| {
            e.region != "intl"
                && (e.status == "live" || e.status == "upcoming")
                && calendar_event_family(&e.title) == family
        });
        let has_history = events.iter().any(|e| {
            e.status == "history" && calendar_event_family(&e.title) == family
        });
        if !has_open && !has_history && !family.is_empty() {
            events.push(history_marker(&events[i].title));
        }
        events.remove(i);
    }
}

/// 国际服窗口对照国服公告。有今年日期就写日期；只有往年记录就标明今年等公告。不编开启日。
pub(crate) fn annotate_intl_against_cn(events: &mut [SkyCalendarItem]) {
    let snapshot: Vec<SkyCalendarItem> = events.to_vec();
    for event in events.iter_mut() {
        if event.region != "intl" {
            continue;
        }
        let family = calendar_event_family(&event.title);
        if let Some(cn) = cn_window_for_family(&snapshot, &family) {
            event.note = match cn.status.as_str() {
                "live" => format!("国服进行中 {} ~ {}", cn.start, cn.end),
                "upcoming" if cn.end.is_empty() => format!("国服 {} 开启", cn.start),
                "upcoming" => format!("国服 {} ~ {}", cn.start, cn.end),
                _ => continue,
            };
            continue;
        }
        if snapshot.iter().any(|e| e.status == "history" && calendar_event_family(&e.title) == family) {
            event.note = "国服往年办过，今年开启日等公告".into();
        }
    }
}

pub(crate) fn merge_calendar_events(dst: &mut Vec<SkyCalendarItem>, extra: Vec<SkyCalendarItem>) {
    for mut it in extra {
        it.title = canonical_event_title(&it.title);
        if it.status == "history" {
            let family = family_of(&it);
            if !dst.iter().any(|e| calendar_event_family(&e.title) == family && e.status == "history") {
                dst.push(it);
            }
            continue;
        }
        // 同场已有在档条目时并进去；历史占位只在没有在档条目时才升级，否则会多出一条同名活动。
        let open_idx = dst
            .iter()
            .position(|e| e.status != "history" && calendar_items_same_event(e, &it));
        if open_idx.is_none() {
            if let Some(existing) = dst.iter_mut().find(|e| {
                e.status == "history" && calendar_event_family(&e.title) == calendar_event_family(&it.title) && calendar_same_server(e, &it)
            }) {
                if calendar_item_dated(&it) && (it.status == "live" || it.status == "upcoming") {
                    existing.id = it.id.clone();
                    existing.start = it.start.clone();
                    existing.end = it.end.clone();
                    existing.status = it.status.clone();
                    existing.title = canonical_event_title(&it.title);
                }
                overlay_event_media(existing, &it);
                continue;
            }
        }
        if let Some(idx) = open_idx {
            let incoming_keeps = calendar_item_dated(&it)
                && (dst[idx].icon.is_empty() && !it.icon.is_empty()
                    || it.previews.iter().any(|p| p.slot == "兑换" || p.slot == "商城")
                        && !dst[idx]
                            .previews
                            .iter()
                            .any(|p| p.slot == "兑换" || p.slot == "商城"));
            if incoming_keeps {
                adopt_calendar_window(&mut it, &dst[idx]);
                overlay_event_media(&mut it, &dst[idx]);
                dst[idx] = it;
            } else {
                adopt_calendar_window(&mut dst[idx], &it);
                overlay_event_media(&mut dst[idx], &it);
            }
            continue;
        }
        if !calendar_item_dated(&it) {
            continue;
        }
        dst.push(it);
    }
    for e in dst.iter_mut() {
        e.title = canonical_event_title(&e.title);
    }
    dst.sort_by(|a, b| {
        event_status_rank(&a.status)
            .cmp(&event_status_rank(&b.status))
            .then_with(|| a.start.cmp(&b.start))
    });
}

pub(crate) fn overlay_event_media(dst: &mut SkyCalendarItem, src: &SkyCalendarItem) {
    let src_title = canonical_event_title(&src.title);
    let dst_title = canonical_event_title(&dst.title);
    if has_cjk(&src_title) && (!has_cjk(&dst_title) || src_title.chars().count() >= dst_title.chars().count()) {
        dst.title = src_title;
    } else if has_cjk(&dst_title) {
        dst.title = dst_title;
    }
    // 卡片图标只补空位。后到的微博长图不能替换已经有的 wiki 图标或官网封面。
    if dst.icon.is_empty() && !src.icon.is_empty() {
        dst.icon = src.icon.clone();
    }
    if note_is_noise(&dst.note) && !note_is_noise(&src.note) {
        dst.note = src.note.clone();
    }
    if (dst.url.is_empty() && !src.url.is_empty())
        || (src.url.contains("sky.163.com") && !dst.url.contains("sky.163.com"))
    {
        dst.url = src.url.clone();
    }
    let dst_has_looks = dst.previews.iter().any(|p| {
        (p.slot == "兑换" || p.slot == "先祖" || p.slot == "季卡") && (!p.wear.is_empty() || !p.icon.is_empty())
    });
    for p in &src.previews {
        if p.slot != "资讯" && p.slot != "商城" && p.slot != "兑换" {
            continue;
        }
        // 目录或复刻表已经有兑换/先祖列表时，后到的活动海报不能再塞进同一张卡。
        if dst_has_looks && p.slot == "资讯" {
            continue;
        }
        if p.slot == "资讯" && !(calendar_icon_usable(&p.icon) || p.icon.starts_with("File:")) {
            continue;
        }
        if dst.previews.iter().any(|x| {
            x.slot == p.slot
                && ((!p.icon.is_empty() && same_media_file(&x.icon, &p.icon))
                    || (p.slot != "资讯" && preview_stem(x) == preview_stem(p)))
        }) {
            continue;
        }
        dst.previews.push(p.clone());
    }
}

pub(crate) fn dedupe_news_previews(item: &mut SkyCalendarItem) {
    let has_looks = item.previews.iter().any(|p| {
        (p.slot == "兑换" || p.slot == "先祖" || p.slot == "季卡") && (!p.wear.is_empty() || !p.icon.is_empty())
    });
    let mut seen = std::collections::HashSet::new();
    item.previews.retain(|p| {
        if p.slot != "资讯" {
            return true;
        }
        if has_looks {
            return false;
        }
        if !calendar_icon_usable(&p.icon) || p.icon.to_ascii_lowercase().contains(".gif") {
            return false;
        }
        let key = calendar_cover_filename(&p.icon);
        !key.is_empty() && seen.insert(key)
    });
}

pub(crate) fn news_preview_is_cover_dup(item: &SkyCalendarItem, p: &SkyCalendarPreview) -> bool {
    if p.slot != "资讯" || p.icon.is_empty() {
        return false;
    }
    if item.icon.is_empty() {
        return false;
    }
    if p.icon == item.icon {
        return true;
    }
    let a = calendar_cover_filename(&item.icon);
    let b = calendar_cover_filename(&p.icon);
    !a.is_empty() && a == b
}

pub(crate) fn calendar_icon_usable(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() || u.starts_with("File:") {
        return false;
    }
    if u.contains("Special:FilePath") || u.contains("Special:Redirect/file") {
        return false;
    }
    if u.contains("static.wikia.nocookie.net")
        || u.contains("nie.res.netease.com")
        || u.contains("patchwiki.biligame.com")
        || u.contains("wiki.biligame.com")
        || u.contains("sinaimg.cn")
        || u.contains("mmbiz.qpic.cn")
        || u.contains("mmbiz.qlogo.cn")
        || u.contains("ok.166.net")
        || u.contains("img.166.net")
    {
        return u.starts_with("https://") || u.starts_with("http://");
    }
    if u.contains("fandom.com") {
        return false;
    }
    u.starts_with("https://") || u.starts_with("http://")
}
