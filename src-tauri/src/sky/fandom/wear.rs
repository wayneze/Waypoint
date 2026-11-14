// Fandom 试穿打分与节日装扮填充。
use super::super::{SkyCalendarItem, SkyCalendarPreview};
use super::super::wiki::*;

use super::parse::*;
use super::items::*;
use super::fetch::*;
use super::icons::event_uses_icon_catalog;

pub(crate) fn fandom_lookup_url(map: &std::collections::HashMap<String, String>, file: &str) -> Option<String> {
    map.get(file)
        .or_else(|| map.get(&file.replace(' ', "_")))
        .or_else(|| map.get(&file.replace('_', " ")))
        .cloned()
}

pub(crate) fn preview_kind_list(kind: &str) -> Vec<String> {
    kind.split(['、', '，', ','])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(crate) fn fandom_preview_pool(item_kind: &str, sets: &FandomFileSets, p: &SkyCalendarPreview) -> Vec<String> {
    if p.slot == "先祖" || p.slot == "资讯" || is_furniture_kind(&p.kind) {
        return Vec::new();
    }
    if item_kind == "season" && (p.slot == "兑换" || p.slot == "季卡") {
        let stem = preview_stem(p);
        let graduation = p.kind.starts_with("毕业");
        if !graduation {
            if let Some(files) = sets.spirit.get(stem) {
                return files.clone();
            }
        }
        // 先祖页缺（wiki 英文名和 Fandom 对不上）或毕业礼：从季节页里排除别的先祖已认领的物品。
        // Fandom 的毕业礼一律叫 Ultimate。
        let claimed: std::collections::HashSet<String> = sets
            .spirit
            .values()
            .flatten()
            .map(|f| fandom_item_key(f))
            .collect();
        return sets
            .season_page
            .iter()
            .filter(|f| !fandom_stem_used(&claimed, f))
            .filter(|f| f.to_ascii_lowercase().contains("ultimate") == graduation)
            .cloned()
            .collect();
    }
    if item_kind == "season" {
        return sets.season_page.clone();
    }
    sets.event_page.clone()
}

pub(crate) fn fandom_file_conflicts_hint(hint: &str, file: &str) -> bool {
    let toks = fandom_hint_tokens(hint);
    let f = file.to_ascii_lowercase().replace([' ', '_'], "-");
    let wants_trousers = toks.iter().any(|t| t == "trousers" || t == "pants");
    let wants_jumpsuit = toks.iter().any(|t| t == "jumpsuit" || t == "tied");
    if wants_trousers && !wants_jumpsuit && (f.contains("jumpsuit") || f.contains("-tied-")) {
        return true;
    }
    if wants_jumpsuit && !wants_trousers && f.contains("trousers") {
        return true;
    }
    let wants_jeans = toks.iter().any(|t| t == "jeans" || t == "denim");
    if wants_jeans && (f.contains("jumpsuit") || f.contains("-tied-")) {
        return true;
    }
    if wants_jumpsuit && f.contains("jeans") {
        return true;
    }
    false
}

pub(crate) fn scored_wear_candidates(
    files: &[String],
    kind: &str,
    hint: &str,
    used_stems: &std::collections::HashSet<String>,
) -> Vec<(usize, usize, u8, String)> {
    scored_wear_candidates_named(files, kind, hint, used_stems, &std::collections::HashMap::new())
}

pub(crate) fn scored_wear_candidates_named(
    files: &[String],
    kind: &str,
    hint: &str,
    used_stems: &std::collections::HashSet<String>,
    names: &std::collections::HashMap<String, String>,
) -> Vec<(usize, usize, u8, String)> {
    let mut out = Vec::new();
    for f in files {
        if fandom_stem_used(used_stems, f)
            || is_fandom_rejected_wear(f)
            || !fandom_kind_match(kind, f)
            || fandom_file_conflicts_hint(hint, f)
        {
            continue;
        }
        let blob = fandom_file_blob(f, names);
        let (matched, extra) = fandom_content_match_blob(hint, &blob);
        out.push((matched, extra, fandom_file_rank(f), f.clone()));
    }
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    out
}

pub(crate) fn fandom_has_named_token_blob(hint: &str, blob: &str) -> bool {
    let hint_toks: Vec<String> = fandom_hint_tokens(hint)
        .into_iter()
        .filter(|t| !fandom_kindish_token(t))
        .collect();
    let file_toks: Vec<String> = fandom_en_tokens(blob)
        .into_iter()
        .filter(|t| !fandom_kindish_token(t))
        .collect();
    hint_toks.iter().any(|t| {
        !fandom_color_token(t) && file_toks.iter().any(|f| fandom_tok_hit(t, f))
    })
}

pub(crate) fn pick_named_wear_one_named(
    files: &[String],
    kind: &str,
    hint: &str,
    used_stems: &std::collections::HashSet<String>,
    names: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let scored = scored_wear_candidates_named(files, kind, hint, used_stems, names);
    let best = scored.first()?;
    let blob = fandom_file_blob(&best.3, names);
    if best.0 == 0 || !fandom_has_named_token_blob(hint, &blob) {
        return None;
    }
    let top: Vec<&(usize, usize, u8, String)> = scored
        .iter()
        .filter(|x| x.0 == best.0 && x.1 == best.1)
        .collect();
    let mut keys: Vec<String> = top.iter().map(|x| fandom_item_key(&x.3)).collect();
    keys.sort();
    keys.dedup();
    let unique = keys.iter().all(|a| keys.iter().all(|b| fandom_keys_same(a, b)));
    if unique {
        Some(top[0].3.clone())
    } else {
        None
    }
}

#[cfg(test)]
pub(crate) fn pick_named_wear(
    files: &[String],
    kind: &str,
    hint: &str,
    used_stems: &std::collections::HashSet<String>,
) -> Option<String> {
    pick_named_wear_named(files, kind, hint, used_stems, &std::collections::HashMap::new())
}

pub(crate) fn pick_named_wear_named(
    files: &[String],
    kind: &str,
    hint: &str,
    used_stems: &std::collections::HashSet<String>,
    names: &std::collections::HashMap<String, String>,
) -> Option<String> {
    for k in preview_kind_list(kind) {
        if let Some(f) = pick_named_wear_one_named(files, &k, hint, used_stems, names) {
            return Some(f);
        }
    }
    None
}

pub(crate) fn pick_unique_kind_wear(
    files: &[String],
    kind: &str,
    used_stems: &std::collections::HashSet<String>,
) -> Option<String> {
    for k in preview_kind_list(kind) {
        let scored = scored_wear_candidates(files, &k, "", used_stems);
        if scored.is_empty() {
            continue;
        }
        let mut keys: Vec<String> = scored.iter().map(|x| fandom_item_key(&x.3)).collect();
        keys.sort();
        keys.dedup();
        if keys.iter().all(|a| keys.iter().all(|b| fandom_keys_same(a, b))) {
            return Some(scored[0].3.clone());
        }
    }
    None
}

pub(crate) fn fandom_stem_from_wear_url(url: &str) -> Option<String> {
    if url.contains("biligame") || url.contains("展示图") {
        return None;
    }
    let name = url.rsplit('/').next()?.split(['?', '#']).next()?;
    if !name.to_ascii_lowercase().ends_with(".png") {
        return None;
    }
    Some(fandom_item_stem(name))
}

pub(crate) fn choose_fandom_wear_files(item: &SkyCalendarItem, sets: &FandomFileSets) -> Vec<Option<String>> {
    if item.kind == "event" && event_uses_icon_catalog(item) {
        return item.previews.iter().map(|p| {
            if !p.wear.is_empty() || (p.slot != "兑换" && p.slot != "商城") { return None; }
            sets.event_wear.get(&(item.title.clone(), p.icon.clone())).cloned()
        }).collect();
    }
    // 复刻只按来源声明的先祖和物品类型取图，不进入整页文件名打分。
    if item.kind == "reprint" {
        let mut chosen: Vec<Option<String>> = item.previews.iter().map(|p| {
            if p.slot != "兑换" || !p.wear.is_empty() {
                return None;
            }
            sets.reprint_wear.get(preview_stem(p))?.get(&p.kind).cloned()
        }).collect();
        let mut counts = std::collections::HashMap::new();
        for (p, file) in item.previews.iter().zip(&chosen) {
            if let Some(file) = file {
                *counts.entry((preview_stem(p), file.replace('_', " "))).or_insert(0) += 1;
            }
        }
        for (p, file) in item.previews.iter().zip(&mut chosen) {
            if file.as_ref().is_some_and(|f| counts[&(preview_stem(p), f.replace('_', " "))] > 1) {
                *file = None;
            }
        }
        return chosen;
    }
    let item_kind = item.kind.clone();
    let mut used_stems = std::collections::HashSet::new();
    for p in &item.previews {
        if let Some(stem) = fandom_stem_from_wear_url(&p.wear) {
            used_stems.insert(fandom_item_key(&stem));
        }
    }
    let n = item.previews.len();
    let mut chosen: Vec<Option<String>> = vec![None; n];
    for (i, p) in item.previews.iter().enumerate() {
        if fandom_stem_from_wear_url(&p.wear).is_some() {
            continue;
        }
        let files = fandom_preview_pool(&item_kind, sets, p);
        if let Some(file) = pick_named_wear_named(
            &files,
            &p.kind,
            preview_stem(p),
            &used_stems,
            &sets.file_names,
        ) {
            used_stems.insert(fandom_item_key(&file));
            chosen[i] = Some(file);
        }
    }
    if item_kind == "season" {
        for (i, p) in item.previews.iter().enumerate() {
            if chosen[i].is_some() || !p.wear.is_empty() {
                continue;
            }
            if p.slot == "先祖" || is_furniture_kind(&p.kind) {
                continue;
            }
            let files = fandom_preview_pool(&item_kind, sets, p);
            if let Some(file) = pick_unique_kind_wear(&files, &p.kind, &used_stems) {
                used_stems.insert(fandom_item_key(&file));
                chosen[i] = Some(file);
            }
        }
    }
    chosen
}

pub(crate) fn apply_fandom_wear(
    item: &mut SkyCalendarItem,
    sets: &FandomFileSets,
    map: &std::collections::HashMap<String, String>,
) {
    if map.is_empty() {
        return;
    }
    let chosen = choose_fandom_wear_files(item, sets);
    for (p, file) in item.previews.iter_mut().zip(chosen) {
        if let Some(file) = file {
            if let Some(url) = fandom_lookup_url(map, &file) {
                p.wear = url;
            }
        }
    }
}

pub(crate) fn days_items_for_event(sets: &FandomFileSets, event: &SkyCalendarItem) -> Vec<(String, String)> {
    let Some(en) = fandom_en_for_cn_event(&event.title) else {
        return Vec::new();
    };
    let year = calendar_event_year(event);
    let mut year_hit = false;
    for item in &sets.days_items {
        if days_item_belongs(&item.days, en) && !year.is_empty() && item.year == year {
            year_hit = true;
            break;
        }
    }
    let mut out = Vec::new();
    for item in &sets.days_items {
        let (y, name, files) = (&item.year, &item.name, &item.files);
        if !days_item_belongs(&item.days, en) {
            continue;
        }
        if year_hit && y != &year {
            continue;
        }
        if !year_hit && !year.is_empty() && !y.is_empty() && y != &year {
            continue;
        }
        let file = files.first().cloned().unwrap_or_default();
        let kind = fandom_kind_from_en_name(name)
            .or_else(|| fandom_kind_from_en_name(&file))
            .unwrap_or_default();
        if kind.is_empty() || is_furniture_kind(&kind) {
            continue;
        }
        let label = fandom_cn_item_name(name)
            .or_else(|| fandom_cn_item_name(&file))
            .filter(|s| has_cjk(s))
            .unwrap_or_default();
        if label.is_empty() {
            continue;
        }
        if !out.iter().any(|(n, _)| n == &label) {
            out.push((label, kind));
        }
    }
    out
}

pub(crate) fn fill_event_from_fandom(event: &mut SkyCalendarItem, sets: &FandomFileSets) {
    if event.kind != "event" {
        return;
    }
    if event.previews.iter().any(|p| p.slot == "兑换") {
        return;
    }
    let mut items = days_items_for_event(sets, event);
    if items.is_empty() {
        items = sets
            .event_items
            .get(&event.title)
            .cloned()
            .unwrap_or_default();
    }
    let mut seen = std::collections::HashSet::new();
    for (name, kind) in items {
        let label = if has_cjk(&name) {
            name
        } else {
            fandom_cn_item_name(&name).unwrap_or_default()
        };
        if label.is_empty() || !seen.insert(label.clone()) {
            continue;
        }
        event.previews.push(preview(
            "兑换",
            format!("{} · {}", label, kind),
            kind.clone(),
            cosmetic_icon_url(&kind, &label),
            cosmetic_wear_url(&kind, &label),
        ));
    }
}

pub(crate) fn days_item_belongs(days: &str, en_event: &str) -> bool {
    let d = days.trim().to_ascii_lowercase();
    if d.len() < 3 {
        return false;
    }
    let en = en_event.to_ascii_lowercase();
    let hit = en.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == d);
    hit || (d == "rainbow" && en.split(|c: char| !c.is_ascii_alphanumeric()).any(|t| t == "color"))
}
