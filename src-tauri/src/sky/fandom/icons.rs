// 两站共享游戏图标，但译名、文件名、边框和配色不同。关联只在同一活动目录内进行。
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use super::{days_item_belongs, fandom_en_for_cn_event, fandom_file_urls, fandom_http_client_timeout,
    fandom_lookup_url, FandomDaysItem, FandomFileSets};
use super::super::{SkyCalendarItem, wiki::is_furniture_kind};

#[derive(Clone)]
struct IconMask {
    pixels: Vec<u8>,
    edges: Vec<f32>,
    ratio: f32,
}

pub(crate) struct IconSignature {
    masks: [Option<IconMask>; 2],
    colors: [f32; 12],
}

pub(crate) fn icon_signature(bytes: &[u8]) -> Option<IconSignature> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (width, height) = (reader.info().width as usize, reader.info().height as usize);
    if !(16..=1024).contains(&width) || !(16..=1024).contains(&height) {
        return None;
    }
    let mut bytes = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut bytes).ok()?;
    let channels = info.color_type.samples();
    let rgba: Vec<[u8; 4]> = bytes[..info.buffer_size()].chunks_exact(channels).map(|p| {
        match info.color_type {
            png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            _ => [p[0], p[0], p[0], 255],
        }
    }).collect();
    let masks = [false, true].map(|bright| icon_mask(&rgba, width, height, bright));
    let mut colors = [0.0; 12];
    for p in &rgba {
        let [r, g, b, alpha] = p.map(|v| v as f32 / 255.0);
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let delta = max - min;
        if max < 0.3 || delta / max < 0.35 || alpha < 0.5 { continue; }
        let hue = if max == r { ((g - b) / delta).rem_euclid(6.0) }
            else if max == g { (b - r) / delta + 2.0 } else { (r - g) / delta + 4.0 };
        let bin = hue * 2.0;
        let index = bin.floor() as usize % 12;
        colors[index] += (1.0 - bin.fract()) * delta * alpha;
        colors[(index + 1) % 12] += bin.fract() * delta * alpha;
    }
    let sum: f32 = colors.iter().sum();
    if sum > 0.0 {
        let histogram = colors;
        for i in 0..12 {
            colors[i] = (histogram[(i + 11) % 12] + 2.0 * histogram[i] + histogram[(i + 1) % 12]) / (4.0 * sum);
        }
    }
    Some(IconSignature { masks, colors })
}

fn icon_mask(rgba: &[[u8; 4]], width: usize, height: usize, bright: bool) -> Option<IconMask> {
    // 轮廓保留彩色装扮；亮部保留白色图标内的细节，容忍两站不同的阴影颜色。
    let mut mask: Vec<bool> = rgba.iter().map(|p| {
        let value = if bright { *p[..3].iter().min().unwrap() } else { *p[..3].iter().max().unwrap() };
        u32::from(value) * u32::from(p[3]) > (if bright { 128 } else { 96 }) * 255
    }).collect();
    let mut seen = vec![false; mask.len()];
    for start in 0..mask.len() {
        if !mask[start] || seen[start] { continue; }
        let mut component = vec![start];
        seen[start] = true;
        let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
        let mut cursor = 0;
        while cursor < component.len() {
            let p = component[cursor];
            cursor += 1;
            let (x, y) = (p % width, p / width);
            left = left.min(x); right = right.max(x); top = top.min(y); bottom = bottom.max(y);
            for next in [
                (x > 0).then(|| p - 1), (x + 1 < width).then(|| p + 1),
                (y > 0).then(|| p - width), (y + 1 < height).then(|| p + width),
            ].into_iter().flatten() {
                if mask[next] && !seen[next] { seen[next] = true; component.push(next); }
            }
        }
        // 图标边框同时覆盖四条边；单侧贴边的物品不能被当成边框删除。
        let frame = left * 100 < width * 4 && right * 100 > width * 96
            && top * 100 < height * 4 && bottom * 100 > height * 96;
        if frame || component.len() < 8 {
            for p in component { mask[p] = false; }
        }
    }
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    for (p, foreground) in mask.iter().enumerate() {
        if *foreground {
            left = left.min(p % width); right = right.max(p % width);
            top = top.min(p / width); bottom = bottom.max(p / width);
        }
    }
    if left >= right || top >= bottom { return None; }
    let (w, h) = ((right - left + 1) as f32, (bottom - top + 1) as f32);
    // 面积采样保持小图的细线；缩放、留白与原图分辨率不参与身份判断。
    let mut pixels = vec![0.0; 64 * 64];
    let mut luminance = vec![0.0; 64 * 64];
    for y in 0..64 {
        for x in 0..64 {
            let (x0, x1) = (x as f32 * w / 64.0, (x + 1) as f32 * w / 64.0);
            let (y0, y1) = (y as f32 * h / 64.0, (y + 1) as f32 * h / 64.0);
            let mut area = 0.0;
            for sy in y0.floor() as usize..y1.ceil() as usize {
                for sx in x0.floor() as usize..x1.ceil() as usize {
                    if mask[(top + sy.min(h as usize - 1)) * width + left + sx.min(w as usize - 1)] {
                        let weight = (x1.min((sx + 1) as f32) - x0.max(sx as f32))
                            * (y1.min((sy + 1) as f32) - y0.max(sy as f32));
                        area += weight;
                        let p = rgba[(top + sy.min(h as usize - 1)) * width + left + sx.min(w as usize - 1)];
                        luminance[y * 64 + x] += weight * (p[0] as f32 + p[1] as f32 + p[2] as f32)
                            * p[3] as f32 / (3.0 * 255.0);
                    }
                }
            }
            pixels[y * 64 + x] = area / ((x1 - x0) * (y1 - y0));
            luminance[y * 64 + x] /= (x1 - x0) * (y1 - y0);
        }
    }
    let mut smooth = vec![0.0; 64 * 64];
    for y in 1..63 {
        for x in 1..63 {
            for dy in 0..3 {
                for dx in 0..3 {
                    smooth[y * 64 + x] += luminance[(y + dy - 1) * 64 + x + dx - 1]
                        * [1.0, 2.0, 1.0][dx] * [1.0, 2.0, 1.0][dy] / 16.0;
                }
            }
        }
    }
    let mut edges = vec![0.0; 64 * 64];
    for y in 1..63 {
        for x in 1..63 {
            let i = y * 64 + x;
            edges[i] = (smooth[i + 1] - smooth[i - 1]).hypot(smooth[i + 64] - smooth[i - 64]);
        }
    }
    Some(IconMask { pixels: pixels.into_iter().map(|p| (p * 255.0).round() as u8).collect(), edges, ratio: w / h })
}

pub(crate) fn icon_similarity(a: &IconSignature, b: &IconSignature) -> f32 {
    let monochrome = a.colors.iter().sum::<f32>() == 0.0 || b.colors.iter().sum::<f32>() == 0.0;
    let similarities: Vec<_> = a.masks.iter().zip(&b.masks).map(|pair| match pair {
        (Some(a), Some(b)) => {
            let (mut overlap, mut union) = (0.0, 0.0);
            for (x, y) in a.pixels.iter().zip(&b.pixels) {
                overlap += *x.min(y) as f32;
                union += *x.max(y) as f32;
            }
            let outline = if union > 0.0 && a.ratio.min(b.ratio) / a.ratio.max(b.ratio) >= 0.8 { overlap / union } else { 0.0 };
            let (mut dot, mut aa, mut bb) = (0.0, 0.0, 0.0);
            for (x, y) in a.edges.iter().zip(&b.edges) { dot += x * y; aa += x * x; bb += y * y; }
            let detail = if aa * bb > 0.0 { dot / (aa * bb).sqrt() } else { 0.0 };
            (outline, detail)
        }
        _ => (0.0, 0.0),
    }).collect();
    let color: f32 = a.colors.iter().zip(b.colors).map(|(x, y)| x.min(y)).sum();
    // 游戏导出的单色图标没有物品配色；缺少颜色不能算作颜色不匹配。
    if monochrome {
        // 同一亮部遮罩比较轮廓和细节，避免人物虚线、挂绳阴影改变物品身份。
        similarities.iter().map(|(outline, detail)| 0.5 * outline + 0.5 * detail).fold(0.0, f32::max)
    } else {
        0.4 * similarities.iter().map(|(outline, _)| *outline).fold(0.0, f32::max)
            + 0.4 * similarities[0].1 + 0.2 * color
    }
}

pub(crate) fn choose_icon_wear<'a>(
    icon: &IconSignature,
    candidates: &[(&'a FandomDaysItem, &IconSignature)],
) -> Option<&'a str> {
    let index = choose_icon_match(icon, &candidates.iter().map(|(_, signature)| *signature).collect::<Vec<_>>())?;
    candidates[index].0.files.first().map(String::as_str)
}

pub(crate) fn choose_icon_match(icon: &IconSignature, candidates: &[&IconSignature]) -> Option<usize> {
    let mut scored: Vec<_> = candidates.iter().enumerate().map(|(index, other)| (index, icon_similarity(icon, other))).collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (index, score) = scored.first()?;
    // 实际跨站图标允许抗锯齿/阴影差异；接近的同形变体必须拒绝，不能靠列表顺序选中。
    if *score < 0.78 || scored.get(1).is_some_and(|(_, next)| score - next < 0.06) {
        return None;
    }
    Some(*index)
}

pub(crate) fn event_uses_icon_catalog(item: &SkyCalendarItem) -> bool {
    fandom_en_for_cn_event(&item.title).is_some_and(|en| !en.starts_with("NetEase/"))
}

async fn load_icon(client: &reqwest::Client, url: &str) -> Option<Arc<IconSignature>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, Arc<IconSignature>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().ok()?.get(url).cloned() { return Some(hit); }
    // 图床默认把 .png 转码为 WebP；匹配读取原始像素，避免有损压缩改变细线和颜色。
    let request_url = if url.contains("wikia.nocookie.net") {
        format!("{}{}format=original", url, if url.contains('?') { "&" } else { "?" })
    } else { url.to_owned() };
    let response = client.get(&request_url).header(reqwest::header::REFERER,
        if url.contains("wikia.nocookie.net") { "https://sky-children-of-the-light.fandom.com/" }
        else { "https://wiki.biligame.com/sky/" }).send().await.ok()?.error_for_status().ok()?;
    if response.content_length().is_some_and(|n| n > 2_000_000) { return None; }
    let bytes = response.bytes().await.ok()?;
    if bytes.len() > 2_000_000 { return None; }
    let signature = Arc::new(icon_signature(&bytes)?);
    cache.lock().ok()?.insert(url.to_owned(), signature.clone());
    Some(signature)
}

pub(crate) async fn resolve_event_icon_wear(items: &[SkyCalendarItem], sets: &mut FandomFileSets) {
    let events: Vec<_> = items.iter().filter(|e| e.kind == "event" && event_uses_icon_catalog(e)).collect();
    if events.is_empty() { return; }
    let candidates: Vec<_> = sets.days_items.iter().filter(|i| {
        !i.icon.is_empty() && events.iter().any(|e| fandom_en_for_cn_event(&e.title)
            .is_some_and(|en| days_item_belongs(&i.days, en)))
    }).collect();
    let icon_files: Vec<_> = candidates.iter().map(|i| i.icon.clone()).collect();
    let icon_urls = fandom_file_urls(&icon_files).await;
    let mut urls: HashSet<String> = candidates.iter().filter_map(|i| fandom_lookup_url(&icon_urls, &i.icon)).collect();
    for event in &events {
        urls.extend(event.previews.iter().filter(|p| p.wear.is_empty() && !p.icon.is_empty()
            && !is_furniture_kind(&p.kind) && (p.slot == "兑换" || p.slot == "商城")).map(|p| p.icon.clone()));
    }
    let Ok(client) = fandom_http_client_timeout(12) else { return; };
    let urls: Vec<_> = urls.into_iter().collect();
    let mut signatures = HashMap::new();
    for chunk in urls.chunks(6) {
        let client = &client;
        let get = |index: usize| async move {
            let url = chunk.get(index)?;
            Some((url.clone(), load_icon(client, url).await?))
        };
        let (a, b, c, d, e, f) = tokio::join!(get(0), get(1), get(2), get(3), get(4), get(5));
        signatures.extend([a, b, c, d, e, f].into_iter().flatten());
    }
    for event in events {
        let Some(en) = fandom_en_for_cn_event(&event.title) else { continue; };
        let source: Vec<_> = candidates.iter().filter(|i| days_item_belongs(&i.days, en))
            .filter_map(|i| Some((*i, signatures.get(&fandom_lookup_url(&icon_urls, &i.icon)?)?.as_ref())))
            .collect();
        for p in &event.previews {
            if !p.wear.is_empty() || is_furniture_kind(&p.kind) || (p.slot != "兑换" && p.slot != "商城") { continue; }
            let Some(icon) = signatures.get(&p.icon) else { continue; };
            if let Some(file) = choose_icon_wear(icon, &source) {
                sets.event_wear.insert((event.title.clone(), p.icon.clone()), file.to_owned());
            }
        }
    }
}
