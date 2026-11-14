import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import type { TaskItem, VaultItem } from '../types';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', async importOriginal => ({
  ...await importOriginal<typeof import('@tauri-apps/api/core')>(),
  invoke,
}));

let storage: Map<string, string>;
const roster: VaultItem[] = [
  {
    id: 'jx3-account', platform: '剑网3', account_name: '江湖', assets: [],
    characters: [
      { id: 'jx3-a', name: '剑三一号', assets: [], track_daily: true },
      { id: 'jx3-b', name: '剑三二号', assets: [], track_daily: true },
    ],
  },
  {
    id: 'sky-account', platform: '光遇', account_name: '旅人', assets: [],
    characters: [
      { id: 'sky-a', name: '光遇一号', assets: [], track_daily: true },
      { id: 'sky-b', name: '光遇二号', assets: [], track_daily: false },
    ],
  },
];

beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  storage = new Map();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
    removeItem: (key: string) => storage.delete(key),
  });
  vi.stubGlobal('window', { location: { search: '' } });
});

afterEach(() => vi.unstubAllGlobals());

async function openWorkspace() {
  const core = await import('./core');
  core.vaultList.value = structuredClone(roster);
  const { nextTick } = await import('vue');
  await nextTick();
  return core;
}

test('游戏页面自动恢复各自角色，同步任务并保留未挂日常板的选择', async () => {
  const core = await openWorkspace();
  const task: TaskItem = {
    id: 'sky-task', character_id: 'sky-b', title: '跑图', category: 'daily',
    reset_rule: '00:00', source_type: 'custom', done: false,
  };
  core.allAccountTasks.value = { 'sky-b': [task] };
  core.currentTab.value = 'jx3';
  core.activeCharacter.value = 'jx3-b';
  core.currentTab.value = 'sky';
  expect(core.activeCharacter.value).toBe('sky-a');
  core.activeCharacter.value = 'sky-b';
  core.currentTab.value = 'jx3';
  expect(core.activeCharacter.value).toBe('jx3-b');
  expect(core.state.dailyTasks).toEqual([]);
  core.currentTab.value = 'sky';
  expect(core.activeCharacter.value).toBe('sky-b');
  expect(core.activeCharObj.value?.name).toBe('光遇二号');
  expect(core.state.dailyTasks.map(row => row.id)).toEqual(['sky-task']);
  expect(core.characterChoices.value.map(row => row.id)).toEqual(['sky-a', 'sky-b']);
  expect(core.characterList.value.map(row => row.id)).not.toContain('sky-b');
});

test('重启及桌面设置延迟加载后仍恢复各游戏选择，并沿用已有单角色设置', async () => {
  storage.set('waypoint_active_character', 'jx3-b');
  let core = await openWorkspace();
  expect(core.activeCharacter.value).toBe('jx3-b');
  core.currentTab.value = 'sky';
  core.activeCharacter.value = 'sky-b';
  core.currentTab.value = 'jx3';
  const savedSettings = Object.fromEntries(storage);
  vi.resetModules();
  vi.stubGlobal('window', { location: { search: '?tab=sky' }, __TAURI_INTERNALS__: {} });
  invoke.mockImplementation(async (command: string) => command === 'get_all_settings' ? savedSettings : null);
  core = await import('./core');
  const { hydrateKv } = await import('../services/kv');
  await hydrateKv();
  core.vaultList.value = structuredClone(roster);
  const { nextTick } = await import('vue');
  await nextTick();
  expect(core.activeCharacter.value).toBe('sky-b');
  core.currentTab.value = 'jx3';
  expect(core.activeCharacter.value).toBe('jx3-b');
});

test('角色删除或游戏没有角色时只使用本游戏有效角色，空名册清空任务', async () => {
  const core = await openWorkspace();
  core.currentTab.value = 'sky';
  core.activeCharacter.value = 'sky-b';
  core.vaultList.value[1].characters.pop();
  const { nextTick } = await import('vue');
  await nextTick();
  expect(core.activeCharacter.value).toBe('sky-a');
  core.vaultList.value = core.vaultList.value.filter(account => account.id !== 'sky-account');
  await nextTick();
  expect(core.activeCharacter.value).toBe('');
  expect(core.characterChoices.value).toEqual([]);
  expect(core.state.dailyTasks).toEqual([]);
  core.currentTab.value = 'jx3';
  expect(core.activeCharacter.value).toBe('jx3-a');
  core.vaultList.value = [];
  await nextTick();
  expect(core.activeCharacter.value).toBe('');
});

test('返回今日单角色视图恢复聚焦角色，公共页面选择与游戏记忆一致', async () => {
  const core = await openWorkspace();
  core.selectScope('jx3-b');
  core.currentTab.value = 'sky';
  expect(core.activeCharacter.value).toBe('sky-a');
  expect(core.selectedScope.value).toBe('jx3-b');
  core.currentTab.value = 'daily';
  expect(core.activeCharacter.value).toBe('jx3-b');
  core.activeCharacter.value = 'sky-a';
  expect(core.selectedScope.value).toBe('sky-a');
  core.selectScope('all');
  core.currentTab.value = 'vault';
  core.activeCharacter.value = 'sky-b';
  core.currentTab.value = 'jx3';
  core.currentTab.value = 'sky';
  expect(core.activeCharacter.value).toBe('sky-b');
  core.currentTab.value = 'daily';
  expect(core.selectedScope.value).toBe('all');
});

test('错误通知保留到关闭，普通提示仍自动消失', async () => {
  vi.useFakeTimers();
  try {
    const { showToast, toast } = await import('./core');
    showToast('已读取', '书库已打开');
    showToast('操作失败', '完整的错误原因', { persistent: true });
    await vi.advanceTimersByTimeAsync(10000);
    expect(toast.show).toBe(true);
    expect(toast.body).toBe('完整的错误原因');
    toast.show = false;
    showToast('已保存', '新设置已保存');
    await vi.advanceTimersByTimeAsync(4000);
    expect(toast.show).toBe(false);
  } finally { vi.useRealTimers(); }
});

test('角色待办新增、改名和删除保存失败时保留原始清单与录入内容', async () => {
  const core = await openWorkspace();
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
  const task: TaskItem = {
    id: 'task', character_id: 'jx3-a', title: '原来的事项', category: 'daily',
    reset_rule: '07:00', source_type: 'custom', done: false,
  };
  core.allAccountTasks.value = { 'jx3-a': [task] };
  core.charQuickAddInputs['jx3-a'] = '待保存事项';
  invoke.mockRejectedValue(new Error('disk full'));
  await expect(core.handleCharQuickAdd('jx3-a')).rejects.toThrow('disk full');
  expect(core.charQuickAddInputs['jx3-a']).toBe('待保存事项');
  expect(core.allAccountTasks.value['jx3-a']).toHaveLength(1);
  await expect(core.handleRenameTask(task, '新名称', 'jx3-a')).rejects.toThrow('disk full');
  expect(task.title).toBe('原来的事项');
  await expect(core.handleDeleteMatrixTask(task.id, 'jx3-a')).rejects.toThrow('disk full');
  expect(core.allAccountTasks.value['jx3-a']).toHaveLength(1);
});
