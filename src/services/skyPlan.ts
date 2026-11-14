import type { SeasonalGuide } from '../types';
import type { SkyCalendarFeed, SkyCalendarItem } from './skyApi';
import { calendarStatus } from './skyCalendarService';
import type { SkyDailyGuidePayload, SkyEventDaily } from './skyDailyService';
import { dailyChecked, progressGet } from './progress';
import { eventDailyInstructions, eventName, matchEventGuide } from './skyEventService';

export interface SkyTodoItem {
  id: string;
  title: string;
  keys: string[];
  done: boolean;
  detail?: string;
}
export interface SkyTodoGroup {
  id: string;
  title: string;
  hint: string;
  scope: string;
  items: SkyTodoItem[];
  image?: string;
  done: boolean;
  ready: boolean;
  day?: string;
  event?: SkyCalendarItem;
}

/** 档期是提醒的依据；只有明确的日常说明或用户选择才成为每日打卡项。 */
export function currentSkyPeriods(feed: SkyCalendarFeed | null, day: string): SkyCalendarItem[] {
  if (!feed) return [];
  const rows = [...feed.events, ...feed.cn_now, ...feed.cn_next];
  const seen = new Set<string>();
  return rows.filter(row => {
    if (row.region === 'intl' || calendarStatus(row, day) !== 'live') return false;
    const key = periodKey(row);
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  }).sort((a, b) => (a.end || '9999').localeCompare(b.end || '9999'));
}

export function periodKey(row: SkyCalendarItem): string {
  return `${row.kind}/${row.title}/${row.start}`;
}

export function skyActivityDailies(payload: SkyDailyGuidePayload | null, feed: SkyCalendarFeed | null, day: string, characterId: string, catalog: SeasonalGuide[] = []): SkyEventDaily[] {
  const guide = payload?.dateStr === day ? payload.eventDaily : undefined;
  const events: SkyEventDaily[] = [];
  const currentEvents = currentSkyPeriods(feed, day).filter(r => r.kind === 'event');
  let guideUsed = false;
  for (const row of currentEvents) {
    // 名称精确对应才附上攻略，避免把同期开启的活动混为一项。
    const matched = !!guide && eventName(guide.name) === eventName(row.title);
    const instructions = eventDailyInstructions(matchEventGuide(row, catalog));
    const selected = progressGet<boolean>('sky.event_daily_enabled', periodKey(row), characterId);
    const hasDaily = !!instructions || /每日|每天|日常/.test(row.note);
    if (selected === false || (!matched && !hasDaily && selected !== true)) continue;
    if (matched) guideUsed = true;
    events.push({
      ...(matched ? guide! : { name: row.title, how: instructions, imageUrl: '', completed: false, guidePending: !instructions }),
      how: matched ? guide!.how || instructions : instructions,
      id: row.title,
      window: `${row.start} ~ ${row.end}`,
    });
  }
  if (guide && !guideUsed) {
    const row = currentEvents.find(r => eventName(r.title) === eventName(guide.name));
    if (!row || progressGet('sky.event_daily_enabled', periodKey(row), characterId) !== false) events.push({ ...guide });
  }
  const checked = dailyChecked('sky.event_daily', day, characterId);
  return events.map(event => ({ ...event, completed: checked.has(event.id || event.name) }));
}

/** 首页和光遇页的唯一日常投影，不生成第二份普通任务。 */
export function skyTodoGroups(payload: SkyDailyGuidePayload | null, feed: SkyCalendarFeed | null, day: string, characterId: string, catalog: SeasonalGuide[] = []): SkyTodoGroup[] {
  const data = payload?.dateStr === day ? payload : null;
  const quests = dailyChecked('sky.quest', day, characterId);
  const candles = dailyChecked('sky.season_candle', day, characterId);
  const questItems = (data?.quests || []).map(q => ({
    id: q.id, title: q.title, keys: [q.id, q.title], done: quests.has(q.id) || quests.has(q.title), detail: [q.realm, q.tip].filter(Boolean).join(' · '),
  }));
  const candleItems = (data?.seasonalCandles.spots || []).map(s => ({
    id: String(s.index), title: s.label, keys: [String(s.index)], done: candles.has(String(s.index)), detail: s.desc,
  }));
  const group = (id: string, title: string, hint: string, scope: string, items: SkyTodoItem[], image?: string): SkyTodoGroup => ({
    id, title, hint, scope, items, image, ready: items.length > 0,
    done: items.length > 0 && items.every(i => i.done),
  });
  return [
    group('quests', '每日四任务', questItems.length ? `${questItems.filter(i => i.done).length} / ${questItems.length}` : '任务待更新', 'sky.quest', questItems, data?.overviewImages.questsImg),
    group('candles', '季节蜡烛', candleItems.length ? `${data?.seasonalCandles.realm} · ${candleItems.filter(i => i.done).length} / ${candleItems.length}` : '点位待更新', 'sky.season_candle', candleItems, data?.overviewImages.seasonCandleImg),
    ...skyActivityDailies(data, feed, day, characterId, catalog).map(e => ({
      ...group(
        `event/${e.id || e.name}`, e.name, e.guidePending ? '攻略待更新 · 可手动打卡' : '活动日常',
        'sky.event_daily', [{ id: e.id || e.name, title: e.how || '完成今日活动日常', keys: [e.id || e.name], done: e.completed, detail: e.window }], e.imageUrl,
      ),
      event: currentSkyPeriods(feed, day).find(row => row.kind === 'event' && eventName(row.title) === eventName(e.name)),
    })),
  ];
}

export interface SkyPeriodGoal {
  id: string;
  title: string;
  scope: string;
  done: boolean;
  guideId?: string;
  itemId?: string;
}

/** 解锁与永久翼是角色永久记录，活动一次性任务按本届档期保存。 */
export function skyPeriodGoals(row: SkyCalendarItem, characterId: string, seasons: SeasonalGuide[], events: SeasonalGuide[]): SkyPeriodGoal[] {
  const goals: SkyPeriodGoal[] = [];
  if (row.kind === 'reprint') {
    for (const name of row.names) {
      for (const [suffix, label] of [['unlock', '解锁先祖'], ['wing', '兑换永久翼']]) {
        const id = `${name}/${suffix}`;
        goals.push({ id, title: `${name} · ${label}`, scope: 'sky.returning_spirit', done: progressGet('sky.returning_spirit', id, characterId) === true });
      }
    }
  } else if (row.kind === 'event') {
    const id = periodKey(row);
    goals.push({ id, title: '本期一次性奖励已确认', scope: 'sky.event_once', done: progressGet('sky.event_once', id, characterId) === true });
  }
  const eventGuide = matchEventGuide(row, events);
  const guides = row.kind === 'reprint' ? seasons : eventGuide ? [eventGuide] : [];
  for (const guide of guides) {
    for (const item of guide.itemList || []) {
      if (row.kind === 'reprint' && (!item.spirit || !row.names.includes(item.spirit))) continue;
      const scope = row.kind === 'reprint' ? 'sky.season_wish' : 'sky.event_wish';
      const record = progressGet<{ items?: Record<string, boolean> }>(scope, guide.id, characterId);
      goals.push({ id: `${guide.id}/${item.id}`, title: item.name, scope, guideId: guide.id, itemId: item.id, done: record?.items?.[item.id] === true });
    }
  }
  return goals;
}
