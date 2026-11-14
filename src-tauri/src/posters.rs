//! 图片本地盘缓存：书库海报（`img` 协议 `?poster=1&u=…`，存 `poster-cache/`）和光遇 wiki/Fandom
//! 图标、实穿图（存 `wiki-cache/`）。第一次显示时下载到 `<dir>/<hash>.<ext>`，之后直接读盘，
//! 离线也能看，滚动时不再一张张跳出来。

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn client() -> Result<reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client,String>> = OnceLock::new();
    CLIENT.get_or_init(|| reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .connect_timeout(crate::PUBLIC_CONNECT_TIMEOUT)
        .timeout(crate::PUBLIC_REQUEST_TIMEOUT)
        .build().map_err(|e|e.to_string())).clone()
}

/// FNV-1a，文件名只要求稳定
fn url_hash(url: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn mime_of(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/jpeg",
    }
}

fn ext_for(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "jpg",
    }
}

fn cached(dir: &Path, key: &str) -> Option<PathBuf> {
    ["jpg", "png", "webp", "gif"].iter().map(|e| dir.join(format!("{key}.{e}"))).find(|p| p.exists())
}

/// 返回 (mime, bytes)。命中缓存读盘，否则下载并写盘。`referer` 给要求来源页的图床（Fandom）。
pub(crate) async fn load_poster(cache_dir: &Path, url: &str, referer: Option<&str>) -> Result<(String, Vec<u8>), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("图片地址不是网络图片".into());
    }
    let key = url_hash(url);
    if let Some(p) = cached(cache_dir, &key) {
        let bytes = std::fs::read(&p).map_err(|e| e.to_string())?;
        return Ok((mime_of(&p).to_string(), bytes));
    }
    // 同一进程复用连接池，并限制首次封面下载，避免整页同时新建 TLS 连接。
    static DOWNLOADS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
    let _permit = DOWNLOADS.acquire().await.map_err(|e|e.to_string())?;
    let mut req = client()?.get(url);
    if let Some(r) = referer {
        req = req.header(reqwest::header::REFERER, r);
    }
    let resp = crate::public_http::send(req).await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("poster {}", resp.status()));
    }
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .split(';')
        .next()
        .unwrap_or("image/jpeg")
        .trim()
        .to_string();
    if !mime.starts_with("image/") {
        return Err("not an image".into());
    }
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?.to_vec();
    let _ = std::fs::create_dir_all(cache_dir);
    // 先写临时文件再改名，半截文件不会被当成缓存
    let path = cache_dir.join(format!("{key}.{}", ext_for(&mime)));
    let tmp = path.with_extension("part");
    if std::fs::write(&tmp, &bytes).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
    Ok((mime, bytes))
}

/// 目录超过 `max_bytes` 时从最久没改过的文件删起，删到一半为止。启动时调一次。
pub(crate) fn prune_dir(dir: &Path, max_bytes: u64) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = rd
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            meta.is_file().then(|| (meta.modified().unwrap_or(std::time::UNIX_EPOCH), meta.len(), e.path()))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    if total <= max_bytes {
        return;
    }
    files.sort_by_key(|f| f.0);
    for (_, len, path) in files {
        if total <= max_bytes / 2 {
            break;
        }
        if std::fs::remove_file(&path).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_stable_and_cache_hit_reads_disk() {
        assert_eq!(url_hash("https://t.vndb.org/cv/91/88791.jpg"), url_hash("https://t.vndb.org/cv/91/88791.jpg"));
        assert_ne!(url_hash("a"), url_hash("b"));
        let dir = std::env::temp_dir().join(format!("wp-poster-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let url = "https://example.invalid/p.png";
        std::fs::write(dir.join(format!("{}.png", url_hash(url))), b"PNGDATA").unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let (mime, bytes) = rt.block_on(load_poster(&dir, url, None)).unwrap();
        assert_eq!((mime.as_str(), bytes.as_slice()), ("image/png", b"PNGDATA".as_slice()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
