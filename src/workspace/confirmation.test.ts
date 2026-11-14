import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', async original => ({
  ...await original<typeof import('@tauri-apps/api/core')>(),
  invoke,
  Channel: class { onmessage = () => {}; },
}));

beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  vi.useFakeTimers();
  vi.stubGlobal('localStorage', { getItem: () => null });
  vi.stubGlobal('window', { location: { search: '' }, __TAURI_INTERNALS__: {} });
});
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

test('云端冲突等待应用内决定，取消不覆盖，确认只发出一次覆盖请求', async () => {
  const settings = await import('./settings');
  const { confirmation, finishConfirmation, formatConfirmationTime } = await import('./confirmation');
  invoke.mockImplementation(async (command, args) => {
    if (command === 'upload_webdav_backup') return args.force
      ? { success: true }
      : { conflict: true, remote_modified: 'Thu, 05 Nov 2026 07:10:55 GMT', last_sync_time: '2026-11-05 02:03:41' };
    if (command === 'get_webdav_status') return { configured: true };
  });
  settings.backupPassword.value = 'test-only-password';
  const canceled = settings.handleUploadWebdav();
  await vi.waitFor(() => expect(confirmation.value?.title).toBe('替换云端备份？'));
  expect(settings.isUploadingWebdav.value).toBe(true);
  expect(confirmation.value?.details?.[0]?.value).not.toContain('GMT');
  expect(formatConfirmationTime('2026-11-05 02:03:41', '')).toBe('2026-11-05 02:03');
  expect(invoke).toHaveBeenCalledTimes(1);
  finishConfirmation(false);
  await canceled;
  expect(invoke).toHaveBeenCalledTimes(1);
  expect(settings.backupPassword.value).toBe('test-only-password');

  const confirmed = settings.handleUploadWebdav();
  await vi.waitFor(() => expect(confirmation.value).not.toBeNull());
  await settings.handleUploadWebdav();
  expect(invoke).toHaveBeenCalledTimes(2);
  finishConfirmation(true);
  await confirmed;
  expect(invoke.mock.calls.filter(([command, args]) => command === 'upload_webdav_backup' && args.force)).toHaveLength(1);
  expect(settings.backupPassword.value).toBe('');
  expect(settings.isUploadingWebdav.value).toBe(false);
  expect(confirmation.value).toBeNull();
});
