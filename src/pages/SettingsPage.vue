<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { ArrowDownToLine, ArrowUpFromLine, Bell, Check, ChevronDown, Cloud, ExternalLink, Eye, EyeOff, FolderOpen, Gamepad2, KeyRound, Link2, Settings2 } from '@lucide/vue';
import GameStampIcon from '../components/GameStampIcon.vue';
import DiscoveryPreferences from '../components/DiscoveryPreferences.vue';
import { askConfirmation, formatConfirmationTime } from '../workspace/confirmation';
import {
  configurableResetGames,
  enabledGameIds,
  GAMES,
  gameResetIsCustom,
  isGameEnabled,
  setGameEnabled,
  TOGGLEABLE_GAMES,
} from '../services/games';
import {
  dbInfo,
  settingsSection,
  mediaAutoplay,
  mediaMuted,
  handleExportDatabase,
  handleImportDatabase,
  handleOpenDbFolder,
  handleOpenVaultFolder,
  handlePickObsidianFolder,
  handleScanObsidian,
  isExportingDatabase,
  isImportingDatabase,
  isScanningObsidian,
  captureHotkey,
  hotkeyError,
  notifyDailyEnabled,
  panelHotkey,
  saveHotkey,
  notifyWeeklyEnabled,
  webdavUrl,
  webdavUsername,
  webdavPassword,
  backupPassword,
  webdavStatus,
  webdavTestResult,
  isTestingWebdav,
  isUploadingWebdav,
  isDownloadingWebdav,
  webdavUploadProgress,
  webdavUploadResult,
  backupBusy,
  restoreError,
  isCancellingWebdav,
  handleCancelWebdav,
  handleSaveWebdavConfig,
  handleTestWebdav,
  handleUploadWebdav,
  handleDownloadWebdav,
} from '../workspace/settings';
import {
  currentTab,
  gameResetEpoch,
  isProjectingObsidian,
  obsidianGameGuides,
  obsidianVaultPath,
  saveGameResetTimes,
  showToast,
  triggerSampleToast,
} from '../workspace/core';
import { handleProjectAllToObsidian } from '../workspace/bootstrap';
import { selectVaultGroup, steamWebApiKeyConfigured } from '../workspace/vault';
import { handleUndoRestore, notifyDailyTime, notifyEventEndEnabled } from '../workspace/settings';
import { WEEKDAY_OPTIONS, formatClockTime, parseResetClock } from '../services/cycleReset';
import { describeError } from '../services/errors';

const MOD_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta']);
const recordingHotkey = ref<'panel' | 'capture' | null>(null);

function captureHotkeyPress(which: 'panel' | 'capture', event: KeyboardEvent) {
  if (recordingHotkey.value !== which) return;
  if (event.key === 'Tab') { recordingHotkey.value = null; return; }
  event.preventDefault();
  if (event.key === 'Escape') { recordingHotkey.value = null; return; }
  if (MOD_KEYS.has(event.key)) return;
  if (event.repeat) return;
  const parts: string[] = [];
  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Super');
  const key = event.key === ' ' ? 'Space' : event.key.length === 1 ? event.key.toUpperCase() : event.key;
  parts.push(key);
  recordingHotkey.value = null;
  void saveHotkey(which, parts.join('+'));
}

type ResetDraft = { daily: string; weekday: number; weeklyTime: string };

const featureGames = computed(() => {
  enabledGameIds.value;
  return TOGGLEABLE_GAMES;
});

const resetGames = computed(() => {
  gameResetEpoch.value;
  enabledGameIds.value;
  return configurableResetGames().filter(game => game.id === 'other' || isGameEnabled(game.id));
});

const steamKeyVisible = computed(() => isGameEnabled('galgame'));

const showWebdavPassword = ref(false);
const uploadLabel = computed(() => {
  if (isCancellingWebdav.value) return '正在取消，等待当前步骤结束…';
  const progress = webdavUploadProgress.value;
  if (!progress) return '正在保存待备份的数据…';
  const labels = { checking: '正在检查云端版本…', packing: '正在压缩并加密数据库和图片…',
    uploading: '正在发送加密备份…', verifying: '正在核对云端文件…', publishing: '正在替换云端备份并确认结果…' };
  if (progress.phase !== 'uploading' || !progress.total) return labels[progress.phase];
  const mb = (bytes: number) => (bytes / 1024 / 1024).toFixed(1);
  return progress.sent === progress.total
    ? `已发送 ${mb(progress.total)} MiB，等待服务器确认…`
    : `已发送 ${mb(progress.sent)} / ${mb(progress.total)} MiB`;
});
const categories = [
  { id: 'games', label: '游戏与日课', icon: Gamepad2 },
  { id: 'backup', label: '备份与恢复', icon: Cloud },
  { id: 'connections', label: '外部连接', icon: Link2 },
  { id: 'desktop', label: '快捷键与提醒', icon: Bell },
] as const;
const hotkeyFields = [
  { id: 'panel', label: '呼出主面板', description: '在其他窗口中快速打开或收起 Waypoint。' },
  { id: 'capture', label: '记一笔', description: '随时打开快速记录窗口。' },
] as const;

function fillWebdavPreset(kind: 'infinicloud' | 'jianguoyun') {
  if (kind === 'infinicloud') {
    if (!webdavUrl.value) webdavUrl.value = 'https://tera.teracloud.jp/dav/waypoint/';
  } else if (kind === 'jianguoyun') {
    if (!webdavUrl.value) webdavUrl.value = 'https://dav.jianguoyun.com/dav/waypoint/';
  }
}

async function confirmRestoreAndDownload() {
  if (await askConfirmation({
    title: '从云端恢复？',
    message: '云端备份将替换本机账号、任务、收集进度和设置。恢复前会自动保留本机快照，可以撤销。游戏存档需单独备份和恢复。',
    confirmLabel: '覆盖并恢复',
  })) await handleDownloadWebdav();
}

const resetDrafts = reactive<Record<string, ResetDraft>>({});

function draftFromMeta(id: string, defaults = false): ResetDraft {
  const games = defaults ? GAMES : configurableResetGames();
  const meta = games.find(g => g.id === id) || games[games.length - 1];
  const daily = parseResetClock(meta.dailyReset, '00:00');
  const weekly = parseResetClock(meta.weeklyReset, meta.dailyReset);
  return {
    daily: formatClockTime(daily.hour, daily.minute),
    weekday: weekly.weekday ?? 1,
    weeklyTime: formatClockTime(weekly.hour, weekly.minute)
  };
}

function syncResetDrafts() {
  for (const g of configurableResetGames()) {
    resetDrafts[g.id] = draftFromMeta(g.id);
  }
}

syncResetDrafts();
watch(gameResetEpoch, syncResetDrafts);

function weeklyRuleOf(draft: ResetDraft) {
  const wd = WEEKDAY_OPTIONS.find(o => o.value === draft.weekday)?.label || '周一';
  return `${wd} ${draft.weeklyTime || '00:00'}`;
}

const isSavingResets = ref(false);
const resetChanges = computed(() => resetGames.value.flatMap(game => {
  const draft = resetDrafts[game.id];
  if (!draft) return [];
  const weeklyReset = weeklyRuleOf(draft);
  return draft.daily !== game.dailyReset || weeklyReset !== game.weeklyReset
    ? [{ gameId: game.id, dailyReset: draft.daily, weeklyReset }]
    : [];
}));

function resetCanDefault(id: string) {
  return JSON.stringify(resetDrafts[id]) !== JSON.stringify(draftFromMeta(id, true));
}

async function commitResets() {
  if (isSavingResets.value || !resetChanges.value.length) return;
  isSavingResets.value = true;
  try {
    await saveGameResetTimes(resetChanges.value);
  } catch (error) {
    showToast('重置时间保存失败', describeError(error).message);
  } finally {
    isSavingResets.value = false;
  }
}

</script>

<template>
  <section class="settings-workspace">
    <header class="settings-heading">
      <div><p class="settings-eyebrow">按你的习惯</p><h1>设置<span>.</span></h1><p>安排日课、连接资料，让每次打开都顺手。</p></div>
      <Settings2 aria-hidden="true" />
    </header>
    <nav class="settings-tabs" aria-label="设置分类">
      <button v-for="category in categories" :key="category.id" type="button"
        :aria-current="settingsSection === category.id ? 'page' : undefined"
        :aria-controls="'settings-' + category.id" @click="settingsSection = category.id">
        <component :is="category.icon" /><span>{{ category.label }}</span>
      </button>
    </nav>

    <div v-show="settingsSection === 'games'" id="settings-games" class="settings-content" role="region" aria-label="游戏与日课" tabindex="0">
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">01</span><h2>游戏偏好</h2><p>只留下你正在玩的游戏。</p></div>
        <div class="settings-section-body">
          <label v-for="game in featureGames" :key="game.id" class="settings-row settings-feature" :data-game="game.stamp">
            <GameStampIcon :game="game.stamp" />
            <span class="settings-row-copy"><strong>{{ game.label }}</strong><span>{{ game.pageTitle }}</span></span>
            <input type="checkbox" role="switch" class="settings-switch" :aria-label="'启用' + game.label"
              :checked="isGameEnabled(game.id)" @change="setGameEnabled(game.id, ($event.target as HTMLInputElement).checked)" />
          </label>
          <p class="settings-help">关闭后隐藏导航并停止拉取，账号和收集进度仍会保留。</p>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><h2>作品影音</h2><p>进入游戏详情时的默认播放方式。</p></div>
        <div class="settings-section-body">
          <label class="settings-row"><span class="settings-row-copy"><strong>自动播放</strong><span>有可播放媒体时自动开始；离开详情或启动游戏后停止。</span></span><input v-model="mediaAutoplay" type="checkbox" role="switch" class="settings-switch" aria-label="自动播放作品影音" /></label>
          <label class="settings-row"><span class="settings-row-copy"><strong>默认静音</strong><span>关闭时有声播放，也可以在播放器中临时调整。</span></span><input v-model="mediaMuted" type="checkbox" role="switch" class="settings-switch" aria-label="作品影音默认静音" /></label>
          <p class="settings-help">自动保存，下次进入作品时生效。</p>
        </div>
      </section>
      <form class="settings-section settings-reset-section" @submit.prevent="commitResets">
        <div class="settings-section-intro"><span class="settings-section-number">02</span><div class="settings-reset-heading"><h2>日课重置</h2><button v-if="resetGames.length" type="submit" class="settings-button is-primary" :disabled="isSavingResets || !resetChanges.length">{{ isSavingResets ? '保存中…' : resetChanges.length ? '保存修改' : '已保存' }}</button></div><p>设置每款游戏的换日与周常刷新时间。</p><p>统一保存，当前任务勾选不变。</p></div>
        <div class="settings-section-body">
          <p v-if="!resetGames.length" class="settings-empty">启用带日课的游戏后，可以在这里调整重置时间。</p>
          <fieldset v-for="game in resetGames" :key="game.id" class="settings-reset" :disabled="isSavingResets" :aria-label="game.label + '重置时间'">
            <div class="settings-block-title"><h3>{{ game.label }}</h3><div class="settings-reset-status"><span v-if="resetChanges.some(change => change.gameId === game.id)" class="settings-state">未保存</span><span v-else-if="gameResetIsCustom(game.id)" class="settings-muted">自定义</span><span v-else class="settings-muted">默认时间</span><button v-if="resetCanDefault(game.id)" type="button" class="settings-text-button" :aria-label="'恢复' + game.label + '默认时间'" @click="resetDrafts[game.id] = draftFromMeta(game.id, true)">恢复默认</button></div></div>
            <div v-if="resetDrafts[game.id]" class="settings-reset-fields">
              <label class="settings-field"><span>每日重置</span><input type="time" v-model="resetDrafts[game.id].daily" :aria-label="game.label + '每日重置'" required /></label>
              <label class="settings-field"><span>周常重置日</span><select v-model.number="resetDrafts[game.id].weekday" :aria-label="game.label + '周常重置日'"><option v-for="day in WEEKDAY_OPTIONS" :key="day.value" :value="day.value">{{ day.label }}</option></select></label>
              <label class="settings-field"><span>周常时间</span><input type="time" v-model="resetDrafts[game.id].weeklyTime" :aria-label="game.label + '周常时间'" required /></label>
            </div>
          </fieldset>
        </div>
      </form>
    </div>

    <div v-show="settingsSection === 'backup'" id="settings-backup" class="settings-content" role="region" aria-label="备份与恢复" tabindex="0">
      <p v-if="restoreError" role="alert" class="text-sm text-alert break-words select-text">恢复失败：{{ restoreError }}。请检查后重试。</p>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">01</span><h2>备份密码</h2><p>本地和云端共用一份密码，用于加密整份备份。</p></div>
        <div class="settings-section-body">
          <label class="settings-field" for="backup-password"><span><KeyRound />本次备份或恢复使用的密码</span><input id="backup-password" v-model="backupPassword" type="password" autocomplete="off" aria-describedby="backup-password-help" placeholder="创建至少 10 个字符；恢复时填写原密码" /></label>
          <p id="backup-password-help" class="settings-help">密码不保存、不上传，操作成功后清空。请自行保管，遗失后无法恢复。</p>
          <p class="settings-help">此处恢复工作台资料和图片。游戏目录、实际游戏存档及其备份文件需单独迁移；恢复时会保留这些文件。</p>
          <details class="settings-disclosure"><summary>备份包含哪些内容？<ChevronDown /></summary><div class="settings-disclosure-body"><p>账号、角色、任务、收集进度、记事、设置，以及已缓存的封面和图标。</p><p>WebDAV 登录密码与 Steam API Key 需在新电脑重新填写。Obsidian 仓库、游戏存档和插件文件需另外迁移。</p><p>旧版 .db / .gz 没有备份密码，可留空；仅在账号能够解密时允许恢复。</p></div></details>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">02</span><h2>云端备份</h2><p>通过 WebDAV 保存加密备份，换电脑时带回完整资料。</p></div>
        <div class="settings-section-body">
          <div class="settings-block-title"><h3>WebDAV</h3><span class="settings-state" :class="{ 'is-muted': !webdavStatus?.configured }"><Check v-if="webdavStatus?.configured" />{{ webdavStatus?.configured ? '已配置' : '待配置' }}</span></div>
          <dl class="settings-facts">
            <div><dt>上次同步</dt><dd>{{ formatConfirmationTime(webdavStatus?.last_sync_time, '尚未同步') }}</dd></div>
            <div><dt>云端文件</dt><dd>{{ webdavStatus?.remote_error ? '暂时无法查询' : webdavStatus?.remote_exists ? (webdavStatus.remote_size ? (webdavStatus.remote_size / 1024).toFixed(1) + ' KB' : '已有备份') : webdavStatus?.configured ? '暂无备份' : '配置后查看' }}</dd></div>
          </dl>
          <p v-if="webdavStatus?.remote_error" role="status" class="settings-help text-alert">{{ webdavStatus.remote_error }}。本机连接配置已保留，可展开下方配置检查连接。</p>
          <div class="settings-actions">
            <button type="button" class="settings-button is-primary" :disabled="backupBusy || !webdavStatus?.configured" @click="handleUploadWebdav"><ArrowUpFromLine />{{ isUploadingWebdav ? '正在上传…' : '备份到云端' }}</button>
            <button type="button" class="settings-button" :disabled="backupBusy || !webdavStatus?.configured" @click="confirmRestoreAndDownload"><ArrowDownToLine />{{ isDownloadingWebdav ? '正在恢复…' : '从云端恢复' }}</button>
            <button v-if="isUploadingWebdav" type="button" class="settings-text-button" :disabled="isCancellingWebdav || webdavUploadProgress?.phase === 'publishing'" @click="handleCancelWebdav">{{ isCancellingWebdav ? '正在取消…' : '取消上传' }}</button>
          </div>
          <div v-if="isUploadingWebdav" role="status" class="mt-3 grid min-w-0 gap-2 text-xs text-muted">
            <p>{{ uploadLabel }}</p>
            <progress class="h-1.5 w-full accent-beacon" aria-label="加密备份发送进度"
              :value="webdavUploadProgress?.total ? webdavUploadProgress.sent : undefined"
              :max="webdavUploadProgress?.total || 1" />
            <p class="text-faint">传输完成后还会校验并替换云端备份。持续传输不受总时长限制；连续 2 分钟无进展会停止。</p>
          </div>
          <p v-if="!isUploadingWebdav && webdavUploadResult" :role="webdavUploadResult.ok ? 'status' : 'alert'"
            class="settings-help break-words select-text" :class="webdavUploadResult.ok ? 'text-done' : 'text-alert'">{{ webdavUploadResult.message }}</p>
          <details class="settings-disclosure settings-webdav-config" :open="!webdavStatus?.configured">
            <summary>连接配置<ChevronDown /></summary>
            <div class="settings-disclosure-body">
              <label class="settings-field"><span>服务器地址</span><input v-model="webdavUrl" type="url" placeholder="https://你的服务器/dav/waypoint/" spellcheck="false" /></label>
              <div class="settings-presets"><span>地址示例</span><button type="button" @click="fillWebdavPreset('infinicloud')">InfiniCLOUD</button><button type="button" @click="fillWebdavPreset('jianguoyun')">坚果云</button></div>
              <div class="settings-two-fields">
                <label class="settings-field"><span>用户名 / 账号</span><input v-model="webdavUsername" type="text" autocomplete="off" placeholder="账号或邮箱" /></label>
                <div class="settings-field"><label for="webdav-password">密码 / 应用密码</label><div class="settings-secret-field"><input id="webdav-password" v-model="webdavPassword" :type="showWebdavPassword ? 'text' : 'password'" autocomplete="off" :placeholder="webdavStatus?.has_password ? '已保存 · 留空不修改' : '输入 WebDAV 密码'" /><button type="button" :aria-label="showWebdavPassword ? '隐藏 WebDAV 密码' : '显示 WebDAV 密码'" :aria-pressed="showWebdavPassword" @click="showWebdavPassword = !showWebdavPassword"><EyeOff v-if="showWebdavPassword" /><Eye v-else /></button></div></div>
              </div>
              <div class="settings-actions"><button type="button" class="settings-button" :disabled="backupBusy" @click="handleSaveWebdavConfig">保存连接</button><button type="button" class="settings-text-button" :disabled="isTestingWebdav || backupBusy || !webdavUrl" @click="handleTestWebdav">{{ isTestingWebdav ? '测试中…' : '测试连接' }}</button></div>
              <p class="settings-help">连接密码保存在本机，留空不修改；测试连接只验证当前输入，修改后请点“保存连接”。</p>
              <p v-if="webdavTestResult" role="status" class="settings-help" :class="webdavTestResult.ok ? 'text-done' : 'text-alert'">{{ webdavTestResult.message }}</p>
            </div>
          </details>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">03</span><h2>本地备份</h2><p>导出一份 .wpbak 文件，自行保存或带到另一台电脑。</p></div>
        <div class="settings-section-body">
          <div class="settings-actions"><button type="button" class="settings-button is-primary" :disabled="backupBusy" @click="handleExportDatabase"><ArrowUpFromLine />{{ isExportingDatabase ? '正在导出…' : '导出加密备份' }}</button><button type="button" class="settings-button" :disabled="backupBusy" @click="handleImportDatabase"><ArrowDownToLine />{{ isImportingDatabase ? '正在导入…' : '从文件恢复' }}</button></div>
          <p class="settings-help">恢复会替换本机资料，并自动保留恢复前快照。</p>
          <div v-if="dbInfo?.undo_restore_at" class="settings-recovery"><div><strong>可以撤销上次恢复</strong><p>{{ formatConfirmationTime(dbInfo.undo_restore_at, '时间未知') }} 的恢复前快照</p></div><button type="button" class="settings-button" :disabled="backupBusy" @click="handleUndoRestore">撤销恢复</button></div>
          <details class="settings-disclosure"><summary>本机存储位置<ChevronDown /></summary><div class="settings-disclosure-body"><template v-if="dbInfo"><p class="settings-path">{{ dbInfo.path }}</p><p class="settings-help">SQLite · {{ (dbInfo.size_bytes / 1024).toFixed(1) }} KB</p></template><p v-else class="settings-help">本机存储信息在桌面客户端中可用。</p><button type="button" class="settings-text-button" :disabled="!dbInfo" @click="handleOpenDbFolder"><FolderOpen />打开数据目录</button></div></details>
        </div>
      </section>
    </div>

    <div v-show="settingsSection === 'connections'" id="settings-connections" class="settings-content" role="region" aria-label="外部连接" tabindex="0">
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">01</span><h2>Obsidian</h2><p>让游戏攻略与本地笔记相连。</p></div>
        <div class="settings-section-body">
          <div class="settings-block-title"><h3>攻略仓库</h3><span class="settings-state" :class="{ 'is-muted': !obsidianVaultPath }">{{ obsidianVaultPath ? '已关联 ' + obsidianGameGuides.length + ' 篇' : '尚未绑定' }}</span></div>
          <label class="settings-field"><span>本地仓库路径</span><input v-model="obsidianVaultPath" type="text" @keydown.enter="handleScanObsidian" placeholder="选择或输入 Obsidian 仓库路径" spellcheck="false" /></label>
          <div class="settings-actions"><button type="button" class="settings-button" @click="handlePickObsidianFolder"><FolderOpen />选择目录</button><button type="button" class="settings-button" :disabled="isScanningObsidian || !obsidianVaultPath" @click="handleScanObsidian">{{ isScanningObsidian ? '扫描中…' : '扫描攻略' }}</button><button type="button" class="settings-text-button" :disabled="!obsidianVaultPath" @click="handleOpenVaultFolder"><ExternalLink />打开目录</button><button v-if="obsidianVaultPath" type="button" class="settings-text-button" @click="obsidianVaultPath = ''">清空路径</button></div>
          <p class="settings-help">扫描后，攻略会出现在对应游戏页面。可以直接阅读本地笔记。</p>
          <div class="settings-row settings-export-notes"><span class="settings-row-copy"><strong>写出全部航标笔记</strong><span>将所有页面的航标笔记写入仓库，原有笔记不改不删。</span></span><button type="button" class="settings-button" :disabled="isProjectingObsidian || !obsidianVaultPath" @click="handleProjectAllToObsidian">{{ isProjectingObsidian ? '写出中…' : '写出全部' }}</button></div>
        </div>
      </section>
      <section v-if="steamKeyVisible" class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">02</span><h2>Steam</h2><p>关联库存，补全单机游戏档案。</p></div>
        <div class="settings-section-body">
          <div class="settings-block-title"><h3>Web API Key</h3><span class="settings-state" :class="{ 'is-muted': !steamWebApiKeyConfigured }">{{ steamWebApiKeyConfigured ? '已配置' : '尚未配置' }}</span></div>
          <p class="settings-help">Steam 账号、密钥与同步统一在资产金库管理。已保存的密钥可继续使用，无需重新填写。</p>
          <button class="settings-button" @click="selectVaultGroup('steam'); currentTab = 'vault'">前往金库 · Steam 连接</button>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">03</span><h2>发现与优惠</h2><p>选择感兴趣的游戏类型和商店地区。</p></div>
        <div class="settings-section-body"><DiscoveryPreferences /></div>
      </section>
    </div>

    <div v-show="settingsSection === 'desktop'" id="settings-desktop" class="settings-content" role="region" aria-label="快捷键与提醒" tabindex="0">
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">01</span><h2>全局快捷键</h2><p>点击组合键后录入新快捷键，Esc 取消。</p></div>
        <div class="settings-section-body">
          <div v-for="field in hotkeyFields" :key="field.id" class="settings-row">
            <span class="settings-row-copy"><strong>{{ field.label }}</strong><span>{{ field.description }}</span></span>
            <button type="button" class="settings-hotkey" :class="{ 'is-recording': recordingHotkey === field.id }" :aria-label="'修改' + field.label + '快捷键'" :aria-pressed="recordingHotkey === field.id" @click="recordingHotkey = field.id" @keydown.stop="captureHotkeyPress(field.id, $event)" @blur="recordingHotkey = null">{{ recordingHotkey === field.id ? '按下新组合…' : field.id === 'panel' ? panelHotkey : captureHotkey }}</button>
          </div>
          <p v-if="hotkeyError" role="alert" class="settings-help text-alert">{{ hotkeyError }}</p>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">02</span><h2>提醒</h2><p>窗口收进托盘后，后台仍会提醒。</p></div>
        <div class="settings-section-body">
          <div class="settings-row"><label class="settings-row-copy" for="notify-daily"><strong>日课未完成</strong><span>每天在指定时间检查尚未完成的日课。</span></label><div class="settings-row-controls"><input v-model="notifyDailyTime" type="time" aria-label="日课提醒时间" :disabled="!notifyDailyEnabled" /><input id="notify-daily" v-model="notifyDailyEnabled" type="checkbox" role="switch" class="settings-switch" aria-label="日课未完成提醒" /></div></div>
          <label class="settings-row"><span class="settings-row-copy"><strong>周常刷新</strong><span>新一周的周常开放时提醒。</span></span><input v-model="notifyWeeklyEnabled" type="checkbox" role="switch" class="settings-switch" aria-label="周常刷新提醒" /></label>
          <label class="settings-row"><span class="settings-row-copy"><strong>光遇活动即将结束</strong><span>在活动结束前一天提醒。</span></span><input v-model="notifyEventEndEnabled" type="checkbox" role="switch" class="settings-switch" aria-label="光遇活动结束提醒" /></label>
          <div class="settings-actions"><button type="button" class="settings-button" @click="triggerSampleToast"><Bell />发送测试通知</button><span class="settings-muted">开关和提醒时间会自动保存</span></div>
        </div>
      </section>
      <section class="settings-section">
        <div class="settings-section-intro"><span class="settings-section-number">03</span><h2>界面操作</h2><p>常用操作，随时触手可及。</p></div>
        <dl class="settings-section-body settings-facts settings-shortcuts"><div><dt>调整界面大小</dt><dd>底栏缩放 · <kbd>Ctrl + / − / 0</kbd></dd></div><div><dt>展开或收起侧栏</dt><dd>点击分隔线 · <kbd>Ctrl+B</kbd></dd></div><div><dt>关闭窗口</dt><dd>收进系统托盘，继续在后台运行</dd></div></dl>
      </section>
    </div>
  </section>
</template>

<style scoped>
.settings-workspace { height: 100%; min-height: 0; max-width: 1240px; margin-inline: auto; display: flex; flex-direction: column; container-type: inline-size; }
.settings-heading { display: flex; align-items: center; justify-content: space-between; gap: 20px; padding-bottom: 24px; flex-shrink: 0; }
.settings-heading h1 { font-size: 32px; line-height: 1.25; font-weight: 600; letter-spacing: -.04em; margin: 4px 0 8px; }
.settings-heading h1 span { color: var(--color-beacon); }
.settings-heading p { font-size: 12px; color: var(--color-muted); }
.settings-heading .settings-eyebrow { font-size: 10px; color: var(--color-faint); letter-spacing: .12em; }
.settings-heading > svg { width: 30px; height: 30px; stroke-width: 1; color: var(--color-beacon); margin-right: 4px; }
.settings-tabs { display: flex; gap: 12px; border-bottom: 1px solid var(--color-hairline); flex-shrink: 0; overflow-x: auto; scrollbar-width: thin; scrollbar-color: var(--color-hairline) transparent; }
.settings-tabs button { display: inline-flex; align-items: center; gap: 7px; padding: 12px 8px; border-bottom: 2px solid transparent; color: var(--color-faint); font-size: 13px; white-space: nowrap; cursor: pointer; }
.settings-tabs button:hover { color: var(--color-ink); }
.settings-tabs button[aria-current] { color: var(--color-beacon); border-bottom-color: var(--color-beacon); }
.settings-tabs svg { width: 15px; height: 15px; }
.settings-content { flex: 1; min-height: 0; overflow-y: auto; overflow-x: hidden; scrollbar-gutter: stable; padding-right: 12px; padding-bottom: 40px; }
.settings-section { display: grid; grid-template-columns: minmax(130px, .7fr) minmax(0, 2fr); gap: 36px; padding: 30px 0; border-bottom: 1px solid var(--color-hairline); }
.settings-section:last-child { border-bottom: 0; }
.settings-section-intro h2 { color: var(--color-ink); font-size: 17px; font-weight: 600; margin: 4px 0 8px; }
.settings-section-intro p { color: var(--color-faint); font-size: 12px; line-height: 1.8; margin-top: 6px; max-width: 210px; }
.settings-section-number { color: var(--color-beacon); font-family: var(--font-display); font-size: 12px; }
.settings-section-body { min-width: 0; }
.settings-row { display: flex; align-items: center; gap: 18px; padding: 18px 0; border-bottom: 1px solid var(--color-hairline); min-width: 0; }
.settings-row:first-child { padding-top: 4px; }
.settings-row-copy { flex: 1; min-width: 0; display: grid; gap: 5px; }
.settings-row-copy strong { font-size: 14px; font-weight: 500; }
.settings-row-copy > span { font-size: 12px; color: var(--color-faint); line-height: 1.7; }
.settings-feature > .inline-flex { width: 25px; height: 25px; }
.settings-feature[data-game='jx3'] > .inline-flex { color: var(--color-stamp-jx3); }
.settings-feature[data-game='sky'] > .inline-flex { color: var(--color-stamp-sky); }
.settings-feature[data-game='gal'] > .inline-flex { color: var(--color-stamp-gal); }
.settings-feature { cursor: pointer; }
.settings-switch { appearance: none; position: relative; width: 34px; height: 20px; border: 1px solid var(--color-hairline); border-radius: 20px; background: var(--color-raised); flex-shrink: 0; cursor: pointer; transition: background 150ms ease; }
.settings-switch::after { content: ''; position: absolute; top: 3px; left: 3px; width: 12px; height: 12px; border-radius: 50%; background: var(--color-muted); transition: transform 150ms ease; }
.settings-switch:checked { background: var(--color-beacon); border-color: var(--color-beacon); }
.settings-switch:checked::after { transform: translateX(14px); background: var(--color-chrome); }
.settings-help, .settings-empty { font-size: 12px; line-height: 1.85; color: var(--color-faint); margin-top: 12px; }
.settings-help.text-alert { color: var(--color-alert); }
.settings-help.text-done { color: var(--color-done); }
.settings-muted { font-size: 11px; color: var(--color-faint); }
.settings-block-title { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 8px; margin-bottom: 18px; }
.settings-block-title h3 { font-size: 14px; font-weight: 600; }
.settings-state { display: inline-flex; align-items: center; gap: 5px; color: var(--color-done); font-size: 11px; }
.settings-state svg { width: 12px; height: 12px; }
.settings-state.is-muted { color: var(--color-faint); }
.settings-reset { padding: 22px 0; border-bottom: 1px solid var(--color-hairline); }
.settings-reset-heading { display: flex; align-items: center; justify-content: space-between; gap: 8px; flex-wrap: wrap; }
.settings-reset-heading h2 { margin-block: 4px 8px; }
.settings-reset-heading .settings-button { flex-shrink: 0; min-height: 30px; padding: 4px 8px; }
.settings-reset-status { display: flex; align-items: center; gap: 12px; }
.settings-reset-status .settings-text-button { min-height: 0; padding: 0; font-size: 11px; }
.settings-reset:first-child { padding-top: 4px; }
.settings-reset:last-child { border-bottom: 0; padding-bottom: 0; }
.settings-reset-fields { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px; }
.settings-field { display: flex; flex-direction: column; gap: 8px; min-width: 0; }
.settings-field > span, .settings-field > label { display: flex; align-items: center; gap: 7px; color: var(--color-muted); font-size: 12px; }
.settings-field svg { width: 14px; height: 14px; }
.settings-field input, .settings-field select, .settings-row-controls input[type='time'] { min-width: 0; width: 100%; min-height: 38px; border: 1px solid var(--color-hairline); border-radius: 5px; padding: 8px 10px; background: var(--color-chrome); color: var(--color-ink); font-size: 12px; color-scheme: dark; }
.settings-field input::placeholder { color: var(--color-faint); }
.settings-actions { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; margin-top: 16px; }
.settings-button, .settings-text-button { display: inline-flex; align-items: center; justify-content: center; gap: 7px; min-height: 34px; padding: 6px 12px; border: 1px solid var(--color-hairline); border-radius: 5px; color: var(--color-muted); font-size: 12px; cursor: pointer; }
.settings-button:hover { color: var(--color-ink); background: var(--color-paper); }
.settings-button.is-primary { color: var(--color-beacon); border-color: color-mix(in srgb, var(--color-beacon) 32%, transparent); background: color-mix(in srgb, var(--color-beacon) 7%, transparent); }
.settings-button.is-primary:hover { background: color-mix(in srgb, var(--color-beacon) 14%, transparent); }
.settings-text-button { border-color: transparent; padding-inline: 4px; }
.settings-text-button:hover { color: var(--color-ink); }
.settings-button svg, .settings-text-button svg { width: 14px; height: 14px; }
button:disabled, input:disabled { opacity: .4; cursor: default; }
.settings-disclosure { margin-top: 20px; border-top: 1px solid var(--color-hairline); }
.settings-disclosure summary { list-style: none; display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 13px 0; font-size: 12px; color: var(--color-muted); cursor: pointer; }
.settings-disclosure summary::-webkit-details-marker { display: none; }
.settings-disclosure summary svg { width: 13px; height: 13px; transition: transform 150ms ease; }
.settings-disclosure[open] > summary svg { transform: rotate(180deg); }
.settings-disclosure-body { padding: 5px 0 8px; display: grid; gap: 12px; color: var(--color-faint); font-size: 12px; line-height: 1.8; }
.settings-disclosure-body .settings-actions, .settings-disclosure-body .settings-help { margin-top: 0; }
.settings-facts { display: grid; gap: 12px; font-size: 12px; }
.settings-facts > div { display: flex; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
.settings-facts dt { color: var(--color-faint); }
.settings-facts dd { color: var(--color-muted); overflow-wrap: anywhere; }
.settings-two-fields { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.settings-secret-field { position: relative; }
.settings-secret-field input { padding-right: 40px; }
.settings-secret-field button { position: absolute; right: 3px; top: 3px; bottom: 3px; width: 32px; display: grid; place-items: center; cursor: pointer; }
.settings-presets { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; color: var(--color-faint); font-size: 11px; }
.settings-presets button { color: var(--color-muted); cursor: pointer; text-decoration: underline; text-underline-offset: 3px; }
.settings-recovery { margin-top: 20px; padding: 15px 0; border-top: 1px solid var(--color-hairline); display: flex; justify-content: space-between; align-items: center; gap: 15px; }
.settings-recovery strong { font-size: 12px; font-weight: 500; }
.settings-recovery p { margin-top: 4px; font-size: 11px; color: var(--color-faint); }
.settings-path { overflow-wrap: anywhere; user-select: text; font-family: var(--font-mono); }
.settings-export-notes { margin-top: 24px; border-bottom: 0; border-top: 1px solid var(--color-hairline); }
.settings-hotkey { flex-shrink: 0; min-width: 84px; min-height: 34px; padding: 7px 10px; border: 1px solid var(--color-hairline); border-radius: 5px; background: var(--color-chrome); font-family: var(--font-mono); font-size: 12px; color: var(--color-muted); cursor: pointer; }
.settings-hotkey.is-recording { border-color: var(--color-beacon); color: var(--color-beacon); }
.settings-row-controls { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }
.settings-row-controls input[type='time'] { width: 112px; }
.settings-shortcuts { padding-top: 5px; align-content: start; gap: 22px; }
.settings-shortcuts kbd { font-family: var(--font-mono); font-size: 11px; }
:where(button, input, select, summary):focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 3px; }
.settings-content:focus-visible { outline: 1px solid var(--color-hairline); outline-offset: -1px; }
@container (max-width: 700px) {
  .settings-section { grid-template-columns: 1fr; gap: 18px; padding-block: 24px; }
  .settings-section-intro { display: grid; grid-template-columns: 24px 1fr; column-gap: 8px; }
  .settings-section-number { grid-row: 1 / 4; padding-top: 3px; }
  .settings-section-intro h2 { margin: 0 0 4px; }
  .settings-section-intro p { grid-column: 2; max-width: none; margin-top: 2px; }
  .settings-reset-heading { grid-column: 2; }
  .settings-heading { padding-bottom: 18px; }
  .settings-tabs { gap: 5px; }
  .settings-tabs button { padding-inline: 7px; font-size: 12px; }
}
@container (max-width: 470px) {
  .settings-tabs svg { display: none; }
  .settings-two-fields { grid-template-columns: 1fr; }
  .settings-reset-fields { gap: 6px; }
  .settings-field input, .settings-field select { padding-inline: 6px; }
  .settings-row { flex-wrap: wrap; gap: 10px; }
  .settings-row-controls { margin-left: auto; }
  .settings-recovery, .settings-export-notes { flex-wrap: wrap; }
}
@media (max-height: 600px) {
  .settings-heading { padding-bottom: 12px; }
  .settings-heading h1 { font-size: 24px; margin-block: 0; }
  .settings-heading .settings-eyebrow, .settings-heading h1 + p { display: none; }
  .settings-tabs button { padding-block: 9px; }
}
@media (prefers-reduced-motion: reduce) { *, *::after { transition: none !important; } }
</style>
