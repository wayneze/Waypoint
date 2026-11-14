// 国服光翼查询：官方 queryRoleWingBuff 现需大神 App 签名（868），桌面端走社区只读中转。
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

const OFFICIAL_WING_QUERY: &str = "https://god.gameyw.netease.com/v1/app/gameData/queryRoleWingBuff";
const PROXY_WING_QUERY: &str = "http://bluerosion.vincentzyu233.cn:51024/queryGuangyi";
const LIVE_WING_MAP: &str = "https://s.166.net/config/ds_yy_02/ma75_wing_wings.json";
const BUNDLED_WING_MAP: &str = include_str!("wing_map.json");
const SKY_SERVER: &str = "8000";

const WORLD_REALM_ORDER: &[&str] = &[
    "晨岛",
    "云野",
    "雨林",
    "霞谷",
    "暮土",
    "禁阁",
    "暴风眼",
    "云巢",
    "遇境",
    "破碎虚空",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkyRealmWings {
    pub name: String,
    pub collected: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkyWingBuff {
    pub name: String,
    pub collected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkyRoleWings {
    pub success: bool,
    pub role_id: String,
    pub source: String,
    pub collected: u32,
    pub total: u32,
    pub maps: Vec<SkyRealmWings>,
    #[serde(default)]
    pub buffs: Vec<SkyWingBuff>,
    pub error: String,
}

pub fn normalize_role_id(raw: &str) -> Result<String, String> {
    let id: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    if id.len() < 6 || id.len() > 64 {
        return Err("游戏ID长度不对，请用设置 → 精灵 → 工具指南里的查询ID".into());
    }
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("游戏ID只接受字母和数字".into());
    }
    Ok(id)
}

fn realm_alias(name: &str) -> &str {
    match name {
        "破晓季" => "破碎虚空",
        other => other,
    }
}

fn is_world_light(name: &str, realm: &str) -> bool {
    name.starts_with("l_") && realm != "普通永久" && realm != "复刻永久"
}

fn bundled_wing_map() -> BTreeMap<String, String> {
    serde_json::from_str(BUNDLED_WING_MAP).unwrap_or_default()
}

fn overlay_live_map(base: &mut BTreeMap<String, String>, live: &Value) {
    let Some(rows) = live.as_array() else {
        return;
    };
    for row in rows {
        let Some(name) = row.get("光翼名字").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(realm) = row.get("一级标签").and_then(|v| v.as_str()) else {
            continue;
        };
        if !name.is_empty() && !realm.is_empty() {
            base.insert(name.to_string(), realm.to_string());
        }
    }
}

fn unwrap_result_object(value: &Value) -> Option<Value> {
    if let Some(s) = value.as_str() {
        return serde_json::from_str(s).ok();
    }
    if value.is_object() {
        return Some(value.clone());
    }
    None
}

pub fn parse_wing_payload(root: &Value) -> Result<Vec<(String, bool)>, String> {
    if root.get("success") == Some(&Value::Bool(false)) {
        let msg = root
            .get("result")
            .and_then(|v| v.as_str())
            .or_else(|| root.get("message").and_then(|v| v.as_str()))
            .unwrap_or("查询失败");
        return Err(msg.to_string());
    }

    let data = root.get("data").unwrap_or(root);
    let code = data
        .get("code")
        .and_then(|v| v.as_i64())
        .or_else(|| root.get("code").and_then(|v| v.as_i64()));
    if let Some(code) = code {
        if code != 200 {
            let msg = data
                .get("errmsg")
                .and_then(|v| v.as_str())
                .or_else(|| root.get("errmsg").and_then(|v| v.as_str()))
                .unwrap_or("查询失败");
            return Err(format!("{code}:{msg}"));
        }
    }

    let result = data
        .get("result")
        .or_else(|| root.get("result"))
        .ok_or_else(|| "查询结果为空".to_string())?;
    let parsed = unwrap_result_object(result).ok_or_else(|| "查询结果无法解析".to_string())?;
    let buffs: Vec<SkyWingBuff> = serde_json::from_value(
        parsed.get("wing_buffs").cloned().ok_or("没有光翼列表")?
    ).map_err(|_| "光翼状态不完整或格式不正确，未更新本地记录".to_string())?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(buffs.len());
    for buff in buffs {
        if buff.name.trim().is_empty() || !seen.insert(buff.name.clone()) {
            return Err("光翼列表含空名称或重复条目，未更新本地记录".into());
        }
        out.push((buff.name, buff.collected));
    }
    if out.is_empty() {
        return Err("光翼列表为空".into());
    }
    Ok(out)
}

pub fn summarize_world_wings(
    buffs: &[(String, bool)],
    map: &BTreeMap<String, String>,
) -> (u32, u32, Vec<SkyRealmWings>) {
    let mut buckets: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for (name, collected) in buffs {
        let realm = map
            .get(name)
            .map(|s| realm_alias(s).to_string())
            .unwrap_or_else(|| "未分类".to_string());
        if !is_world_light(name, &realm) {
            continue;
        }
        let entry = buckets.entry(realm).or_insert((0, 0));
        entry.1 += 1;
        if *collected {
            entry.0 += 1;
        }
    }

    let mut collected = 0;
    let mut total = 0;
    let mut maps = Vec::new();
    for name in WORLD_REALM_ORDER {
        if let Some((got, all)) = buckets.remove(*name) {
            collected += got;
            total += all;
            maps.push(SkyRealmWings {
                name: (*name).to_string(),
                collected: got,
                total: all,
            });
        }
    }
    for (name, (got, all)) in buckets {
        collected += got;
        total += all;
        maps.push(SkyRealmWings {
            name,
            collected: got,
            total: all,
        });
    }
    (collected, total, maps)
}

fn wing_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36")
        .connect_timeout(std::time::Duration::from_secs(6))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn query_official(client: &reqwest::Client, role_id: &str) -> Result<Value, String> {
    let res = client
        .post(OFFICIAL_WING_QUERY)
        .header("content-type", "application/json")
        .header("gl-clienttype", "60")
        .header("origin", "https://act.ds.163.com")
        .header("referer", "https://act.ds.163.com/3af4b8a2832c44a9/")
        .body(serde_json::json!({ "roleId": role_id, "server": SKY_SERVER }).to_string())
        .send()
        .await
        .map_err(|e| format!("官方查询网络错误: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("官方查询 HTTP {}", res.status().as_u16()));
    }
    res.json::<Value>()
        .await
        .map_err(|e| format!("官方查询无法解析: {e}"))
}

async fn query_proxy(client: &reqwest::Client, role_id: &str) -> Result<Value, String> {
    let res = client
        .get(PROXY_WING_QUERY)
        .query(&[("id", role_id)])
        .send()
        .await
        .map_err(|e| {
            let why = if e.is_timeout() || e.is_connect() { "连不上" } else { "请求失败" };
            format!("官方接口要大神 App 签名，社区中转当前{why}。可先在图鉴里手动勾选")
        })?;
    if !res.status().is_success() {
        return Err(format!("中转查询 HTTP {}", res.status().as_u16()));
    }
    res.json::<Value>()
        .await
        .map_err(|e| format!("中转查询无法解析: {e}"))
}

fn official_needs_fallback(err: &str) -> bool {
    err.starts_with("868:") || err.starts_with("806:") || err.contains("升级版本")
}

#[tauri::command]
pub async fn fetch_sky_role_wings(role_id: String) -> Result<SkyRoleWings, String> {
    let role_id = normalize_role_id(&role_id)?;
    let client = wing_client()?;
    let mut map = bundled_wing_map();
    if let Ok(res) = client.get(LIVE_WING_MAP).send().await {
        if let Ok(live) = res.json::<Value>().await {
            overlay_live_map(&mut map, &live);
        }
    }

    let mut source = "official";
    let payload = match query_official(&client, &role_id).await {
        Ok(v) => match parse_wing_payload(&v) {
            Ok(buffs) => buffs,
            Err(err) if official_needs_fallback(&err) => {
                source = "proxy";
                parse_wing_payload(&query_proxy(&client, &role_id).await?)?
            }
            Err(err) => return Err(err),
        },
        Err(_) => {
            source = "proxy";
            parse_wing_payload(&query_proxy(&client, &role_id).await?)?
        }
    };

    let (collected, total, maps) = summarize_world_wings(&payload, &map);
    let buffs = payload
        .into_iter()
        .map(|(name, collected)| SkyWingBuff { name, collected })
        .collect();
    Ok(SkyRoleWings {
        success: true,
        role_id,
        source: source.into(),
        collected,
        total,
        maps,
        buffs,
        error: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_role_id_strips_spaces() {
        assert_eq!(normalize_role_id(" 137106295 ").unwrap(), "137106295");
        assert!(normalize_role_id("12").is_err());
        assert!(normalize_role_id("id with space!").is_err());
    }

    #[test]
    fn parse_proxy_nested_result() {
        let root = serde_json::json!({
            "success": true,
            "roleId": "137106295",
            "data": {
                "code": 200,
                "errmsg": "OK",
                "result": "{\"result\":\"ok\",\"wing_buffs\":[{\"name\":\"l_Rain_0\",\"collected\":true},{\"name\":\"l_Rain_1\",\"collected\":false},{\"name\":\"s_wave\",\"collected\":true}]}"
            }
        });
        let buffs = parse_wing_payload(&root).unwrap();
        assert_eq!(buffs.len(), 3);
        let map = bundled_wing_map();
        let (got, total, maps) = summarize_world_wings(&buffs, &map);
        assert_eq!((got, total), (1, 2));
        let rain = maps.iter().find(|m| m.name == "雨林").unwrap();
        assert_eq!((rain.collected, rain.total), (1, 2));
        assert!(!maps.iter().any(|m| m.name == "普通永久"));
    }

    #[test]
    fn incomplete_or_ambiguous_wings_are_not_treated_as_uncollected() {
        for buffs in [
            serde_json::json!([{"name":"l_Rain_0"}]),
            serde_json::json!([{"name":"l_Rain_0","collected":null}]),
            serde_json::json!([{"name":"l_Rain_0","collected":"false"}]),
            serde_json::json!([{"name":"","collected":false}]),
            serde_json::json!([
                {"name":"l_Rain_0","collected":true},
                {"name":"l_Rain_0","collected":false}
            ]),
        ] {
            let root = serde_json::json!({"code":200,"result":{"wing_buffs":buffs}});
            assert!(parse_wing_payload(&root).is_err());
        }
    }

    #[test]
    fn parse_official_version_gate() {
        let root = serde_json::json!({"code":868,"errmsg":"请升级版本体验最新功能"});
        let err = parse_wing_payload(&root).unwrap_err();
        assert!(err.starts_with("868:"));
        assert!(official_needs_fallback(&err));
    }

    #[test]
    fn role_wings_carries_buffs() {
        let snap = SkyRoleWings {
            success: true,
            role_id: "137106295".into(),
            source: "proxy".into(),
            collected: 1,
            total: 2,
            maps: vec![],
            buffs: vec![SkyWingBuff {
                name: "l_Rain_0".into(),
                collected: true,
            }],
            error: String::new(),
        };
        assert_eq!(snap.buffs[0].name, "l_Rain_0");
        assert!(snap.buffs[0].collected);
    }

    #[test]
    fn void_space_aliases_to_shattering() {
        let buffs = vec![
            ("l_StormEvent_VoidSpace_0".into(), true),
            ("l_StormEvent_VoidSpace_1".into(), false),
        ];
        let map = bundled_wing_map();
        let (_, _, maps) = summarize_world_wings(&buffs, &map);
        let shard = maps.iter().find(|m| m.name == "破碎虚空").unwrap();
        assert_eq!((shard.collected, shard.total), (1, 2));
    }
}
