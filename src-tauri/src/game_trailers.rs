//! Public video discovery. Store pages establish work identity; community search always needs user selection.
use crate::game_library::{GameVideo, response};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
pub struct VideoSearchResult {
    #[serde(flatten)]
    pub video: GameVideo,
    pub author: String,
    pub duration: String,
}
fn plain(value: &str) -> String {
    Html::parse_fragment(value).root_element().text().collect::<String>()
}
#[tauri::command]
pub async fn search_game_videos(query: String) -> Result<Vec<VideoSearchResult>, String> {
    let query = query.trim();
    if query.is_empty() || query.len() > 300 { return Err("请输入作品名和 OP / PV 等关键词".into()); }
    let client = crate::game_library::client()?;
    let value = response(client.get("https://api.bilibili.com/x/web-interface/search/type")
        .header(reqwest::header::USER_AGENT,"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Waypoint/0.1 Safari/537.36")
        .header(reqwest::header::REFERER,"https://search.bilibili.com/")
        .query(&[("search_type","video"),("keyword",query),("page","1"),("page_size","12"),("order","totalrank")])
        .timeout(Duration::from_secs(20))).await.map_err(|error| {
            if error.contains("412 Precondition") || error.contains("403 Forbidden") {
                "Bilibili 暂时拒绝了搜索请求，可稍后重试或打开站内搜索。".into()
            } else { error }
        })?;
    if value["code"] != 0 {
        return Err(format!("Bilibili 搜索暂不可用：{}。可稍后重试或打开站内搜索。",value["message"].as_str().unwrap_or("服务拒绝请求")));
    }
    let results = value["data"]["result"].as_array().ok_or("Bilibili 搜索结果格式无法读取，请重试或打开站内搜索。")?;
    Ok(results.iter().filter_map(|v| {
        let id = v["bvid"].as_str()?;
        if id.len() != 12 || !id.starts_with("BV") || !id.bytes().all(|c| c.is_ascii_alphanumeric()) { return None; }
        let page = format!("https://www.bilibili.com/video/{id}/");
        let thumbnail = v["pic"].as_str().unwrap_or_default();
        Some(VideoSearchResult {
            video:GameVideo { title:plain(v["title"].as_str().unwrap_or("Bilibili 视频")),url:page.clone(),page_url:page,
                thumbnail:if thumbnail.starts_with("//") { format!("https:{thumbnail}") } else { thumbnail.replace("http://","https://") } },
            author:v["author"].as_str().unwrap_or_default().into(),duration:v["duration"].as_str().unwrap_or_default().into(),
        })
    }).collect())
}
fn promotion(value: &str) -> bool {
    let text = value.to_lowercase();
    ["trailer","promotion","opening","pv","オープニング","デモムービー","宣传","预告"].iter().any(|s| text.contains(s))
}
fn url(base: &Url, raw: &str) -> Option<Url> {
    let mut url = base.join(raw.trim()).ok()?;
    if !matches!(url.scheme(),"http"|"https") || !url.username().is_empty() || url.password().is_some() { return None; }
    if url.scheme() == "http" { url.set_scheme("https").ok()?; }
    Some(url)
}
/// Parse inert markup only. Decorative loops and scene samples are not promotional videos.
fn page_media(base: &Url, html: &str) -> (Vec<GameVideo>, Vec<Url>) {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href],video[src],video source[src],iframe[src]").unwrap();
    let mut videos: Vec<GameVideo> = Vec::new();
    let mut pages = Vec::new();
    let root = base.join("./").unwrap();
    for element in document.select(&selector) {
        if element.value().attr("loop").is_some()
            || element.ancestors().filter_map(scraper::ElementRef::wrap).any(|e| e.value().name()=="video" && e.value().attr("loop").is_some()) { continue; }
        let raw = element.value().attr("href").or(element.value().attr("src")).unwrap_or_default();
        let Some(link) = url(base,raw) else { continue };
        let caption = format!("{} {} {}",element.text().collect::<String>(),
            element.value().attr("title").unwrap_or_default(),
            element.select(&Selector::parse("img[alt]").unwrap()).filter_map(|i|i.value().attr("alt")).collect::<Vec<_>>().join(" "));
        let context = format!("{raw} {caption}").to_lowercase();
        if ["ero","hscene","h-scene","sex","成人样片","エッチ"].iter().any(|s|context.contains(s)) { continue; }
        let path = link.path().to_lowercase();
        let video_file = [".mp4",".webm",".m4v",".m3u8"].iter().any(|ext|path.ends_with(ext));
        let video_page = matches!(link.host_str(),Some("www.youtube.com"|"youtube.com"|"youtu.be"|"www.youtube-nocookie.com"|"www.bilibili.com"|"player.bilibili.com"));
        if promotion(&context) && (video_file || video_page) && !videos.iter().any(|v|v.url == link.as_str()) {
            let caption = caption.trim();
            videos.push(GameVideo { title:if caption.is_empty() { "官网 PV / OP".into() } else { caption.chars().take(120).collect() },
                url:link.to_string(),page_url:base.to_string(),thumbnail:String::new() });
        }
        if element.value().name()=="a" && link.origin()==root.origin() && root.path()!="/" && link.path().starts_with(root.path())
            && ["movie","download","down.","special"].iter().any(|key|path.contains(key))
            && (path.ends_with(".html")||path.ends_with(".htm")||path.ends_with('/')) && !pages.contains(&link) {
            pages.push(link);
        }
    }
    (videos,pages)
}
async fn read_page(client: &Client, url: Url) -> Result<(Url,String),String> {
    let mut response = crate::public_http::send(client.get(url).timeout(Duration::from_secs(12))).await.map_err(|e|e.to_string())?;
    if !response.status().is_success() { return Err(format!("HTTP {}",response.status())); }
    let base = response.url().clone();
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e|e.to_string())? {
        if data.len()+chunk.len()>2_000_000 { return Err("页面超过读取上限".into()); }
        data.extend_from_slice(&chunk);
    }
    let html = match std::str::from_utf8(&data) {
        Ok(text)=>text.to_owned(),
        Err(_)=>encoding_rs::SHIFT_JIS.decode(&data).0.into_owned(),
    };
    Ok((base,html))
}
pub(crate) async fn official_trailers(client: &Client, pages: Vec<String>) -> (Vec<GameVideo>,Vec<String>) {
    let mut roots = Vec::new();
    for raw in pages {
        let Ok(base)=Url::parse(&raw) else { continue };
        let Some(page)=url(&base,&raw) else { continue };
        let key=page.join("./").unwrap();
        if !roots.iter().any(|(seen,_):&(Url,Url)|seen==&key) { roots.push((key,page)); }
    }
    let mut videos: Vec<GameVideo> = Vec::new();
    let mut warnings = Vec::new();
    for (_,page) in roots.into_iter().take(3) {
        match read_page(client,page.clone()).await {
            Ok((base,html)) => {
                let (found,linked)=page_media(&base,&html);videos.extend(found);
                // One level, two relevant pages; never crawl a publisher's entire catalogue.
                for next in linked.into_iter().take(2) {
                    if let Ok((base,html))=read_page(client,next).await { videos.extend(page_media(&base,&html).0); }
                }
            }
            Err(e)=>warnings.push(format!("官网宣传片暂未取得（{}）：{e}",page.host_str().unwrap_or("官网"))),
        }
    }
    let mut seen = std::collections::HashSet::new();
    videos.retain(|v|seen.insert(v.url.clone()));
    (videos,warnings)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_promotion_keeps_identity_and_excludes_samples_loops_and_unrelated_pages() {
        let base=Url::parse("https://studio.test/game/index.html").unwrap();
        let (videos,pages)=page_media(&base,r#"
            <video><source src="pv3.mp4"></video>
            <video src="opening.mp4" loop muted></video>
            <a href="ero-pv.mp4">サンプル</a><a href="movie.mp4">OP オープニング</a>
            <a href="https://www.youtube.com/watch?v=abc123def45">PV</a>
            <a href="../other/movie.html">別作品</a><a href="movie.html">ムービー</a>
            <a href="javascript:alert(1)">PV</a>
        "#);
        assert_eq!(videos.len(),3);
        assert_eq!(pages,vec![base.join("movie.html").unwrap()]);
        assert!(videos.iter().all(|v|v.page_url==base.as_str()));
    }
    #[tokio::test]
    #[ignore="live read-only official PV enrichment and media header"]
    async fn live_official_discovery() {
        let full=crate::game_library::load_game_metadata("vndb".into(),"v8038".into()).await.unwrap();
        assert!(full.videos.iter().any(|v|v.url.ends_with("pv3.mp4")),"{:?}",full.warnings);
        assert!(!full.sources.iter().any(|s|s.source=="steam"));
        println!("Work enrichment without Steam: {:?}; warnings={:?}",full.videos,full.warnings);
        let mut media=crate::game_library::client().unwrap().get(&full.videos[0].url)
            .header(reqwest::header::RANGE,"bytes=0-1023").send().await.unwrap();
        assert!(media.status().is_success());
        let header=media.chunk().await.unwrap().unwrap();
        assert!(header.windows(4).any(|bytes|bytes==b"ftyp"));
        if let Ok(path)=std::env::var("WAYPOINT_OFFICIAL_MEDIA_OUTPUT") {
            std::fs::write(path,serde_json::to_string_pretty(&full).unwrap()).unwrap();
        }
    }
    #[tokio::test]
    #[ignore="live Bilibili search; source may reject public requests"]
    async fn live_community_search() {
        let results=search_game_videos("美好的每一天 OP".into()).await.unwrap();
        assert!(!results.is_empty());
        println!("Bilibili: {results:?}");
        if let Ok(path)=std::env::var("WAYPOINT_VIDEO_SEARCH_OUTPUT") {
            std::fs::write(path,serde_json::to_string_pretty(&results).unwrap()).unwrap();
        }
    }
}
