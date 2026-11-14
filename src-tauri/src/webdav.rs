use std::{convert::Infallible, pin::Pin, sync::Mutex, task::{Context, Poll}, time::Duration};
use bytes::Bytes;
use chrono::Local;
use http_body::{Body, Frame, SizeHint};
use reqwest::{Client, Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{ipc::Channel, AppHandle, Manager, State};
use tokio::{sync::watch, time::Instant};

use crate::db::{restore_backup, db_file_path};
use crate::settings::read_app_setting;
use crate::AppState;

// Keep the remote object's name so existing WebDAV versions/conflict stamps
// remain usable. Its bytes now carry WPBACK01; decode also accepts old Gzip.
const REMOTE_BACKUP_FILENAME: &str = "waypoint-backup.db.gz";
// Full backups include images; metadata requests retain the short timeout.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const UPLOAD_IDLE_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Serialize)]
pub struct BackupProgress {
    phase: &'static str,
    sent: u64,
    total: u64,
}

#[derive(Default)]
pub struct BackupUpload(Mutex<Option<watch::Sender<bool>>>);

struct UploadOperation<'a>(&'a BackupUpload);
impl Drop for UploadOperation<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.0.lock() { active.take(); }
    }
}

#[tauri::command]
pub fn cancel_webdav_backup(upload: State<'_, BackupUpload>) -> Result<(), String> {
    if let Some(cancel) = upload.0.lock().map_err(|e| e.to_string())?.as_ref() {
        cancel.send_replace(true);
    }
    Ok(())
}

fn report(channel: Option<&Channel<BackupProgress>>, phase: &'static str, sent: u64, total: u64) {
    if let Some(channel) = channel { let _ = channel.send(BackupProgress { phase, sent, total }); }
}

// Supply bounded frames under the HTTP client's backpressure. Report bytes handed
// to the transport, never claim that they are already stored on the server.
struct UploadBody {
    bytes: Bytes,
    sent: u64,
    total: u64,
    activity: watch::Sender<Instant>,
    channel: Option<Channel<BackupProgress>>,
    last_report: Instant,
}

impl Body for UploadBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        if self.bytes.is_empty() { return Poll::Ready(None); }
        let length = self.bytes.len().min(64 * 1024);
        let chunk = self.bytes.split_to(length);
        self.sent += length as u64;
        let now = Instant::now();
        self.activity.send_replace(now);
        if now.duration_since(self.last_report) >= Duration::from_millis(250) || self.sent == self.total {
            report(self.channel.as_ref(), "uploading", self.sent, self.total);
            self.last_report = now;
        }
        Poll::Ready(Some(Ok(Frame::data(chunk))))
    }

    fn is_end_stream(&self) -> bool { self.bytes.is_empty() }
    fn size_hint(&self) -> SizeHint { SizeHint::with_exact(self.bytes.len() as u64) }
}

async fn send_upload(
    request: reqwest::RequestBuilder, bytes: Vec<u8>,
    channel: Option<&Channel<BackupProgress>>, cancel: &mut watch::Receiver<bool>,
    idle_timeout: Duration,
) -> Result<reqwest::Response, String> {
    let (activity, progress) = watch::channel(Instant::now());
    let total = bytes.len() as u64;
    let body = UploadBody {
        bytes: bytes.into(), sent: 0, total, activity,
        channel: channel.cloned(), last_report: Instant::now(),
    };
    let sending = async {
        request.body(reqwest::Body::wrap(body)).send().await.map_err(|e| request_error("上传备份", e))
    };
    watch_upload(sending, progress, cancel, idle_timeout).await
}

async fn watch_upload<T>(
    sending: impl std::future::Future<Output = Result<T, String>>,
    mut progress: watch::Receiver<Instant>, cancel: &mut watch::Receiver<bool>, idle_timeout: Duration,
) -> Result<T, String> {
    tokio::pin!(sending);
    let mut progress_open = true;
    let mut cancel_open = true;
    loop {
        if *cancel.borrow() { return Err("已取消上传，云端原备份未替换".into()); }
        let deadline = *progress.borrow() + idle_timeout;
        tokio::select! {
            response = &mut sending => return response,
            changed = cancel.changed(), if cancel_open => {
                cancel_open = changed.is_ok();
                if *cancel.borrow() { return Err("已取消上传，云端原备份未替换".into()); }
            },
            changed = progress.changed(), if progress_open => { progress_open = changed.is_ok(); },
            _ = tokio::time::sleep_until(deadline) => {
                return Err("上传连续 2 分钟没有传输进展或服务器确认，已停止；云端原备份未替换。请检查网络或 WebDAV 服务后重试".into());
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebdavConfig {
    pub url: String,
    pub username: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebdavStatus {
    pub configured: bool,
    pub url: String,
    pub username: String,
    pub has_password: bool,
    pub last_sync_time: Option<String>,
    pub remote_exists: bool,
    pub remote_size: Option<u64>,
    pub remote_modified: Option<String>,
    pub remote_error: Option<String>,
}

fn build_client() -> Result<Client, String> {
    client_builder().timeout(Duration::from_secs(30))
        .build().map_err(|e| format!("初始化 HTTP 客户端失败: {e}"))
}

fn client_builder() -> reqwest::ClientBuilder {
    Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .danger_accept_invalid_certs(false)
}

fn normalize_base_url(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn backup_target_url(base_url: &str) -> String {
    format!("{}{}", normalize_base_url(base_url), REMOTE_BACKUP_FILENAME)
}

/// 本机最后一次上传或下载时云端文件的版本（ETag，没有就用 Last-Modified）。
const REMOTE_STAMP_KEY: &str = "webdav_remote_stamp";

fn request_error(action: &str, error: reqwest::Error) -> String {
    if error.is_timeout() {
        return format!("{action}超时，请检查网络后重试");
    }
    let mut details = Vec::new();
    let mut source = std::error::Error::source(&error);
    while let Some(cause) = source {
        let detail = cause.to_string();
        if !details.contains(&detail) { details.push(detail); }
        source = cause.source();
    }
    if details.is_empty() { details.push(error.without_url().to_string()); }
    format!("{action}失败：{}", details.join("；"))
}

#[derive(Debug, PartialEq, Eq)]
struct RemoteVersion {
    etag: Option<String>,
    modified: Option<String>,
    size: Option<u64>,
}

impl RemoteVersion {
    fn stamp(&self) -> &str {
        self.etag.as_deref().or(self.modified.as_deref()).unwrap_or("")
    }
}

/// Only HTTP 404 means absent. Connection and authorization errors must not
/// turn a failed conflict check into permission to overwrite a cloud backup.
async fn remote_version(client: &Client, target: &str, user: &str, pass: &str) -> Result<Option<RemoteVersion>, String> {
    let mut req = client.head(target);
    if !user.is_empty() {
        req = req.basic_auth(user, Some(pass));
    }
    let resp = req.send().await.map_err(|e| request_error("检查云端备份", e))?;
    if resp.status() == StatusCode::NOT_FOUND { return Ok(None); }
    if !resp.status().is_success() {
        return Err(format!("检查云端备份失败：HTTP {}", resp.status()));
    }
    let header = |name: reqwest::header::HeaderName| resp.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    Ok(Some(RemoteVersion {
        etag: header(reqwest::header::ETAG),
        modified: header(reqwest::header::LAST_MODIFIED),
        // HEAD has no response body; the header describes the remote file.
        size: header(reqwest::header::CONTENT_LENGTH).and_then(|s| s.parse().ok()),
    }))
}

async fn upload_backup(
    client: &Client, target: &str, user: &str, pass: &str,
    bytes: Vec<u8>, expected: Option<&RemoteVersion>,
    channel: Option<&Channel<BackupProgress>>, cancel: &mut watch::Receiver<bool>,
) -> Result<RemoteVersion, String> {
    use ring::rand::SecureRandom;
    let mut nonce = [0u8; 16];
    ring::rand::SystemRandom::new().fill(&mut nonce).map_err(|_| "生成上传文件名失败")?;
    let suffix: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let staging = format!("{target}.upload-{suffix}");
    let size = bytes.len() as u64;
    let request = |method, url: &str| {
        let req = client.request(method, url);
        if user.is_empty() { req } else { req.basic_auth(user, Some(pass)) }
    };
    let result = async {
        // The total duration depends on backup size and link speed. A separate
        // idle watchdog bounds stalls without aborting a progressing upload.
        let transfer = client_builder().build().map_err(|e| e.to_string())?;
        let mut put = transfer.put(&staging)
            .header("If-None-Match", "*")
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", size);
        if !user.is_empty() { put = put.basic_auth(user, Some(pass)); }
        report(channel, "uploading", 0, size);
        let response = send_upload(put, bytes, channel, cancel, UPLOAD_IDLE_TIMEOUT).await?;
        if !response.status().is_success() {
            return Err(format!("WebDAV 服务器拒绝上传：HTTP {}", response.status()));
        }
        report(channel, "verifying", size, size);
        let uploaded = remote_version(client, &staging, user, pass).await?;
        if uploaded.as_ref().and_then(|v| v.size) != Some(size) {
            return Err("云端上传文件长度不完整，未替换原备份，请重试".into());
        }
        if remote_version(client, target, user, pass).await?.as_ref() != expected {
            return Err("云端备份在上传期间发生变化，本次未覆盖，请重新备份并确认冲突".into());
        }
        if *cancel.borrow() { return Err("已取消上传，云端原备份未替换".into()); }
        // MOVE publishes the completed file. Never PUT directly over the backup.
        report(channel, "publishing", size, size);
        let publish = request(Method::from_bytes(b"MOVE").unwrap(), &staging)
            .header("Destination", target)
            .header("Overwrite", if expected.is_some() { "T" } else { "F" });
        let response = publish.send().await.map_err(|e| request_error("发布完整云端备份", e))?;
        if response.status() == StatusCode::PRECONDITION_FAILED {
            return Err("云端备份在上传期间发生变化，本次未覆盖，请重新备份并确认冲突".into());
        }
        if matches!(response.status(), StatusCode::METHOD_NOT_ALLOWED | StatusCode::NOT_IMPLEMENTED) {
            return Err("服务器不支持 WebDAV MOVE，无法安全替换备份；请先导出本地备份".into());
        }
        if !response.status().is_success() {
            return Err(format!("发布完整云端备份失败：HTTP {}，请检查云端版本后重试", response.status()));
        }
        let published = remote_version(client, target, user, pass).await?
            .ok_or("上传后找不到云端备份，请重试")?;
        if published.size != Some(size) {
            return Err("上传后云端备份长度不符，请重新检查云端文件".into());
        }
        Ok(published)
    }.await;
    if result.is_err() {
        // A failed upload can leave a partial staging object. It is never used
        // by restore, even when the network also prevents this cleanup.
        let _ = request(Method::DELETE, &staging).send().await;
    }
    result
}

fn save_remote_stamp(state: &State<'_, AppState>, stamp: &str) {
    if let Ok(conn) = state.db.lock() {
        let _ = conn.execute("INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)", [REMOTE_STAMP_KEY, stamp]);
    }
}

pub(crate) fn get_config_from_db(conn: &rusqlite::Connection) -> (String, String, String) {
    let url = read_app_setting(conn, "webdav_url").unwrap_or_default();
    let user = read_app_setting(conn, "webdav_username").unwrap_or_default();
    let pass = crate::secrets::load_secret("webdav_password", read_app_setting(conn, "webdav_password")).unwrap_or_default();
    (url, user, pass)
}

pub(crate) async fn upload_new_file(config: &(String, String, String), name: &str, bytes: Vec<u8>) -> Result<(), String> {
    let (url, user, pass) = config;
    if url.is_empty() { return Err("请先在设置 → 备份与恢复中配置 WebDAV".into()); }
    let target = format!("{}{name}", normalize_base_url(url));
    let client = build_client()?;
    if remote_version(&client, &target, user, pass).await?.is_some() { return Err("云端已有这个备份版本，无需重复上传".into()); }
    let (_cancel, mut cancellation) = watch::channel(false);
    upload_backup(&client, &target, user, pass, bytes, None, None, &mut cancellation).await.map(|_| ())
}

async fn bounded_body(mut response: reqwest::Response, limit: usize) -> Result<Vec<u8>, String> {
    if !response.status().is_success() { return Err(format!("读取云端备份失败：HTTP {}", response.status())); }
    if response.content_length().is_some_and(|n| n > limit as u64) { return Err("云端响应超过大小限制".into()); }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| request_error("读取云端备份", e))? {
        if bytes.len().saturating_add(chunk.len()) > limit { return Err("云端响应超过大小限制".into()); }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub(crate) async fn download_file(config: &(String, String, String), name: &str, limit: usize) -> Result<Vec<u8>, String> {
    let (url, user, pass) = config;
    if url.is_empty() { return Err("请先配置 WebDAV".into()); }
    let response = build_client()?.get(format!("{}{name}", normalize_base_url(url)))
        .basic_auth(user, Some(pass)).timeout(TRANSFER_TIMEOUT).send().await.map_err(|e| request_error("下载存档", e))?;
    bounded_body(response, limit).await
}

pub(crate) async fn list_files(config: &(String, String, String)) -> Result<Vec<String>, String> {
    let (url, user, pass) = config;
    if url.is_empty() { return Err("请先在设置 → 备份与恢复中配置 WebDAV".into()); }
    let response = build_client()?.request(Method::from_bytes(b"PROPFIND").unwrap(), normalize_base_url(url))
        .basic_auth(user, Some(pass)).header("Depth", "1")
        .header("Content-Type", "application/xml")
        .body(r#"<?xml version="1.0"?><d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/></d:prop></d:propfind>"#)
        .send().await.map_err(|e| request_error("读取云端存档列表", e))?;
    let bytes = bounded_body(response, 4 * 1024 * 1024).await?;
    let base = reqwest::Url::parse(&normalize_base_url(url)).map_err(|e| e.to_string())?;
    let mut reader = quick_xml::Reader::from_reader(bytes.as_slice());
    let mut names = Vec::new();
    loop {
        match reader.read_event().map_err(|e| format!("WebDAV 列表无效：{e}"))? {
            quick_xml::events::Event::Start(tag) if tag.local_name().as_ref() == "href" => {
                let href = reader.read_text(tag.name()).map_err(|e| e.to_string())?;
                let href = quick_xml::escape::unescape(&href).map_err(|e| e.to_string())?;
                let remote = base.join(&href).map_err(|e| e.to_string())?;
                if remote.origin() != base.origin() { continue; }
                if let Some(name) = remote.path().strip_prefix(base.path()).filter(|name| !name.is_empty() && !name.contains('/')) {
                    names.push(name.to_string());
                }
            },
            quick_xml::events::Event::Eof => break,
            _ => {},
        }
    }
    Ok(names)
}

// Tauri Command: 保存 WebDAV 配置
#[tauri::command]
pub fn save_webdav_config(config: WebdavConfig, state: State<AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('webdav_url', ?1)",
        [&config.url.trim()],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('webdav_username', ?1)",
        [&config.username.trim()],
    )
    .map_err(|e| e.to_string())?;

    if let Some(pwd) = config.password {
        if !pwd.is_empty() {
            let mark = crate::secrets::store_secret("webdav_password", &pwd)?;
            conn.execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('webdav_password', ?1)",
                [&mark],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// Tauri Command: 测试 WebDAV 连通性
#[tauri::command]
pub async fn test_webdav_connection(
    url: String,
    username: String,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("WebDAV 服务器 URL 不能为空".into());
    }

    let pass = match password {
        Some(p) if !p.is_empty() => p,
        _ => {
            let conn = state.db.lock().map_err(|e| e.to_string())?;
            get_config_from_db(&conn).2
        }
    };

    let client = build_client()?;
    let base_url = normalize_base_url(url);

    // 尝试发送 PROPFIND 或 GET 检测根目录连通性
    let propfind_method = Method::from_bytes(b"PROPFIND").unwrap_or(Method::GET);
    let mut req = client
        .request(propfind_method, &base_url)
        .header("Depth", "0");
    if !username.is_empty() {
        req = req.basic_auth(&username, Some(&pass));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("连接 WebDAV 服务器失败: {}", e))?;

    let status = resp.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err("身份认证失败：用户名或密码错误 (HTTP 401/403)".into());
    }

    // 如果远端返回 404，尝试自动创建目录 (MKCOL)
    if status == StatusCode::NOT_FOUND {
        let mkcol_method = Method::from_bytes(b"MKCOL").unwrap_or(Method::PUT);
        let mut mkcol_req = client.request(mkcol_method, &base_url);
        if !username.is_empty() {
            mkcol_req = mkcol_req.basic_auth(&username, Some(&pass));
        }
        if let Ok(m_resp) = mkcol_req.send().await {
            if m_resp.status().is_success() || m_resp.status() == StatusCode::METHOD_NOT_ALLOWED {
                return Ok(json!({ "ok": true, "message": "连接成功，已自动初始化远端同步目录" }));
            }
        }
        return Err("远端目录不存在 (HTTP 404)，且无法自动创建，请在网盘先建好该文件夹".into());
    }

    if status.is_success() || status.as_u16() == 207 {
        Ok(json!({ "ok": true, "message": "WebDAV 认证与目录读取正常；实际上传请以备份结果为准" }))
    } else {
        Err(format!("WebDAV 服务器返回状态码: {}", status))
    }
}

// Tauri Command: 获取当前 WebDAV 配置及远端备份状态
#[tauri::command]
pub async fn get_webdav_status(state: State<'_, AppState>) -> Result<WebdavStatus, String> {
    let (url, user, pass) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        get_config_from_db(&conn)
    };
    let last_sync = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        read_app_setting(&conn, "webdav_last_sync_time")
    };

    Ok(read_webdav_status(url, user, pass, last_sync).await)
}

// 本机配置不依赖远端可用性；查询失败必须和「没有备份」分开显示。
async fn read_webdav_status(url: String, user: String, pass: String, last_sync: Option<String>) -> WebdavStatus {
    let remote = if url.is_empty() {
        Ok(None)
    } else {
        match build_client() {
            Ok(client) => remote_version(&client, &backup_target_url(&url), &user, &pass).await,
            Err(error) => Err(error),
        }
    };
    let (remote, remote_error) = match remote {
        Ok(version) => (version, None),
        Err(error) => (None, Some(error)),
    };
    WebdavStatus {
        configured: !url.is_empty(),
        url,
        username: user,
        has_password: !pass.is_empty(),
        last_sync_time: last_sync,
        remote_exists: remote.is_some(),
        remote_size: remote.as_ref().and_then(|v| v.size),
        remote_modified: remote.and_then(|v| v.modified),
        remote_error,
    }
}

// Tauri Command: 上传本地数据库备份至 WebDAV
#[tauri::command]
pub async fn upload_webdav_backup(
    app: AppHandle,
    password: String,
    force: Option<bool>,
    on_progress: Channel<BackupProgress>,
    state: State<'_, AppState>,
    upload: State<'_, BackupUpload>,
) -> Result<serde_json::Value, String> {
    let (cancel, mut cancellation) = watch::channel(false);
    {
        let mut active = upload.0.lock().map_err(|e| e.to_string())?;
        if active.is_some() { return Err("已有备份正在上传，请等待或取消后重试".into()); }
        *active = Some(cancel);
    }
    let _operation = UploadOperation(&upload);
    report(Some(&on_progress), "checking", 0, 0);
    let (url, user, pass, known_stamp) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let (u, n, p) = get_config_from_db(&conn);
        (u, n, p, read_app_setting(&conn, REMOTE_STAMP_KEY))
    };
    if url.is_empty() {
        return Err("尚未配置 WebDAV 服务的 URL".into());
    }

    let client = build_client()?;
    let target = backup_target_url(&url);
    let previous = remote_version(&client, &target, &user, &pass).await?;
    // 冲突检测：云端在本机上次同步之后变化过，先确认再覆盖。
    if !force.unwrap_or(false) {
        if let Some(remote) = &previous {
            if known_stamp.as_deref() != Some(remote.stamp()) {
                return Ok(json!({
                    "conflict": true,
                    "remote_modified": remote.modified,
                    "last_sync_time": state.db.lock().ok().and_then(|c| read_app_setting(&c, "webdav_last_sync_time")),
                }));
            }
        }
    }
    if *cancellation.borrow() { return Err("已取消上传，云端原备份未替换".into()); }

    report(Some(&on_progress), "packing", 0, 0);
    let packing_app = app.clone();
    let (compressed_bytes, raw_len) = tauri::async_runtime::spawn_blocking(move || {
        let state = packing_app.state::<AppState>();
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let raw_len: i64 = conn.query_row("SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()", [], |r| r.get(0)).map_err(|e| e.to_string())?;
        let bytes = crate::backup::encode(&conn, &password, db_file_path(&packing_app)?.parent().ok_or("数据目录不存在")?)?;
        Ok::<_, String>((bytes, raw_len))
    }).await.map_err(|e| e.to_string())??;
    if *cancellation.borrow() { return Err("已取消上传，云端原备份未替换".into()); }
    let comp_len = compressed_bytes.len();

    let published = upload_backup(&client, &target, &user, &pass, compressed_bytes, previous.as_ref(),
        Some(&on_progress), &mut cancellation).await?;
    save_remote_stamp(&state, published.stamp());
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let _ = conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('webdav_last_sync_time', ?1)",
            [&now_str],
        );
    }

    Ok(json!({
        "success": true,
        "raw_size": raw_len,
        "compressed_size": comp_len,
        "sync_time": now_str,
    }))
}

// Tauri Command: 从 WebDAV 下载并还原数据库
#[tauri::command]
pub async fn download_webdav_backup(
    app: AppHandle,
    password: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (url, user, pass) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        get_config_from_db(&conn)
    };
    if url.is_empty() {
        return Err("尚未配置 WebDAV 服务的 URL".into());
    }

    let client = build_client()?;
    let target = backup_target_url(&url);

    let mut req = client.get(&target).timeout(TRANSFER_TIMEOUT);
    if !user.is_empty() {
        req = req.basic_auth(&user, Some(&pass));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| request_error("下载云端备份", e))?;

    if resp.status() == StatusCode::NOT_FOUND {
        return Err(format!("云端未找到备份文件: {}", REMOTE_BACKUP_FILENAME));
    }
    // 恢复后的库里带的是上传方当时的旧标记，这里记下云端当前版本
    let downloaded_stamp = {
        let h = resp.headers();
        let get = |n: reqwest::header::HeaderName| h.get(n).and_then(|v| v.to_str().ok()).map(str::to_string);
        get(reqwest::header::ETAG).or_else(|| get(reqwest::header::LAST_MODIFIED))
    };

    if !resp.status().is_success() {
        return Err(format!(
            "下载失败，状态码: {} ({})",
            resp.status(),
            resp.status().canonical_reason().unwrap_or("未知原因")
        ));
    }

    let gz_bytes = resp
        .bytes()
        .await
        .map_err(|e| request_error("读取云端备份", e))?;

    let restored_len = gz_bytes.len();
    restore_backup(&app, &gz_bytes, password.as_deref().unwrap_or(""), &state)?;

    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let _ = conn.execute(
            "INSERT OR REPLACE INTO app_settings (key, value) VALUES ('webdav_last_sync_time', ?1)",
            [&now_str],
        );
    }
    if let Some(stamp) = downloaded_stamp {
        save_remote_stamp(&state, &stamp);
    }

    Ok(json!({
        "success": true,
        "restored_size": restored_len,
        "sync_time": now_str,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn upload_watchdog_allows_long_progress_but_bounds_stalls_and_cancellation() {
        let (activity, progress) = watch::channel(Instant::now());
        let (_cancel, mut cancellation) = watch::channel(false);
        let transfer = async {
            for _ in 0..12 {
                tokio::time::sleep(Duration::from_secs(60)).await;
                activity.send_replace(Instant::now());
            }
            Ok(())
        };
        let start = Instant::now();
        watch_upload(transfer, progress, &mut cancellation, UPLOAD_IDLE_TIMEOUT).await.unwrap();
        assert!(start.elapsed() > Duration::from_secs(10 * 60));

        let (_activity, progress) = watch::channel(Instant::now());
        let start = Instant::now();
        let stalled = std::future::pending::<Result<(), String>>();
        let error = watch_upload(stalled, progress, &mut cancellation, UPLOAD_IDLE_TIMEOUT).await.unwrap_err();
        assert_eq!(start.elapsed(), UPLOAD_IDLE_TIMEOUT);
        assert!(error.contains("没有传输进展"));

        let (_activity, progress) = watch::channel(Instant::now());
        let (cancel, mut cancellation) = watch::channel(false);
        let transfer = async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            cancel.send_replace(true);
            std::future::pending::<Result<(), String>>().await
        };
        assert!(watch_upload(transfer, progress, &mut cancellation, UPLOAD_IDLE_TIMEOUT).await.unwrap_err().contains("已取消"));
    }

    #[test]
    fn status_retains_local_config_when_remote_query_fails() {
        use std::io::{BufRead, BufReader, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() { break; }
            }
            write!(stream, "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
        });
        let status = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
            read_webdav_status(url.clone(), "test-user".into(), "test-password".into(), Some("2026-11-05 20:00".into()))
        );
        server.join().unwrap();
        assert!(status.configured);
        assert_eq!(status.url, url);
        assert_eq!(status.username, "test-user");
        assert!(status.has_password);
        assert_eq!(status.last_sync_time.as_deref(), Some("2026-11-05 20:00"));
        assert!(status.remote_error.unwrap().contains("401"));
        assert!(!status.remote_exists);
    }
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    // Exercise the HTTP boundary: the live backup must never receive a PUT,
    // and MOVE is allowed only after size and concurrent-version checks pass.
    #[test]
    fn publishes_only_complete_backups_and_cleans_failed_uploads() {
        for failure in ["none", "put", "partial", "changed", "move", "metadata"] {
            let payload = vec![0x7b; 200_000];
            let expected_body = payload.clone();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let target = format!("http://{}/backup", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let mut steps = vec![("PUT", true, if failure == "put" { 503 } else { 201 }, 0, "")];
                if failure != "put" {
                    steps.push(("HEAD", true, 200, if failure == "partial" { 2 } else { 200_000 }, "new"));
                    if failure != "partial" {
                        steps.push(("HEAD", false, if failure == "metadata" { 401 } else { 200 }, 3,
                            if failure == "changed" { "other" } else { "old" }));
                        if !matches!(failure, "changed" | "metadata") {
                            steps.push(("MOVE", true, if failure == "move" { 405 } else { 204 }, 0, ""));
                            if failure == "none" { steps.push(("HEAD", false, 200, 200_000, "new")); }
                        }
                    }
                }
                if failure != "none" { steps.push(("DELETE", true, 204, 0, "")); }
                let mut staging = None;
                for (method, is_staging, status, size, etag) in steps {
                    let deadline = std::time::Instant::now() + Duration::from_secs(5);
                    let mut stream = loop {
                        match listener.accept() {
                            Ok((stream, _)) => break stream,
                            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && std::time::Instant::now() < deadline =>
                                std::thread::sleep(Duration::from_millis(5)),
                            Err(e) => panic!("missing {method}: {e}"),
                        }
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let request: Vec<_> = line.split_whitespace().collect();
                    assert_eq!(request[0], method, "{failure} expected response status={status} size={size}");
                    let path = request[1].to_string();
                    if is_staging {
                        assert!(path.starts_with("/backup.upload-"), "{path}");
                        assert_eq!(staging.get_or_insert(path.clone()), &path);
                    } else { assert_eq!(path, "/backup"); }
                    let mut length = 0usize;
                    let mut destination = None;
                    loop {
                        let mut header = String::new();
                        reader.read_line(&mut header).unwrap();
                        if header == "\r\n" { break; }
                        let (key, value) = header.split_once(':').unwrap();
                        if key.eq_ignore_ascii_case("content-length") { length = value.trim().parse().unwrap(); }
                        if key.eq_ignore_ascii_case("destination") { destination = Some(value.trim().to_string()); }
                    }
                    let mut body = vec![0; length];
                    reader.read_exact(&mut body).unwrap();
                    if method == "PUT" { assert_eq!(body, expected_body); }
                    if method == "MOVE" { assert!(destination.unwrap().ends_with("/backup")); }
                    write!(stream, "HTTP/1.1 {status} Test\r\nContent-Length: {size}\r\nETag: \"{etag}\"\r\nConnection: close\r\n\r\n").unwrap();
                }
            });
            let result = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let previous = RemoteVersion { etag: Some("\"old\"".into()), modified: None, size: Some(3) };
                let (_cancel, mut cancellation) = watch::channel(false);
                upload_backup(&build_client().unwrap(), &target, "", "", payload, Some(&previous), None, &mut cancellation).await
            });
            assert!(server.join().is_ok(), "{failure}: {result:?}");
            assert_eq!(result.is_ok(), failure == "none", "{failure}: {result:?}");
            if failure == "metadata" { assert!(result.unwrap_err().contains("401")); }
        }
    }
}
