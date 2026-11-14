use serde::{Deserialize, Serialize};
use serde_json::Value;

const GOODS_API: &str = "https://api.jiansanzs.cn/api/trpc";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3ExteriorItem {
    pub kind: String,
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3ExteriorSnapshot {
    pub role: String,
    pub server: String,
    pub items: Vec<Jx3ExteriorItem>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3ExteriorScan {
    pub client_root: String,
    pub addon_installed: bool,
    pub snapshots: Vec<Jx3ExteriorSnapshot>,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3GoodsHit {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub price: Option<i64>,
    #[serde(default)]
    pub price_source: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Jx3GoodsQuote {
    pub id: String,
    pub name: String,
    pub alias: String,
    pub price: Option<i64>,
    #[serde(default)]
    pub price_source: String,
    pub note_band: String,
    pub url: String,
    pub updated: String,
    pub failed_at: String,
    #[serde(default)]
    pub image_url: String,
    #[serde(default)]
    pub icon_url: String,
}

fn lua_string_at(src: &str, key: &str) -> Option<String> {
    let token = format!("{key}=");
    let start = src.find(&token)? + token.len();
    let rest = src[start..].trim_start();
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
            continue;
        }
        if ch == '"' {
            break;
        }
        out.push(ch);
    }
    Some(out)
}

fn parse_shot(text: &str, path: &std::path::Path) -> Result<Jx3ExteriorSnapshot, String> {
    let role = lua_string_at(text, "role").unwrap_or_default();
    let server = lua_string_at(text, "server").unwrap_or_default();
    let mut items = Vec::new();
    for chunk in text.split("{kind=") {
        if !chunk.contains("id=") {
            continue;
        }
        let kind = lua_string_at(&format!("kind={chunk}"), "kind").unwrap_or_else(|| "body".into());
        let id = chunk
            .split("id=")
            .nth(1)
            .and_then(|rest| rest.split([',', '}']).next())
            .and_then(|raw| raw.trim().parse::<i64>().ok())
            .unwrap_or(0);
        if id < 0 {
            continue;
        }
        let name = lua_string_at(&format!("kind={chunk}"), "name").unwrap_or_default();
        if id == 0 && name.is_empty() {
            continue;
        }
        items.push(Jx3ExteriorItem { kind, id, name });
    }
    if role.is_empty() && items.is_empty() {
        return Err(format!("无法解析外观快照 {}", path.display()));
    }
    Ok(Jx3ExteriorSnapshot {
        role,
        server,
        items,
        path: path.to_string_lossy().into_owned(),
    })
}

fn install_addon(client_root: &std::path::Path) -> Result<bool, String> {
    let dest = client_root.join("interface").join("WaypointExterior");
    std::fs::create_dir_all(&dest).map_err(|e| format!("无法创建插件目录: {e}"))?;
    for (name, content) in [
        ("info.ini", include_bytes!("../../addons/jx3-waypoint-exterior/info.ini").as_slice()),
        ("main.lua", include_bytes!("../../addons/jx3-waypoint-exterior/main.lua").as_slice()),
    ] {
        std::fs::write(dest.join(name), content)
            .map_err(|e| format!("安装 {name} 失败: {e}"))?;
    }
    std::fs::create_dir_all(client_root.join("interface").join("Waypoint#DATA"))
        .map_err(|e| format!("无法创建快照目录: {e}"))?;
    Ok(dest.join("main.lua").exists())
}

#[tauri::command]
pub fn scan_jx3_exterior(client_root: Option<String>, state: tauri::State<crate::AppState>) -> Result<Jx3ExteriorScan, String> {
    let client_root = client_root.filter(|s| !s.trim().is_empty()).or_else(|| {
        state.db.lock().ok().and_then(|conn|
            crate::settings::read_app_setting(&conn, "waypoint_jx3_mingyi_data_root"))
    });
    let hint = client_root.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let root_path = match hint.map(std::path::PathBuf::from).filter(|p| p.join("interface").is_dir()) {
        Some(root) => root,
        None => {
            let data = crate::mingyi::resolve_mingyi_data_root(hint)?;
            data.parent().and_then(|p| p.parent()).ok_or("无法确定剑网3客户端目录")?.to_path_buf()
        }
    };
    if !root_path.join("interface").is_dir() {
        return Err(format!("找不到剑网3客户端: {}", root_path.display()));
    }
    let addon_installed = install_addon(&root_path)?;
    let shot_dir = root_path.join("interface").join("Waypoint#DATA");
    let mut snapshots = Vec::new();
    let mut warning = None;
    if shot_dir.is_dir() {
        for entry in std::fs::read_dir(&shot_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("jx3dat") {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
            let text = decode_text(&bytes)?;
            match parse_shot(&text, &path) {
                Ok(shot) => snapshots.push(shot),
                Err(e) => warning = Some(e),
            }
        }
    }
    if snapshots.is_empty() && warning.is_none() {
        warning = Some("还没有外观快照。进一次对应角色后回来刷新。".into());
    }
    Ok(Jx3ExteriorScan {
        client_root: root_path.to_string_lossy().into_owned(),
        addon_installed,
        snapshots,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_shot;

    #[test]
    fn addon_installs_from_bundled_files_without_the_source_checkout() {
        let client = tempfile::tempdir().unwrap();
        assert!(super::install_addon(client.path()).unwrap());
        let addon = client.path().join("interface/WaypointExterior");
        assert_eq!(std::fs::read(addon.join("main.lua")).unwrap(),
            include_bytes!("../../addons/jx3-waypoint-exterior/main.lua"));
        assert!(std::fs::read_to_string(addon.join("info.ini")).unwrap().contains("[WaypointExterior]"));
        assert!(client.path().join("interface/Waypoint#DATA").is_dir());
    }

    #[test]
    fn parses_role_server_and_items() {
        let text = r#"
return {
  role="测试角色",
  server="唯我独尊",
  items={
    {kind="body",id=12,name="金发·璨月蝶心"},
    {kind="hair",id=0,name="狐金"},
  },
}
"#;
        let shot = parse_shot(text, std::path::Path::new("exterior.jx3dat")).unwrap();
        assert_eq!(shot.role, "测试角色");
        assert_eq!(shot.server, "唯我独尊");
        assert_eq!(shot.items.len(), 2);
        assert_eq!(shot.items[1].kind, "hair");
        assert_eq!(shot.items[1].name, "狐金");
    }

    #[test]
    fn parses_floating_point_and_fallback_prices() {
        use super::{hit_from_doc, parse_goods_price, quote_from_goods};

        // 浮点数万宝楼均价：1521.83 -> 四舍五入为 1522
        let doc = serde_json::json!({
            "_id": "64d1c51ec47e1afc4a594663",
            "goodsName": "玉雪灵龙·衣",
            "bm": "灵龙衣服",
            "wblTradePrice": 1521.83,
            "wblPrice": 1540
        });
        assert_eq!(parse_goods_price(&doc), (Some(1522), "成交均价".to_string()));
        let hit = hit_from_doc(&doc).unwrap();
        assert_eq!(hit.price, Some(1522));

        let quote_json = serde_json::json!({
            "result": {
                "data": doc
            }
        });
        let quote = quote_from_goods(&quote_json);
        assert_eq!(quote.price, Some(1522));
        assert_eq!(quote.name, "玉雪灵龙·衣");
        let image_quote = quote_from_goods(&serde_json::json!({
            "result": { "data": { "img": "", "imgList": ["https://cos2.jiansanzs.com/goods/detail.png"], "itemBase": { "icon": "https://dl.pvp.xoyo.com/prod/icon.png" } } }
        }));
        assert_eq!(image_quote.image_url, "https://cos2.jiansanzs.com/goods/detail.png");
        assert_eq!(image_quote.icon_url, "https://dl.pvp.xoyo.com/prod/icon.png");

        // 成交均价为 0 时用挂牌价，并标明来源
        let doc_fallback = serde_json::json!({
            "_id": "64e7349c1bb2a84862d38bac",
            "goodsName": "玉雪灵龙礼盒",
            "wblTradePrice": 0,
            "wblPrice": 1680
        });
        assert_eq!(parse_goods_price(&doc_fallback), (Some(1680), "挂牌价".to_string()));
        // 绑定的单件只有站内报价 price，不算市价
        let bound = serde_json::json!({ "goodsName": "玉雪灵龙·黑发", "wblTradePrice": 0, "wblPrice": 0, "price": 300 });
        assert_eq!(parse_goods_price(&bound), (None, String::new()));
    }

    #[test]
    fn picks_goods_by_tier_alias_and_reports_ambiguity() {
        use super::pick_goods_doc;
        use serde_json::json;
        let name_of = |r: Result<Option<&serde_json::Value>, String>| {
            r.map(|d| d.and_then(|d| d["goodsName"].as_str()).unwrap_or("").to_string())
        };
        // 接口实际返回顺序：礼盒排在衣服前面，名字要先对「·衣」
        let lingl = [
            json!({"goodsName": "玉雪灵龙礼盒", "bm": "灵龙盒", "goodsType": "外观礼盒"}),
            json!({"goodsName": "玉雪灵龙·衣", "bm": "灵龙衣服", "goodsType": "上衣"}),
            json!({"goodsName": "玉雪灵龙·黑发", "bm": "灵龙黑发"}),
            json!({"goodsName": "玉雪灵龙·白发", "bm": "灵龙白发", "goodsType": "发型"}),
        ];
        assert_eq!(name_of(pick_goods_doc("玉雪灵龙", &lingl)).unwrap(), "玉雪灵龙·衣");
        // 别名完全一致
        assert_eq!(name_of(pick_goods_doc("灵龙白发", &lingl)).unwrap(), "玉雪灵龙·白发");
        let hu = [json!({"goodsName": "金发·璨月蝶心", "bm": "狐金", "goodsType": "发型"})];
        assert_eq!(name_of(pick_goods_doc("狐金", &hu)).unwrap(), "金发·璨月蝶心");
        // 头像不当成发型
        let long = [
            json!({"goodsName": "金发·龙章凤姿", "bm": "龙金", "goodsType": "发型"}),
            json!({"goodsName": "像韵·龙章凤姿", "bm": "龙金头像", "goodsType": "小头像"}),
        ];
        assert_eq!(name_of(pick_goods_doc("龙章凤姿", &long)).unwrap(), "金发·龙章凤姿");
        // 同档两件就报出来，不挑
        let two = [
            json!({"goodsName": "玉露仙音·黑发", "goodsType": "发型"}),
            json!({"goodsName": "玉露仙音·白发", "goodsType": "发型"}),
        ];
        assert!(pick_goods_doc("玉露仙音", &two).unwrap_err().contains("对上多件"));
        // 只是含有这几个字的不算
        let loose = [json!({"goodsName": "逐山海"}), json!({"goodsName": "山海异兽册", "bm": "三代脚印"})];
        assert_eq!(name_of(pick_goods_doc("山海", &loose)).unwrap(), "");
    }
}

fn decode_text(bytes: &[u8]) -> Result<String, String> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        if text.contains("return") {
            return Ok(text.to_string());
        }
    }
    let (cow, _, _) = encoding_rs::GBK.decode(bytes);
    Ok(cow.into_owned())
}

fn extract_price(val: Option<&Value>) -> Option<i64> {
    let v = val?;
    if let Some(i) = v.as_i64() {
        if i > 0 {
            return Some(i);
        }
    }
    if let Some(f) = v.as_f64() {
        let rounded = f.round() as i64;
        if rounded > 0 {
            return Some(rounded);
        }
    }
    if let Some(s) = v.as_str() {
        if let Ok(f) = s.parse::<f64>() {
            let rounded = f.round() as i64;
            if rounded > 0 {
                return Some(rounded);
            }
        }
    }
    None
}

// 三个字段不是同一种价：wblTradePrice 万宝楼近期成交均价，wblPrice 万宝楼挂牌价，
// price 是助手站用户报的成交价。只认万宝楼两项，并标明是哪一种，不拿报价冒充市价。
fn parse_goods_price(data: &Value) -> (Option<i64>, String) {
    if let Some(p) = extract_price(data.get("wblTradePrice")) {
        return (Some(p), "成交均价".into());
    }
    if let Some(p) = extract_price(data.get("wblPrice")) {
        return (Some(p), "挂牌价".into());
    }
    (None, String::new())
}

fn goods_docs(value: &Value) -> Vec<Value> {
    value
        .pointer("/result/data/docs")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn hit_from_doc(doc: &Value) -> Option<Jx3GoodsHit> {
    let id = doc
        .get("_id")
        .or_else(|| doc.get("id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if id.is_empty() {
        return None;
    }
    let name = doc
        .get("goodsName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let (price, price_source) = parse_goods_price(doc);
    Some(Jx3GoodsHit {
        alias: doc.get("bm").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        price,
        price_source,
        url: format!("https://www.jiansanzs.cn/goods/detail/?id={id}"),
        id,
        name,
    })
}

fn quote_from_goods(value: &Value) -> Jx3GoodsQuote {
    let data = value
        .pointer("/result/data")
        .cloned()
        .unwrap_or(Value::Null);
    let id = data
        .get("id")
        .or_else(|| data.get("_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = data
        .get("goodsName")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let alias = data.get("bm").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let (price, price_source) = parse_goods_price(&data);
    Jx3GoodsQuote {
        id: id.clone(),
        name,
        alias,
        price,
        price_source,
        note_band: String::new(),
        url: if id.is_empty() {
            String::new()
        } else {
            format!("https://www.jiansanzs.cn/goods/detail/?id={id}")
        },
        updated: data
            .get("updated")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        failed_at: String::new(),
        image_url: data.get("img").and_then(Value::as_str).filter(|url| !url.is_empty())
            .or_else(|| data.get("imgList").and_then(Value::as_array).and_then(|rows| rows.first()).and_then(Value::as_str))
            .filter(|url| url.starts_with("https://"))
            .unwrap_or("").to_string(),
        icon_url: data.pointer("/itemBase/icon").and_then(Value::as_str)
            .filter(|url| url.starts_with("https://")).unwrap_or("").to_string(),
    }
}

async fn get_trpc(path: &str, input: &str) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .user_agent("Waypoint/0.1")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(format!("{GOODS_API}/{path}"))
        .query(&[("input", input)])
        .send()
        .await
        .map_err(|e| format!("剑三交易助手网络错误: {e}"))?;
    let status = res.status();
    let body: Value = res
        .json()
        .await
        .map_err(|e| format!("剑三交易助手返回无法解析: {e}"))?;
    if !status.is_success() {
        let message = body
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .unwrap_or("请求失败");
        return Err(format!("剑三交易助手 {status}: {message}"));
    }
    Ok(body)
}

async fn fetch_wbl_trade_data_price(goods_id: &str) -> Option<i64> {
    if goods_id.is_empty() {
        return None;
    }
    let input = serde_json::json!({
        "goodsId": goods_id,
        "timeRange": 30
    }).to_string();
    let body = get_trpc("wblGoodsTradeData.findDataByGoodsId", &input).await.ok()?;
    let list = body.pointer("/result/data")?.as_array()?;
    if let Some(latest) = list.last() {
        extract_price(latest.get("wblTradePrice"))
            .or_else(|| extract_price(latest.get("marketPrice")))
            .or_else(|| extract_price(latest.get("wblPrice")))
    } else {
        None
    }
}

// wblGoods.getPaginate 返回的是历史成交记录（state=6，含一两年前的单），不是当前挂单，不能当在售价用。
async fn resolve_realtime_wbl_price(goods_id: &str) -> (Option<i64>, String) {
    match fetch_wbl_trade_data_price(goods_id).await {
        Some(p) => (Some(p), "30日成交".into()),
        None => (None, String::new()),
    }
}

#[tauri::command]
pub async fn fetch_jx3_goods_quote(id: String) -> Result<Jx3GoodsQuote, String> {
    let id = id.trim();
    if id.is_empty() || id.len() > 40 {
        return Err("商品 id 无效".into());
    }
    let input = serde_json::to_string(id).map_err(|e| e.to_string())?;
    let body = get_trpc("goods.findById", &input).await?;
    let mut quote = quote_from_goods(&body);
    if quote.id.is_empty() {
        quote.id = id.to_string();
    }
    if quote.price.is_none() {
        (quote.price, quote.price_source) = resolve_realtime_wbl_price(&quote.id).await;
    }
    Ok(quote)
}

#[tauri::command]
pub async fn search_jx3_goods(name: String) -> Result<Vec<Jx3GoodsHit>, String> {
    let name = name.trim();
    if name.chars().count() < 1 || name.chars().count() > 40 {
        return Ok(Vec::new());
    }
    let input = serde_json::json!({
        "filter": { "name": name },
        "page": 1,
        "pageSize": 8
    })
    .to_string();
    let body = get_trpc("goods.getPaginateByName", &input).await?;
    let mut hits = goods_docs(&body).iter().filter_map(hit_from_doc).take(8).collect::<Vec<_>>();
    for hit in hits.iter_mut().take(4) {
        if hit.price.is_none() {
            (hit.price, hit.price_source) = resolve_realtime_wbl_price(&hit.id).await;
        }
    }
    Ok(hits)
}

fn doc_text<'a>(doc: &'a Value, key: &str) -> &'a str {
    doc.get(key).and_then(|v| v.as_str()).unwrap_or("").trim()
}

type NameTier<'n> = dyn Fn(&str, &str) -> bool + 'n;

/// 按名字从模糊搜索结果里挑商品，按档从严到宽：
/// 正式名或别名完全一致 → 「名·衣」「名衣」→ 「名礼盒」「名盒」→ 「名·X」「X·名」。
/// 停在第一个有结果的档；这一档里对上多件（黑发/白发、发型/头像）就报出来，
/// 不按接口返回顺序或"有价的"随手挑一件。头像（像韵·）只在名字本身带「像韵」时才算。
fn pick_goods_doc<'a>(name: &str, docs: &'a [Value]) -> Result<Option<&'a Value>, String> {
    let avatar_ok = name.contains("像韵");
    let docs: Vec<&Value> = docs
        .iter()
        .filter(|d| avatar_ok || !doc_text(d, "goodsType").contains("头像"))
        .collect();
    let tiers: [&NameTier<'_>; 4] = [
        &|g, bm| g == name || bm == name,
        &|g, _| g == format!("{name}·衣") || g == format!("{name}衣"),
        &|g, _| g == format!("{name}礼盒") || g == format!("{name}盒"),
        &|g, _| g.starts_with(&format!("{name}·")) || g.ends_with(&format!("·{name}")),
    ];
    for tier in tiers {
        let hits: Vec<&Value> = docs
            .iter()
            .copied()
            .filter(|d| tier(doc_text(d, "goodsName"), doc_text(d, "bm")))
            .collect();
        match hits.len() {
            0 => continue,
            1 => return Ok(Some(hits[0])),
            _ => {
                let names: Vec<&str> = hits.iter().map(|d| doc_text(d, "goodsName")).collect();
                return Err(format!("「{name}」对上多件：{}，从候选里点选具体那件", names.join("、")));
            }
        }
    }
    Ok(None)
}

#[tauri::command]
pub async fn fetch_jx3_goods_by_name(name: String) -> Result<Jx3GoodsQuote, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err("外观名无效".into());
    }
    let input = serde_json::json!({ "name": name }).to_string();
    let body = get_trpc("goods.getGoodsByName", &input).await?;
    let mut quote = quote_from_goods(&body);
    if quote.id.is_empty() {
        // 模糊搜索候选：「玉雪灵龙」先对「玉雪灵龙·衣」，再对礼盒；「龙章凤姿」对「金发·龙章凤姿」。
        let search_input = serde_json::json!({
            "filter": { "name": name },
            "page": 1,
            "pageSize": 8
        })
        .to_string();
        if let Ok(search_body) = get_trpc("goods.getPaginateByName", &search_input).await {
            let docs = goods_docs(&search_body);
            if let Some(doc) = pick_goods_doc(name, &docs)? {
                quote = quote_from_goods(&serde_json::json!({ "result": { "data": doc } }));
            }
        }
    }
    if quote.id.is_empty() {
        return Err(format!("没有对上「{name}」"));
    }
    if quote.price.is_none() {
        (quote.price, quote.price_source) = resolve_realtime_wbl_price(&quote.id).await;
    }
    Ok(quote)
}
