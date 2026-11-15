// 系统设置：仓库路径、备份、重置点。
import { computed, ref, watch } from 'vue';
import { type BackupProgress, type RestoreProgress, type DbInfo, type ObsidianFile, type WebdavStatus } from '../types';
import { isTauri, saveSetting } from '../services/api';
import {
  applyHotkeys,
  exportDatabase,
  importDatabase,
  undoLastRestore,
  openInExplorer,
  pickFolder,
  pickOpenFile,
  pickSaveFile,
  scanObsidianVault,
  getWebdavStatus,
  saveWebdavConfig,
  testWebdavConnection,
  uploadWebdavBackup,
  cancelWebdavBackup,
  downloadWebdavBackup,
  type HotkeyStatus,
} from '../services/systemApi';
import { kvFlush, kvGet, kvGetJson, kvSet } from '../services/kv';
import { progressFlush } from '../services/progress';
import {
  DEFAULT_CAPTURE_HOTKEY,
  DEFAULT_PANEL_HOTKEY,
  HOTKEY_CAPTURE_KEY,
  HOTKEY_PANEL_KEY,
  normalizeHotkey,
} from '../services/hotkeys';
import { obsidianGameGuides, obsidianVaultPath, showToast } from './core';
import { loadObsidianGuides } from './guides/library';
import { describeError } from '../services/errors';
import { askConfirmation, formatConfirmationTime } from './confirmation';

export const obsidianFiles = ref<ObsidianFile[]>([]);
export const isScanningObsidian = ref(false);
export const dbInfo = ref<DbInfo | null>(null);
export const settingsSection = ref<'games' | 'backup' | 'connections' | 'desktop'>('games');
export const mediaAutoplay = ref(kvGetJson('waypoint_media_autoplay', true));
export const mediaMuted = ref(kvGetJson('waypoint_media_muted', false));
watch(mediaAutoplay, value => kvSet('waypoint_media_autoplay', JSON.stringify(value)));
watch(mediaMuted, value => kvSet('waypoint_media_muted', JSON.stringify(value)));
function storedHotkey(key: string, fallback: string): string {
  const stored = normalizeHotkey(kvGet(key) || '');
  if (key === HOTKEY_CAPTURE_KEY && stored === 'Alt+Shift+M') return fallback;
  return stored || fallback;
}

export const panelHotkey = ref(storedHotkey(HOTKEY_PANEL_KEY, DEFAULT_PANEL_HOTKEY));
export const captureHotkey = ref(storedHotkey(HOTKEY_CAPTURE_KEY, DEFAULT_CAPTURE_HOTKEY));
export const hotkeyError = ref('');
export const notifyDailyEnabled = ref(kvGetJson('waypoint_notify_daily', true));
export const notifyWeeklyEnabled = ref(kvGetJson('waypoint_notify_weekly', true));
export const notifyEventEndEnabled = ref(kvGetJson('waypoint_notify_event_end', true));
// 提醒由后端线程按这些设置发出，键名与 src-tauri/src/cycle.rs 一致
export const notifyDailyTime = ref(kvGet('waypoint_notify_daily_time') || '20:00');
export const isExportingDatabase = ref(false);
const restoreSource = ref<'file' | 'cloud' | 'undo' | null>(null);
export const isRestoringDatabase = computed(() => restoreSource.value !== null);
export const isImportingDatabase = computed(() => restoreSource.value === 'file' || restoreSource.value === 'undo');
export const isDownloadingWebdav = computed(() => restoreSource.value === 'cloud');
export const restoreProgress = ref<RestoreProgress | null>(null);
export const restoreError = ref('');
// Only held for this page operation; never stored in SQLite or browser storage.
export const backupPassword = ref('');

/** 换库成功后保持锁定，直到新页面重新读取 SQLite；失败和取消才返回旧页面。 */
async function runDatabaseRestore(
  source: NonNullable<typeof restoreSource.value>,
  operation: (onProgress: (progress: RestoreProgress) => void) => Promise<boolean>,
) {
  if (!isTauri() || backupBusy.value) return;
  restoreSource.value = source;
  restoreError.value = '';
  restoreProgress.value = null;
  try {
    if (!await operation(progress => { restoreProgress.value = progress; })) {
      restoreSource.value = null;
      return;
    }
    backupPassword.value = '';
    restoreProgress.value = { phase: 'reloading', received: 0, total: 0 };
    window.location.reload();
  } catch (error) {
    restoreError.value = describeError(error).message;
    restoreSource.value = null;
    showToast('恢复失败', restoreError.value, { persistent: true });
  }
}

function requireBackupPassword(): string {
  if ([...backupPassword.value].length < 10) {
    throw new Error('请先填写至少 10 个字符的备份密码。换电脑恢复时需要同一密码。');
  }
  return backupPassword.value;
}

function describeHotkeyStatus(status: HotkeyStatus | null): string {
  if (!status) return '';
  return [status.panel_error, status.capture_error].filter(Boolean).join('；');
}

export async function saveHotkey(which: 'panel' | 'capture', raw: string) {
  const next = normalizeHotkey(raw);
  if (!next) {
    hotkeyError.value = '热键至少要一个修饰键加一个主键，例如 Alt+W';
    return;
  }
  const other = which === 'panel' ? captureHotkey.value : panelHotkey.value;
  if (next.toLowerCase() === other.toLowerCase()) {
    hotkeyError.value = '主面板和记一笔不能用同一条热键';
    return;
  }
  kvSet(which === 'panel' ? HOTKEY_PANEL_KEY : HOTKEY_CAPTURE_KEY, next);
  if (which === 'panel') panelHotkey.value = next;
  else captureHotkey.value = next;
  const status = await applyHotkeys();
  hotkeyError.value = describeHotkeyStatus(status) || (status ? '' : '只在桌面窗口里生效');
}

watch(notifyDailyEnabled, (val) => kvSet('waypoint_notify_daily', JSON.stringify(val)));
watch(notifyWeeklyEnabled, (val) => kvSet('waypoint_notify_weekly', JSON.stringify(val)));
watch(notifyEventEndEnabled, (val) => kvSet('waypoint_notify_event_end', JSON.stringify(val)));
watch(notifyDailyTime, (val) => {
  if (/^\d{1,2}:\d{2}$/.test(val)) kvSet('waypoint_notify_daily_time', val);
});

function backupFileStamp() {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}

export async function handleExportDatabase() {
  if (!isTauri()) {
    showToast('仅桌面端', '浏览器调试没有同一份 SQLite，请用桌面窗口导出');
    return;
  }
  isExportingDatabase.value = true;
  try {
    const password = requireBackupPassword();
    const dest = await pickSaveFile(
      '导出加密备份',
      'Waypoint 加密备份 (*.wpbak)|*.wpbak',
      `waypoint-backup-${backupFileStamp()}.wpbak`
    );
    if (!dest) return;
    await Promise.all([kvFlush(), progressFlush()]);
    const path = await exportDatabase(dest, password);
    backupPassword.value = '';
    showToast('已导出加密备份', path);
  } catch (err: any) {
    showToast('导出失败', (err ? describeError(err).message : '无法写出备份文件'));
  } finally {
    isExportingDatabase.value = false;
  }
}

export async function handleImportDatabase() {
  if (!isTauri()) {
    showToast('仅桌面端', '浏览器调试没有同一份 SQLite，请用桌面窗口导入');
    return;
  }
  await runDatabaseRestore('file', async onProgress => {
    const src = await pickOpenFile(
      '导入 Waypoint 备份',
      'Waypoint 备份 (*.wpbak;*.db;*.gz)|*.wpbak;*.db;*.gz|所有文件 (*.*)|*.*'
    );
    if (!src) return false;
    await Promise.all([kvFlush(), progressFlush()]);
    await importDatabase(src, backupPassword.value, onProgress);
    return true;
  });
}

export async function handleUndoRestore() {
  if (!isTauri() || backupBusy.value) return;
  const at = dbInfo.value?.undo_restore_at;
  if (!at || !await askConfirmation({
    title: '撤销上次恢复？',
    message: '换回上次恢复前的数据库，当前数据会留作新的回滚点。',
    confirmLabel: '撤销恢复',
    details: [{ label: '恢复前快照', value: formatConfirmationTime(at, '时间未知') }],
  })) return;
  await runDatabaseRestore('undo', async onProgress => {
    await Promise.all([kvFlush(), progressFlush()]);
    await undoLastRestore(onProgress);
    return true;
  });
}

// 选择本地 Obsidian 文件夹
export async function handlePickObsidianFolder() {
  try {
    const selected = await pickFolder();
    if (selected) {
      obsidianVaultPath.value = selected;
      await handleScanObsidian();
    }
  } catch (err: any) {
    showToast('选择目录失败', (err ? describeError(err).message : '无法打开系统选择器'));
  }
}

// 在资源管理器中打开当前仓库目录
export async function handleOpenVaultFolder() {
  const p = obsidianVaultPath.value.trim();
  if (!p) {
    showToast('路径为空', '请先选择或输入 Obsidian 仓库路径');
    return;
  }
  try {
    await openInExplorer(p);
  } catch (err: any) {
    showToast('打开失败', (err ? describeError(err).message : '无法打开目录'));
  }
}

// 扫描 Obsidian 真实仓库
export async function handleScanObsidian() {
  const p = obsidianVaultPath.value.trim();
  if (!p) {
    showToast('请输入路径', '请先选择或输入 Obsidian 仓库路径');
    return;
  }
  isScanningObsidian.value = true;
  try {
    obsidianFiles.value = await scanObsidianVault(p);
    await saveSetting('obsidian_vault_path', p);
    await loadObsidianGuides();
    showToast('扫描完成', `共扫描到 ${obsidianFiles.value.length} 篇 Markdown 笔记 (${obsidianGameGuides.value.length} 篇游戏攻略)`);
  } catch (err: any) {
    showToast('扫描失败', (err ? describeError(err).message : '目录不存在或无读取权限'));
  } finally {
    isScanningObsidian.value = false;
  }
}

// 打开 SQLite 数据所在文件夹
export async function handleOpenDbFolder() {
  if (dbInfo.value && dbInfo.value.dir) {
    try {
      await openInExplorer(dbInfo.value.dir);
      showToast('已打开目录', dbInfo.value.dir);
    } catch (err: any) {
      showToast('打开失败', (err ? describeError(err).message : '无法打开目录'));
    }
  }
}

// ================= WebDAV 云端同步 =================
export const webdavUrl = ref('');
export const webdavUsername = ref('');
export const webdavPassword = ref('');
export const webdavStatus = ref<WebdavStatus | null>(null);
export const isTestingWebdav = ref(false);
export const isUploadingWebdav = ref(false);
export const backupBusy = computed(() => isExportingDatabase.value || isUploadingWebdav.value || isRestoringDatabase.value);
export const webdavTestResult = ref<{ ok: boolean; message: string } | null>(null);
export const webdavUploadProgress = ref<BackupProgress | null>(null);
export const webdavUploadResult = ref<{ ok: boolean; message: string } | null>(null);
export const isCancellingWebdav = ref(false);

export async function handleCancelWebdav() {
  if (!isUploadingWebdav.value || isCancellingWebdav.value) return;
  isCancellingWebdav.value = true;
  try {
    await cancelWebdavBackup();
  } catch (error) {
    isCancellingWebdav.value = false;
    showToast('取消失败', describeError(error).message, { persistent: true });
  }
}

export async function refreshWebdavStatus() {
  try {
    const status = await getWebdavStatus();
    webdavStatus.value = status;
    if (!webdavUrl.value && status.url) webdavUrl.value = status.url;
    if (!webdavUsername.value && status.username) webdavUsername.value = status.username;
  } catch (err: any) {
    console.warn('获取 WebDAV 状态失败:', err);
  }
}

export async function handleSaveWebdavConfig() {
  if (!isTauri()) {
    showToast('仅桌面端', 'WebDAV 同步仅在桌面端可用');
    return;
  }
  if (!webdavUrl.value.trim()) {
    showToast('配置错误', 'WebDAV 服务器 URL 不能为空');
    return;
  }
  try {
    await saveWebdavConfig({
      url: webdavUrl.value.trim(),
      username: webdavUsername.value.trim(),
      password: webdavPassword.value || undefined,
    });
    webdavPassword.value = '';
    await refreshWebdavStatus();
    showToast('WebDAV 配置已保存', '可点击测试连接或备份验证');
  } catch (err: any) {
    showToast('保存失败', (err ? describeError(err).message : '无法保存 WebDAV 配置'));
  }
}

export async function handleTestWebdav() {
  if (!isTauri()) return;
  if (!webdavUrl.value.trim()) {
    showToast('提示', '请先填写 WebDAV URL');
    return;
  }
  isTestingWebdav.value = true;
  webdavTestResult.value = null;
  try {
    const res = await testWebdavConnection(
      webdavUrl.value.trim(),
      webdavUsername.value.trim(),
      webdavPassword.value || undefined
    );
    webdavTestResult.value = res;
    showToast('连接成功', res.message);
  } catch (err: any) {
    const msg = (err ? describeError(err).message : '连接超时或认证失败');
    webdavTestResult.value = { ok: false, message: msg };
    showToast('连接失败', msg);
  } finally {
    isTestingWebdav.value = false;
  }
}

export async function handleUploadWebdav() {
  if (!isTauri()) return;
  if (isUploadingWebdav.value) return;
  isUploadingWebdav.value = true;
  webdavUploadProgress.value = null;
  webdavUploadResult.value = null;
  isCancellingWebdav.value = false;
  try {
    const password = requireBackupPassword();
    await Promise.all([kvFlush(), progressFlush()]);
    const updateProgress = (progress: BackupProgress) => { webdavUploadProgress.value = progress; };
    let res = await uploadWebdavBackup(password, false, updateProgress);
    if (res.conflict) {
      const keepLocal = await askConfirmation({
        title: '替换云端备份？',
        message: '云端版本与本机上次同步的记录不同。继续将用本机完整数据替换云端备份；也可以先取消，检查云端那份数据。',
        confirmLabel: '用本机覆盖云端',
        cancelLabel: '暂不上传',
        details: [
          { label: '云端更新时间（本地时间）', value: formatConfirmationTime(res.remote_modified, '时间未知') },
          { label: '本机上次同步', value: formatConfirmationTime(res.last_sync_time, '从未同步') },
        ],
      });
      if (!keepLocal) {
        webdavUploadResult.value = { ok: false, message: '已取消上传，云端备份未修改。' };
        showToast('已取消上传', '云端备份未修改');
        return;
      }
      res = await uploadWebdavBackup(password, true, updateProgress);
    }
    backupPassword.value = '';
    await refreshWebdavStatus();
    const rawKb = res.raw_size ? (res.raw_size / 1024).toFixed(1) : '?';
    const compKb = res.compressed_size ? (res.compressed_size / 1024).toFixed(1) : '?';
    webdavUploadResult.value = { ok: true, message: `已加密备份到云端 · ${compKb} KB · ${res.sync_time || ''}` };
    showToast('已加密备份到云端', `原库 ${rawKb} KB → 加密备份 ${compKb} KB`);
  } catch (err: any) {
    const message = err ? describeError(err).message : '网络错误或无写入权限';
    webdavUploadResult.value = { ok: false, message };
    showToast('上传备份失败', message, { persistent: true });
  } finally {
    isUploadingWebdav.value = false;
    isCancellingWebdav.value = false;
  }
}

export async function handleDownloadWebdav() {
  await runDatabaseRestore('cloud', async onProgress => {
    await Promise.all([kvFlush(), progressFlush()]);
    await downloadWebdavBackup(backupPassword.value, onProgress);
    return true;
  });
}
