import { beforeEach, expect, test, vi } from 'vitest';
import { ref } from 'vue';
import type { GameRecord } from '../types';

vi.mock('./core', () => ({ gameRecords: ref([]), showToast: vi.fn() }));
vi.mock('../services/api', () => ({
  updateGameRecord: vi.fn(async (id: string, update: (record: GameRecord) => GameRecord) => update({
    id, title: id, category: 'galgame', status: 'backlog', progress: '', rating: 0,
  })),
}));
vi.mock('../services/gameSaves', () => ({
  discoverGameSaves: vi.fn(),
  createSaveBackup: vi.fn(async (id: string) => ({ id: `snapshot-${id}` })),
  uploadSaveBackup: vi.fn(),
}));
const service = await import('../services/gameSaves');
const workspace = await import('./gameSaves');
beforeEach(() => {
  vi.clearAllMocks();
  workspace.gameSaveRows.value = ['one', 'two'].map(id => ({
    id, title: id, include: true, phase: 'pending', error: '', selected: ['saves'],
    discovery: { warnings: [], candidates: [{ source: { slot: 'saves', path: `C:/Games/${id}/saves`, kind: 'directory' }, status: 'ready', automatic: true, reason: 'fixture' }] },
  }));
});
test('batch keeps successful local backups, continues after upload failure, and retries only the failed upload', async () => {
  vi.mocked(service.uploadSaveBackup).mockRejectedValueOnce(new Error('offline')).mockResolvedValue(undefined);
  await workspace.runSaveBatch('test-password', true);
  expect(workspace.gameSaveRows.value.map(r => r.phase)).toEqual(['failed', 'done']);
  expect(workspace.gameSaveRows.value[0]?.backupId).toBe('snapshot-one');
  expect(service.createSaveBackup).toHaveBeenCalledTimes(2);
  await workspace.runSaveBatch('', true, true);
  expect(service.createSaveBackup).toHaveBeenCalledTimes(2);
  expect(service.uploadSaveBackup).toHaveBeenCalledTimes(3);
  expect(workspace.gameSaveRows.value.map(r => r.phase)).toEqual(['done', 'done']);
});
test('closing the dialog retains the running batch and cancellation finishes the current game only', async () => {
  let finish!: (value: Awaited<ReturnType<typeof service.createSaveBackup>>) => void;
  vi.mocked(service.createSaveBackup).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  const running = workspace.runSaveBatch('test-password', false);
  await vi.waitFor(() => expect(service.createSaveBackup).toHaveBeenCalledOnce());
  workspace.showGameSaveManager.value = false;
  expect(workspace.saveBatch.value.running).toBe(true);
  workspace.stopSaveBatch();
  finish({ id: 'saved', created_at: 1, bytes: 10, remote: false });
  await running;
  expect(service.createSaveBackup).toHaveBeenCalledOnce();
  expect(workspace.saveBatch.value).toMatchObject({ running: false, stopped: true, done: 1, total: 2 });
});
