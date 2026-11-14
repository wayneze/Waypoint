//! 按偏好发现与分地区报价。作品 ID 和商店商品 ID 保持分离，不用近似标题自动绑定价格。
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use scraper::{Html, Selector};
use std::sync::OnceLock;
use crate::game_library::{client, response, bangumi_request, bgm_metadata, Metadata};

const ITAD_KEY: &str = "itad_api_key";
const SHOPS: &str = "61,16,35"; // Steam, Epic, GOG

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Money { pub amount: f64, pub currency: String }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Offer {
    pub country: String,
    pub shop_id: u64,
    pub shop: String,
    pub price: Money,
    pub regular: Option<Money>,
    pub store_low: Option<Money>,
    pub discount: f64,
    pub url: String,
    pub updated_at: String,
    pub observed_at: String,
    pub expires_at: Option<String>,
    pub drm: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketGame {
    pub id: String,
    pub title: String,
    pub cover: String,
    pub url: String,
    pub steam_id: Option<String>,
    pub itad_id: Option<String>,
    pub metadata: Option<Metadata>,
    pub offer: Option<Offer>,
}
#[derive(Debug, Clone, Serialize)]
pub struct MarketPage { pub games: Vec<MarketGame>, pub next_offset: Option<u32>, pub warning: Option<String> }
impl MarketPage {
    fn single(games: Vec<MarketGame>) -> Self { Self {games,next_offset:None,warning:None} }
}
#[derive(Debug, Clone, Serialize)]
pub struct Quotes {
    pub country: String,
    pub offers: Vec<Offer>,
    pub itad_id: Option<String>,
    pub checked_at: String,
    pub warning: Option<String>,
}
fn text(v: &Value, key: &str) -> String { v[key].as_str().unwrap_or_default().into() }
fn optional(v: &Value, key: &str) -> Option<String> { v[key].as_str().filter(|s| !s.is_empty()).map(str::to_owned) }
fn money(v: &Value) -> Option<Money> {
    let amount = v["amount"].as_f64()?;
    let currency = v["currency"].as_str()?;
    (amount.is_finite() && amount >= 0.0 && currency.len() == 3).then(|| Money { amount, currency: currency.into() })
}
fn offer(v: &Value, country: &str) -> Option<Offer> {
    let shop_id = v["shop"]["id"].as_u64()?;
    if ![61,16,35].contains(&shop_id) || !text(v,"url").starts_with("https://") { return None; }
    Some(Offer {
        country:country.into(), shop_id, shop: text(&v["shop"],"name"), price: money(&v["price"])?,
        regular: money(&v["regular"]), store_low: money(&v["storeLow"]),
        discount: v["cut"].as_f64().unwrap_or_default(), url: text(v,"url"),
        updated_at: text(v,"timestamp"), observed_at: chrono::Utc::now().to_rfc3339(), expires_at: optional(v,"expiry"),
        drm: v["drm"].as_array().into_iter().flatten().filter_map(|d| optional(d,"name")).collect(),
    })
}
fn from_meta(meta: Metadata) -> MarketGame {
    let steam_id = if meta.source == "steam" { Some(meta.source_id.clone()) }
        else { meta.sources.iter().find(|s| s.source == "steam").map(|s| s.source_id.clone()) };
    MarketGame { id:format!("{}:{}",meta.source,meta.source_id),title:meta.title.clone(),
        cover:meta.cover.clone(),url:meta.url.clone(),steam_id, itad_id:None, metadata:Some(meta),offer:None }
}
fn itad_key() -> Result<String,String> {
    crate::secrets::load_secret(ITAD_KEY,Some(crate::secrets::STORED_MARK.into()))
        .filter(|s| !s.is_empty()).ok_or("请先在「价格来源」连接 IsThereAnyDeal，以读取 Epic、GOG 和历史价格。".into())
}
fn itad_request(path: &str) -> Result<reqwest::RequestBuilder,String> {
    Ok(client()?.get(format!("https://api.isthereanydeal.com{path}")).header("ITAD-API-Key",itad_key()?))
}
#[tauri::command]
pub fn market_connected() -> bool { itad_key().is_ok() }

#[tauri::command]
pub async fn connect_market(key: String) -> Result<(),String> {
    let key = key.trim();
    if key.is_empty() { crate::secrets::store_secret(ITAD_KEY,"")?; return Ok(()); }
    // 先用只读查询验证，失败不替换原凭据。
    response(client()?.get("https://api.isthereanydeal.com/games/lookup/v1")
        .query(&[("appid","367520")]).header("ITAD-API-Key",key)).await?;
    crate::secrets::store_secret(ITAD_KEY,key)?;
    Ok(())
}

fn options() -> &'static Value {
    static OPTIONS: OnceLock<Value> = OnceLock::new();
    OPTIONS.get_or_init(|| serde_json::from_str(include_str!("../../src/data/discoveryOptions.json")).expect("discovery catalog"))
}
fn region(country: &str) -> Result<&'static Value,String> {
    options()["regions"].as_array().unwrap().iter().find(|r|r["id"]==country).ok_or("不支持的商店地区".into())
}
fn interests(ids: &[String]) -> Result<Vec<&'static Value>,String> {
    if ids.len()>options()["interests"].as_array().unwrap().len() { return Err("偏好数量超出范围".into()); }
    let mut selected=vec![];
    for id in ids {
        let item=options()["interests"].as_array().unwrap().iter().find(|i|i["id"]==*id).ok_or("不支持的游戏偏好")?;
        if !selected.contains(&item) { selected.push(item); }
    }
    Ok(selected)
}
fn steam_url(id: &str,country:&str) -> String { format!("https://store.steampowered.com/app/{id}/?cc={}&l=schinese",country.to_lowercase()) }
fn next_page(v:&Value,offset:u32,limit:u32)->Option<u32> {
    let step=v["limit"].as_u64().unwrap_or(u64::from(limit)) as u32;
    let next=offset+step;
    (step>0 && next<=1000 && u64::from(next)<v["total"].as_u64().unwrap_or_default()).then_some(next)
}
async fn bangumi_page(query:&str,offset:u32,limit:u32,interest:Option<&Value>)->Result<MarketPage,String> {
    let v=if query.is_empty() && interest.is_none() {
        response(bangumi_request(client()?.get("https://api.bgm.tv/v0/subjects"))
            .query(&[("type","4"),("sort","rank"),("limit",&limit.to_string()),("offset",&offset.to_string())])).await?
    } else {
        let mut filter=json!({"type":[4],"nsfw":true});
        if let Some(i)=interest {filter["tag"]=json!([i["bangumi"]]);}
        if query.is_empty() {filter["rank"]=json!([">0"]);}
        response(bangumi_request(client()?.post("https://api.bgm.tv/v0/search/subjects"))
            .query(&[("limit",limit),("offset",offset)])
            .json(&json!({"keyword":query,"sort":if query.is_empty(){"rank"}else{"match"},"filter":filter}))).await?
    };
    let rows=v["data"].as_array().ok_or("Bangumi 没有返回有效作品列表")?;
    Ok(MarketPage{games:rows.iter().map(|r|from_meta(bgm_metadata(r))).collect(),next_offset:next_page(&v,offset,limit),warning:None})
}
fn steam_page(v:&Value,country:&str,offset:u32,limit:u32,deals:bool)->Result<MarketPage,String> {
    let html=v["results_html"].as_str().ok_or("Steam 没有返回有效作品列表")?;
    let doc=Html::parse_fragment(html);
    let rows=Selector::parse("a.search_result_row[data-ds-appid]").unwrap();
    let page_size=doc.select(&Selector::parse("a.search_result_row").unwrap()).count() as u32;
    let title=Selector::parse(".title").unwrap();
    let image=Selector::parse(".search_capsule img").unwrap();
    let price=Selector::parse(".search_price_discount_combined[data-price-final]").unwrap();
    let discount=Selector::parse(".discount_block[data-bundlediscount='0']").unwrap();
    let currency=text(region(country)?,"currency");
    let games=doc.select(&rows).filter_map(|r| {
        let id=r.value().attr("data-ds-appid")?;
        if validate_id(id,true).is_err() {return None;}
        let name=r.select(&title).next()?.text().collect::<String>();
        let cover=r.select(&image).next().and_then(|i|i.value().attr("src")).unwrap_or_default();
        let mut game=from_meta(Metadata{source:"steam".into(),source_id:id.into(),title:name,cover:cover.into(),url:steam_url(id,country),..Default::default()});
        if deals {
            // 外层价格属于本篇；内层可能是推荐捆绑包，不可把其折扣价当成本篇价。
            let amount=r.select(&price).next()?.value().attr("data-price-final")?.parse::<f64>().ok()?/100.0;
            let cut=r.select(&discount).next().and_then(|d|d.value().attr("data-discount")).and_then(|s|s.parse::<f64>().ok()).unwrap_or_default();
            game.offer=Some(Offer{country:country.into(),shop_id:61,shop:"Steam".into(),price:Money{amount,currency:currency.clone()},regular:None,store_low:None,
                discount:cut,url:game.url.clone(),updated_at:chrono::Utc::now().to_rfc3339(),observed_at:chrono::Utc::now().to_rfc3339(),expires_at:None,drm:vec!["Steam".into()]});
        }
        Some(game)
    }).collect();
    Ok(MarketPage{games,next_offset:next_page(&json!({"total":v["total_count"],"limit":page_size}),offset,limit),warning:None})
}
async fn steam_search(query:&str,offset:u32,limit:u32,country:&str,interest:Option<&Value>,deals:bool)->Result<MarketPage,String> {
    let mut request=client()?.get("https://store.steampowered.com/search/results/")
        .query(&[("term",query),("start",&offset.to_string()),("count",&limit.to_string()),("infinite","1"),("category1","998"),("cc",&country.to_lowercase()),("l","schinese")]);
    if let Some(i)=interest {request=request.query(&[("tags",i["steam"].as_u64().unwrap())]);}
    if deals {request=request.query(&[("specials","1")]);}
    steam_page(&response(request).await?,country,offset,limit,deals)
}
async fn preference_pages(source:&str,query:&str,offset:u32,country:&str,selected:Vec<&'static Value>,deals:bool)->Result<MarketPage,String> {
    let filters:Vec<_>=if selected.is_empty(){vec![None]}else{selected.into_iter().map(Some).collect()};
    let limit=if source=="steam"{25}else{20};
    let mut jobs=tokio::task::JoinSet::new();
    for (index,filter) in filters.into_iter().enumerate() {
        let (source,query,country)=(source.to_owned(),query.to_owned(),country.to_owned());
        jobs.spawn(async move {
            let page=if source=="bangumi"{bangumi_page(&query,offset,limit,filter).await}else{steam_search(&query,offset,limit,&country,filter,deals).await};
            let label=filter.and_then(|f|f["label"].as_str()).unwrap_or("全部类型").to_string();
            (index,label,page)
        });
    }
    let mut results=vec![];
    while let Some(done)=jobs.join_next().await {results.push(done.map_err(|e|e.to_string())?);}
    let mut page=merge_pages(results)?;
    if source=="steam" && !page.games.is_empty() {
        // 官方素材接口一次取整页，使用实际文件名（含本地化和限时活动图），不猜 CDN 路径。
        let input=json!({"ids":page.games.iter().filter_map(|g|g.steam_id.as_ref()?.parse::<u64>().ok().map(|id|json!({"appid":id}))).collect::<Vec<_>>(),
            "context":{"language":"schinese","country_code":country},"data_request":{"include_assets":true}});
        match response(client()?.get("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/")
            .query(&[("input_json",input.to_string())])).await {
            Ok(value)=>apply_steam_assets(&mut page,&value),
            Err(_)=>{
                let warning="高清封面暂未取得，先显示商店缩略图。";
                page.warning=Some(format!("{}{}",page.warning.unwrap_or_default(),warning));
            }
        }
    }
    Ok(page)
}
fn apply_steam_assets(page:&mut MarketPage,value:&Value) {
    for item in value["response"]["store_items"].as_array().into_iter().flatten() {
        let Some(id)=item["appid"].as_u64().map(|id|id.to_string()) else {continue;};
        let Some(game)=page.games.iter_mut().find(|g|g.steam_id.as_ref()==Some(&id)) else {continue;};
        let assets=&item["assets"];
        let Some(file)=assets["header_2x"].as_str().filter(|s|!s.is_empty()).or_else(||assets["header"].as_str().filter(|s|!s.is_empty())) else {continue;};
        let Some(format)=assets["asset_url_format"].as_str().filter(|s|s.starts_with("steam/apps/") && s.contains("${FILENAME}")) else {continue;};
        game.cover=format!("https://shared.fastly.steamstatic.com/store_item_assets/{}",format.replace("${FILENAME}",file));
        if let Some(meta)=game.metadata.as_mut(){meta.cover=game.cover.clone();}
    }
}
fn merge_pages(results: Vec<(usize,String,Result<MarketPage,String>)>) -> Result<MarketPage,String> {
    let mut pages=vec![];
    let mut errors=vec![];
    for (index,label,result) in results {
        match result {Ok(page)=>pages.push((index,page)),Err(error)=>errors.push(format!("{label}：{error}"))}
    }
    if pages.is_empty(){return Err(errors.join("\n"));}
    pages.sort_by_key(|p|p.0);
    let next_offset=pages.iter().filter_map(|p|p.1.next_offset).min();
    let mut games=vec![];
    // 多种偏好按轮次交错；同一来源作品只出现一次，不把第一类挤占整页。
    for row in 0..pages.iter().map(|p|p.1.games.len()).max().unwrap_or_default() {
        for (_,page) in &pages {
            if let Some(game)=page.games.get(row) {if !games.iter().any(|g:&MarketGame|g.id==game.id){games.push(game.clone());}}
        }
    }
    Ok(MarketPage{games,next_offset,warning:(!errors.is_empty()).then(||format!("部分类型暂未取得，已显示其余结果。{}",errors.join("；")))})
}
#[tauri::command]
pub async fn discover_games(source:String,query:String,offset:u32,country:String,interests:Vec<String>)->Result<MarketPage,String> {
    region(&country)?;
    let selected=self::interests(&interests)?;
    let query=query.trim();
    if query.len()>300 || offset>1000 {return Err("搜索条件超出范围".into());}
    match source.as_str() {
        "bangumi"|"steam"=>preference_pages(&source,query,offset,&country,selected,false).await,
        "itad"=>{
            if query.is_empty(){return Err("请输入商店中的作品名".into());}
            let v=response(itad_request("/games/search/v1")?.query(&[("title",query),("results","12")])).await?;
            Ok(MarketPage::single(v.as_array().ok_or("价格来源没有返回有效列表")?.iter().filter(|r|r["type"]=="game").map(|r|MarketGame{
                id:format!("itad:{}",text(r,"id")),title:text(r,"title"),cover:text(&r["assets"],"boxart"),url:format!("https://isthereanydeal.com/game/{}/",text(r,"slug")),
                steam_id:None,itad_id:optional(r,"id"),metadata:None,offer:None,
            }).collect()))
        }
        _=>Err("不支持的发现来源".into()),
    }
}
fn deal_request(country:&str,offset:u32,selected:&[&Value])->Value {
    let mut filter=json!({"type":[1]});
    if !selected.is_empty(){filter["tagsUnion"]=json!(selected.iter().map(|i|&i["itad"]).collect::<Vec<_>>());}
    json!({"country":country,"shops":[61,16,35],"limit":24,"offset":offset,"sort":"-cut","nondeals":false,"mature":true,"filter":filter})
}
#[tauri::command]
pub async fn discover_deals(offset:u32,country:String,interests:Vec<String>)->Result<MarketPage,String> {
    region(&country)?;
    let selected=self::interests(&interests)?;
    if offset>1000 {return Err("页数超出范围".into());}
    if !market_connected(){return preference_pages("steam","",offset,&country,selected,true).await;}
    let v=response(client()?.post("https://api.isthereanydeal.com/deals/v2").header("ITAD-API-Key",itad_key()?)
        .json(&deal_request(&country,offset,&selected))).await?;
    let games=v["list"].as_array().ok_or("价格来源没有返回有效优惠列表")?.iter().filter(|r|r["type"]=="game").filter_map(|r|{
        let price=offer(&r["deal"],&country)?;
        Some(MarketGame{id:format!("itad:{}",text(r,"id")),title:text(r,"title"),cover:text(&r["assets"],"boxart"),url:format!("https://isthereanydeal.com/game/{}/",text(r,"slug")),steam_id:None,itad_id:optional(r,"id"),metadata:None,offer:Some(price)})
    }).collect();
    Ok(MarketPage{games,next_offset:v["nextOffset"].as_u64().filter(|n|v["hasMore"]==true && *n>u64::from(offset) && *n<=1000).map(|n|n as u32),warning:None})
}
fn validate_id(id: &str, steam: bool) -> Result<(),String> {
    if id.is_empty() || if steam { !id.bytes().all(|b| b.is_ascii_digit()) }
        else { id.len()!=36 || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') } {
        return Err("作品标识无效".into());
    }
    Ok(())
}
fn steam_offer(v: &Value, id: &str, country: &str) -> Result<Vec<Offer>,String> {
    if v[id]["success"] != true { return Err("Steam 当前地区未返回此作品的可购信息".into()); }
    let d = &v[id]["data"];
    let price = if d["is_free"] == true { Some(Money { amount:0.0,currency:text(region(country)?,"currency") }) }
        else { d["price_overview"]["final"].as_f64().map(|c|Money {amount:c/100.0,currency:text(&d["price_overview"],"currency")}) };
    let Some(price) = price else { return Ok(Vec::new()) };
    Ok(vec![Offer { country:country.into(), shop_id:61,shop:"Steam".into(),price,
        regular:d["price_overview"]["initial"].as_f64().map(|c|Money { amount:c/100.0,currency:text(&d["price_overview"],"currency") }),
        store_low:None,discount:d["price_overview"]["discount_percent"].as_f64().unwrap_or_default(),
        url:steam_url(id,country),updated_at:chrono::Utc::now().to_rfc3339(),
        observed_at:chrono::Utc::now().to_rfc3339(),expires_at:None,drm:vec!["Steam".into()] }])
}
#[tauri::command]
pub async fn game_prices(steam_id: Option<String>, itad_id: Option<String>, country: String) -> Result<Quotes,String> {
    region(&country)?;
    if let Some(id)=&steam_id { validate_id(id,true)?; }
    if let Some(id)=&itad_id { validate_id(id,false)?; }
    let mut result=Quotes {country:country.clone(),offers:vec![],itad_id,checked_at:chrono::Utc::now().to_rfc3339(),warning:None};
    if market_connected() {
        let fetched: Result<Vec<Offer>,String> = async {
            if result.itad_id.is_none() {
                let steam=steam_id.as_deref().ok_or("这部作品尚未绑定商店条目，请搜索并选择准确的商品版本。")?;
                let found=response(itad_request("/games/lookup/v1")?.query(&[("appid",steam)])).await?;
                result.itad_id=optional(&found["game"],"id");
            }
            let id=result.itad_id.as_ref().ok_or("价格来源未收录这部作品")?;
            let data=response(client()?.post("https://api.isthereanydeal.com/games/prices/v3")
                .header("ITAD-API-Key",itad_key()?).query(&[("country",country.as_str()),("shops",SHOPS),("vouchers","false")])
                .json(&json!([id]))).await?;
            let rows=data.as_array().ok_or("价格来源返回了无效报价")?;
            let row=rows.iter().find(|r|r["id"].as_str()==Some(id.as_str())).ok_or("价格来源未返回该作品")?;
            Ok(row["deals"].as_array().ok_or("价格来源缺少报价列表")?.iter().filter_map(|v|offer(v,&country)).collect())
        }.await;
        match fetched { Ok(offers)=>result.offers=offers,Err(e)=>result.warning=Some(e) }
    } else { result.warning=Some("尚未连接跨店价格源；当前仅查询所选地区的 Steam，没有历史价格。".into()); }
    if result.offers.is_empty() {
        if let Some(id)=steam_id {
            let v=response(client()?.get("https://store.steampowered.com/api/appdetails/")
                .query(&[("appids",id.as_str()),("cc",country.as_str()),("l","schinese")])).await?;
            result.offers=steam_offer(&v,&id,&country)?;
        } else if let Some(warning)=result.warning { return Err(warning); }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "read-only live discovery acceptance requires network"]
    async fn live_discovery_public_sources() {
        let (games, steam, deals, prices, vn) = tokio::join!(
            discover_games("bangumi".into(), "".into(), 0,"JP".into(),vec!["visual_novel".into(),"story".into()]),
            discover_games("steam".into(), "".into(), 0,"JP".into(),vec!["visual_novel".into()]),
            discover_deals(0,"JP".into(),vec!["visual_novel".into(),"story".into()]),
            game_prices(Some("367520".into()),None,"JP".into()),
            crate::game_library::search_game_metadata("vndb".into(),"v810".into()),
        );
        let (games,steam,deals,prices,vn)=(games.unwrap(),steam.unwrap(),deals.unwrap(),prices.unwrap(),vn.unwrap());
        assert!(!games.games.is_empty() && !steam.games.is_empty() && !deals.games.is_empty() && !vn.is_empty());
        assert!(prices.offers.iter().any(|o|o.country=="JP" && o.price.currency=="JPY"));
        let (page_two,puzzles)=tokio::join!(
            discover_games("steam".into(),"".into(),steam.next_offset.expect("Steam next page"),"JP".into(),vec!["visual_novel".into()]),
            discover_games("bangumi".into(),"".into(),0,"JP".into(),vec!["puzzle".into()]),
        );
        let (page_two,puzzles)=(page_two.unwrap(),puzzles.unwrap());
        assert!(!page_two.games.is_empty() && !puzzles.games.is_empty());
        assert_ne!(steam.games[0].id,page_two.games[0].id);
        assert!(puzzles.games.iter().all(|g|g.metadata.as_ref().unwrap().tags.iter().any(|t|t=="解谜")));
        println!("Bangumi={} Steam={} deals={} Steam price={:?}; VNDB={}",games.games.len(),steam.games.len(),deals.games.len(),prices.offers[0].price,vn[0].title);
        println!("Steam next offset={:?}, page2={}; Bangumi puzzle={}",steam.next_offset,page_two.games.len(),puzzles.games.len());
        let cache=tempfile::tempdir().unwrap();
        let mut covers=tokio::task::JoinSet::new();
        for game in games.games.iter().take(6).chain(deals.games.iter().take(6)) {
            let (url,path)=(game.cover.clone(),cache.path().to_path_buf());
            covers.spawn(async move {
                let result=crate::posters::load_poster(&path,&url,None).await;
                assert!(result.as_ref().is_ok_and(|(mime,bytes)|mime.starts_with("image/") && bytes.len()>100),"{url}: {result:?}");
            });
        }
        while let Some(done)=covers.join_next().await {done.unwrap();}
        if let Ok(path)=std::env::var("WAYPOINT_DISCOVERY_OUTPUT") {
            std::fs::write(path,serde_json::to_string(&json!({"games":games,"steam":steam,"deals":deals,"prices":prices,"vn":vn,"page_two":page_two,"puzzles":puzzles})).unwrap()).unwrap();
        }
    }
    #[test]
    fn unavailable_is_not_free_and_edition_ids_are_required() {
        assert!(steam_offer(&json!({"1":{"success":true,"data":{}}}),"1","JP").unwrap().is_empty());
        assert!(steam_offer(&json!({"1":{"success":false}}),"1","JP").is_err());
        let free=steam_offer(&json!({"1":{"success":true,"data":{"is_free":true}}}),"1","JP").unwrap();
        assert_eq!(free[0].price.amount,0.0);
        assert_eq!(free[0].price.currency,"JPY");
        assert_eq!(free[0].country,"JP");
        assert!(validate_id("game title",true).is_err());
    }
    #[test]
    fn quotes_keep_currency_store_history_and_expiry() {
        let parsed=offer(&json!({"shop":{"id":35,"name":"GOG"},"price":{"amount":3.99,"currency":"USD"},
            "storeLow":{"amount":2.99,"currency":"USD"},"cut":50,"url":"https://itad.link/offer",
            "timestamp":"2026-11-08T10:00:00Z","expiry":"2026-11-09T10:00:00Z","drm":[]}),"JP").unwrap();
        assert_eq!(parsed.price.currency,"USD");
        assert_eq!(parsed.country,"JP");
        assert_eq!(parsed.store_low.unwrap().amount,2.99);
        assert!(parsed.expires_at.is_some());
        assert!(money(&json!({"amount":null,"currency":"CNY"})).is_none());
    }
    #[test]
    fn preferences_and_region_reach_the_market_query() {
        assert!(region("CN").is_err());
        assert!(interests(&["unknown".into()]).is_err());
        let selected=interests(&["visual_novel".into(),"puzzle".into(),"visual_novel".into()]).unwrap();
        let request=deal_request("JP",24,&selected);
        assert_eq!(request["country"],"JP");
        assert_eq!(request["filter"],json!({"type":[1],"tagsUnion":["Visual Novel","Puzzle"]}));
        assert_eq!(deal_request("HK",0,&[])["filter"],json!({"type":[1]}));
    }
    #[test]
    fn one_unavailable_interest_does_not_discard_successful_results() {
        let page=merge_pages(vec![
            (0,"Galgame".into(),Ok(MarketPage::single(vec![from_meta(Metadata{source:"steam".into(),source_id:"1".into(),..Default::default()})]))),
            (1,"剧情".into(),Err("连接中断".into())),
        ]).unwrap();
        assert_eq!(page.games.len(),1);
        assert!(page.warning.unwrap().contains("剧情"));
        assert!(merge_pages(vec![(0,"Galgame".into(),Err("连接中断".into()))]).is_err());
    }
    #[test]
    fn steam_prices_belong_to_the_game_and_pagination_counts_filtered_packages() {
        let html=r#"<a class="search_result_row" data-ds-appid="1">
            <span class="title">Game</span><div class="search_price_discount_combined" data-price-final="170000">
            <div class="discount_block" data-bundlediscount="1" data-discount="90" data-price-final="10000"></div>
            </div></a><a class="search_result_row"><span class="title">Package</span></a>"#;
        let mut page=steam_page(&json!({"results_html":html,"total_count":50}),"JP",25,25,true).unwrap();
        assert_eq!(page.next_offset,Some(27));
        assert_eq!(page.games.len(),1);
        apply_steam_assets(&mut page,&json!({"response":{"store_items":[{"appid":1,"assets":{
            "asset_url_format":"steam/apps/1/${FILENAME}?t=7","header_2x":"hash/header_alt_assets_3_schinese_2x.jpg"}}]}}));
        assert_eq!(page.games[0].cover,"https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/1/hash/header_alt_assets_3_schinese_2x.jpg?t=7");
        let offer=page.games[0].offer.as_ref().unwrap();
        assert_eq!(offer.price.amount,1700.0);
        assert_eq!(offer.price.currency,"JPY");
        assert_eq!(offer.discount,0.0);
        assert!(offer.url.contains("cc=jp"));
    }
}
