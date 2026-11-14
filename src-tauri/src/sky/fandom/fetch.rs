// Fandom API 抓取与文件集。
use super::super::wiki::*;

use super::parse::*;
use super::items::*;
use super::wear::days_item_belongs;

pub(crate) struct FandomFileSets {
    pub(crate) spirit: std::collections::HashMap<String, Vec<String>>,
    pub(crate) reprint_wear: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    pub(crate) warnings: Vec<String>,
    pub(crate) season_page: Vec<String>,
    pub(crate) event_page: Vec<String>,
    pub(crate) file_names: std::collections::HashMap<String, String>,
    pub(crate) event_items: std::collections::HashMap<String, Vec<(String, String)>>,
    pub(crate) days_items: Vec<FandomDaysItem>,
    pub(crate) event_wear: std::collections::HashMap<(String, String), String>,
}

impl FandomFileSets {
    pub(crate) fn empty() -> Self {
        Self {
            spirit: std::collections::HashMap::new(),
            reprint_wear: std::collections::HashMap::new(),
            warnings: Vec::new(),
            season_page: Vec::new(),
            event_page: Vec::new(),
            file_names: std::collections::HashMap::new(),
            event_items: std::collections::HashMap::new(),
            days_items: Vec::new(),
            event_wear: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn all_files(&self) -> Vec<String> {
        let mut out = Vec::new();
        for v in self.spirit.values() {
            out.extend(v.iter().cloned());
        }
        for items in self.reprint_wear.values() {
            out.extend(items.values().cloned());
        }
        out.extend(self.season_page.iter().cloned());
        out.extend(self.event_page.iter().cloned());
        out.sort();
        out.dedup();
        out
    }
}

pub(crate) fn fandom_http_client() -> Result<reqwest::Client, String> {
    fandom_http_client_timeout(20)
}

pub(crate) fn fandom_http_client_timeout(secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(secs))
        .build()
        .map_err(|e| e.to_string())
}

/// Fandom 会限流（429）或偶发 5xx。失败时退避重试两次，再不行才算这页缺失。
pub(crate) async fn fandom_post(client: &reqwest::Client, form: &[(&str, &str)]) -> Option<serde_json::Value> {
    for attempt in 0..3u64 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(800 * attempt)).await;
        }
        let res = client
            .post("https://sky-children-of-the-light.fandom.com/api.php")
            .form(form)
            .send()
            .await;
        match res {
            Ok(r) if r.status().is_success() => {
                if let Ok(v) = r.json().await {
                    return Some(v);
                }
            }
            Ok(r) if r.status().as_u16() != 429 && !r.status().is_server_error() => return None,
            _ => {}
        }
    }
    None
}

pub(crate) fn fandom_pages_map(val: &serde_json::Value) -> Vec<&serde_json::Value> {
    val.pointer("/query/pages")
        .and_then(|v| v.as_object())
        .map(|o| o.values().collect())
        .unwrap_or_default()
}

pub(crate) async fn fandom_file_urls(files: &[String]) -> std::collections::HashMap<String, String> {
    fandom_file_catalog(files).await.urls
}

pub(crate) struct FandomFileCatalog {
    pub urls: std::collections::HashMap<String, String>,
    pub unavailable: usize,
}

/// 区分来源明确缺图和网络失败；缺图不能触发永久“网络重试”。
pub(crate) async fn fandom_file_catalog(files: &[String]) -> FandomFileCatalog {
    let mut out = std::collections::HashMap::new();
    let mut unavailable = 0;
    let Ok(client) = fandom_http_client() else {
        return FandomFileCatalog { urls: out, unavailable: files.len() };
    };
    let mut uniq = files.to_vec();
    uniq.sort();
    uniq.dedup();
    uniq.retain(|s| !s.is_empty());
    // MediaWiki 单次 titles 上限 50。
    for chunk in uniq.chunks(50) {
        let titles = chunk
            .iter()
            .map(|f| format!("File:{}", f))
            .collect::<Vec<_>>()
            .join("|");
        let Some(val) = fandom_post(
            &client,
            &[
                ("action", "query"),
                ("titles", titles.as_str()),
                ("prop", "imageinfo"),
                ("iiprop", "url|size"),
                ("iiurlwidth", "480"),
                ("redirects", "1"),
                ("format", "json"),
            ],
        )
        .await
        else {
            unavailable += chunk.len();
            continue;
        };
        if val.pointer("/query/pages").is_none() {
            unavailable += chunk.len();
            continue;
        }
        for page in fandom_pages_map(&val) {
            if page.get("missing").is_some() {
                continue;
            }
            let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let info = page
                .get("imageinfo")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first());
            let Some(info) = info else {
                continue;
            };
            let url = info
                .get("thumburl")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| info.get("url").and_then(|v| v.as_str()))
                .unwrap_or("");
            if url.is_empty() {
                continue;
            }
            let name = title.strip_prefix("File:").unwrap_or(title);
            out.insert(name.to_string(), url.to_string());
            out.insert(name.replace(' ', "_"), url.to_string());
            out.insert(name.replace('_', " "), url.to_string());
        }
        // MediaWiki 会规范化首字母、下划线及文件重定向，保留请求名称的索引。
        for field in ["redirects", "normalized"] {
            if let Some(aliases) = val.pointer(&format!("/query/{field}")).and_then(|v| v.as_array()) {
                for alias in aliases {
                    if let (Some(from), Some(to)) = (alias["from"].as_str(), alias["to"].as_str()) {
                        if let Some(url) = out.get(to.trim_start_matches("File:")).cloned() {
                            out.insert(from.trim_start_matches("File:").to_string(), url);
                        }
                    }
                }
            }
        }
    }
    FandomFileCatalog { urls: out, unavailable }
}

pub(crate) async fn fandom_titles_images(
    client: &reqwest::Client,
    titles: &[String],
) -> std::collections::HashMap<String, Vec<String>> {
    let mut out = std::collections::HashMap::new();
    let mut uniq = titles.to_vec();
    uniq.retain(|s| !s.is_empty());
    uniq.sort();
    uniq.dedup();
    for chunk in uniq.chunks(8) {
        let joined = chunk.join("|");
        let Some(val) = fandom_post(
            client,
            &[
                ("action", "query"),
                ("titles", joined.as_str()),
                ("prop", "images"),
                ("imlimit", "max"),
                ("redirects", "1"),
                ("format", "json"),
            ],
        )
        .await
        else {
            continue;
        };
        let mut aliases: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        if let Some(normalized) = val.pointer("/query/normalized").and_then(|v| v.as_array()) {
            for n in normalized {
                if let (Some(from), Some(to)) = (
                    n.get("from").and_then(|v| v.as_str()),
                    n.get("to").and_then(|v| v.as_str()),
                ) {
                    aliases.entry(to.to_string()).or_default().push(from.to_string());
                }
            }
        }
        if let Some(redirects) = val.pointer("/query/redirects").and_then(|v| v.as_array()) {
            for n in redirects {
                if let (Some(from), Some(to)) = (
                    n.get("from").and_then(|v| v.as_str()),
                    n.get("to").and_then(|v| v.as_str()),
                ) {
                    aliases.entry(to.to_string()).or_default().push(from.to_string());
                    if let Some(prev) = aliases.remove(from) {
                        aliases.entry(to.to_string()).or_default().extend(prev);
                    }
                }
            }
        }
        for page in fandom_pages_map(&val) {
            if page.get("missing").is_some() {
                continue;
            }
            let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let files = fandom_files_from_images_value(page);
            out.insert(title.to_string(), files.clone());
            if let Some(froms) = aliases.get(title) {
                for from in froms {
                    out.insert(from.clone(), files.clone());
                }
            }
        }
    }
    out
}

pub(crate) async fn fandom_search_title(client: &reqwest::Client, q: &str) -> Option<String> {
    let q = q.trim();
    if q.is_empty() {
        return None;
    }
    let val = fandom_post(
        client,
        &[
            ("action", "query"),
            ("list", "search"),
            ("srsearch", q),
            ("srlimit", "8"),
            ("format", "json"),
        ],
    )
    .await?;
    let hits = val.pointer("/query/search")?.as_array()?;
    for h in hits {
        let title = h.get("title").and_then(|v| v.as_str()).unwrap_or("");
        if title.is_empty() || title.starts_with("File:") || title.starts_with("Category:") {
            continue;
        }
        let low = title.to_ascii_lowercase().replace('_', " ");
        if low == "netease" {
            continue;
        }
        if low.starts_with("netease/") && !is_netease_event_page(title) {
            continue;
        }
        if low.starts_with("netease/") {
            return Some(title.replace('_', " "));
        }
        return Some(fandom_event_root(title));
    }
    None
}

pub(crate) fn is_netease_event_page(title: &str) -> bool {
    let low = title.trim().to_ascii_lowercase().replace('_', " ");
    let Some(rest) = low.strip_prefix("netease/") else {
        return false;
    };
    !matches!(
        rest,
        "cosmetics"
            | "seasons"
            | "shard eruptions"
            | "special events"
            | "traveling spirits"
            | "version history"
            | "light-yu for pc"
            | ""
    )
}

pub(crate) fn fandom_index_title(member: &str) -> Option<String> {
    let t = member.trim();
    if t.is_empty() || t.starts_with("File:") {
        return None;
    }
    let t = t.strip_prefix("Category:").unwrap_or(t);
    let root = fandom_event_root(t);
    if root.is_empty() {
        None
    } else {
        Some(root)
    }
}

pub(crate) fn fandom_zh_names_from_page(page: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(links) = page.get("langlinks").and_then(|v| v.as_array()) else {
        return out;
    };
    for ll in links {
        let lang = ll.get("lang").and_then(|v| v.as_str()).unwrap_or("");
        if !lang.starts_with("zh") {
            continue;
        }
        let name = ll.get("*").and_then(|v| v.as_str()).unwrap_or("").trim();
        if !name.is_empty() && !out.iter().any(|s| s == name) {
            out.push(name.to_string());
        }
    }
    out
}

pub(crate) async fn fandom_category_pages(client: &reqwest::Client, cat: &str) -> Vec<String> {
    let Some(val) = fandom_post(
        client,
        &[
            ("action", "query"),
            ("list", "categorymembers"),
            ("cmtitle", cat),
            ("cmlimit", "80"),
            ("format", "json"),
        ],
    )
    .await
    else {
        return Vec::new();
    };
    val.pointer("/query/categorymembers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("title").and_then(|v| v.as_str()))
                .filter_map(fandom_index_title)
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) async fn fandom_expand_item_wear(client: &reqwest::Client, names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut uniq = names.to_vec();
    uniq.retain(|s| !s.is_empty() && s.len() < 80 && !s.contains('{'));
    uniq.sort();
    uniq.dedup();
    for chunk in uniq.chunks(16) {
        let blob = chunk
            .iter()
            .map(|n| format!("{{{{Icon Name|{n}|real}}}}\n{{{{Icon Name|{n}|front}}}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let Some(val) = fandom_post(
            client,
            &[
                ("action", "expandtemplates"),
                ("text", blob.as_str()),
                ("prop", "wikitext"),
                ("format", "json"),
            ],
        )
        .await
        else {
            continue;
        };
        let text = val
            .pointer("/expandtemplates/wikitext")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        for name in parse_expanded_pngs(text) {
            if !out.iter().any(|s| s == &name) {
                out.push(name);
            }
        }
    }
    out
}

pub(crate) async fn fandom_page_wear_files(
    client: &reqwest::Client,
    titles: &[String],
    skip_names: &std::collections::HashSet<String>,
) -> Vec<String> {
    let mut files = Vec::new();
    let images = fandom_titles_images(client, titles).await;
    for t in titles {
        if let Some(f) = images.get(t) {
            files.extend(f.iter().cloned());
        }
    }
    let texts = fandom_wikitext_many(client, titles).await;
    for t in titles {
        let Some(wt) = texts.get(t) else {
            continue;
        };
        let mut names = parse_fandom_item_names(wt);
        names.retain(|n| {
            !skip_names.contains(n)
                && !n.to_ascii_lowercase().contains("kite")
                && !n.contains("先祖")
        });
        if names.is_empty() {
            continue;
        }
        files.extend(fandom_expand_item_wear(client, &names).await);
    }
    files.sort();
    files.dedup();
    files
}

pub(crate) async fn fandom_wikitext_many(
    client: &reqwest::Client,
    titles: &[String],
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut uniq = titles.to_vec();
    uniq.retain(|s| !s.is_empty());
    uniq.sort();
    uniq.dedup();
    for chunk in uniq.chunks(8) {
        let joined = chunk.join("|");
        let Some(val) = fandom_post(
            client,
            &[
                ("action", "query"),
                ("titles", joined.as_str()),
                ("prop", "revisions"),
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
        let mut aliases: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        if let Some(normalized) = val.pointer("/query/normalized").and_then(|v| v.as_array()) {
            for n in normalized {
                if let (Some(from), Some(to)) = (
                    n.get("from").and_then(|v| v.as_str()),
                    n.get("to").and_then(|v| v.as_str()),
                ) {
                    aliases.entry(to.to_string()).or_default().push(from.to_string());
                }
            }
        }
        if let Some(redirects) = val.pointer("/query/redirects").and_then(|v| v.as_array()) {
            for n in redirects {
                if let (Some(from), Some(to)) = (
                    n.get("from").and_then(|v| v.as_str()),
                    n.get("to").and_then(|v| v.as_str()),
                ) {
                    aliases.entry(to.to_string()).or_default().push(from.to_string());
                    if let Some(prev) = aliases.remove(from) {
                        aliases.entry(to.to_string()).or_default().extend(prev);
                    }
                }
            }
        }
        for page in fandom_pages_map(&val) {
            if page.get("missing").is_some() {
                continue;
            }
            let title = page.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let text = page
                .pointer("/revisions/0/slots/main/*")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if title.is_empty() || text.is_empty() {
                continue;
            }
            out.insert(title.to_string(), text.to_string());
            if let Some(froms) = aliases.get(title) {
                for from in froms {
                    out.insert(from.clone(), text.to_string());
                }
            }
        }
    }
    out
}

pub(crate) async fn fandom_wikitext(client: &reqwest::Client, title: &str) -> Option<String> {
    let val = fandom_post(
        client,
        &[
            ("action", "query"),
            ("titles", title),
            ("prop", "revisions"),
            ("rvprop", "content"),
            ("rvslots", "main"),
            ("redirects", "1"),
            ("format", "json"),
        ],
    )
    .await?;
    for page in fandom_pages_map(&val) {
        if page.get("missing").is_some() {
            continue;
        }
        let text = page
            .pointer("/revisions/0/slots/main/*")
            .and_then(|v| v.as_str())?;
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

pub(crate) fn fandom_gallery_is_sticker(file: &str, caption: &str) -> bool {
    let blob = format!("{} {}", file, caption).to_ascii_lowercase();
    blob.contains("sticker") || blob.contains("mural")
}

pub(crate) fn fandom_event_search_pages(hit: &str, year: &str) -> Vec<String> {
    let mut pages = Vec::new();
    if !year.is_empty() {
        pages.push(fandom_year_page(hit, year));
    }
    let parent = hit
        .rsplit_once('/')
        .and_then(|(base, tail)| {
            if tail.len() == 4 && tail.chars().all(|c| c.is_ascii_digit()) {
                Some(base.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| hit.to_string());
    if !pages.iter().any(|p| p == &parent) {
        pages.push(parent);
    }
    pages
}

pub(crate) fn absorb_days_item_catalog(sets: &mut FandomFileSets, catalog: Vec<FandomDaysItem>) {
    for item in catalog {
        for f in &item.files {
            if !item.name.is_empty() {
                sets.file_names.insert(f.clone(), item.name.clone());
            }
            if !sets.event_page.iter().any(|s| s == f) {
                sets.event_page.push(f.clone());
            }
        }
        sets.days_items.push(item);
    }
}

pub(crate) async fn fandom_fetch_sets(
    spirit_en: &std::collections::HashMap<String, String>,
    reprint_en: &std::collections::HashMap<String, String>,
    event_queries: &[(String, String)],
) -> FandomFileSets {
    let mut sets = FandomFileSets::empty();
    let Ok(client) = fandom_http_client_timeout(40) else {
        return sets;
    };
    let mut en_titles: Vec<String> = spirit_en.values().cloned().collect();
    en_titles.sort();
    en_titles.dedup();
    let first_en = en_titles.first().cloned();
    let (images, season_name, days_catalog, event_pages, reprint_data) = tokio::join!(
        fandom_titles_images(&client, &en_titles),
        async {
            let wt = fandom_wikitext(&client, &first_en?).await?;
            fandom_parse_season_name(&wt)
        },
        async {
            if event_queries.is_empty() {
                return None;
            }
            let wt = fandom_wikitext(&client, "Module:Days Item/data").await?;
            Some(parse_fandom_days_item_data(&wt).into_iter().filter(|item| {
                event_queries.iter().any(|(event, _)| days_item_belongs(&item.days, event))
            }).collect())
        },
        async {
            let mut pages = Vec::new();
            let mut i = 0;
            while i < event_queries.len() {
                let a = event_queries.get(i);
                let b = event_queries.get(i + 1);
                let (ha, hb) = tokio::join!(
                    async {
                        match a {
                            Some((t, y)) => fandom_search_title(&client, t)
                                .await
                                .map(|h| (h, y.clone())),
                            None => None,
                        }
                    },
                    async {
                        match b {
                            Some((t, y)) => fandom_search_title(&client, t)
                                .await
                                .map(|h| (h, y.clone())),
                            None => None,
                        }
                    },
                );
                for (hit, year) in [ha, hb].into_iter().flatten() {
                    for page in fandom_event_search_pages(&hit, &year) {
                        if !pages.iter().any(|p| p == &page) {
                            pages.push(page);
                        }
                    }
                }
                i += 2;
            }
            pages
        },
        async {
            if reprint_en.is_empty() {
                None
            } else {
                fandom_wikitext(&client, "Module:Spirits/data").await
            }
        },
    );
    if let Some(wt) = reprint_data {
        sets.reprint_wear = parse_fandom_spirit_wear(&wt, reprint_en);
    } else if !reprint_en.is_empty() {
        sets.warnings.push("Fandom 复刻实穿数据查询失败，部分实穿图可能缺失".into());
    }
    for (cn, en) in spirit_en {
        if let Some(files) = images.get(en) {
            if !files.is_empty() {
                sets.spirit.insert(cn.clone(), files.clone());
            }
        }
    }
    if let Some(catalog) = days_catalog {
        absorb_days_item_catalog(&mut sets, catalog);
    }
    let skip: std::collections::HashSet<String> = en_titles.into_iter().collect();
    let season_candidates = season_name.map(|season| {
        let mut candidates = vec![season.clone()];
        if !season.to_ascii_lowercase().contains("season") {
            candidates.push(format!("Season of {}", season));
            candidates.push(format!("Season of the {}", season));
        }
        candidates
    });
    let (season_files, event_files) = tokio::join!(
        async {
            if let Some(candidates) = season_candidates.as_ref() {
                fandom_page_wear_files(&client, candidates, &skip).await
            } else {
                Vec::new()
            }
        },
        async {
            if event_pages.is_empty() {
                Vec::new()
            } else {
                fandom_page_wear_files(&client, &event_pages, &std::collections::HashSet::new()).await
            }
        },
    );
    sets.season_page = season_files;
    for f in event_files {
        if !sets.event_page.iter().any(|s| s == &f) {
            sets.event_page.push(f);
        }
    }
    let event_texts = fandom_wikitext_many(&client, &event_pages).await;
    for page in event_pages {
        let Some(wt) = event_texts.get(&page) else {
            continue;
        };
        let names = parse_fandom_item_names(wt);
        if names.is_empty() {
            continue;
        }
        let mut items = Vec::new();
        for name in names {
            let kind = fandom_kind_from_en_name(&name).unwrap_or_default();
            if kind.is_empty() || is_furniture_kind(&kind) {
                continue;
            }
            if !items.iter().any(|(n, _)| n == &name) {
                items.push((name, kind));
            }
        }
        if items.is_empty() {
            continue;
        }
        let title = canonical_event_title(&fandom_event_title_zh(&page, wt, &[]));
        if !title.is_empty() {
            sets.event_items.entry(title).or_default().extend(items);
        }
    }
    sets
}
