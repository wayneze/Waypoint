import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { VaultItem } from '../../types';
import type { SkyRoleWings } from '../../services/skyApi';

const fetchWings = vi.hoisted(() => vi.fn());
const invoke = vi.hoisted(() => vi.fn());
vi.mock('../../services/skyApi', async importOriginal => ({
  ...await importOriginal<typeof import('../../services/skyApi')>(),
  fetchSkyRoleWings: fetchWings,
}));
vi.mock('@tauri-apps/api/core', async importOriginal => ({
  ...await importOriginal<typeof import('@tauri-apps/api/core')>(),
  invoke,
}));

const roster: VaultItem[] = [{
  id: 'sky-account', platform: '光遇', account_name: '测试账号', assets: [],
  characters: [
    { id: 'sky-a', name: '晨岛号', query_id: 'queryA', assets: [], track_daily: true },
    { id: 'sky-b', name: '雨林号', query_id: 'queryB', assets: [], track_daily: false },
  ],
}];

beforeEach(() => {
  vi.resetModules();
  fetchWings.mockReset();
  invoke.mockReset();
  vi.useFakeTimers();
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
  });
  vi.stubGlobal('window', { location: { search: '' } });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function openAtlas(useRoster = roster) {
  const progress = await import('../../services/progress');
  await progress.hydrateProgress();
  const core = await import('../core');
  core.vaultList.value = structuredClone(useRoster);
  const { nextTick } = await import('vue');
  await nextTick();
  const atlas = await import('./atlas');
  return { core, atlas, progress };
}

test('探索打卡、献祭重置和重启按角色隔离，包括未挂日常板的角色', async () => {
  let { core, atlas, progress } = await openAtlas();
  core.activeCharacter.value = 'sky-a';
  await atlas.toggleAtlasLight('light-a');
  await atlas.toggleAtlasShrine('shrine-a');
  await atlas.toggleAtlasAction('action-a');
  core.activeCharacter.value = 'sky-b';
  expect(atlas.isAtlasLightChecked('light-a')).toBe(false);
  await atlas.toggleAtlasLight('light-b');
  core.activeCharacter.value = 'sky-a';
  await atlas.resetAtlasLightsOnly();
  expect(atlas.isAtlasLightChecked('light-a')).toBe(false);
  expect(atlas.isAtlasShrineChecked('shrine-a')).toBe(true);
  expect(atlas.isAtlasActionChecked('action-a')).toBe(true);
  expect(progress.progressEntries('sky.atlas.lights', '')).toEqual([]);

  core.activeCharacter.value = 'sky-b';
  const { nextTick } = await import('vue');
  await nextTick();
  await progress.progressFlush();
  vi.resetModules();
  ({ core, atlas, progress } = await openAtlas());
  expect(core.activeCharacter.value).toBe('sky-b');
  expect(core.activeCharObj.value?.name).toBe('雨林号');
  expect(core.characterList.value.map(ch => ch.id)).toEqual(['sky-a']);
  expect(atlas.isAtlasLightChecked('light-b')).toBe(true);
  expect(atlas.isAtlasShrineChecked('shrine-a')).toBe(false);
  core.activeCharacter.value = 'sky-a';
  expect(atlas.isAtlasLightChecked('light-a')).toBe(false);
  expect(atlas.isAtlasActionChecked('action-a')).toBe(true);

  core.activeCharacter.value = 'jx3-other';
  await atlas.toggleAtlasLight('no-owner');
  expect(progress.progressEntries('sky.atlas.lights', '')).toEqual([]);
  expect(atlas.atlasQueryId.value).toBe('');
  await atlas.syncAtlasFromDashen();
  expect(fetchWings).not.toHaveBeenCalled();
});

test('大神请求绑定发起角色，切换后不污染当前角色；变更身份或失败不写入', async () => {
  const { core, atlas, progress } = await openAtlas();
  const { ATLAS_LIGHTS } = await import('../../services/skyAtlas');
  const { default: wingKeys } = await import('../../content/sky/wing-keys.json');
  const [key, lightId] = Object.entries(wingKeys).find(([, id]) => ATLAS_LIGHTS.some(l => l.id === id))!;
  const snapshot: SkyRoleWings = {
    success: true, role_id: 'queryA', source: 'test', collected: 1, total: 2,
    maps: [{ name: '晨岛', collected: 1, total: 2 }],
    buffs: [{ name: key, collected: true }], error: '',
  };
  let finish!: (value: SkyRoleWings) => void;
  fetchWings.mockImplementation(() => new Promise<SkyRoleWings>(resolve => { finish = resolve; }));
  core.activeCharacter.value = 'sky-a';
  await atlas.toggleAtlasShrine('shrine-a');
  const unreportedId = ATLAS_LIGHTS.find(l => l.id !== lightId)!.id;
  await atlas.toggleAtlasLight(unreportedId);
  const syncing = atlas.syncAtlasFromDashen();
  expect(fetchWings).toHaveBeenCalledWith('queryA');
  expect(atlas.atlasCanEdit.value).toBe(false);
  await atlas.resetAtlasLightsOnly();
  core.activeCharacter.value = 'sky-b';
  await atlas.toggleAtlasLight('light-b');
  finish(snapshot);
  await syncing;
  expect(atlas.isAtlasLightChecked('light-b')).toBe(true);
  expect(atlas.isAtlasLightChecked(lightId)).toBe(false);
  expect(atlas.atlasDashenMaps.value).toEqual([]);
  expect(atlas.atlasDashenSummary.value).toBe('');
  core.activeCharacter.value = 'sky-a';
  expect(atlas.isAtlasLightChecked(lightId)).toBe(true);
  expect(atlas.isAtlasShrineChecked('shrine-a')).toBe(true);
  expect(atlas.isAtlasLightChecked(unreportedId)).toBe(true);
  const clearingReported = atlas.syncAtlasFromDashen();
  finish({ ...snapshot, buffs: [{ name: key, collected: false }] });
  await clearingReported;
  expect(atlas.isAtlasLightChecked(lightId)).toBe(false);
  expect(atlas.isAtlasLightChecked(unreportedId)).toBe(true);

  const before = progress.progressEntries('sky.atlas.lights', 'sky-a');
  const stale = atlas.syncAtlasFromDashen();
  core.vaultList.value[0].characters[0].query_id = 'changed';
  finish({ ...snapshot, buffs: [{ name: key, collected: false }] });
  await stale;
  expect(progress.progressEntries('sky.atlas.lights', 'sky-a')).toEqual(before);
  fetchWings.mockRejectedValueOnce(new Error('offline'));
  await atlas.syncAtlasFromDashen();
  expect(progress.progressEntries('sky.atlas.lights', 'sky-a')).toEqual(before);
  expect(atlas.isSyncingAtlasDashen.value).toBe(false);

  const removed = atlas.syncAtlasFromDashen();
  core.vaultList.value[0].characters.splice(0, 1);
  finish({ ...snapshot, buffs: [{ name: key, collected: false }] });
  await removed;
  expect(progress.progressEntries('sky.atlas.lights', 'sky-a')).toEqual(before);
});

test('旧共享记录不自动分配，显式归入只合并目标角色并在重启后保持', async () => {
  const progress = await import('../../services/progress');
  progress.progressSet('sky.atlas.lights', 'legacy-light', true);
  progress.progressSet('sky.atlas.shrines', 'legacy-shrine', true);
  progress.progressSet('sky.atlas.actions', 'legacy-action', true);
  // 即便只有一个角色，历史数据也不猜测归属。
  const { core, atlas } = await openAtlas([{ ...roster[0], characters: [roster[0].characters[0]] }]);
  expect(atlas.atlasLegacyCount.value).toBe(3);
  expect(atlas.isAtlasLightChecked('legacy-light')).toBe(false);
  await atlas.toggleAtlasLight('existing-light');
  await atlas.claimAtlasLegacy();
  expect(atlas.isAtlasLightChecked('legacy-light')).toBe(true);
  expect(atlas.isAtlasLightChecked('existing-light')).toBe(true);
  expect(atlas.atlasLegacyCount.value).toBe(0);
  core.vaultList.value = structuredClone(roster);
  core.activeCharacter.value = 'sky-b';
  expect(atlas.isAtlasLightChecked('legacy-light')).toBe(false);
  await atlas.claimAtlasLegacy(); // 不允许再次归属到另一个角色。
  expect(progress.progressEntries('sky.atlas.lights', 'sky-b')).toEqual([]);
  await progress.progressFlush();
  vi.resetModules();
  const restarted = await openAtlas();
  restarted.core.activeCharacter.value = 'sky-a';
  expect(restarted.atlas.isAtlasActionChecked('legacy-action')).toBe(true);
  expect(restarted.atlas.isAtlasLightChecked('existing-light')).toBe(true);
  expect(restarted.atlas.atlasLegacyCount.value).toBe(0);
});

test('旧记录归属在桌面端使用单次事务，落库失败不丢共享记录且可重试', async () => {
  const progress = await import('../../services/progress');
  progress.progressSet('sky.atlas.lights', 'legacy-light', true);
  progress.progressSet('sky.atlas.actions', 'legacy-action', true);
  const { atlas } = await openAtlas();
  await atlas.toggleAtlasLight('existing-light');
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
  invoke.mockRejectedValueOnce(new Error('database is locked'));
  await atlas.claimAtlasLegacy();
  expect(atlas.atlasLegacyCount.value).toBe(2);
  expect(progress.progressMap('sky.atlas.lights', '')).toEqual({ 'legacy-light': true });
  expect(progress.progressMap('sky.atlas.lights', 'sky-a')).toEqual({ 'existing-light': true });
  expect(atlas.isClaimingAtlasLegacy.value).toBe(false);
  expect(invoke).toHaveBeenCalledTimes(1);
  const [command, args] = invoke.mock.calls[0];
  expect(command).toBe('apply_progress_ops');
  expect(args.ops).toEqual([
    { op: 'put', scope: 'sky.atlas.lights', character_id: 'sky-a', item_id: 'legacy-light', state: true },
    { op: 'del', scope: 'sky.atlas.lights', character_id: '', item_id: 'legacy-light' },
    { op: 'put', scope: 'sky.atlas.actions', character_id: 'sky-a', item_id: 'legacy-action', state: true },
    { op: 'del', scope: 'sky.atlas.actions', character_id: '', item_id: 'legacy-action' },
  ]);
  invoke.mockResolvedValueOnce(undefined);
  await atlas.claimAtlasLegacy();
  expect(atlas.atlasLegacyCount.value).toBe(0);
  expect(atlas.isAtlasLightChecked('legacy-light')).toBe(true);
  expect(atlas.isAtlasLightChecked('existing-light')).toBe(true);
});

test('大神同步等待落库成功，失败保留勾选并允许重试，保存期间切角色不串显示', async () => {
  const { core, atlas, progress } = await openAtlas();
  const { ATLAS_LIGHTS } = await import('../../services/skyAtlas');
  const { default: wingKeys } = await import('../../content/sky/wing-keys.json');
  const [key, lightId] = Object.entries(wingKeys).find(([, id]) => ATLAS_LIGHTS.some(l => l.id === id))!;
  fetchWings.mockResolvedValue({
    success: true, role_id: 'queryA', source: 'test', collected: 1, total: 1,
    maps: [{ name: '晨岛', collected: 1, total: 1 }],
    buffs: [{ name: key, collected: true }], error: '',
  });
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
  let finish!: () => void;
  let fail!: (reason: Error) => void;
  invoke.mockImplementation(() => new Promise<void>((resolve, reject) => { finish = resolve; fail = reject; }));
  const syncing = atlas.syncAtlasFromDashen();
  await vi.waitFor(() => expect(invoke).toHaveBeenCalled());
  expect(atlas.isSyncingAtlasDashen.value).toBe(true);
  expect(atlas.isAtlasLightChecked(lightId)).toBe(false);
  expect(progress.progressMap('sky.atlas.lights', 'sky-a')).toEqual({});
  expect(core.toast.title).not.toContain('已勾选');
  fail(new Error('database is locked'));
  await syncing;
  expect(core.toast.title).toContain('同步失败');
  expect(progress.progressMap('sky.atlas.lights', 'sky-a')).toEqual({});

  const retry = atlas.syncAtlasFromDashen();
  await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
  core.activeCharacter.value = 'sky-b';
  finish();
  await retry;
  expect(progress.progressMap('sky.atlas.lights', 'sky-a')).toEqual({ [lightId]: true });
  expect(atlas.isAtlasLightChecked(lightId)).toBe(false);
  expect(atlas.atlasDashenMaps.value).toEqual([]);
  expect(atlas.atlasDashenSummary.value).toBe('');
  core.activeCharacter.value = 'sky-a';
  expect(atlas.isAtlasLightChecked(lightId)).toBe(true);
  invoke.mockRejectedValueOnce(new Error('disk full'));
  await atlas.resetAtlasLightsOnly();
  expect(atlas.isAtlasLightChecked(lightId)).toBe(true);
  expect(core.toast.title).toBe('探索进度保存失败，原记录已保留');
  invoke.mockResolvedValueOnce(undefined);
  await atlas.resetAtlasLightsOnly();
  expect(atlas.isAtlasLightChecked(lightId)).toBe(false);
  expect(core.toast.title).toContain('小金人已重置');
});

test('地区清单统一承载类型、动作来源、搜索和未收筛选，筛选不改写角色进度', async () => {
  const { core, atlas } = await openAtlas();
  const { ATLAS_LIGHT_COUNT, ATLAS_SHRINE_COUNT, ATLAS_ACTION_COUNT } = await import('../../services/skyAtlas');
  const all = atlas.atlasAreaCards.value;
  expect(all.reduce((n, area) => n + area.lights.length, 0)).toBe(ATLAS_LIGHT_COUNT);
  expect(all.reduce((n, area) => n + area.shrines.length, 0)).toBe(ATLAS_SHRINE_COUNT);
  expect(all.reduce((n, area) => n + area.actions.length, 0)).toBe(ATLAS_ACTION_COUNT);

  atlas.atlasMode.value = 'lights';
  expect(atlas.atlasAreaCards.value.every(area => area.lights.length > 0 && !area.shrines.length && !area.actions.length)).toBe(true);
  const area = atlas.atlasAreaCards.value[0];
  const light = area.lights[0];
  atlas.atlasRealmFilter.value = area.realmId;
  atlas.atlasQuery.value = light.hint;
  expect(atlas.atlasAreaCards.value.flatMap(card => card.lights).map(row => row.id)).toContain(light.id);
  await atlas.toggleAtlasLight(light.id);
  atlas.atlasHideCollected.value = true;
  expect(atlas.atlasAreaCards.value.flatMap(card => card.lights).map(row => row.id)).not.toContain(light.id);
  expect(atlas.isAtlasLightChecked(light.id)).toBe(true);
  core.activeCharacter.value = 'sky-b';
  expect(atlas.atlasAreaCards.value.flatMap(card => card.lights).map(row => row.id)).toContain(light.id);

  atlas.atlasQuery.value = '';
  atlas.atlasRealmFilter.value = 'all';
  atlas.atlasMode.value = 'shrines';
  expect(atlas.atlasAreaCards.value.every(card => card.shrines.length > 0 && !card.lights.length && !card.actions.length)).toBe(true);
  atlas.atlasMode.value = 'actions';
  for (const kind of ['regular', 'season'] as const) {
    atlas.atlasActionKind.value = kind;
    const cards = atlas.atlasAreaCards.value;
    expect(cards.length).toBeGreaterThan(0);
    expect(cards.every(card => !card.shrines.length && !card.lights.length && card.actions.length > 0)).toBe(true);
    expect(cards.flatMap(card => card.actions).every(action => kind === 'regular' ? action.season === '常驻' : action.season !== '常驻')).toBe(true);
  }
  expect(atlas.atlasProgress.value.lights).toBe(0);
  core.activeCharacter.value = 'sky-a';
  expect(atlas.atlasProgress.value.lights).toBe(1);
});

test('探索展开未收位置图，已获得后收起且按角色隔离，不借用其他位置图', async () => {
  const { atlas, core } = await openAtlas();
  const { createSSRApp } = await import('vue');
  const { renderToString } = await import('vue/server-renderer');
  const { default: Panel } = await import('../../components/sky/SkyAtlasPanel.vue');
  const { ATLAS_ACTIONS } = await import('../../services/skyAtlas');
  expect(ATLAS_ACTIONS.every(action =>
    action.locationImage?.preview.startsWith('/sky/locations/') && action.locationImage.full && action.sourceUrl,
  )).toBe(true);
  atlas.atlasMode.value = 'actions';
  atlas.atlasRealmFilter.value = 'all';
  atlas.atlasQuery.value = '指路动作';
  const html = await renderToString(createSSRApp(Panel));
  expect(html).toContain('指路动作 · 位置参考');
  expect(html).toContain('晨岛通往神殿的大斜坡左侧，洞穴外面。');
  expect(html).toContain('<figure');
  expect(html).toMatch(/<details[^>]* open/);
  expect(html).not.toContain('查看动作位置');
  const sassy = ATLAS_ACTIONS.find(action => action.id === '0yGgX4Yk3c')!;
  expect(sassy.locationImage?.preview).toBe('/sky/locations/0yGgX4Yk3c.webp');
  expect(sassy.locationImage?.full).toContain('Sassy_Drifter');
  expect(sassy.locationImage?.full).not.toContain('passage');
  atlas.atlasQuery.value = '踢毽子';
  const passage = await renderToString(createSSRApp(Panel));
  expect(passage).toContain('需先在篝火旁打坐切换到夜晚');
  expect(passage).toContain('踢毽子 · 位置参考');
  expect(passage).toContain('<figure');
  expect(passage).not.toContain('暂无定位图');
  expect(atlas.atlasProgress.value.actions).toBe(0);
  const kick = ATLAS_ACTIONS.find(action => action.name === '踢毽子')!;
  await atlas.toggleAtlasAction(kick.id);
  const collected = await renderToString(createSSRApp(Panel));
  expect(collected).toContain('踢毽子 · 位置参考');
  expect(collected).not.toMatch(/<details[^>]* open/);
  expect(atlas.atlasProgress.value.actions).toBe(1);
  core.activeCharacter.value = 'sky-b';
  const otherCharacter = await renderToString(createSSRApp(Panel));
  expect(otherCharacter).toMatch(/<details[^>]* open/);
  expect(atlas.atlasProgress.value.actions).toBe(0);
});
