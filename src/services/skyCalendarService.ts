import { beijingDay as todayStr } from './cycleReset';
/**
 * 光遇档期：当前季节 / 国服回归先祖 / 国际服已公告回归。
 * 名字只来自 wiki 复刻项；国服空档可展示未点名窗口，国际服只展示已点名档期。
 */

import { fetchSkyCalendar, type SkyCalendarFeed, type SkyCalendarItem } from './skyApi';
import { cachedFetch } from './fetchCache';

export type { SkyCalendarFeed, SkyCalendarItem };

const CACHE_KEY = 'sky.calendar';



function parseYmd(raw: string): { y: number; m: number; d: number; iso: string } | null {
  const m = (raw || '').trim().match(/^(\d{4})-(\d{1,2})-(\d{1,2})/);
  if (!m) return null;
  const y = Number(m[1]);
  const mo = Number(m[2]);
  const d = Number(m[3]);
  const date = new Date(Date.UTC(y, mo - 1, d));
  if (!y || date.getUTCFullYear() !== y || date.getUTCMonth() !== mo - 1 || date.getUTCDate() !== d) return null;
  return { y, m: mo, d, iso: `${y}-${String(mo).padStart(2, '0')}-${String(d).padStart(2, '0')}` };
}

function dayNumber(raw: string): number | null {
  const date = parseYmd(raw);
  return date ? Date.UTC(date.y, date.m - 1, date.d) / 86400000 : null;
}

export function calendarStatus(item: SkyCalendarItem, today = todayStr()): string {
  if (item.status === 'inferred') return 'inferred';
  const now = dayNumber(today), start = dayNumber(item.start), end = dayNumber(item.end);
  if (now === null) return item.status;
  if (end !== null && end < now) return 'ended';
  if (start !== null && start > now) return 'upcoming';
  if (start !== null && (end !== null || item.kind === 'season')) return 'live';
  return item.status;
}

function sanitizeItem(item: SkyCalendarItem): SkyCalendarItem {
  const start = parseYmd(item.start);
  const end = parseYmd(item.end);
  return {
    ...item,
    status: calendarStatus(item),
    start: start?.iso || item.start,
    end: end?.iso || item.end
  };
}

function sanitizeFeed(feed: SkyCalendarFeed): SkyCalendarFeed {
  const currentSeason = feed.season ? sanitizeItem(feed.season) : null;
  const season = currentSeason?.status === 'ended' ? null : currentSeason;
  const events = (feed.events || []).map(sanitizeItem).filter(e => e.status !== 'ended');
  const cn = [...(feed.cn_now || []), ...(feed.cn_next || [])].map(sanitizeItem);
  const cn_now = cn.filter(it => it.status === 'live');
  const cn_next = cn.filter(it => it.status === 'upcoming' || it.status === 'inferred');
  const intl = [...(feed.intl_now || []), ...(feed.intl_next || [])].map(sanitizeItem).filter(it => it.names.length > 0);
  const intl_now = intl.filter(it => it.status === 'live');
  const intl_next = intl.filter(it => it.status === 'upcoming');
  return { ...feed, season, events, cn_now, cn_next, intl_now, intl_next };
}

function feedHasRows(feed: SkyCalendarFeed): boolean {
  return !!(
    feed.season ||
    feed.events.length ||
    feed.cn_now.length ||
    feed.cn_next.length ||
    feed.intl_now.length ||
    feed.intl_next.length
  );
}

export async function getSkyCalendarFeed(
  force = false,
  onUpdate?: (feed: SkyCalendarFeed) => void
): Promise<SkyCalendarFeed | null> {
  const feed = await cachedFetch<SkyCalendarFeed>({
    key: CACHE_KEY,
    label: '光遇档期',
    fetch: fetchSkyCalendar,
    fresh: (data, at) => data.as_of === todayStr() && Date.now() - at < 15 * 60_000,
    valid: feedHasRows,
    warn: data => (data.warnings || []).join('；'),
    onUpdate: next => onUpdate?.(sanitizeFeed(next)),
    force
  });
  return feed ? sanitizeFeed(feed) : null;
}

export function calendarRange(item: SkyCalendarItem): string {
  const start = parseYmd(item.start)?.iso || item.start;
  const end = parseYmd(item.end)?.iso || item.end;
  if (start && end) return `${start} ~ ${end}`;
  if (start) return `${start} 起`;
  return '';
}

export function calendarStatusLabel(item: SkyCalendarItem): string {
  const status = calendarStatus(item);
  if (status === 'live') return '进行中';
  if (status === 'upcoming') return '即将开始';
  if (status === 'inferred') return '窗口未公布名字';
  if (status === 'ended') return '已结束';
  return '';
}

export function calendarKindLabel(item: SkyCalendarItem): string {
  if (item.kind === 'season') return '季节';
  if (item.kind === 'event') return item.region === 'intl' ? '国际活动' : '活动';
  if (item.region === 'intl') {
    return item.title.includes('集体') ? '国际集体回归' : '国际回归';
  }
  return item.title.includes('集体') ? '国服集体回归' : '国服回归';
}

export function calendarPreviews(item: SkyCalendarItem, slot?: string) {
  return (item.previews || []).filter(p => (!slot || p.slot === slot) && (p.name || p.icon || p.wear));
}

export function calendarItemGroups(item: SkyCalendarItem) {
  const groups = new Map<string, ReturnType<typeof calendarPreviews>>();
  for (const p of calendarPreviews(item)) {
    if (p.slot === '先祖' || p.slot === '资讯') continue;
    const slot = p.slot || '物品';
    if (!groups.has(slot)) groups.set(slot, []);
    groups.get(slot)!.push(p);
  }
  return Array.from(groups, ([slot, items]) => ({ slot, items }));
}

export function calendarLookSummary(item: SkyCalendarItem): string {
  const parts: string[] = [];
  const spirits = calendarPreviews(item, '先祖').length;
  if (spirits && (spirits > 1 || item.kind === 'season')) parts.push(`先祖 ${spirits}`);
  parts.push(...calendarItemGroups(item).map(g => `${g.slot} ${g.items.length}`));
  return parts.join(' · ');
}

export function calendarBriefPreviews(item: SkyCalendarItem) {
  if (calendarLookSummary(item)) return [];
  return (item.previews || []).filter(p => p.slot === '资讯' && p.icon);
}

export function calendarRemain(item: SkyCalendarItem, today = todayStr()): string {
  const status = calendarStatus(item, today);
  if (status === 'inferred' || status === 'ended') return '';
  const now = dayNumber(today), start = dayNumber(item.start), end = dayNumber(item.end);
  if (now === null) return '';
  if (status === 'upcoming' && start !== null) {
    const days = start - now;
    return days === 1 ? '明天开始' : `${days} 天后开始`;
  }
  if (end === now) return '今天结束';
  if (start === now) return '今天开始';
  if (end === now + 1) return '明天结束';
  return end !== null ? `还剩 ${end - now} 天` : '';
}
