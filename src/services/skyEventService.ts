import { getTodayDateStr as todayStr } from './skyDailyService';
/**
 * 光遇节日页目录：wiki 节日活动名单，每种节日只留最新一届。
 * 打卡只存在本地，不写回 wiki。
 */
import { fetchSkyEvents, fetchSkyEventGuideImages, type SkyEventGuideImage, type SkyEventGuideImages, type SkyCalendarItem, type SkyEventCatalog, type SkySeasonCatalogItem } from './skyApi';
import type { SkyDailyGuidePayload } from './skyDailyService';
import type { SeasonalGuide, SeasonalItem } from '../types';
import { cachedFetch } from './fetchCache';
import { progressMap } from './progress';

const CACHE_KEY = 'sky.events';
const WISH_SCOPE = 'sky.event_wish';

export type EventWishStatus = Record<string, { items?: Record<string, boolean> }>;

export function eventName(name: string): string {
  return name.trim().replace(/^\d{4}\s*年\s*/, '').replace(/\s+/g, '');
}

/** 目录是国服的最新一届；名称与开场日期同时对应才可附攻略和兑换项。 */
export function matchEventGuide(row: SkyCalendarItem, guides: SeasonalGuide[]): SeasonalGuide | undefined {
  if (row.kind !== 'event' || row.region === 'intl' || !row.start) return undefined;
  return guides.find(guide => eventName(guide.season) === eventName(row.title) && guide.start === row.start);
}

export function eventDailyInstructions(guide?: SeasonalGuide): string {
  return (guide?.quests || '').split(/(?<=[。！？])|\n+/)
    .map(line => line.trim()).filter(line => /每日|每天|日常/.test(line)).join('\n');
}

export function eventImageSources(row: SkyCalendarItem, guides: SeasonalGuide[]): string[] {
  return [...new Set([row.url, matchEventGuide(row, guides)?.sourceUrl].filter((url): url is string => !!url))]
    .filter(url => {
      try {
        const parsed = new URL(url);
        return parsed.protocol === 'https:' && ['sky.163.com', 'wiki.biligame.com', 'sky-children-of-the-light.fandom.com'].includes(parsed.hostname);
      } catch { return false; }
    });
}

export function dailyEventImages(row: SkyCalendarItem, payload: SkyDailyGuidePayload | null, day: string): SkyEventGuideImage[] {
  if (row.region === 'intl' || payload?.dateStr !== day || row.start > day || (row.end && row.end < day)) return [];
  const images = (payload.activityImages || []).filter(image => eventName(image.caption).includes(eventName(row.title)))
    .map(image => ({ title: image.caption, url: image.url, source_url: 'https://m.weibo.cn/u/7360748659' }));
  const daily = payload.eventDaily;
  if (daily?.imageUrl && eventName(daily.name) === eventName(row.title) && !images.some(image => image.url === daily.imageUrl)) {
    images.unshift({ title: `${day} · 每日代币攻略`, url: daily.imageUrl, source_url: 'https://m.weibo.cn/u/7360748659' });
  }
  return images;
}

export async function getSkyEventGuideImages(row: SkyCalendarItem, sources: string[], force = false, onUpdate?: (data: SkyEventGuideImages) => void) {
  return cachedFetch<SkyEventGuideImages>({
    key: `sky.event_images/${row.region}/${eventName(row.title)}/${row.start}/${sources.join('|')}`,
    label: `${row.title}图文攻略`,
    fetch: () => fetchSkyEventGuideImages(sources),
    fresh: (data, at) => !data.unavailable_sources.length && Date.now() - at < (data.images.length ? 24 : .5) * 3600_000,
    warn: data => data.unavailable_sources.length ? '部分攻略来源暂时无法访问' : '',
    force, onUpdate,
  });
}

function statusLabel(status?: string): string {
  if (status === 'live') return '进行中';
  if (status === 'upcoming') return '未开';
  return '已结束';
}

export function loadEventWishStatus(characterId = ""): EventWishStatus {
  return characterId ? progressMap(WISH_SCOPE, characterId) : {};
}

function catalogItemToGuide(row: SkySeasonCatalogItem, wish: EventWishStatus): SeasonalGuide {
  const st = wish[row.id] || {};
  const itemList: SeasonalItem[] = (row.items || []).map(it => ({
    id: it.id,
    name: it.name,
    costDesc: it.cost_desc || (it.kind ? it.kind : ''),
    candles: it.candles || 0,
    ascended: it.ascended || 0,
    hearts: it.hearts || 0,
    obtained: st.items?.[it.id] === true,
    icon: it.icon || '',
    slot: it.slot || '',
    wear: it.wear || '',
    kind: it.kind,
    spirit: it.spirit
  }));
  return {
    id: row.id,
    sourceUrl: row.url,
    season: row.title,
    shortName: row.title,
    realm: '',
    loc: row.loc || '',
    quests: row.quests || '',
    benefit: row.slogan || '',
    items: itemList.map(i => i.name).join('、'),
    itemList,
    task: row.quests || '',
    priority: statusLabel(row.status),
    candles: 0,
    ascended: 0,
    hearts: 0,
    selected: false,
    status: row.status,
    start: row.start,
    end: row.end,
    icon: row.icon || ''
  };
}

export function mergeEventCatalog(catalog: SkyEventCatalog, wish: EventWishStatus): SeasonalGuide[] {
  return (catalog.events || []).map(row => catalogItemToGuide(row, wish));
}

export async function getSkyEventCatalog(
  force = false,
  onUpdate?: (catalog: SkyEventCatalog) => void
): Promise<SkyEventCatalog | null> {
  return cachedFetch<SkyEventCatalog>({
    key: CACHE_KEY,
    label: '光遇节日目录',
    fetch: fetchSkyEvents,
    fresh: data => data.as_of === todayStr(),
    valid: data => Array.isArray(data.events) && data.events.length > 0,
    onUpdate,
    force
  });
}
