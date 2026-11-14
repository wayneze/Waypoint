use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ExploreHit {
    pub title: String,
    pub url: String,
    /// 资源文件大小，例如 1.2 GB。没有就空。
    pub detail: String,
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36")
        .cookie_store(true)
        .connect_timeout(crate::PUBLIC_CONNECT_TIMEOUT)
        .timeout(crate::PUBLIC_REQUEST_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())
}

fn search_error(error: reqwest::Error) -> String {
    if error.is_timeout() { return "搜索超时，请稍后重试或在网站查看".into(); }
    if error.is_connect() { return "无法连接站点，请检查网络或在浏览器中打开".into(); }
    "读取站点结果失败，请稍后重试或在网站查看".into()
}

fn origin_of(home: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(home).map_err(|e| format!("站点地址无效: {}", e))?;
    let host = url.host_str().ok_or_else(|| "站点地址没有域名".to_string())?;
    let port = url.port().map(|p| format!(":{}", p)).unwrap_or_default();
    Ok(format!("{}://{}{}", url.scheme(), host, port))
}

fn json_body(text: &str) -> Result<Value, String> {
    let start = text.find(['{', '[']).ok_or_else(|| "没有结果".to_string())?;
    let end = text.rfind(['}', ']']).ok_or_else(|| "没有结果".to_string())?;
    serde_json::from_str(&text[start..=end]).map_err(|e| format!("结果解析失败: {}", e))
}

fn decode_title(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        if let Some(end) = rest.find(';') {
            let token = &rest[..=end];
            let decoded = if let Some(num) = token.strip_prefix("&#").and_then(|s| s.strip_suffix(';')) {
                num.parse::<u32>().ok().and_then(char::from_u32).map(|c| c.to_string())
            } else {
                match token {
                    "&amp;" => Some("&".to_string()),
                    "&lt;" => Some("<".to_string()),
                    "&gt;" => Some(">".to_string()),
                    "&quot;" => Some("\"".to_string()),
                    "&nbsp;" => Some(" ".to_string()),
                    _ => None,
                }
            };
            if let Some(text) = decoded {
                out.push_str(&text);
                rest = &rest[end + 1..];
                continue;
            }
        }
        out.push('&');
        rest = &rest[1..];
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn wp_hits(value: &Value) -> Vec<ExploreHit> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let url = row.get("link")?.as_str()?.trim();
            let title = row.get("title")?.get("rendered")?.as_str()?;
            if !url.starts_with("http") { return None; }
            Some(ExploreHit { title: decode_title(title), url: url.to_string(), detail: String::new() })
        })
        .take(4)
        .collect()
}

fn bangumi_hits(value: &Value) -> Vec<ExploreHit> {
    value
        .get("list")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let url = row.get("url")?.as_str()?.trim().replace("http://", "https://");
            let cn = row.get("name_cn").and_then(Value::as_str).unwrap_or("").trim();
            let name = row.get("name").and_then(Value::as_str).unwrap_or("").trim();
            let title = if cn.is_empty() { name } else { cn };
            if title.is_empty() || !url.starts_with("http") { return None; }
            Some(ExploreHit { title: title.to_string(), url, detail: String::new() })
        })
        .take(4)
        .collect()
}

fn dfan_hits(body: &str) -> Vec<ExploreHit> {
    let mut hits = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find("subject-card-title") {
        rest = &rest[pos..];
        let Some(href_at) = rest.find("href=\"") else { break };
        let href_body = &rest[href_at + 6..];
        let Some(href_end) = href_body.find('"') else { break };
        let href = &href_body[..href_end];
        let Some(close) = href_body.find("</a>") else { break };
        let raw = &href_body[..close];
        let title = decode_title(&strip_tags(&raw[href_end + 1..]));
        if href.starts_with("/subjects/") && !title.is_empty() {
            hits.push(ExploreHit {
                title,
                url: format!("https://2dfan.com{}", href),
                detail: String::new(),
            });
        }
        rest = &href_body[close..];
        if hits.len() == 4 { break; }
    }
    hits
}

fn strip_tags(raw: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for ch in raw.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => out.push(ch),
            _ => {}
        }
    }
    out
}

fn galgames_hits(body: &str, home: &str) -> Result<Vec<ExploreHit>, String> {
    let origin = origin_of(home)?;
    let mut hits = Vec::new();
    for heading in body.split("<h2").skip(1) {
        let Some((heading, _)) = heading.split_once("</h2>") else { continue };
        let Some((_, link)) = heading.split_once("href=\"") else { continue };
        let Some((href, rest)) = link.split_once('"') else { continue };
        if !href.starts_with("/games/") { continue; }
        let Some((_, text)) = rest.split_once('>') else { continue };
        let title = decode_title(&strip_tags(text));
        if !title.is_empty() {
            hits.push(ExploreHit { title, url: format!("{}{}", origin, decode_title(href)), detail: String::new() });
        }
        if hits.len() == 6 { break; }
    }
    if hits.is_empty() && !body.contains("没有匹配") {
        return Err("站点页面已变化，暂时无法读取结果，请前往网站搜索".into());
    }
    Ok(hits)
}

fn seiya_hits(body: &str, query: &str) -> Vec<ExploreHit> {
    let key = |text: &str| text.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>();
    let needle = key(query);
    if needle.is_empty() { return Vec::new(); }
    let document = scraper::Html::parse_document(body);
    let selector = scraper::Selector::parse("a[href]").unwrap();
    let base = reqwest::Url::parse("https://seiya-saiga.com/game/").unwrap();
    document.select(&selector).filter_map(|a| {
        let title = a.text().collect::<String>();
        if !key(&title).contains(&needle) { return None; }
        let url = base.join(a.value().attr("href")?).ok()?;
        if !matches!(url.host_str(), Some("seiya-saiga.com" | "galge.seiya-saiga.com")) || !url.path().ends_with(".html") { return None; }
        Some(ExploreHit { title: decode_title(&title), url: url.to_string(), detail: String::new() })
    }).take(6).collect()
}

fn loli_hits(body: &str) -> Vec<ExploreHit> {
    let mut hits = Vec::new();
    let mut rest = body;
    while let Some(pos) = rest.find("post-title") {
        rest = &rest[pos..];
        let Some(href_at) = rest.find("href=\"") else { break };
        let href_body = &rest[href_at + 6..];
        let Some(href_end) = href_body.find('"') else { break };
        let href = href_body[..href_end].trim();
        let Some(close) = href_body.find("</a>") else { break };
        let title = decode_title(&strip_tags(&href_body[href_end + 1..close]));
        if href.starts_with("http") && !title.is_empty() {
            hits.push(ExploreHit { title, url: href.to_string(), detail: String::new() });
        }
        rest = &href_body[close..];
        if hits.len() == 6 { break; }
    }
    hits
}

fn check_target(body: &str) -> Option<String> {
    let marker = "window.location=";
    let at = body.find(marker)?;
    let mut target = String::new();
    let mut rest = &body[at + marker.len()..];
    while let Some(start) = rest.find('"') {
        let piece = &rest[start + 1..];
        let end = piece.find('"')?;
        target.push_str(&piece[..end]);
        rest = &piece[end + 1..];
        let trimmed = rest.trim_start();
        if !trimmed.starts_with('+') { break; }
        rest = &trimmed[1..];
    }
    if target.starts_with('/') { Some(target) } else { None }
}

fn qingju_hits(value: &Value, home: &str) -> Vec<ExploreHit> {
    let origin = origin_of(home).unwrap_or_else(|_| "https://x.qingju.org".to_string());
    value
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let title = row.get("title")?.as_str()?.trim();
            let id = row.get("id")?.as_u64()?;
            if title.is_empty() { return None; }
            let downloads = row.get("resource_download_count").and_then(Value::as_u64);
            let detail = downloads.map(|n| format!("{} 次下载", n)).unwrap_or_default();
            Some(ExploreHit {
                title: title.to_string(),
                url: format!("{}/zh-CN/articles/view?id={}", origin, id),
                detail,
            })
        })
        .take(6)
        .collect()
}

fn touch_hits(value: &Value) -> Vec<ExploreHit> {
    value
        .get("galgames")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let name = row.get("name")?.as_str()?.trim();
            let uid = row.get("uniqueId")?.as_str()?.trim();
            if name.is_empty() || uid.is_empty() { return None; }
            let resources = row.get("_count").and_then(|c| c.get("resource")).and_then(Value::as_u64);
            let detail = resources.map(|n| format!("{} 个资源", n)).unwrap_or_default();
            Some(ExploreHit {
                title: name.to_string(),
                url: format!("https://www.touchgal.ink/{}", uid),
                detail,
            })
        })
        .take(6)
        .collect()
}

fn steam_hits(value: &Value, guides: bool) -> Vec<ExploreHit> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let name = row.get("name")?.as_str()?.trim();
            let id = row.get("appid")?.as_str()?.parse::<u64>().ok()?;
            if name.is_empty() { return None; }
            Some(ExploreHit {
                title: name.to_string(),
                url: if guides { format!("https://steamcommunity.com/app/{}/guides/", id) } else { format!("https://store.steampowered.com/app/{}/", id) },
                detail: String::new(),
            })
        })
        .take(4)
        .collect()
}

fn juij_hits(value: &Value, query: &str, home: &str) -> Result<Vec<ExploreHit>, String> {
    let base = reqwest::Url::parse(&origin_of(home)?).map_err(|e| e.to_string())?;
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut hits = Vec::new();
    for row in value.as_array().into_iter().flatten() {
        let Some(title) = row.get("title").and_then(Value::as_str) else { continue };
        let names = format!("{} {} {}", title, row.get("tags").unwrap_or(&Value::Null), row.get("search_keywords").unwrap_or(&Value::Null)).to_lowercase();
        if !words.iter().all(|word| names.contains(word)) { continue; }
        let Some(path) = row.get("permalink").and_then(Value::as_str) else { continue };
        let Ok(url) = base.join(path) else { continue };
        if url.origin() != base.origin() { continue; }
        hits.push(ExploreHit {
            title: decode_title(title),
            url: url.into(),
            detail: row.get("description").and_then(Value::as_str).unwrap_or("").to_string(),
        });
        if hits.len() == 6 { break; }
    }
    Ok(hits)
}

pub(crate) fn parse_explore_hits(kind: &str, body: &str, query: &str, home: &str) -> Result<Vec<ExploreHit>, String> {
    if body.contains("Just a moment") || body.contains("Checking your browser") || body.contains("window.location=") {
        return Err("站点要求浏览器验证，请前往网站搜索".into());
    }
    if kind == "galgames" { return galgames_hits(body, home); }
    if kind == "dfan" || kind == "dfan-guides" {
        let mut hits = dfan_hits(body);
        if kind == "dfan-guides" {
            for hit in &mut hits {
                hit.url.push_str("/walkthroughs");
                hit.detail = "查看此作品的中文攻略列表".into();
            }
        }
        return Ok(hits);
    }
    if kind == "seiya" {
        return Ok(seiya_hits(body, query));
    }
    if kind == "loli" {
        return Ok(loli_hits(body));
    }
    let value = json_body(body)?;
    let valid = match kind {
        "wp" | "juij" => value.is_array(),
        "bangumi" => value.get("list").is_some_and(Value::is_array) || value.get("results").and_then(Value::as_u64) == Some(0),
        "steam" | "steam-guides" => value.is_array(),
        "touch" => value.get("galgames").is_some_and(Value::is_array),
        "qingju" => value.get("data").is_some_and(Value::is_array),
        _ => return Err("这个站没有结果接口".into()),
    };
    if !valid { return Err("站点没有返回有效搜索结果，请前往网站查看".into()); }
    Ok(match kind {
        "juij" => return juij_hits(&value, query, home),
        "wp" => wp_hits(&value),
        "bangumi" => bangumi_hits(&value),
        "steam" | "steam-guides" => steam_hits(&value, kind == "steam-guides"),
        "touch" => touch_hits(&value),
        "qingju" => qingju_hits(&value, home),
        _ => return Err("这个站没有结果接口".to_string()),
    })
}

#[tauri::command]
pub async fn search_explore_site(
    kind: String, query: String, home: String,
    juij: tauri::State<'_, crate::juij::JuijClient>, cache: tauri::State<'_, crate::cache::CacheDb>,
) -> Result<Vec<ExploreHit>, String> {
    let q = query.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    if kind == "juij" {
        let url = format!("{}/search/index.json", origin_of(&home)?);
        let resource = juij.read(&url, true, &cache).await.map_err(|e| e.message())?;
        let body = std::str::from_utf8(&resource.bytes).map_err(|_| "站点索引编码无效")?;
        return parse_explore_hits("juij", body, q, &home);
    }
    if kind == "seiya" {
        let key = "explore:https://seiya-saiga.com/game/kouryaku.html";
        let now = chrono::Utc::now().timestamp_millis();
        let cached: Option<String> = {
            let conn = cache.0.lock().map_err(|e|e.to_string())?;
            conn.query_row("SELECT body FROM fetch_cache WHERE key=?1 AND fetched_at>?2", rusqlite::params![key, now - 86_400_000], |r|r.get(0)).ok()
        };
        let body = match cached {
            Some(body) => body,
            None => {
                let response = crate::public_http::send(client()?.get("https://seiya-saiga.com/game/kouryaku.html")).await.map_err(search_error)?.error_for_status().map_err(search_error)?;
                let bytes = response.bytes().await.map_err(search_error)?;
                let body = encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned();
                if !body.contains(".html") || body.len() < 1000 { return Err("誠也攻略索引返回异常，请查看原网站".into()); }
                let conn = cache.0.lock().map_err(|e|e.to_string())?;
                crate::cache::put(&conn, key, &body, now).map_err(|e|e.to_string())?;
                body
            }
        };
        return parse_explore_hits("seiya", &body, q, &home);
    }
    let client = client()?;
    let encoded = urlencoding_query(q);
    if kind == "qingju" {
        let _ = client.get(origin_of(&home)?).send().await;
    }
    let url = match kind.as_str() {
        "wp" => format!(
            "{}/wp-json/wp/v2/posts?search={}&per_page=4&_fields=link,title",
            origin_of(&home)?,
            encoded
        ),
        "bangumi" => format!(
            "https://api.bgm.tv/search/subject/{}?type=4&responseGroup=small&max_results=4",
            encoded
        ),
        "steam" | "steam-guides" => format!(
            "https://steamcommunity.com/actions/SearchApps/{}",
            encoded
        ),
        "galgames" => format!("{}/search?q={}", origin_of(&home)?, encoded),
        "qingju" => format!("{}/api/v1/articles?keyword={}&page=1&page_size=6&search_title=true&search_alias=true&search_vndb=true&show_nsfw=true", origin_of(&home)?, encoded),
        "touch" => "https://www.touchgal.ink/api/search".into(),
        "dfan" | "dfan-guides" => format!("https://2dfan.com/search?keyword={}", encoded),
        "loli" => format!("{}/?s={}", origin_of(&home)?, encoded),
        _ => return Err("这个站没有结果接口".to_string()),
    };
    let request = if kind == "touch" {
        client.post(&url)
            .header("Origin", "https://www.touchgal.ink")
            .header("Referer", "https://www.touchgal.ink/search")
            .json(&serde_json::json!({
                "queryString": serde_json::json!([{"type":"keyword","mode":"include","name":q}]).to_string(),
                "limit":6, "searchOption":{"searchInIntroduction":false,"searchInAlias":true,"searchInTag":false},
                "page":1, "selectedType":"all", "selectedLanguage":"all", "selectedPlatform":"all",
                "sortField":"resource_update_time", "sortOrder":"desc", "selectedYears":["all"], "selectedMonths":["all"], "minRatingCount":0
            }))
    } else { client.get(&url) };
    let res = crate::public_http::send(request)
        .await
        .map_err(search_error)?;
    if !res.status().is_success() {
        return Err(match res.status().as_u16() {
            403 => "站点限制自动访问，请在浏览器中打开并完成验证".into(),
            429 => "站点请求过于频繁，请稍后重试".into(),
            404 => "站点搜索地址已失效，请前往首页查看".into(),
            code => format!("站点暂时无法访问（HTTP {}），请稍后重试", code),
        });
    }
    let mut text = res.text().await.map_err(search_error)?;
    if kind == "loli" {
        if let Some(target) = check_target(&text) {
            let next = crate::public_http::send(client
                .get(format!("{}{}", origin_of(&home)?, target)))
                .await
                .map_err(search_error)?;
            if !next.status().is_success() {
                return Err(format!("没取到: HTTP {}", next.status().as_u16()));
            }
            text = next.text().await.map_err(search_error)?;
        }
    }
    parse_explore_hits(&kind, &text, q, &home)
}

fn urlencoding_query(q: &str) -> String {
    let mut out = String::new();
    for b in q.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "read-only live guide source acceptance requires network"]
    async fn live_guide_sources() {
        let index=crate::public_http::send(client().unwrap().get("https://seiya-saiga.com/game/kouryaku.html")).await.unwrap().error_for_status().unwrap().bytes().await.unwrap();
        let body=encoding_rs::SHIFT_JIS.decode(&index).0.into_owned();
        let seiya=parse_explore_hits("seiya",&body,"STEINS;GATE","").unwrap();
        assert!(!seiya.is_empty());
        let response=crate::public_http::send(client().unwrap().get("https://2dfan.com/search").query(&[("keyword","命运石之门")])).await.unwrap().error_for_status().unwrap();
        let dfan=parse_explore_hits("dfan-guides",&response.text().await.unwrap(),"命运石之门","").unwrap();
        assert!(!dfan.is_empty());
        println!("Guide results: seiya={} 2dfan={}",seiya.len(),dfan.len());
        if let Ok(path)=std::env::var("WAYPOINT_GUIDE_OUTPUT") {
            std::fs::write(path,serde_json::to_string(&serde_json::json!({"seiya":seiya,"dfan-guides":dfan})).unwrap()).unwrap();
        }
    }

    #[test]
    fn juij_search_matches_titles_and_aliases_without_unrelated_body_text() {
        let body = r#"[{"title":"SCSKiller 汉化","permalink":"/zh/scskiller/","description":"更新 v1.4.5","tags":["Shader Compilation Stutter Killer"]},{"title":"别的工具","permalink":"/zh/other/","content":"SCSKiller"},{"title":"SCSKiller","permalink":"https://other.example/"}]"#;
        let hits = parse_explore_hits("juij", body, "scskiller", "https://h.juij.fun/").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://h.juij.fun/zh/scskiller/");
        assert_eq!(hits[0].detail, "更新 v1.4.5");
        assert_eq!(parse_explore_hits("juij", body, "shader killer", "https://h.juij.fun/").unwrap().len(), 1);
        for url in ["https://h.juij.fun.evil.test/", "file:///tmp/image", "https://user:pass@juij.fun/", "https://juij.fun:8443/"] {
            assert!(!crate::juij::is_juij_url(&reqwest::Url::parse(url).unwrap()));
        }
    }

    #[test]
    fn wp_body_ignores_trailing_comment_and_entities() {
        let body = r#"[{"link":"https://www.ttloli.com/fu.html","title":{"rendered":"Fate&#8211;stay night"}}] <!--压缩前-->"#;
        let hits = parse_explore_hits("wp", body, "", "").unwrap();
        assert_eq!(hits[0].title, "Fate–stay night");
        assert_eq!(hits[0].url, "https://www.ttloli.com/fu.html");
    }

    #[test]
    fn loli_reads_post_titles_after_check() {
        let gate = r#"<script>window.location="/?s=Fate" + "&ok=1";</script>"#;
        assert_eq!(check_target(gate).as_deref(), Some("/?s=Fate&ok=1"));
        let page = r#"<span class="post-title"><a href="https://www.ttloli.com/fu.html"> 腐姬 </a></span>"#;
        let hits = parse_explore_hits("loli", page, "Fate", "").unwrap();
        assert_eq!(hits[0].title, "腐姬");
        assert_eq!(hits[0].url, "https://www.ttloli.com/fu.html");
    }

    #[test]
    fn touch_reads_unique_id() {
        let body = r#"{"galgames":[{"name":"Fate/stay night","uniqueId":"d018ec0f","_count":{"resource":17}}]}"#;
        let hits = parse_explore_hits("touch", body, "", "").unwrap();
        assert_eq!(hits[0].url, "https://www.touchgal.ink/d018ec0f");
        assert_eq!(hits[0].detail, "17 个资源");
    }

    #[test]
    fn dfan_reads_subject_card_titles() {
        let body = r#"<h3 class="subject-card-title"><a href="/subjects/18959">Fate/stay night REMASTERED</a></h3>"#;
        let hits = parse_explore_hits("dfan", body, "", "").unwrap();
        assert_eq!(hits[0].url, "https://2dfan.com/subjects/18959");
        assert_eq!(hits[0].title, "Fate/stay night REMASTERED");
        assert_eq!(parse_explore_hits("dfan-guides", body, "", "").unwrap()[0].url, "https://2dfan.com/subjects/18959/walkthroughs");
    }

    #[test]
    fn seiya_matches_guide_title() {
        let body = r#"<A href="typemoon/fate.html">Fate/stay night</A><A href="https://galge.seiya-saiga.com/game/nitroplus/steinsgate.html">STEINS;GATE</A><A href="yuzu-soft/senrenbanka.html">千恋＊万花</A>"#;
        let hits = parse_explore_hits("seiya", body, "Fate", "").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://seiya-saiga.com/game/typemoon/fate.html");
        assert_eq!(parse_explore_hits("seiya", body, "STEINS；GATE", "").unwrap()[0].url, "https://galge.seiya-saiga.com/game/nitroplus/steinsgate.html");
        assert_eq!(parse_explore_hits("seiya", body, "千恋万花", "").unwrap()[0].title, "千恋＊万花");
    }

    #[test]
    fn current_site_results_and_failed_responses_are_distinct() {
        let html = r#"<h2><a href="/games/13167?q=Fate"><mark>Fate</mark>/stay night</a></h2>"#;
        let hits = parse_explore_hits("galgames", html, "Fate", "https://gal.games/").unwrap();
        assert_eq!(hits[0].title, "Fate/stay night");
        assert_eq!(hits[0].url, "https://gal.games/games/13167?q=Fate");
        let steam = r#"[{"appid":"620","name":"Portal 2"}]"#;
        assert_eq!(parse_explore_hits("steam-guides", steam, "Portal", "").unwrap()[0].url, "https://steamcommunity.com/app/620/guides/");
        for (kind, body) in [("qingju", r#"{"error":"blocked"}"#), ("steam", "{}"), ("loli", r#"<script>window.location="/check";</script>"#)] {
            assert!(parse_explore_hits(kind, body, "Fate", "").is_err());
        }
        assert!(parse_explore_hits("steam", "[]", "absent", "").unwrap().is_empty());
    }

    #[test]
    fn qingju_reads_article_id() {
        let body = r#"{"data":[{"id":944,"title":"沙耶之歌","resource_download_count":277}],"meta":{"total":1}}"#;
        let hits = parse_explore_hits("qingju", body, "沙耶", "https://x.qingju.org/").unwrap();
        assert_eq!(hits[0].title, "沙耶之歌");
        assert_eq!(hits[0].url, "https://x.qingju.org/zh-CN/articles/view?id=944");
        assert_eq!(hits[0].detail, "277 次下载");
    }

    #[test]
    fn bangumi_prefers_chinese_name() {
        let body = r#"{"list":[{"url":"http://bgm.tv/subject/16","name":"Fate/stay night","name_cn":"命运石之门"}]}"#;
        let hits = parse_explore_hits("bangumi", body, "", "").unwrap();
        assert_eq!(hits[0].title, "命运石之门");
        assert_eq!(hits[0].url, "https://bgm.tv/subject/16");
    }
}
