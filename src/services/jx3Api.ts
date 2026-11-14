import { invoke } from "@tauri-apps/api/core";
import type { Jx3DailyCalendar, Jx3MapEvent, MingyiScanResult, Jx3ExteriorScan, Jx3GoodsHit, Jx3GoodsQuote } from "../types";
import { isTauri } from "./api";

// 日历和诛恶只走 Rust 解析（src-tauri/src/jx3.rs）；浏览器调试没有这两个数据源。
// 失败时抛出，调用方据此记数据源状态。

// 剑网3活动日历。权威路径 /active/calendar；num=0 当天。
export async function getJx3DailyCalendar(num = 0): Promise<Jx3DailyCalendar | null> {
  if (num < 0 || !isTauri()) return null;
  return await invoke<Jx3DailyCalendar>("fetch_jx3_daily", { num });
}

export async function getJx3Celebs(name: '云从社' | '披风会' | '楚天社'): Promise<Jx3MapEvent[] | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<Jx3MapEvent[]>("fetch_jx3_celebs", { name });
  } catch (e) {
    console.warn("Tauri fetch_jx3_celebs error:", e);
    return null;
  }
}

/** 魔盒副本树。权威是 fb_map.json 资料片→本，不是掉落表。 */
export async function getJx3BoxDungeonTree(): Promise<unknown | null> {
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_fb_map");
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_fb_map error:", e);
      return null;
    }
  }
  try {
    const res = await fetch("https://cdn.jsdelivr.net/gh/JX3BOX/jx3box-data@master/data/fb/fb_map.json");
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX fb_map:", err);
    return null;
  }
}

export async function getJx3BoxSerendipities(page: number, per = 50): Promise<unknown | null> {
  const p = Math.max(1, Math.floor(page) || 1);
  const n = Math.min(50, Math.max(1, Math.floor(per) || 50));
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_serendipities", { page: p, per: n });
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_serendipities error:", e);
      return null;
    }
  }
  try {
    const res = await fetch(`https://node.jx3box.com/serendipities?page=${p}&per=${n}`);
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX serendipities:", err);
    return null;
  }
}

export async function getJx3BoxReputations(page = 1): Promise<unknown | null> {
  const p = Math.max(1, Math.floor(page) || 1);
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_reputations", { page: p });
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_reputations error:", e);
      return null;
    }
  }
  try {
    const res = await fetch(`https://node.jx3box.com/reputations?client=std&page=${p}&per=50`);
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX reputations:", err);
    return null;
  }
}

/** 物品 id（`5_87714`）批量换名字，一次最多 50 个。 */
export async function getJx3BoxItemsByIds(ids: string[]): Promise<unknown | null> {
  const clean = ids.map(id => id.trim()).filter(id => /^\d+_\d+$/.test(id)).slice(0, 50);
  if (!clean.length) return null;
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_items_by_ids", { ids: clean });
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_items_by_ids error:", e);
      return null;
    }
  }
  try {
    const res = await fetch(`https://node.jx3box.com/item_merged/id/${clean.join(",")}?per=50`);
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX items by ids:", err);
    return null;
  }
}

export async function searchJx3BoxAchievements(keyword: string, page = 1): Promise<unknown | null> {
  const q = keyword.trim();
  if (!q) return null;
  const p = Math.max(1, Math.floor(page) || 1);
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_achievement_search", { keyword: q, page: p });
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_achievement_search error:", e);
      return null;
    }
  }
  try {
    const res = await fetch(
      `https://node.jx3box.com/achievement/search?keyword=${encodeURIComponent(q)}&page=${p}`
    );
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX achievement search:", err);
    return null;
  }
}

export async function searchJx3BoxItems(keyword: string): Promise<unknown | null> {
  const q = keyword.trim();
  if (!q) return null;
  if (isTauri()) {
    try {
      return await invoke<unknown>("fetch_jx3_box_item_search", { keyword: q });
    } catch (e) {
      console.warn("Tauri fetch_jx3_box_item_search error:", e);
      return null;
    }
  }
  try {
    const res = await fetch(`https://node.jx3box.com/item/search?keyword=${encodeURIComponent(q)}&per=50`);
    if (!res.ok) return null;
    return await res.json();
  } catch (err) {
    console.error("Failed to fetch JX3BOX item search:", err);
    return null;
  }
}

export async function scanJx3Exterior(clientRoot?: string | null): Promise<Jx3ExteriorScan> {
  if (isTauri()) {
    return await invoke<Jx3ExteriorScan>("scan_jx3_exterior", { clientRoot: clientRoot || null });
  }
  throw new Error("仅在桌面端可用");
}

export async function fetchJx3GoodsQuote(id: string): Promise<Jx3GoodsQuote> {
  if (isTauri()) {
    return await invoke<Jx3GoodsQuote>("fetch_jx3_goods_quote", { id });
  }
  throw new Error("仅在桌面端可用");
}

export async function searchJx3Goods(name: string): Promise<Jx3GoodsHit[]> {
  if (isTauri()) {
    return await invoke<Jx3GoodsHit[]>("search_jx3_goods", { name });
  }
  throw new Error("仅在桌面端可用");
}

export async function fetchJx3GoodsByName(name: string): Promise<Jx3GoodsQuote> {
  if (isTauri()) {
    return await invoke<Jx3GoodsQuote>("fetch_jx3_goods_by_name", { name });
  }
  throw new Error("仅在桌面端可用");
}

export async function scanMingyiRoles(path?: string | null): Promise<MingyiScanResult> {
  if (isTauri()) {
    return await invoke<MingyiScanResult>("scan_mingyi_roles", { path: path || null });
  }
  throw new Error("仅在桌面端可用");
}
