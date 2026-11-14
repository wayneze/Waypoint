/**
 * 游戏日/周切点：光遇 00:00，剑三 07:00，其余默认 00:00。
 * 只算本地墙上时钟，不调时区。
 */

export interface ResetClock {
  hour: number;
  minute: number;
  weekday: number | null;
}

const WEEKDAY: Array<[string, number]> = [
  ['星期日', 0],
  ['星期天', 0],
  ['周日', 0],
  ['周天', 0],
  ['星期一', 1],
  ['周一', 1],
  ['星期二', 2],
  ['周二', 2],
  ['星期三', 3],
  ['周三', 3],
  ['星期四', 4],
  ['周四', 4],
  ['星期五', 5],
  ['周五', 5],
  ['星期六', 6],
  ['周六', 6]
];

export function parseResetClock(rule: string | undefined, fallback = '00:00'): ResetClock {
  const src = (rule || fallback).trim() || fallback;
  let weekday: number | null = null;
  for (const [label, day] of WEEKDAY) {
    if (src.includes(label)) {
      weekday = day;
      break;
    }
  }
  const match = src.match(/(\d{1,2}):(\d{2})/);
  const fb = fallback.match(/(\d{1,2}):(\d{2})/);
  const hour = clampHour(match ? Number(match[1]) : fb ? Number(fb[1]) : 0);
  const minute = clampMinute(match ? Number(match[2]) : fb ? Number(fb[2]) : 0);
  return { hour, minute, weekday };
}

function clampHour(n: number) {
  if (!Number.isFinite(n)) return 0;
  return Math.min(23, Math.max(0, Math.floor(n)));
}

function clampMinute(n: number) {
  if (!Number.isFinite(n)) return 0;
  return Math.min(59, Math.max(0, Math.floor(n)));
}

function atLocal(base: Date, hour: number, minute: number) {
  return new Date(base.getFullYear(), base.getMonth(), base.getDate(), hour, minute, 0, 0);
}

export function lastDailyBoundary(now: Date, hour: number, minute: number): Date {
  const today = atLocal(now, hour, minute);
  if (now.getTime() >= today.getTime()) return today;
  const prev = atLocal(now, hour, minute);
  prev.setDate(prev.getDate() - 1);
  return prev;
}

export function lastWeeklyBoundary(now: Date, weekday: number, hour: number, minute: number): Date {
  const wd = ((weekday % 7) + 7) % 7;
  const candidate = atLocal(now, hour, minute);
  const delta = (candidate.getDay() - wd + 7) % 7;
  candidate.setDate(candidate.getDate() - delta);
  if (now.getTime() < candidate.getTime()) candidate.setDate(candidate.getDate() - 7);
  return candidate;
}

export function periodStamp(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  return `${y}-${m}-${day}T${hh}:${mm}`;
}

export const WEEKDAY_OPTIONS: Array<{ value: number; label: string }> = [
  { value: 0, label: '周日' },
  { value: 1, label: '周一' },
  { value: 2, label: '周二' },
  { value: 3, label: '周三' },
  { value: 4, label: '周四' },
  { value: 5, label: '周五' },
  { value: 6, label: '周六' }
];

function pad2(n: number) {
  return String(n).padStart(2, '0');
}

export function formatClockTime(hour: number, minute: number): string {
  return `${pad2(hour)}:${pad2(minute)}`;
}

export function normalizeDailyRule(input: string, fallback = '00:00'): string {
  const c = parseResetClock(input, fallback);
  return formatClockTime(c.hour, c.minute);
}

export function normalizeWeeklyRule(weekday: number, time: string, fallback = '周一 00:00'): string {
  const c = parseResetClock(time, fallback);
  const wd = ((weekday % 7) + 7) % 7;
  return `周${'日一二三四五六'[wd]} ${formatClockTime(c.hour, c.minute)}`;
}
