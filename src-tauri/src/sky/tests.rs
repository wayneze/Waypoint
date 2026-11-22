use super::*;
use super::wiki::*;
use super::weibo::*;
use super::fandom::*;
use super::calendar::*;
use super::seasons::*;
use super::events::*;

#[test]
fn sky_calendar_reprint_group_and_infer() {
    let wt = "{{复刻项|id=246|复刻类型=常规复刻|同类序号=199|开始时间=2026-09-03|结束时间=2026-09-07|先祖中文名=风行领航员|所属季节=风行季}}\n{{复刻项|id=231|复刻类型=常规复刻|同类序号=198|开始时间=2026-08-27|结束时间=2026-08-30|先祖中文名=跳跃舞者|所属季节=感恩季|装扮=面具、手持}}\n{{复刻项|id=230|复刻类型=集体复刻|同类序号=14|开始时间=2026-08-28|结束时间=2026-09-10|先祖中文名=采药人|所属季节=九色鹿季}}\n{{复刻项|id=229|复刻类型=集体复刻|同类序号=14|开始时间=2026-08-28|结束时间=2026-09-10|先祖中文名=猎手|所属季节=九色鹿季}}";
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
    let items = grouped_reprints(extract_reprint_templates(wt), "intl", "https://example", today);
    assert_eq!(items.len(), 3);
    let live = items.iter().find(|i| i.status == "live").unwrap();
    assert!(live.title.contains("集体回归 14"));
    assert_eq!(live.names.len(), 2);
    let ended = items.iter().find(|i| i.status == "ended").unwrap();
    assert_eq!(ended.title, "风行领航员");
    let last = last_regular(&items).unwrap();
    assert_eq!(last.0, chrono::NaiveDate::from_ymd_opt(2026, 9, 3).unwrap());
    let named = std::collections::HashSet::from([last.0]);
    let windows = infer_regular_windows(last, today, &named, 2);
    assert_eq!(windows[0].0, chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap());
    assert_eq!(windows[0].1, chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap());
    assert_eq!(
        parse_ymd_prefix("2023-1-2"),
        chrono::NaiveDate::from_ymd_opt(2023, 1, 2)
    );
    let old = "{{复刻项|id=1|复刻类型=常规复刻|同类序号=1|开始时间=2022-12-29|结束时间=2023-1-2|先祖中文名=沾沾自喜的自恋者|所属季节=小王子季}}";
    let old_items = grouped_reprints(extract_reprint_templates(old), "cn", "https://example", today);
    assert_eq!(old_items.len(), 1);
    assert_eq!(old_items[0].status, "ended");
    assert_eq!(old_items[0].end, "2023-01-02");
    let cn_wt = "{{复刻项|id=1|复刻类型=常规复刻|同类序号=1|开始时间=2026-09-03|结束时间=2026-09-07|先祖中文名=风行领航员|所属季节=风行季}}";
    let cn_items = grouped_reprints(extract_reprint_templates(cn_wt), "cn", "https://example", today);
    let (intl_now, intl_next) = current_intl_reprints(items.clone(), &cn_items, today);
    assert_eq!(intl_now.len(), 1);
    assert!(intl_now[0].title.contains("集体回归"));
    assert!(intl_next.is_empty());
    assert!(intl_now[0].note.contains("未查到国服历史回归记录"));
    let cn_group = "{{复刻项|id=251|复刻类型=集体复刻|同类序号=12|开始时间=2026-09-24|结束时间=2026-10-14|先祖中文名=采药人|所属季节=九色鹿季|装扮=发型、裤子、小型家具}}\n{{复刻项|id=250|复刻类型=集体复刻|同类序号=12|开始时间=2026-09-24|结束时间=2026-10-14|先祖中文名=猎手|所属季节=九色鹿季|装扮=发型、裤子、斗篷}}\n{{复刻项|id=249|复刻类型=集体复刻|同类序号=12|开始时间=2026-09-24|结束时间=2026-10-14|先祖中文名=公主|所属季节=九色鹿季|装扮=发型、裤子、面具、斗篷}}\n{{复刻项|id=248|复刻类型=集体复刻|同类序号=12|开始时间=2026-09-24|结束时间=2026-10-14|先祖中文名=领主|所属季节=九色鹿季|装扮=面具、发饰、斗篷}}";
    let today_gap = chrono::NaiveDate::from_ymd_opt(2026, 9, 15).unwrap();
    let group_items = grouped_reprints(extract_reprint_templates(cn_group), "cn", "https://example", today_gap);
    assert_eq!(group_items.len(), 1);
    let group = &group_items[0];
    assert_eq!(group.status, "upcoming");
    assert_eq!(group.names, vec!["采药人", "猎手", "公主", "领主"]);
    assert!(group.previews.iter().any(|p| p.name == "采药人 · 发型"));
    assert!(group.previews.iter().any(|p| p.name == "猎手 · 斗篷"));
    assert!(!group.previews.iter().any(|p| p.name == "采药人 · 斗篷"));
    assert!(group.previews.iter().any(|p| p.name == "采药人 · 小型家具"));
    let (cn_now, cn_next) = split_now_next(group_items.clone());
    assert!(cn_now.is_empty());
    assert_eq!(cn_next.len(), 1);
    assert_eq!(cn_next[0].names.len(), 4);
    let mut comparison_cn = group_items.clone();
    let mut past = group_items[0].clone();
    past.start = "2025-09-20".into();
    past.end = "2025-09-30".into();
    past.names = vec!["采药人".into()];
    comparison_cn.push(past);
    let (intl_now, _) = current_intl_reprints(items.clone(), &comparison_cn, today);
    assert_eq!(intl_now[0].names, vec!["采药人", "猎手"]);
    assert!(intl_now[0].note.contains("采药人 · 国服上次回归：2025-09-20"));
    assert!(intl_now[0].note.contains("猎手 · 未查到国服历史回归记录"));
    assert!(!intl_now[0].note.contains("2026-09-24"));
    let (now, next) = current_intl_reprints(group_items, &comparison_cn, today_gap);
    assert!(now.is_empty());
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].names.len(), 4);
    let after_end = chrono::NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
    let ended = grouped_reprints(extract_reprint_templates(wt), "intl", "https://example", after_end);
    let (now, next) = current_intl_reprints(ended, &comparison_cn, after_end);
    assert!(now.is_empty() && next.is_empty());
}
#[test]
fn sky_calendar_exchange_and_pack_slots() {
    // 摘自 wiki 珍珠季页：季节先祖板块里嵌兑换图，格子是位置参数，字段数各季不一。
    let wt = "{{季节先祖板块\n|先祖中文名=迟疑渔翁\n|兑换图={{兑换图\n    |先祖=迟疑渔翁\n    |季节=珍珠季\n    |好感等级=1/2/3/4//5\n    |动作;1级|随机魔法;;季蜡;4|邀请\n    |乐谱;;季蜡;18|随机魔法;;季蜡;7|动作;2级;;;季卡\n    |动作;3级;季蜡;28|白色染料;×3;季蜡;8|面具;;;;季卡\n    |裤子;;季蜡;32|好感任务|动作;4级;;;季卡\n    |||季心;;季蜡;3;季卡\n    }}\n}}\n{{兑换图\n    |先祖=渔翁的灯笼\n    |任务|传送|珍珠季logo;;;;季卡\n    |任务||毕业发饰;;季心;1;季卡\n    ||小型家具;;蜡烛;32|\n    }}";
    let trees = parse_exchange_trees(wt);
    assert_eq!(trees.len(), 2);
    assert_eq!(trees[0].spirit, "迟疑渔翁");
    let kinds: Vec<(&str, &str, &str)> = trees[0]
        .items
        .iter()
        .map(|i| (i.slot.as_str(), i.kind.as_str(), i.cost_desc.as_str()))
        .collect();
    assert_eq!(kinds, vec![("兑换", "乐谱", "18季蜡"), ("季卡", "面具", ""), ("兑换", "裤子", "32季蜡")]);
    assert_eq!(trees[0].items[2].candles, 32);
    let lantern: Vec<(&str, &str, u32, u32)> = trees[1]
        .items
        .iter()
        .map(|i| (i.slot.as_str(), i.kind.as_str(), i.candles, i.hearts))
        .collect();
    assert_eq!(lantern, vec![("季卡", "毕业发饰", 0, 1), ("兑换", "小型家具", 32, 0)]);
    assert_eq!(split_cosmetics("发型、斗篷、乐谱"), vec!["发型".to_string(), "斗篷".to_string()]);
    assert!(spirit_icon_url("跳跃舞者").contains("Special:FilePath/UI-"));
    assert!(spirit_icon_url("《花瓶里的十五朵向日葵》").contains("Special:FilePath/"));
    assert!(!spirit_icon_url("《花瓶里的十五朵向日葵》").contains("UI-"));
    let mut reprint = SkyCalendarItem {
        id: "t".into(),
        kind: "reprint".into(),
        region: "intl".into(),
        title: "跳跃舞者".into(),
        names: vec!["跳跃舞者".into()],
        season: "感恩季".into(),
        start: "2026-08-27".into(),
        end: "2026-08-30".into(),
        status: "ended".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: "面具、手持".into(),
        icon: String::new(),
        previews: Vec::new(),
    };
    fill_reprint_media(&mut reprint);
    assert!(reprint.icon.contains("Special:FilePath/UI-"));
    assert!(reprint.previews.iter().any(|p| p.slot == "先祖" && p.name == "跳跃舞者"));
    assert!(reprint.previews.iter().any(|p| p.slot == "兑换" && p.kind == "面具"));
    let wear = reprint.previews.iter().find(|p| p.kind == "面具").unwrap();
    assert!(wear.wear.contains("%E5%B1%95%E7%A4%BA%E5%9B%BE") || wear.wear.contains("展示图") || wear.wear.contains("FilePath"));
    assert_eq!(
        wear_filename_from_url(&cosmetic_wear_url("斗篷", "回旋大师")).as_deref(),
        Some("回旋大师斗篷展示图.png")
    );
    let mut checked = reprint.clone();
    let mut hit = std::collections::HashMap::new();
    hit.insert("跳跃舞者面具展示图.png".into(), "https://patchwiki.example/wear.png".into());
    apply_wear_urls(&mut checked, &hit);
    assert_eq!(
        checked.previews.iter().find(|p| p.kind == "面具").unwrap().wear,
        "https://patchwiki.example/wear.png"
    );
    // wiki 上查不到的拼凑展示图要清掉，不能留一个 404 链接
    assert!(checked.previews.iter().find(|p| p.kind == "手持").unwrap().wear.is_empty());
    let exhibits = parse_exhibit_items("{{展示图|物品|彩环短发|发型}}{{展示图|物品|彩虹斗篷|斗篷}}");
    assert_eq!(exhibits[0], ("彩环短发".into(), "发型".into()));
    assert_eq!(exhibits[1], ("彩虹斗篷".into(), "斗篷".into()));
    let mut event = SkyCalendarItem {
        id: "event-2026年缤纷飞行日".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "缤纷飞行日".into(),
        names: vec![],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    let mut all_exhibits = vec![
        ("蝴蝶风筝".into(), "小型家具".into()),
        ("彩环短发".into(), "发型".into()),
        ("彩虹斗篷".into(), "斗篷".into()),
    ];
    all_exhibits.extend((0..13).map(|i| (format!("外观{i}"), "斗篷".into())));
    for region in ["cn", "intl"] {
        event.region = region.into();
        fill_event_media(&mut event, all_exhibits.clone(), &serde_json::json!({}));
        assert_eq!(event.previews.len(), 16);
        assert!(event.previews.iter().any(|p| p.name == "蝴蝶风筝 · 小型家具"));
        assert!(event.previews.iter().any(|p| p.name == "外观12 · 斗篷"));
    }
    let hair = event
        .previews
        .iter()
        .find(|p| p.name.contains("彩环短发"))
        .unwrap();
    assert_eq!(
        wiki_filename_from_url(&hair.icon).as_deref(),
        Some("UI-发型-彩环短发-无框.png")
    );
    assert_eq!(
        wear_filename_from_url(&hair.wear).as_deref(),
        Some("彩环短发发型展示图.png")
    );
    assert_eq!(
        calendar_event_wiki_pages(&event),
        vec!["2026年缤纷飞行日".to_string(), "缤纷飞行日".to_string()]
    );
    assert_eq!(
        fandom_event_search_pages("Days of Moonlight", "2026"),
        vec!["Days of Moonlight/2026".to_string(), "Days of Moonlight".to_string()]
    );
    assert_eq!(
        fandom_event_search_pages("NetEase/Minecraft", "2026"),
        vec![
            "NetEase/Minecraft/2026".to_string(),
            "NetEase/Minecraft".to_string()
        ]
    );
    let event_files = vec![
        "Rainbow-Tufted-Hair-real.png".into(),
        "Rainbow-Tufted-Hair-icon.png".into(),
        "Days-of-Color-Rainbow-Smock-front.png".into(),
        "Bird-Kite-real.png".into(),
        "Rainbow-Mask-real.png".into(),
    ];
    let used = std::collections::HashSet::new();
    assert_eq!(
        pick_named_wear(&event_files, "发型", "彩环短发", &used).as_deref(),
        Some("Rainbow-Tufted-Hair-real.png")
    );
    assert!(!pick_named_wear(&event_files, "发型", "彩环短发", &used)
        .unwrap()
        .to_ascii_lowercase()
        .contains("icon"));
    assert_eq!(
        pick_named_wear(&event_files, "裤子", "浅彩系腰长袍", &used).as_deref(),
        Some("Days-of-Color-Rainbow-Smock-front.png")
    );
    let shop_files = vec![
        "Dear-Van-Gogh-Starry-Night-Mantle-Cape-front.png".into(),
        "Dear-Van-Gogh-Starry-Night-Mantle-Accessory-front.png".into(),
        "Dear-Van-Gogh-Wheatfield-Cape-front.png".into(),
        "Dear-Van-Gogh-Starry-Night-Kiss-Accessory.png".into(),
    ];
    assert_eq!(
        pick_named_wear(&shop_files, "斗篷", "星夜披风套装礼包", &used).as_deref(),
        Some("Dear-Van-Gogh-Starry-Night-Mantle-Cape-front.png")
    );
    let parsed = parse_fandom_item_names(
        "{{Days Item|Rainbow Tufted Hair||link=x}}{{Icon Name|Rainbow Smock|real}}",
    );
    assert!(parsed.contains(&"Rainbow Tufted Hair".to_string()));
    assert!(parsed.contains(&"Rainbow Smock".to_string()));
    assert_eq!(
        pick_named_wear(&shop_files, "斗篷", "鸦渡麦浪斗篷礼包", &used).as_deref(),
        Some("Dear-Van-Gogh-Wheatfield-Cape-front.png")
    );
    let carnival_files = vec![
        "Carnival-Puzzle-Director-Hair-front.png".into(),
        "Carnival-Puzzle-Director-Outfit-front.png".into(),
        "Carnival-Puzzle-Director-Emote-icon.png".into(),
    ];
    assert_eq!(
        pick_unique_kind_wear(&carnival_files, "发型", &used).as_deref(),
        Some("Carnival-Puzzle-Director-Hair-front.png")
    );
    assert_eq!(
        parse_spirit_en_names(
            "{{季节先祖板块\n|先祖中文名=荷兰的回忆\n|先祖英文名=Dutch Memory\n}}\n{{季节先祖板块\n|先祖中文名=狂欢巧思编导\n|先祖英文名=Carnival Puzzle Director\n}}"
        )
        .get("荷兰的回忆")
        .map(String::as_str),
        Some("Dutch Memory")
    );
    let mut season = SkyCalendarItem {
        id: "season-致梵高季".into(),
        kind: "season".into(),
        region: "cn".into(),
        title: "致梵高季".into(),
        names: vec!["致梵高季".into()],
        season: "致梵高季".into(),
        start: String::new(),
        end: String::new(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    fill_season_media(
        &mut season,
        parse_exchange_trees("{{兑换图|先祖=荷兰的回忆|小型家具;;季蜡;18|斗篷;;季蜡;26}}"),
        &serde_json::json!({}),
    );
    assert!(season.previews.iter().any(|p| p.kind == "小型家具"));
    let mut complete_season = season.clone();
    complete_season.previews.clear();
    let many_trees = (0..9).map(|i| {
        format!("{{{{兑换图|先祖=先祖{i}|小型家具;;季蜡;18|斗篷;;季蜡;26|面具;;;;季卡}}}}")
    }).collect::<String>();
    fill_season_media(&mut complete_season, parse_exchange_trees(&many_trees), &serde_json::json!({}));
    assert_eq!(complete_season.previews.iter().filter(|p| p.slot == "兑换").count(), 18);
    assert_eq!(complete_season.previews.iter().filter(|p| p.slot == "季卡").count(), 9);
    let cape = season
        .previews
        .iter()
        .find(|p| p.name.contains("荷兰的回忆") && p.kind == "斗篷")
        .unwrap();
    assert!(
        wear_filename_from_url(&cape.wear).as_deref() == Some("荷兰的回忆斗篷展示图.png"),
        "wear={}",
        cape.wear
    );
    let dutch_files = vec![
        "Dear-Van-Gogh-Dutch-Memory-Cape-front.png".into(),
        "Dear-Van-Gogh-Dutch-Memory-Cape-back.png".into(),
        "Dear-Van-Gogh-Dutch-Memory-Outfit-front.png".into(),
        "Dear-Van-Gogh-Dutch-Memory-Emote-icon.png".into(),
    ];
    let used = std::collections::HashSet::new();
    assert_eq!(
        pick_unique_kind_wear(&dutch_files, "斗篷", &used).as_deref(),
        Some("Dear-Van-Gogh-Dutch-Memory-Cape-front.png")
    );
    let mut filled = season.clone();
    if let Some(cape) = filled
        .previews
        .iter_mut()
        .find(|p| p.name.contains("荷兰的回忆") && p.kind == "斗篷")
    {
        cape.wear.clear();
    }
    let mut sets = FandomFileSets::empty();
    sets.spirit.insert("荷兰的回忆".into(), dutch_files.clone());
    let mut fmap = std::collections::HashMap::new();
    fmap.insert(
        "Dear-Van-Gogh-Dutch-Memory-Cape-front.png".into(),
        "https://static.wikia.nocookie.net/cape.png".into(),
    );
    apply_fandom_wear(&mut filled, &sets, &fmap);
    assert_eq!(
        filled
            .previews
            .iter()
            .find(|p| p.name.contains("荷兰的回忆") && p.kind == "斗篷")
            .unwrap()
            .wear,
        "https://static.wikia.nocookie.net/cape.png"
    );
    filled
        .previews
        .iter_mut()
        .find(|p| p.name.contains("荷兰的回忆") && p.kind == "斗篷")
        .unwrap()
        .wear = "https://patchwiki.example/keep.png".into();
    apply_fandom_wear(&mut filled, &sets, &fmap);
    assert_eq!(
        filled
            .previews
            .iter()
            .find(|p| p.name.contains("荷兰的回忆") && p.kind == "斗篷")
            .unwrap()
            .wear,
        "https://patchwiki.example/keep.png"
    );
    let mut towel = SkyCalendarItem {
        id: "event-夏之日".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "夏之日".into(),
        names: vec![],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: "ended".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: vec![preview(
            "兑换",
            "粉色毛巾斗篷".into(),
            "斗篷".into(),
            String::new(),
            "https://patchwiki.biligame.com/keep.png".into(),
        )],
    };
    let mut towel_sets = FandomFileSets::empty();
    towel_sets.event_page = vec!["Sunlight-Pink-Towel-Cape-front.png".into()];
    let mut towel_map = std::collections::HashMap::new();
    towel_map.insert(
        "Sunlight-Pink-Towel-Cape-front.png".into(),
        "https://static.wikia.nocookie.net/pink.png".into(),
    );
    apply_fandom_wear(&mut towel, &towel_sets, &towel_map);
    assert_eq!(
        towel.previews[0].wear,
        "https://patchwiki.biligame.com/keep.png"
    );
}
#[test]
fn sky_season_catalog_parses_trees_and_kinds() {
    let wt = "{{季节顶部\n|季节名=致梵高季\n|宣传语=每颗星都值得被看见\n|有无季节任务=6\n}}\n{{季节简介\n|开启时间=2026-07-17\n|结束时间=2026-10-02\n|先祖数量=4\n}}\n{{季节先祖板块\n|先祖中文名=荷兰的回忆\n|先祖类型=非人形先祖\n|所在地图=禁阁\n|先祖位置=\n|物品=小型家具、斗篷\n|季卡物品=裤子、发型\n|兑换图={{兑换图\n    |先祖=荷兰的回忆\n    |季节=致梵高季\n    |动作;1级|随机魔法;;季蜡;4|邀请\n    |小型家具;;季蜡;18|随机魔法;;季蜡;6|动作;2级;;;;季卡\n    |动作;3级;季蜡;22|白色染料;×3;季蜡;10|发型;;;;季卡\n    |斗篷;;季蜡;26||动作;4级;;;;季卡\n    }}\n}}\n{{季节先祖板块\n|先祖中文名=致梵高季向导\n|先祖类型=季节向导\n|所在地图=云巢\n|先祖位置=向日葵展厅\n|物品=\n|季卡物品=毕业发型\n}}";
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
    let row = parse_season_page("致梵高季", "", wt, "2026-07-17", "2026-10-02", today);
    assert_eq!(row.status, "live");
    assert_eq!(row.realm, "云巢");
    assert_eq!(row.loc, "向日葵展厅");
    assert_eq!(row.slogan, "每颗星都值得被看见");
    assert!(row.icon.contains("UI-") && row.icon.contains("logo.png"));
    assert!(row.items.iter().any(|i| i.spirit == "荷兰的回忆" && i.kind == "斗篷" && i.candles == 26 && i.slot == "兑换"));
    assert!(row.items.iter().any(|i| {
        i.spirit == "荷兰的回忆"
            && i.kind == "斗篷"
            && wiki_filename_from_url(&i.icon).as_deref() == Some("UI-斗篷-荷兰的回忆-无框.png")
    }));
    assert!(row.items.iter().any(|i| i.spirit == "荷兰的回忆" && i.kind == "发型" && i.slot == "季卡"));
    assert!(row.items.iter().any(|i| i.kind == "小型家具" && i.candles == 18));
    // 整树季蜡含染料、随机魔法这些不进清单的格子：4+18+6+22+10+26
    assert_eq!(row.tree_candles.get("荷兰的回忆"), Some(&86));
    assert!(row.items.iter().any(|i| {
        i.kind == "毕业发型"
            && wiki_filename_from_url(&i.icon).as_deref() == Some("UI-发型-致梵高季毕业礼-无框.png")
    }));
    let logo = season_logo_url("致梵高季");
    assert_eq!(
        wiki_filename_from_url(&logo).as_deref(),
        Some("UI-致梵高季logo.png")
    );
    let mut mapped = vec![row.clone()];
    let mut urls = std::collections::HashMap::new();
    urls.insert(
        "UI-致梵高季logo.png".into(),
        "https://patchwiki.example/logo.png".into(),
    );
    urls.insert(
        "UI-斗篷-荷兰的回忆-无框.png".into(),
        "https://patchwiki.example/cape.png".into(),
    );
    urls.insert("荷兰的回忆斗篷展示图.png".into(), "https://patchwiki.example/cape-wear.png".into());
    apply_season_media_urls(&mut mapped, &urls);
    assert!(mapped[0].items.iter().any(|i| i.kind == "斗篷" && i.wear.ends_with("cape-wear.png")));
    assert_eq!(mapped[0].icon, "https://patchwiki.example/logo.png");
    assert!(mapped[0].items.iter().any(|i| i.kind == "斗篷" && i.icon == "https://patchwiki.example/cape.png"));
    assert!(mapped[0].items.iter().any(|i| {
        i.kind == "发型" && i.icon.contains("Special:Redirect/file/")
    }));
    assert!(!row.items.iter().any(|i| i.kind.contains("魔法") || i.kind == "邀请" || i.kind == "占位"));
    let assembly = "{{季节先祖板块\n|先祖中文名=集结季向导\n|先祖类型=季节向导\n|所在地图=雨林\n|先祖位置=大树屋\n|物品=大型家具1、大型家具2\n|季卡物品=面具、发型\n}}";
    let old = parse_season_page("集结季", "", assembly, "2021-04-28", "2021-07-13", today);
    assert_eq!(old.status, "ended");
    assert_eq!(old.realm, "雨林");
    assert_eq!(old.loc, "大树屋");
    assert!(old.items.iter().any(|i| i.kind == "大型家具1" && i.candles == 0 && i.cost_desc.is_empty()));
    assert!(old.items.iter().any(|i| i.slot == "季卡" && i.kind == "面具"));
}
#[test]
fn sky_event_catalog_parses_family_and_exhibits() {
    let gameplay = "<!--\n==限定地图==\n往年入口\n-->\n==活动代币==\n本次活动代币为糖果代币。通过体验魔法剧情可获得糖果代币15枚。\n游玩每日游戏、在[[魔法坩埚|坩埚]]旁收集，每日最多5枚。\n<gallery>\nFile:代币.png|图标\n</gallery>\n==兑换==\n{{兑换图|旧内容}}\n==活动任务==\n'''完成剧情'''<br>可领取奖励。";
    let parsed = parse_event_page("2026年恶作剧之日", "", gameplay, "2026-10-24", "2026-11-13",
        chrono::NaiveDate::from_ymd_opt(2026, 11, 7).unwrap(), &serde_json::json!({}));
    assert_eq!(parsed.quests, "本次活动代币为糖果代币。通过体验魔法剧情可获得糖果代币15枚。\n\n游玩每日游戏、在坩埚旁收集，每日最多5枚。\n\n完成剧情可领取奖励。");
    assert_eq!(event_family_of("2026年缤纷飞行日"), "缤纷飞行日");
    assert_eq!(event_family_of("2025年恶作剧之日"), "恶作剧之日");
    assert_eq!(event_family_of("中国绊爱联动"), "中国绊爱联动");
    assert_eq!(infer_cosmetic_kind("彩环短发"), Some("发型"));
    assert_eq!(infer_cosmetic_kind("彩虹斗篷"), Some("斗篷"));
    assert_eq!(infer_cosmetic_kind("浅彩系腰长袍"), Some("裤子"));
    assert_eq!(infer_cosmetic_kind("枯叶发饰"), Some("发饰"));
    assert_eq!(infer_cosmetic_kind("蝴蝶结遮阳帽"), Some("发饰"));
    assert_eq!(infer_cosmetic_kind("夏日阳伞"), Some("手持"));
    let rows = vec![
        ("2025年缤纷飞行日".into(), String::new(), "2025-07-12".into(), "2025-07-25".into()),
        ("2026年缤纷飞行日".into(), String::new(), "2026-09-04".into(), "2026-09-20".into()),
        ("2026年夏之日".into(), String::new(), "2026-08-02".into(), "2026-08-18".into()),
    ];
    let latest = pick_latest_event_rows(rows);
    assert_eq!(latest.len(), 2);
    assert_eq!(latest[0].0, "2026年缤纷飞行日");
    assert_eq!(latest[1].0, "2026年夏之日");
    let color_wt = "{{节日信息\n|开启时间=2026-09-04 00:00\n|结束时间=2026-09-20 23:59\n|节日名称=2026年缤纷飞行日\n}}\n====彩环短发====\n{{展示图|物品|彩环短发|发型}}\n====浅彩系腰长袍====\n{{展示图|物品|浅彩系腰长袍|裤子}}\n====蝴蝶风筝====\n{{展示图|物品|蝴蝶风筝|小型家具}}\n====彩虹斗篷====\n{{展示图|物品|彩虹斗篷|斗篷}}";
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 14).unwrap();
    let packs = serde_json::json!({
        "query": {
            "results": {
                "缤纷飘带斗篷礼包": {
                    "printouts": {
                        "物品": ["斗篷"],
                        "价格": ["98"],
                        "所属": ["缤纷飞行日"]
                    }
                },
                "彩虹阔腿裤礼包": {
                    "printouts": {
                        "物品": ["裤子"],
                        "价格": ["68"],
                        "所属": ["缤纷飞行日"]
                    }
                }
            }
        }
    });
    let row = parse_event_page("2026年缤纷飞行日", "", color_wt, "2026-09-04", "2026-09-20", today, &packs);
    assert_eq!(row.id, "wiki-event-缤纷飞行日");
    assert_eq!(row.title, "缤纷飞行日");
    assert_eq!(row.status, "live");
    assert_eq!(row.start, "2026-09-04");
    assert_eq!(row.end, "2026-09-20");
    assert!(row.items.iter().any(|i| i.name == "彩环短发" && i.kind == "发型"));
    assert!(row.items.iter().any(|i| i.name == "浅彩系腰长袍" && i.kind == "裤子"));
    assert!(row.items.iter().any(|i| i.kind == "小型家具"));
    assert!(row.items.iter().any(|i| {
        i.name == "彩虹斗篷"
            && wiki_filename_from_url(&i.icon).as_deref() == Some("UI-斗篷-彩虹斗篷-无框.png")
            && wear_filename_from_url(&i.wear).as_deref() == Some("彩虹斗篷展示图.png")
    }));
    assert!(row.items.iter().any(|i| {
        i.slot == "商城"
            && i.name == "缤纷飘带斗篷礼包"
            && i.kind == "斗篷"
            && i.cost_desc.contains("98")
    }));
    assert!(row.items.iter().any(|i| i.slot == "商城" && i.name == "彩虹阔腿裤礼包"));
    let halloween_wt = "{{节日信息\n|开启时间=2025-10-31 00:00\n|结束时间=2025-11-20 23:59\n|节日名称=2025年恶作剧之日\n}}\n===鸦喙面具展示===\n[[file:鸦喙面具展示图.png]]\n===哥特斗篷展示===\n[[file:哥特斗篷展示图.png]]\n===枯叶发饰展示===\n[[file:枯叶发饰展示图.png]]";
    let hall = parse_event_page("2025年恶作剧之日", "", halloween_wt, "2025-10-31", "2025-11-20", today, &serde_json::json!({}));
    assert_eq!(hall.title, "恶作剧之日");
    assert_eq!(hall.status, "ended");
    let open_end = parse_event_page(
        "2024年花憩节",
        "",
        "",
        "2024-04-01",
        "",
        today,
        &serde_json::json!({}),
    );
    assert_eq!(open_end.status, "ended");
    assert_eq!(
        event_window_status(
            chrono::NaiveDate::from_ymd_opt(2026, 9, 4),
            None,
            today
        ),
        "ended"
    );
    assert_eq!(
        event_window_status(
            chrono::NaiveDate::from_ymd_opt(2026, 9, 4),
            chrono::NaiveDate::from_ymd_opt(2026, 9, 20),
            today
        ),
        "live"
    );
    assert!(hall.items.iter().any(|i| i.name == "鸦喙面具" && i.kind == "面具"));
    assert!(hall.items.iter().any(|i| i.name == "哥特斗篷" && i.kind == "斗篷"));
    assert!(hall.items.iter().any(|i| i.name == "枯叶发饰" && i.kind == "发饰"));
}
#[test]
fn sky_calendar_announced_start_does_not_inherit_old_dates() {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 8).unwrap();
    let wt = "{{SkyEvent\n|date= November 13, 2026 to TBA\n}}\nPreviously April 27 to May 17, 2024.";
    let (start, end) = parse_fandom_calendar_window(wt).unwrap();
    let event = fandom_calendar_item("New Collaboration", "新联动", start, end, today, None, String::new(), "intl");
    assert_eq!(event.status, "upcoming");
    assert_eq!(event.start, "2026-11-13");
    assert!(event.end.is_empty());
    assert!(parse_fandom_calendar_window("Special Events\nDecember 12, 2025 to January 1, 2026").is_none());
    assert!(parse_fandom_calendar_window("{{SkyEvent|date=TBA}}").is_none());
    let (start, end) = parse_fandom_calendar_window("{{SkyEvent|date=December 12, 2025 to January 1, 2026}}").unwrap();
    assert_eq!(event_window_status(Some(start), end, today), "ended");
    assert_eq!(fandom_event_title_zh("Days of Feast", "", &[]), "宴会节");
    assert_eq!(fandom_en_for_cn_event("宴会节"), Some("Days of Feast"));
    let annual = "The event began on '''Friday, February 13, 2026 at 00:00 PST (UTC-8)''' and ended on '''Thursday, February 26, 2026 at 23:59 PST (UTC-8)'''.";
    let (start, end) = parse_fandom_event_page_window("Days of Fortune/2026", annual).unwrap();
    assert_eq!(ymd(start), "2026-02-13");
    assert_eq!(end.map(ymd).as_deref(), Some("2026-02-26"));
    assert!(parse_fandom_event_page_window("Days of Fortune", annual).is_none());
    assert!(parse_fandom_event_page_window("Special Events", annual).is_none());
    assert_eq!(parse_fandom_event_page_window("New Collaboration", wt), parse_fandom_calendar_window(wt));
}

#[test]
fn sky_calendar_merges_numbered_collaboration_across_sources() {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 10).unwrap();
    let mut fandom = netease_calendar_item(
        "NetEase/Netease Cloud Music", "网易云音乐联动",
        chrono::NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 11, 27).unwrap(),
        today, Some("Netease_Cloud_Music_Event_2026_Promo_Image.jpg"), String::new(),
    );
    let mut wiki = fandom.clone();
    wiki.id = "event-网易云音乐联动 第三期".into();
    wiki.title = "网易云音乐联动 第三期".into();
    wiki.icon.clear();
    wiki.url = "https://wiki.biligame.com/sky/网易云音乐联动_第三期".into();
    assert!(calendar_items_same_event(&wiki, &fandom));
    assert_eq!(calendar_event_family("网易云音乐联动 第3期"), calendar_event_family(&wiki.title));
    let mut events = vec![wiki];
    merge_calendar_events(&mut events, vec![fandom.clone()]);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].title, "网易云音乐联动 第三期");
    assert_eq!((&*events[0].start, &*events[0].end), ("2026-11-14", "2026-11-27"));
    assert!(!events[0].icon.is_empty());
    fandom.id = "intl-music".into();
    fandom.region = "intl".into();
    assert!(!calendar_items_same_event(&events[0], &fandom));
    assert_ne!(calendar_event_family("网易云音乐联动 第三期"), calendar_event_family("九色鹿联动 第三期"));
    merge_calendar_events(&mut events, vec![fandom]);
    assert_eq!(events.len(), 2);
}

#[test]
fn sky_event_fandom_matches_by_langlink_or_unique_dates() {
    let weibo_today = chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap();
    assert_eq!(fandom_event_root("Days of Color/2020"), "Days of Color");
    assert_eq!(
        parse_fandom_event_span("July 31 to August 20, 2026"),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 7, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()
        ))
    );
    assert_eq!(
        parse_fandom_event_date_field(
            "{{SkyEvent | | image = x.jpg | | date = July 31 to August 20, 2026 | }}"
        ),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 7, 31).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 8, 20).unwrap()
        ))
    );
    let mc_wt = "[[File:Sky x Minecraft Promo Image.jpg|center]]\nGet ready as a new Collaboration event crafts its way to Light-Yu, from '''October 2, 2026''' to '''October 18, 2026''', lasting '''16''' days, the Light-Yu x Minecraft Collab will begin";
    assert_eq!(
        parse_fandom_latest_span(mc_wt),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap()
        ))
    );
    assert_eq!(
        parse_fandom_promo_file(mc_wt).as_deref(),
        Some("Sky x Minecraft Promo Image.jpg")
    );
    let moonlight_wt = "{{SkyEvent\n| image = Days-of-Moonlight-2026-promo-image.png\n| date = September 19 to October 9, 2026\n}}\n\nGather beneath the moon at Crescent Oasis to celebrate the Nine-Colored Deer!\nDays of Moonlight returns from '''Saturday, September 19, 2026 at 00:00 PDT (UTC-7)''' through '''Friday, October 9, 2026 at 23:59 PDT (UTC-7)''', lasting '''21''' days.\n<gallery>\nDays of Moonlight 2026 Dates.png|Event Dates\nDays of Moonlight 2026 New Cosmetics.png|New Cosmetics\n</gallery>";
    assert_eq!(
        parse_fandom_latest_span(moonlight_wt),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 9).unwrap()
        ))
    );
    assert_eq!(
        parse_fandom_promo_file(moonlight_wt).as_deref(),
        Some("Days-of-Moonlight-2026-promo-image.png")
    );
    assert_eq!(
        parse_fandom_gallery_entries(moonlight_wt),
        vec![
            ("Days of Moonlight 2026 Dates.png".to_string(), "活动日程".to_string()),
            ("Days of Moonlight 2026 New Cosmetics.png".to_string(), "新增物品".to_string())
        ]
    );
    assert_eq!(calendar_event_family("月华节"), calendar_event_family("秋宵节"));
    assert_eq!(canonical_event_title("月華節"), "秋宵节");
    assert_eq!(fandom_en_for_cn_event("月华节"), Some("Days of Moonlight"));
    assert_eq!(fandom_cn_item_name("Moonlight Garland Cape").as_deref(), Some("纤云弄巧"));
    let mut moonlight_media = fandom_calendar_item(
        "Days of Moonlight/2026",
        "月华节",
        chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 10, 9),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
        None,
        String::new(),
        "intl",
    );
    attach_fandom_event_media(&mut moonlight_media, moonlight_wt);
    assert!(moonlight_media.previews.iter().any(|p| p.slot == "资讯" && p.name == "活动日程"));
    assert!(moonlight_media.previews.iter().any(|p| p.slot == "资讯" && p.name == "新增物品"));
    assert!(!moonlight_media.previews.iter().any(|p| p.name.starts_with("资讯 ")));
    assert!(!keep_cn_notice_for_year(
        &serde_json::json!({
            "title": "玉兔踏清辉 月满映秋亭 | 《光·遇》秋宵节：对月寄相思",
            "url": "//sky.163.com/news/official/20250929/26689_1262415.html"
        }),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap()
    ));
    assert!(note_is_noise(&fandom_lead_note(moonlight_wt)));
    assert_eq!(
        cn_notice_short_title("乘风送鸢 拂云赏色丨《光·遇》缤纷飞行日：一起放风筝！"),
        "缤纷飞行日"
    );
    assert_eq!(
        cn_notice_short_title("玉兔踏清辉 月满映秋亭 | 《光·遇》秋宵节：对月寄相思"),
        "秋宵节"
    );
    assert_eq!(
        parse_cn_notice_span("《光·遇》秋宵节将于10月1日开启。2025年10月1日—2025年10月14日"),
        Some((
            chrono::NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2025, 10, 14).unwrap()
        ))
    );
    let notice = cn_notice_to_item(
        &serde_json::json!({
            "code": "1313290",
            "title": "乘风送鸢 拂云赏色丨《光·遇》缤纷飞行日：一起放风筝！",
            "comment": "《光·遇》缤纷飞行日节已于9月4日正式上线，乘风送鸢，拂云赏色，一起来放风筝吧~",
            "imageUrl": "https://nie.res.netease.com/r/pic/20260907/cover.png",
            "imageUrls": "https://nie.res.netease.com/r/pic/20260907/a.png;https://nie.res.netease.com/r/pic/20260907/b.png;",
            "url": "//sky.163.com/news/official/20260907/26689_1313290.html",
            "showtime": 1788451200000i64
        }),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .unwrap();
    assert_eq!(notice.title, "缤纷飞行日");
    assert!(notice.note.contains("放风筝"));
    assert!(notice.icon.starts_with("https://nie.res.netease.com/"));
    assert_eq!(notice.previews.iter().filter(|p| p.slot == "资讯").count(), 2);
    let kite = cn_notice_to_item(
        &serde_json::json!({
            "code": "x",
            "title": "《光·遇》缤纷飞行日：一起放风筝！",
            "comment": "亲爱的旅人们：活动现已开启。",
            "imageUrl": "https://nie.res.netease.com/r/pic/20260907/cover.png",
            "showtime": 1788451200000i64
        }),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .unwrap();
    assert_eq!(kite.title, "缤纷飞行日");
    assert!(kite.start.is_empty());
    assert_ne!(kite.status, "live");
    assert!(kite.icon.contains("nie.res.netease.com"));
    assert!(cn_notice_to_item(
        &serde_json::json!({
            "code": "1313038",
            "title": "9月7日丨《光·遇》打击非法第三方辅助软件公告",
            "comment": "为守护公平环境，持续查处非法第三方辅助软件。",
            "imageUrl": "https://nie.res.netease.com/r/pic/20260907/ban.png",
            "url": "//sky.163.com/news/update/20260903/26690_1313038.html",
            "showtime": 1788778800000i64
        }),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
    )
    .is_none());
    assert!(!calendar_icon_usable(
        "https://sky-children-of-the-light.fandom.com/wiki/Special:FilePath/x.jpg"
    ));
    assert!(calendar_icon_usable(
        "https://static.wikia.nocookie.net/sky-children-of-the-light/images/f/f5/Sky_x_Minecraft_Promo_Image.jpg"
    ));
    assert!(calendar_icon_usable(
        "https://nie.res.netease.com/r/pic/20260907/cover.png"
    ));
    assert!(calendar_icon_usable(
        "https://mmbiz.qpic.cn/sz_mmbiz_png/HASH1/0?wx_fmt=png"
    ));
    assert!(same_media_file(
        "https://mmbiz.qpic.cn/sz_mmbiz_png/HASH1/0?wx_fmt=png",
        "https://mmbiz.qpic.cn/sz_mmbiz_png/HASH1/640?wx_fmt=png"
    ));
    assert!(!same_media_file(
        "https://mmbiz.qpic.cn/sz_mmbiz_png/HASH1/0?wx_fmt=png",
        "https://mmbiz.qpic.cn/sz_mmbiz_png/HASH2/0?wx_fmt=png"
    ));
    let wx_html = r#"<meta property="og:title" content="《光·遇》《我的世界》联动活动更新内容公告">
<div id="js_content" class="rich_media_content">
<p>美西螈悠悠漫步云端河畔。金苹果熠熠点亮友人前路。10月2日～10月18日开启。</p>
<img data-src="https://mmbiz.qpic.cn/sz_mmbiz_gif/GIFhash/0?wx_fmt=gif" data-ratio="0.33">
<img data-src="https://mmbiz.qpic.cn/sz_mmbiz_png/AxolotlHash/0?wx_fmt=png" data-ratio="6.075">
<img data-src="https://mmbiz.qpic.cn/sz_mmbiz_png/AppleHash/640?wx_fmt=png" data-ratio="5.421">
<img data-src="https://mmbiz.qpic.cn/sz_mmbiz_png/AppleHash/0?wx_fmt=png" data-ratio="5.421">
<img data-src="https://mmbiz.qpic.cn/mmbiz_png/SpacerHash/0?wx_fmt=png" data-ratio="0.0203125">
往期精彩回顾
<img data-src="https://mmbiz.qpic.cn/sz_mmbiz_png/FooterHash/0?wx_fmt=png" data-ratio="1">
</div>"#;
    let wx = parse_weixin_article(wx_html, "https://mp.weixin.qq.com/s/IbZgqEn8694IPEseq4Hj8Q").unwrap();
    assert_eq!(wx.title, "我的世界联动");
    assert!(wx.note.contains("美西螈"));
    assert_eq!(wx.previews.iter().filter(|p| p.slot == "资讯").count(), 2);
    assert!(!wx.previews.iter().any(|p| p.icon.contains("SpacerHash")));
    assert!(wx.previews.iter().any(|p| p.icon.contains("AxolotlHash")));
    assert!(wx.previews.iter().any(|p| p.icon.contains("AppleHash")));
    assert!(!wx.previews.iter().any(|p| p.icon.contains("FooterHash")));
    assert!(!wx.previews.iter().any(|p| p.icon.contains("GIFhash")));
    assert!(!wx.icon.contains("AxolotlHash"));
    let ds_skip = dashen_event_from_feed(&serde_json::json!({
        "id": "old",
        "feedContent": {"type": 2, "body": {"title": "亲身体验一个家族的温情故事", "text": "家族", "media": [{"url": "https://ok.166.net/reunionpub/a.jpeg"}]}}
    }));
    assert!(ds_skip.is_none());
    let ds_mc = dashen_event_from_feed(&serde_json::json!({
        "id": "mcfeed",
        "feedContent": {
            "type": 2,
            "body": {
                "title": "《光·遇》《我的世界》联动",
                "text": "遇见方块新朋友",
                "media": [
                    {"url": "https://ok.166.net/reunionpub/mc-scene.jpeg"},
                    {"url": "https://ok.166.net/reunionpub/mc-body.png"}
                ]
            }
        }
    }))
    .unwrap();
    assert_eq!(ds_mc.title, "我的世界联动");
    assert!(ds_mc.previews.iter().any(|p| p.icon.contains("mc-scene.jpeg")));
    assert!(ds_mc.previews.iter().any(|p| p.icon.contains("mc-body.png")));
    assert_eq!(
        fandom_event_title_zh("Days of Moonlight/2026", moonlight_wt, &["月華節".into()]),
        "秋宵节"
    );
    let qixi_wt = "== August 19, 2026 ==\nThe event began on '''August 19, 2026 at 00:00 (GMT+8)'''  and ended on '''September 1, 2026 at 23:59 (GMT+8)''', lasting '''13''' days.";
    assert_eq!(
        parse_fandom_latest_span(qixi_wt),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 8, 19).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        ))
    );
    let kfc_wt = "A collaboration event between Light•Yu and Kentucky Fried Chicken has been confirmed to run from Jan 19 2024, 0:00 to Feb 1 2024, 23:59 (GMT+8 time zone) lasting 14 days.";
    assert_eq!(
        parse_fandom_latest_span(kfc_wt),
        Some((
            chrono::NaiveDate::from_ymd_opt(2024, 1, 19).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2024, 2, 1).unwrap()
        ))
    );
    let special = r#"=== List of Special Events ===
* [[NetEase/Qixi Festival|Qixi Festival]] 七夕节
=== List of Special Collaboration ===
* Kizuna AI China 中国绊爱
* [[NetEase/Minecraft|Minecraft]] 我的世界
"#;
    let listed = parse_netease_special_event_links(special);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0], ("NetEase/Qixi Festival".into(), "七夕节".into(), false));
    assert_eq!(listed[1], ("NetEase/Minecraft".into(), "我的世界".into(), true));
    assert_eq!(
        netease_event_title("NetEase/Minecraft", "我的世界", mc_wt, true),
        "我的世界联动"
    );
    assert_eq!(canonical_event_title("Minecraft"), "我的世界联动");
    let mut mc_media = netease_calendar_item(
        "NetEase/Minecraft",
        "我的世界联动",
        chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 21).unwrap(),
        Some("Sky x Minecraft Promo Image.jpg"),
        String::new(),
    );
    attach_fandom_event_media(&mut mc_media, mc_wt);
    assert!(mc_media.icon.contains("Sky x Minecraft Promo Image.jpg"));
    assert!(!mc_media.previews.iter().any(|p| p.slot == "资讯"));
    assert_eq!(
        parse_weibo_status_ids(
            "Announcement on [https://weibo.com/6355968578/5343388682424870 Weibo]\n3rd announcement on [https://weibo.com/6355968578/5344113458418056 Weibo]"
        ),
        vec![
            "5343388682424870".to_string(),
            "5344113458418056".to_string()
        ]
    );
    assert!(same_media_file(
        "https://wx3.sinaimg.cn/mw2000/006W8ZMely1ih6i61nlh3j30rs4orhdw.jpg",
        "https://wx1.sinaimg.cn/mw2000/006W8ZMely1ih6i61nlh3j30rs4orhdw.jpg"
    ));
    assert_eq!(
        weibo_mblog_images(&serde_json::json!({
            "pics": [{"large": {"url": "https://wx3.sinaimg.cn/large/006W8ZMely1ih6i61nlh3j30rs4orhdw.jpg"}}],
            "pic_ids": ["006W8ZMely1ih6i61nlh3j30rs4orhdw"]
        })),
        vec!["https://wx3.sinaimg.cn/mw2000/006W8ZMely1ih6i61nlh3j30rs4orhdw.jpg".to_string()]
    );
    assert_eq!(
        parse_weibo_created_at("Sat Nov 21 08:30:00 +0800 2026"),
        chrono::NaiveDate::from_ymd_opt(2026, 11, 21)
    );
    assert_eq!(
        parse_weibo_created_at("Fri Nov 20 23:59:00 +0800 2026"),
        chrono::NaiveDate::from_ymd_opt(2026, 11, 20)
    );
    assert_eq!(
        weibo_numbered_images(&serde_json::json!({
            "pic_ids": ["image0002", null, "image0001"],
            "pic_infos": {
                "image0001": {"large": {"url": "https://wx1.sinaimg.cn/large/1.jpg"}},
                "image0002": {"large": {"url": "https://wx1.sinaimg.cn/large/2.jpg"}}
            }
        })),
        ["https://wx1.sinaimg.cn/mw2000/2.jpg", "", "https://wx1.sinaimg.cn/mw2000/1.jpg"]
    );
    let promo_post = weibo_event_from_mblog(&serde_json::json!({
        "text": "#光遇我的世界联动# 宣传图",
        "pics": [{"large": {"url": "https://wx3.sinaimg.cn/large/Sky-x-Minecraft-Promo.jpg"}}]
    }), weibo_today)
    .unwrap();
    assert!(promo_post.icon.is_empty());
    assert!(promo_post.previews.iter().any(|p| p.slot == "资讯" && p.icon.contains("Sky-x-Minecraft-Promo.jpg")));
    assert_eq!(
        weibo_card_event_title("#光遇我的世界联动# 联动活动将于10月2日开启"),
        Some("我的世界联动".into())
    );
    assert_eq!(
        weibo_card_event_title("#光遇秋宵节# 秋宵节：再见九色鹿"),
        Some("秋宵节".into())
    );
    assert_eq!(
        weibo_card_event_title("#光遇我的世界联动# #光遇秋宵节# 新朋旧识"),
        None
    );
    let weibo_mc = weibo_event_from_mblog(&serde_json::json!({
        "text": "#光遇# #光遇我的世界联动# 美西螈悠悠漫步云端河畔 《光·遇》《我的世界》联动活动将于10月2日开启",
        "pics": [
            {"large": {"url": "https://wx3.sinaimg.cn/large/mc-axolotl.jpg"}},
            {"large": {"url": "https://wx4.sinaimg.cn/large/mc-apple.jpg"}}
        ]
    }), weibo_today)
    .unwrap();
    overlay_event_media(&mut mc_media, &wx);
    overlay_event_media(&mut mc_media, &ds_mc);
    overlay_event_media(&mut mc_media, &weibo_mc);
    overlay_event_media(
        &mut mc_media,
        &weibo_event_from_mblog(&serde_json::json!({
            "text": "#光遇# #光遇我的世界联动# 美西螈",
            "pics": [
                {"large": {"url": "https://wx1.sinaimg.cn/large/mc-axolotl.jpg"}},
                {"large": {"url": "https://wx4.sinaimg.cn/large/mc-apple.jpg"}}
            ]
        }), weibo_today)
        .unwrap(),
    );
    dedupe_news_previews(&mut mc_media);
    assert_eq!(
        mc_media
            .previews
            .iter()
            .filter(|p| p.slot == "资讯")
            .count(),
        6
    );
    let dump = inspect_feed_json(
        &SkyCalendarFeed {
            as_of: "2026-09-21".into(),
            season: None,
            events: vec![mc_media.clone()],
            cn_now: vec![],
            cn_next: vec![],
            intl_now: vec![],
            intl_next: vec![],
            event_history: vec![],
            warnings: vec![],
        },
        Some("我的世界联动"),
    );
    assert_eq!(dump["count"], 1);
    assert_eq!(dump["items"][0]["slots"]["资讯"], 6);
    assert!(dump["items"][0]["previews"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["icon"].as_str().unwrap_or("").contains("AxolotlHash")));
    assert!(mc_media.previews.iter().any(|p| p.icon.contains("AxolotlHash")));
    assert!(mc_media.previews.iter().any(|p| p.icon.contains("AppleHash")));
    assert!(mc_media.previews.iter().any(|p| p.icon.contains("mc-axolotl.jpg")));
    assert!(mc_media.previews.iter().any(|p| p.icon.contains("mc-apple.jpg")));
    assert!(!mc_media.previews.iter().any(|p| p.icon.contains("Promo Image")));
    let mut after_fill = mc_media.clone();
    fill_event_media(&mut after_fill, Vec::new(), &serde_json::json!({}));
    overlay_event_media(&mut after_fill, &weibo_mc);
    overlay_event_media(&mut after_fill, &wx);
    overlay_event_media(&mut after_fill, &ds_mc);
    assert_eq!(
        after_fill
            .previews
            .iter()
            .filter(|p| p.slot == "资讯")
            .count(),
        6
    );
    assert!(!after_fill.previews.iter().any(|p| p.icon.to_ascii_lowercase().contains("promo")));
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap();
    let mut events = vec![SkyCalendarItem {
        id: "event-2026年缤纷飞行日".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "缤纷飞行日".into(),
        names: vec!["缤纷飞行日".into()],
        season: String::new(),
        start: "2026-09-04".into(),
        end: "2026-09-20".into(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    }];
    merge_calendar_events(
        &mut events,
        vec![netease_calendar_item(
            "NetEase/Minecraft",
            "我的世界联动",
            chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            today,
            Some("Sky x Minecraft Promo Image.jpg"),
            "国服专页".into(),
        )],
    );
    assert!(events.iter().any(|e| e.id == "cn-netease-minecraft"
        && e.title == "我的世界联动"
        && e.start == "2026-10-02"
        && e.end == "2026-10-18"
        && e.status == "upcoming"));
    merge_calendar_events(
        &mut events,
        vec![netease_calendar_item(
            "NetEase/Minecraft",
            "我的世界联动",
            chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 18).unwrap(),
            today,
            None,
            String::new(),
        )],
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.title.contains("我的世界"))
            .count(),
        1
    );
    merge_calendar_events(
        &mut events,
        vec![fandom_calendar_item(
            "Days of Moonlight/2026",
            "秋宵节",
            chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 9),
            today,
            Some("Days-of-Moonlight-2026-promo-image.png"),
            "Crescent Oasis".into(),
            "intl",
        )],
    );
    let qx = events.iter().find(|e| e.title == "秋宵节").unwrap();
    assert_eq!(qx.region, "intl");
    assert_eq!(qx.start, "2026-09-19");
    assert_eq!(qx.end, "2026-10-09");
    assert!(qx.icon.contains("Days-of-Moonlight-2026-promo-image.png"));
    assert!(events.iter().all(|e| e.region == "intl" || e.title != "秋宵节"));
    let mut cn_qx = SkyCalendarItem {
        id: "event-2025年秋宵节".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "秋宵节".into(),
        names: vec!["2025年秋宵节".into()],
        season: String::new(),
        start: "2025-10-01".into(),
        end: "2025-10-14".into(),
        status: "ended".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    adopt_calendar_window(&mut cn_qx, qx);
    assert_eq!(cn_qx.start, "2025-10-01");
    assert_eq!(cn_qx.status, "ended");
    merge_calendar_events(
        &mut events,
        vec![SkyCalendarItem {
            id: "cn-notice-qx".into(),
            kind: "event".into(),
            region: "cn".into(),
            title: "秋宵节".into(),
            names: vec![],
            season: String::new(),
            start: "2026-10-01".into(),
            end: "2026-10-14".into(),
            status: "upcoming".into(),
            note: "国服将于10月1日开启".into(),
            url: "https://sky.163.com/news/official/qx".into(),
            cosmetics: String::new(),
            icon: String::new(),
            previews: Vec::new(),
        }],
    );
    assert_eq!(events.iter().filter(|e| e.title == "秋宵节").count(), 2);
    assert!(events.iter().any(|e| e.region == "cn"
        && e.title == "秋宵节"
        && e.start == "2026-10-01"
        && e.status == "upcoming"));
    annotate_intl_against_cn(&mut events);
    assert!(events.iter().any(|e| e.region == "intl"
        && e.title == "秋宵节"
        && e.note.contains("国服 2026-10-01")));
    let mut waiting = events.clone();
    waiting.retain(|e| e.region != "cn" || e.title != "秋宵节");
    merge_calendar_events(&mut waiting, vec![history_marker("秋宵节")]);
    annotate_intl_against_cn(&mut waiting);
    assert!(waiting.iter().any(|e| e.region == "intl"
        && e.title == "秋宵节"
        && e.note.contains("今年开启日等公告")));
    // 同场已在档时，微博再来一条同名活动只能并入，不能借历史占位再升级出一条。
    let live_cn = events
        .iter()
        .find(|e| e.region == "cn" && e.title == "秋宵节")
        .cloned()
        .unwrap();
    let mut both = vec![live_cn.clone(), history_marker("秋宵节")];
    let mut weibo = live_cn;
    weibo.id = "cn-weibo-秋宵节".into();
    merge_calendar_events(&mut both, vec![weibo]);
    assert_eq!(both.iter().filter(|e| e.status != "history").count(), 1);
    assert_eq!(
        parse_cn_notice_window("秋宵节将于10月1日开启", chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap()),
        Some((chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), None))
    );
    assert_eq!(
        published_date_from_url("//sky.163.com/news/official/20260120/26689_1282679.html"),
        Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap())
    );
    assert_eq!(
        parse_cn_notice_window(
            "性格测试活动将于1月24日开启",
            chrono::NaiveDate::from_ymd_opt(2026, 1, 20).unwrap()
        ),
        Some((chrono::NaiveDate::from_ymd_opt(2026, 1, 24).unwrap(), None))
    );
    assert_eq!(
        parse_cn_notice_window(
            "性格测试活动将于1月24日开启",
            chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap()
        ),
        None
    );
    assert_eq!(
        parse_cn_notice_window(
            "2月13日-2月15日，光之子们可参与花灯搭建",
            chrono::NaiveDate::from_ymd_opt(2026, 2, 10).unwrap()
        ),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 2, 13).unwrap(),
            Some(chrono::NaiveDate::from_ymd_opt(2026, 2, 15).unwrap())
        ))
    );
    let old_notice = cn_notice_to_item(
        &serde_json::json!({
            "code": "1282679",
            "title": "自我探寻丨《光·遇》性格测试活动",
            "comment": "性格测试活动将于1月24日开启",
            "url": "//sky.163.com/news/official/20260120/26689_1282679.html",
            "imageUrl": "https://nie.res.netease.com/r/pic/20260120/cover.png"
        }),
        chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap(),
    )
    .unwrap();
    assert_eq!(old_notice.start, "2026-01-24");
    assert_eq!(old_notice.status, "ended");
    let weibo_qx = weibo_event_from_mblog(&serde_json::json!({
        "text": "#光遇秋宵节# 将于10月1日开启",
        "pics": [{"large": {"url": "https://wx4.sinaimg.cn/large/qx.jpg"}}]
    }), weibo_today)
    .unwrap();
    assert_eq!(weibo_qx.title, "秋宵节");
    assert_eq!(weibo_qx.start, "2026-10-01");
    assert_eq!(weibo_qx.status, "upcoming");
    let today_qx = chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap();
    assert_eq!(
        parse_cn_notice_span("追寻壁画故事。9月24日-10月14日前往活动页面。2026年9月23日"),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 14).unwrap()
        ))
    );
    assert_eq!(
        parse_cn_notice_window(
            "追寻壁画故事。9月24日-10月14日前往活动页面。2026年9月23日",
            today_qx
        ),
        Some((
            chrono::NaiveDate::from_ymd_opt(2026, 9, 24).unwrap(),
            Some(chrono::NaiveDate::from_ymd_opt(2026, 10, 14).unwrap())
        ))
    );
    let weibo_live = weibo_event_from_mblog(&serde_json::json!({
        "text": "#光遇# #光遇九色鹿联动回归# #月光遇见九色鹿# #光遇秋宵节# 追寻壁画故事 月下重逢神鹿 9月24日-10月14日 前往【秋宵·鹿影踏月】活动页面",
        "pics": [{"large": {"url": "https://wx4.sinaimg.cn/large/qx-live.jpg"}}]
    }), weibo_today)
    .unwrap();
    assert_eq!(weibo_live.title, "秋宵节");
    assert_eq!(weibo_live.start, "2026-09-24");
    assert_eq!(weibo_live.end, "2026-10-14");
    assert_eq!(weibo_live.status, "upcoming");
    assert_eq!(
        collect_weixin_article_urls(&[
            "详见 https://mp.weixin.qq.com/s/ljQyoJO5Qjzjof9SksStcQ 以及页脚 https://mp.weixin.qq.com/s?__biz=MzU3MTE3MzY2NA==&amp;mid=2247488251&amp;idx=1&amp;sn=75d89c60dc2a315ee90aab5e0b387069&amp;scene=21#wechat_redirect".into(),
            "别人的号 https://mp.weixin.qq.com/s?__biz=OTHER&mid=1&idx=1&sn=abc".into(),
        ]),
        vec![
            "https://mp.weixin.qq.com/s/ljQyoJO5Qjzjof9SksStcQ".to_string(),
            "https://mp.weixin.qq.com/s?__biz=MzU3MTE3MzY2NA==&mid=2247488251&idx=1&sn=75d89c60dc2a315ee90aab5e0b387069".to_string(),
        ]
    );
    merge_calendar_events(
        &mut events,
        vec![fandom_calendar_item(
            "Days of Moonlight/2026",
            "月华节",
            chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 9),
            today,
            None,
            String::new(),
            "intl",
        )],
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e.region == "intl" && calendar_event_family(&e.title) == "秋宵")
            .count(),
        1
    );
    assert_eq!(events.iter().filter(|e| e.title == "秋宵节").count(), 2);
    assert!(!events.iter().any(|e| e.title.contains("月华")));
    let moonlight_ex = parse_event_exhibits(
        "===金桂发饰展示===\n[[file:金桂发饰展示图.png]]\n===玉兔尾巴展示===\n[[file:玉兔尾巴颈饰展示图.png]]",
    );
    assert!(moonlight_ex.iter().any(|(n, k)| n.contains("金桂") && k == "发饰"));
    assert!(moonlight_ex.iter().any(|(n, k)| n.contains("玉兔尾巴") && k == "颈饰"));
    let mut moonlight = SkyCalendarItem {
        id: "event-秋宵节".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "秋宵节".into(),
        names: vec![],
        season: String::new(),
        start: "2026-09-19".into(),
        end: "2026-10-09".into(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    let mut moonlight_sets = FandomFileSets::empty();
    moonlight_sets.days_items.push(FandomDaysItem { days: "Moonlight".into(), year: "2026".into(), name: "Moonlight Garland Cape".into(), files: vec!["Moonlight-Garland-Cape-front.png".into()], ..Default::default() });
    moonlight_sets.days_items.push(FandomDaysItem { days: "Moonlight".into(), year: "2025".into(), name: "Old Lantern".into(), files: vec!["Old-Lantern-real.png".into()], ..Default::default() });
    moonlight.icon = "File:Days-of-Moonlight-2026-promo-image.png".into();
    let qx_packs = serde_json::json!({
        "query": {
            "results": {
                "纤云弄巧礼包": {
                    "printouts": { "物品": ["斗篷"], "价格": ["98"] }
                },
                "月华霓裳礼包": {
                    "printouts": { "物品": ["发型", "裤子"], "价格": ["98"] }
                },
                "满月耳坠礼包": {
                    "printouts": { "物品": ["耳饰"], "价格": ["18"] }
                }
            }
        }
    });
    fill_event_media(&mut moonlight, Vec::new(), &qx_packs);
    fill_event_from_fandom(&mut moonlight, &moonlight_sets);
    let wear = moonlight
        .previews
        .iter()
        .find(|p| p.slot == "兑换" && p.name.contains("纤云弄巧"))
        .map(|p| p.wear.clone())
        .unwrap_or_default();
    assert!(wear.contains("Special:FilePath"), "wear={wear}");
    assert!(!moonlight.previews.iter().any(|p| p.name.contains("Old Lantern")));
    assert!(moonlight.previews.iter().any(|p| p.slot == "商城" && p.name.contains("纤云弄巧礼包")));
    assert!(moonlight.previews.iter().any(|p| p.slot == "商城" && p.name.contains("月华霓裳")));
    moonlight.previews.push(preview(
        "资讯",
        "活动海报".into(),
        String::new(),
        "https://wx4.sinaimg.cn/mw2000/qx-poster.jpg".into(),
        String::new(),
    ));
    dedupe_news_previews(&mut moonlight);
    assert!(
        !moonlight.previews.iter().any(|p| p.slot == "资讯"),
        "已有兑换列表时不再留活动图"
    );
    let mut season = SkyCalendarItem {
        id: "season-致梵高季".into(),
        kind: "season".into(),
        region: "cn".into(),
        title: "致梵高季".into(),
        names: vec![],
        season: String::new(),
        start: "2026-07-17".into(),
        end: "2026-10-02".into(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    fill_season_media(
        &mut season,
        parse_exchange_trees("{{兑换图|先祖=荷兰的回忆|斗篷;;季蜡;26}}"),
        &serde_json::json!({}),
    );
    let season_wear = season
        .previews
        .iter()
        .find(|p| p.slot == "兑换")
        .map(|p| p.wear.clone())
        .unwrap_or_default();
    assert!(
        wear_filename_from_url(&season_wear).as_deref() == Some("荷兰的回忆斗篷展示图.png"),
        "季节兑换必须带实穿，wear={season_wear}"
    );
    let mut reprint = SkyCalendarItem {
        id: "cn-reprint".into(),
        kind: "reprint".into(),
        region: "cn".into(),
        title: "国服集体回归".into(),
        names: vec!["采药人".into()],
        season: String::new(),
        start: "2026-09-24".into(),
        end: "2026-10-07".into(),
        status: "upcoming".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: "斗篷".into(),
        icon: String::new(),
        previews: Vec::new(),
    };
    fill_reprint_looks(&mut reprint, &[("采药人".into(), "面具、斗篷".into())]);
    reprint.previews.push(preview(
        "资讯",
        "活动海报".into(),
        String::new(),
        "https://wx4.sinaimg.cn/mw2000/reprint.jpg".into(),
        String::new(),
    ));
    dedupe_news_previews(&mut reprint);
    assert!(reprint.previews.iter().any(|p| p.slot == "先祖"));
    assert!(reprint.previews.iter().any(|p| {
        p.slot == "兑换" && wear_filename_from_url(&p.wear).as_deref() == Some("采药人斗篷展示图.png")
    }));
    assert!(!reprint.previews.iter().any(|p| p.slot == "资讯"));
    let mut bare = SkyCalendarItem {
        id: "cn-weibo-秋宵节".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "秋宵节".into(),
        names: vec![],
        season: String::new(),
        start: "2026-09-24".into(),
        end: "2026-10-14".into(),
        status: "upcoming".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: vec![preview(
            "资讯",
            "活动资讯".into(),
            String::new(),
            "https://wx4.sinaimg.cn/mw2000/qx.jpg".into(),
            String::new(),
        )],
    };
    let mut catalog_event = SkySeasonCatalogItem {
        id: "wiki-event-秋宵节".into(),
        title: "秋宵节".into(),
        slogan: "2025年秋宵节".into(),
        realm: String::new(),
        loc: String::new(),
        start: "2025-10-01".into(),
        end: "2025-10-14".into(),
        status: "ended".into(),
        url: String::new(),
        icon: "https://patchwiki.biligame.com/images/sky/ui-qx.png".into(),
        quests: String::new(),
        sheet: String::new(),
        tree_candles: Default::default(),
        items: vec![SkySeasonWishItem {
            id: "qx|纤云弄巧|斗篷".into(),
            name: "纤云弄巧".into(),
            spirit: "秋宵节".into(),
            kind: "斗篷".into(),
            slot: "兑换".into(),
            cost_desc: String::new(),
            candles: 0,
            hearts: 0,
            ascended: 0,
            icon: "https://patchwiki.biligame.com/images/sky/ui-cape.png".into(),
            wear: "https://patchwiki.biligame.com/images/sky/wear-cape.png".into(),
        }],
    };
    let extra_items: Vec<_> = (1..15).map(|i| {
        let mut item = catalog_event.items[0].clone();
        item.id = format!("qx|外观{i}");
        item.name = format!("外观{i}");
        if i == 1 { item.kind = "小型家具".into(); }
        if i == 2 { item.icon.clear(); item.wear.clear(); }
        item
    }).collect();
    catalog_event.items.extend(extra_items);
    assert!(needs_catalog_looks(&bare));
    attach_catalog_looks(&mut bare, &[catalog_event]);
    assert!(!needs_catalog_looks(&bare), "已有兑换列表的情报不应等待历史活动目录");
    assert_eq!(bare.previews.iter().filter(|p| p.slot == "兑换").count(), 15);
    let mut fandom_event = bare.clone();
    fandom_event.previews.clear();
    let mut many_items = FandomFileSets::empty();
    many_items.event_items.insert(fandom_event.title.clone(),
        (0..15).map(|i| (format!("外观{i}"), if i == 0 { "小型家具" } else { "斗篷" }.to_string())).collect());
    fill_event_from_fandom(&mut fandom_event, &many_items);
    assert_eq!(fandom_event.previews.iter().filter(|p| p.slot == "兑换").count(), 15);
    assert_eq!(bare.start, "2026-09-24");
    assert!(bare.icon.contains("ui-qx.png"));
    assert!(!bare.previews.iter().any(|p| p.slot == "资讯"));
    assert!(bare.previews.iter().any(|p| p.slot == "兑换" && p.wear.contains("wear-cape.png")));
    let worn = parse_event_page(
        "2025年秋宵节",
        "",
        "===纤云弄巧斗篷展示===\n[[file:纤云弄巧斗篷展示图.png]]",
        "2025-10-01",
        "2025-10-14",
        chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap(),
        &serde_json::json!({}),
    );
    assert!(worn.items.iter().any(|i| i.slot == "兑换" && i.wear.contains("Special:FilePath")));
    assert!(worn.sheet.is_empty());
    let sheet = parse_event_page(
        "2025年秋宵节",
        "",
        "[[file:2025秋宵节兑换图.png|class=img-kk]]\n[[file:玉兔发饰展示图.png]]",
        "2025-10-01",
        "2025-10-14",
        chrono::NaiveDate::from_ymd_opt(2026, 9, 23).unwrap(),
        &serde_json::json!({}),
    );
    assert_eq!(sheet.sheet, "2025秋宵节兑换图.png");
    let grouped_packs = group_event_packs(&serde_json::json!({
        "query": {
            "results": {
                "珍珠季礼包": {
                    "printouts": { "所属": ["珍珠季"], "物品": ["斗篷"], "价格": ["98"] }
                },
                "纤云弄巧礼包": {
                    "printouts": { "所属": ["秋宵节"], "物品": ["斗篷"], "价格": ["98"] }
                },
                "月华霓裳礼包": {
                    "printouts": { "所属": ["秋宵节"], "物品": ["发型", "裤子"], "价格": ["98"] }
                }
            }
        }
    }));
    assert_eq!(pack_owner_key("秋宵节"), pack_owner_key("月华节"));
    assert!(packs_for_event_title(&grouped_packs, "珍珠季").pointer("/query/results/珍珠季礼包").is_some());
    let from_family = packs_for_event_title(&grouped_packs, "秋宵节");
    assert!(from_family
        .pointer("/query/results/纤云弄巧礼包")
        .is_some());
    let mut moonlight_shop = moonlight.clone();
    moonlight_shop.previews.clear();
    fill_event_media(
        &mut moonlight_shop,
        Vec::new(),
        &packs_for_event_title(&grouped_packs, "月华节"),
    );
    assert!(moonlight_shop
        .previews
        .iter()
        .any(|p| p.slot == "商城" && p.name.contains("纤云弄巧礼包")));
    assert!(moonlight_shop
        .previews
        .iter()
        .any(|p| p.slot == "商城" && p.name.contains("月华霓裳")));
    let shop_icon = moonlight
        .previews
        .iter()
        .find(|p| p.slot == "商城" && p.name.contains("月华霓裳"))
        .map(|p| p.icon.clone())
        .unwrap_or_default();
    assert!(shop_icon.contains("Special:FilePath"));
    let mut kept = vec![moonlight.clone()];
    resolve_fandom_media_urls(&mut kept, &std::collections::HashMap::new());
    assert!(kept[0]
        .previews
        .iter()
        .any(|p| p.slot == "商城" && p.icon.contains("Special:FilePath")));
    let mut wiki_hit = std::collections::HashMap::new();
    wiki_hit.insert(
        wiki_filename_from_url(&shop_icon).unwrap(),
        "https://patchwiki.biligame.com/images/sky/moonlight-pack.png".into(),
    );
    apply_wiki_preview_icons(&mut kept[0], &wiki_hit);
    assert!(kept[0]
        .previews
        .iter()
        .any(|p| p.slot == "商城"
            && p.name.contains("月华霓裳")
            && p.icon.contains("patchwiki.biligame.com")));
    assert_eq!(moonlight.icon, "File:Days-of-Moonlight-2026-promo-image.png");
    let mut wiki_map = std::collections::HashMap::new();
    wiki_map.insert("秋宵节".into(), "#重定向 [[2025年秋宵节]]".into());
    wiki_map.insert("2025年秋宵节".into(), "{{节日信息|开启时间=2025-10-01}}".into());
    assert!(calendar_event_wikitext(&moonlight, &wiki_map).is_none());
    assert!(events.iter().any(|e| e.title == "缤纷飞行日"));
    let days_wt = r#"
pink_towel_cape = {
name= "Sunlight Pink Beach Towel Cape",
front= "Sunlight-Pink-Towel-Cape-front.png",
real= "Sunlight-Pink-Towel-Cape-back.png",
days= "Sunlight",
year= "2023",
},
mischief_beak_mask = {
name= "Mischief Beak Mask",
front= "Days-of-Mischief-Beak-Mask-front.png",
days= "Mischief",
year= "2025",
},
"#;
    let catalog = parse_fandom_days_item_data(days_wt);
    assert!(days_item_belongs("Sunlight", "Days of Sunlight"));
    assert!(days_item_belongs("rainbow", "Days of Color"));
    assert!(!days_item_belongs("Sunlight", "Days of Mischief"));
    let sunlight: Vec<String> = catalog
        .iter()
        .filter(|item| days_item_belongs(&item.days, "Days of Sunlight"))
        .flat_map(|item| item.files.iter().cloned())
        .collect();
    let used = std::collections::HashSet::new();
    assert_eq!(
        pick_named_wear(&sunlight, "斗篷", "粉色毛巾斗篷", &used).as_deref(),
        Some("Sunlight-Pink-Towel-Cape-front.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Days-of-Mischief-Beak-Mask-front.png".into()],
            "面具",
            "鸦喙面具",
            &used
        )
        .as_deref(),
        Some("Days-of-Mischief-Beak-Mask-front.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Sunlight-Snorkel-Mask-real.png".into()],
            "面具",
            "潜水面具",
            &used
        )
        .as_deref(),
        Some("Sunlight-Snorkel-Mask-real.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Sunlight-Pink-Towel-Cape-front.png".into(),
                "Sunlight-Blue-Towel-Cape-front.png".into(),
                "Sunlight-Orange-Towel-Cape-front.png".into(),
            ],
            "斗篷",
            "蓝色毛巾斗篷",
            &used
        )
        .as_deref(),
        Some("Sunlight-Blue-Towel-Cape-front.png")
    );
    // 真实模块：缩进的结尾括号，条目里还有 alt_name 内层表。名字和图必须来自同一条目。
    let nested_wt = "\tmischief_feline_tail = {\n\t\tname\t\t= \"Mischief Feline Tail Accessory\" ,\n\t\talt_name\t= {\"Mischief Cat Tail\" },\n\t\treal\t\t= \"Days-of-Mischief-Feline-Accessory-Tail-back.png\",\n\t\tdays\t\t= \"Mischief\",\n\t\tyear\t\t= \"2025\",\n\t},\n\n\tmischief_puzzle_box = {\n\t\tname\t\t= \"Mischief Puzzle Box\",\n\t\talt_name\t= { \"Puzzle Box\" },\n\t\treal\t\t= \"Days-of-Mischief-Puzzle-Box.png\",\n\t\tdays\t\t= \"Mischief\",\n\t\tyear\t\t= \"2025\",\n\t},\n";
    let nested = parse_fandom_days_item_data(nested_wt);
    assert_eq!(nested.len(), 2);
    assert_eq!(nested[0].name, "Mischief Feline Tail Accessory");
    assert_eq!(nested[0].files, vec!["Days-of-Mischief-Feline-Accessory-Tail-back.png".to_string()]);
    assert_eq!(nested[1].name, "Mischief Puzzle Box");
    assert_eq!(nested[1].files, vec!["Days-of-Mischief-Puzzle-Box.png".to_string()]);
    let tabbed_wt = "sunlight_bonnet = {\nname= \"Sunlight Bonnet\",\nfront= \"Sunlight-Bonnet-Hat-front.png\",\ndays\t\t= \"Sunlight\",\nyear= \"2026\",\n},\n";
    let tabbed = parse_fandom_days_item_data(tabbed_wt);
    assert!(
        tabbed
            .iter()
            .any(|item| item.days == "Sunlight" && item.files.iter().any(|x| x.contains("Bonnet")))
    );
    assert_eq!(
        pick_named_wear(
            &["Sunlight-Bonnet-Hat-front.png".into()],
            "发饰",
            "蝴蝶结遮阳帽",
            &used
        )
        .as_deref(),
        Some("Sunlight-Bonnet-Hat-front.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Sunlight-Sporty-Surfboard.png".into()],
            "手持",
            "入门冲浪板",
            &used
        )
        .as_deref(),
        Some("Sunlight-Sporty-Surfboard.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Sunlight-Pink-Towel-Cape-front.png".into()],
            "斗篷",
            "粉色斗篷",
            &used
        ),
        None
    );
    let mixed = vec![
        "Days-of-Color-Rainbow-Cape-front.png".into(),
        "Days-of-Color-Dark-Rainbow-Cape-front.png".into(),
        "Days-of-Mischief-Goth-Cape-front.png".into(),
        "Sunlight-Pink-Towel-Cape-front.png".into(),
    ];
    assert_eq!(
        pick_named_wear(&mixed, "斗篷", "彩虹斗篷", &used).as_deref(),
        Some("Days-of-Color-Rainbow-Cape-front.png")
    );
    assert_eq!(
        pick_named_wear(&mixed, "斗篷", "暗彩斗篷", &used).as_deref(),
        Some("Days-of-Color-Dark-Rainbow-Cape-front.png")
    );
    assert_eq!(
        pick_named_wear(&mixed, "斗篷", "哥特斗篷", &used).as_deref(),
        Some("Days-of-Mischief-Goth-Cape-front.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Days-of-Summer-umbrella.png".into(), "Sunlight-Parasol.png".into()],
            "手持",
            "飞鸟纸伞礼包",
            &used
        ),
        None
    );
    let pants = vec![
        "Rainbow-Tied-Jumpsuit-real.png".into(),
        "Rainbow-Tied-Jumpsuit-back.png".into(),
        "Days_of_Rainbow_2022-Trousers.png".into(),
        "Rainbow-Trousers-Outfit-Back.png".into(),
        "Days-of-Color-Rainbow-Smock-front.png".into(),
    ];
    assert_eq!(
        fandom_item_key("Days_of_Rainbow_2022-Trousers.png"),
        "rainbow-trousers"
    );
    assert_eq!(
        fandom_item_key("Rainbow-Trousers-Outfit-Back.png"),
        "rainbow-trousers"
    );
    assert!(fandom_keys_same(
        &fandom_item_key("Days_of_Rainbow_2022-Trousers.png"),
        &fandom_item_key("Rainbow-Trousers-Outfit-Back.png")
    ));
    assert!(!fandom_keys_same(
        &fandom_item_key("Days_of_Rainbow_2022-Trousers.png"),
        &fandom_item_key("Rainbow-Tied-Jumpsuit-real.png")
    ));
    assert_eq!(
        pick_named_wear(&pants, "裤子", "彩虹链带短袖裤装", &used).as_deref(),
        Some("Days_of_Rainbow_2022-Trousers.png")
    );
    assert_eq!(
        pick_named_wear(&pants, "裤子", "彩虹阔腿裤礼包", &used).as_deref(),
        Some("Rainbow-Tied-Jumpsuit-real.png")
    );
    let mut used_pants = std::collections::HashSet::new();
    let chain = pick_named_wear(&pants, "裤子", "彩虹链带短袖裤装", &used_pants).unwrap();
    used_pants.insert(fandom_item_key(&chain));
    assert_eq!(
        pick_named_wear(&pants, "裤子", "彩虹阔腿裤礼包", &used_pants).as_deref(),
        Some("Rainbow-Tied-Jumpsuit-real.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Qixi Umbrella Front.png".into(),
                "Days-of-Summer-umbrella.png".into(),
                "Sunlight-Parasol.png".into(),
            ],
            "手持",
            "飞鸟纸伞礼包",
            &used
        )
        .as_deref(),
        Some("Qixi Umbrella Front.png")
    );
    assert_eq!(
        pick_named_wear(
            &["Qixi Festival Shoes.png".into(), "Sunlight-Sandals.png".into()],
            "鞋子",
            "盘扣平底鞋",
            &used
        )
        .as_deref(),
        Some("Qixi Festival Shoes.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Qixi pin.png".into(),
                "Hand in hand pin.png".into(),
                "Sunlight-Bonnet-Hat-front.png".into(),
            ],
            "发饰",
            "花梳发饰",
            &used
        )
        .as_deref(),
        Some("Qixi pin.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Wind Chime Prop.png".into(),
                "Wind Chime Prop Back.png".into(),
                "Qixi Umbrella Front.png".into(),
            ],
            "手持",
            "比翼风铃",
            &used
        )
        .as_deref(),
        Some("Wind Chime Prop.png")
    );
    assert_eq!(infer_cosmetic_kind("比翼风铃"), Some("手持"));
    assert_eq!(infer_cosmetic_kind("盘扣平底鞋"), Some("鞋子"));
    assert_eq!(
        pick_named_wear(
            &[
                "Sunlight-Crab-Float-real.png".into(),
                "Sunlight-Manta-Float.png".into(),
            ],
            "背饰",
            "鳐鲲泳圈",
            &used
        )
        .as_deref(),
        Some("Sunlight-Manta-Float.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Days-of-Treasure-Prop.png".into(),
                "Sunlight-Surfboard.png".into(),
            ],
            "手持",
            "寻宝铲",
            &used
        )
        .as_deref(),
        Some("Days-of-Treasure-Prop.png")
    );
    let mut turtle_names = std::collections::HashMap::new();
    turtle_names.insert(
        "Days-of-Nature-2022-Cape-front.png".into(),
        "Nature Turtle Cape".into(),
    );
    turtle_names.insert("Ocean_cape_front_v2.png".into(), "Ocean Cape".into());
    assert_eq!(
        pick_named_wear_named(
            &[
                "Days-of-Nature-2022-Cape-front.png".into(),
                "Ocean_cape_front_v2.png".into(),
                "Rainbow_cape_front.png".into(),
            ],
            "斗篷",
            "海龟斗篷",
            &used,
            &turtle_names
        )
        .as_deref(),
        Some("Days-of-Nature-2022-Cape-front.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Rainbow-Tied-Jumpsuit-real.png".into(),
                "Style-Wide-Leg-Jeans.png".into(),
            ],
            "裤子",
            "阔腿牛仔裤礼包",
            &used
        )
        .as_deref(),
        Some("Style-Wide-Leg-Jeans.png")
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Days-of-Fortune-Dragon-Mask.png".into(),
                "Dragonboat cape.png".into(),
            ],
            "面具",
            "粽叶发带面具",
            &used
        ),
        None
    );
    assert_eq!(
        pick_named_wear(
            &[
                "Mischief-Spider-Bun-front.png".into(),
                "Hair_2021_mischief_web_hair_iap.png".into(),
            ],
            "发型",
            "蜘蛛发型",
            &used
        )
        .as_deref(),
        Some("Mischief-Spider-Bun-front.png")
    );
    let dual = vec![
        "Rainbow-Cape-front.png".into(),
        "Days-of-Color-Rainbow-Cape-front.png".into(),
        "Days-of-Color-Dark-Rainbow-Cape-front.png".into(),
    ];
    let rainbow = pick_named_wear(&dual, "斗篷", "彩虹斗篷", &used).unwrap();
    assert_eq!(fandom_item_key(&rainbow), "rainbow-cape");
    assert_eq!(
        pick_named_wear(&dual, "斗篷", "暗彩斗篷", &used).as_deref(),
        Some("Days-of-Color-Dark-Rainbow-Cape-front.png")
    );
    assert!(fandom_keys_same(
        &fandom_item_key("Rainbow-Cape-front.png"),
        &fandom_item_key("Days-of-Color-Rainbow-Cape-front.png")
    ));
    assert!(!fandom_keys_same(
        &fandom_item_key("Rainbow-Cape-front.png"),
        &fandom_item_key("Days-of-Color-Dark-Rainbow-Cape-front.png")
    ));
    let mut guess = SkyCalendarItem {
        id: "event-x".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "恶作剧之日".into(),
        names: vec![],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: "ended".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: vec![preview(
            "兑换",
            "未知名斗篷".into(),
            "斗篷".into(),
            String::new(),
            String::new(),
        )],
    };
    let mut gsets = FandomFileSets::empty();
    gsets.event_page = vec!["Days-of-Mischief-Goth-Cape-front.png".into()];
    let mut gmap = std::collections::HashMap::new();
    gmap.insert(
        "Days-of-Mischief-Goth-Cape-front.png".into(),
        "https://static.wikia.nocookie.net/goth.png".into(),
    );
    apply_fandom_wear(&mut guess, &gsets, &gmap);
    assert!(guess.previews[0].wear.is_empty());
    let mut color = SkyCalendarItem {
        id: "event-color".into(),
        kind: "event".into(),
        region: "cn".into(),
        title: "缤纷飞行日".into(),
        names: vec![],
        season: String::new(),
        start: String::new(),
        end: String::new(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: vec![
            preview("兑换", "彩虹链带短袖裤装".into(), "裤子".into(), String::new(), String::new()),
            preview("商城", "彩虹阔腿裤礼包".into(), "裤子".into(), String::new(), String::new()),
        ],
    };
    let mut csets = FandomFileSets::empty();
    csets.event_wear.insert((color.title.clone(), "trousers-icon".into()), "Days_of_Rainbow_2022-Trousers.png".into());
    csets.event_wear.insert((color.title.clone(), "jumpsuit-icon".into()), "Rainbow-Tied-Jumpsuit-real.png".into());
    color.previews[0].icon = "trousers-icon".into();
    color.previews[1].icon = "jumpsuit-icon".into();
    let mut cmap = std::collections::HashMap::new();
    cmap.insert(
        "Days_of_Rainbow_2022-Trousers.png".into(),
        "https://static.wikia.nocookie.net/trousers.png".into(),
    );
    cmap.insert(
        "Rainbow-Tied-Jumpsuit-real.png".into(),
        "https://static.wikia.nocookie.net/jumpsuit.png".into(),
    );
    apply_fandom_wear(&mut color, &csets, &cmap);
    assert_eq!(color.previews[0].wear, "https://static.wikia.nocookie.net/trousers.png");
    assert_eq!(color.previews[1].wear, "https://static.wikia.nocookie.net/jumpsuit.png");
}
#[test]
fn sky_calendar_season_table() {
    let wt = "|[[致梵高季]]\n|2026-07-17\n|未知\n|style=\"text-align:left\"|\n|[[狂欢季]]\n|2026-04-23\n|2026-07-08\n";
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
    let items = parse_season_wikitext(wt, today);
    let names: Vec<_> = items.iter().map(|i| i.title.as_str()).collect();
    assert!(names.contains(&"致梵高季"));
    let vg = items.iter().find(|i| i.title == "致梵高季").unwrap();
    assert_eq!(vg.status, "live");
    assert_eq!(vg.start, "2026-07-17");
}
#[test]
fn sky_event_edition_uses_exact_page_items() {
    let catalog = parse_fandom_days_item_data(r#"
        return {
            hat = { name = "Winter Hat", alt_name = { "Fluffy Hat" }, icon = "Hat-icon.png", front = "Hat-front.png", iap = "Hat-iap.png", days = "Feast", year = "2024" },
            cape = { name = "Winter Cape", icon = "Cape-icon.png", front = "Cape-front.png", days = "Feast", year = "2025" },
            future = { name = "Future Cape", icon = "Future-icon.png", front = "Future-front.png", days = "Feast", year = "2026" }
        }
    "#);
    let items = fandom_edition_previews("{{Days Item|Fluffy Hat||table}}{{Icon Name|Winter Cape|name}}{{Days Item|Winter Cape||table}}", &catalog);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].slot, "商城");
    assert_eq!(items[0].wear, "File:Hat-front.png");
    assert_eq!(items[1].slot, "兑换");
    assert_eq!(items[1].icon, "File:Cape-icon.png");
    assert!(!items.iter().any(|p| p.name.contains("Future")));
}

#[test]
fn sky_calendar_smw_date_object() {
    let printouts = serde_json::json!({
        "开启时间": [{"raw": "2026-07-17 08:00:00", "timestamp": "2026-07-17T00:00:00+08:00"}],
        "结束时间": ["2026-10-02 23:59:00"]
    });
    assert_eq!(smw_first_text(&printouts, "开启时间"), "2026-07-17 08:00:00");
    assert_eq!(smw_first_text(&printouts, "结束时间"), "2026-10-02 23:59:00");
    let val = serde_json::json!({
        "query": {
            "results": {
                "致梵高季": {
                    "printouts": printouts,
                    "fullurl": "https://wiki.biligame.com/sky/%E8%87%B4%E6%A2%B5%E9%AB%98%E5%AD%A3"
                }
            }
        }
    });
    let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 9).unwrap();
    let items = parse_smw_pages(&val, "season", today);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "致梵高季");
    assert_eq!(items[0].start, "2026-07-17");
    assert_eq!(items[0].end, "2026-10-02");
    assert_eq!(items[0].status, "live");
    let event_open = serde_json::json!({
        "query": {
            "results": {
                "2024年花憩节": {
                    "printouts": {
                        "开启时间": ["2024-04-01 00:00:00"],
                        "结束时间": []
                    },
                    "fullurl": "https://wiki.biligame.com/sky/x"
                }
            }
        }
    });
    let open_items = parse_smw_pages(&event_open, "event", today);
    assert_eq!(open_items[0].status, "ended");
    let events = serde_json::json!({"query": {"results": {
        "2025年宴会节": {
            "fullurl": "https://wiki.biligame.com/sky/2025年宴会节",
            "printouts": {
                "开启时间": ["2025-12-24"], "结束时间": ["2026-01-13"],
                "分类": [{"fulltext": "分类:节日活动"}, {"fulltext": "分类:宴会节"}]
            }
        },
        "2025年狼人杀特别活动": {
            "fullurl": "https://wiki.biligame.com/sky/2025年狼人杀特别活动",
            "printouts": {
                "开启时间": ["2025-11-19"], "结束时间": ["2025-12-02"],
                "分类": [{"fulltext": "分类:节日活动"}, {"fulltext": "分类:联动活动"}]
            }
        }
    }}});
    let history = parse_smw_event_history(&events);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].title, "宴会节");
    assert_eq!(history[0].region, "cn");
    // 联动仍属于正式活动，只从预测历史中排除。
    assert_eq!(parse_smw_pages(&events, "event", today).len(), 2);
}
#[test]
fn sky_season_wear_without_spirit_page() {
    // 珍珠季实况：wiki 给迟疑渔翁的英文名 Hesitant Fisherman 在 Fandom 不存在（那边叫 Old Angler），
    // 毕业礼在 Fandom 叫 Pearl Ultimate。两者都只能从季节页文件池里排除别的先祖后认领。
    let mut sets = FandomFileSets::empty();
    sets.spirit.insert(
        "嬉闹孩童".into(),
        vec!["Cuddly-Rolling-Child-Hair-front.png".into(), "Cuddly-Rolling-Child-Outfit-front.png".into()],
    );
    sets.season_page = vec![
        "Cuddly-Rolling-Child-Outfit-front.png".into(),
        "Old-Angler-Hair-Accessory-front.png".into(),
        "Old-Angler-Mask-front.png".into(),
        "Old-Angler-Outfit-front.png".into(),
        "Pearl-Ultimate-Hair-Accessory-front.png".into(),
    ];
    let mut season = SkyCalendarItem {
        id: "season-珍珠季".into(),
        kind: "season".into(),
        region: "cn".into(),
        title: "珍珠季".into(),
        names: vec!["珍珠季".into()],
        season: "珍珠季".into(),
        start: String::new(),
        end: String::new(),
        status: "live".into(),
        note: String::new(),
        url: String::new(),
        cosmetics: String::new(),
        icon: String::new(),
        previews: Vec::new(),
    };
    for (name, kind) in [("迟疑渔翁 · 裤子", "裤子"), ("迟疑渔翁 · 发饰", "发饰"), ("渔翁的灯笼 · 毕业发饰", "毕业发饰")] {
        season.previews.push(preview("季卡", name.into(), kind.into(), String::new(), String::new()));
    }
    let chosen = choose_fandom_wear_files(&season, &sets);
    assert_eq!(
        chosen,
        vec![
            Some("Old-Angler-Outfit-front.png".to_string()),
            Some("Old-Angler-Hair-Accessory-front.png".to_string()),
            Some("Pearl-Ultimate-Hair-Accessory-front.png".to_string()),
        ]
    );
}

#[test]
fn sky_reprint_structured_wear() {
    let rows = extract_reprint_templates(
        "{{复刻项|复刻类型=集体复刻|同类序号=1|开始时间=2026-11-01|结束时间=2026-11-08|先祖中文名=蓬头青年|先祖英文名=Hairtousle Teen|装扮=耳饰、手持}}
{{复刻项|复刻类型=集体复刻|同类序号=1|开始时间=2026-11-01|结束时间=2026-11-08|先祖中文名=天才建筑师|先祖英文名=Talented Builder|装扮=发型、颈饰、裤子}}
{{复刻项|复刻类型=集体复刻|同类序号=1|开始时间=2026-11-01|结束时间=2026-11-08|先祖中文名=跳跃舞者|先祖英文名=Leaping Dancer|装扮=面具、手持}}",
    );
    let mut names = reprint_spirit_names(&rows);
    names.insert("别名先祖".into(), "Old_Name".into());
    // 来自 Module:Spirits/data 的关联；这些真实文件名不能靠 front/类型词猜出来。
    let data = r#"local data = {
        hairtousle_teen = {
            name = "Hairtousle Teen",
            instrument_real = "Belonging-Hairtousle-teen-Ukulele.png",
            headpiece_real = "Belonging_Headgear_Forest_Ear_Muffs.png",
            facepiece_real = "Belonging_Headgear_Forest_Ear_Muffs.png",
        },
        talented_builder = {
            name = "Talented Builder",
            alt_name = { "Voila" },
            hair_real = "Season_of_flight_hair_1_talented_builder_gs.png",
            neckpiece_real = "Season_of_flight_neckpiece_talented_builder.png",
            outfit_real = "Season_of_flight_pants_2_talented_builder_gs.PNG",
        },
        leaping_dancer = {
            name = "Leaping Dancer",
            instrument_real = "Gratitude-Leaping-Dancer-Small-Bell.png",
            mask_real = "Mask19-Seasonal_SoG_2019.png",
        },
        other = {
            name = "New Name",
            alt_name = { "Old Name", "Braces } in a name" },
            -- mask_front = "Wrong.png",
            mask = "Icon.png",
            mask_real = "Icon.png",
            hair_real = "A.jpg",
            hair_b_real = "B.jpg",
            cape_real = "Rear.png",
            cape_front = "Front.webp",
        },
        neighbor = {
            name = "Unrelated Owner",
            mask_real = "Wrong-owner.png",
        },
    }"#;
    let mut sets = FandomFileSets::empty();
    sets.reprint_wear = parse_fandom_spirit_wear(data, &names);
    let alias = &sets.reprint_wear["别名先祖"];
    assert_eq!(alias.get("斗篷").map(String::as_str), Some("Front.webp"));
    assert!(!alias.contains_key("面具"), "图标或相邻先祖的图片不能冒充实穿");
    assert!(!alias.contains_key("发型"), "同类多件物品没有明确对应时不能任意挑选");
    let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    let mut item = grouped_reprints(rows, "intl", "", today).remove(0);
    let mut wiki = std::collections::HashMap::new();
    for p in &item.previews {
        if let Some(name) = wear_filename_from_url(&p.wear) {
            wiki.insert(name, String::new());
        }
    }
    wiki.insert("天才建筑师裤子展示图.png".into(), "https://patchwiki.example/outfit.png".into());
    apply_wear_urls(&mut item, &wiki);
    let files = sets.all_files();
    let map = files.iter().map(|f| (f.clone(), format!("https://static.wikia.nocookie.net/{f}"))).collect();
    apply_fandom_wear(&mut item, &sets, &map);
    let expected = [
        ("蓬头青年 · 耳饰", "Belonging_Headgear_Forest_Ear_Muffs.png"),
        ("蓬头青年 · 手持", "Belonging-Hairtousle-teen-Ukulele.png"),
        ("天才建筑师 · 发型", "Season_of_flight_hair_1_talented_builder_gs.png"),
        ("天才建筑师 · 颈饰", "Season_of_flight_neckpiece_talented_builder.png"),
        ("跳跃舞者 · 面具", "Mask19-Seasonal_SoG_2019.png"),
        ("跳跃舞者 · 手持", "Gratitude-Leaping-Dancer-Small-Bell.png"),
    ];
    for (name, file) in expected {
        assert_eq!(item.previews.iter().find(|p| p.name == name).unwrap().wear, format!("https://static.wikia.nocookie.net/{file}"));
    }
    assert_eq!(item.previews.iter().find(|p| p.name == "天才建筑师 · 裤子").unwrap().wear, "https://patchwiki.example/outfit.png");
    assert!(item.previews.iter().filter(|p| p.slot == "先祖").all(|p| p.wear.is_empty()));
    let before: Vec<_> = item.previews.iter().map(|p| p.wear.clone()).collect();
    apply_fandom_wear(&mut item, &sets, &map);
    assert_eq!(item.previews.iter().map(|p| p.wear.clone()).collect::<Vec<_>>(), before);
    item.previews = vec![
        preview("兑换", "蓬头青年 · 耳饰".into(), "耳饰".into(), String::new(), String::new()),
        preview("兑换", "蓬头青年 · 面饰".into(), "面饰".into(), String::new(), String::new()),
        preview("兑换", "未知先祖 · 面具".into(), "面具".into(), String::new(), String::new()),
    ];
    sets.event_page.push("Other-Mask-front.png".into());
    assert!(choose_fandom_wear_files(&item, &sets).iter().all(Option::is_none));
}

#[test]
fn sky_event_icon_identity() {
    // 同一形状在两站的缩放、背景和白色边框不改变物品身份。
    let encode = |size: usize, frame: bool, color: [u8; 3], flipped: bool| {
        let mut pixels = vec![0u8; size * size * 4];
        for y in 0..size {
            for x in 0..size {
                let nx = if flipped { size - 1 - x } else { x };
                let shape = (nx > size / 5 && nx < size * 2 / 5 && y > size / 5 && y < size * 4 / 5)
                    || (nx > size / 5 && nx < size * 4 / 5 && y > size * 3 / 5 && y < size * 4 / 5);
                let border = frame && (x < 3 || y < 3 || x >= size - 3 || y >= size - 3);
                let i = (y * size + x) * 4;
                if shape { pixels[i..i + 4].copy_from_slice(&[color[0], color[1], color[2], 255]); }
                else if border { pixels[i..i + 4].copy_from_slice(&[255; 4]); }
            }
        }
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, size as u32, size as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&pixels).unwrap();
        }
        icon_signature(&bytes).unwrap()
    };
    let source = encode(128, true, [240, 80, 90], false);
    let same = encode(256, false, [230, 75, 85], false);
    let blue = encode(256, false, [50, 140, 240], false);
    let other = encode(256, false, [240, 80, 90], true);
    let monochrome = encode(256, false, [252, 242, 217], false);
    let item = FandomDaysItem { name: "不依赖名称".into(), files: vec!["opaque.png".into()], ..Default::default() };
    let variant = FandomDaysItem { files: vec!["other.png".into()], ..Default::default() };
    assert_eq!(choose_icon_wear(&source, &[(&item, &same), (&variant, &blue)]), Some("opaque.png"));
    assert_eq!(choose_icon_wear(&source, &[(&item, &same), (&variant, &same)]), None);
    assert_eq!(choose_icon_wear(&source, &[(&variant, &other)]), None);
    assert_eq!(choose_icon_wear(&monochrome, &[(&item, &same), (&variant, &other)]), Some("opaque.png"));
    // 单色图标无法区分只改配色的变体，不能猜一件填上。
    assert_eq!(choose_icon_wear(&monochrome, &[(&item, &same), (&variant, &blue)]), None);
    assert!(icon_signature(b"broken image").is_none());
    let missing = FandomDaysItem::default();
    assert_eq!(choose_icon_wear(&source, &[(&missing, &same)]), None);

    let catalog = parse_fandom_days_item_data(r#"
item = {
 name = "Opaque item",
 icon = "identity.png",
 real = "photo.png",
 days = "Mischief",
 year = "2026",
},
no_wear = {
 name = "No wear",
 icon = "only.png",
 real = "only.png",
 days = "Mischief",
}
"#);
    assert_eq!(catalog[0].icon, "identity.png");
    assert_eq!(catalog[0].files, ["photo.png"]);
    assert!(catalog[1].files.is_empty());
    let cosmetics = parse_fandom_cosmetics(r#"
event = {
 name = "Event outfit",
 days = "Bloom",
 icon = "outfit-icon.png",
 front = "outfit-photo.png",
},
spirit = {
 name = "Spirit",
 order = 3104,
 hairpiece = "hat-icon.png",
 hairpiece_real = "hat-photo.png",
},
missing = {
 name = "Unpictured",
 item_type = "outfit",
 icon = "icon.png",
 real = "icon.png",
}
"#);
    assert_eq!(cosmetics.len(), 2);
    assert!(cosmetics.iter().any(|e| e.kind == "*" && e.wear == "outfit-photo.png"));
    assert!(cosmetics.iter().any(|e| e.kind == "horn" && e.wear == "hat-photo.png"));
    assert_eq!(cosmetics.iter().find(|e| e.kind == "horn").unwrap().season, Some(31));
    let furniture = parse_fandom_closet_cosmetics(
        "| {{Icon| stool-icon.png | tab | label=Stool }}\n| {{Icon| stool-photo.png | closet_display | label=Stool }}\n| {{Icon| another-icon.png | tab }}", "prop");
    assert_eq!(furniture.len(), 1);
    assert_eq!(furniture[0].wear, "stool-photo.png");
}

#[test]
fn sky_season_catalog_structured_wear() {
    let names = std::collections::HashMap::from([
        ("嬉闹孩童".into(), "Cuddly Rolling Child".into()),
        ("渔翁的灯笼".into(), "Angler's Lantern".into()),
    ]);
    let data = r#"local data = {
        child = {
            name = "Cuddly Rolling Child",
            cape = "Cape-icon.png",
            cape_front = "Cape-front.png",
            cape_real = "Cape-back.png",
        },
        --[=[
        unbalanced ' " { brace in a multiline source comment
        ]=]
        pearl = {
            name = "Season of Pearl",
            guide_name = "Angler's Lantern",
            cape_u_front = "Ultimate-front.png",
            prop_u_real = "Couch.png ",
            prop_b_real = "Ship.png",
        },
    }"#;
    let media = parse_fandom_spirit_wear(data, &names);
    assert_eq!(media["嬉闹孩童"]["斗篷"], "Cape-front.png");
    assert_eq!(media["渔翁的灯笼"]["毕业斗篷"], "Ultimate-front.png");
    assert_eq!(media["渔翁的灯笼"]["毕业小型家具"], "Couch.png");
    assert_eq!(media["渔翁的灯笼"]["小型家具"], "Ship.png");
    let wt = "{{季节先祖板块\n|先祖中文名=嬉闹孩童\n|物品=斗篷\n}}";
    let mut seasons = vec![parse_season_page("珍珠季", "", wt, "", "", chrono::NaiveDate::from_ymd_opt(2026,11,9).unwrap())];
    assert!(seasons[0].items[0].wear.contains("Special:FilePath"));
    apply_season_media_urls(&mut seasons, &Default::default());
    let candidates = season_fandom_candidates(&seasons, &media);
    assert_eq!(candidates[&seasons[0].items[0].id], "Cape-front.png");
    let duplicate = seasons[0].items[0].clone();
    seasons[0].items.push(duplicate);
    assert!(season_fandom_candidates(&seasons, &media).is_empty());
    let wt = "{{季节先祖板块\n|先祖中文名=渔翁的灯笼\n|物品=毕业小型家具、毕业大型家具\n}}";
    let mut ambiguous = vec![parse_season_page("测试季节", "", wt, "", "", chrono::NaiveDate::from_ymd_opt(2026,11,9).unwrap())];
    apply_season_media_urls(&mut ambiguous, &Default::default());
    assert_eq!(media["渔翁的灯笼"]["毕业小型家具"], media["渔翁的灯笼"]["毕业大型家具"]);
    assert!(season_fandom_candidates(&ambiguous, &media).is_empty(), "一张 prop 图不能同时认作两件不同的家具");
}
