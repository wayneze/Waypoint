import { beforeEach, expect, test, vi } from 'vitest';

vi.mock('./core', () => ({ showToast: vi.fn(), obsidianGameGuides: {}, obsidianVaultPath: {} }));
vi.mock('./guides/library', () => ({ loadObsidianGuides: vi.fn() }));
vi.mock('../services/api', () => ({ isTauri: () => true, saveSetting: vi.fn() }));
vi.mock('../services/kv', () => ({
  kvFlush: vi.fn(async () => {}), kvGet: () => null, kvGetJson: (_: string, fallback: unknown) => fallback, kvSet: vi.fn(),
}));
vi.mock('../services/progress', () => ({ progressFlush: vi.fn(async () => {}) }));
vi.mock('../services/systemApi', () => ({
  uploadWebdavBackup: vi.fn(), cancelWebdavBackup: vi.fn(async () => {}),
  getWebdavStatus: vi.fn(async () => ({ configured: true })),
  downloadWebdavBackup: vi.fn(), importDatabase: vi.fn(), undoLastRestore: vi.fn(),
  pickOpenFile: vi.fn(),
}));

const api = await import('../services/systemApi');
const workspace = await import('./settings');
const { showToast } = await import('./core');

beforeEach(() => {
  vi.clearAllMocks();
  workspace.backupPassword.value = 'test-backup-password';
  workspace.webdavUploadResult.value = null;
});

test('上传进度、取消和失败保留在设置页，失败不清除恢复密码，重试成功后才清除', async () => {
  let rejectUpload!: (error: Error) => void;
  vi.mocked(api.uploadWebdavBackup).mockImplementationOnce(async (_password, _force, progress) => {
    progress?.({ phase: 'uploading', sent: 1024, total: 4096 });
    return await new Promise((_resolve, reject) => { rejectUpload = reject; });
  });
  const running = workspace.handleUploadWebdav();
  await vi.waitFor(() => expect(workspace.webdavUploadProgress.value?.sent).toBe(1024));
  await workspace.handleUploadWebdav();
  expect(api.uploadWebdavBackup).toHaveBeenCalledOnce();
  await workspace.handleCancelWebdav();
  expect(api.cancelWebdavBackup).toHaveBeenCalledOnce();
  expect(workspace.isCancellingWebdav.value).toBe(true);
  rejectUpload(new Error('上传连续 2 分钟没有传输进展'));
  await running;
  expect(workspace.webdavUploadResult.value).toEqual({ ok: false, message: '上传连续 2 分钟没有传输进展' });
  expect(showToast).toHaveBeenLastCalledWith('上传备份失败', '上传连续 2 分钟没有传输进展', { persistent: true });
  expect(workspace.backupPassword.value).toBe('test-backup-password');
  expect(workspace.isUploadingWebdav.value).toBe(false);
  expect(workspace.isCancellingWebdav.value).toBe(false);

  vi.mocked(api.uploadWebdavBackup).mockResolvedValueOnce({ success: true, compressed_size: 4096, sync_time: '2026-11-11 19:00:00' });
  await workspace.handleUploadWebdav();
  expect(workspace.webdavUploadResult.value).toMatchObject({ ok: true });
  expect(workspace.backupPassword.value).toBe('');
});

test('云端恢复保留真实进度和失败原因，成功后立即重载并保持锁定旧资料', async () => {
  const reload = vi.fn();
  vi.stubGlobal('window', { location: { reload } });
  let rejectDownload!: (error: Error) => void;
  vi.mocked(api.downloadWebdavBackup).mockImplementationOnce(async (_password, progress) => {
    progress?.({ phase: 'downloading', received: 1024, total: 4096 });
    return await new Promise((_resolve, reject) => { rejectDownload = reject; });
  });
  const running = workspace.handleDownloadWebdav();
  await vi.waitFor(() => expect(workspace.restoreProgress.value?.received).toBe(1024));
  expect(workspace.isRestoringDatabase.value).toBe(true);
  await workspace.handleDownloadWebdav();
  expect(api.downloadWebdavBackup).toHaveBeenCalledOnce();
  expect(reload).not.toHaveBeenCalled();
  rejectDownload(new Error('密码错误或备份已损坏'));
  await running;
  expect(workspace.restoreError.value).toBe('密码错误或备份已损坏');
  expect(showToast).toHaveBeenLastCalledWith('恢复失败', '密码错误或备份已损坏', { persistent: true });
  expect(workspace.backupPassword.value).toBe('test-backup-password');
  expect(workspace.backupBusy.value).toBe(false);
  expect(reload).not.toHaveBeenCalled();

  // 本地取消也要释放恢复状态；不得重载或清空用于重试的密码。
  vi.mocked(api.pickOpenFile).mockResolvedValueOnce(null);
  await workspace.handleImportDatabase();
  expect(api.importDatabase).not.toHaveBeenCalled();
  expect(workspace.backupBusy.value).toBe(false);
  expect(workspace.backupPassword.value).toBe('test-backup-password');

  vi.mocked(api.downloadWebdavBackup).mockImplementationOnce(async (_password, progress) => {
    progress?.({ phase: 'replacing', received: 0, total: 0 });
    expect(reload).not.toHaveBeenCalled();
    return { success: true, restored_size: 4096 };
  });
  await workspace.handleDownloadWebdav();
  expect(reload).toHaveBeenCalledOnce(); // 无延迟计时器。
  expect(workspace.restoreError.value).toBe('');
  expect(workspace.restoreProgress.value?.phase).toBe('reloading');
  expect(workspace.backupPassword.value).toBe('');
  expect(workspace.backupBusy.value).toBe(true); // 新页面加载前不能再操作旧资料。
  vi.unstubAllGlobals();
});
