// Fandom 物品识别：文件名、种类、中英名对照。
use super::super::wiki::*;

pub(crate) fn fandom_parse_season_name(wt: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = wt[from..].find("|season") {
        let i = from + i;
        let after = &wt[i + "|season".len()..];
        if after.starts_with("al") || after.starts_with("s") {
            from = i + 1;
            continue;
        }
        let line = wt[i + 1..].lines().next().unwrap_or("");
        let v = line.split_once('=')?.1.trim();
        let v = v.trim_start_matches('[').trim_end_matches(']');
        let v = v.split('|').next().unwrap_or(v).trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
        from = i + 1;
    }
    None
}

pub(crate) fn is_fandom_rejected_wear(name: &str) -> bool {
    let f = name.to_ascii_lowercase().replace([' ', '_'], "-");
    if !f.ends_with(".png") {
        return true;
    }
    const REJECT: &[&str] = &[
        "icon",
        "emote",
        "spell",
        "dyeable",
        "question-mark",
        "promo",
        "ticket",
        "logo",
        "discord",
        "kite",
        "furniture",
    ];
    REJECT.iter().any(|k| f.contains(k))
}

pub(crate) fn is_fandom_wear_file(name: &str) -> bool {
    if is_fandom_rejected_wear(name) {
        return false;
    }
    let f = name.to_ascii_lowercase().replace([' ', '_'], "-");
    let studio = f.contains("front")
        || f.contains("real")
        || f.contains("on-back")
        || f.contains("-back.");
    let typed = [
        "-cape.",
        "-outfit.",
        "-hair.",
        "-mask.",
        "-shoes.",
        "-accessory.",
        "-head-wrap.",
        "-trousers.",
        "-smock.",
        "-jumpsuit.",
        "-hat.",
        "-wetsuit.",
        "-shorts.",
        "-dress.",
        "-parasol.",
        "-umbrella.",
        "-prop.",
        "-pin.",
        "-instrument.",
        "-guitar.",
        "-jeans.",
        "-sunglasses.",
        "-wetsuit.",
        "-flipper.",
        "-float.",
    ]
    .iter()
    .any(|k| f.contains(k));
    studio || typed
}

pub(crate) fn fandom_kind_match(kind: &str, file: &str) -> bool {
    let f = file.to_ascii_lowercase().replace([' ', '_'], "-");
    let k = cosmetic_file_kind(kind);
    match k.as_str() {
        "斗篷" => f.contains("cape") || f.contains("shawl") || f.contains("cloak"),
        "裤子" => {
            f.contains("outfit")
                || f.contains("smock")
                || f.contains("jumpsuit")
                || f.contains("trousers")
                || f.contains("tunic")
                || f.contains("shorts")
                || f.contains("pants")
                || f.contains("dress")
                || f.contains("garment")
                || f.contains("wetsuit")
                || f.contains("jeans")
                || f.contains("frock")
        }
        "发型" => {
            (f.contains("hair") && !f.contains("accessory") && !f.contains("hat"))
                || f.contains("beret")
                || f.contains("beanie")
                || f.contains("head-wrap")
                || f.contains("glam")
                || f.contains("braid")
                || f.contains("bun")
                || f.contains("quiff")
                || f.contains("updo")
        }
        "发饰" => {
            f.contains("hair-accessory")
                || f.contains("head-accessory")
                || f.contains("headpiece")
                || f.contains("hat")
                || f.contains("bonnet")
                || f.contains("feline")
                || f.contains("-pin.")
                || f.contains("sunglass")
                || f.contains("veil")
                || f.contains("halo")
                || f.contains("-ears.")
                || f.contains("ears-")
                || f.ends_with("-ears")
                || f.contains("blossom")
                || f.contains("nimbus")
        }
        "鞋子" => {
            f.contains("shoe")
                || f.contains("loafer")
                || f.contains("sandal")
                || f.contains("boot")
                || f.contains("slipper")
                || f.contains("flipper")
                || f.contains("flat")
        }
        "面具" => f.contains("mask") || f.contains("visage") || f.contains("face-paint") || f.contains("snorkel"),
        "颈饰" => {
            f.contains("neck")
                || f.contains("necklace")
                || f.contains("tail")
                || (f.contains("accessory") && !f.contains("hair") && !f.contains("head") && !f.contains("ear"))
        }
        "耳饰" => f.contains("earring") || f.contains("headphones") || f.contains("kiss") || f.contains("hoop"),
        "手持" => {
            f.contains("prop")
                || f.contains("held")
                || f.contains("umbrella")
                || f.contains("canopy")
                || f.contains("parasol")
                || f.contains("broom")
                || f.contains("surfboard")
                || f.contains("fan")
                || f.contains("instrument")
                || f.contains("guitar")
                || f.contains("saxophone")
                || f.contains("violin")
                || f.contains("handpan")
                || f.contains("shovel")
                || f.contains("laptop")
                || f.contains("pipa")
                || f.contains("lute")
                || f.contains("lantern")
        }
        "面饰" => f.contains("face") || f.contains("sunglass") || f.contains("monocle") || f.contains("glasses"),
        "背饰" => {
            f.contains("on-back")
                || f.contains("buddy")
                || f.contains("float")
                || f.contains("backpack")
                || f.contains("jelly")
        }
        _ => false,
    }
}

pub(crate) fn fandom_file_rank(file: &str) -> u8 {
    let f = file.to_ascii_lowercase();
    if f.contains("front") {
        0
    } else if f.contains("real") {
        1
    } else if f.contains("back") {
        3
    } else {
        2
    }
}

pub(crate) fn fandom_en_tokens(s: &str) -> Vec<String> {
    s.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| t.len() >= 3 && !t.chars().all(|c| c.is_ascii_digit()))
        .filter(|t| {
            !matches!(
                *t,
                "the"
                    | "and"
                    | "for"
                    | "png"
                    | "with"
                    | "from"
                    | "days"
                    | "dear"
                    | "van"
                    | "gogh"
                    | "season"
                    | "sky"
                    | "guide"
                    | "color"
            )
        })
        .map(|t| t.to_string())
        .collect()
}

pub(crate) const FANDOM_CN_GLOSS: &[(&str, &str)] = &[
    ("星夜", "starry night"),
    ("星月", "starry night"),
    ("星流", "starry visage"),
    ("麦浪", "wheatfield wheat"),
    ("鸦渡", "wheatfield"),
    ("彩虹", "rainbow"),
    ("彩环", "rainbow tufted"),
    ("缤纷", "rainbow"),
    ("暗彩", "dark rainbow"),
    ("浅彩", "rainbow"),
    ("绘彩", "face paint"),
    ("摇滚", "glam"),
    ("飘带", "ribbon shawl"),
    ("绒绒", "beret"),
    ("头巾", "head wrap"),
    ("长袍", "smock"),
    // Fandom 英文和中文相反：Tied Jumpsuit 是阔腿，Trousers 是链带裤。
    ("阔腿", "tied jumpsuit"),
    ("链带", "trousers"),
    ("飞鸟纸伞", "qixi umbrella"),
    ("纸伞", "qixi umbrella"),
    ("花梳", "qixi pin"),
    ("盘扣", "qixi"),
    ("风铃", "chime"),
    ("清香粽叶", "dragonboat cape"),
    ("蒲葵", "dragonboat"),
    ("牛仔裤", "jeans"),
    ("阔腿牛仔", "wide jeans"),
    ("向日葵头巾", "sunflower bandana"),
    ("头巾脏辫", "treasure bandana"),
    ("寻宝铲", "treasure shovel"),
    ("寻宝长靴", "cavalier boots"),
    ("寻宝搭档", "treasure mate"),
    ("秘宝坠饰", "treasure coin"),
    ("探宝翎羽", "treasure seeker hat"),
    ("宝藏猎人", "treasure bounty"),
    ("围布寻宝", "adventurer garb"),
    ("珊瑚", "coral"),
    ("海浪面具", "ocean wave mask"),
    ("海浪斗篷", "nature wave cape"),
    ("海蓝波浪", "wave touched"),
    ("水波", "sea foam"),
    ("海洋斗篷", "ocean cape"),
    ("潮汐斗篷", "sea foam cape"),
    ("潮汐鞋子", "sea foam boots"),
    ("海龟", "turtle"),
    ("自然之声", "sonorous seashell"),
    ("绿叶短衫", "green tunic"),
    ("向阳花伞", "sunflower umbrella"),
    ("花瓣裙", "sundress"),
    ("玫瑰褶边", "rose embroidered"),
    ("骏首", "horse mask"),
    ("琅骏金织", "horse cape"),
    ("灵蛇", "snake"),
    ("舞龙鼓", "fortune drum"),
    ("龙头面具", "dragon mask"),
    ("舞狮", "lion"),
    ("祥兔面具", "rabbit mask"),
    ("虎年", "tiger"),
    ("牛年", "bull ox"),
    ("宝骏丹心", "ribboned ponytail"),
    ("启岁长裙", "pleated dress"),
    ("骐骥", "plush mount"),
    ("亨通眼镜", "token glasses"),
    ("献瑞折扇", "hand fan"),
    ("龙鳞束身", "dragon vestment"),
    ("盘龙围巾", "dragon stole"),
    ("年年有鱼", "fish"),
    ("温暖绒帽", "wool hat"),
    ("大桔", "orange"),
    ("流星斗篷", "meteor mantle"),
    ("优雅领巾", "cravat"),
    ("惊喜手杖", "scepter"),
    ("渐变双马尾", "amethyst tails"),
    ("心心相印", "heart beret"),
    ("冰晶滑雪", "ice snowboard"),
    ("爱心单片", "heart monocle"),
    ("立立星", "straight star"),
    ("卷卷星", "curved star"),
    ("绒边棉衣", "feast coat"),
    ("白雪斗篷", "snowflake cape"),
    ("雪人头饰", "snowkid"),
    ("暖冬围巾", "winter scarf"),
    ("雪花发卡", "snowflake pin"),
    ("火焰墨镜", "flame sunglasses"),
    ("爱心墨镜", "heart sunglasses"),
    ("蛛网朋克", "spider quiff"),
    ("蜘蛛发型", "spider bun"),
    ("疯女巫", "witch long"),
    ("女巫帽", "witch hat"),
    ("南瓜帽", "pumpkin"),
    ("飞行扫帚", "withered broom"),
    ("吸蟹伯爵斗篷", "crabula cloak"),
    ("吸蟹伯爵面具", "crabula mask"),
    ("玉兔尾巴", "bunny tail"),
    ("满月耳坠", "moonlight earring"),
    ("月华髻", "moonlight updo"),
    ("玉兔拖鞋", "bunny slipper"),
    ("玉兔发饰", "bunny accessory"),
    ("灯笼手持", "lantern"),
    ("纤云弄巧", "garland cape"),
    ("礼乐队斗篷", "marching cape"),
    ("礼乐队帽", "marching hat"),
    ("进行曲制服", "marching uniform"),
    ("凯旋提琴", "violin"),
    ("雏鸟之琴", "fledgling harp"),
    ("音韵吉他", "rhythm guitar"),
    ("凯旋手碟", "handpan"),
    ("萨克斯", "saxophone"),
    ("贝壳发夹", "seashell hairpin"),
    ("墨镜头饰", "sunglasses"),
    ("潜水套装", "diver wetsuit"),
    ("日光耳环", "helios hoop"),
    ("笔记本电脑", "laptop"),
    ("鸭舌帽", "swag cap"),
    ("连帽斗篷", "hoodie"),
    ("小狗拖鞋", "slippers"),
    ("礼服斗篷", "tuxedo cape"),
    ("电吉他", "electric guitar"),
    ("米色西服", "anniversary suit"),
    ("向日葵连衣裙", "sunflower sundress"),
    ("绊爱", "kizuna"),
    ("耳坠", "earring"),
    ("耳环", "earring"),
    ("耳机", "headphones"),
    ("披风", "mantle cape"),
    ("斗篷", "cape"),
    ("短发", "tufted hair"),
    ("向日葵", "sunflower"),
    ("花瓶", "vase"),
    ("粉", "pink"),
    ("蓝", "blue"),
    ("黄", "yellow orange"),
    ("橙", "orange"),
    ("毛巾", "towel"),
    ("纤云", "garland cape"),
    ("月华霓裳", "moonlight frock"),
    ("月华髻", "moonlight updo"),
    ("满月耳坠", "moonlight earring"),
    ("玉兔拖鞋", "bunny slippers"),
    ("玉兔尾巴", "tufted tail"),
    ("玉兔耳", "moonlight ears"),
    ("玉兔", "bunny"),
    ("金桂", "blossom"),
    ("花纱", "floral veil"),
    ("月华面纱", "moonlight veil"),
    ("莲叶光环", "lotus halo"),
    ("月华琵琶", "moonlight pipa"),
    ("月华长裤", "moonlight trousers"),
    ("月华祥云", "moonlight nimbus"),
    ("灯笼", "lantern"),
    ("莲叶", "lotus"),
    ("月相", "banner"),
    ("潜水", "snorkel"),
    ("冲浪板", "surfboard"),
    ("冲浪", "surfer"),
    ("坡道", "ramp"),
    ("泳圈", "float"),
    ("沙滩", "beach"),
    ("短裤", "shorts"),
    ("遥鲲", "manta"),
    ("鳐鲲", "manta"),
    ("披肩", "shawl"),
    ("哥特", "goth"),
    ("鸦喙", "beak"),
    ("枯叶", "withered leaf"),
    ("蜘蛛", "spider"),
    ("猫耳", "cat"),
    ("皮皮猫", "cat feline"),
    ("蛛网", "web spider"),
    ("鬼蝠", "bat"),
    ("女巫", "witch"),
    ("遮阳帽", "bonnet"),
    ("遮阳", "bonnet"),
    ("阳伞", "parasol umbrella"),
    ("凉鞋", "sandal"),
    ("水母", "jelly"),
    ("墨镜", "sunglasses"),
    ("蝴蝶结", "bow"),
    ("发夹", "hairpin"),
    ("鸦羽", "raven"),
    ("头巾", "head wrap"),
];

pub(crate) fn fandom_hint_tokens(hint: &str) -> Vec<String> {
    let mut keys: Vec<(&str, &str)> = FANDOM_CN_GLOSS.to_vec();
    keys.sort_by_key(|(cn, _)| std::cmp::Reverse(cn.chars().count()));
    let chars: Vec<char> = hint.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut hit: Option<(usize, &str)> = None;
        for (cn, en) in &keys {
            let cn_chars: Vec<char> = cn.chars().collect();
            if chars[i..].starts_with(&cn_chars) {
                hit = Some((cn_chars.len(), *en));
                break;
            }
        }
        if let Some((len, en)) = hit {
            tokens.extend(fandom_en_tokens(en));
            i += len;
            continue;
        }
        if chars[i].is_ascii_alphanumeric() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_alphanumeric() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            tokens.extend(fandom_en_tokens(&word));
        } else {
            i += 1;
        }
    }
    tokens
}

pub(crate) fn fandom_kindish_token(tok: &str) -> bool {
    matches!(
        tok,
        "cape"
            | "hair"
            | "outfit"
            | "mask"
            | "shoe"
            | "shoes"
            | "accessory"
            | "front"
            | "real"
            | "back"
            | "prop"
            | "held"
            | "hat"
            | "cloak"
            | "garment"
            | "umbrella"
    )
}

pub(crate) fn fandom_color_token(tok: &str) -> bool {
    matches!(
        tok,
        "pink"
            | "blue"
            | "yellow"
            | "orange"
            | "green"
            | "red"
            | "white"
            | "black"
            | "dark"
            | "light"
            | "gold"
            | "purple"
            | "brown"
    )
}

pub(crate) fn fandom_tok_hit(tok: &str, other: &str) -> bool {
    if tok == other {
        return true;
    }
    tok.len() >= 4 && other.len() >= tok.len() && other.contains(tok)
}

pub(crate) fn fandom_file_blob(file: &str, names: &std::collections::HashMap<String, String>) -> String {
    match names.get(file) {
        Some(n) if !n.is_empty() => format!("{n} {file}"),
        _ => file.to_string(),
    }
}

pub(crate) fn fandom_content_match_blob(hint: &str, blob: &str) -> (usize, usize) {
    let hint_toks: Vec<String> = fandom_hint_tokens(hint)
        .into_iter()
        .filter(|t| !fandom_kindish_token(t))
        .collect();
    let file_toks: Vec<String> = fandom_en_tokens(blob)
        .into_iter()
        .filter(|t| !fandom_kindish_token(t))
        .collect();
    let matched = hint_toks
        .iter()
        .filter(|t| file_toks.iter().any(|f| fandom_tok_hit(t, f)))
        .count();
    let extra = file_toks
        .iter()
        .filter(|f| !hint_toks.iter().any(|t| fandom_tok_hit(t, f)))
        .count();
    (matched, extra)
}

pub(crate) fn fandom_item_stem(file: &str) -> String {
    let mut f = file.to_ascii_lowercase().replace(['_', ' '], "-");
    if let Some(i) = f.rfind('.') {
        f.truncate(i);
    }
    loop {
        let mut changed = false;
        for suf in [
            "-front",
            "-real",
            "-back",
            "-side",
            "-interior",
            "-exterior",
            "-on-back",
            "-v2",
            "-v3",
            "-v4",
            "-v5",
        ] {
            if let Some(stripped) = f.strip_suffix(suf) {
                if !stripped.is_empty() {
                    f = stripped.to_string();
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    f
}

pub(crate) fn fandom_event_prefix_token(tok: &str) -> bool {
    matches!(
        tok,
        "color"
            | "colors"
            | "sunlight"
            | "mischief"
            | "fortune"
            | "love"
            | "bloom"
            | "nature"
            | "feast"
            | "music"
            | "style"
            | "treasure"
            | "moonlight"
            | "camping"
            | "fireworks"
            | "sky"
            | "tournament"
            | "personality"
            | "workshop"
            | "lazy"
            | "aurora"
            | "healing"
            | "cinnamoroll"
            | "kizuna"
    )
}

pub(crate) fn fandom_item_key(file: &str) -> String {
    let mut s = fandom_item_stem(file);
    while let Some(rest) = s.strip_prefix("days-of-") {
        if let Some(i) = rest.find('-') {
            if i + 1 < rest.len() && fandom_event_prefix_token(&rest[..i]) {
                s = rest[i + 1..].to_string();
                continue;
            }
        }
        s = rest.to_string();
        break;
    }
    s = s
        .split('-')
        .filter(|p| !(p.len() == 4 && p.bytes().all(|c| c.is_ascii_digit())))
        .collect::<Vec<_>>()
        .join("-");
    for suf in ["-outfit", "-prop"] {
        if let Some(stripped) = s.strip_suffix(suf) {
            if !stripped.is_empty() {
                s = stripped.to_string();
            }
        }
    }
    s
}

pub(crate) fn fandom_keys_same(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    let Some(prefix) = long.strip_suffix(short).and_then(|p| p.strip_suffix('-')) else {
        return false;
    };
    !prefix.is_empty() && !prefix.split('-').any(fandom_color_token)
}

pub(crate) fn fandom_stem_used(used: &std::collections::HashSet<String>, file: &str) -> bool {
    let k = fandom_item_key(file);
    used.contains(&k) || used.iter().any(|u| fandom_keys_same(u, &k))
}

pub(crate) fn fandom_files_from_images_value(page: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let Some(images) = page.get("images").and_then(|v| v.as_array()) else {
        return out;
    };
    for im in images {
        let title = im.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let name = title.strip_prefix("File:").unwrap_or(title).to_string();
        if is_fandom_wear_file(&name) {
            out.push(name);
        }
    }
    out
}

pub(crate) fn fandom_year_page(hit: &str, year: &str) -> String {
    if year.len() == 4 && year.chars().all(|c| c.is_ascii_digit()) {
        if let Some((base, tail)) = hit.rsplit_once('/') {
            if tail.len() == 4 && tail.chars().all(|c| c.is_ascii_digit()) {
                return format!("{}/{}", base, year);
            }
        }
        return format!("{}/{}", hit.trim_end_matches('/'), year);
    }
    hit.to_string()
}

pub(crate) fn fandom_event_root(title: &str) -> String {
    if let Some((base, tail)) = title.rsplit_once('/') {
        if tail.len() == 4 && tail.chars().all(|c| c.is_ascii_digit()) {
            return base.to_string();
        }
    }
    title.trim_end_matches('/').to_string()
}

pub(crate) fn fandom_cn_item_name(en: &str) -> Option<String> {
    let en_toks = fandom_en_tokens(en);
    if en_toks.is_empty() {
        return None;
    }
    let mut best: Option<(usize, usize, String)> = None;
    for (cn, phrase) in FANDOM_CN_GLOSS {
        let phrase_toks = fandom_en_tokens(phrase);
        if phrase_toks.is_empty() {
            continue;
        }
        if phrase_toks.iter().all(|t| fandom_kindish_token(t)) {
            continue;
        }
        if !phrase_toks
            .iter()
            .all(|t| en_toks.iter().any(|e| fandom_tok_hit(t, e) || fandom_tok_hit(e, t)))
        {
            continue;
        }
        let hits = phrase_toks.len();
        let cn_len = cn.chars().count();
        let better = match best.as_ref() {
            None => true,
            Some((h, l, _)) => hits > *h || (hits == *h && cn_len > *l),
        };
        if better {
            best = Some((hits, cn_len, (*cn).to_string()));
        }
    }
    best.map(|(_, _, cn)| cn)
}
