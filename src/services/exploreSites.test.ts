import { expect, test, vi } from 'vitest';
import type { ObsidianGameGuide } from '../types';

const storage = vi.hoisted(() => ({ value: null as unknown, migrated: false }));
const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({ invoke }));
vi.mock('./kv', () => ({
  kvGetJson: (key: string, fallback: unknown) => key === 'waypoint_explore_sites' ? storage.value : storage.migrated || fallback,
  kvSetJson: (key: string, value: unknown) => { if (key === 'waypoint_explore_sites') storage.value = value; else storage.migrated = value === true; },
}));
import { loadExploreSites, siteUrl, isHttp, exploreTerms, saveExploreSites } from './exploreSites';
import { resolveJuijDownload, juijFailure } from './juijDownloads';

test('Juij distinguishes share pages, rejects unresolved HTML and preserves server cooldown', async () => {
  const page = 'https://i.juij.fun/SCSKiller?jump=download@SCSKiller';
  invoke.mockResolvedValueOnce({ url: page, destination: 'https://pan.quark.cn/s/example', bytes: [] });
  expect(await resolveJuijDownload(page)).toEqual({ url: 'https://pan.quark.cn/s/example', kind: 'share' });
  invoke.mockResolvedValueOnce({ url: page, destination: 'https://files.example/tool.zip', bytes: [] });
  expect(await resolveJuijDownload(page)).toMatchObject({ kind: 'external' });
  invoke.mockResolvedValueOnce({ url: 'https://q.juij.fun/qrcode/?qrcode=SCSKiller', bytes: [] });
  await expect(resolveJuijDownload(page)).rejects.toThrow('未取得外部下载地址');
  const failure = { kind: 'rate_limited', message: '站点已限流', retryAt: Date.now() + 600_000 };
  invoke.mockRejectedValueOnce(failure);
  await expect(resolveJuijDownload(page).catch(cause => { throw juijFailure(cause); })).rejects.toEqual(failure);
  invoke.mockResolvedValueOnce({ url: page, destination: 'https://user:secret@files.example/tool.zip', bytes: [] });
  await expect(resolveJuijDownload(page)).rejects.toThrow('未取得外部下载地址');
});

test('published site addresses update without overwriting personal choices or restoring deleted sites', () => {
  storage.value = null;
  const defaults = loadExploreSites();
  const zi = defaults.find(site => site.id === 'zi0')!;
  storage.value = [
    { ...zi, home: 'https://zi6.cc/', search: 'https://zi6.cc/?s={q}', result: 'alist', hidden: true, order: 25 },
    { ...defaults[0], home: 'https://custom.example/', search: 'https://custom.example/?q={q}', order: 40 },
  ];
  const sites = loadExploreSites();
  expect(sites).toHaveLength(2);
  expect(sites[0]).toMatchObject({ home: 'https://gal.games/', result: 'galgames', hidden: true, order: 25 });
  expect(siteUrl(sites[0]!, 'Fate / 命运')).toBe('https://gal.games/search?q=Fate%20%2F%20%E5%91%BD%E8%BF%90');
  expect(sites[1]).toMatchObject({ home: 'https://custom.example/', search: 'https://custom.example/?q={q}', order: 40 });
  storage.value = [];
  expect(loadExploreSites()).toEqual([]);
});

test('Steam guide links use the game community and invalid URLs are rejected', () => {
  storage.value = null;
  const site = loadExploreSites().find(item => item.id === 'steam-guides')!;
  const game: ObsidianGameGuide = {
    id: 'portal', title: 'Portal 2', aliases: [], category: 'single', status: '在玩', rating: 0,
    tags: [], genres: [], file_path: '', relative_path: '', file_name: '', modified: '',
    links: { steam: 'https://store.steampowered.com/app/620/Portal_2/' },
  };
  expect(siteUrl(site, 'Portal', game)).toBe('https://steamcommunity.com/app/620/guides/');
  expect(siteUrl(site, 'Portal')).toBe('https://steamcommunity.com/?subsection=guides');
  expect(isHttp('https://')).toBe(false);
  expect(isHttp('javascript:alert(1)')).toBe(false);
});

test('guide sites choose language aliases and migrate once without restoring removals', () => {
  storage.value = null; storage.migrated = false;
  const sites = loadExploreSites();
  const japanese = sites.find(s => s.id === 'seiya')!;
  const chinese = sites.find(s => s.id === '2dfan-guides')!;
  const guide = { title: 'STEINS;GATE (命运石之门)', original_title: 'STEINS;GATE', aliases: ['Steins Gate', 'シュタインズ・ゲート'] } as ObsidianGameGuide;
  expect(exploreTerms(japanese, 'STEINS;GATE', guide)[0]).toBe('シュタインズ・ゲート');
  expect(exploreTerms(chinese, 'STEINS;GATE', guide)[0]).toBe('命运石之门');
  expect(exploreTerms(japanese, '手动关键词', guide)).toEqual(['手动关键词']);
  saveExploreSites([japanese]);
  expect(loadExploreSites().map(s => s.id)).toEqual(['seiya']);
  storage.migrated = false;
  storage.value = [{ ...japanese, home: 'https://seiya-saiga.com/game/kouryaku.html', search: 'https://seiya-saiga.com/game/kouryaku.html', hidden: true }];
  expect(loadExploreSites()).toEqual([expect.objectContaining({ ...japanese, hidden: true }), chinese]);
  saveExploreSites([]);
  expect(loadExploreSites()).toEqual([]);
});
