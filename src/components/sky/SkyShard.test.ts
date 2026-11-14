import { expect, test, vi } from 'vitest';
import { createSSRApp, ref } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { calculateSkyShard, shardOccurrenceState } from '../../services/skyShards';

vi.mock('../../workspace/sky/daily', () => ({
  skyShard: ref(calculateSkyShard(new Date('2026-11-07T14:30:00+08:00'), 'netease')),
  skyServerMode: ref('netease'), setSkyServerMode: vi.fn(), dailyCanEdit: ref(true), dailyCharacterId: ref('test'),
}));
vi.mock('../../workspace/sky/plan', () => ({
  skyTodayGroups: ref([]), skyTodayProgress: ref({ done: 2, total: 2 }), skyEdenDone: ref(false),
  toggleSkyEden: vi.fn(), skyPlanDay: ref('2026-11-07'), skyPeriods: ref([]), goalsFor: () => [],
}));
vi.mock('../../workspace/sky/tabs', () => ({ skyCalendarFeed: ref({}), skyDailyData: ref(null), setSkySubTab: vi.fn() }));
vi.mock('../../workspace/guides/lightbox', () => ({ openLightbox: vi.fn() }));
vi.mock('./wikiImage', () => ({ vWikiImg: {}, hotlinkImageSrc: (url: string) => url }));
vi.mock('../../workspace/vault', () => ({ activeSkyCharacter: ref({ name: '测试角色' }) }));
vi.mock('../../workspace/core', () => ({ currentTab: ref('sky') }));
vi.mock('./SkyPeriodPanel.vue', () => ({ default: { render: () => null } }));
vi.mock('./SkyTodoList.vue', () => ({ default: { render: () => null } }));
vi.mock('../GameStampIcon.vue', () => ({ default: { render: () => null } }));

import { skyShard } from '../../workspace/sky/daily';
import SkyDailyPanel from './SkyDailyPanel.vue';
import TodaySky from '../today/TodaySky.vue';

test('time windows and both visible surfaces agree before, during, and after the current occurrence', async () => {
  for (const [time, status, slots, live] of [
    ['14:07:59', 'upcoming', ['ended', 'next', 'upcoming'], false],
    ['14:08:00', 'active', ['ended', 'active', 'upcoming'], true],
    ['14:30:00', 'active', ['ended', 'active', 'upcoming'], true],
    ['14:59:59', 'active', ['ended', 'active', 'upcoming'], true],
    ['15:00:00', 'upcoming', ['ended', 'ended', 'next'], false],
    ['23:00:00', 'all_ended', ['ended', 'ended', 'ended'], false],
  ] as const) {
    const shard = calculateSkyShard(new Date(`2026-11-07T${time}+08:00`), 'netease');
    skyShard.value = shard;
    expect(shard.status, time).toBe(status);
    expect(shard.occurrences.map(occ => shardOccurrenceState(shard, occ.index)), time).toEqual(slots);
    const daily = await renderToString(createSSRApp(SkyDailyPanel));
    const overview = await renderToString(createSSRApp(TodaySky));
    const shardClass = daily.match(/<section class="([^"]+)" aria-label="落石时段"/)?.[1].split(' ');
    expect(shardClass, time).toContain('sky-shard');
    expect(shardClass?.includes('is-live'), time).toBe(live);
    expect(overview.includes('is-live'), time).toBe(live);
    expect(daily.includes('aria-current="time"'), time).toBe(live);
    expect(daily.indexOf('aria-label="落石时段"')).toBeLessThan(daily.indexOf('aria-label="今天要做"'));
    if (live) {
      expect(daily).toMatch(/class="is-active" aria-current="time"[^>]*>.*?第 2 场.*?进行中.*?14:08 ~ 15:00/s);
      expect(overview).toContain('第 2 次爆发进行中');
    }
  }
});

test('falling and no-shard days remain distinct from an active window', async () => {
  const schedule = calculateSkyShard(new Date('2026-11-07T14:00:00+08:00'), 'global');
  const shard = calculateSkyShard(schedule.occurrences[0].start, 'global');
  expect(shard.status).toBe('falling');
  expect(shardOccurrenceState(shard, 1)).toBe('falling');
  skyShard.value = shard;
  expect(await renderToString(createSSRApp(SkyDailyPanel))).toContain('正在降落');
  skyShard.value = calculateSkyShard(new Date('2026-11-09T14:30:00+08:00'), 'netease');
  const html = await renderToString(createSSRApp(SkyDailyPanel));
  expect(html).toContain('今日无落石');
  expect(html).not.toContain('is-live');
  expect(html).not.toContain('aria-current="time"');
});
