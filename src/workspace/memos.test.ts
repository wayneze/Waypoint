import { beforeEach, expect, test, vi } from 'vitest';
import type { CustomMemo } from '../types';

const api = vi.hoisted(() => ({ add: vi.fn(), update: vi.fn() }));
vi.mock('../services/api', () => ({
  addCustomMemo: api.add, updateCustomMemo: api.update,
  deleteCustomMemo: vi.fn(), toggleCustomMemo: vi.fn(),
}));
vi.mock('./core', () => ({
  gameResetEpoch: { value: 0 }, newTaskId: () => 'm_test', showToast: vi.fn(),
}));
vi.mock('../services/kv', () => ({ kvSet: vi.fn(), kvGet: () => null, kvGetJson: (_key: string, fallback: unknown) => fallback }));

import { createMemo, customMemos, handleToggleMemoStep, handleUpdateMemo } from './memos';

function memo(): CustomMemo {
  return {
    id: 'm1', title: '原始记事', tag: '生活', cycle_type: 'weekly', cycle_val: '1,5',
    note: '- [ ] 准备行李', character_id: 'c1',
    has_progress: false, current_progress: 0, target_progress: 0, done: false,
    created_at: '2026-11-12 00:00:00',
  };
}
beforeEach(() => {
  vi.resetAllMocks();
  customMemos.value = [];
});

test('保存失败保留原始事项；重试成功后才发布全部编辑字段', async () => {
  const original = memo();
  const patch = { title: '新的标题', cycle_type: 'once' as const, cycle_val: '', note: '', character_id: '' };
  api.update.mockRejectedValueOnce(new Error('disk full'));
  expect(await handleUpdateMemo(original, patch)).toBe(false);
  expect(original).toEqual(memo());
  api.update.mockResolvedValueOnce(undefined);
  expect(await handleUpdateMemo(original, patch)).toBe(true);
  expect(original).toMatchObject({ title: '新的标题', cycle_type: 'once', done: false });
  expect(original.cycle_val).toBeUndefined();
  expect(original.note).toBeUndefined();
  expect(original.character_id).toBeUndefined();
});

test('步骤写入失败不提前完成事项，成功后步骤与完成状态一起发布', async () => {
  const original = memo();
  api.update.mockRejectedValueOnce(new Error('disk full'));
  await expect(handleToggleMemoStep(original, 0)).rejects.toThrow('disk full');
  expect(original).toEqual(memo());
  api.update.mockResolvedValueOnce(undefined);
  await handleToggleMemoStep(original, 0);
  expect(original.note).toBe('- [x] 准备行李');
  expect(original.done).toBe(true);
  expect(original.last_completed_at).toBeTruthy();
});

test('原生新增事件先到达时，新增返回不再插入重复事项', async () => {
  api.add.mockImplementation(async (added: CustomMemo) => { customMemos.value.unshift(added); });
  expect(await createMemo(memo())).toBe(true);
  expect(customMemos.value).toHaveLength(1);
  expect(customMemos.value[0].id).toBe('m_test');
});
