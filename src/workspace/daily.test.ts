import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { CustomMemo, TaskItem } from '../types';

beforeEach(() => {
  vi.resetModules();
  const storage = new Map<string, string>();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
  });
  vi.stubGlobal('window', { location: { search: '?tab=daily' } });
});
afterEach(() => vi.unstubAllGlobals());

async function setup() {
  const core = await import('./core');
  core.vaultList.value = [{
    id: 'account', platform: '剑网3', account_name: '测试', assets: [],
    characters: ['a', 'b'].map(id => ({ id, name: '角色' + id, assets: [], track_daily: true })),
  }];
  const task = (id: string, character_id: string, done = false): TaskItem => ({
    id, character_id, title: id, category: 'daily', done, reset_rule: '07:00', source_type: 'custom',
  });
  core.allAccountTasks.value = { a: [task('任务甲', 'a'), task('已完成任务', 'a', true)], b: [task('任务乙', 'b')] };
  const memos = await import('./memos');
  const memo = (id: string, character_id?: string): CustomMemo => ({
    id, title: id, character_id, tag: '生活', cycle_type: 'daily', has_progress: false,
    current_progress: 0, target_progress: 0, done: false, created_at: '2026-11-12',
  });
  memos.customMemos.value = [memo('公共记事'), memo('角色记事', 'a')];
  const home = await import('./daily');
  return { core, memos, home };
}

test('总览、单角色与侧栏使用同一份日常计数，公共记事只出现在总览', async () => {
  const { home } = await setup();
  const all = home.homeSectionsFor()[0];
  expect({ total: all.total, remaining: all.remaining, done: all.done }).toEqual({ total: 5, remaining: 4, done: 1 });
  expect(home.homeMatrixStats.value).toMatchObject({ totalDaily: 5, remainingDaily: 4, doneDaily: 1 });
  expect(home.homeSectionsFor('a')[0]).toMatchObject({ total: 3, remaining: 2, done: 1 });
  expect(home.homeCharStats('a')).toMatchObject({ dailyTotal: 3, dailyRemaining: 2, dailyDone: 1 });
  expect(home.homeSectionsFor('b')[0].groups.flatMap(group => group.lines).map(line => line.title)).toEqual(['任务乙']);
});

test('完成项可回看与搜索，状态变化只移动同一条记录，关闭游戏后隐藏所属事项', async () => {
  const { core, memos, home } = await setup();
  expect(home.homeSectionsFor('all', '已完成任务')[0]).toMatchObject({ total: 1, remaining: 0, done: 1 });
  memos.customMemos.value[1].done = true;
  let section = home.homeSectionsFor('a')[0];
  expect(section).toMatchObject({ total: 3, remaining: 1, done: 2 });
  expect(section.completed.map(line => line.title)).toContain('角色记事');
  core.allAccountTasks.value.a[0].done = true;
  section = home.homeSectionsFor('a')[0];
  expect(section).toMatchObject({ total: 3, remaining: 0, done: 3 });
  const games = await import('../services/games');
  games.setGameEnabled('jx3', false);
  expect(home.homeSectionsFor()[0].groups.flatMap(group => group.lines).map(line => line.title)).toEqual(['公共记事']);
});
