use serde::{Deserialize, Serialize};
use tauri::State;
use crate::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObsidianFile {
    pub name: String,
    pub relative_path: String,
    pub full_path: String,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObsidianGameGuide {
    pub series: Option<String>,
    pub series_order: Option<f64>,
    pub id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub aliases: Vec<String>,
    pub release_date: Option<String>,
    pub year: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub category: String,
    pub status: String,
    pub rating_vndb: Option<f64>,
    pub rating_personal: Option<f64>,
    pub rating_bangumi: Option<f64>,
    pub rating: f64,
    pub routes_total: Option<u32>,
    pub routes_cleared: Option<u32>,
    pub progress_note: Option<String>,
    pub tags: Vec<String>,
    pub genres: Vec<String>,
    pub file_path: String,
    pub relative_path: String,
    pub file_name: String,
    pub modified: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
    /// 笔记前言 `links:` 里的直链。口碑搜索优先打开这里，不再拿标题去搜。
    pub links: std::collections::HashMap<String, String>,
}

// Tauri Commands: 扫描真实 Obsidian 目录下的 Markdown 笔记
#[tauri::command]
pub fn scan_obsidian_vault(vault_path: String) -> Result<Vec<ObsidianFile>, String> {
    let base_path = std::path::Path::new(&vault_path);
    if !base_path.exists() || !base_path.is_dir() {
        return Err(format!("指定路径不存在或非文件夹: {}", vault_path));
    }

    let mut files = Vec::new();
    fn visit_dirs(dir: &std::path::Path, base: &std::path::Path, files: &mut Vec<ObsidianFile>, depth: usize) {
        if depth > 6 { return; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.starts_with('.') {
                    continue; // 忽略 .obsidian, .git 等隐藏系统目录
                }
                if path.is_dir() {
                    if file_name.eq_ignore_ascii_case("Waypoint") {
                        continue;
                    }
                    visit_dirs(&path, base, files, depth + 1);
                } else if path.is_file() && path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("md")).unwrap_or(false) {
                    let relative = path.strip_prefix(base).unwrap_or(&path).to_string_lossy().to_string();
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let modified = metadata
                        .and_then(|m| m.modified().ok())
                        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "-".to_string());

                    files.push(ObsidianFile {
                        name: file_name,
                        relative_path: relative,
                        full_path: path.to_string_lossy().to_string(),
                        size,
                        modified,
                    });
                }
            }
        }
    }

    visit_dirs(base_path, base_path, &mut files, 0);
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    files.truncate(100);
    Ok(files)
}

// Tauri Commands: 读取指定的 Markdown 笔记文本
#[tauri::command]
pub fn read_obsidian_file(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

// Tauri Commands: 安全更新 Markdown 笔记的 Frontmatter 元数据 (如状态 status, 已通关路线 routes.cleared)
#[tauri::command]
pub fn update_guide_frontmatter(
    file_path: String,
    status: Option<String>,
    routes_cleared: Option<u32>,
    route_title: Option<String>,
    route_completed: Option<bool>,
) -> Result<(), String> {
    let p = std::path::Path::new(&file_path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    let content = std::fs::read_to_string(p).map_err(|e| format!("读取文件失败: {}", e))?;

    let (fm_str, mut body, has_fm, line_ending) = if let Some(after) = content.strip_prefix("---\n") {
        if let Some(end_pos) = after.find("\n---") {
            let fm = &after[..end_pos];
            let rest = &after[end_pos + 4..];
            (fm.to_string(), rest.to_string(), true, "\n")
        } else {
            ("".to_string(), content.clone(), false, "\n")
        }
    } else if let Some(after) = content.strip_prefix("---\r\n") {
        if let Some(end_pos) = after.find("\r\n---") {
            let fm = &after[..end_pos];
            let rest = &after[end_pos + 5..];
            (fm.to_string(), rest.to_string(), true, "\r\n")
        } else {
            ("".to_string(), content.clone(), false, "\r\n")
        }
    } else {
        ("".to_string(), content.clone(), false, "\n")
    };

fn extract_numeral_str(s: &str) -> Option<u32> {
    if s.contains('Ⅰ') { return Some(1); }
    if s.contains('Ⅱ') { return Some(2); }
    if s.contains('Ⅲ') { return Some(3); }
    if s.contains('Ⅳ') { return Some(4); }
    if s.contains('Ⅴ') { return Some(5); }
    if s.contains('Ⅵ') { return Some(6); }
    if s.contains('Ⅶ') { return Some(7); }
    if s.contains('Ⅷ') { return Some(8); }
    if s.contains('Ⅸ') { return Some(9); }
    if s.contains('Ⅹ') { return Some(10); }

    for token in s.split(|c: char| !c.is_alphanumeric()) {
        match token.to_uppercase().as_str() {
            "I" | "1" => return Some(1),
            "II" | "2" => return Some(2),
            "III" | "3" => return Some(3),
            "IV" | "4" => return Some(4),
            "V" | "5" => return Some(5),
            "VI" | "6" => return Some(6),
            "VII" | "7" => return Some(7),
            "VIII" | "8" => return Some(8),
            "IX" | "9" => return Some(9),
            "X" | "10" => return Some(10),
            _ => {}
        }
    }
    None
}

fn clean_route_title_str(s: &str) -> String {
    let mut res = s.to_string();
    if let Some(pos) = res.find("｜折叠") {
        res.truncate(pos);
    }
    if let Some(pos) = res.find("| 折叠") {
        res.truncate(pos);
    }
    if let Some(pos) = res.find('｜') {
        res.truncate(pos);
    }

    // 剔除括号及其内部修饰说明，如 (Hill of Sunflower)、（由岐真实视点）等
    let mut out = String::new();
    let mut paren_depth = 0;
    for c in res.chars() {
        match c {
            '(' | '（' | '【' | '[' => {
                paren_depth += 1;
                out.push(' ');
            }
            ')' | '）' | '】' | ']' => {
                if paren_depth > 0 {
                    paren_depth -= 1;
                }
                out.push(' ');
            }
            _ => {
                if paren_depth == 0 {
                    out.push(c);
                }
            }
        }
    }
    out.trim().to_string()
}

fn match_route_titles_rust(a: &str, b: &str) -> bool {
    let trim_a = a.trim();
    let trim_b = b.trim();
    if trim_a.is_empty() || trim_b.is_empty() {
        return false;
    }
    if trim_a.eq_ignore_ascii_case(trim_b) {
        return true;
    }

    let num_a = extract_numeral_str(a);
    let num_b = extract_numeral_str(b);
    if let (Some(na), Some(nb)) = (num_a, num_b) {
        if na != nb {
            return false;
        }
    }

    let ca = clean_route_title_str(a).to_lowercase();
    let cb = clean_route_title_str(b).to_lowercase();
    if ca == cb {
        return true;
    }

    let core_a = ca.trim_end_matches(['线', '篇', '章', '段', '条', '分', '流', '路']);
    let core_b = cb.trim_end_matches(['线', '篇', '章', '段', '条', '分', '流', '路']);
    if !core_a.is_empty() && !core_b.is_empty() {
        if core_a == core_b {
            return true;
        }
        // CJK prefix/suffix matching (e.g. 天女目瑛 vs 瑛, or 素晴らしき日々 vs 素晴らしき日々...)
        if (core_a.ends_with(core_b) || core_b.ends_with(core_a) || core_a.starts_with(core_b) || core_b.starts_with(core_a)) 
            && (core_a.chars().any(|c| ('\u{4e00}'..='\u{9fa5}').contains(&c)) || core_b.chars().any(|c| ('\u{4e00}'..='\u{9fa5}').contains(&c))) {
            return true;
        }
        // Leading word match for English (length >= 4, e.g. jabberwocky)
        let first_word_a = core_a.split_whitespace().next().unwrap_or("");
        let first_word_b = core_b.split_whitespace().next().unwrap_or("");
        if first_word_a.len() >= 4 && first_word_a == first_word_b {
            return true;
        }
    }

    false
}

    // 同步更新正文表格中的路线通关状态 (| 路线名 | ... | 未 | -> | 已 |)
    if let (Some(target_title), Some(is_completed)) = (&route_title, route_completed) {
        let mut body_lines: Vec<String> = body.split(line_ending).map(|s| s.to_string()).collect();
        let mut status_col_idx = 2; // 默认第二数据列 (parts[2])
        let mut route_col_idx = 1;
        let mut in_table = false;

        let num_lines = body_lines.len();
        let mut i = 0;
        while i < num_lines {
            let trimmed = body_lines[i].trim().to_string();
            if trimmed.starts_with('|') && trimmed.ends_with('|') {
                // 只有紧随其后是 | --- | 的行，才是表头行！绝不受备注中“通关”字样的干扰
                if i + 1 < num_lines && body_lines[i + 1].trim().starts_with('|') && body_lines[i + 1].contains("---") {
                    let parts: Vec<&str> = body_lines[i].split('|').collect();
                    for (idx, p) in parts.iter().enumerate().skip(1).take(parts.len().saturating_sub(2)) {
                        let p_t = p.trim();
                        if p_t == "完成度" || p_t == "状态" || p_t == "进度" 
                           || p_t == "通关" || p_t == "达成" || p_t == "完成"
                           || p_t.contains("状态") || p_t.contains("完成度") || p_t.contains("进度")
                           || p_t.eq_ignore_ascii_case("status") || p_t.eq_ignore_ascii_case("clear") {
                            status_col_idx = idx;
                            break;
                        }
                    }
                    for (idx, p) in parts.iter().enumerate().skip(1).take(parts.len().saturating_sub(2)) {
                        let p_t = p.trim();
                        if p_t.contains("路线") || p_t.contains("线路") || p_t.contains("结局")
                           || p_t.contains("女主") || p_t.contains("章节") || p_t.contains("角色")
                           || p_t.contains("分支") || p_t.contains("卷") || p_t.contains("名称")
                           || p_t.contains("标题") || p_t.eq_ignore_ascii_case("route") || p_t.eq_ignore_ascii_case("chapter") {
                            route_col_idx = idx;
                            break;
                        }
                    }
                    if route_col_idx == status_col_idx {
                        route_col_idx = if status_col_idx == 1 { 2 } else { 1 };
                    }
                    in_table = true;
                    i += 2; // 跳过表头和紧随其后的分隔行
                    continue;
                }

                // 数据行
                if in_table && !trimmed.contains("---") {
                    let parts: Vec<&str> = body_lines[i].split('|').collect();
                    if route_col_idx < parts.len() && status_col_idx < parts.len() {
                        let route_cell = parts[route_col_idx].trim();
                        if !route_cell.is_empty() && match_route_titles_rust(route_cell, target_title) {
                            let mut new_parts: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
                            let cur_cell = new_parts[status_col_idx].clone();
                            let cur_val = cur_cell.trim().to_string();
                            let target_text = if cur_val.len() > 3 {
                                if is_completed { "已通关" } else { "未通关" }
                            } else {
                                if is_completed { "已" } else { "未" }
                            };
                            if !cur_val.is_empty() {
                                new_parts[status_col_idx] = cur_cell.replace(&cur_val, target_text);
                            } else {
                                new_parts[status_col_idx] = format!(" {} ", target_text);
                            }
                            body_lines[i] = new_parts.join("|");
                        }
                    }
                }
            } else {
                in_table = false;
            }
            i += 1;
        }
        body = body_lines.join(line_ending);
    }

    if has_fm {
        let mut lines: Vec<String> = fm_str.split(line_ending).map(|s| s.to_string()).collect();
        let mut in_progress = false;
        let mut in_routes = false;
        let mut status_updated = false;
        let mut routes_cleared_updated = false;

        for line in lines.iter_mut() {
            let trimmed = line.trim().to_string();
            if trimmed == "progress:" {
                in_progress = true;
                in_routes = false;
                continue;
            } else if trimmed == "routes:" {
                in_routes = true;
                in_progress = false;
                continue;
            } else if !line.starts_with(' ') && !line.starts_with('\t') {
                in_progress = false;
                in_routes = false;
            }

            if let Some(new_status) = &status {
                if in_progress && (trimmed.starts_with("status:") || trimmed.starts_with("status :")) {
                    *line = format!("  status: {}", new_status);
                    status_updated = true;
                    continue;
                } else if !in_progress && (trimmed.starts_with("status:") || trimmed.starts_with("status :")) {
                    *line = format!("status: {}", new_status);
                    status_updated = true;
                    continue;
                }
            }

            if let Some(cleared_val) = routes_cleared {
                if in_routes && (trimmed.starts_with("cleared:") || trimmed.starts_with("cleared :")) {
                    *line = format!("  cleared: {}", cleared_val);
                    routes_cleared_updated = true;
                }
            }
        }

        if let Some(new_status) = &status {
            if !status_updated {
                if let Some(pos) = lines.iter().position(|l| l.trim() == "progress:") {
                    lines.insert(pos + 1, format!("  status: {}", new_status));
                } else {
                    lines.push(format!("status: {}", new_status));
                }
            }
        }

        if let Some(cleared_val) = routes_cleared {
            if !routes_cleared_updated {
                if let Some(pos) = lines.iter().position(|l| l.trim() == "routes:") {
                    lines.insert(pos + 1, format!("  cleared: {}", cleared_val));
                }
            }
        }

        let new_fm = lines.join(line_ending);
        let new_content = format!("---{}{}{}---{}", line_ending, new_fm, line_ending, body);
        std::fs::write(p, new_content).map_err(|e| format!("写入文件失败: {}", e))?;
    } else {
        std::fs::write(p, body).map_err(|e| format!("写入文件失败: {}", e))?;
    }

    Ok(())
}

// 解析单个游戏攻略 Markdown 文件的 YAML 元数据与概览
pub(crate) fn parse_guide_file(path: &std::path::Path, base_vault: &std::path::Path) -> Option<ObsidianGameGuide> {
    let file_name = path.file_name()?.to_string_lossy().to_string();
    if file_name.starts_with('.') || !file_name.ends_with(".md") {
        return None;
    }
    if file_name == "攻略整理规范.md"
        || file_name == "Galgame索引.md"
        || file_name == "游戏账号与资产金库.md"
        || file_name == "账号金库.md"
        || file_name == "登录凭据.md"
        || file_name == "航标索引.md"
    {
        return None;
    }

    let content = std::fs::read_to_string(path).ok()?;
    let relative = path.strip_prefix(base_vault).unwrap_or(path).to_string_lossy().to_string();
    let metadata = path.metadata().ok();
    let modified = metadata
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "-".to_string());

    let mut title: Option<String> = None;
    let mut series: Option<String> = None;
    let mut series_order: Option<f64> = None;
    let mut is_collection = false;
    let mut original_title: Option<String> = None;
    let mut aliases: Vec<String> = Vec::new();
    let mut release_date: Option<String> = None;
    let mut year: Option<String> = None;
    let mut developer: Option<String> = None;
    let mut publisher: Option<String> = None;
    let mut status: Option<String> = None;
    let mut progress_note: Option<String> = None;
    let mut rating_vndb: Option<f64> = None;
    let mut rating_personal: Option<f64> = None;
    let mut rating_bangumi: Option<f64> = None;
    let mut routes_total: Option<u32> = None;
    let mut routes_cleared: Option<u32> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut genres: Vec<String> = Vec::new();
    let mut poster: Option<String> = None;
    let mut summary: Option<String> = None;
    let mut links: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;
    let mut current_section = "";
    let mut body_lines: Vec<&str> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if idx == 0 && trimmed == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter {
            if trimmed == "---" {
                in_frontmatter = false;
                continue;
            }

            if !line.starts_with(' ') && !line.starts_with('\t') && trimmed.ends_with(':') {
                current_section = trimmed.trim_end_matches(':');
            }

            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim();
                let val = v.trim().trim_matches('"').trim_matches('\'');

                match key {
                    "entry-type" => is_collection = val == "series",
                    "series" => series = (!val.is_empty()).then(|| val.to_string()),
                    "series-order" => series_order = val.parse::<f64>().ok().filter(|n| n.is_finite()),
                    "title" => {
                        if title.is_none() && !val.is_empty() {
                            title = Some(val.to_string());
                        }
                    }
                    "original-title" => {
                        if !val.is_empty() {
                            original_title = Some(val.to_string());
                        }
                    }
                    "release-date" => {
                        if !val.is_empty() {
                            release_date = Some(val.to_string());
                            let val_chars: Vec<char> = val.chars().collect();
                            if val_chars.len() >= 4 && val_chars[..4].iter().all(|c| c.is_ascii_digit()) {
                                year = Some(val_chars[..4].iter().collect());
                            }
                        }
                    }
                    "developer" => {
                        if !val.is_empty() {
                            developer = Some(val.trim_matches('[').trim_matches(']').trim().to_string());
                        }
                    }
                    "publisher" => {
                        if !val.is_empty() {
                            publisher = Some(val.trim_matches('[').trim_matches(']').trim().to_string());
                        }
                    }
                    "vndb" => {
                        if current_section == "links" && is_http_url(val) {
                            links.insert(key.to_string(), val.to_string());
                        } else if let Ok(num) = val.parse::<f64>() {
                            rating_vndb = Some(num);
                        }
                    }
                    "personal" => {
                        if let Ok(num) = val.parse::<f64>() {
                            rating_personal = Some(num);
                        }
                    }
                    "bangumi" => {
                        if current_section == "links" && is_http_url(val) {
                            links.insert(key.to_string(), val.to_string());
                        } else if let Ok(num) = val.parse::<f64>() {
                            rating_bangumi = Some(num);
                        }
                    }
                    "status" => {
                        if !val.is_empty() {
                            status = Some(val.to_string());
                        }
                    }
                    "note" => {
                        if current_section == "progress" && !val.is_empty() {
                            progress_note = Some(val.to_string());
                        }
                    }
                    "total" => {
                        if current_section == "routes" {
                            if let Ok(num) = val.parse::<u32>() {
                                routes_total = Some(num);
                            }
                        }
                    }
                    "cleared" => {
                        if current_section == "routes" {
                            if let Ok(num) = val.parse::<u32>() {
                                routes_cleared = Some(num);
                            }
                        }
                    }
                    "poster" => {
                        if !val.is_empty() {
                            poster = Some(val.to_string());
                        }
                    }
                    "tags" => {
                        if let Some(inner) = val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                            for item in inner.split(',') {
                                let t = item.trim().trim_matches('"').trim_matches('\'');
                                if !t.is_empty() {
                                    tags.push(t.to_string());
                                }
                            }
                        }
                    }
                    "genres" => {
                        if let Some(inner) = val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                            for item in inner.split(',') {
                                let g = item.trim().trim_matches('"').trim_matches('\'');
                                if !g.is_empty() {
                                    genres.push(g.to_string());
                                }
                            }
                        }
                    }
                    "aliases" => {
                        if let Some(inner) = val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                            for item in inner.split(',') {
                                let a = item.trim().trim_matches('"').trim_matches('\'');
                                if !a.is_empty() {
                                    aliases.push(a.to_string());
                                }
                            }
                        }
                    }
                    _ => {
                        if current_section == "links" && !val.is_empty() && is_http_url(val) {
                            links.insert(key.to_string(), val.to_string());
                        }
                    }
                }
            } else if let Some(rest) = trimmed.strip_prefix("- ") {
                let val = rest.trim().trim_matches('"').trim_matches('\'');
                if current_section == "tags" && !val.is_empty() {
                    tags.push(val.to_string());
                } else if current_section == "genres" && !val.is_empty() {
                    genres.push(val.to_string());
                } else if current_section == "aliases" && !val.is_empty() {
                    aliases.push(val.to_string());
                }
            }
        } else {
            body_lines.push(line);
        }
    }

    // 系列目录保留 Obsidian 导航，不参与作品导入、计数或攻略匹配。
    if is_collection {
        return None;
    }
    let clean_fname = file_name.trim_end_matches(".md");
    let fname_chars: Vec<char> = clean_fname.chars().collect();
    if fname_chars.len() >= 5 && fname_chars[4] == '-' && fname_chars[..4].iter().all(|c| c.is_ascii_digit()) {
        if year.is_none() {
            year = Some(fname_chars[..4].iter().collect());
        }
        if title.is_none() {
            title = Some(fname_chars[5..].iter().collect());
        }
    } else if fname_chars.len() >= 4 && fname_chars[..4].iter().all(|c| c.is_ascii_digit())
        && year.is_none() {
            year = Some(fname_chars[..4].iter().collect());
        }

    let final_title = title.unwrap_or_else(|| clean_fname.to_string());

    let rating = rating_vndb
        .or(rating_personal)
        .or(rating_bangumi)
        .unwrap_or(9.0);

    let final_status = status.unwrap_or_else(|| "已收藏".to_string());

    let rel_lower = relative.to_lowercase();
    if rel_lower.starts_with("waypoint/") || rel_lower.starts_with("waypoint\\") {
        return None;
    }
    let category = if rel_lower.contains("galgame") {
        "galgame".to_string()
    } else if file_name == "群星.md" || rel_lower.contains("steam") || rel_lower.contains("single") || rel_lower.contains("单机") {
        "single".to_string()
    } else if rel_lower.contains("剑网3") {
        "jx3".to_string()
    } else if rel_lower.contains("光遇") {
        "sky".to_string()
    } else if rel_lower.contains("逆水寒") {
        "nsh".to_string()
    } else if file_name == "手游.md" || rel_lower.contains("mobile") || rel_lower.contains("手游") {
        "mobile".to_string()
    } else if rating_vndb.is_some() || routes_total.is_some() {
        "galgame".to_string()
    } else {
        "single".to_string()
    };

    for bline in &body_lines {
        let btrimmed = bline.trim();
        if !btrimmed.is_empty() 
            && !btrimmed.starts_with('#') 
            && !btrimmed.starts_with('>') 
            && !btrimmed.starts_with('-') 
            && !btrimmed.starts_with('|') 
            && !btrimmed.starts_with('*')
            && !btrimmed.starts_with("![")
            && !btrimmed.contains("折叠文本行尾")
            && !btrimmed.contains("回到索引")
            && !btrimmed.contains("返回索引")
            && !btrimmed.contains("未删减")
            && !btrimmed.contains("重排折叠与索引")
        {
            // 清洗 cite 符号与 Unicode 私用区字符 (E000..F8FF)
            let cleaned: String = btrimmed.chars().filter(|&c| {
                let code = c as u32;
                !(0xE000..=0xF8FF).contains(&code)
            }).collect();
            let cleaned_trimmed = cleaned.trim();
            if cleaned_trimmed.len() > 3 {
                let s = if cleaned_trimmed.chars().count() > 140 {
                    format!("{}...", cleaned_trimmed.chars().take(140).collect::<String>())
                } else {
                    cleaned_trimmed.to_string()
                };
                summary = Some(s);
                break;
            }
        }
    }

    let id = format!("guide_{}", relative.replace(['/', '\\', ' '], "_").trim_end_matches(".md"));

    Some(ObsidianGameGuide {
        series,
        series_order,
        id,
        title: final_title,
        original_title,
        aliases,
        release_date,
        year,
        developer,
        publisher,
        category,
        status: final_status,
        rating_vndb,
        rating_personal,
        rating_bangumi,
        rating,
        routes_total,
        routes_cleared,
        progress_note,
        tags,
        genres,
        file_path: path.to_string_lossy().to_string(),
        relative_path: relative,
        file_name,
        modified,
        poster,
        summary,
        links,
    })
}

fn is_http_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (lower.starts_with("https://") || lower.starts_with("http://"))
        && !lower.chars().any(|c| c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn links_keep_vndb_and_bangumi_urls() {
        let dir = std::env::temp_dir().join(format!("wp-guide-links-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("2015-sample.md");
        let mut file = std::fs::File::create(&path).unwrap();
        write!(
            file,
            "---\noriginal-title: STEINS;GATE 0\nrating:\n  vndb: 8.3\n  bangumi: 8.1\nlinks:\n  vndb: https://vndb.org/v1946\n  bangumi: https://bgm.tv/subject/140725\n---\n# body\n"
        )
        .unwrap();

        let guide = parse_guide_file(&path, &dir).unwrap();
        assert_eq!(guide.rating_vndb, Some(8.3));
        assert_eq!(guide.rating_bangumi, Some(8.1));
        assert_eq!(guide.links.get("vndb").map(String::as_str), Some("https://vndb.org/v1946"));
        assert_eq!(guide.links.get("bangumi").map(String::as_str), Some("https://bgm.tv/subject/140725"));
        assert_eq!(guide.original_title.as_deref(), Some("STEINS;GATE 0"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn series_indexes_are_not_games_and_members_keep_order_and_identity() {
        let dir = tempfile::tempdir().unwrap();
        let index = dir.path().join("合集.md");
        std::fs::write(&index, "---\ntitle: 系列目录\nentry-type: series\n---\n# 目录\n").unwrap();
        assert!(parse_guide_file(&index, dir.path()).is_none());
        let member = dir.path().join("2014-独立作品.md");
        std::fs::write(&member, "---\ntitle: 独立作品\nis-overview: true\nseries: 美少女万华镜\nseries-order: 2.5\noriginal-title: Separate work\naliases: [独立别名]\nlinks:\n  vndb: https://vndb.org/v14240\n---\n# 攻略\n").unwrap();
        let guide = parse_guide_file(&member, dir.path()).unwrap();
        assert_eq!(guide.series.as_deref(), Some("美少女万华镜"));
        assert_eq!(guide.series_order, Some(2.5));
        assert_eq!(guide.title, "独立作品");
        assert_eq!(guide.aliases, vec!["独立别名"]);
        assert_eq!(guide.links["vndb"], "https://vndb.org/v14240");
    }

}

// Tauri Commands: 智能扫描并结构化解析 Obsidian 游戏与 Galgame 攻略库
#[tauri::command]
pub fn scan_obsidian_game_guides(
    state: State<AppState>,
    custom_vault_path: Option<String>,
) -> Result<Vec<ObsidianGameGuide>, String> {
    let vault_path = if let Some(p) = custom_vault_path.filter(|s| !s.trim().is_empty()) {
        p
    } else {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'obsidian_vault_path'",
            [],
            |r| r.get(0),
        )
        .ok()
        .filter(|s: &String| !s.trim().is_empty())
        .ok_or_else(|| {
            "尚未绑定 Obsidian 仓库，请先在【系统设置】中选择 Obsidian 仓库路径".to_string()
        })?
    };

    let base_path = std::path::Path::new(&vault_path);
    if !base_path.exists() {
        return Err(format!("Obsidian 仓库目录不存在: {}", vault_path));
    }

    let game_dir = if base_path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.contains("游戏"))
        .unwrap_or(false)
    {
        base_path.to_path_buf()
    } else {
        let c1 = base_path.join("🎮 游戏");
        let c2 = base_path.join("游戏");
        if c1.exists() && c1.is_dir() {
            c1
        } else if c2.exists() && c2.is_dir() {
            c2
        } else {
            base_path.to_path_buf()
        }
    };

    let mut guides = Vec::new();

    fn walk_dir(dir: &std::path::Path, base_vault: &std::path::Path, guides: &mut Vec<ObsidianGameGuide>, depth: usize) {
        if depth > 5 { return; }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    if fname.eq_ignore_ascii_case("Waypoint") {
                        continue;
                    }
                    walk_dir(&path, base_vault, guides, depth + 1);
                } else if path.is_file() && fname.ends_with(".md") {
                    let fname_lower = fname.to_lowercase();
                    if fname == "攻略整理规范.md"
                        || fname == "Galgame索引.md"
                        || fname == "游戏账号与资产金库.md"
                        || fname == "账号.md"
                        || fname == "账号金库.md"
                        || fname == "登录凭据.md"
                        || fname == "航标索引.md"
                        || fname_lower.ends_with("规范.md")
                        || fname_lower.ends_with("索引.md")
                    {
                        continue;
                    }
                    if let Some(guide) = parse_guide_file(&path, base_vault) {
                        guides.push(guide);
                    }
                }
            }
        }
    }

    walk_dir(&game_dir, base_path, &mut guides, 0);

    guides.sort_by(|a, b| {
        let y_cmp = b.year.cmp(&a.year);
        if y_cmp != std::cmp::Ordering::Equal {
            return y_cmp;
        }
        b.modified.cmp(&a.modified)
    });

    Ok(guides)
}

// Tauri Commands: 直接用系统关联程序 (Obsidian) 打开攻略笔记文件
#[tauri::command]
pub fn open_in_obsidian(file_path: String) -> Result<(), String> {
    let p = std::path::Path::new(&file_path);
    if !p.exists() {
        return Err(format!("文件不存在: {}", file_path));
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &file_path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("启动默认编辑器失败: {}", e))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("xdg-open").arg(&file_path).spawn();
        Ok(())
    }
}
