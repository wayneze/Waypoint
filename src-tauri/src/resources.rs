use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use tauri::{Emitter, Manager};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct BrowserContext {
    pub game_id: String,
    pub source_url: String,
    pub title: String,
}
#[derive(Default)]
pub struct ResourceBrowser {
    current: Mutex<Option<String>>,
    sequence: AtomicU64,
}

#[derive(Serialize, Deserialize)]
pub struct ResourceArticle {
    pub title: String,
    pub url: String,
    pub markdown: String,
}

fn web_url(raw: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(raw).map_err(|_| "网页地址无效")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
        || !url.username().is_empty() || url.password().is_some() {
        return Err("只支持不含登录凭据的 HTTP / HTTPS 网页".into());
    }
    Ok(url)
}

fn selector(value: &str) -> Selector { Selector::parse(value).expect("static selector") }
fn text(element: ElementRef<'_>) -> String { element.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ") }
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]").replace('|', "\\|").replace('<', "＜").replace('>', "＞")
}
fn link(base: &reqwest::Url, value: &str) -> Option<String> {
    let url = base.join(value).ok()?;
    web_url(url.as_str()).ok().map(|u| u.to_string().replace('(', "%28").replace(')', "%29"))
}

// Convert a parsed article to inert Markdown. No remote HTML, scripts or styles enter the app.
fn markdown(element: ElementRef<'_>, base: &reqwest::Url, depth: usize) -> String {
    if depth > 80 { return String::new(); }
    let name = element.value().name();
    if matches!(name, "script" | "style" | "nav" | "footer" | "form" | "button" | "input" | "iframe" | "noscript" | "svg")
        || element.value().attr("hidden").is_some() || element.value().attr("aria-hidden") == Some("true") {
        return String::new();
    }
    if name == "table" && element.select(&selector("table")).next().is_none() {
        let rows: Vec<Vec<(String, usize)>> = element.select(&selector("tr")).map(|row| row.children().filter_map(ElementRef::wrap)
            .filter(|cell| matches!(cell.value().name(), "td" | "th"))
            .map(|cell| (markdown(cell, base, depth + 1).trim().to_string(), cell.value().attr("colspan").and_then(|v|v.parse().ok()).unwrap_or(1).clamp(1, 100))).collect())
            .filter(|row: &Vec<(String, usize)>| !row.is_empty()).collect();
        let width = rows.iter().map(|row| row.iter().map(|(_, span)|span).sum::<usize>()).max().unwrap_or(0);
        if width == 0 { return String::new(); }
        let mut out = String::from("\n\n");
        let mut in_table = false;
        for row in &rows {
            // Older walkthroughs use full-width table cells as paragraphs.
            // Preserve those as prose instead of squeezing them into column one.
            if row.len() == 1 && (width == 1 || row[0].1 == width) {
                out.push_str(&format!("\n\n{}\n\n", row[0].0));
                in_table = false;
                continue;
            }
            let mut cells = Vec::new();
            for (content, span) in row {
                cells.push(content.lines().map(str::trim).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("<br>"));
                cells.extend(vec![String::new(); span - 1]);
            }
            cells.resize(width, String::new());
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
            if !in_table { out.push_str(&format!("| {} |\n", vec!["---"; width].join(" | "))); }
            in_table = true;
        }
        out.push('\n'); return out;
    }
    let inner = element.children().map(|node| {
        if let Some(child) = ElementRef::wrap(node) { markdown(child, base, depth + 1) }
        else { node.value().as_text().map(|t| escape(t)).unwrap_or_default() }
    }).collect::<String>();
    match name {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => format!("\n\n{} {}\n\n", "#".repeat(name[1..].parse().unwrap_or(2)), inner.trim()),
        "p" | "div" | "section" | "article" | "blockquote" | "pre" => format!("\n\n{}\n\n", inner.trim()),
        "br" => "  \n".into(),
        "li" => format!("\n- {}\n", inner.trim()),
        "hr" => "\n\n---\n\n".into(),
        "strong" | "b" => format!("**{}**", inner.trim()),
        "a" => match element.value().attr("href").and_then(|raw| link(base, raw)) {
            Some(url) => format!("[{}]({})", if inner.trim().is_empty() { &url } else { inner.trim() }, url),
            None => inner,
        },
        "img" => element.value().attr("src").and_then(|raw| link(base, raw)).map(|url| format!("\n\n![{}]({})\n\n", escape(element.value().attr("alt").unwrap_or("原文配图")), url)).unwrap_or_default(),
        _ => inner,
    }
}

pub(crate) fn parse_article(body: &str, url: &reqwest::Url) -> Result<ResourceArticle, String> {
    let html = Html::parse_document(body);
    let title = html.select(&selector("h1")).next().or_else(|| html.select(&selector("title")).next()).map(text).unwrap_or_default();
    let content = [".entry-content", ".post-content", ".article-content", ".rich_media_content", ".guide.subSections", "article", "main", "#content", "body"]
        .iter().find_map(|rule| html.select(&selector(rule)).filter(|e| text(*e).chars().count() >= 80).max_by_key(|e| text(*e).len()))
        .ok_or("页面没有可提取的正文，可在应用内网页中查看")?;
    let raw = markdown(content, url, 0);
    let mut lines = Vec::new(); let mut blank = false;
    for line in raw.lines() {
        if line.trim().is_empty() { if !blank { lines.push(""); } blank = true; }
        else { lines.push(line.trim_start()); blank = false; }
    }
    let output = lines.join("\n").trim().to_string();
    if output.chars().count() < 80 { return Err("正文过短或需要登录，请在应用内网页中查看".into()); }
    Ok(ResourceArticle { title, url: url.to_string(), markdown: output })
}

#[tauri::command]
pub fn write_resource_markdown(dest_path: String, article: ResourceArticle, game_title: String) -> Result<String, String> {
    use std::io::Write;
    let path = std::path::Path::new(&dest_path);
    if !path.is_absolute() || path.extension().and_then(|s|s.to_str()).map(|s| !s.eq_ignore_ascii_case("md")).unwrap_or(true) {
        return Err("请选择完整的 .md 文件保存路径".into());
    }
    let parent = path.parent().filter(|p| p.is_dir()).ok_or("保存目录不存在，请重新选择")?;
    let url = web_url(&article.url)?;
    if article.markdown.trim().is_empty() || article.markdown.len() > 8 * 1024 * 1024 { return Err("攻略正文为空或过大".into()); }
    let quote = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let body = format!("---\ntitle: {}\ngame: {}\nsource: {}\nsaved_at: {}\n---\n\n# {}\n\n> 作品：{}  \n> 来源：[原文]({})\n\n{}\n",
        quote(&article.title), quote(&game_title), quote(url.as_str()), quote(&chrono::Utc::now().to_rfc3339()),
        escape(&article.title.replace(['\r','\n'], " ")), escape(&game_title.replace(['\r','\n'], " ")),
        url.as_str().replace('(', "%28").replace(')', "%29"), article.markdown.trim());
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| format!("无法创建攻略文件：{e}"))?;
    file.write_all(body.as_bytes()).and_then(|_|file.as_file().sync_all()).map_err(|e| format!("攻略写入失败：{e}"))?;
    file.persist(path).map_err(|e| format!("攻略保存失败：{e}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn read_resource_article(
    url: String,
    juij: tauri::State<'_, crate::juij::JuijClient>,
    cache: tauri::State<'_, crate::cache::CacheDb>,
) -> Result<ResourceArticle, String> {
    let parsed = web_url(&url)?;
    if crate::juij::is_juij_url(&parsed) {
        let resource = juij.read(&url, false, &cache).await.map_err(|error| error.message())?;
        return parse_article(&String::from_utf8_lossy(&resource.bytes), &web_url(&resource.url)?);
    }
    fetch_web_article(url).await
}

fn decode_page(bytes: &[u8], content_type: &str) -> String {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_lowercase();
    let hint = format!("{} {head}", content_type.to_lowercase());
    let label = hint.split("charset=").nth(1).map(|s|s.trim_start_matches(['\'', '"', ' '])
        .split(['\'', '"', ';', '>', ' ', '\r', '\n']).next().unwrap_or(""));
    let encoding = label.and_then(|label|encoding_rs::Encoding::for_label(label.as_bytes())).unwrap_or(encoding_rs::UTF_8);
    encoding.decode(bytes).0.into_owned()
}

fn page_redirect(body: &str, base: &reqwest::Url) -> Result<Option<reqwest::Url>, String> {
    let document = Html::parse_document(body);
    for meta in document.select(&Selector::parse("meta[http-equiv]").unwrap()) {
        if !meta.value().attr("http-equiv").unwrap_or("").eq_ignore_ascii_case("refresh") { continue; }
        let content = meta.value().attr("content").unwrap_or("");
        let Some((_, location)) = content.split_once(';') else { continue; };
        let Some((key, value)) = location.trim().split_once('=') else { continue; };
        if !key.trim().eq_ignore_ascii_case("url") { continue; }
        let url = base.join(value.trim().trim_matches(['\'', '"'])).map_err(|e|e.to_string())?;
        return web_url(url.as_str()).map(Some);
    }
    Ok(None)
}

async fn fetch_web_article(url: String) -> Result<ResourceArticle, String> {
    let mut url = web_url(&url)?;
    let client = reqwest::Client::builder().user_agent("Mozilla/5.0 Waypoint/0.1")
        .connect_timeout(crate::PUBLIC_CONNECT_TIMEOUT)
        .timeout(crate::PUBLIC_REQUEST_TIMEOUT)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 || web_url(attempt.url().as_str()).is_err() { attempt.stop() } else { attempt.follow() }
        })).build().map_err(|e| e.to_string())?;
    let mut visited = std::collections::HashSet::new();
    for _ in 0..5 {
        if !visited.insert(url.to_string()) { return Err("网页跳转形成循环，请在应用内网页中查看".into()); }
        let mut response = crate::public_http::send(client.get(url)).await.map_err(|e| format!("读取正文失败：{e}"))?;
        if !response.status().is_success() { return Err(format!("站点返回 HTTP {}，可在应用内网页中查看", response.status().as_u16())); }
        let final_url = response.url().clone();
        let content_type = response.headers().get("content-type").and_then(|h| h.to_str().ok()).unwrap_or("").to_lowercase();
        if !content_type.contains("html") { return Err("这个链接不是正文网页，请在应用内打开或下载".into()); }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
            if bytes.len() + chunk.len() > 4 * 1024 * 1024 { return Err("页面过大，请在应用内网页查看".into()); }
            bytes.extend_from_slice(&chunk);
        }
        let body = decode_page(&bytes, &content_type);
        if let Some(next) = page_redirect(&body, &final_url)? { url = next; continue; }
        return parse_article(&body, &final_url);
    }
    Err("网页跳转次数过多，请在应用内网页中查看".into())
}

struct ResourceSession {
    context: BrowserContext,
    downloads: Mutex<Vec<(std::path::PathBuf, String, BrowserContext)>>,
}
impl ResourceSession {
    fn downloading(&self) -> bool { self.downloads.lock().map(|items| !items.is_empty()).unwrap_or(true) }
}

#[tauri::command]
pub async fn open_resource_web(app: tauri::AppHandle, url: String, context: BrowserContext) -> Result<(), String> {
    open_resource_window(app, url, context)
}

fn open_resource_window(app: tauri::AppHandle, url: String, context: BrowserContext) -> Result<(), String> {
    let url = web_url(&url)?;
    let state = app.state::<ResourceBrowser>();
    let current = state.current.lock().map_err(|e|e.to_string())?.clone();
    // Native downloads retain their originating document. Navigation during a
    // download opens a successor; the old window is reclaimed after completion.
    let label = format!("resource-web-{}", state.sequence.fetch_add(1, Ordering::Relaxed));
    let session = Arc::new(ResourceSession { context: context.clone(), downloads: Mutex::new(Vec::new()) });
    let navigation_app = app.clone();
    let navigation_session = session.clone();
    let popup_app = app.clone();
    let popup_session = session.clone();
    let download_session = session.clone();
    let download_label = label.clone();
    let close_session = session.clone();
    let close_app = app.clone();
    let close_label = label.clone();
    let title = format!("{} · Waypoint", context.title);
    let back = tauri::menu::MenuItem::with_id(&app, "resource-back", "← 后退", true, Some("Alt+Left")).map_err(|e|e.to_string())?;
    let forward = tauri::menu::MenuItem::with_id(&app, "resource-forward", "前进 →", true, Some("Alt+Right")).map_err(|e|e.to_string())?;
    let reload = tauri::menu::MenuItem::with_id(&app, "resource-reload", "刷新", true, Some("F5")).map_err(|e|e.to_string())?;
    let menu = tauri::menu::Menu::with_items(&app, &[&back, &forward, &reload]).map_err(|e|e.to_string())?;
    let window = tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::External(url))
        .title(title).inner_size(1040.0, 780.0).min_inner_size(680.0, 480.0).menu(menu)
        .on_menu_event(|window, event| {
            let script = match event.id.as_ref() { "resource-back" => "history.back()", "resource-forward" => "history.forward()", "resource-reload" => "location.reload()", _ => return };
            if let Some(webview) = window.get_webview_window(window.label()) { let _ = webview.eval(script); }
        })
        .on_navigation(move |url| {
            if web_url(url.as_str()).is_err() { return false; }
            if navigation_session.downloading() {
                let app = navigation_app.clone();
                let context = navigation_session.context.clone();
                let url = url.to_string();
                tauri::async_runtime::spawn(async move { let _ = open_resource_window(app, url, context); });
                return false;
            }
            true
        })
        .on_new_window(move |url, _| {
            if web_url(url.as_str()).is_ok() {
                let app = popup_app.clone();
                let context = popup_session.context.clone();
                tauri::async_runtime::spawn(async move { let _ = open_resource_window(app, url.to_string(), context); });
            }
            tauri::webview::NewWindowResponse::Deny
        })
        .on_download(move |webview, event| {
            match event {
                tauri::webview::DownloadEvent::Requested { destination, url } => {
                    let context = download_session.context.clone();
                    let name = destination.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let picked = crate::dialogs::windows_file_dialog(true, "保存游戏资料", "所有文件 (*.*)|*.*", &name);
                    match picked {
                        Ok(Some(path)) => *destination = std::path::PathBuf::from(path),
                        Ok(None) => return false,
                        Err(error) => {
                            let _ = webview.app_handle().emit_to("main", "resource-download", serde_json::json!({
                                "gameId":context.game_id, "success":false, "error":error
                            }));
                            return false;
                        }
                    }
                    let Ok(mut downloads) = download_session.downloads.lock() else { return false };
                    downloads.push((destination.clone(), url.to_string(), context));
                }
                tauri::webview::DownloadEvent::Finished { path, url, success } => {
                    let item = download_session.downloads.lock().ok().and_then(|mut items| {
                        let index = items.iter().position(|(target, source, _)| path.as_ref().map(|p|p == target).unwrap_or_else(|| source == url.as_str()))?;
                        Some(items.remove(index))
                    });
                    if let Some((_, _, context)) = item {
                        let _ = webview.app_handle().emit_to("main", "resource-download", serde_json::json!({
                            "gameId":context.game_id,"sourceUrl":context.source_url,"title":context.title,
                            "path":path.map(|p|p.to_string_lossy().to_string()),"success":success
                        }));
                    }
                    let current = webview.app_handle().state::<ResourceBrowser>().current.lock().ok().and_then(|v|v.clone());
                    if !download_session.downloading() && current.as_deref() != Some(&download_label) {
                        if let Some(window) = webview.app_handle().get_webview_window(&download_label) { let _ = window.close(); }
                    }
                }
                _ => {}
            }
            true
        }).build().map_err(|e|e.to_string())?;
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            if close_session.downloading() {
                api.prevent_close();
                if let Some(window) = close_app.get_webview_window(&close_label) { let _ = window.hide(); }
                let state = close_app.state::<ResourceBrowser>();
                if let Ok(mut current) = state.current.lock() { if current.as_deref() == Some(&close_label) { *current = None; } };
            }
        }
    });
    *state.current.lock().map_err(|e|e.to_string())? = Some(label);
    // Closing idle predecessors is safe. A downloading predecessor hides until its receipt arrives.
    if let Some(previous) = current.and_then(|label|app.get_webview_window(&label)) { let _ = previous.close(); }
    Ok(())
}

#[tauri::command]
pub async fn resource_web_current(app: tauri::AppHandle) -> Result<String, String> {
    let state = app.state::<ResourceBrowser>();
    let label = state.current.lock().map_err(|e|e.to_string())?.clone().ok_or("资料网页尚未打开")?;
    app.get_webview_window(&label).ok_or("资料网页尚未打开")?.url().map(|u|u.to_string()).map_err(|e|e.to_string())
}

const SKY_BENEFITS_URL: &str = "https://pay.ds.163.com/activity/ma75?appKey=ma75&channel=yydc_cps10.pc_web&goodsFrom=module&hitCrowd=1&listCount=9&setId=6463784fde624c000131e517&wvFullScreen=true&wvStatusBarStyle=white";
const SKY_BENEFITS_CONFIG: &str = "https://payms.ds.163.com/v1/plutusconfig/page_config/common_app_config?id=6463784fde624c000131e517";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BenefitsPage { Checkin, Recharge }

impl BenefitsPage {
    fn label(self) -> &'static str {
        match self { Self::Checkin => "sky-checkin", Self::Recharge => "sky-recharge" }
    }
}

fn official_checkin_link(item: &serde_json::Value) -> Option<String> {
    if item.get("hidden").and_then(|v| v.as_bool()) == Some(true) { return None; }
    if item.get("name").and_then(|v| v.as_str()) == Some("小组件") {
        let extra = item.get("ext").and_then(|v| v.as_str())
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok());
        if let Some(entries) = extra.as_ref().and_then(|v| v.get("list")).and_then(|v| v.as_array()) {
            for entry in entries {
                if entry.get("title").and_then(|v| v.as_str()) == Some("签到福利") {
                    let url = web_url(entry.get("url")?.as_str()?).ok()?;
                    if url.scheme() == "https" && url.host_str() == Some("act.ds.163.com") {
                        return Some(url.into());
                    }
                }
            }
        }
    }
    item.get("itemList").and_then(|v| v.as_array())?.iter().find_map(official_checkin_link)
}

async fn sky_benefits_url(page: BenefitsPage) -> Result<reqwest::Url, String> {
    if matches!(page, BenefitsPage::Recharge) { return web_url(SKY_BENEFITS_URL); }
    // Resolve the current campaign from the same public configuration as the official recharge page.
    let config: serde_json::Value = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15)).build().map_err(|e| e.to_string())?
        .get(SKY_BENEFITS_CONFIG).send().await.map_err(|e| format!("读取官方签到入口失败：{e}"))?
        .error_for_status().map_err(|e| format!("读取官方签到入口失败：{e}"))?
        .json().await.map_err(|e| format!("读取官方签到入口失败：{e}"))?;
    if config.get("code").and_then(|v| v.as_i64()) != Some(0) {
        return Err("官方签到入口暂时不可用，请稍后重试".into());
    }
    let link = config.get("result").and_then(official_checkin_link)
        .ok_or("官网当前没有可用的签到活动")?;
    let mut url = web_url(&link)?;
    url.query_pairs_mut().append_pair("channel", "yydc_cps10.pc_web");
    Ok(url)
}

#[derive(Deserialize)]
pub struct BenefitsBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[tauri::command]
pub async fn set_sky_benefits_view(app: tauri::AppHandle, webview: tauri::Webview, page: BenefitsPage, bounds: Option<BenefitsBounds>) -> Result<(), String> {
    if webview.label() != "main" { return Err("只能从工作台打开福利页".into()); }
    for other in [BenefitsPage::Checkin, BenefitsPage::Recharge] {
        if bounds.is_none() || other.label() != page.label() {
            if let Some(view) = app.get_webview(other.label()) { view.hide().map_err(|e| e.to_string())?; }
        }
    }
    let Some(bounds) = bounds else {
        return Ok(());
    };
    if ![bounds.x, bounds.y, bounds.width, bounds.height].iter().all(|n| n.is_finite())
        || bounds.x < 0.0 || bounds.y < 0.0 || bounds.width < 1.0 || bounds.height < 1.0 {
        return Err("福利页显示区域无效".into());
    }
    let position = tauri::LogicalPosition::new(bounds.x, bounds.y);
    let size = tauri::LogicalSize::new(bounds.width, bounds.height);
    if let Some(view) = app.get_webview(page.label()) {
        view.set_bounds(tauri::Rect { position: position.into(), size: size.into() }).map_err(|e| e.to_string())?;
        return view.show().map_err(|e| e.to_string());
    }
    let url = sky_benefits_url(page).await?;
    let popup_app = app.clone();
    let builder = tauri::webview::WebviewBuilder::new(page.label(), tauri::WebviewUrl::External(url))
        // Keep the existing WebView2 profile: both pages share cookies and local storage across app restarts.
        .data_directory(app.path().app_local_data_dir().map_err(|e| e.to_string())?)
        .incognito(false)
        .on_navigation(|url| web_url(url.as_str()).is_ok())
        .on_new_window(move |url, _| {
            if web_url(url.as_str()).is_ok() {
                if let Some(view) = popup_app.get_webview(page.label()) { let _ = view.navigate(url); }
            }
            tauri::webview::NewWindowResponse::Deny
        });
    // A child webview keeps official login in the main window without third-party iframe cookies.
    webview.window().add_child(builder, position, size).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BenefitsAction { Back, Home }

#[tauri::command]
pub async fn navigate_sky_benefits(app: tauri::AppHandle, webview: tauri::Webview, page: BenefitsPage, action: BenefitsAction) -> Result<(), String> {
    if webview.label() != "main" { return Err("只能从工作台操作福利页".into()); }
    let view = app.get_webview(page.label()).ok_or("福利页尚未打开")?;
    match action {
        BenefitsAction::Home => view.navigate(sky_benefits_url(page).await?),
        BenefitsAction::Back => view.eval("history.back()"),
    }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn japanese_page_encoding_and_declared_redirect_are_resolved_before_reading() {
        let source = "<meta charset='EUC-JP'><title>サクラノ詩 攻略</title><body>選択肢を選ぶ</body>";
        let (bytes, _, _) = encoding_rs::EUC_JP.encode(source);
        assert_eq!(decode_page(&bytes, "text/html"), source);
        let base = web_url("https://seiya-saiga.com/game/nitroplus/steinsgate.html").unwrap();
        let redirect = r#"<meta http-equiv="refresh" content="0;url=https://galge.seiya-saiga.com/game/nitroplus/steinsgate.html">"#;
        assert_eq!(page_redirect(redirect,&base).unwrap().unwrap().host_str(),Some("galge.seiya-saiga.com"));
        assert!(page_redirect(r#"<meta http-equiv="refresh" content="0;url=file:///C:/secret">"#,&base).is_err());
    }
    #[test]
    fn checkin_entry_follows_visible_official_configuration() {
        let config = serde_json::json!({ "itemList": [
            { "name": "小组件", "hidden": true, "ext": r#"{"list":[{"title":"签到福利","url":"https://act.ds.163.com/old"}]}"# },
            { "itemList": [{ "name": "小组件", "ext": r#"{"list":[{"title":"优惠充值","url":"https://pay.ds.163.com/activity/ma75"},{"title":"签到福利","url":"https://act.ds.163.com/current"}]}"# }] }
        ]});
        assert_eq!(official_checkin_link(&config).as_deref(), Some("https://act.ds.163.com/current"));
        let invalid = serde_json::json!({ "name": "小组件", "ext": r#"{"list":[{"title":"签到福利","url":"https://example.com/login"}]}"# });
        assert!(official_checkin_link(&invalid).is_none());
    }
    #[test]
    fn article_preserves_routes_tables_links_and_excludes_active_content() {
        let body = r#"<title>Walkthrough</title><nav>navigation</nav><article><h2>Chapter 1</h2><p>This is a sufficiently long guide introduction describing the choices and routes in the game.</p><p>Option A<br>Option B</p><table><tr><td><table><tr><th>Route</th><th>Choice</th></tr><tr><td>Alice</td><td>Left<br>Right</td></tr></table></td></tr></table><a href="/patch">Patch</a><a href="javascript:alert(1)">bad</a><script>evil()</script><img src="/cover.jpg" onerror="evil()"></article>"#;
        let article = parse_article(body, &web_url("https://example.org/guide").unwrap()).unwrap();
        assert!(article.markdown.contains("## Chapter 1"));
        assert!(article.markdown.contains("Option A  \nOption B"));
        assert!(article.markdown.contains("| Alice | Left<br>Right |"));
        assert_eq!(article.markdown.matches("Alice").count(), 1);
        assert!(article.markdown.contains("https://example.org/patch"));
        assert!(!article.markdown.contains("evil") && !article.markdown.contains("javascript:") && !article.markdown.contains("navigation"));
        assert!(web_url("file:///C:/secret").is_err());
        assert!(web_url("https://user:password@example.org").is_err());
        assert!(parse_article("<body>Login</body>", &web_url("https://example.org").unwrap()).is_err());
        let layout = parse_article("<article><table><tr><th>Route</th><th>Choice</th></tr><tr><td colspan='2'>A long full-width instruction describing the route, its save points and how to complete every ending.</td></tr><tr><td>Alice</td><td>Left</td></tr></table></article>", &web_url("https://example.org").unwrap()).unwrap();
        assert!(layout.markdown.contains("\n\nA long full-width instruction"));
        assert!(!layout.markdown.contains("| A long"));
    }
    #[test]
    fn markdown_export_writes_utf8_and_preserves_source_and_table_layout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("命运石之门 - 攻略.md");
        let url = web_url("https://example.org/guides/page.html").unwrap();
        let article = parse_article(r#"<article><h1>路线攻略</h1><p>A sufficiently long introduction to this guide, which preserves route choices, images and source links.</p><table><tr><th>路线</th><th>选择</th></tr><tr><td>A|B</td><td>左<br>右<img src="../image.png"></td></tr></table></article>"#, &url).unwrap();
        let result = write_resource_markdown(path.to_string_lossy().into(), article, "测试游戏".into()).unwrap();
        assert_eq!(result, path.to_string_lossy());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("game: \"测试游戏\""));
        assert!(text.contains("| A\\|B | 左<br>右<br>![原文配图](https://example.org/image.png) |"));
        assert!(text.contains("source: \"https://example.org/guides/page.html\""));
        let invalid = ResourceArticle { title: "x".into(), url: url.to_string(), markdown: "body".into() };
        assert!(write_resource_markdown(dir.path().join("patch.exe").to_string_lossy().into(), invalid, "x".into()).is_err());
        assert!(!dir.path().join("patch.exe").exists());
    }
    #[tokio::test]
    #[ignore = "requires internet; explicit live reader check"]
    async fn live_seiya_article() {
        for (path, title) in [("nitroplus/steinsgate.html", "STEINS"),("makura-soft/sakuuta.html", "サクラノ詩")] {
            let article = fetch_web_article(format!("https://seiya-saiga.com/game/{path}")).await.unwrap();
            assert!(article.markdown.chars().count() > 500);
            assert!(article.title.contains(title), "{}", article.title);
            assert!(!article.markdown.contains('\u{fffd}'));
            println!("article={} chars={} url={}", article.title, article.markdown.chars().count(), article.url);
        }
    }
}
