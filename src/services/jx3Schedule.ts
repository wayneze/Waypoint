import type {
  Jx3DungeonPendant,
  Jx3ReputationItem,
  Jx3AdventureItem,
  Jx3SetTokenSeries,
  Jx3WorldBossStoredWeek,
  Jx3WorldBossWeek,
  Jx3WorldBossDay,
  Jx3CatalogKind,
  Jx3CalendarSlotKey,
  Jx3MapRefreshInfo,
  Jx3Achievement,
  Jx3CollectItem,
  Jx3BoxDungeon
} from '../types';
import { kvGet, kvSet } from './kv';
import { JX3_WORLD_BOSS_SCHEDULE, compactQuery, scoreText } from './jx3CollectData';
import { JX3_COLLECT_KIND_LABEL, jx3BoxDungeonThumbUrl, jx3BoxItemIconUrl, resolveJx3BoxDungeon, resolveJx3CollectItemIconId, resolveJx3SeedItemIconId } from './jx3Collect';
import { jx3AchievementIconUrl, jx3CategoryImage, jx3ReputationIconUrl } from './jx3Artwork';

export function jx3WorldBossCadence(): string {
  const slots = JX3_WORLD_BOSS_SCHEDULE.slots;
  if (!slots.length) return '';
  const days = slots.map(s => s.weekdayLabel).join('、');
  const start = String(slots[0].startHour).padStart(2, '0');
  const end = String(slots[0].endHour).padStart(2, '0');
  return `每周${days} ${start}:00 - ${end}:00`;
}

function pad2(n: number) {
  return String(n).padStart(2, '0');
}

export function formatJx3Date(date: Date): string {
  return `${date.getFullYear()}-${pad2(date.getMonth() + 1)}-${pad2(date.getDate())}`;
}

export function startOfJx3Week(date: Date): Date {
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const day = d.getDay();
  d.setDate(d.getDate() + (day === 0 ? -6 : 1 - day));
  return d;
}

function dateOnWeekday(weekStart: Date, weekday: number): Date {
  const d = new Date(weekStart);
  d.setDate(d.getDate() + (weekday === 0 ? 6 : weekday - 1));
  return d;
}

function weekdayIndex(day: number) {
  return day === 0 ? 7 : day;
}

export function daysUntilJx3Weekday(from: Date, weekday: number): number | null {
  const delta = weekdayIndex(weekday) - weekdayIndex(from.getDay());
  return delta < 0 ? null : delta;
}

const STORAGE_WORLD_BOSS_WEEK_KEY = 'waypoint_jx3_world_boss_week_v1';

export function loadJx3WorldBossWeek(): Jx3WorldBossStoredWeek | null {
  try {
    const raw = kvGet(STORAGE_WORLD_BOSS_WEEK_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Jx3WorldBossStoredWeek;
    if (!parsed?.weekStart || !Array.isArray(parsed.days)) return null;
    return {
      weekStart: parsed.weekStart,
      days: parsed.days
        .filter(d => Number.isInteger(d.weekday) && Array.isArray(d.names))
        .map(d => ({
          weekday: d.weekday,
          dateStr: d.dateStr || '',
          names: d.names.map(n => String(n).trim()).filter(Boolean)
        }))
    };
  } catch (_) {
    return null;
  }
}

export function saveJx3WorldBossWeek(week: Jx3WorldBossStoredWeek) {
  try {
    kvSet(STORAGE_WORLD_BOSS_WEEK_KEY, JSON.stringify(week));
  } catch (_) {}
}

export function upsertJx3WorldBossDay(
  stored: Jx3WorldBossStoredWeek | null,
  weekday: number,
  dateStr: string,
  names: string[],
  weekStart: string
): Jx3WorldBossStoredWeek {
  const live = names.map(n => n.trim()).filter(Boolean);
  const base: Jx3WorldBossStoredWeek = stored?.weekStart === weekStart
    ? { weekStart, days: [...stored.days] }
    : { weekStart, days: [] };
  if (!live.length) return base;
  const row = { weekday, dateStr, names: live };
  const idx = base.days.findIndex(d => d.weekday === weekday);
  if (idx >= 0) base.days[idx] = row;
  else base.days.push(row);
  return base;
}

export function calculateJx3WorldBossWeek(
  date = new Date(),
  stored: Jx3WorldBossStoredWeek | null = null
): Jx3WorldBossWeek {
  const schedule = JX3_WORLD_BOSS_SCHEDULE;
  const weekStartDate = startOfJx3Week(date);
  const weekStart = formatJx3Date(weekStartDate);
  const nowDay = date.getDay();
  const nowMin = date.getHours() * 60 + date.getMinutes();
  const storedDays = stored?.weekStart === weekStart ? stored.days : [];
  const storedMap = new Map(storedDays.map(d => [d.weekday, d]));

  const days: Jx3WorldBossDay[] = schedule.slots.map(slot => {
    const slotDate = dateOnWeekday(weekStartDate, slot.weekday);
    const cached = storedMap.get(slot.weekday);
    const names = (cached?.names || []).map(n => n.trim()).filter(Boolean);
    const isToday = nowDay === slot.weekday;
    const startMin = slot.startHour * 60;
    const endMin = slot.endHour * 60;
    const inWindow = isToday && nowMin >= startMin && nowMin < endMin;
    const ended = isToday && nowMin >= endMin;
    const isPast = weekdayIndex(slot.weekday) < weekdayIndex(nowDay) || ended;
    const timeDesc = `${pad2(slot.startHour)}:00 - ${pad2(slot.endHour)}:00`;
    let statusText = `${slot.weekdayLabel} ${pad2(slot.startHour)}:00`;
    if (inWindow) statusText = `开战中 · ${pad2(slot.endHour)}:00 结束`;
    else if (ended) statusText = '今日已结束';
    else if (isToday) {
      const diff = startMin - nowMin;
      const h = Math.floor(diff / 60);
      const m = diff % 60;
      statusText = `今晚 ${pad2(slot.startHour)}:00（${h > 0 ? `${h}小时` : ''}${m}分钟后）`;
    } else if (isPast) statusText = '本周已过';
    return {
      weekday: slot.weekday,
      weekdayLabel: slot.weekdayLabel,
      dateStr: cached?.dateStr || formatJx3Date(slotDate),
      startHour: slot.startHour,
      endHour: slot.endHour,
      timeDesc,
      names,
      namesLine: names.length ? names.join('、') : (isPast ? '名单未缓存' : '名单待同步'),
      statusText,
      isToday,
      inWindow,
      ended,
      isPast
    };
  });

  const today = days.find(d => d.isToday);
  const upcoming = days.find(d => !d.isPast);
  let headerHint = upcoming
    ? (upcoming.isToday ? upcoming.statusText : `下次 ${upcoming.weekdayLabel} ${pad2(upcoming.startHour)}:00`)
    : '本周已结束';
  if (today?.inWindow) headerHint = today.statusText;

  return {
    weekStart,
    cadence: jx3WorldBossCadence(),
    headerHint,
    isBossDay: !!today,
    inWindow: !!today?.inWindow,
    days
  };
}

export interface Jx3TaskScheduleStatus {
  label: string;
  status: 'active' | 'waiting' | 'ended' | 'scheduled';
  statusText: string;
  badgeClass: string;
}

export function normalizeScheduleTimeString(raw: string): string {
  return (raw || '')
    .replace(/[:：]/g, ':')
    .replace(/[-~至到]/g, ' - ')
    .replace(/\s+/g, ' ')
    .trim();
}

export interface Jx3TimedBossEvent {
  id: string;
  name: string;
  aliases: string[];
  map: string;
  achievementName?: string;
  achievementId?: string;
  scheduleTime: string;
  desc: string;
  iconId?: number;
}

export const JX3_TIMED_BOSS_EVENTS: Jx3TimedBossEvent[] = [
  {
    id: 'boss_yuanfeiguizhu',
    name: '猿飞鬼助',
    aliases: ['金水猿飞鬼助', '猿飞', '金水首领', '金水世界首领', '金水小boss', '金水小BOSS', '急报！金水镇', '急报！猿飞鬼助'],
    map: '金水镇',
    achievementName: '急报！猿飞鬼助',
    achievementId: '1283',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '金水镇经典世界首领 · 周三周五 20:00 · 成就【急报！猿飞鬼助】',
    iconId: 763
  },
  {
    id: 'boss_wangmang',
    name: '汪莽',
    aliases: ['寇岛汪莽', '寇岛首领', '寇岛世界首领', '急报！寇岛', '急报！汪莽'],
    map: '寇岛',
    achievementName: '急报！汪莽',
    achievementId: '1284',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '寇岛经典世界首领 · 周三周五 20:00 · 成就【急报！汪莽】',
    iconId: 763
  },
  {
    id: 'boss_murongzhuifeng',
    name: '慕容追风',
    aliases: ['洛道慕容追风', '洛道首领', '洛道世界首领', '急报！洛道', '急报！慕容追风'],
    map: '洛道',
    achievementName: '急报！慕容追风',
    achievementId: '1285',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '洛道经典世界首领 · 周三周五 20:00 · 成就【急报！慕容追风】',
    iconId: 763
  },
  {
    id: 'boss_jiaoqing',
    name: '焦晴',
    aliases: ['枫华谷焦晴', '枫华谷首领', '枫华谷世界首领', '急报！枫华谷', '急报！焦晴'],
    map: '枫华谷',
    achievementName: '急报！焦晴',
    achievementId: '1286',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '枫华谷经典世界首领 · 周三周五 20:00 · 成就【急报！焦晴】',
    iconId: 763
  },
  {
    id: 'boss_zhuoyifei',
    name: '卓翼飞',
    aliases: ['巴陵卓翼飞', '巴陵县卓翼飞', '巴陵首领', '巴陵世界首领', '急报！巴陵县', '急报！卓翼飞'],
    map: '巴陵县',
    achievementName: '急报！卓翼飞',
    achievementId: '1287',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '巴陵县经典世界首领 · 周三周五 20:00 · 成就【急报！卓翼飞】',
    iconId: 763
  },
  {
    id: 'boss_sunfeiliang',
    name: '孙飞亮',
    aliases: ['龙门孙飞亮', '龙门荒漠孙飞亮', '龙门首领', '龙门世界首领', '急报！龙门荒漠', '急报！孙飞亮'],
    map: '龙门荒漠',
    achievementName: '急报！孙飞亮',
    achievementId: '1288',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '龙门荒漠经典世界首领 · 周三周五 20:00 · 成就【急报！孙飞亮】',
    iconId: 763
  },
  {
    id: 'boss_shentubuxiu',
    name: '申屠不休',
    aliases: ['申屠', '苍山洱海申屠不休', '苍山洱海首领', '苍山首领'],
    map: '苍山洱海',
    achievementName: '击败申屠不休',
    scheduleTime: '每日 12:00 - 13:00 / 18:00 - 19:00',
    desc: '苍山洱海无双首领 · 每日野外定时刷新',
    iconId: 763
  },
  {
    id: 'boss_censhang',
    name: '岑伤',
    aliases: ['世界首领岑伤', '跨服首领岑伤'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'boss_heichiyuanrou',
    name: '黑齿元佑',
    aliases: ['世界首领黑齿元佑', '黑齿元佑'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'boss_hanjingqing',
    name: '韩敬青',
    aliases: ['世界首领韩敬青', '韩敬青'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'boss_dantaimieming',
    name: '澹台灭明',
    aliases: ['世界首领澹台灭明', '澹台灭明'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'boss_senjiulan',
    name: '森九岚',
    aliases: ['世界首领森九岚', '森九岚'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'boss_ashinacongli',
    name: '阿史那从礼',
    aliases: ['世界首领阿史那从礼', '阿史那从礼'],
    map: '跨服世界首领',
    scheduleTime: '每周三、周五 20:00 - 21:00',
    desc: '跨服世界首领 · 周三周五 20:00 - 21:00',
    iconId: 763
  },
  {
    id: 'event_zhue_pifeng',
    name: '披风会诛恶',
    aliases: ['河西瀚漠诛恶', '河西诛恶', '披风会诛恶事件'],
    map: '河西瀚漠',
    get scheduleTime() { return `每日 ${jx3ZhueTimesText('披风会')}`; },
    desc: '披风会 · 河西瀚漠定时诛恶事件',
    iconId: 763
  },
  {
    id: 'event_zhue_yuncang',
    name: '云从社诛恶',
    aliases: ['银霜口诛恶', '云从社诛恶事件'],
    map: '银霜口',
    get scheduleTime() { return `每日 ${jx3ZhueTimesText('云从社')}`; },
    desc: '云从社 · 银霜口定时诛恶事件',
    iconId: 763
  },
  {
    id: 'event_zhue_chutian',
    name: '楚天社诛恶',
    aliases: ['楚州诛恶', '百溪诛恶', '烂柯山诛恶'],
    map: '楚天社',
    scheduleTime: '整点或半点轮巡',
    desc: '楚天社 · 楚州/百溪/烂柯山诛恶事件',
    iconId: 763
  }
];

export function findJx3TimedBossEvent(query: string): Jx3TimedBossEvent | undefined {
  const raw = query.trim().toLowerCase();
  const q = raw.replace(/[\s·・\-_/!！【】()（）]/g, '');
  if (!q || q.length < 2) return undefined;
  for (const b of JX3_TIMED_BOSS_EVENTS) {
    const cleanName = b.name.toLowerCase().replace(/[\s·・\-_/!！]/g, '');
    if (cleanName.includes(q) || q.includes(cleanName)) return b;
    for (const a of b.aliases) {
      const cleanA = a.toLowerCase().replace(/[\s·・\-_/!！]/g, '');
      if (cleanA.includes(q) || q.includes(cleanA)) return b;
    }
  }
  return undefined;
}

/**
 * 从待办标题中自动提取特定时间/时段
 * 例如: "金水猿飞鬼助 20:00" -> { title: "金水猿飞鬼助", scheduleTime: "20:00" }
 * 或直接输入已知首领: "猿飞鬼助" -> { title: "猿飞鬼助", scheduleTime: "每周三、周五 20:00 - 21:00", matchedBoss }
 */
export function extractJx3TaskScheduleTime(raw: string): { title: string; scheduleTime?: string; matchedBoss?: Jx3TimedBossEvent } {
  const title = (raw || '').trim();
  if (!title) return { title: '' };

  const regex = /(?:(?:每?周[一二三四五六日天](?:[、,\s/和与]+每?周[一二三四五六日天])*|周末|每天|工作日)\s*)?(?:\d{1,2}[:：]\d{2}(?:\s*[-~至到]\s*\d{1,2}[:：]\d{2})?)/;
  const match = regex.exec(title);
  if (match && match[0].length >= 4) {
    const timeStr = match[0].trim();
    const cleanedTitle = title.replace(match[0], '').replace(/\s+/g, ' ').trim();
    if (cleanedTitle.length >= 1) {
      const boss = findJx3TimedBossEvent(cleanedTitle);
      return {
        title: cleanedTitle,
        scheduleTime: normalizeScheduleTimeString(timeStr),
        matchedBoss: boss
      };
    }
  }

  // 若没有手写具体时间，检查是否命中已知定时首领/事件
  const boss = findJx3TimedBossEvent(title);
  if (boss) {
    return {
      title,
      scheduleTime: boss.scheduleTime,
      matchedBoss: boss
    };
  }

  return { title };
}

export function getJx3TaskScheduleStatus(scheduleTime?: string, now = new Date()): Jx3TaskScheduleStatus | null {
  if (!scheduleTime) return null;
  const raw = scheduleTime.trim();
  if (!raw) return null;

  const weekdays: number[] = [];
  const WEEKDAY_MAP: Record<string, number> = {
    周一: 1, 星期一: 1,
    周二: 2, 星期二: 2,
    周三: 3, 星期三: 3,
    周四: 4, 星期四: 4,
    周五: 5, 星期五: 5,
    周六: 6, 星期六: 6,
    周日: 0, 周天: 0, 星期日: 0, 星期天: 0
  };

  if (/周末/.test(raw)) {
    weekdays.push(6, 0);
  } else if (/工作日/.test(raw)) {
    weekdays.push(1, 2, 3, 4, 5);
  } else {
    for (const [key, val] of Object.entries(WEEKDAY_MAP)) {
      if (raw.includes(key) && !weekdays.includes(val)) {
        weekdays.push(val);
      }
    }
  }

  const timeMatch = /(\d{1,2})[:：](\d{2})(?:\s*[-~至到]\s*(\d{1,2})[:：](\d{2}))?/.exec(raw);
  if (!timeMatch) {
    return {
      label: raw,
      status: 'scheduled',
      statusText: raw,
      badgeClass: 'bg-raised text-muted border-hairline'
    };
  }

  const startHour = parseInt(timeMatch[1], 10);
  const startMinute = parseInt(timeMatch[2], 10);
  const endHour = timeMatch[3] ? parseInt(timeMatch[3], 10) : startHour + 1;
  const endMinute = timeMatch[4] ? parseInt(timeMatch[4], 10) : (timeMatch[3] ? 0 : startMinute);

  const startMin = startHour * 60 + startMinute;
  const endMin = (endHour * 60 + endMinute) || (startMin + 60);

  const curDay = now.getDay();
  const curMin = now.getHours() * 60 + now.getMinutes();
  const isToday = weekdays.length === 0 || weekdays.includes(curDay);

  const startPad = `${String(startHour).padStart(2, '0')}:${String(startMinute).padStart(2, '0')}`;
  const endPad = `${String(endHour).padStart(2, '0')}:${String(endMinute).padStart(2, '0')}`;
  const displayLabel = timeMatch[3] ? `${startPad} - ${endPad}` : startPad;

  if (isToday) {
    if (curMin >= startMin && curMin < endMin) {
      const left = endMin - curMin;
      return {
        label: displayLabel,
        status: 'active',
        statusText: `进行中 · 剩${left}分钟`,
        badgeClass: 'bg-done/15 text-done border-done/35 font-semibold'
      };
    }
    if (curMin < startMin) {
      const diff = startMin - curMin;
      const h = Math.floor(diff / 60);
      const m = diff % 60;
      const timeAgo = h > 0 ? `${h}小时${m > 0 ? `${m}分` : ''}` : `${m}分钟`;
      return {
        label: displayLabel,
        status: 'waiting',
        statusText: `${startPad}（${timeAgo}后）`,
        badgeClass: 'bg-beacon/15 text-beacon border-beacon/30'
      };
    }
    return {
      label: displayLabel,
      status: 'ended',
      statusText: '今日已结束',
      badgeClass: 'bg-raised text-faint border-hairline'
    };
  }

  let nextDiffDays = 7;
  for (const wd of weekdays) {
    const diff = (wd - curDay + 7) % 7 || 7;
    if (diff < nextDiffDays) nextDiffDays = diff;
  }
  const nextDate = new Date(now);
  nextDate.setDate(now.getDate() + nextDiffDays);
  const weekdayNames = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
  const nextLabel = `${weekdayNames[nextDate.getDay()]} ${startPad}`;

  return {
    label: displayLabel,
    status: 'scheduled',
    statusText: nextLabel,
    badgeClass: 'bg-raised text-muted border-hairline'
  };
}

export function isJx3ZhueEvent(ev: { stage?: string; desc?: string }): boolean {
  return (ev.stage || '').includes('诛恶') || (ev.desc || '').includes('诛恶');
}

export function filterJx3ZhueEvents<T extends { stage?: string; desc?: string }>(events: T[]): T[] {
  return events.filter(isJx3ZhueEvent);
}

export type Jx3ZhueSociety = '云从社' | '披风会' | '楚天社';

export const JX3_ZHUE_CLOCK: Record<Jx3ZhueSociety, { map: string; slots: { hour: number; minute: number }[] } | null> = {
  云从社: {
    map: '银霜口',
    slots: [
      { hour: 12, minute: 40 },
      { hour: 18, minute: 39 },
      { hour: 23, minute: 40 }
    ]
  },
  披风会: {
    map: '河西瀚漠',
    slots: [
      { hour: 12, minute: 25 },
      { hour: 18, minute: 24 },
      { hour: 23, minute: 25 }
    ]
  },
  楚天社: null
};

/** 诛恶时刻文案，从 JX3_ZHUE_CLOCK 生成，页面和目录都用这一份。 */
export function jx3ZhueTimesText(society: Jx3ZhueSociety): string {
  const clock = JX3_ZHUE_CLOCK[society];
  if (!clock?.slots.length) return '整点或半点轮巡';
  return clock.slots.map(s => `${String(s.hour).padStart(2, '0')}:${String(s.minute).padStart(2, '0')}`).join(' / ');
}

const ZHUE_APPROACH_MIN = 30;

const ZHUE_WINDOW_MIN = 30;

const MINUTES_PER_DAY = 24 * 60;

function parseJx3ClockMinutes(time: string): number | null {
  const m = /^(\d{1,2}):(\d{2})$/.exec(time.trim());
  if (!m) return null;
  const hour = Number(m[1]);
  const minute = Number(m[2]);
  if (hour > 23 || minute > 59) return null;
  return hour * 60 + minute;
}

function minutesUntilJx3Clock(nowMin: number, slotMin: number, nextDay: boolean): number {
  return nextDay ? MINUTES_PER_DAY - nowMin + slotMin : slotMin - nowMin;
}

function jx3SlotInWindow(nowMin: number, startMin: number, windowMin = ZHUE_WINDOW_MIN): boolean {
  const endMin = startMin + windowMin;
  if (endMin <= MINUTES_PER_DAY) return nowMin >= startMin && nowMin < endMin;
  return nowMin >= startMin || nowMin < endMin - MINUTES_PER_DAY;
}

function zhueClockFromSlots(map: string, slots: { hour: number; minute: number }[], date: Date): Jx3MapRefreshInfo | null {
  if (!slots.length) return null;
  const nowMin = date.getHours() * 60 + date.getMinutes();
  const live = slots.find(s => jx3SlotInWindow(nowMin, s.hour * 60 + s.minute));
  if (live) {
    const hhmm = `${pad2(live.hour)}:${pad2(live.minute)}`;
    return {
      map,
      nextText: `进行中 · ${hhmm}`,
      dueSoon: true,
      approaching: false,
      inWindow: true
    };
  }
  const today = slots.find(s => s.hour * 60 + s.minute > nowMin);
  const slot = today ?? slots[0];
  const nextDay = today == null;
  const diff = minutesUntilJx3Clock(nowMin, slot.hour * 60 + slot.minute, nextDay);
  const approaching = !nextDay && diff > 0 && diff <= ZHUE_APPROACH_MIN;
  const when = nextDay ? '明日' : '今日';
  return {
    map,
    nextText: `${when} ${pad2(slot.hour)}:${pad2(slot.minute)}（${diff} 分钟后）`,
    dueSoon: approaching,
    approaching,
    inWindow: false
  };
}

/** 云从社 / 披风会用每日三场。楚天社没有固定钟，只按接口诛恶时刻套框。临近 30 分钟日期框，开场后 30 分钟时间框。 */
export function calculateJx3ZhueClock(
  society: Jx3ZhueSociety,
  date = new Date(),
  events: { time?: string }[] = []
): Jx3MapRefreshInfo | null {
  const clock = JX3_ZHUE_CLOCK[society];
  if (clock?.slots.length) return zhueClockFromSlots(clock.map, clock.slots, date);
  const seen = new Set<number>();
  const slots: { hour: number; minute: number }[] = [];
  for (const ev of events) {
    const min = parseJx3ClockMinutes(ev.time || '');
    if (min == null || seen.has(min)) continue;
    seen.add(min);
    slots.push({ hour: Math.floor(min / 60), minute: min % 60 });
  }
  slots.sort((a, b) => a.hour * 60 + a.minute - (b.hour * 60 + b.minute));
  return zhueClockFromSlots('楚州', slots, date);
}

export const JX3_CALENDAR_SLOTS: Array<{
  key: Jx3CalendarSlotKey;
  mark: string;
  label: string;
  tag: string;
  category: 'daily' | 'weekly';
  defaultOn: boolean;
}> = [
  { key: 'war', mark: '今日大战', label: '今日大战', tag: 'JX3API自动同步', category: 'daily', defaultOn: true },
  { key: 'mine', mark: '今日矿车', label: '今日矿车', tag: 'JX3API自动同步', category: 'daily', defaultOn: true },
  { key: 'school', mark: '门派日常', label: '门派日常', tag: 'JX3API自动同步', category: 'daily', defaultOn: true },
  { key: 'battle', mark: '今日战场', label: '今日战场', tag: 'JX3API自动同步', category: 'daily', defaultOn: false },
  { key: 'rescue', mark: '今日驰援', label: '今日驰援', tag: 'JX3API自动同步', category: 'daily', defaultOn: false },
  { key: 'card', mark: '秘境日常', label: '秘境日常', tag: 'JX3API自动同步', category: 'daily', defaultOn: false },
  { key: 'leader', mark: '世界首领', label: '世界首领', tag: 'JX3API自动同步', category: 'weekly', defaultOn: false },
  { key: 'raid', mark: '本周团本', label: '本周团本', tag: 'JX3API自动同步', category: 'weekly', defaultOn: false },
  { key: 'conn', mark: '本周公共', label: '本周公共', tag: 'JX3API自动同步', category: 'weekly', defaultOn: false }
];

export interface Jx3CatalogHit {
  kind: Jx3CatalogKind;
  id: string;
  title: string;
  hint: string;
  tag: string;
  category: 'daily' | 'weekly';
  score: number;
  iconId?: number;
  dungeonIcon?: string;
  schedule_time?: string;
}

export function searchJx3Catalog(
  query: string,
  catalogs: {
    pendants: Jx3DungeonPendant[];
    reputations: Jx3ReputationItem[];
    adventures: Jx3AdventureItem[];
    tokens: Jx3SetTokenSeries[];
    achievements: Jx3Achievement[];
    collectItems?: Jx3CollectItem[];
    dungeons?: Jx3BoxDungeon[];
  }
): Jx3CatalogHit[] {
  const q = query.trim();
  if (q.length < 1) return [];
  const compact = compactQuery(q);
  const low = q.toLowerCase();
  const hits: Jx3CatalogHit[] = [];
  const dens = catalogs.dungeons || [];
  const dungeonIconOf = (label?: string) => {
    if (!label) return undefined;
    return resolveJx3BoxDungeon(label, dens)?.icon;
  };

  for (const b of JX3_TIMED_BOSS_EVENTS) {
    const score = scoreText(low, compact, [b.name, b.map, b.desc, ...(b.aliases || []), b.achievementName || '']);
    if (score) {
      hits.push({
        kind: 'boss',
        id: b.id,
        title: b.name,
        hint: b.desc,
        tag: '首领',
        category: b.scheduleTime.includes('周') ? 'weekly' : 'daily',
        score: score + 25,
        schedule_time: b.scheduleTime,
        iconId: b.iconId || 763
      });
      if (b.achievementName) {
        hits.push({
          kind: 'achievement',
          id: b.achievementId || b.id,
          title: b.achievementName,
          hint: `${b.map} · 击败${b.name}`,
          tag: '成就',
          category: b.scheduleTime.includes('周') ? 'weekly' : 'daily',
          score: score + 20,
          schedule_time: b.scheduleTime,
          iconId: b.iconId || 763
        });
      }
    }
  }

  for (const p of catalogs.pendants) {
    const score = scoreText(low, compact, [p.name, p.dungeon, p.boss, p.type, p.desc || '']);
    if (score) {
      hits.push({
        kind: 'pendant',
        id: p.id,
        title: p.name,
        hint: `${p.type} · ${p.dungeon}`,
        tag: '挂件',
        category: p.cdRule.includes('周一') ? 'weekly' : 'daily',
        score,
        iconId: resolveJx3SeedItemIconId(p.id) || resolveJx3SeedItemIconId(p.name)
      });
    }
  }
  for (const r of catalogs.reputations) {
    const score = scoreText(low, compact, [r.name, r.group, r.dungeon || '', r.recommendedItem, r.rewardHighlight]);
    if (score) {
      hits.push({
        kind: 'reputation',
        id: r.id,
        title: r.name,
        hint: `${r.group}${r.dungeon ? ` · ${r.dungeon}` : ''}`,
        tag: '声望',
        category: 'daily',
        score,
        dungeonIcon: dungeonIconOf(r.dungeon)
      });
    }
  }
  for (const a of catalogs.adventures) {
    const reqNames = a.requirements.map(req => req.name).join(' ');
    const score = scoreText(low, compact, [a.name, a.type, a.reward, a.triggerDaily, reqNames]);
    if (score) {
      hits.push({
        kind: 'adventure',
        id: a.id,
        title: a.name,
        hint: a.reward ? `${a.type} · ${a.reward}` : a.type,
        tag: '奇遇',
        category: 'daily',
        score
      });
    }
  }
  for (const t of catalogs.tokens) {
    const pieceNames = Object.values(t.pieces).map(p => `${p.name} ${p.boss}`).join(' ');
    const score = scoreText(low, compact, [t.seriesName, t.pinyin, t.dungeon, t.level, pieceNames]);
    if (score) {
      hits.push({
        kind: 'token',
        id: t.id,
        title: t.seriesName,
        hint: `${t.level} · ${t.dungeon}`,
        tag: '牌子',
        category: 'weekly',
        score
      });
    }
  }
  for (const ach of catalogs.achievements) {
    const score = scoreText(low, compact, [ach.name, ach.sub_name, ach.desc, ach.id]);
    if (score) {
      hits.push({
        kind: 'achievement',
        id: ach.id,
        title: ach.name,
        hint: `${ach.sub_name} · ${ach.point}资历`,
        tag: '成就',
        category: 'daily',
        score
      });
    }
  }
  for (const it of catalogs.collectItems || []) {
    if (it.kind === 'pendant' || it.kind === 'token') continue;
    const score = scoreText(low, compact, [it.name, it.dungeonName, it.note || '', it.boss || '']);
    if (score) {
      hits.push({
        kind: it.kind,
        id: it.id,
        title: it.name,
        hint: `${JX3_COLLECT_KIND_LABEL[it.kind]} · ${it.dungeonName}`,
        tag: JX3_COLLECT_KIND_LABEL[it.kind],
        category: 'weekly',
        score,
        iconId: resolveJx3CollectItemIconId(it)
      });
    }
  }

  hits.sort((a, b) => b.score - a.score || a.title.localeCompare(b.title, 'zh'));
  return hits.slice(0, 8);
}

export function jx3CatalogHitIconUrl(hit: Pick<Jx3CatalogHit, 'iconId' | 'kind' | 'dungeonIcon'>): string {
  return jx3BoxItemIconUrl(hit.iconId) || jx3BoxDungeonThumbUrl(hit.dungeonIcon) || jx3CatalogCategoryImage(hit.kind);
}

function jx3CatalogCategoryImage(kind?: Jx3CatalogKind): string {
  const category = ({ pendant: '装备', special: '杂闻', appearance: '装备', token: '装备', reputation: '声望', adventure: '杂闻', boss: '战斗', achievement: '剑侠录' } as Record<string, string>)[kind || ''] || '任务';
  return jx3CategoryImage(category);
}

export function jx3CalendarCategoryImage(key: Jx3CalendarSlotKey): string {
  const category = { war: '秘境', mine: '阵营', school: '武学', battle: '战斗', rescue: '阵营', card: '秘境', leader: '战斗', raid: '秘境', conn: '活动' }[key];
  return jx3CategoryImage(category);
}
export function calendarPayloadFromTitle(title: string): string {
  return String(title || '')
    .replace(/^【[^】]+】/, '')
    .replace(/\s*[（(][^）)]+[）)]\s*$/, '')
    .trim();
}

export function resolveJx3TaskVisual(
  task: {
    catalog_kind?: Jx3CatalogKind;
    catalog_id?: string;
    api_key?: string;
    title?: string;
  },
  catalogs: {
    collectItems?: Jx3CollectItem[];
    pendants?: Jx3DungeonPendant[];
    tokens?: Jx3SetTokenSeries[];
    reputations?: Jx3ReputationItem[];
    dungeons?: Jx3BoxDungeon[];
  } = {}
): { iconUrl: string; sceneUrl?: string } {
  const dens = catalogs.dungeons || [];
  const items = catalogs.collectItems || [];
  const thumbOf = (label?: string) => {
    if (!label) return '';
    return jx3BoxDungeonThumbUrl(resolveJx3BoxDungeon(label, dens)?.icon);
  };

  if (task.catalog_kind && task.catalog_id) {
    if (task.catalog_kind === 'pendant') {
      const it = items.find(x => x.kind === 'pendant' && (x.catalogId === task.catalog_id || x.id === task.catalog_id));
      const iconId = resolveJx3CollectItemIconId(it) || resolveJx3SeedItemIconId(task.catalog_id);
      return { iconUrl: jx3BoxItemIconUrl(iconId) || jx3CatalogCategoryImage(task.catalog_kind) };
    }
    if (task.catalog_kind === 'special' || task.catalog_kind === 'appearance') {
      const it = items.find(x => x.id === task.catalog_id);
      return { iconUrl: jx3BoxItemIconUrl(resolveJx3CollectItemIconId(it)) || jx3CatalogCategoryImage(task.catalog_kind) };
    }
    if (task.catalog_kind === 'token') {
      const it = items.find(x => x.kind === 'token' && (x.tokenSeriesId === task.catalog_id || x.id === task.catalog_id));
      return { iconUrl: jx3BoxItemIconUrl(resolveJx3CollectItemIconId(it)) || jx3CatalogCategoryImage(task.catalog_kind) };
    }
    if (task.catalog_kind === 'reputation') {
      const r = (catalogs.reputations || []).find(x => x.id === task.catalog_id);
      return { iconUrl: jx3ReputationIconUrl(r) || jx3CatalogCategoryImage('reputation') };
    }
    if (task.catalog_kind === 'boss' || task.catalog_kind === 'achievement') {
      return { iconUrl: jx3AchievementIconUrl(task.catalog_id) || jx3CatalogCategoryImage(task.catalog_kind) };
    }
    return { iconUrl: jx3CatalogCategoryImage(task.catalog_kind) };
  }

  const sceneUrl = thumbOf(calendarPayloadFromTitle(task.title || ''));
  if (task.api_key === 'war' || /【今日大战】/.test(task.title || '')) {
    return { iconUrl: jx3CalendarCategoryImage('war'), sceneUrl };
  }
  const slot = JX3_CALENDAR_SLOTS.find(slot => task.api_key === slot.key || task.title?.includes(`【${slot.mark}】`));
  return { iconUrl: slot ? jx3CalendarCategoryImage(slot.key) : jx3CategoryImage(sceneUrl ? '秘境' : '任务'), sceneUrl };
}
