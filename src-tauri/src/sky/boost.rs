//! 代练反馈只读适配。供应商协议在此解释，不执行催单、续费或登录操作。
use aws_lc_rs::{
    cipher::{PaddedBlockEncryptingKey, UnboundCipherKey, AES_256},
    rand::{SecureRandom, SystemRandom},
    rsa::{Pkcs1PublicEncryptingKey, PublicEncryptingKey},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{FixedOffset, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

// 来自该系统公开网页的请求加密公钥；不是账号凭据。
const MZ_PUBLIC_KEY: &str = concat!(
    "MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyDpuProPOwauUsxJZMk4",
    "BhiiJYZV4jKBgiomvMEs5OLy6aV89mpO8RDDAmkvXr5WdB7+gg+Li64diQfVmErl",
    "XLtaxqASFTnO7ugTepaZX/XjojJ91yjzIO/rcqGHfU1eOtjSsNYa3qpf0+Z3Psii",
    "zzs+yUXtFnn0UG5SfVQIYerIuY7WU9nRyzodxpuNBZtkIab5CoT5rhN/rIzmaZtw",
    "jqH3XeWxkTQYkubhsW41yMyeoPhU8oKG4pxqWsp/ehbcQFo6gCbDTj/Y5/EC8bEK",
    "z1GaBhD7fwzcyPGfmupc1tFCB4O005pyndruIpf/YKlw2+cg4T27AjZk/9S1US+w",
    "QwIDAQAB"
);

#[derive(Debug, Serialize)]
pub struct SkyBoostFeedback {
    pub day: String,
    pub fetched_at: String,
    pub status: String,
    pub completed: bool,
    pub order_state: String,
    pub days: Option<i64>,
    pub expiration_date: Option<String>,
    pub start_time: Option<String>,
    pub done_time: Option<String>,
    pub assets: SkyBoostAssets,
}

#[derive(Debug, Serialize, Default)]
pub struct SkyBoostAssets {
    pub balances: BTreeMap<String, u64>,
    pub outfits: Option<Vec<SkyBoostOutfit>>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SkyBoostOutfit {
    pub id: String,
    pub name: String,
    pub kind: String,
}

fn parse_assets(data: &Value) -> SkyBoostAssets {
    let mut assets = SkyBoostAssets::default();
    for key in ["candles", "season_candle", "heart", "prestige"] {
        if let Some(amount) = data["todayCurrencyChange"][key]["after"].as_u64() {
            assets.balances.insert(key.into(), amount);
        }
    }
    let raw = &data["accountAssets"];
    if raw.is_null() { return assets; }
    let decoded;
    let inventory = if let Some(text) = raw.as_str() {
        decoded = serde_json::from_str::<Value>(text).unwrap_or(Value::Null);
        &decoded
    } else { raw };
    let inventory = if inventory.get("data").is_some() {
        if inventory["success"].as_bool() == Some(false) {
            assets.warnings.push("外观查询未成功，保留上次外观记录".into());
            return assets;
        }
        &inventory["data"]
    } else { inventory };
    let rows = &inventory["outfits"];
    let parsed = (|| {
        let rows = rows.as_array()?;
        if rows.len() > 10_000 { return None; }
        let mut seen = HashSet::new();
        let mut outfits = Vec::new();
        for row in rows {
            let id = row["id"].as_u64()?.to_string();
            let name = row["name"].as_str()?;
            let kind = row["outfit_type"].as_str()?;
            if name.is_empty() || name.len() > 180 || kind.is_empty() || kind.len() > 32 || !seen.insert(id.clone()) {
                return None;
            }
            outfits.push(SkyBoostOutfit { id, name: name.into(), kind: kind.into() });
        }
        if let Some(count) = inventory["count"].as_u64() {
            if count != outfits.len() as u64 { return None; }
        }
        Some(outfits)
    })();
    if parsed.is_none() {
        assets.warnings.push("外观反馈不完整，保留上次外观记录".into());
    }
    assets.outfits = parsed;
    assets
}

fn mz_order(link: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(link.trim()).map_err(|_| "反馈链接格式不正确")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str() != Some("dp.mz997.com")
        || url.port() != Some(92)
        || !url.username().is_empty() || url.password().is_some()
    {
        return Err("该反馈平台暂未适配，可保留链接并打开原页查看".into());
    }
    let fragment = url.fragment().unwrap_or("");
    let (route, query) = fragment.split_once('?').ok_or("请粘贴完整的订单反馈链接")?;
    if route != "/order-search" { return Err("请使用订单查询页链接".into()); }
    let parsed = reqwest::Url::parse(&format!("https://query.invalid/?{query}")).map_err(|_| "链接参数无效")?;
    let orders: Vec<_> = parsed.query_pairs().filter(|(k, _)| k == "order").collect();
    if orders.len() != 1 { return Err("反馈链接必须包含一个订单号".into()); }
    let order = orders[0].1.to_string();
    if order.is_empty() || order.len() > 160 || !order.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("订单号格式不正确".into());
    }
    Ok(order)
}

fn encrypted_query(order: &str, ts: i64) -> Result<(String, String), String> {
    let fail = |_| "无法生成代练查询请求".to_string();
    let mut secret = [0u8; 32];
    SystemRandom::new().fill(&mut secret).map_err(fail)?;
    let aes = UnboundCipherKey::new(&AES_256, &secret).map_err(fail)?;
    let cipher = PaddedBlockEncryptingKey::cbc_pkcs7(aes).map_err(fail)?;
    let mut data = serde_json::json!({ "order": order, "_ts": ts }).to_string().into_bytes();
    let context = cipher.encrypt(&mut data).map_err(fail)?;
    let iv: &[u8] = (&context).try_into().map_err(fail)?;
    let body = STANDARD.encode([iv, data.as_slice()].concat());
    let der = STANDARD.decode(MZ_PUBLIC_KEY).map_err(|_| "代练平台公钥无效")?;
    let key = PublicEncryptingKey::from_der(&der).map_err(|_| "代练平台公钥无效")?;
    let rsa = Pkcs1PublicEncryptingKey::new(key).map_err(fail)?;
    let mut buffer = vec![0; rsa.ciphertext_size()];
    let wrapped = rsa.encrypt(STANDARD.encode(secret).as_bytes(), &mut buffer).map_err(fail)?;
    Ok((body, STANDARD.encode(wrapped)))
}

fn parse_feedback(value: &Value, fetched_at: chrono::DateTime<FixedOffset>) -> Result<SkyBoostFeedback, String> {
    if value["code"].as_i64() != Some(200) {
        // 不把服务器的任意文本或账号信息写进日志/错误。
        return Err("代练平台查询失败，请打开反馈原页核对链接与订单状态".into());
    }
    let data = &value["data"];
    let status = data["todayState"].as_str().filter(|s| !s.is_empty())
        .ok_or("反馈缺少今日完成状态，本地记录未改动")?;
    let order_state = data["state"].as_str().ok_or("反馈缺少订单状态")?;
    let order_text = |field: &str| {
        data[field].as_str().map(str::trim)
            .filter(|value| !value.is_empty() && value.chars().count() <= 64)
            .map(str::to_string)
    };
    Ok(SkyBoostFeedback {
        day: fetched_at.format("%Y-%m-%d").to_string(),
        fetched_at: fetched_at.to_rfc3339(),
        status: status.to_string(),
        completed: status == "今日代练已完成" && order_state == "正常",
        order_state: order_state.to_string(),
        days: data["days"].as_i64()
            .or_else(|| data["days"].as_str().and_then(|value| value.parse().ok())),
        expiration_date: order_text("expirationDate"),
        start_time: order_text("startTime"),
        done_time: order_text("doneTime"),
        assets: parse_assets(data),
    })
}

#[tauri::command]
pub async fn fetch_sky_boost_feedback(link: String) -> Result<SkyBoostFeedback, String> {
    let order = mz_order(&link)?;
    let offset = FixedOffset::east_opt(8 * 3600).unwrap();
    let started = Utc::now().with_timezone(&offset);
    let ts = started.timestamp();
    let (body, key) = encrypted_query(&order, ts)?;
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(8))
        .timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| "无法建立反馈连接")?;
    let response = client.post("http://dp.mz997.com:8006/Self/queryOrder")
        .header("X-ENC", "2").header("X-KEY", key).header("X-TS", ts.to_string())
        .json(&body).send().await.map_err(|error| {
            if error.is_timeout() { "代练平台响应超时，稍后可手动刷新" } else { "反馈连接失败，稍后可重试" }
        })?;
    if !response.status().is_success() { return Err("代练平台暂不可用".into()); }
    let value: Value = response.json().await.map_err(|_| "反馈格式已变化，本地记录未改动")?;
    let fetched = Utc::now().with_timezone(&offset);
    if started.date_naive() != fetched.date_naive() { return Err("查询跨过换日，请重新同步".into()); }
    parse_feedback(&value, fetched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_completion_of_normal_order_counts() {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(28800).unwrap());
        for (status, state, done) in [
            ("今日代练已完成", "正常", true),
            ("跑手正在号上代练，请勿顶号。", "正常", false),
            ("正在等待跑手上号代练..", "正常", false),
            ("今日代练已完成", "异常", false),
        ] {
            let value = serde_json::json!({"code": 200, "data": {"todayState": status, "state": state}});
            assert_eq!(parse_feedback(&value, now).unwrap().completed, done);
        }
        assert!(parse_feedback(&serde_json::json!({"code":200,"data":{}}), now).is_err());
        let details = parse_feedback(&serde_json::json!({"code":200,"data":{
            "todayState":"跑手正在号上代练","state":"正常","days":"3","expirationDate":"2026-11-24"
        }}), now).unwrap();
        assert_eq!(details.days, Some(3));
        assert_eq!(details.expiration_date.as_deref(), Some("2026-11-24"));
        assert!(mz_order("http://localhost:92/#/order-search?order=test").is_err());
        assert!(mz_order("http://dp.mz997.com:92/#/order-search?order=test&order=other").is_err());
    }

    // 使用用户给出的链接手动运行；不把订单号或账号保存为测试夹具。
    #[tokio::test]
    #[ignore]
    async fn live_query() {
        let link = std::env::var("WAYPOINT_SKY_FEEDBACK_LINK").expect("set feedback link");
        let feedback = fetch_sky_boost_feedback(link).await.unwrap();
        println!("feedback: {} {} completed={}", feedback.day, feedback.status, feedback.completed);
        println!("assets: balances={:?} outfits={} warnings={:?}", feedback.assets.balances,
            feedback.assets.outfits.as_ref().map_or(0, Vec::len), feedback.assets.warnings);
    }

    #[test]
    fn assets_keep_exact_owned_ids_without_ack_or_missing_balance_inference() {
        let inventory = serde_json::json!({"success":true,"data":{"count":2,"outfits":[
            {"id":1,"name":"CharSkyKid_Body_AP21Coat","outfit_type":"body","ack":false},
            {"id":2,"name":"CharSkyKid_Wing_AP21LinenScarf","outfit_type":"wing","ack":true}
        ]}});
        for raw in [inventory.clone(), Value::String(inventory.to_string()), inventory["data"].clone()] {
            let assets = parse_assets(&serde_json::json!({"accountAssets":raw,
                "todayCurrencyChange":{"candles":{"after":457},"heart":{"after":0},"prestige":{"after":-1}}}));
            assert_eq!(assets.outfits.unwrap().len(), 2);
            assert_eq!(assets.balances.get("candles"), Some(&457));
            assert_eq!(assets.balances.get("heart"), Some(&0));
            assert!(!assets.balances.contains_key("prestige"));
        }
        let mut invalid = inventory.clone();
        invalid["data"]["count"] = 3.into();
        let assets = parse_assets(&serde_json::json!({"accountAssets":invalid}));
        assert!(assets.outfits.is_none());
        assert!(!assets.warnings.is_empty());
        assert!(parse_assets(&serde_json::json!({})).outfits.is_none());
    }
}
