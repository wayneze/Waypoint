use tauri::State;
use crate::AppState;

fn json_arr<'a>(v: &'a serde_json::Value, key: &str) -> &'a [serde_json::Value] {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[])
}

fn json_obj(v: &serde_json::Value, key: &str) -> serde_json::Value {
    v.get(key).cloned().unwrap_or(serde_json::Value::Null)
}

fn json_str(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string()
}

fn json_bool(v: &serde_json::Value, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(false)
}

fn json_i64(v: &serde_json::Value, key: &str) -> i64 {
    v.get(key)
        .and_then(|x| x.as_i64().or_else(|| x.as_f64().map(|n| n as i64)))
        .unwrap_or(0)
}

fn json_f64(v: &serde_json::Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|n| n as f64)))
        .unwrap_or(0.0)
}

fn md_check(done: bool) -> &'static str {
    if done { "- [x]" } else { "- [ ]" }
}

fn write_owned_note(dir: &std::path::Path, name: &str, body: &str) -> Result<std::path::PathBuf, String> {
    let path = dir.join(name);
    std::fs::write(&path, body.as_bytes()).map_err(|e| format!("写入 {} 失败: {}", name, e))?;
    Ok(path)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CalloutFold {
    None,
    Expanded,
    Collapsed,
}

fn push_callout_with_fold(md: &mut String, kind: &str, fold: CalloutFold, title: &str, body: &[String]) {
    let fold_str = match fold {
        CalloutFold::None => "",
        CalloutFold::Expanded => "+",
        CalloutFold::Collapsed => "-",
    };
    md.push_str(&format!("> [!{}]{} {}  \n", kind, fold_str, title));
    if body.is_empty() {
        md.push('\n');
        return;
    }
    for line in body {
        for part in line.split('\n') {
            let t = part.trim_end();
            if t.is_empty() {
                md.push_str(">\n");
            } else if t.starts_with('|') && t.ends_with('|') {
                md.push_str("> ");
                md.push_str(t);
                md.push('\n');
            } else {
                md.push_str("> ");
                md.push_str(t);
                md.push_str("  \n");
            }
        }
    }
    md.push('\n');
}


fn category_label(s: &str) -> &str {
    match s {
        "daily" => "日常",
        "weekly" => "周常",
        "custom" => "自定义",
        "galgame" => "Galgame",
        "single" => "单机",
        "steam" => "Steam",
        "playing" => "在玩",
        "completed" => "已通关",
        "backlog" => "待玩",
        "dropped" => "弃坑",
        other => other,
    }
}

fn cycle_label(kind: &str, val: &str) -> String {
    let head = match kind {
        "once" => "一次性",
        "daily" => "每日",
        "weekly" => "每周",
        "interval" => "间隔",
        "deadline" => "截止",
        other => other,
    };
    if val.is_empty() {
        head.to_string()
    } else {
        format!("{} · {}", head, val)
    }
}

fn heading(title: &str) -> String {
    format!("# {}\n\n", title)
}

fn build_readme(now: &str, counts: &serde_json::Value) -> String {
    let mut md = heading("航标中枢索引");

    let vault_count = counts.get("vault").and_then(|x| x.as_u64()).unwrap_or(0);
    let task_count = counts.get("tasks").and_then(|x| x.as_u64()).unwrap_or(0);
    let char_count = counts.get("characters").and_then(|x| x.as_u64()).unwrap_or(0);
    let memo_count = counts.get("memos").and_then(|x| x.as_u64()).unwrap_or(0);
    let game_count = counts.get("game_records").and_then(|x| x.as_u64()).unwrap_or(0);

    let summary_lines = vec![
        format!("- **同步时间**：`{}`", now),
        format!("- **管理账号**：{} 个", vault_count),
        format!("- **受管角色**：{} 个", char_count),
        format!("- **待办任务**：{} 项", task_count),
        format!("- **万象记事**：{} 条", memo_count),
        format!("- **游戏档案**：{} 部", game_count),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "数据同步总览", &summary_lines);

    md.push_str("## 核心模块导航\n\n");
    md.push_str("| 模块名称 | 作用说明 | 快速入口 |\n");
    md.push_str("| :--- | :--- | :--- |\n");
    md.push_str(&format!("| 🏛️ **账号金库** | 跨平台游戏账号、资产明细与安全状态 | [[账号金库]] ({} 条) |\n", vault_count));
    md.push_str(&format!("| 📅 **日常与周常** | 角色日常/周常循环任务与打卡清单 | [[日常与周常]] ({} 项 / {} 角色) |\n", task_count, char_count));
    md.push_str(&format!("| 📝 **万象记事** | 周期备忘、目标进度追踪与灵感便签 | [[万象记事]] ({} 条) |\n", memo_count));
    md.push_str(&format!("| 🎮 **游戏档案** | 单机/Galgame游玩状态、进度与评分汇总 | [[游戏档案]] ({} 部) |\n", game_count));
    md.push_str("| ⚔️ **剑网3图鉴** | 挂件副本掉落、门派阵营声望与套装收集 | [[剑网3图鉴]] |\n");
    md.push_str("| 🕊️ **光遇进度** | 常驻六图先祖解锁与季节毕业追踪 | [[光遇进度]] |\n\n");

    md
}

fn group_by_str<'a>(
    items: &'a [serde_json::Value],
    key: &str,
) -> std::collections::BTreeMap<String, Vec<&'a serde_json::Value>> {
    let mut map: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
        std::collections::BTreeMap::new();
    for item in items {
        let k = json_str(item, key);
        let k = if k.is_empty() { "未分组".to_string() } else { k };
        map.entry(k).or_default().push(item);
    }
    map
}

fn vault_title(item: &serde_json::Value) -> String {
    let account = json_str(item, "account_name");
    if !account.is_empty() {
        account
    } else {
        "未命名".into()
    }
}

fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ").replace('\r', "")
}

fn vault_asset_badge(asset: &serde_json::Value) -> String {
    let name = escape_cell(&json_str(asset, "name"));
    let qty = escape_cell(&json_str(asset, "qty"));
    if name.is_empty() && qty.is_empty() {
        String::new()
    } else if qty.is_empty() {
        format!("`{}`", name)
    } else if name.is_empty() {
        format!("`{}`", qty)
    } else {
        format!("`{} {}`", name, qty)
    }
}

fn build_characters_table(account_server: &str, characters: &[serde_json::Value]) -> Vec<String> {
    if characters.is_empty() {
        return Vec::new();
    }

    let mut has_server = false;
    let mut has_sect = false;
    for ch in characters {
        let ch_server = json_str(ch, "server");
        let effective_server = if ch_server.is_empty() { account_server } else { &ch_server };
        if !effective_server.is_empty() {
            has_server = true;
        }
        if !json_str(ch, "sect").is_empty() {
            has_sect = true;
        }
    }

    let mut header = vec!["角色".to_string()];
    let mut sep = vec![":---".to_string()];

    if has_server {
        header.push("区服".to_string());
        sep.push(":---".to_string());
    }
    if has_sect {
        header.push("门派/职业".to_string());
        sep.push(":---".to_string());
    }
    header.push("资产".to_string());
    sep.push(":---".to_string());

    let header_line = format!("| {} |", header.join(" | "));
    let sep_line = format!("| {} |", sep.join(" | "));

    let mut lines = Vec::new();
    lines.push(String::new());
    lines.push(header_line);
    lines.push(sep_line);

    for ch in characters {
        let name = json_str(ch, "name");
        let name_display = if name.is_empty() {
            "未命名".to_string()
        } else {
            format!("**{}**", escape_cell(&name))
        };

        let mut row = vec![name_display];

        if has_server {
            let ch_server = json_str(ch, "server");
            let effective_server = if ch_server.is_empty() { account_server } else { &ch_server };
            row.push(if effective_server.is_empty() {
                "-".to_string()
            } else {
                escape_cell(effective_server)
            });
        }

        if has_sect {
            let sect = json_str(ch, "sect");
            row.push(if sect.is_empty() {
                "-".to_string()
            } else {
                escape_cell(&sect)
            });
        }

        let ch_assets: Vec<String> = json_arr(ch, "assets")
            .iter()
            .map(vault_asset_badge)
            .filter(|s| !s.is_empty())
            .collect();
        row.push(if ch_assets.is_empty() {
            "-".to_string()
        } else {
            ch_assets.join(" ")
        });

        lines.push(format!("| {} |", row.join(" | ")));
    }
    lines.push(String::new());
    lines
}

pub(crate) fn build_vault_md(_now: &str, items: &[serde_json::Value]) -> String {
    let mut md = heading("账号金库");
    if items.is_empty() {
        md.push_str("> [!info] 尚无账号记录。\n");
        return md;
    }

    let grouped = group_by_str(items, "platform");
    let mut total_characters = 0;
    for rows in grouped.values() {
        for item in rows {
            total_characters += json_arr(item, "characters").len();
        }
    }

    let summary_lines = vec![
        format!("- **管理平台**：{} 个", grouped.len()),
        format!("- **账号总数**：{} 个", items.len()),
        format!("- **角色总数**：{} 个", total_characters),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "金库总览", &summary_lines);

    for (platform, rows) in grouped {
        md.push_str(&format!("## {}\n\n", platform));
        for item in rows {
            let server = json_str(item, "server");
            let steamid64 = json_str(item, "steamid64");
            let notes = json_str(item, "notes");
            let characters = json_arr(item, "characters");

            let mut body = Vec::new();

            let mut meta_items = Vec::new();
            if !server.is_empty() {
                meta_items.push(format!("**默认区服**：{}", server));
            }
            if !steamid64.is_empty() {
                meta_items.push(format!("**SteamID64**：`{}`", steamid64));
            }
            if !meta_items.is_empty() {
                body.push(meta_items.join(" · "));
            }

            if !characters.is_empty() {
                let table_lines = build_characters_table(&server, characters);
                body.extend(table_lines);
            }

            let account_assets: Vec<String> = json_arr(item, "assets")
                .iter()
                .map(vault_asset_badge)
                .filter(|s| !s.is_empty())
                .collect();
            if !account_assets.is_empty() {
                body.push(format!("💎 **账号资产**：{}", account_assets.join(" ")));
            }

            if !notes.is_empty() {
                body.push(format!("📝 **备注**：{}", notes));
            }

            if body.is_empty() {
                body.push("（暂无角色或资产记录）".to_string());
            }

            push_callout_with_fold(
                &mut md,
                "note",
                CalloutFold::Expanded,
                &vault_title(item),
                &body,
            );
        }
    }
    md
}

pub(crate) fn build_dailies_md(_now: &str, characters: &[serde_json::Value], tasks: &[serde_json::Value]) -> String {
    let mut md = heading("日常与周常");
    if characters.is_empty() {
        md.push_str("> [!info] 尚无角色。\n");
        return md;
    }

    let mut total_tasks = 0;
    let mut done_tasks = 0;
    for t in tasks {
        total_tasks += 1;
        if json_bool(t, "done") {
            done_tasks += 1;
        }
    }

    let summary_lines = vec![
        format!("- **角色总数**：{} 个", characters.len()),
        format!("- **循环任务总计**：{} 项", total_tasks),
        format!("- **完成进度**：{}/{} 项已达成", done_tasks, total_tasks),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "任务统计看板", &summary_lines);

    for c in characters {
        let cid = json_str(c, "id");
        let cname = json_str(c, "name");
        let heading_text = if cname.is_empty() { cid.clone() } else { cname };
        md.push_str(&format!("## {}\n\n", heading_text));

        let server = json_str(c, "server");
        let sect = json_str(c, "sect");
        let mut badges = Vec::new();
        if !server.is_empty() {
            badges.push(format!("`区服: {}`", escape_cell(&server)));
        }
        if !sect.is_empty() {
            badges.push(format!("`门派: {}`", escape_cell(&sect)));
        }
        if !badges.is_empty() {
            md.push_str(&format!("{}\n\n", badges.join(" ")));
        }

        let mine: Vec<&serde_json::Value> = tasks
            .iter()
            .filter(|t| json_str(t, "character_id") == cid)
            .collect();
        if mine.is_empty() {
            md.push_str("> [!info] 该角色暂无循环任务。\n\n");
            continue;
        }

        let categories = [
            ("daily", "今日日常", "todo"),
            ("weekly", "本周周常", "summary"),
            ("custom", "自定义循环", "list"),
        ];

        for (cat, label, kind) in categories {
            let bucket: Vec<&&serde_json::Value> = mine
                .iter()
                .filter(|t| json_str(t, "category") == cat)
                .collect();
            if bucket.is_empty() {
                continue;
            }
            let done_count = bucket.iter().filter(|t| json_bool(t, "done")).count();
            let mut lines = Vec::new();
            for t in bucket {
                let tag = json_str(t, "tag");
                let title = escape_cell(&json_str(t, "title"));
                let check = md_check(json_bool(t, "done"));
                let line = if tag.is_empty() {
                    format!("{} {}", check, title)
                } else {
                    format!("{} {} `{}`", check, title, escape_cell(&tag))
                };
                lines.push(line);
            }
            let callout_title = format!("{} ({}/{})", label, done_count, lines.len());
            push_callout_with_fold(&mut md, kind, CalloutFold::Expanded, &callout_title, &lines);
        }
    }
    md
}

fn build_memos_md(_now: &str, memos: &[serde_json::Value]) -> String {
    let mut md = heading("万象记事");
    if memos.is_empty() {
        md.push_str("> [!info] 尚无记事。\n");
        return md;
    }

    let total = memos.len();
    let done_count = memos.iter().filter(|m| json_bool(m, "done")).count();
    let summary_lines = vec![
        format!("- **记事总计**：{} 条", total),
        format!("- **进行中**：{} 条", total - done_count),
        format!("- **已归档**：{} 条", done_count),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "记事总览", &summary_lines);

    for (tag, rows) in group_by_str(memos, "tag") {
        md.push_str(&format!("## {}\n\n", tag));

        let mut pending = Vec::new();
        let mut completed = Vec::new();

        for m in rows {
            if json_bool(m, "done") {
                completed.push(m);
            } else {
                pending.push(m);
            }
        }

        for m in pending {
            let title = json_str(m, "title");
            let mut body = Vec::new();
            let mut meta = Vec::new();
            let cycle = cycle_label(&json_str(m, "cycle_type"), &json_str(m, "cycle_val"));
            if !cycle.is_empty() {
                meta.push(format!("`⏱ {}`", escape_cell(&cycle)));
            }
            if json_bool(m, "has_progress") {
                meta.push(format!(
                    "`🎯 进度: {}/{}`",
                    json_i64(m, "current_progress"),
                    json_i64(m, "target_progress")
                ));
            }
            if !meta.is_empty() {
                body.push(meta.join(" "));
            }
            let note = json_str(m, "note");
            if !note.is_empty() {
                body.push(format!("📝 **备注**：{}", escape_cell(&note)));
            }
            if body.is_empty() {
                body.push("（暂无附加说明）".to_string());
            }
            push_callout_with_fold(&mut md, "tip", CalloutFold::Expanded, &title, &body);
        }

        if !completed.is_empty() {
            let mut archive_lines = Vec::new();
            for m in completed {
                let title = escape_cell(&json_str(m, "title"));
                let progress_info = if json_bool(m, "has_progress") {
                    format!("（已达成 {}）", json_i64(m, "target_progress"))
                } else {
                    String::new()
                };
                archive_lines.push(format!("- [x] {}{}", title, progress_info));
            }
            let archive_title = format!("已完成归档 ({})", archive_lines.len());
            push_callout_with_fold(&mut md, "check", CalloutFold::Collapsed, &archive_title, &archive_lines);
        }
    }
    md
}

fn build_games_md(_now: &str, records: &[serde_json::Value]) -> String {
    let mut md = heading("游戏档案");
    if records.is_empty() {
        md.push_str("> [!info] 尚无档案。\n");
        return md;
    }

    let total = records.len();
    let mut playing_count = 0;
    let mut completed_count = 0;
    let mut rated_count = 0;
    let mut rating_sum = 0.0;

    for r in records {
        let status = json_str(r, "status");
        if status == "playing" {
            playing_count += 1;
        } else if status == "completed" {
            completed_count += 1;
        }
        let rate = json_f64(r, "rating");
        if rate > 0.0 {
            rating_sum += rate;
            rated_count += 1;
        }
    }

    let avg_rate = if rated_count > 0 {
        format!("{:.1}", rating_sum / rated_count as f64)
    } else {
        "-".into()
    };

    let summary_lines = vec![
        format!("- **游戏总数**：{} 部", total),
        format!("- **游玩状态**：{} 部在玩 · {} 部已通关", playing_count, completed_count),
        format!("- **平均评分**：⭐ {}（共 {} 部已评）", avg_rate, rated_count),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "游戏总览", &summary_lines);

    md.push_str("## 档案清单\n\n");
    md.push_str("| 游戏名称 | 类别 | 状态 | 游玩进度 | 评分 |\n");
    md.push_str("| :--- | :---: | :---: | :--- | :---: |\n");

    for r in records {
        let title = escape_cell(&json_str(r, "title"));
        let cat_raw = json_str(r, "category");
        let cat = category_label(&cat_raw);
        let status_raw = json_str(r, "status");
        let status = category_label(&status_raw);
        let progress = escape_cell(&json_str(r, "progress"));
        let progress_display = if progress.is_empty() { "-".into() } else { progress };
        let rating = json_f64(r, "rating");
        let rating_display = if rating > 0.0 { format!("⭐ {:.1}", rating) } else { "-".into() };

        md.push_str(&format!(
            "| **{}** | `{}` | `{}` | {} | {} |\n",
            title, cat, status, progress_display, rating_display
        ));
    }
    md.push('\n');
    md
}

fn build_jx3_md(_now: &str, jx3: &serde_json::Value) -> String {
    let pendants = json_arr(jx3, "pendants");
    let reputations = json_arr(jx3, "reputations");
    let tokens = json_arr(jx3, "tokens");
    let achievements = json_arr(jx3, "achievements");
    let mut md = heading("剑网3图鉴");

    let obtained_pendants = pendants.iter().filter(|p| json_bool(p, "obtained")).count();
    let cd_cleared_pendants = pendants.iter().filter(|p| json_bool(p, "cdCleared")).count();
    let summary_lines = vec![
        format!("- **挂件收集**：{}/{} 已拥有 · {} 项本周已刷", obtained_pendants, pendants.len(), cd_cleared_pendants),
        format!("- **阵营门派声望**：{} 项记录", reputations.len()),
        format!("- **套装牌子系列**：{} 套记录", tokens.len()),
        format!("- **已完成成就**：{} 项", achievements.len()),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "剑网3图鉴总览", &summary_lines);

    md.push_str("## 挂件收集与掉落\n\n");
    if pendants.is_empty() {
        md.push_str("> [!info] 尚无挂件记录。\n\n");
    } else {
        md.push_str("| 挂件名称 | 掉落秘境 / 来源 | 拥有状态 | 本周CD打卡 |\n");
        md.push_str("| :--- | :--- | :---: | :---: |\n");
        for p in pendants {
            let name = escape_cell(&json_str(p, "name"));
            let dungeon = escape_cell(&json_str(p, "dungeon"));
            let dungeon_display = if dungeon.is_empty() { "-".into() } else { dungeon };
            let obtained = if json_bool(p, "obtained") { "✅ 已拥有" } else { "❌ 未拥有" };
            let cd = if json_bool(p, "cdCleared") { "🧹 已清" } else { "⏳ 未打卡" };
            md.push_str(&format!("| **{}** | {} | {} | {} |\n", name, dungeon_display, obtained, cd));
        }
        md.push('\n');
    }

    md.push_str("## 势力与门派声望\n\n");
    if reputations.is_empty() {
        md.push_str("> [!info] 尚无声望记录。\n\n");
    } else {
        md.push_str("| 声望势力 | 当前阶段等级 |\n");
        md.push_str("| :--- | :--- |\n");
        for r in reputations {
            let name = escape_cell(&json_str(r, "name"));
            let level = escape_cell(&json_str(r, "currentLevel"));
            let level_display = if level.is_empty() { "-".into() } else if level == "钦佩" { format!("🏆 **{}**", level) } else { format!("`{}`", level) };
            md.push_str(&format!("| **{}** | {} |\n", name, level_display));
        }
        md.push('\n');
    }

    md.push_str("## 套装牌子收集矩阵\n\n");
    if tokens.is_empty() {
        md.push_str("> [!info] 尚无套装牌子记录。\n\n");
    } else {
        md.push_str("| 套装系列 | 头部 | 上衣 | 腰带 | 护腕 | 鞋子 | 达成进度 |\n");
        md.push_str("| :--- | :---: | :---: | :---: | :---: | :---: | :---: |\n");
        for t in tokens {
            let series_name = escape_cell(&json_str(t, "seriesName"));
            let pieces = json_obj(t, "pieces");
            let head = json_bool(&pieces, "head");
            let body = json_bool(&pieces, "body");
            let belt = json_bool(&pieces, "belt");
            let wrist = json_bool(&pieces, "wrist");
            let shoes = json_bool(&pieces, "shoes") || json_bool(&pieces, "foot");

            let cnt = [head, body, belt, wrist, shoes].iter().filter(|&&b| b).count();
            let check_icon = |b: bool| if b { "✅" } else { "❌" };
            let progress = if cnt == 5 { "5/5 🏆 满".to_string() } else { format!("{}/5", cnt) };

            md.push_str(&format!(
                "| **{}** | {} | {} | {} | {} | {} | {} |\n",
                series_name,
                check_icon(head),
                check_icon(body),
                check_icon(belt),
                check_icon(wrist),
                check_icon(shoes),
                progress
            ));
        }
        md.push('\n');
    }

    md.push_str("## 已完成成就\n\n");
    let ids: Vec<&str> = achievements.iter().filter_map(|a| a.as_str()).collect();
    if ids.is_empty() {
        md.push_str("> [!info] 尚无成就记录。\n\n");
    } else {
        let lines: Vec<String> = ids.iter().map(|id| format!("- [x] {}", escape_cell(id))).collect();
        let ach_title = format!("已达成成就清册 ({} 项)", ids.len());
        push_callout_with_fold(&mut md, "check", CalloutFold::Collapsed, &ach_title, &lines);
    }
    md
}

fn build_sky_md(_now: &str, sky: &serde_json::Value) -> String {
    let spirits = json_arr(sky, "spirits");
    let seasons = json_arr(sky, "seasons");
    let mut md = heading("光遇进度");

    let unlocked_spirits = spirits.iter().filter(|s| json_bool(s, "unlocked")).count();
    let total_spirits = spirits.len();
    let spirit_pct = (unlocked_spirits * 100).checked_div(total_spirits).unwrap_or(0);

    let summary_lines = vec![
        format!("- **常驻先祖解锁**：{}/{} 项（{}%）", unlocked_spirits, total_spirits, spirit_pct),
        format!("- **季节追踪**：{} 个季节记录", seasons.len()),
    ];
    push_callout_with_fold(&mut md, "abstract", CalloutFold::None, "光遇进度总览", &summary_lines);

    md.push_str("## 常驻先祖地图解锁\n\n");
    if spirits.is_empty() {
        md.push_str("> [!info] 尚无先祖记录。\n\n");
    } else {
        for (map, rows) in group_by_str(spirits, "map") {
            let total_map = rows.len();
            let unlocked_map = rows.iter().filter(|s| json_bool(s, "unlocked")).count();
            let mut body = Vec::new();
            for s in rows {
                body.push(format!(
                    "{} {}",
                    md_check(json_bool(s, "unlocked")),
                    escape_cell(&json_str(s, "name"))
                ));
            }
            let is_full = total_map > 0 && unlocked_map == total_map;
            let icon = if is_full { "⭐ " } else { "" };
            let callout_title = format!("{}{}: {}/{} ({}%)", icon, map, unlocked_map, total_map, (unlocked_map * 100).checked_div(total_map).unwrap_or(0));
            push_callout_with_fold(&mut md, "note", CalloutFold::Expanded, &callout_title, &body);
        }
    }

    md.push_str("## 季节毕业与先祖追踪\n\n");
    if seasons.is_empty() {
        md.push_str("> [!info] 尚无季节记录。\n\n");
    } else {
        for season in seasons {
            let title = json_str(season, "season");
            let items = json_arr(season, "items");
            let is_selected = json_bool(season, "selected");
            let mut body = Vec::new();
            let obtained_items = items.iter().filter(|item| json_bool(item, "obtained")).count();

            if items.is_empty() {
                body.push(if is_selected {
                    "📌 正在追踪中".into()
                } else {
                    "⏳ 未追踪".into()
                });
            } else {
                for item in items {
                    body.push(format!(
                        "{} {}",
                        md_check(json_bool(item, "obtained")),
                        escape_cell(&json_str(item, "name"))
                    ));
                }
            }

            let status_badge = if is_selected { "📌 正在追踪" } else { "历史季节" };
            let callout_title = if items.is_empty() {
                format!("{} ({})", title, status_badge)
            } else {
                format!("{} ({} · {}/{})", title, status_badge, obtained_items, items.len())
            };
            let fold = if is_selected { CalloutFold::Expanded } else { CalloutFold::Collapsed };
            push_callout_with_fold(&mut md, "tip", fold, &callout_title, &body);
        }
    }
    md
}

fn rel_posix(vault: &std::path::Path, file: &std::path::Path) -> String {
    file.strip_prefix(vault)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| "Waypoint".to_string())
}

fn snapshot_scopes(snapshot: &serde_json::Value) -> Vec<String> {
    let mut scopes: Vec<String> = json_arr(snapshot, "scopes")
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    if scopes.is_empty() {
        scopes.push("all".into());
    }
    scopes
}

fn scope_enabled(scopes: &[String], name: &str) -> bool {
    scopes.iter().any(|s| s == "all" || s == name)
}

fn game_records_of(snapshot: &serde_json::Value) -> &[serde_json::Value] {
    let camel = json_arr(snapshot, "gameRecords");
    if camel.is_empty() {
        json_arr(snapshot, "game_records")
    } else {
        camel
    }
}

pub(crate) fn projection_files(snapshot: &serde_json::Value, now: &str) -> Vec<(String, String)> {
    let scopes = snapshot_scopes(snapshot);
    let characters = json_arr(snapshot, "characters");
    let tasks = json_arr(snapshot, "tasks");
    let vault_items = json_arr(snapshot, "vault");
    let memos = json_arr(snapshot, "memos");
    let game_records = game_records_of(snapshot);
    let jx3 = json_obj(snapshot, "jx3");
    let sky = json_obj(snapshot, "sky");
    let counts = serde_json::json!({
        "vault": vault_items.len(),
        "tasks": tasks.len(),
        "memos": memos.len(),
        "game_records": game_records.len(),
        "characters": characters.len()
    });

    let mut files = Vec::new();
    if scope_enabled(&scopes, "all") {
        files.push(("航标索引.md".into(), build_readme(now, &counts)));
    }
    if scope_enabled(&scopes, "vault") {
        files.push(("账号金库.md".into(), build_vault_md(now, vault_items)));
    }
    if scope_enabled(&scopes, "dailies") {
        files.push(("日常与周常.md".into(), build_dailies_md(now, characters, tasks)));
    }
    if scope_enabled(&scopes, "memos") {
        files.push(("万象记事.md".into(), build_memos_md(now, memos)));
    }
    if scope_enabled(&scopes, "games") {
        files.push(("游戏档案.md".into(), build_games_md(now, game_records)));
    }
    if scope_enabled(&scopes, "jx3") {
        files.push(("剑网3图鉴.md".into(), build_jx3_md(now, &jx3)));
    }
    if scope_enabled(&scopes, "sky") {
        files.push(("光遇进度.md".into(), build_sky_md(now, &sky)));
    }
    files
}

// 将应用状态投影到仓库根下独占的 Waypoint/。中文文件名。只覆盖传入范围。
// 金库页只写一篇「账号金库」，账号和密码在同一个折叠块里。
#[tauri::command]
pub fn export_waypoint_projection(
    snapshot: serde_json::Value,
    state: State<AppState>,
) -> Result<serde_json::Value, String> {
    let vault_path: String = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM app_settings WHERE key = 'obsidian_vault_path'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "尚未绑定 Obsidian 仓库，请先在【系统设置】中选择 Obsidian 仓库路径".to_string())?
    };

    let base_path = std::path::Path::new(&vault_path);
    if !base_path.exists() || !base_path.is_dir() {
        return Err(format!("Obsidian 仓库路径不存在或无效: {}", vault_path));
    }

    let target_dir = base_path.join("Waypoint");
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| format!("无法创建 Waypoint 目录: {}", e))?;

    let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let planned = projection_files(&snapshot, &now_str);
    let mut written = Vec::new();
    for (name, body) in &planned {
        written.push(write_owned_note(&target_dir, name, body)?);
    }

    for stale in [
        "README.md",
        "vault.md",
        "dailies.md",
        "memos.md",
        "games.md",
        "jx3.md",
        "sky.md",
        "登录凭据.md",
    ] {
        let _ = std::fs::remove_file(target_dir.join(stale));
    }

    let files: Vec<String> = written.iter().map(|p| rel_posix(base_path, p)).collect();
    let vault_items = json_arr(&snapshot, "vault");
    let characters = json_arr(&snapshot, "characters");
    let tasks = json_arr(&snapshot, "tasks");
    let memos = json_arr(&snapshot, "memos");
    let game_records = game_records_of(&snapshot);

    Ok(serde_json::json!({
        "root": target_dir.to_string_lossy(),
        "relative_root": "Waypoint",
        "files": files,
        "counts": {
            "vault": vault_items.len(),
            "tasks": tasks.len(),
            "memos": memos.len(),
            "game_records": game_records.len(),
            "characters": characters.len()
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vault() -> Vec<serde_json::Value> {
        vec![serde_json::json!({
            "id": "v1",
            "platform": "剑网3",
            "account_name": "player@example.com",
            "notes": "已绑定手机玲珑密保锁",
            "assets": [{"id": "a1", "name": "玲珑密保锁", "qty": "已绑"}],
            "characters": [
                {"id": "v1", "name": "示例角色甲", "server": "绝代华双", "sect": "藏剑", "assets": [{"id": "c1a", "name": "金", "qty": "1200万"}]},
                {"id": "v2", "name": "示例角色乙", "server": "梦江南", "sect": "少林", "assets": []}
            ],
            "credential": "Secret#Pass99"
        })]
    }

    #[test]
    fn vault_is_one_note_with_folded_account() {
        let md = build_vault_md("2026-09-08 12:00", &sample_vault());
        assert!(md.starts_with("# 账号金库\n"));
        assert!(md.contains("> [!abstract] 金库总览"));
        assert!(md.contains("## 剑网3"));
        assert!(md.contains("> [!note]+ player@example.com"));
        assert!(md.contains("| 角色 | 区服 | 门派/职业 | 资产 |"));
        assert!(md.contains("| **示例角色甲** | 绝代华双 | 藏剑 | `金 1200万` |"));
        assert!(md.contains("| **示例角色乙** | 梦江南 | 少林 | - |"));
        assert!(md.contains("💎 **账号资产**：`玲珑密保锁 已绑`"));
        assert!(md.contains("📝 **备注**：已绑定手机玲珑密保锁"));
        assert!(!md.contains("Secret#Pass99"));
        assert!(!md.contains("[[登录凭据]]"));
        assert!(!md.contains("source:"));
    }

    #[test]
    fn dailies_use_folded_sections() {
        let characters = vec![serde_json::json!({
            "id": "jx3_cangjian",
            "name": "藏剑",
            "game_id": "jx3",
            "server": "绝代华双",
            "sect": "藏剑"
        })];
        let tasks = vec![serde_json::json!({
            "id": "1",
            "character_id": "jx3_cangjian",
            "title": "今日大战",
            "category": "daily",
            "reset_rule": "07:00",
            "tag": "个人循环",
            "done": true
        })];
        let md = build_dailies_md("2026-09-08 12:00", &characters, &tasks);
        assert!(md.contains("# 日常与周常"));
        assert!(md.contains("> [!abstract] 任务统计看板"));
        assert!(md.contains("## 藏剑"));
        assert!(md.contains("`区服: 绝代华双` `门派: 藏剑`"));
        assert!(md.contains("> [!todo]+ 今日日常 (1/1)"));
        assert!(md.contains("- [x] 今日大战 `个人循环`"));
    }

    #[test]
    fn vault_scope_writes_only_one_note() {
        let snapshot = serde_json::json!({
            "scopes": ["vault"],
            "vault": sample_vault(),
            "characters": [{"id": "jx3_cangjian", "name": "藏剑", "game_id": "jx3"}],
            "tasks": [{"id": "1", "character_id": "jx3_cangjian", "title": "今日大战", "category": "daily", "done": false}],
            "memos": [{"id": "m1", "title": "不该写出", "tag": "剑网3", "cycle_type": "once", "done": false}],
            "gameRecords": [{"id": "g1", "title": "不该写出", "category": "galgame", "status": "playing", "progress": "", "rating": 0}]
        });
        let files = projection_files(&snapshot, "2026-09-08 12:00");
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["账号金库.md"]);
    }

    #[test]
    fn readme_renders_moc_dashboard() {
        let counts = serde_json::json!({
            "vault": 2,
            "tasks": 5,
            "characters": 1,
            "memos": 3,
            "game_records": 4
        });
        let md = build_readme("2026-10-12 12:00", &counts);
        assert!(md.contains("# 航标中枢索引"));
        assert!(md.contains("> [!abstract] 数据同步总览"));
        assert!(md.contains("| 🏛️ **账号金库** | 跨平台游戏账号、资产明细与安全状态 | [[账号金库]] (2 条) |"));
    }

    #[test]
    fn games_renders_gfm_table() {
        let records = vec![
            serde_json::json!({
                "id": "g1",
                "title": "艾尔登法环",
                "category": "single",
                "status": "playing",
                "progress": "王城罗德尔",
                "rating": 9.8
            })
        ];
        let md = build_games_md("2026-10-12 12:00", &records);
        assert!(md.contains("# 游戏档案"));
        assert!(md.contains("> [!abstract] 游戏总览"));
        assert!(md.contains("| **艾尔登法环** | `单机` | `在玩` | 王城罗德尔 | ⭐ 9.8 |"));
    }

    #[test]
    fn jx3_renders_matrix_and_table() {
        let jx3 = serde_json::json!({
            "pendants": [{"name": "夜话白鹭", "dungeon": "荻花宫", "obtained": true, "cdCleared": true}],
            "reputations": [{"name": "大唐兵部", "currentLevel": "钦佩"}],
            "tokens": [{"seriesName": "星演套", "pieces": {"head": true, "body": true, "belt": true, "wrist": true, "shoes": true}}],
            "achievements": ["千秋霸业"]
        });
        let md = build_jx3_md("2026-10-12 12:00", &jx3);
        assert!(md.contains("# 剑网3图鉴"));
        assert!(md.contains("| **夜话白鹭** | 荻花宫 | ✅ 已拥有 | 🧹 已清 |"));
        assert!(md.contains("| **大唐兵部** | 🏆 **钦佩** |"));
        assert!(md.contains("| **星演套** | ✅ | ✅ | ✅ | ✅ | ✅ | 5/5 🏆 满 |"));
    }
}
