import { afterEach, expect, test, vi } from 'vitest';
import { calendarItemGroups, calendarLookSummary, calendarRemain, calendarStatus, getSkyCalendarFeed } from './skyCalendarService';
import { cachedFetch } from './fetchCache';
import type { SkyCalendarFeed, SkyCalendarItem } from './skyApi';
import { forecastSkyEvents, forecastReferenceItem } from './skyEventForecast';
import { currentSkyPeriods } from './skyPlan';

vi.mock('./fetchCache', () => ({ cachedFetch: vi.fn() }));
vi.mock('./skyApi', () => ({ fetchSkyCalendar: vi.fn() }));
afterEach(() => vi.useRealTimers());

function item(values: Partial<SkyCalendarItem> = {}): SkyCalendarItem {
  return { id: 'event', kind: 'event', region: 'cn', title: '活动', names: [], season: '',
    start: '2026-11-01', end: '2026-11-05', status: 'live', note: '', url: '', ...values };
}

test('all regions and slots use complete data for groups and summary, even without images', () => {
  const previews = ['兑换', '季卡', '商城', '特别物品'].flatMap(slot =>
    Array.from({ length: 15 }, (_, i) => ({ slot, name: `${slot}${i}`, kind: '', icon: '' })));
  for (const region of ['cn', 'intl']) {
    const event = item({ region, previews });
    expect(calendarItemGroups(event).map(g => [g.slot, g.items.length])).toEqual([
      ['兑换', 15], ['季卡', 15], ['商城', 15], ['特别物品', 15],
    ]);
    expect(calendarLookSummary(event)).toBe('兑换 15 · 季卡 15 · 商城 15 · 特别物品 15');
  }
});

test('festival estimates keep server histories separate and yield to announced editions', () => {
  const event_history = [
    { title: '宴会节', region: 'cn', start: '2025-12-24', end: '2026-01-13', url: 'https://wiki.biligame.com/sky/2025年宴会节' },
    { title: '宴会节', region: 'cn', start: '2024-12-24', end: '2025-01-13', url: 'https://wiki.biligame.com/sky/2024年宴会节' },
    { title: '宴会节', region: 'intl', start: '2025-12-12', end: '2026-01-01', url: 'https://sky-children-of-the-light.fandom.com/wiki/Days_of_Feast/2025' },
    { title: '宴会节', region: 'intl', start: '2024-12-23', end: '2025-01-12', url: 'https://sky-children-of-the-light.fandom.com/wiki/Days_of_Feast/2024' },
  ];
  const feed: SkyCalendarFeed = { as_of: '2026-11-08', events: [], cn_now: [], cn_next: [], intl_now: [], intl_next: [], event_history };
  const forecasts = forecastSkyEvents(feed, '2026-11-08');
  expect(forecasts.map(e => [e.region, e.window]).sort()).toEqual([['cn', '12 月下旬'], ['intl', '12 月中旬—12 月下旬']]);
  expect(forecasts.every(e => e.history.length === 2)).toBe(true);
  expect(forecasts.map(forecastReferenceItem).map(e => [e.region, e.start, e.status]).sort())
    .toEqual([['cn', '2025-12-24', 'ended'], ['intl', '2025-12-12', 'ended']]);
  expect(forecastReferenceItem(forecasts[0]).url).toBe(forecasts[0].history[0].url);
  expect(currentSkyPeriods(feed, '2026-12-24')).toEqual([]);
  feed.events.push(item({ title: '宴会节', start: '2026-12-24', end: '2027-01-13', status: 'upcoming' }));
  expect(forecastSkyEvents(feed, '2026-11-08').map(e => e.region)).toEqual(['intl']);
  expect(forecastSkyEvents(feed, '2027-01-01')).toEqual([]);
  expect(forecastSkyEvents(feed, '2027-11-08')).toEqual([]);
});

test('festival estimates need consecutive annual evidence and reject unstable timing', () => {
  const feed: SkyCalendarFeed = { as_of: '2026-11-08', events: [], cn_now: [], cn_next: [], intl_now: [], intl_next: [],
    event_history: [
      { title: '新节日', region: 'cn', start: '2025-12-24', end: '2026-01-13', url: 'https://wiki.biligame.com/sky/history' },
      { title: '新节日', region: 'cn', start: '2024-12-23', end: '2025-01-12', url: 'https://wiki.biligame.com/sky/history' },
    ] };
  expect(forecastSkyEvents(feed, '2026-11-08')).toHaveLength(1);
  expect(forecastSkyEvents(feed, '2026-06-01')).toEqual([]);
  feed.event_history![1].start = '2023-12-23';
  expect(forecastSkyEvents(feed, '2026-11-08')).toEqual([]);
  feed.event_history![1].start = '2024-10-23';
  expect(forecastSkyEvents(feed, '2026-11-08')).toEqual([]);
});

test('status and remaining time agree on date boundaries and stale source status', () => {
  const event = item({ status: 'upcoming' });
  expect(calendarRemain(event, '2026-10-31')).toBe('明天开始');
  expect(calendarStatus(event, '2026-11-01')).toBe('live');
  expect(calendarRemain(event, '2026-11-01')).toBe('今天开始');
  expect(calendarRemain(event, '2026-11-03')).toBe('还剩 2 天');
  expect(calendarRemain(event, '2026-11-04')).toBe('明天结束');
  expect(calendarRemain(event, '2026-11-05')).toBe('今天结束');
  expect(calendarStatus(event, '2026-11-06')).toBe('ended');
  expect(calendarRemain(event, '2026-11-06')).toBe('');
  expect(calendarRemain(item({ status: 'inferred' }), '2026-11-04')).toBe('');
  expect(calendarRemain(item({ start: '', end: '2026-02-30' }), '2026-02-28')).toBe('');
});

test('cached feeds move announced returns between upcoming and live and drop them after ending', async () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 10, 4, 12));
  const feed: SkyCalendarFeed = { as_of: '2026-11-01', season: item({ kind: 'season', end: '2026-11-02' }),
    events: [item({ status: 'upcoming' }), item({ id: 'ended', end: '2026-11-02' })],
    cn_now: [], cn_next: [item({ status: 'upcoming' })],
    intl_now: [item({ id: 'expired-live', kind: 'reprint', region: 'intl', names: ['已结束'], end: '2026-11-03' })],
    intl_next: [
      item({ id: 'history', kind: 'reprint', region: 'intl', names: ['旧记录'], end: '2026-10-25', status: 'ended' }),
      item({ id: 'started', kind: 'reprint', region: 'intl', names: ['已开始'], status: 'upcoming' }),
      item({ id: 'next', kind: 'reprint', region: 'intl', names: ['已官宣'], start: '2026-11-05', end: '2026-11-08', status: 'upcoming' }),
      item({ id: 'unnamed', kind: 'reprint', region: 'intl', start: '2026-11-05', end: '2026-11-08', status: 'inferred' }),
    ] };
  vi.mocked(cachedFetch).mockResolvedValue(feed);
  const result = await getSkyCalendarFeed();
  expect(result?.season).toBeNull();
  expect(result?.events.map(e => e.status)).toEqual(['live']);
  expect(result?.cn_now).toHaveLength(1);
  expect(result?.cn_next).toHaveLength(0);
  expect(result?.intl_now.map(e => e.id)).toEqual(['started']);
  expect(result?.intl_next.map(e => e.id)).toEqual(['next']);
  vi.setSystemTime(new Date(2026, 10, 5, 12));
  expect((await getSkyCalendarFeed())?.intl_now.map(e => e.id)).toEqual(['started', 'next']);
  expect((await getSkyCalendarFeed())?.intl_next).toEqual([]);
  vi.setSystemTime(new Date(2026, 10, 9, 12));
  const expired = await getSkyCalendarFeed();
  expect(expired?.intl_now).toEqual([]);
  expect(expired?.intl_next).toEqual([]);
});
