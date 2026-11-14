use std::{collections::HashMap, path::Path, sync::Arc};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use super::fandom::*;

#[derive(Deserialize)]
pub struct CollectionMediaRequest {
    pub icon: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct CollectionMediaResult {
    pub images: HashMap<String, String>,
    pub checked: Vec<String>,
    pub unavailable: usize,
}

struct MediaIndex {
    entries: Vec<(FandomCosmetic, Arc<IconSignature>)>,
    unavailable: usize,
}

fn matches_color_variant(icon: &str, candidate: &FandomCosmetic) -> bool {
    let filename = icon.rsplit('/').next().unwrap_or("").split('.').next().unwrap_or("");
    let color = filename.rsplit('_').next().unwrap_or("").to_ascii_lowercase();
    if !matches!(color.as_str(), "red" | "blue" | "green" | "yellow" | "pink" | "purple" | "orange" | "black" | "white") {
        return true;
    }
    // 导出图标可能只有单色轮廓，明确的颜色变体必须由来源文件身份佐证。
    [&candidate.icon, &candidate.wear].iter().any(|file|
        file.split(|c: char| !c.is_ascii_alphabetic()).any(|word| word.eq_ignore_ascii_case(&color)))
}

async fn signature(dir: &Path, url: &str) -> Option<Arc<IconSignature>> {
    let referer = super::wiki_image_referer(url)?;
    let url = if url.contains("wikia.nocookie.net") {
        format!("{url}{}format=original", if url.contains('?') { "&" } else { "?" })
    } else { url.to_string() };
    let (_, bytes) = crate::posters::load_poster(dir, &url, referer).await.ok()?;
    Some(Arc::new(icon_signature(&bytes)?))
}

async fn media_index(dir: &Path) -> Result<MediaIndex, String> {
    let client = fandom_http_client_timeout(60)?;
    let titles: Vec<String> = ["Module:Spirits/data", "Module:Seasons/data", "Module:Days Item/data", "Module:Cosmetics/data", "Small Props", "Large Props"]
        .into_iter().map(String::from).collect();
    let pages = fandom_wikitext_many(&client, &titles).await;
    if titles.iter().any(|t| pages.get(t).is_none_or(|s| s.is_empty())) {
        return Err("外观图片目录未完整载入，请重试".into());
    }
    let mut entries: Vec<_> = titles.iter().flat_map(|t| parse_fandom_cosmetics(&pages[t])).collect();
    for title in ["Small Props", "Large Props"] {
        entries.extend(parse_fandom_closet_cosmetics(&pages[title], "prop"));
    }
    entries.sort_by(|a, b| (&a.kind, &a.icon).cmp(&(&b.kind, &b.icon)));
    entries.dedup_by(|a, b| a.kind == b.kind && a.icon == b.icon);
    let catalog = fandom_file_catalog(&entries.iter().map(|e| e.icon.clone()).collect::<Vec<_>>()).await;
    let urls = catalog.urls;
    let mut indexed = Vec::new();
    let mut unavailable = catalog.unavailable;
    let available: Vec<_> = entries.into_iter().filter(|e| fandom_lookup_url(&urls, &e.icon).is_some()).collect();
    for chunk in available.chunks(6) {
        let get = |index: usize| {
            let urls = &urls;
            async move {
                let entry = chunk.get(index)?;
                let url = fandom_lookup_url(urls, &entry.icon)?;
                let icon = signature(dir, &url).await?;
                Some((entry.clone(), icon))
            }
        };
        let (a,b,c,d,e,f) = tokio::join!(get(0),get(1),get(2),get(3),get(4),get(5));
        let batch: Vec<_> = [a,b,c,d,e,f].into_iter().flatten().collect();
        unavailable += chunk.len() - batch.len();
        indexed.extend(batch);
    }
    if indexed.is_empty() { return Err("外观图标来源暂时无法访问".into()); }
    Ok(MediaIndex { entries: indexed, unavailable })
}

pub(crate) async fn resolve_collection_media(dir: &Path, items: Vec<CollectionMediaRequest>, retry: bool) -> Result<CollectionMediaResult, String> {
    for item in &items {
        if super::wiki_image_referer(&item.icon).is_none() { return Err("不支持此外观图标来源".into()); }
    }
    static INDEX: tokio::sync::Mutex<Option<Arc<MediaIndex>>> = tokio::sync::Mutex::const_new(None);
    let index = {
        let mut cached = INDEX.lock().await;
        // 部分来源失败时，用户的重试必须能重建目录；完整目录供当前进程复用。
        if cached.is_none() || (retry && cached.as_ref().is_some_and(|i| i.unavailable > 0)) {
            *cached = Some(Arc::new(media_index(dir).await?));
        }
        cached.as_ref().unwrap().clone()
    };
    let mut files = HashMap::new();
    let mut checked = Vec::new();
    let mut unavailable = index.unavailable;
    for item in items {
        let Some(icon) = signature(dir, &item.icon).await else { unavailable += 1; continue };
        // 游戏 AP 编号包含初始季，Wiki sort/order 从感恩季起算；只在有明确 AP 编号时约束来源。
        let season = item.icon.split("UiOutfit").nth(1).and_then(|s| s.split("AP").nth(1))
            .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok())
            .and_then(|n| n.checked_sub(1));
        // 游戏旧分类与 Wiki 当前分类不同：旧发型中的帽子已独立为头饰，眼镜也已归到面饰。
        let candidates: Vec<_> = index.entries.iter().filter(|(e,_)| season.is_none_or(|n| e.season == Some(n)))
            .filter(|(e,_)| matches_color_variant(&item.icon, e))
            .filter(|(e,_)| e.kind == item.kind || e.kind == "*"
            || (matches!(item.kind.as_str(), "hair" | "hat" | "horn") && matches!(e.kind.as_str(), "hair" | "horn"))
            || (matches!(item.kind.as_str(), "mask" | "face") && matches!(e.kind.as_str(), "mask" | "face"))
            || (item.kind == "tail" && e.kind == "neck")).collect();
        if let Some(found) = choose_icon_match(&icon, &candidates.iter().map(|(_,s)| s.as_ref()).collect::<Vec<_>>()) {
            files.insert(item.icon.clone(), candidates[found].0.wear.clone());
        }
        if index.unavailable == 0 && !files.contains_key(&item.icon) {
            checked.push(item.icon);
        }
    }
    let urls = fandom_file_urls(&files.values().cloned().collect::<Vec<_>>()).await;
    let mut images = HashMap::new();
    for (icon, file) in files {
        if let Some(url) = fandom_lookup_url(&urls, &file) {
            checked.push(icon.clone());
            images.insert(icon, url);
        } else {
            unavailable += 1;
        }
    }
    Ok(CollectionMediaResult { images, checked, unavailable })
}

#[tauri::command]
pub async fn resolve_sky_collection_media(app: tauri::AppHandle, items: Vec<CollectionMediaRequest>, retry: bool) -> Result<CollectionMediaResult, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("wiki-cache");
    resolve_collection_media(&dir, items, retry).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monochrome_color_variant_needs_matching_source_identity() {
        let mut item = FandomCosmetic { kind: "feet".into(), icon: "Ballet-Flats-icon.png".into(), wear: "Ballet-Flats.png".into(), season: None };
        assert!(!matches_color_variant("https://example/UiOutfitFeetBalletFlats_Red.png", &item));
        assert!(matches_color_variant("https://example/UiOutfitFeetBalletFlats.png", &item));
        item.wear = "Red-Ballet-Flats.png".into();
        assert!(matches_color_variant("https://example/UiOutfitFeetBalletFlats_Red.png", &item));
    }
}
