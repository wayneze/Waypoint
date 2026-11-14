import { fetchSkyEventEdition, type SkyCalendarFeed, type SkyCalendarItem, type SkyEventEdition } from './skyApi';
import { eventName } from './skyEventService';
import { cachedFetch } from './fetchCache';

export interface SkyEventForecast {
  id: string;
  title: string;
  region: string;
  year: number;
  window: string;
  history: SkyEventEdition[];
}

/** 只给活动浏览页使用真实往届身份，预测不会写入正式档期。 */
export function forecastReferenceItem(forecast: SkyEventForecast): SkyCalendarItem {
  const edition = forecast.history[0];
  return { ...edition, id: `forecast/${forecast.id}`, kind: 'event', names: [], season: '',
    status: 'ended', note: '', previews: [] };
}

export async function getForecastContent(forecast: SkyEventForecast, force = false, onUpdate?: (item: SkyCalendarItem) => void) {
  const edition = forecast.history[0];
  return cachedFetch<SkyCalendarItem>({
    key: `sky.event_edition/${edition.region}/${edition.start}/${edition.url}`,
    label: `${edition.title}往届内容`,
    fetch: () => fetchSkyEventEdition(edition),
    fresh: (_, at) => Date.now() - at < 24 * 3600_000,
    valid: item => item.region === edition.region && item.start === edition.start && item.end === edition.end,
    force, onUpdate,
  });
}

const DAY = 86_400_000;
function date(value: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const result = new Date(`${value}T00:00:00Z`);
  return Number.isFinite(+result) && result.toISOString().slice(0, 10) === value ? result : null;
}
function periodLabel(value: Date): string {
  return `${value.getUTCMonth() + 1} 月${value.getUTCDate() <= 10 ? '上旬' : value.getUTCDate() <= 20 ? '中旬' : '下旬'}`;
}

/** 历史样本只生成独立预估，不进入正式档期、倒计时或每日待办。 */
export function forecastSkyEvents(feed: SkyCalendarFeed | null, today: string): SkyEventForecast[] {
  const now = date(today);
  if (!feed || !now) return [];
  const groups = new Map<string, SkyEventEdition[]>();
  for (const row of feed.event_history || []) {
    const start = date(row.start), end = date(row.end);
    if (!start || !end || end < start || !row.url || !['cn', 'intl'].includes(row.region)) continue;
    const key = `${row.region}/${eventName(row.title)}`;
    const group = groups.get(key) || [];
    if (!group.some(e => e.start === row.start)) group.push(row);
    groups.set(key, group);
  }
  const candidates: (SkyEventForecast & { order: number })[] = [];
  for (const [key, rows] of groups) {
    rows.sort((a, b) => b.start.localeCompare(a.start));
    for (const year of [now.getUTCFullYear(), now.getUTCFullYear() + 1]) {
      // 本届已有历史或公告就移除预估；换年时也不让停办多年活动重新出现。
      if (rows.some(e => Number(e.start.slice(0, 4)) === year)) continue;
      const first = rows[0];
      if (feed.events.some(e => `${e.region}/${eventName(e.title)}` === key && Number(e.start.slice(0, 4)) === year)) continue;
      const history: SkyEventEdition[] = [];
      for (let y = year - 1; y >= year - 3; y--) {
        const editions = rows.filter(e => Number(e.start.slice(0, 4)) === y);
        if (editions.length !== 1) break;
        history.push(editions[0]);
      }
      if (history.length < 2) continue;
      const projected = history.map(e => date(`${year}${e.start.slice(4)}`));
      if (projected.some(d => !d)) continue;
      const times = (projected as Date[]).map(d => +d);
      const earliest = Math.min(...times), latest = Math.max(...times);
      // 超过六周的波动不适合按公历给出季节窗口；春节等只提供实际样本覆盖的宽区间。
      if (latest - earliest > 42 * DAY) continue;
      const from = new Date(earliest), until = new Date(latest);
      from.setUTCDate(from.getUTCDate() <= 10 ? 1 : from.getUTCDate() <= 20 ? 11 : 21);
      if (until.getUTCDate() <= 10) until.setUTCDate(10);
      else if (until.getUTCDate() <= 20) until.setUTCDate(20);
      else until.setUTCMonth(until.getUTCMonth() + 1, 0);
      if (+until < +now || +from > +now + 90 * DAY) continue;
      const firstLabel = periodLabel(from), lastLabel = periodLabel(until);
      candidates.push({
        id: `${key}/${year}`, title: first.title, region: first.region, year,
        window: firstLabel === lastLabel ? firstLabel : `${firstLabel}—${lastLabel}`,
        history, order: +from,
      });
    }
  }
  return candidates.sort((a, b) => a.order - b.order || a.id.localeCompare(b.id));
}
