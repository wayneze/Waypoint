// 活动图文按来源页结构提取，活动身份与届次由档期和目录匹配负责。
use serde::Serialize;
use scraper::{Html, Selector};
use super::wiki::{same_media_file, sky_http_client, wiki_decode};

#[derive(Clone, Debug, Serialize)]
pub struct SkyEventGuideImage {
    pub title: String,
    pub url: String,
    pub source_url: String,
}

#[derive(Debug, Serialize)]
pub struct SkyEventGuideImages {
    pub images: Vec<SkyEventGuideImage>,
    pub unavailable_sources: Vec<String>,
}

fn source_url(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw).map_err(|_| "活动来源地址无效")?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err("活动来源必须是 HTTPS 页面".into());
    }
    match url.host_str() {
        Some("sky.163.com") if url.path().starts_with("/news/") || url.path().starts_with("/m/news/") => Ok(url),
        Some("wiki.biligame.com") if url.path().starts_with("/sky/") => Ok(url),
        Some("sky-children-of-the-light.fandom.com") if url.path().starts_with("/wiki/") => Ok(url),
        _ => Err("不支持此活动图文来源".into()),
    }
}

fn guide_caption(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["攻略", "长图", "公告图", "位置", "路线", "地图", "任务", "获取", "兑换图", "兑换树",
        "guide", "location", "map", "quest", "infographic"]
        .iter().any(|word| lower.contains(word))
}

/// 只认正文内有攻略语义的图片；网页导航、活动封面、道具图标不作攻略。
pub(crate) fn parse_event_guide_images(html: &str, source: &str) -> Vec<SkyEventGuideImage> {
    let Ok(base) = source_url(source) else { return Vec::new() };
    let official = base.host_str() == Some("sky.163.com");
    let doc = Html::parse_document(html);
    let root_selector = Selector::parse(if official { ".artText" } else { ".mw-parser-output" }).unwrap();
    let Some(root) = doc.select(&root_selector).next() else { return Vec::new() };
    let nodes = Selector::parse("h2, h3, h4, p, figcaption, img").unwrap();
    let mut heading = String::new();
    let mut caption = String::new();
    let mut images = Vec::new();
    for element in root.select(&nodes) {
        match element.value().name() {
            "h2" | "h3" | "h4" => {
                heading = element.text().collect::<String>().trim().to_string();
                caption.clear();
                continue;
            }
            "p" | "figcaption" => {
                let text = element.text().collect::<String>().trim().to_string();
                if !text.is_empty() { caption = text; }
                continue;
            }
            "img" => {}
            _ => continue,
        }
        let attributes = element.value();
        let alt = attributes.attr("alt").unwrap_or("");
        let raw = attributes.attr("data-src").or_else(|| attributes.attr("src")).unwrap_or("");
        let Ok(mut url) = base.join(raw) else { continue };
        if url.scheme() != "https" { continue; }
        let file = wiki_decode(url.path());
        let own_label = format!("{alt} {file}");
        let is_icon = ["UI-", "图标", "icon", "logo"].iter().any(|word| own_label.to_lowercase().contains(&word.to_lowercase()));
        if is_icon { continue; }
        let context = format!("{heading} {caption}");
        if !guide_caption(&own_label) && !(official && (guide_caption(&context) || context.contains("代币"))) { continue; }
        // Wiki 缩略图的原图在图片自身的链接上，优先使用原始分辨率。
        if !official {
            if let Some(parent) = element.parent().and_then(scraper::ElementRef::wrap) {
                if parent.value().name() == "a" {
                    if let Some(full) = parent.value().attr("href").and_then(|h| base.join(h).ok()) {
                        if full.scheme() == "https" && matches!(full.host_str(),
                            Some("patchwiki.biligame.com" | "static.wikia.nocookie.net"))
                        { url = full; }
                    }
                }
            }
        }
        if images.iter().any(|image: &SkyEventGuideImage| same_media_file(&image.url, url.as_str())) { continue; }
        let title = if official && guide_caption(&caption) && caption.chars().count() <= 36 {
            caption.clone()
        } else if !official && !alt.is_empty() {
            alt.to_string()
        } else if !heading.is_empty() {
            heading.clone()
        } else { "活动图文".into() };
        images.push(SkyEventGuideImage { title, url: url.to_string(), source_url: source.to_string() });
    }
    // 总览长图优先，局部配图仍可查看；没有任何活动名称或日期特判。
    if official && images.iter().any(|image| image.title.contains("长图")) {
        images.retain(|image| image.title.contains("长图"));
    }
    images.sort_by_key(|image| !image.title.contains("长图"));
    images
}

async fn wiki_article(client: &reqwest::Client, page: &str) -> Result<serde_json::Value, String> {
    client.get("https://wiki.biligame.com/sky/api.php")
        .query(&[("action", "parse"), ("page", page), ("prop", "text|wikitext"), ("format", "json")])
        .send().await.map_err(|e| e.to_string())?.error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())
}

async fn fetch_source_images(client: &reqwest::Client, source: &str) -> Result<Vec<SkyEventGuideImage>, String> {
    let url = source_url(source)?;
    let html = if url.host_str() == Some("wiki.biligame.com") {
        let page = wiki_decode(url.path().trim_start_matches("/sky/"));
        let json = wiki_article(client, &page).await?;
        let html = json.pointer("/parse/text/*").and_then(|v| v.as_str()).ok_or("活动 Wiki 正文读取失败")?;
        let mut images = parse_event_guide_images(html, source);
        let wt = json.pointer("/parse/wikitext/*").and_then(|v| v.as_str()).unwrap_or("");
        // 节日信息里的“详细公告”是这一届显式关联的正文，不按活动名搜索别届内容。
        if let Some(inner) = super::wiki::find_template_inners(wt, "节日信息").first() {
            if let Some(notice) = super::wiki::parse_template_fields_nested(inner).get("详细公告").filter(|v| !v.is_empty()) {
                let linked = wiki_article(client, notice).await?;
                let html = linked.pointer("/parse/text/*").and_then(|v| v.as_str()).ok_or("关联公告读取失败")?;
                let linked_url = format!("https://wiki.biligame.com/sky/{}", super::wiki::wiki_encode(notice));
                images.extend(parse_event_guide_images(html, &linked_url));
            }
        }
        return Ok(images);
    } else if url.host_str() == Some("sky-children-of-the-light.fandom.com") {
        let page = wiki_decode(url.path().trim_start_matches("/wiki/"));
        let json = super::fandom::fandom_post(client, &[
            ("action", "parse"), ("page", page.as_str()), ("prop", "text"), ("format", "json"),
        ]).await.ok_or("活动 Fandom 正文读取失败")?;
        json.pointer("/parse/text/*").and_then(|v| v.as_str())
            .ok_or("活动 Fandom 正文读取失败")?.to_string()
    } else {
        client.get(url).send().await.map_err(|e| e.to_string())?
            .error_for_status().map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?
    };
    Ok(parse_event_guide_images(&html, source))
}

#[tauri::command]
pub async fn fetch_sky_event_guide_images(sources: Vec<String>) -> Result<SkyEventGuideImages, String> {
    if sources.is_empty() || sources.len() > 2 { return Err("活动图文需要一至两个匹配来源".into()); }
    for source in &sources { source_url(source)?; }
    let client = sky_http_client()?;
    let mut result = SkyEventGuideImages { images: Vec::new(), unavailable_sources: Vec::new() };
    for source in &sources {
        match fetch_source_images(&client, source).await {
            Ok(images) => {
                for image in images {
                    if !result.images.iter().any(|old| same_media_file(&old.url, &image.url)) {
                        result.images.push(image);
                    }
                }
            }
            Err(_) => result.unavailable_sources.push(source.clone()),
        }
    }
    if result.unavailable_sources.len() == sources.len() { return Err("活动图文来源暂时无法访问，请重试".into()); }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_guides_by_page_structure_across_events() {
        for event in ["恶作剧之日", "缤纷飞行日", "未来的新活动"] {
            let html = format!(r#"<img src="/logo.png"><div class="artText"><h2>{event}</h2>
                <p>活动封面</p><p><img src="/cover.png"></p><p>更多详情请查看公告长图</p>
                <p><img src="https://nie.res.netease.com/guide.png"></p>
                <h2>全新礼包</h2><p><img src="/shop.png"></p></div>"#);
            let images = parse_event_guide_images(&html, "https://sky.163.com/news/update/20261101/1.html");
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].url, "https://nie.res.netease.com/guide.png");
        }
        let wiki = r#"<div class="mw-parser-output"><img alt="糖果代币图标" src="/UI-token.png">
            <img alt="糖果代币烛火" src="/token-flame.png">
            <img alt="海洋节代币位置攻略" src="https://patchwiki.biligame.com/guide.png">
            <img alt="海洋节斗篷展示" src="/cape.png"></div>"#;
        assert_eq!(parse_event_guide_images(wiki, "https://wiki.biligame.com/sky/2026年海洋节").len(), 1);
        let notice = r#"<div class="mw-parser-output"><img alt="20251220公告图1.png" src="https://patchwiki.biligame.com/notice.png"></div>"#;
        let images = parse_event_guide_images(notice, "https://wiki.biligame.com/sky/宴会节更新内容公告");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].title, "20251220公告图1.png");
        assert!(source_url("https://example.com/guide").is_err());
        assert!(source_url("http://127.0.0.1/").is_err());
    }
}
