import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./api";
import { beijingDay } from './cycleReset';

export interface SkyDailyRawResult {
  success: boolean;
  date_str: string;
  post_text: string;
  all_images: string[];
}

export async function fetchSkyDailyFromBackend(): Promise<SkyDailyRawResult | null> {
  if (isTauri()) {
    try {
      return await invoke<SkyDailyRawResult>("fetch_sky_daily_data");
    } catch (e) {
      console.warn("Backend fetch_sky_daily_data failed:", e);
      return null;
    }
  }
  return null;
}

export interface SkyQuestCalendarItem {
  date: string;
  quest_id: string;
  quest_cn: string;
}

export interface SkyQuestCalendar {
  items: SkyQuestCalendarItem[];
  image?: string | null;
  seasonImage?: string | null;
  eventImage?: string | null;
  shardImage?: string | null;
}

function itemsFromCalendarJson(val: unknown, today: string): SkyQuestCalendarItem[] {
  const list = (val as { data?: { task_list?: unknown } } | null)?.data?.task_list;
  if (!Array.isArray(list)) return [];
  const items: SkyQuestCalendarItem[] = [];
  for (const raw of list) {
    const v = raw as { date?: string; quest_id?: string; quest_cn?: string };
    const date = String(v.date || '');
    const quest_cn = String(v.quest_cn || '').trim();
    if (date !== today || !quest_cn) continue;
    items.push({ date, quest_id: String(v.quest_id || ''), quest_cn });
  }
  return items;
}

function imageFromTypedJson(val: unknown, today: string): string | undefined {
  const v = val as { date?: string; tcurl?: string; image?: string } | null;
  if (!v || v.date !== today) return undefined;
  const prefix = `https://api.qmkjcm.cn/gyapi/images/${today.replace(/-/g, '')}/`;
  return [v.tcurl, v.image].find(url => typeof url === 'string' && url.startsWith(prefix));
}

async function fetchSkyQuestCalendarFromWeb(): Promise<SkyQuestCalendar | null> {
  const today = beijingDay();
  const [listVal, imgVal, seasonVal, eventVal, shardVal] = await Promise.all([
    fetch('https://api.qmkjcm.cn/api/gy_mr_rw/').then(r => r.ok ? r.json() : null).catch(() => null),
    fetch('https://api.qmkjcm.cn/gyapi/rwt/mrrw/').then(r => r.ok ? r.json() : null).catch(() => null),
    fetch('https://api.qmkjcm.cn/gyapi/rwt/jlwz/').then(r => r.ok ? r.json() : null).catch(() => null),
    fetch('https://api.qmkjcm.cn/gyapi/rwt/db/').then(r => r.ok ? r.json() : null).catch(() => null),
    fetch('https://api.qmkjcm.cn/gyapi/rwt/lsd/').then(r => r.ok ? r.json() : null).catch(() => null)
  ]);
  const items = itemsFromCalendarJson(listVal, today);
  if (!items.length) return null;
  const image = imageFromTypedJson(imgVal, today);
  const seasonImage = imageFromTypedJson(seasonVal, today);
  const eventImage = imageFromTypedJson(eventVal, today);
  return { items, image, seasonImage, eventImage, shardImage: imageFromTypedJson(shardVal, today) };
}

export async function fetchSkyQuestCalendar(): Promise<SkyQuestCalendar | null> {
  if (isTauri()) {
    try {
      return await invoke<SkyQuestCalendar>("fetch_sky_quest_calendar");
    } catch (e) {
      console.warn("Backend fetch_sky_quest_calendar failed:", e);
    }
  }
  return fetchSkyQuestCalendarFromWeb();
}

function invokeWithTimeout<T>(cmd: string, ms: number, args?: Record<string, unknown>): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error(`${cmd} timed out`)), ms);
    invoke<T>(cmd, args)
      .then(res => {
        clearTimeout(timer);
        resolve(res);
      })
      .catch(err => {
        clearTimeout(timer);
        reject(err);
      });
  });
}

export interface SkyCalendarPreview {
  slot: string;
  name: string;
  kind: string;
  icon: string;
  wear?: string;
}

export interface SkyCalendarItem {
  id: string;
  kind: string;
  region: string;
  title: string;
  names: string[];
  season: string;
  start: string;
  end: string;
  status: string;
  note: string;
  url: string;
  cosmetics?: string;
  icon?: string;
  previews?: SkyCalendarPreview[];
}

export interface SkyEventEdition {
  title: string;
  region: string;
  start: string;
  end: string;
  url: string;
}

export async function fetchSkyEventEdition(edition: SkyEventEdition): Promise<SkyCalendarItem> {
  return invokeWithTimeout<SkyCalendarItem>('fetch_sky_event_edition', 120000, { edition });
}

export interface SkyCalendarFeed {
  as_of: string;
  season?: SkyCalendarItem | null;
  events: SkyCalendarItem[];
  cn_now: SkyCalendarItem[];
  cn_next: SkyCalendarItem[];
  intl_now: SkyCalendarItem[];
  intl_next: SkyCalendarItem[];
  event_history?: SkyEventEdition[];
  /** 档期拉到了、但图片查询之类子数据源失败 */
  warnings?: string[];
}

/** 失败时抛出，由 fetchCache 记进数据源状态。 */
export async function fetchSkyCalendar(): Promise<SkyCalendarFeed | null> {
  if (!isTauri()) return null;
  return await invokeWithTimeout<SkyCalendarFeed>("fetch_sky_calendar", 120000);
}

export interface SkySeasonWishItem {
  id: string;
  name: string;
  spirit: string;
  kind: string;
  slot: string;
  cost_desc: string;
  candles: number;
  ascended: number;
  hearts: number;
  icon?: string;
  wear?: string;
}

export interface SkySeasonCatalogItem {
  id: string;
  title: string;
  slogan: string;
  realm: string;
  loc: string;
  start: string;
  end: string;
  status: string;
  url: string;
  icon?: string;
  quests: string;
  items: SkySeasonWishItem[];
  /** 先祖 → 买齐整棵树的季蜡 */
  tree_candles?: Record<string, number>;
}

export interface SkySeasonCatalog {
  as_of: string;
  seasons: SkySeasonCatalogItem[];
}

/** 失败时抛出，由 fetchCache 记进数据源状态。 */
export async function fetchSkySeasons(): Promise<SkySeasonCatalog | null> {
  if (!isTauri()) return null;
  return await invokeWithTimeout<SkySeasonCatalog>("fetch_sky_seasons", 120000);
}

export interface SkyEventCatalog {
  as_of: string;
  events: SkySeasonCatalogItem[];
}

export interface SkyEventGuideImage {
  title: string;
  url: string;
  source_url: string;
}
export interface SkyEventGuideImages {
  images: SkyEventGuideImage[];
  unavailable_sources: string[];
}
export async function fetchSkyEventGuideImages(sources: string[]): Promise<SkyEventGuideImages> {
  return invoke<SkyEventGuideImages>('fetch_sky_event_guide_images', { sources });
}

/** 失败时抛出，由 fetchCache 记进数据源状态。 */
export async function fetchSkyEvents(): Promise<SkyEventCatalog | null> {
  if (!isTauri()) return null;
  return await invokeWithTimeout<SkyEventCatalog>("fetch_sky_events", 120000);
}

export interface SkyRealmWings {
  name: string;
  collected: number;
  total: number;
}

export interface SkyWingBuff {
  name: string;
  collected: boolean;
}

export interface SkyRoleWings {
  success: boolean;
  role_id: string;
  source: string;
  collected: number;
  total: number;
  maps: SkyRealmWings[];
  buffs: SkyWingBuff[];
  error: string;
}

export async function fetchSkyRoleWings(roleId: string): Promise<SkyRoleWings> {
  if (!isTauri()) {
    throw new Error("桌面端才能从大神同步");
  }
  return await invoke<SkyRoleWings>("fetch_sky_role_wings", { roleId });
}
