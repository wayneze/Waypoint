import { expect, test } from 'vitest';
import { buildSteamRows, selectSteamRows } from './steamImport';
import type { GameRecord, ObsidianGameGuide, SteamLibraryGame } from '../types';
import type { SteamActivity } from './gameActivity';

const game = (appid: number, name: string, minutes = 0, recent = 0): SteamLibraryGame => ({
  vault_item_id: 'account', appid, name, playtime_forever: minutes, rtime_last_played: recent,
});
const record: GameRecord = { id: 'own', title: '我的石头门', category: 'galgame', status: 'playing', progress: '第三章', rating: 9 };
const activity: SteamActivity = { links: [{ record_id: 'own', appid: 1 }], installed: [{ appid: 2, directory: 'C:/game' }], accounts: [],
  titles: { 1: { title: '命运石之门', checked_at: 1 }, 2: { title: '群星', checked_at: 1 } } };

test('library and install sorting compose with bilingual search and selected-only filtering', () => {
  const rows = buildSteamRows([game(2, 'Stellaris', 500, 200), game(1, 'STEINS;GATE', 20, 100), game(3, 'Unknown')], [record], [], activity);
  expect(selectSteamRows(rows, '', 'all', 'all', 'library', false, []).map(r => r.game.appid)).toEqual([1, 2, 3]);
  expect(selectSteamRows(rows, '', 'all', 'all', 'installed', false, [1]).map(r => r.game.appid)).toEqual([2, 1, 3]);
  expect(selectSteamRows(rows, '群星', 'new', 'installed', 'library', false, []).map(r => r.game.appid)).toEqual([2]);
  expect(selectSteamRows(rows, 'steins', 'library', 'all', 'name', true, [1, 2]).map(r => r.game.appid)).toEqual([1]);
  expect(selectSteamRows(rows, '', 'unlinked', 'all', 'name', true, [1])).toEqual([]);
  expect(rows[1]?.title).toBe('我的石头门');
  expect(record.title).toBe('我的石头门');
});

test('identity claims distinguish a title suggestion, conflicting sequel and guide-only library item', () => {
  const guide = { id: 'guide', title: '攻略里的中文名', aliases: [], links: { steam: 'https://store.steampowered.com/app/3/' } } as unknown as ObsidianGameGuide;
  const other = { ...record, id: 'other', title: 'Stellaris（群星）' };
  const conflicting = { ...other, id: 'conflict' };
  const rows = buildSteamRows([game(1, 'STEINS;GATE'), game(2, 'Stellaris'), game(3, 'New game')], [{ ...record, title: 'STEINS;GATE' }, other, conflicting], [guide],
    { ...activity, links: [...activity.links, { record_id: 'conflict', appid: 99 }] });
  expect(rows[0]?.title).toBe('命运石之门');
  expect(rows[1]?.record).toBeUndefined();
  expect(rows[1]?.suggestions.map(r => r.id)).toEqual(['other']);
  expect(rows[1]?.inLibrary).toBe(true);
  expect(rows[1]?.linked).toBe(false);
  expect(rows[2]).toMatchObject({ title: '攻略里的中文名', inLibrary: true, linked: false, guide });
});
