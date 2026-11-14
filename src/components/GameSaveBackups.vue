<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { CloudDownload, CloudUpload, FolderOpen, HardDriveDownload, RefreshCw, RotateCcw } from '@lucide/vue';
import { invoke } from '@tauri-apps/api/core';
import type { GameRecord, GameSaveDiscovery } from '../types';
import { pickFolder } from '../services/systemApi';
import { describeError } from '../services/errors';
import { currentTab } from '../workspace/core';
import { settingsSection } from '../workspace/settings';
import { askConfirmation } from '../workspace/confirmation';
import { discoverGameSaves, listSaveBackups, saveStatus, type SaveVersion as Version } from '../services/gameSaves';
import { setGameSaveSources, showGameSaveManager } from '../workspace/gameSaves';
import { localPlaytime } from '../workspace/gameActivity';

const props = defineProps<{ record: GameRecord }>();
const versions = ref<Version[]>([]);
const remote = ref(false), busy = ref(false), loading = ref(false);
const password = ref(''), error = ref(''), notice = ref('');
const discovery = ref<GameSaveDiscovery>({ candidates: [], warnings: [] });
const selected = ref<string[]>([]);
const chosen = computed(() => discovery.value.candidates.filter(c => selected.value.includes(c.source.slot)));
const path = computed(() => chosen.value.map(c => c.source.path).join('\n'));
const ready = computed(() => chosen.value.length > 0 && chosen.value.every(c => c.status === 'ready'));
let generation = 0;
let detecting = 0;
async function detect(automaticOnly = false) {
  const ticket = ++detecting, id = props.record.id;
  busy.value = true; error.value = '';
  try {
    const result = await discoverGameSaves(id, automaticOnly);
    if (ticket !== detecting) return;
    discovery.value = result;
    selected.value = result.candidates.filter(c => c.automatic && (c.status === 'ready' || c.status === 'missing')).map(c => c.source.slot);
  } catch (e) { if (ticket === detecting) error.value = describeError(e).message; }
  finally { if (ticket === detecting) busy.value = false; }
}
async function load() {
  const ticket = ++generation, id = props.record.id;
  loading.value = true; error.value = '';
  try {
    const rows = await listSaveBackups(id, remote.value);
    if (ticket === generation) versions.value = rows;
  } catch (e) { if (ticket === generation) error.value = describeError(e).message; }
  finally { if (ticket === generation) loading.value = false; }
}
watch(() => props.record.id, () => { password.value = ''; notice.value = ''; versions.value = []; discovery.value = { candidates: [], warnings: [] }; selected.value = []; remote.value = false; void load(); void detect(); }, { immediate: true });
watch(() => !!localPlaytime.value.find(p => p.record_id === props.record.id)?.active, (active, wasActive) => { if (wasActive && !active && !busy.value) void detect(); });
function view(value: boolean) { if (busy.value || remote.value === value) return; remote.value = value; versions.value = []; void load(); }
async function choosePath() {
  if (busy.value) return;
  busy.value = true; error.value = '';
  const id = props.record.id;
  try {
    const selected = await pickFolder('选择这部游戏专用的存档目录');
    if (!selected) return;
    await setGameSaveSources(id, [{ slot: 'manual', path: selected, kind: 'directory' }]);
    await detect();
  } catch (e) { error.value = describeError(e).message; }
  finally { busy.value = false; }
}
async function run(action: 'create' | 'upload' | 'download' | 'restore', version?: Version) {
  if (busy.value) return;
  error.value = ''; notice.value = '';
  if (action !== 'upload' && [...password.value].length < 10) { error.value = '请填写至少 10 个字符的备份密码；恢复时使用创建备份时的同一密码。'; return; }
  busy.value = true;
  const recordId = props.record.id;
  try {
    if (action === 'restore' && !await askConfirmation({
      title: '恢复这份游戏存档？',
      message: '请先退出游戏。将用所选版本替换当前存档目录；恢复前会自动保留当前存档为新版本，可用它撤回。',
      details: [{ label: '作品', value: props.record.title }, { label: '目标目录', value: path.value }, { label: '备份时间', value: date(version!.created_at) }],
      confirmLabel: '备份当前存档并恢复',
    })) return;
    if (action === 'create' || action === 'restore') await setGameSaveSources(recordId, chosen.value.map(c => c.source));
    const result = await invoke<Version | null>(`${action}_game_save_backup`, {
      recordId, id: version?.id, ...(action !== 'upload' ? { password: password.value } : {}),
    });
    password.value = '';
    if (action === 'create') { remote.value = false; notice.value = '本地加密备份已保存，可在下方上传云端。'; }
    if (action === 'upload') notice.value = '这个版本已上传到已配置的 WebDAV。';
    if (action === 'download') { remote.value = false; notice.value = '已下载并校验，可在本地版本中选择恢复。'; }
    if (action === 'restore') notice.value = `存档已恢复。恢复前的存档已保存为 ${date(result!.created_at)} 的本地版本。`;
    await load();
  } catch (e) { error.value = describeError(e).message; }
  finally { busy.value = false; }
}
function date(value: number) { return new Date(value).toLocaleString('zh-CN', { hour12: false }); }
function size(value: number) { return value < 1024 * 1024 ? `${(value / 1024).toFixed(1)} KB` : `${(value / 1024 / 1024).toFixed(1)} MB`; }
function settings() { settingsSection.value = 'backup'; currentTab.value = 'settings'; }
</script>

<template>
  <section class="save-backups" aria-label="游戏存档备份" :aria-busy="busy">
    <header><h3>存档备份</h3><button class="text-action" :disabled="busy" @click="showGameSaveManager = true">批量管理</button><button class="text-action" :disabled="busy" @click="settings">云端设置</button></header>
    <p>自动识别游戏目录、配置指定的文档与 AppData 位置。退出游戏后可备份，云端使用已有 WebDAV。</p>
    <div class="save-path"><button :disabled="busy" @click="detect(true)"><RefreshCw />重新自动识别</button><button :disabled="busy" @click="choosePath"><FolderOpen />手动选择目录</button></div>
    <div v-for="candidate in discovery.candidates" :key="candidate.source.slot" class="save-candidate">
      <label><input v-model="selected" type="checkbox" :value="candidate.source.slot" :disabled="busy || candidate.status === 'unavailable'" /><span>{{ candidate.source.path }}<small>{{ candidate.reason }} · {{ saveStatus(candidate.status) }}{{ !candidate.automatic ? ' · 请确认' : '' }}</small></span></label>
    </div>
    <p v-if="!busy && !discovery.candidates.length">未识别到存档。可先运行游戏保存后重新识别，或手动选择目录。</p>
    <p v-for="warning in discovery.warnings" :key="warning" class="save-error">{{ warning }}</p>
    <div class="save-create">
      <label>备份密码<input v-model="password" type="password" autocomplete="off" :disabled="busy" placeholder="至少 10 个字符，恢复时需同一密码" /></label>
      <button :disabled="busy || !ready" @click="run('create')"><HardDriveDownload />{{ busy ? '处理中…' : '备份当前存档' }}</button>
    </div>
    <p class="save-hint">密码仅用于本次操作，不会保存。上传已加密的本地版本无需再次输入。</p>
    <div class="save-toolbar"><nav aria-label="存档备份位置"><button :aria-pressed="!remote" :disabled="busy" @click="view(false)">本地版本</button><button :aria-pressed="remote" :disabled="busy" @click="view(true)">云端版本</button></nav><button :disabled="busy || loading" @click="load"><RefreshCw />刷新</button></div>
    <p v-if="error" class="save-error" role="alert">{{ error }}</p><p v-if="notice" class="save-notice" role="status">{{ notice }}</p>
    <p v-if="loading">正在读取备份列表…</p>
    <ul v-else-if="versions.length">
      <li v-for="version in versions" :key="version.id"><div><time :datetime="new Date(version.created_at).toISOString()">{{ date(version.created_at) }}</time><small v-if="version.bytes">{{ size(version.bytes) }} · 加密存档</small><small v-else>云端加密存档</small></div><div class="save-actions">
        <button v-if="remote" :disabled="busy" @click="run('download', version)"><CloudDownload />下载并校验</button>
        <template v-else><button :disabled="busy" @click="run('upload', version)"><CloudUpload />上传</button><button :disabled="busy || !chosen.length" @click="run('restore', version)"><RotateCcw />恢复</button></template>
      </div></li>
    </ul>
    <p v-else-if="!error">{{ remote ? '云端还没有这部作品的存档备份。' : '还没有本地备份。选择存档目录后即可创建第一份。' }}</p>
  </section>
</template>

<style scoped>
.save-backups{margin-top:28px;padding-top:22px;border-top:1px solid var(--color-hairline)}
.save-candidate{margin:12px 0}.save-candidate label{display:flex;align-items:start;gap:10px;min-width:0;overflow-wrap:anywhere}.save-candidate input{margin-top:3px;accent-color:var(--color-beacon)}.save-candidate small{display:block;font-size:11px;color:var(--color-muted);margin-top:5px}
header,.save-path,.save-toolbar,li{display:flex;align-items:center;justify-content:space-between;gap:14px}
h3{font:20px var(--font-display)}p{font-size:12px;line-height:1.8;color:var(--color-muted);margin:12px 0}
button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:30px;padding:5px 9px;font-size:12px;border:1px solid var(--color-hairline);border-radius:4px;background:transparent;color:var(--color-muted);cursor:pointer}
button:hover{color:var(--color-beacon);background:var(--color-paper)}button:disabled{opacity:.4;cursor:default}svg{width:14px;height:14px;flex:none}.text-action{border:0;color:var(--color-beacon)}
.save-path{flex-wrap:wrap;font-size:12px;margin:18px 0}.save-path span{overflow-wrap:anywhere;min-width:0;flex:1}.save-create{display:flex;align-items:end;gap:12px;flex-wrap:wrap}
label{display:grid;gap:6px;font-size:11px;color:var(--color-muted);flex:1;min-width:240px}input{padding:8px 10px;min-width:0;font-size:12px;color:var(--color-ink);background:var(--color-chrome);border:1px solid var(--color-hairline);border-radius:4px}
.save-create>button{min-height:36px;color:var(--color-beacon)}.save-hint{font-size:11px;color:var(--color-faint)}.save-toolbar{margin-top:20px;padding-bottom:10px;border-bottom:1px solid var(--color-hairline)}
nav{display:flex;gap:8px}nav button{border:0}nav [aria-pressed=true]{color:var(--color-beacon);background:var(--color-paper)}
ul{list-style:none;padding:0}li{padding:13px 0;border-bottom:1px solid var(--color-hairline);font-size:12px;flex-wrap:wrap}small{display:block;color:var(--color-faint);margin-top:4px}.save-actions{display:flex;gap:8px}.save-error{color:var(--color-alert)}.save-notice{color:var(--color-beacon)}
:where(button,input):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
