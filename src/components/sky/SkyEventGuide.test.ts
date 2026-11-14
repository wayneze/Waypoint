import { afterEach, expect, test, vi } from 'vitest';
import { createSSRApp, reactive, ref } from 'vue';
import { renderToString } from 'vue/server-renderer';
import { dailyEventImages, eventImageSources, mergeEventCatalog, matchEventGuide } from '../../services/skyEventService';
import { periodKey, skyActivityDailies } from '../../services/skyPlan';
import type { SkyCalendarFeed, SkyCalendarItem } from '../../services/skyApi';
import { setProgressEntries } from '../../services/progress';

vi.mock('../../workspace/core', () => ({ state: reactive({ eventGuides: [] }) }));
vi.mock('../../workspace/sky/tabs', () => ({ skyDailyData: ref(null) }));
vi.mock('../../workspace/sky/plan', () => ({ skyPlanDay: ref('2026-11-07') }));
vi.mock('../../workspace/guides/lightbox', () => ({ openLightbox: vi.fn() }));
vi.mock('./wikiImage', () => ({ vWikiImg: {}, hotlinkImageSrc: (url: string) => url }));
import { state } from '../../workspace/core';
import SkyEventGuide from './SkyEventGuide.vue';
import { skyDailyData } from '../../workspace/sky/tabs';
import type { SkyDailyGuidePayload } from '../../services/skyDailyService';

afterEach(() => vi.unstubAllGlobals());
const event: SkyCalendarItem = { id: 'mischief', kind: 'event', region: 'cn', title: '恶作剧之日',
  names: [], season: '', start: '2026-10-24', end: '2026-11-13', status: 'live', note: '', url: 'https://sky.163.com/news/event' };
const feed: SkyCalendarFeed = { as_of: '2026-11-07', events: [event], cn_now: [], cn_next: [], intl_now: [], intl_next: [] };

test('same-edition catalog supplies visible gameplay and daily instructions without mixing editions or regions', async () => {
  const guides = mergeEventCatalog({ as_of: '2026-11-07', events: [{
    id: 'wiki-event-mischief', title: '2026年恶作剧之日', start: event.start, end: event.end, status: 'live',
    slogan: '', realm: '', loc: '', url: 'https://wiki.biligame.com/sky/2026年恶作剧之日', items: [],
    quests: '通过体验魔法剧情可获得糖果代币15枚。游玩每日游戏、在魔法坩埚旁收集，每日最多5枚。',
  }] }, {});
  state.eventGuides = guides;
  const html = await renderToString(createSSRApp(SkyEventGuide, { item: event, compact: true }));
  expect(html).toContain('图文攻略');
  expect(html).toContain('aria-expanded="false"');
  expect(html).not.toContain('sky-event-guide-content');
  const daily = skyActivityDailies(null, feed, '2026-11-07', 'guide-test', guides);
  expect(daily).toHaveLength(1);
  expect(daily[0].how).toBe('游玩每日游戏、在魔法坩埚旁收集，每日最多5枚。');
  expect(daily[0].how).not.toContain('15枚');
  expect(matchEventGuide({ ...event, region: 'intl' }, guides)).toBeUndefined();
  expect(matchEventGuide({ ...event, start: '2025-10-24' }, guides)).toBeUndefined();
  vi.stubGlobal('localStorage', { setItem: vi.fn() });
  await setProgressEntries('sky.event_daily_enabled', [[periodKey(event), false]], 'guide-test');
  expect(skyActivityDailies(null, feed, '2026-11-07', 'guide-test', guides)).toHaveLength(0);
  expect(await renderToString(createSSRApp(SkyEventGuide, { item: event }))).toContain('展开恶作剧之日图文攻略');
});

test('caption matching handles multiple activities, new names and region/date boundaries without event-specific rules', async () => {
  for (const title of ['恶作剧之日', '缤纷飞行日', '未来的新活动']) {
    const row = { ...event, title };
    const payload = { dateStr: '2026-11-07', activityImages: [
      { caption: `${title}活动兑换树`, url: 'https://wx1.sinaimg.cn/large/exchange.jpg' },
      { caption: `${title}代币位置攻略`, url: 'https://wx1.sinaimg.cn/large/tokens.jpg' },
      { caption: '另一个节日攻略', url: 'https://wx1.sinaimg.cn/large/other.jpg' },
    ] } as SkyDailyGuidePayload;
    skyDailyData.value = payload;
    expect(dailyEventImages(row, payload, '2026-11-07')).toHaveLength(2);
    expect(dailyEventImages({ ...row, region: 'intl' }, payload, '2026-11-07')).toHaveLength(0);
    expect(dailyEventImages({ ...row, end: '2025-11-13' }, payload, '2026-11-07')).toHaveLength(0);
    expect(dailyEventImages(row, payload, '2026-11-08')).toHaveLength(0);
    expect(eventImageSources(row, [])).toEqual([event.url]);
    const html = await renderToString(createSSRApp(SkyEventGuide, { item: row }));
    expect(html).toContain(`展开${title}图文攻略`);
    expect(html).toContain('2 张');
    expect(html).not.toContain('<figure');
    expect(html).not.toContain('另一个节日攻略');
  }
  skyDailyData.value = null;
});
