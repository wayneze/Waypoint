// 日常 · 周常总览。
import { computed } from 'vue';
import { type Character, type CustomMemo, type Jx3MapRefreshInfo, type TaskItem } from '../types';
import { JX3_CALENDAR_SLOTS } from '../services/jx3Schedule';
import { type GameStamp, gameMetaOf, isGameEnabled, TOGGLEABLE_GAMES } from '../services/games';
import { allAccountTasks, characterList, gameIdOf, gameResetEpoch, gameStampOf, getCharBoardTitle } from './core';
import { skyGroupsFor, skyWeeklyGroup } from './sky/plan';
import type { SkyTodoGroup } from '../services/skyPlan';
import { cacheGet, sourceStatus } from '../services/fetchCache';
import type { SkyCalendarFeed, SkyCalendarItem } from '../services/skyApi';
import {
  chutianZhueClock,
  findJx3CalendarTask,
  jx3MapRefreshTick,
  jx3WorldBoss,
  pifengZhueClock,
  yuncangZhueClock,
} from './jx3/calendar';
import { memoVisibleOnBoard } from '../services/memoBoard';
import { lastWeeklyBoundary, parseResetClock } from '../services/cycleReset';
import { customMemos, gameIdOfMemoTag, getMemoRemainingText, memoStampOf } from './memos';

export type HomeLine =
  | {
      kind: 'sky';
      id: string;
      title: string;
      stamp: GameStamp;
      identity: string;
      hint: string;
      charId: string;
      group: SkyTodoGroup;
    }
  | {
      kind: 'task';
      id: string;
      title: string;
      stamp: GameStamp;
      identity: string;
      hint: string;
      task: TaskItem;
      charId: string;
    }
  | {
      kind: 'memo';
      id: string;
      title: string;
      stamp: GameStamp;
      identity: string;
      hint: string;
      memo: CustomMemo;
    };

function homeTaskLine(char: Character, task: TaskItem): Extract<HomeLine, { kind: 'task' }> {
  return {
    kind: 'task',
    id: task.id,
    title: task.title,
    stamp: gameStampOf(char.id),
    identity: getCharBoardTitle(char),
    hint: task.tag || '',
    task,
    charId: char.id,
  };
}

function homeMemoLine(memo: CustomMemo): Extract<HomeLine, { kind: 'memo' }> {
  const owner = memo.character_id
    ? characterList.value.find(char => char.id === memo.character_id)
    : undefined;
  return {
    kind: 'memo',
    id: memo.id,
    title: memo.title,
    stamp: owner ? gameStampOf(owner.id) : memoStampOf(memo.tag),
    identity: owner ? getCharBoardTitle(owner) : memo.tag,
    hint: getMemoRemainingText(memo).text,
    memo,
  };
}

function homeGameIdOfLine(line: HomeLine): string {
  if (line.kind === 'task' || line.kind === 'sky') return gameIdOf(line.charId) || 'other';
  if (line.memo.character_id) return gameIdOf(line.memo.character_id) || gameIdOfMemoTag(line.memo.tag) || 'other';
  return gameIdOfMemoTag(line.memo.tag) || 'other';
}

function lineGameVisible(line: HomeLine): boolean {
  const gameId = homeGameIdOfLine(line);
  // 设置里没有开关的游戏（逆水寒、生活、通用）不能被当成已关闭。
  if (!TOGGLEABLE_GAMES.some(game => game.id === gameId)) return true;
  return isGameEnabled(gameId);
}

function taskLinesOf(category: 'daily' | 'weekly', scope: string): Extract<HomeLine, { kind: 'task' }>[] {
  const lines: Extract<HomeLine, { kind: 'task' }>[] = [];
  for (const char of characterList.value) {
    if (scope !== 'all' && char.id !== scope) continue;
    for (const task of allAccountTasks.value[char.id] || []) {
      if (task.category === category) {
        lines.push(homeTaskLine(char, task));
      }
    }
  }
  return lines;
}

export type HomePeriod = 'daily' | 'weekly' | 'memo';

export function homeLineDone(line: HomeLine): boolean {
  return line.kind === 'sky' ? line.group.done : line.kind === 'task' ? line.task.done : line.memo.done;
}

function homeLinesFor(scope: string, period: HomePeriod): HomeLine[] {
  const memos = customMemos.value
    .filter(memo => memoVisibleOnBoard(memo, scope, period === 'memo' ? 'other' : period, true))
    .map(homeMemoLine);
  return [
    ...(period === 'memo' ? [] : skyHomeLines(period, scope === 'all' ? undefined : scope)),
    ...(period === 'memo' ? [] : taskLinesOf(period, scope)),
    ...memos,
  ].filter(lineGameVisible);
}

function skyHomeLines(category: 'daily' | 'weekly', charId?: string): Extract<HomeLine, { kind: 'sky' }>[] {
  return characterList.value.filter(c => c.game_id === 'sky' && (!charId || c.id === charId)).flatMap(char =>
    (category === 'daily' ? skyGroupsFor(char.id) : [skyWeeklyGroup(char.id)]).map(group => ({
      kind: 'sky' as const, id: `sky/${char.id}/${group.id}`, title: group.title, stamp: 'sky' as const,
      identity: getCharBoardTitle(char), hint: group.hint, charId: char.id, group,
    })));
}

export function homeCharStats(charId: string) {
  const daily = homeLinesFor(charId, 'daily');
  const weekly = homeLinesFor(charId, 'weekly');
  const dailyDone = daily.filter(homeLineDone).length;
  return {
    dailyTotal: daily.length, dailyDone, dailyRemaining: daily.length - dailyDone,
    dailyPercent: daily.length ? Math.round(dailyDone / daily.length * 100) : 0,
    weeklyTotal: weekly.length, weeklyDone: weekly.filter(homeLineDone).length,
  };
}
export const homeMatrixStats = computed(() => {
  const daily = homeLinesFor('all', 'daily');
  const totalDaily = daily.length;
  const doneDaily = daily.filter(homeLineDone).length;
  return { totalDaily, doneDaily, remainingDaily: totalDaily - doneDaily, dailyPercent: totalDaily ? Math.round(doneDaily / totalDaily * 100) : 0 };
});

export const hasJx3Characters = computed(() =>
  isGameEnabled('jx3') && characterList.value.some(c => c.game_id === 'jx3')
);

export type HomeNowItem = {
  id: string;
  title: string;
  hint: string;
  tone: 'alert' | 'beacon';
};

function homeBossNames(day?: { namesLine?: string }): string {
  const line = (day?.namesLine || '').trim();
  if (!line || line === '名单待同步' || line === '名单未缓存') return '';
  return line;
}

export const homeNowItems = computed<HomeNowItem[]>(() => {
  if (!hasJx3Characters.value) return [];
  jx3MapRefreshTick.value;
  const items: HomeNowItem[] = [];
  const week = jx3WorldBoss.value;
  const liveBoss = week.days.find(d => d.inWindow);
  const todayBoss = week.days.find(d => d.isToday && !d.ended);
  const nextBoss = week.days.find(d => !d.isPast && !d.ended);
  const boss = liveBoss || todayBoss || nextBoss;
  if (boss) {
    const names = homeBossNames(boss);
    items.push({
      id: 'boss',
      title: names ? `世界首领 · ${names}` : '世界首领',
      hint: boss.statusText,
      tone: boss.inWindow ? 'alert' : 'beacon'
    });
  } else if (week.days.length) {
    const first = week.days[0];
    items.push({
      id: 'boss',
      title: '世界首领',
      hint: `下次 ${first.weekdayLabel} ${String(first.startHour).padStart(2, '0')}:00`,
      tone: 'beacon'
    });
  }
  const clocks: Array<{ id: string; title: string; clock: Jx3MapRefreshInfo | null }> = [
    { id: 'yun', title: '银霜口 · 诛恶', clock: yuncangZhueClock.value },
    { id: 'pi', title: '河西瀚漠 · 诛恶', clock: pifengZhueClock.value },
    { id: 'chu', title: '楚天社 · 诛恶', clock: chutianZhueClock.value }
  ];
  const liveZhue = clocks.filter(row => row.clock?.inWindow);
  const nearZhue = clocks.filter(row => row.clock && !row.clock.inWindow && row.clock.approaching);
  const nextZhue = clocks
    .filter(row => row.clock && !row.clock.inWindow)
    .sort((a, b) => (a.clock?.nextText || '').localeCompare(b.clock?.nextText || '', 'zh'));
  const zhue = liveZhue.length ? liveZhue : (nearZhue.length ? nearZhue : nextZhue.slice(0, 1));
  for (const row of zhue) {
    if (!row.clock) continue;
    items.push({
      id: row.id,
      title: row.title,
      hint: row.clock.nextText,
      tone: row.clock.inWindow ? 'alert' : 'beacon'
    });
  }
  return items;
});

// 档期卡片和页签提醒共用同一份临期列表，避免两处的筛选、去重和日期不同步。
export const homeEndingItems = computed(() => {
  gameResetEpoch.value;
  jx3MapRefreshTick.value;
  void sourceStatus['sky.calendar']?.fetchedAt;
  if (!isGameEnabled('sky')) return [];
  const feed = cacheGet<SkyCalendarFeed>('sky.calendar')?.data;
  if (!feed) return [];
  const all: SkyCalendarItem[] = [...(feed.season ? [feed.season] : []), ...(feed.events || []), ...(feed.cn_now || [])];
  const seen = new Set<string>();
  const now = Date.now();
  return all
    .filter(it => it.status === 'live' && it.region !== 'intl' && it.end)
    .map(it => {
      const match = it.end!.match(/^(\d{4})-(\d{1,2})-(\d{1,2})/);
      const end = match ? new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]), 23, 59, 59).getTime() : NaN;
      return { it, left: Math.floor((end - now) / 86400000) };
    })
    .filter(({ left }) => left >= 0 && left <= 7)
    .filter(({ it }) => seen.has(it.id) ? false : (seen.add(it.id), true))
    .sort((a, b) => a.left - b.left);
});

export const homeBriefingNotice = computed(() => {
  const endingToday = homeEndingItems.value.filter(({ left }) => left === 0).length;
  if (endingToday) return `今日结束 ${endingToday}`;
  const active = homeNowItems.value.filter(item => item.tone === 'alert').length;
  if (active) return `进行中 ${active}`;
  const ending = homeEndingItems.value.length;
  return ending ? `即将结束 ${ending}` : '';
});

function jx3HomeCalendarText(): string {
  const char = characterList.value.find(c => c.game_id === 'jx3');
  if (!char) return '';
  const list = allAccountTasks.value[char.id] || [];
  const parts: string[] = [];
  for (const slot of JX3_CALENDAR_SLOTS) {
    if (!slot.defaultOn) continue;
    const task = findJx3CalendarTask(list, slot.key, slot.mark);
    if (!task) continue;
    const value = task.title.replace(/^【[^】]+】/, '').replace(/\s*[（(][^）)]+[）)]\s*$/, '').trim();
    if (value) parts.push(`${slot.label.replace('今日', '')} ${value}`);
  }
  return parts.join(' · ');
}

export type HomeLineGroup = {
  gameId: string;
  label: string;
  stamp: GameStamp;
  calendar: string;
  lines: HomeLine[];
};

export type HomePeriodSection = {
  id: HomePeriod;
  label: string;
  remaining: number;
  total: number;
  done: number;
  groups: HomeLineGroup[];
  completed: HomeLine[];
};

function homeGameOrder(ids: Iterable<string>): string[] {
  const set = new Set(ids);
  const preferred = ['jx3', 'sky', 'nsh'];
  const rest = [...set].filter(id => !preferred.includes(id)).sort((a, b) =>
    gameMetaOf(a).label.localeCompare(gameMetaOf(b).label, 'zh')
  );
  return [...preferred.filter(id => set.has(id)), ...rest];
}

/** 周常离复位不到一天时的提示。模板里每次渲染现算，不进 computed 缓存。 */
export function weeklyDueText(gameId: string): string {
  const clock = parseResetClock(gameMetaOf(gameId).weeklyReset, '周一 00:00');
  const now = new Date();
  const next = lastWeeklyBoundary(now, clock.weekday ?? 1, clock.hour, clock.minute);
  next.setDate(next.getDate() + 7);
  const hours = (next.getTime() - now.getTime()) / 3600000;
  if (hours >= 24) return '';
  return hours < 1 ? '不到 1 小时复位' : `${Math.floor(hours)} 小时后复位`;
}

function groupHomeLines(lines: HomeLine[], withCalendar = false): HomeLineGroup[] {
  const ids = homeGameOrder(lines.map(homeGameIdOfLine));
  return ids.map(gameId => {
    const meta = gameMetaOf(gameId);
    return {
      gameId,
      label: meta.label,
      stamp: meta.stamp,
      calendar: withCalendar && gameId === 'jx3' ? jx3HomeCalendarText() : '',
      lines: lines.filter(l => homeGameIdOfLine(l) === gameId)
    };
  }).filter(g => g.lines.length);
}

export function homeSectionsFor(scope = 'all', query = ''): HomePeriodSection[] {
  const term = query.trim().toLocaleLowerCase();
  return ([
    { id: 'daily', label: '今天要做' }, { id: 'weekly', label: '本周安排' }, { id: 'memo', label: '万象记事' },
  ] as const).map(period => {
    const lines = homeLinesFor(scope, period.id).filter(line => !term ||
      `${line.title} ${line.identity} ${line.hint} ${line.kind === 'memo' ? line.memo.note || '' : ''}`.toLocaleLowerCase().includes(term));
    const pending = lines.filter(line => !homeLineDone(line));
    const completed = lines.filter(homeLineDone);
    return {
      ...period, remaining: pending.length, total: lines.length, done: completed.length,
      groups: groupHomeLines(pending, period.id === 'daily'), completed,
    };
  });
}
