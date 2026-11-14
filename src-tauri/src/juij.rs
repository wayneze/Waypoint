//! Juij 的搜索、文章和下载跳转共用请求、缓存与限流状态。
use std::{collections::HashMap, sync::Arc, time::Duration};

use reqwest::{header, Url};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::cache::{self, CacheDb};

const CACHE_TTL_MS: i64 = 300_000;
const ERROR_TTL_MS: i64 = 30_000;
const MAX_BODY: usize = 4 * 1024 * 1024;

pub(crate) fn is_juij_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some("h.juij.fun" | "juij.fun" | "i.juij.fun" | "q.juij.fun")
        )
}

fn scope(url: &Url) -> &'static str {
    if matches!(url.host_str(), Some("h.juij.fun" | "juij.fun")) {
        "content"
    } else {
        "download"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JuijError {
    kind: String,
    message: String,
    retry_at: Option<i64>,
}

impl JuijError {
    fn new(kind: &str, message: &str) -> Self {
        Self { kind: kind.into(), message: message.into(), retry_at: None }
    }

    pub(crate) fn message(&self) -> String {
        match self.retry_at.and_then(chrono::DateTime::from_timestamp_millis) {
            Some(time) => format!(
                "{} 可于 {} 再次尝试。",
                self.message,
                time.with_timezone(&chrono::Local).format("%m-%d %H:%M:%S")
            ),
            None => self.message.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct JuijResource {
    pub(crate) url: String,
    pub(crate) bytes: Vec<u8>,
    destination: Option<String>,
}

struct Cached {
    resource: Arc<JuijResource>,
    expires_at: i64,
    etag: Option<header::HeaderValue>,
}

#[derive(Default)]
struct Session {
    entries: HashMap<String, Cached>,
    failures: HashMap<String, JuijError>,
    cooldowns: HashMap<String, JuijError>,
}

pub struct JuijClient {
    client: reqwest::Client,
    // 同一客户端串行检查缓存再发请求；并发调用共享结果，不重复击穿。
    session: Mutex<Session>,
    allowed: fn(&Url) -> bool,
}

fn now() -> i64 { chrono::Utc::now().timestamp_millis() }

fn retry_after(value: Option<&str>, at: i64) -> Option<i64> {
    let value = value?.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(at.saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX).saturating_mul(1000)));
    }
    chrono::DateTime::parse_from_rfc2822(value).ok().map(|date| date.timestamp_millis().max(at))
}

fn freshness(headers: &header::HeaderMap, at: i64) -> (i64, bool) {
    let control = headers.get(header::CACHE_CONTROL).and_then(|h| h.to_str().ok()).unwrap_or("");
    let directives: Vec<_> = control.split(',').map(str::trim).collect();
    let store = !directives.iter().any(|d| d.eq_ignore_ascii_case("no-store"));
    let max_age = directives.iter().find_map(|d| {
        let (name, value) = d.split_once('=')?;
        name.eq_ignore_ascii_case("max-age").then(|| value.trim_matches('"').parse::<i64>().ok()).flatten()
    }).unwrap_or(CACHE_TTL_MS / 1000).max(0);
    let age = headers.get(header::AGE).and_then(|h| h.to_str().ok()).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0).max(0);
    let ttl = if directives.iter().any(|d| d.eq_ignore_ascii_case("no-cache")) { 0 } else { max_age.saturating_sub(age) };
    (at.saturating_add(ttl.saturating_mul(1000)), store)
}

fn read_cooldown(cache: &CacheDb, key: &str) -> Result<Option<JuijError>, JuijError> {
    let conn = cache.0.lock().map_err(|_| JuijError::new("cache", "无法读取站点冷却状态，请重新打开应用。"))?;
    let body: Option<String> = conn.query_row(
        "SELECT body FROM fetch_cache WHERE key = ?1", [key], |r| r.get(0),
    ).optional().map_err(|_| JuijError::new("cache", "无法读取站点冷却状态，请重新打开应用。"))?;
    body.map(|body| serde_json::from_str(&body).map_err(|_| JuijError::new("cache", "站点冷却状态损坏，请检查缓存。"))).transpose()
}

fn save_cooldown(cache: &CacheDb, key: &str, error: &JuijError) -> Result<(), JuijError> {
    let conn = cache.0.lock().map_err(|_| JuijError::new("cache", "无法保存站点冷却状态，请暂勿重启后重试。"))?;
    let body = serde_json::to_string(error).map_err(|_| JuijError::new("cache", "无法保存站点冷却状态。"))?;
    // 未结束的冷却不能被缓存的 30 天清理删掉。
    cache::put(&conn, key, &body, error.retry_at.unwrap_or_else(now))
        .map_err(|_| JuijError::new("cache", "无法保存站点冷却状态，请暂勿重启后重试。"))
}

impl JuijClient {
    pub(crate) fn new() -> Result<Self, reqwest::Error> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("Mozilla/5.0")
                .cookie_store(true)
                .timeout(Duration::from_secs(15))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            session: Mutex::new(Session::default()),
            allowed: is_juij_url,
        })
    }

    pub(crate) async fn read(&self, url: &str, index: bool, cache: &CacheDb) -> Result<Arc<JuijResource>, JuijError> {
        let mut url = Url::parse(url).map_err(|_| JuijError::new("invalid", "站点地址无效。"))?;
        if !(self.allowed)(&url) {
            return Err(JuijError::new("invalid", "只支持非线性列车的文章或下载页面。"));
        }
        url.set_fragment(None);
        let key = format!("{}:{}", if index { "index" } else { "page" }, url);
        let mut session = self.session.lock().await;
        let at = now();
        if let Some(cached) = session.entries.get(&key).filter(|c| c.expires_at > at) {
            return Ok(cached.resource.clone());
        }
        session.failures.retain(|_, e| e.retry_at.is_some_and(|t| t > at));
        if let Some(error) = session.failures.get(&key) { return Err(error.clone()); }
        let etag = session.entries.get(&key).and_then(|c| c.etag.clone());
        let result = tokio::time::timeout(
            Duration::from_secs(20), self.fetch(url, index, cache, &mut session, etag),
        ).await.unwrap_or_else(|_| Err(JuijError::new("network", "站点跳转或读取超时，请稍后重试。")));
        match result {
            Ok((resource, expires_at, etag, store)) => {
                let resource = match resource {
                    Some(resource) => Arc::new(resource),
                    None => session.entries.get(&key).map(|c| c.resource.clone())
                        .ok_or_else(|| JuijError::new("unavailable", "站点返回了无对应缓存的响应。"))?,
                };
                if store {
                    if session.entries.len() >= 16 && !session.entries.contains_key(&key) {
                        if let Some(oldest) = session.entries.iter().min_by_key(|(_, c)| c.expires_at).map(|(k, _)| k.clone()) {
                            session.entries.remove(&oldest);
                        }
                    }
                    session.entries.insert(key, Cached { resource: resource.clone(), expires_at, etag });
                } else {
                    session.entries.remove(&key);
                }
                Ok(resource)
            }
            Err(mut error) => {
                // 短暂合并失败请求；429 的服务级期限另外持久保存。
                if error.retry_at.is_none() { error.retry_at = Some(now() + ERROR_TTL_MS); }
                session.failures.insert(key, error.clone());
                Err(error)
            }
        }
    }

    async fn fetch(
        &self, mut url: Url, index: bool, cache: &CacheDb, session: &mut Session,
        etag: Option<header::HeaderValue>,
    ) -> Result<(Option<JuijResource>, i64, Option<header::HeaderValue>, bool), JuijError> {
        for hop in 0..=5 {
            let cooldown_key = format!("juij.cooldown.{}", scope(&url));
            if !session.cooldowns.contains_key(&cooldown_key) {
                if let Some(error) = read_cooldown(cache, &cooldown_key)? {
                    session.cooldowns.insert(cooldown_key.clone(), error);
                }
            }
            if let Some(error) = session.cooldowns.get(&cooldown_key).filter(|e| e.retry_at.is_some_and(|t| t > now())) {
                return Err(error.clone());
            }
            let mut request = self.client.get(url.clone()).header(header::REFERER, "https://h.juij.fun/");
            if hop == 0 {
                if let Some(etag) = &etag { request = request.header(header::IF_NONE_MATCH, etag); }
            }
            let mut response = request.send().await.map_err(|error| JuijError::new("network",
                if error.is_timeout() { "读取站点超时，请稍后重试或查看原文。" } else { "无法连接站点，请检查网络或查看原文。" }))?;
            let at = now();
            let (expires_at, store) = freshness(response.headers(), at);
            let response_etag = response.headers().get(header::ETAG).cloned();
            if response.status().as_u16() == 429 {
                let retry_at = retry_after(response.headers().get(header::RETRY_AFTER).and_then(|v| v.to_str().ok()), at);
                let error = JuijError {
                    kind: "rate_limited".into(),
                    message: if retry_at.is_some() { "站点已限流，正在遵守服务端的等待时间。" }
                        else { "站点已限流但未提供有效等待时间，应用先暂停请求 5 分钟；这不代表站点届时一定解封。" }.into(),
                    retry_at: Some(retry_at.unwrap_or(at + CACHE_TTL_MS)),
                };
                session.cooldowns.insert(cooldown_key.clone(), error.clone());
                save_cooldown(cache, &cooldown_key, &error)?;
                return Err(error);
            }
            if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                return Ok((None, expires_at, response_etag.or(etag), store));
            }
            if response.status().is_redirection() {
                let destination = response.headers().get(header::LOCATION)
                    .and_then(|v| v.to_str().ok()).and_then(|v| url.join(v).ok())
                    .filter(|u| matches!(u.scheme(), "http" | "https") && u.username().is_empty() && u.password().is_none())
                    .ok_or_else(|| JuijError::new("unavailable", "下载跳转地址无效，请查看原文。"))?;
                if !(self.allowed)(&destination) {
                    if index { return Err(JuijError::new("verification", "搜索索引被跳转到其他网站，请在浏览器查看。")); }
                    return Ok((Some(JuijResource { url: url.into(), bytes: Vec::new(), destination: Some(destination.into()) }),
                        expires_at.min(at + CACHE_TTL_MS), None, store));
                }
                url = destination;
                continue;
            }
            if !response.status().is_success() {
                return Err(if response.status().as_u16() == 403 {
                    JuijError::new("verification", "站点拒绝自动访问，请在浏览器查看提示或完成验证。")
                } else {
                    JuijError::new("unavailable", &format!("站点暂不可用（HTTP {}），请稍后重试或查看原文。", response.status().as_u16()))
                });
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| JuijError::new("network", "读取站点内容中断，请稍后重试。"))? {
                if bytes.len() + chunk.len() > MAX_BODY {
                    return Err(JuijError::new("unavailable", "站点响应过大，请查看原文。"));
                }
                bytes.extend_from_slice(&chunk);
            }
            if index && !serde_json::from_slice::<serde_json::Value>(&bytes).is_ok_and(|v| v.is_array()) {
                return Err(JuijError::new("verification", "站点未返回有效搜索索引，请在浏览器查看。"));
            }
            return Ok((Some(JuijResource { url: url.into(), bytes, destination: None }), expires_at, response_etag, store));
        }
        Err(JuijError::new("verification", "下载页跳转过多，请在浏览器查看。"))
    }
}

/// 外部跳转只返回地址，不请求网盘或文件。
#[tauri::command]
pub async fn read_juij_resource(
    url: String, juij: tauri::State<'_, JuijClient>, cache: tauri::State<'_, CacheDb>,
) -> Result<JuijResource, JuijError> {
    juij.read(&url, false, &cache).await.map(|resource| (*resource).clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Mutex as StdMutex},
        thread,
    };

    struct Server {
        url: String,
        hits: Arc<AtomicUsize>,
        requests: Arc<StdMutex<Vec<String>>>,
        stop: Arc<AtomicBool>,
        thread: Option<thread::JoinHandle<()>>,
    }

    impl Server {
        fn new(responses: Vec<String>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let url = format!("http://{}/", listener.local_addr().unwrap());
            let hits = Arc::new(AtomicUsize::new(0));
            let requests = Arc::new(StdMutex::new(Vec::new()));
            let stop = Arc::new(AtomicBool::new(false));
            let (count, seen, stopped) = (hits.clone(), requests.clone(), stop.clone());
            let thread = thread::spawn(move || {
                while !stopped.load(Ordering::SeqCst) {
                    if let Ok((mut stream, _)) = listener.accept() {
                        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                        let mut request = Vec::new();
                        let mut buf = [0u8; 1024];
                        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => request.extend_from_slice(&buf[..n]),
                            }
                        }
                        seen.lock().unwrap().push(String::from_utf8_lossy(&request).to_string());
                        let n = count.fetch_add(1, Ordering::SeqCst);
                        let response = &responses[n.min(responses.len() - 1)];
                        stream.write_all(response.as_bytes()).unwrap();
                        // Complete the HTTP response before dropping the socket;
                        // an abortive close can discard it on Windows.
                        let _ = stream.shutdown(std::net::Shutdown::Write);
                        let _ = stream.read_to_end(&mut Vec::new());
                    } else { thread::sleep(Duration::from_millis(5)); }
                }
            });
            Self { url, hits, requests, stop, thread: Some(thread) }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
            self.thread.take().unwrap().join().unwrap();
        }
    }
    fn response(status: &str, headers: &str, body: &str) -> String {
        format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len())
    }
    fn client() -> JuijClient {
        let mut client = JuijClient::new().unwrap();
        client.allowed = |url| url.host_str() == Some("127.0.0.1");
        client.client = reqwest::Client::builder().no_proxy().redirect(reqwest::redirect::Policy::none()).build().unwrap();
        client
    }
    fn db(path: &std::path::Path) -> CacheDb { CacheDb(StdMutex::new(cache::open(path).unwrap())) }

    #[tokio::test]
    async fn concurrent_searches_share_index_and_expired_index_revalidates() {
        let body = r#"[{"title":"Paradox Tool","permalink":"/Paradox"},{"title":"SCSKiller","permalink":"/SCSKiller"}]"#;
        let server = Server::new(vec![
            response("200 OK", "Cache-Control: max-age=300\r\nETag: \"index-1\"\r\n", body),
            response("304 Not Modified", "Cache-Control: max-age=300\r\n", ""),
        ]);
        let client = client();
        let cache = db(std::path::Path::new(":memory:"));
        let (a, b) = tokio::join!(client.read(&server.url, true, &cache), client.read(&server.url, true, &cache));
        let (a, b) = (a.unwrap(), b.unwrap());
        assert!(Arc::ptr_eq(&a, &b));
        for query in ["Paradox", "SCSKiller"] {
            assert_eq!(crate::explore::parse_explore_hits("juij", std::str::from_utf8(&a.bytes).unwrap(), query, "https://h.juij.fun/").unwrap().len(), 1);
        }
        assert_eq!(server.hits.load(Ordering::SeqCst), 1);
        client.session.lock().await.entries.values_mut().for_each(|e| e.expires_at = 0);
        assert_eq!(client.read(&server.url, true, &cache).await.unwrap().bytes, a.bytes);
        assert_eq!(server.hits.load(Ordering::SeqCst), 2);
        assert!(server.requests.lock().unwrap()[1].to_lowercase().contains("if-none-match: \"index-1\""));
        client.read(&server.url, true, &cache).await.unwrap();
        assert_eq!(server.hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn rate_limit_survives_restart_and_blocks_other_download_urls() {
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("cache.db");
        for (header, wait_ms) in [
            ("600".to_string(), 600_000),
            ((chrono::Utc::now() + chrono::Duration::minutes(10)).to_rfc2822(), 600_000),
            ("invalid".to_string(), CACHE_TTL_MS),
        ] {
            let server = Server::new(vec![
                response("429 Too Many Requests", &format!("Retry-After: {header}\r\n"), ""),
                response("200 OK", "", "[]"),
            ]);
            {
                let cache = db(&path);
                let client = client();
                let error = client.read(&server.url, false, &cache).await.unwrap_err();
                assert_eq!(error.kind, "rate_limited");
                assert!(error.retry_at.unwrap() > now() + wait_ms - 10_000);
                assert!(error.retry_at.unwrap() <= now() + wait_ms);
                if header == "invalid" { assert!(error.message.contains("未提供有效等待时间")); }
            }
            let cache = db(&path);
            let error = client().read(&format!("{}other", server.url), false, &cache).await.unwrap_err();
            assert_eq!(error.kind, "rate_limited");
            assert_eq!(server.hits.load(Ordering::SeqCst), 1);
            // 下载被限流时，内容域的索引仍能真实发起请求并返回结果。
            let mut content_client = client();
            let addr = Url::parse(&server.url).unwrap().socket_addrs(|| None).unwrap()[0];
            content_client.client = reqwest::Client::builder().no_proxy()
                .resolve("h.juij.fun", addr).redirect(reqwest::redirect::Policy::none()).build().unwrap();
            content_client.allowed = |u| u.host_str() == Some("h.juij.fun");
            let content_url = server.url.replace("127.0.0.1", "h.juij.fun");
            assert_eq!(content_client.read(&content_url, true, &cache).await.unwrap().bytes, b"[]");
            assert_eq!(server.hits.load(Ordering::SeqCst), 2);
            // 用过期记录模拟等待结束，不让测试睡 10 分钟。
            save_cooldown(&cache, "juij.cooldown.download", &JuijError {
                kind: "rate_limited".into(), message: "expired".into(), retry_at: Some(0),
            }).unwrap();
        }
    }

    #[tokio::test]
    async fn redirects_stop_before_external_destination_and_success_is_reused() {
        let server = Server::new(vec![
            response("302 Found", "Location: /download\r\n", ""),
            response("302 Found", "Location: https://pan.quark.cn/s/example\r\n", ""),
        ]);
        let cache = db(std::path::Path::new(":memory:"));
        let client = client();
        let (a, b) = tokio::join!(client.read(&server.url, false, &cache), client.read(&server.url, false, &cache));
        assert_eq!(a.unwrap().destination.as_deref(), Some("https://pan.quark.cn/s/example"));
        assert!(b.unwrap().bytes.is_empty());
        assert_eq!(server.hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalid_index_and_failed_requests_do_not_trigger_retry_storms() {
        for (status, body, kind) in [
            ("200 OK", "<html>Verify your browser</html>", "verification"),
            ("503 Service Unavailable", "", "unavailable"),
        ] {
            let server = Server::new(vec![response(status, "", body)]);
            let cache = db(std::path::Path::new(":memory:"));
            let client = client();
            let (a, b) = tokio::join!(client.read(&server.url, true, &cache), client.read(&server.url, true, &cache));
            assert_eq!(a.unwrap_err().kind, kind);
            assert_eq!(b.unwrap_err().kind, kind);
            assert_eq!(server.hits.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn header_deadlines_and_cache_directives() {
        let at = 1_700_000_000_000;
        assert_eq!(retry_after(Some("60"), at), Some(at + 60_000));
        assert_eq!(retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT"), at), Some(at));
        assert_eq!(retry_after(Some("invalid"), at), None);
        let mut headers = header::HeaderMap::new();
        headers.insert(header::CACHE_CONTROL, "max-age=300".parse().unwrap());
        headers.insert(header::AGE, "100".parse().unwrap());
        assert_eq!(freshness(&headers, at), (at + 200_000, true));
        headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
        assert_eq!(freshness(&headers, at), (at, true));
        headers.insert(header::CACHE_CONTROL, "no-store".parse().unwrap());
        assert!(!freshness(&headers, at).1);
    }

    #[tokio::test]
    #[ignore = "Explicit one-shot live check; never run during ordinary tests"]
    async fn live_juij_search_and_download() {
        let client = JuijClient::new().unwrap();
        let cache = db(std::path::Path::new(":memory:"));
        let index = client.read("https://h.juij.fun/search/index.json", true, &cache).await
            .unwrap_or_else(|e| panic!("{}", e.message()));
        let body = std::str::from_utf8(&index.bytes).unwrap();
        let hits = crate::explore::parse_explore_hits("juij", body, "Paradox", "https://h.juij.fun/").unwrap();
        assert!(!hits.is_empty());
        let article = client.read(&hits[0].url, false, &cache).await.unwrap_or_else(|e| panic!("{}", e.message()));
        assert!(String::from_utf8_lossy(&article.bytes).contains("i.juij.fun"));
        let page = client.read("https://i.juij.fun/Paradox_Interactive?jump=download@Paradox_Interactive", false, &cache).await
            .unwrap_or_else(|e| panic!("{}", e.message()));
        let destination = page.destination.as_deref().and_then(|s| Url::parse(s).ok());
        assert!(destination.is_some(), "No external destination; HTML is not a resolved file");
        println!("Juij live: index_bytes={}, search_hits={}, article_bytes={}, destination_host={}",
            index.bytes.len(), hits.len(), article.bytes.len(), destination.unwrap().host_str().unwrap_or(""));
    }
}
