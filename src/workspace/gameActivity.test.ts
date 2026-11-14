import { expect, test, vi } from 'vitest';
import { ref } from 'vue';
vi.mock('./core', () => ({ gameRecords: ref([]), showToast: vi.fn() }));
vi.mock('./vault', () => ({ steamLibrary: ref([{ appid: 1, name: 'First' }, { appid: 2, name: 'Second' }]), syncSteamAccount: vi.fn(async () => []) }));
vi.mock('../services/api', () => ({ isTauri: () => true, getGameRecords: vi.fn(async () => []) }));
vi.mock('../services/gameActivity', async original => ({
  ...await original<typeof import('../services/gameActivity')>(),
  importSteamGames: vi.fn(async () => ['one']),
  refreshSteamAchievements: vi.fn(),
  getSteamActivity: vi.fn(async () => ({ links: [], accounts: [], installed: [], titles: {} })),
  getLocalPlaytime: vi.fn(async () => []),
}));

test('achievement-only sync never imports; closing and cancelling preserves the current request and stops the queue', async () => {
  const service = await import('../services/gameActivity');
  const workspace = await import('./gameActivity');
  let finish!: (value: Awaited<ReturnType<typeof service.refreshSteamAchievements>>) => void;
  vi.mocked(service.refreshSteamAchievements).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
  workspace.showSteamImport.value = true;
  const running = workspace.runSteamBatch('account', [{ appid: 1 }, { appid: 2 }], 'sync');
  await vi.waitFor(() => expect(service.refreshSteamAchievements).toHaveBeenCalledOnce());
  expect(workspace.steamBatch.value.running).toBe(true);
  workspace.showSteamImport.value = false;
  expect(workspace.steamBatch.value.running).toBe(true);
  await expect(workspace.runSteamBatch('account', [{ appid: 1 }], 'import')).rejects.toThrow();
  workspace.stopSteamBatch();
  finish({ appid: 1, status: 'ready', items: [] });
  await running;
  expect(service.importSteamGames).not.toHaveBeenCalled();
  expect(service.refreshSteamAchievements).toHaveBeenCalledTimes(1);
  expect(workspace.steamBatch.value).toMatchObject({ running: false, stopped: true, done: 1, total: 2, imported: 0 });
  vi.mocked(service.refreshSteamAchievements).mockClear();
  vi.mocked(service.refreshSteamAchievements).mockResolvedValue({ appid: 2, status: 'unsupported', items: [] });
  await workspace.runSteamBatch('account', [{ appid: 2 }], 'import');
  expect(service.importSteamGames).toHaveBeenCalledOnce();
  expect(service.refreshSteamAchievements).toHaveBeenCalledWith('account', 2);
  expect(workspace.steamBatch.value).toMatchObject({ running: false, imported: 1, total: 1, done: 1, synced: 1, errors: [] });
});

test('a saved association survives time or achievement failure and reports partial results', async () => {
  const service = await import('../services/gameActivity');
  const vault = await import('./vault');
  const workspace = await import('./gameActivity');
  vi.mocked(vault.syncSteamAccount).mockRejectedValueOnce(new Error('timeout'));
  vi.mocked(service.refreshSteamAchievements).mockResolvedValueOnce({ appid: 1, status: 'private', error: 'Profile is not public', items: [] });
  await workspace.runSteamBatch('account', [{ appid: 1 }], 'import');
  expect(workspace.steamBatch.value).toMatchObject({ running: false, imported: 1, total: 1, done: 1, synced: 0 });
  expect(workspace.steamBatch.value.errors).toHaveLength(2);
});
