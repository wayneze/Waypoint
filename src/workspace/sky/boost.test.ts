import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { SkyDailyGuidePayload } from '../../services/skyDailyService';
import type { SkyBoostFeedback } from '../../services/skyBoost';
import { beijingDay } from '../../services/cycleReset';

const fetchFeedback = vi.hoisted(() => vi.fn());
const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));

const link = 'http://dp.mz997.com:92/#/order-search?order=test-order';
const day = '2026-11-06';
const now = new Date('2026-11-06T11:00:00Z');
const feedback: SkyBoostFeedback = {
  day, fetched_at: now.toISOString(), status: '今日代练已完成', completed: true, order_state: '正常',
};
function daily(): SkyDailyGuidePayload {
  return {
    dateStr: day, dayOfWeekStr: '星期五', isFallback: false, fetchedAt: now.getTime(),
    quests: Array.from({ length: 4 }, (_, i) => ({
      id: `q${i}`, title: `任务${i}`, category: 'social', categoryLabel: '', realm: '', detailSpot: '', tip: '', imageUrl: '', completed: false,
    })),
    seasonalCandles: { realm: '', headline: '', groupName: '', spots: Array.from({ length: 4 }, (_, i) => ({
      index: i + 1, label: '', desc: '', imageUrl: '', completed: false,
    })) },
    treasureCandles: [], eventDaily: { name: '活动代币', imageUrl: '', completed: false },
    overviewImages: { questsImg: '', seasonCandleImg: '', treasureCandleImg: '' },
  };
}

beforeEach(() => {
  vi.resetModules();
  fetchFeedback.mockReset();
  invoke.mockReset();
  vi.useFakeTimers();
  vi.setSystemTime(now);
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
  });
  vi.stubGlobal('window', { location: { search: '' } });
});
afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); vi.unstubAllGlobals(); });

async function setup() {
  const progress = await import('../../services/progress');
  const service = await import('../../services/skyBoost');
  vi.spyOn(service, 'fetchSkyBoostFeedback').mockImplementation(fetchFeedback);
  const core = await import('../core');
  core.vaultList.value = [{
    id: 'account', platform: '光遇', account_name: '测试账号', assets: [],
    characters: [
      { id: 'a', name: '角色A', assets: [], track_daily: true },
      { id: 'b', name: '角色B', assets: [], track_daily: true },
    ],
  }];
  core.activeCharacter.value = 'a';
  const tabs = await import('./tabs');
  tabs.skyDailyData.value = daily();
  const workspace = await import('./boost');
  const dailyWorkspace = await import('./daily');
  await service.saveSkyBoostBinding('a', { link, autoSync: true, included: { quests: true, candles: true, event: false } });
  return { progress, service, core, tabs, workspace, dailyWorkspace };
}

test('真实同步入口只勾套餐项目；加购活动后才勾活动；重复同步不追加进度，重载后仍隔离角色', async () => {
  const { service, progress, workspace, tabs, core } = await setup();
  fetchFeedback.mockResolvedValue(feedback);
  await workspace.syncSkyBoost(true);
  expect(tabs.skyDailyData.value?.quests.every(q => q.completed)).toBe(true);
  expect(tabs.skyDailyData.value?.seasonalCandles.spots.every(s => s.completed)).toBe(true);
  expect(tabs.skyDailyData.value?.eventDaily?.completed).toBe(false);
  const first = progress.progressEntries('sky.quest', 'a');
  await workspace.syncSkyBoost(true);
  expect(progress.progressEntries('sky.quest', 'a')).toEqual(first);
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), included: { quests: true, candles: true, event: true } });
  await workspace.syncSkyBoost(true);
  expect(tabs.skyDailyData.value?.eventDaily?.completed).toBe(true);
  await progress.hydrateProgress();
  core.activeCharacter.value = 'b';
  expect(tabs.skyDailyData.value?.quests.some(q => q.completed)).toBe(false);
  expect(tabs.skyDailyData.value?.eventDaily?.completed).toBe(false);
  core.activeCharacter.value = 'a';
  expect(tabs.skyDailyData.value?.eventDaily?.completed).toBe(true);
});

test('三项服务所有组合都按独立选项写入', async () => {
  const { service, progress } = await setup();
  for (let mask = 0; mask < 8; mask++) {
    const included = { quests: !!(mask & 1), candles: !!(mask & 2), event: !!(mask & 4) };
    await service.applySkyBoostFeedback(daily(), `combo-${mask}`, { link, autoSync: true, included }, feedback, now);
    expect(progress.dailyChecked('sky.quest', day, `combo-${mask}`).size > 0).toBe(included.quests);
    expect(progress.dailyChecked('sky.season_candle', day, `combo-${mask}`).size > 0).toBe(included.candles);
    expect(progress.dailyChecked('sky.event_daily', day, `combo-${mask}`).size > 0).toBe(included.event);
  }
});

test('外观和余额独立于攻略保存，精确勾收藏且不丢手工记录；缺失响应与重载不清空、不同角色不串号', async () => {
  const { progress, workspace, tabs, core } = await setup();
  const inventory = await import('../../services/skyInventory');
  const collection = await import('../../services/skyCollection');
  const coat = { id: '1', name: 'CharSkyKid_Body_AP21Coat', kind: 'body' };
  const guide = {
    id: 'deer', season: '九色鹿季', loc: '', benefit: '', items: '', priority: '',
    candles: 0, ascended: 0, hearts: 0, selected: false,
    itemList: ['猎手 · 裤子', '猎手 · 斗篷', '公主 · 裤子'].map((name, i) => ({
      id: `item-${i}`, name, costDesc: '', candles: 0, ascended: 0, hearts: 0, obtained: false,
    })),
  };
  core.state.seasonalGuides = [guide];
  await progress.setProgressEntries('sky.season_wish', [['deer', { items: { 'item-2': true } }]], 'a');
  tabs.skyDailyData.value = null;
  fetchFeedback.mockResolvedValue({ ...feedback, assets: {
    outfits: [coat], balances: { candles: 457, season_candle: 28, heart: 0, prestige: 19 }, warnings: [],
  } });
  await workspace.syncSkyBoost(true);
  expect(inventory.loadSkyInventory('a')?.outfits).toEqual([coat]);
  expect(inventory.loadSkyInventory('a')?.balances.heart).toBe(0);
  expect(progress.progressGet('sky.season_wish', 'deer', 'a')).toEqual({ items: { 'item-0': true, 'item-2': true } });
  expect(workspace.boostResult.value?.message).toContain('已同步 1 件外观');
  fetchFeedback.mockResolvedValue({ ...feedback, assets: { outfits: null, balances: { candles: 460 }, warnings: ['未提供外观'] } });
  await workspace.syncSkyBoost(true);
  await progress.hydrateProgress();
  expect(inventory.loadSkyInventory('a')?.outfits).toHaveLength(1);
  expect(inventory.loadSkyInventory('a')?.balances).toEqual({ candles: 460, season_candle: 28, heart: 0, prestige: 19 });
  let entries = collection.skyCollectionEntries('a', core.state.seasonalGuides, []);
  expect(entries.filter(e => e.source === 'season' && e.owned)).toHaveLength(2);
  expect(entries.filter(e => e.source === 'synced')).toHaveLength(0);
  const coatEntry = entries.find(e => e.itemId === 'item-0')!;
  await collection.setSkyCollectionWanted(coatEntry, 'a', true);
  await collection.setSkyCollectionOwned(coatEntry, 'a', false);
  await inventory.syncSkyInventoryCollection('a', core.state.seasonalGuides, 'sky.season_wish');
  await progress.hydrateProgress();
  entries = collection.skyCollectionEntries('a', core.state.seasonalGuides, []);
  expect(entries.find(e => e.key === coatEntry.key)).toMatchObject({ owned: false, wanted: true });
  const plan = await import('./plan');
  await plan.saveSkyCollectionItems('sky.season_wish', 'deer', ['item-1'], true);
  expect(collection.skyCollectionEntries('a', core.state.seasonalGuides, []).find(e => e.itemId === 'item-1')?.owned).toBe(true);
  const spiritEntry = entries.find(e => e.source === 'permanent')!;
  await collection.setSkyCollectionOwned(spiritEntry, 'a', true);
  const spirits = await import('../../services/skyData');
  expect(spirits.loadSkySpirits('a').find(e => e.id === spiritEntry.sourceId)).toMatchObject({ itemObtained: true, wingUnlocked: false, unlocked: false });
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
  invoke.mockRejectedValueOnce(new Error('disk full'));
  await expect(collection.setSkyCollectionOwned(coatEntry, 'a', true)).rejects.toThrow('disk full');
  expect(collection.skyCollectionEntries('a', core.state.seasonalGuides, []).find(e => e.key === coatEntry.key)?.owned).toBe(false);
  vi.stubGlobal('window', { location: { search: '' } });
  core.activeCharacter.value = 'b';
  expect(inventory.loadSkyInventory('b')).toBeUndefined();
  expect(collection.skyCollectionEntries('b', core.state.seasonalGuides, []).some(e => e.owned || e.wanted)).toBe(false);
  expect(inventory.skyOutfitCollectionMatch({ ...coat, name: 'CharSkyKid_Body_Unknown' }, [guide])).toBeUndefined();
  expect(inventory.skyOutfitCollectionMatch(coat, [guide, { ...guide, id: 'duplicate' }])).toBeUndefined();
  expect(inventory.describeSkyOutfit({ id: '2', name: 'CharSkyKid_Wing_MischiefGoth', kind: 'wing' }).image).toContain('UiOutfitCapeMischiefGoth.png');
});

test('未完成、失败、昨日、跨日、过期或不完整反馈都不勾选', async () => {
  const { progress, workspace, tabs } = await setup();
  for (const item of [
    { ...feedback, completed: false, status: '跑手正在号上代练' },
    { ...feedback, day: '2026-11-05' },
    { ...feedback, fetched_at: '2026-11-05T23:59:59+08:00' },
    { ...feedback, fetched_at: 'invalid' },
    { ...feedback, order_state: '异常' },
  ]) {
    fetchFeedback.mockResolvedValue(item);
    await workspace.syncSkyBoost(true);
    expect(progress.progressEntries('sky.quest', 'a')).toEqual([]);
  }
  fetchFeedback.mockRejectedValue(new Error('network'));
  await workspace.syncSkyBoost(true);
  expect(workspace.boostResult.value?.message).toBe('network');
  fetchFeedback.mockResolvedValue(feedback);
  tabs.skyDailyData.value!.quests.pop();
  await workspace.syncSkyBoost(true);
  expect(progress.progressEntries('sky.quest', 'a')).toEqual([]);
});

test('等待期间切换角色仍归原角色；更换订单则丢弃旧响应；保存失败保留状态并可重试', async () => {
  const { service, progress, workspace, core, tabs } = await setup();
  let finish!: (result: SkyBoostFeedback) => void;
  fetchFeedback.mockImplementation(() => new Promise(resolve => { finish = resolve; }));
  let job = workspace.syncSkyBoost(true);
  core.activeCharacter.value = 'b';
  finish(feedback);
  await job;
  expect(tabs.skyDailyData.value?.quests.some(q => q.completed)).toBe(false);
  expect(progress.dailyChecked('sky.quest', day, 'a').has('q0')).toBe(true);
  core.activeCharacter.value = 'a';
  await progress.setCharacterDailyChecks('a', day, { 'sky.quest': ['q0', '任务0', 'q1', '任务1', 'q2', '任务2', 'q3', '任务3'] }, false);
  job = workspace.syncSkyBoost(true);
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), link: link.replace('test-order', 'replacement') });
  finish(feedback);
  await job;
  expect(progress.dailyChecked('sky.quest', day, 'a').size).toBe(0);
  fetchFeedback.mockResolvedValue(feedback);
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
  invoke.mockRejectedValueOnce(new Error('disk full'));
  await workspace.syncSkyBoost(true);
  expect(progress.dailyChecked('sky.quest', day, 'a').size).toBe(0);
  expect(workspace.boostResult.value?.message).toContain('disk full');
  expect(service.loadSkyBoostCompletion('a', service.loadSkyBoostBinding('a'))).toBeUndefined();
  invoke.mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error('completion save failed'));
  await workspace.syncSkyBoost();
  expect(progress.dailyChecked('sky.quest', day, 'a').has('q0')).toBe(true);
  expect(service.loadSkyBoostCompletion('a', service.loadSkyBoostBinding('a'))).toBeUndefined();
  expect(workspace.boostResult.value?.message).toBe('completion save failed');
  invoke.mockResolvedValue(undefined);
  await workspace.syncSkyBoost();
  expect(progress.dailyChecked('sky.quest', day, 'a').has('q0')).toBe(true);
  expect(service.loadSkyBoostCompletion('a', service.loadSkyBoostBinding('a'))).toBeDefined();
});

test('旧共享记录只在选定归属后合并；关闭自动同步仍可手动刷新', async () => {
  const { service, progress, workspace, dailyWorkspace } = await setup();
  const snapshot = JSON.parse(localStorage.getItem('waypoint_progress_debug') || '{"items":[],"checks":[]}');
  snapshot.checks.push(['sky.quest', day, 'old']);
  localStorage.setItem('waypoint_progress_debug', JSON.stringify(snapshot));
  await progress.hydrateProgress();
  await dailyWorkspace.claimDailyLegacy();
  expect(progress.legacyDailyCheckCount(['sky.quest'])).toBe(0);
  expect(progress.dailyChecked('sky.quest', day, 'a').has('old')).toBe(true);
  expect(progress.dailyChecked('sky.quest', day, 'b').size).toBe(0);
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), autoSync: false });
  fetchFeedback.mockResolvedValue(feedback);
  await workspace.syncSkyBoost();
  expect(fetchFeedback).not.toHaveBeenCalled();
  await workspace.syncSkyBoost(true);
  expect(fetchFeedback).toHaveBeenCalledOnce();
});

test('订单过期日已过时自动同步不再请求并持久停止；到期日当天仍可查询', async () => {
  const { service, workspace, progress } = await setup();
  const active = {
    ...feedback, completed: false, status: '跑手正在号上代练',
    days: 0, expiration_date: '2026-11-06',
  };
  expect(service.skyBoostExpired(active, new Date('2026-11-06T12:00:00+08:00'))).toBe(false);
  expect(service.skyBoostExpired(active, new Date('2026-11-07T00:01:00+08:00'))).toBe(true);
  expect(service.skyBoostExpired({ ...active, expiration_date: '2026-11-06 12:00:00' },
    new Date('2026-11-06T12:00:00+08:00'))).toBe(true);
  fetchFeedback.mockResolvedValue(active);
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(1);
  await progress.hydrateProgress();
  vi.setSystemTime(new Date('2026-11-07T00:01:00+08:00'));
  await service.saveSkyBoostBinding('a', {
    ...service.loadSkyBoostBinding('a'), included: { quests: false, candles: true, event: false },
  });
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(1);
  await workspace.syncSkyBoost(true);
  expect(fetchFeedback).toHaveBeenCalledTimes(2);
  fetchFeedback.mockResolvedValue({
    ...active, expiration_date: '2026-11-20', days: 13,
    fetched_at: new Date().toISOString(), day: beijingDay(),
  });
  await workspace.syncSkyBoost(true);
  expect(service.loadSkyBoostLatest('a', service.loadSkyBoostBinding('a'))?.feedback.expiration_date).toBe('2026-11-20');
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(4);
});

test('首页与光遇共享多活动进度；缺攻略仍展示，促销不自动算日常，套餐不会完成周期目标', async () => {
  const { tabs, service, workspace, progress, core } = await setup();
  const row = (title: string, note: string) => ({
    id: title, title, note, kind: 'event', region: 'cn', names: [], season: '',
    start: '2026-11-01', end: '2026-11-20', status: 'live', url: '',
  });
  tabs.skyDailyData.value!.eventDaily = undefined;
  tabs.skyCalendarFeed.value = {
    as_of: day, events: [row('活动甲', '每日收集代币'), row('活动乙', '每天领取'), row('充值优惠', '限时购买')],
    cn_now: [], cn_next: [], intl_now: [], intl_next: [],
  };
  const plan = await import('./plan');
  const home = await import('../daily');
  expect(plan.skyTodayGroups.value.map(g => g.title)).toEqual(['每日四任务', '季节蜡烛', '活动甲', '活动乙']);
  expect(plan.skyTodayGroups.value[2].hint).toContain('攻略待更新');
  const line = home.homeSectionsFor('a')[0].groups.flatMap(group => group.lines).find(line => line.kind === 'sky')!;
  await plan.toggleSkyGroup(line.group, line.charId);
  expect(plan.skyTodayGroups.value[0].done).toBe(true);
  expect(home.homeSectionsFor('a')[0].completed.some(l => l.id === line.id)).toBe(true);
  expect(plan.skyGroupsFor('b')[0].done).toBe(false);
  fetchFeedback.mockResolvedValue(feedback);
  await workspace.syncSkyBoost(true);
  expect(plan.skyTodayGroups.value[2].done).toBe(false);
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), included: { quests: true, candles: true, event: true } });
  await workspace.syncSkyBoost(true);
  expect(plan.skyTodayGroups.value.every(g => g.done)).toBe(true);
  expect(progress.progressEntries('sky.event_once', 'a')).toEqual([]);
  await core.resetCharacterDailies('a');
  expect(plan.skyTodayGroups.value.some(g => g.done)).toBe(false);
  await plan.setActivityDaily(tabs.skyCalendarFeed.value.events[2], true);
  expect(plan.skyTodayGroups.value).toHaveLength(5);
  await plan.setActivityDaily(tabs.skyCalendarFeed.value.events[2], false);
  expect(plan.skyTodayGroups.value).toHaveLength(4);
});

test('本期目标跨天保留；回归兑换与图鉴共用角色记录', async () => {
  const { tabs, core, dailyWorkspace } = await setup();
  const { nextTick } = await import('vue');
  const row = {
    id: 'return', title: '测试先祖回归', note: '', kind: 'reprint', region: 'cn', names: ['测试先祖'], season: '',
    start: '2026-11-01', end: '2026-11-20', status: 'live', url: '',
  };
  tabs.skyCalendarFeed.value = { as_of: day, events: [], cn_now: [row], cn_next: [], intl_now: [], intl_next: [] };
  const seasonService = await import('../../services/skySeasonService');
  const catalog = { as_of: day, seasons: [{
    id: 'season', title: '测试季', items: [{ id: 'cape', name: '斗篷', spirit: '测试先祖' }],
  }] } as unknown as import('../../services/skyApi').SkySeasonCatalog;
  core.state.seasonalGuides = seasonService.mergeSeasonCatalog(catalog, {}, []);
  const plan = await import('./plan');
  expect(plan.goalsFor(row)).toHaveLength(3);
  await plan.togglePeriodGoal(plan.goalsFor(row)[0]);
  await plan.togglePeriodGoal(plan.goalsFor(row)[2]);
  await nextTick();
  expect(core.state.seasonalGuides[0].itemList[0].obtained).toBe(true);
  const collection = await import('../../services/skyCollection');
  const entries = collection.skyCollectionEntries('a', core.state.seasonalGuides, []);
  expect(collection.calendarCollectionEntries(row, entries, core.state.seasonalGuides, [])).toMatchObject([{ itemId: 'cape', owned: true }]);
  expect(collection.calendarCollectionEntries({ ...row, region: 'intl' }, entries, core.state.seasonalGuides, [])).toEqual([]);
  const eventGuide = { ...core.state.seasonalGuides[0], id: 'event', season: '活动', start: row.start, end: row.end, loc: row.start,
    itemList: [{ ...core.state.seasonalGuides[0].itemList[0], kind: '斗篷', costDesc: '斗篷', slot: '商城', spirit: '活动' }] };
  const eventRow = { ...row, kind: 'event', title: '活动' };
  const eventEntries = collection.skyCollectionEntries('a', [], [eventGuide]);
  expect(collection.calendarCollectionEntries(eventRow, eventEntries, [], [eventGuide])).toHaveLength(1);
  expect(eventEntries.find(e => e.source === 'event')).toMatchObject({ cost: '', method: '商城购买', location: undefined, spirit: undefined, period: row.start + ' — ' + row.end });
  expect(collection.calendarCollectionEntries({ ...eventRow, start: '2025-11-01' }, eventEntries, [], [eventGuide])).toEqual([]);
  core.activeCharacter.value = 'b';
  await nextTick();
  expect(plan.goalsFor(row).some(g => g.done)).toBe(false);
  expect(core.state.seasonalGuides[0].itemList[0].obtained).toBe(false);
  core.activeCharacter.value = 'a';
  vi.setSystemTime(new Date('2026-11-07T11:00:00Z'));
  dailyWorkspace.skyShard.value = { ...dailyWorkspace.skyShard.value };
  await nextTick();
  expect(plan.goalsFor(row)[0].done).toBe(true);
  expect(plan.skyTodayGroups.value[0].ready).toBe(false);

});

test('自动同步只在未完成时轮询，完成后跨重启停查；手动、换订单套餐、换角色和次日可重新同步', async () => {
  let { workspace, core, service, progress, tabs } = await setup();
  const listeners = new Map<string, () => void>();
  const documentStub = {
    visibilityState: 'visible',
    addEventListener: (name: string, fn: () => void) => listeners.set(name, fn),
    removeEventListener: (name: string) => listeners.delete(name),
  };
  vi.stubGlobal('document', documentStub);
  let completed = false;
  fetchFeedback.mockImplementation(async () => ({
    ...feedback, completed, day: beijingDay(), fetched_at: new Date().toISOString(),
  }));
  core.currentTab.value = 'daily';
  const stop = workspace.startSkyBoostSync();
  await vi.advanceTimersByTimeAsync(0);
  expect(fetchFeedback).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(300_000);
  expect(fetchFeedback).toHaveBeenCalledTimes(2);
  documentStub.visibilityState = 'hidden';
  await vi.advanceTimersByTimeAsync(300_000);
  expect(fetchFeedback).toHaveBeenCalledTimes(2);
  completed = true;
  documentStub.visibilityState = 'visible';
  listeners.get('visibilitychange')!();
  await vi.advanceTimersByTimeAsync(0);
  expect(fetchFeedback).toHaveBeenCalledTimes(3);
  expect(service.loadSkyBoostCompletion('a', service.loadSkyBoostBinding('a'))).toBeDefined();
  await vi.advanceTimersByTimeAsync(600_000);
  listeners.get('visibilitychange')!();
  await vi.advanceTimersByTimeAsync(0);
  expect(fetchFeedback).toHaveBeenCalledTimes(3);
  stop();
  expect(listeners.size).toBe(0);
  // 使用真正重新加载的模块和持久化记录，不能靠上次请求的内存结果停查。
  vi.resetModules();
  ({ workspace, core, service, progress, tabs } = await setup());
  await progress.hydrateProgress();
  const stopRestarted = workspace.startSkyBoostSync();
  await vi.advanceTimersByTimeAsync(300_000);
  expect(fetchFeedback).toHaveBeenCalledTimes(3);
  await workspace.syncSkyBoost(true);
  expect(fetchFeedback).toHaveBeenCalledTimes(4);
  stopRestarted();
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), included: { quests: true, candles: true, event: true } });
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(5);
  await service.saveSkyBoostBinding('a', { ...service.loadSkyBoostBinding('a'), link: link.replace('test-order', 'next-order') });
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(6);
  core.activeCharacter.value = 'b';
  await service.saveSkyBoostBinding('b', service.loadSkyBoostBinding('a'));
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(7);
  vi.setSystemTime(new Date('2026-11-07T00:01:00+08:00'));
  tabs.skyDailyData.value = { ...daily(), dateStr: '2026-11-07' };
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(8);
  await workspace.syncSkyBoost();
  expect(fetchFeedback).toHaveBeenCalledTimes(8);
});
