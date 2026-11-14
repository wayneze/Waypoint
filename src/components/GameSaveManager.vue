<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { CloudUpload, FolderOpen, HardDriveDownload, RefreshCw, X } from '@lucide/vue';
import { gameSaveRows, saveBatch, saveScan, scanLibrarySaves, runSaveBatch, showGameSaveManager, stopSaveBatch } from '../workspace/gameSaves';
import { saveStatus } from '../services/gameSaves';
import { pickFolder } from '../services/systemApi';
import { gameRecords, currentTab } from '../workspace/core';
import { detailGameId, detailTab } from '../workspace/guides/local';

const dialog = ref<HTMLDialogElement>();
const password = ref(''), upload = ref(false), error = ref('');
const busy = computed(() => saveScan.value.running || saveBatch.value.running);
const selectedCount = computed(() => gameSaveRows.value.filter(r => r.include && r.selected.length).length);
const failed = computed(() => gameSaveRows.value.filter(r => r.phase === 'failed').length);
const success = computed(() => gameSaveRows.value.filter(r => r.phase === 'done').length);
watch(showGameSaveManager, async value => {
  await nextTick();
  if (value) { dialog.value?.showModal(); if (!gameSaveRows.value.length) void scanLibrarySaves(); }
  else { dialog.value?.close(); password.value = ''; }
});
async function run(retry = false) {
  error.value = '';
  try { const secret = password.value; password.value = ''; await runSaveBatch(secret, upload.value, retry); }
  catch (e) { error.value = String(e); }
}
async function manual(id: string) {
  error.value = '';
  try {
    const path = await pickFolder('选择这部游戏专用的存档目录');
    const row = gameSaveRows.value.find(r => r.id === id);
    if (!path || !row) return;
    row.discovery = { candidates: [{ source: { slot: 'manual', path, kind: 'directory' }, reason: '手动选择', status: 'ready', automatic: true }], warnings: [] };
    row.selected = ['manual']; row.include = true; row.phase = 'pending'; row.backupId = undefined; row.error = '';
  } catch (e) { error.value = String(e); }
}
function inspect(id: string) {
  if (!gameRecords.value.some(r => r.id === id)) return;
  detailGameId.value = id; detailTab.value = 'files'; currentTab.value = 'galgame'; showGameSaveManager.value = false;
}
function changed(id: string) {
  const row = gameSaveRows.value.find(r => r.id === id);
  if (row) { row.phase = 'pending'; row.backupId = undefined; row.error = ''; }
}
const phaseLabel = (phase: string) => ({ pending: '', saving: '正在加密备份…', uploading: '正在上传…', done: '已完成', failed: '待重试' }[phase] || '');
</script>
<template>
  <dialog ref="dialog" class="save-manager" aria-labelledby="save-manager-title" @cancel.prevent="showGameSaveManager = false">
    <header><div><p class="eyebrow">SAVE ARCHIVE</p><h2 id="save-manager-title">存档管理</h2><p>集中识别和备份已关联的游戏，保留每部作品自己的加密版本。</p></div><button aria-label="关闭存档管理" @click="showGameSaveManager = false"><X /></button></header>
    <div class="manager-toolbar">
      <span>{{ gameSaveRows.length }} 部作品 · 已选 {{ selectedCount }} 部</span>
      <button :disabled="busy" @click="scanLibrarySaves()"><RefreshCw />重新扫描</button>
      <button :disabled="busy" @click="scanLibrarySaves(true)">重新自动识别</button>
    </div>
    <div v-if="busy || saveBatch.total" class="manager-progress" role="status">
      <span>{{ saveScan.running ? `正在识别 ${saveScan.done} / ${saveScan.total}` : saveBatch.running ? `${saveBatch.current} · ${saveBatch.done} / ${saveBatch.total}` : `${success} 部成功${failed ? ` · ${failed} 部待重试` : ''}${saveBatch.stopped ? ' · 已停止后续处理' : ''}` }}</span>
      <button v-if="saveBatch.running" @click="stopSaveBatch">停止后续处理</button>
      <progress v-if="busy" :value="saveScan.running ? saveScan.done : saveBatch.done" :max="(saveScan.running ? saveScan.total : saveBatch.total) || 1" aria-label="存档处理进度" />
    </div>
    <div class="manager-body">
      <p v-if="!gameSaveRows.length && !saveScan.running" class="manager-empty">还没有已关联安装的作品。先添加本地目录，或在作品中选择存档目录。</p>
      <article v-for="row in gameSaveRows" :key="row.id" class="save-row">
        <div class="row-heading"><label><input v-model="row.include" type="checkbox" :disabled="busy || !row.selected.length" :aria-label="'备份作品：' + row.title" /><strong>{{ row.title }}</strong></label><span :class="{'has-error': row.phase === 'failed'}">{{ phaseLabel(row.phase) }}</span></div>
        <div v-for="candidate in row.discovery.candidates" :key="candidate.source.slot" class="source-row">
          <label><input v-model="row.selected" type="checkbox" :value="candidate.source.slot" :disabled="busy || candidate.status !== 'ready'" :aria-label="'选择存档位置：' + candidate.source.path" @change="changed(row.id)" /><span class="source-path">{{ candidate.source.path }}<small>{{ candidate.reason }}{{ candidate.source.kind === 'nscripter' ? ' · 根目录文件集合' : '' }}</small></span></label><span class="source-status">{{ candidate.automatic ? saveStatus(candidate.status) : '待确认' }}</span>
        </div>
        <p v-if="!row.discovery.candidates.length">{{ row.discovery.warnings.length ? '安装目录离线或无法识别' : '尚未识别到存档位置，可手动选择或运行游戏保存后重试。' }}</p>
        <p v-for="warning in row.discovery.warnings" :key="warning" class="has-error">{{ warning }}</p>
        <p v-if="row.error" class="has-error" role="alert">{{ row.error }}{{ row.backupId ? ' 本地加密版本已保留。' : '' }}</p>
        <div class="row-actions"><button :disabled="busy" @click="manual(row.id)"><FolderOpen />手动选择目录</button><button :disabled="busy" @click="inspect(row.id)">查看本作存档</button></div>
      </article>
    </div>
    <footer>
      <div class="manager-form"><label>本次备份密码<input v-model="password" type="password" autocomplete="off" placeholder="至少 10 个字符" :disabled="busy" /></label><label class="upload-choice"><input v-model="upload" type="checkbox" :disabled="busy" /><CloudUpload />备份后上传 WebDAV</label></div>
      <p v-if="error" class="has-error" role="alert">{{ error }}</p>
      <div class="manager-submit"><p>请先退出游戏。密码不保存；关闭窗口后，当前批次继续处理。</p><button v-if="failed" :disabled="busy" @click="run(true)">仅重试失败项</button><button class="primary" :disabled="busy || !selectedCount" @click="run()"><HardDriveDownload />{{ saveBatch.running ? '备份进行中…' : `备份所选 ${selectedCount} 部` }}</button></div>
    </footer>
  </dialog>
</template>
<style scoped>
.save-manager{width:min(920px,94vw);max-height:90dvh;margin:auto;padding:0;background:var(--color-canvas);color:var(--color-ink);border:1px solid var(--color-hairline);border-radius:8px;box-shadow:0 30px 100px #0009}.save-manager[open]{display:flex;flex-direction:column}.save-manager::backdrop{background:#080d10cc;backdrop-filter:blur(8px)}header{display:flex;justify-content:space-between;gap:20px;padding:24px 28px}.eyebrow{font:10px var(--font-mono);letter-spacing:.16em;color:var(--color-beacon)}h2{font:30px var(--font-display);margin:8px 0}p{font-size:12px;line-height:1.8;color:var(--color-muted)}button{display:inline-flex;align-items:center;justify-content:center;gap:7px;min-height:34px;padding:7px 11px;font-size:12px;border:1px solid var(--color-hairline);border-radius:4px;cursor:pointer}button:hover{background:var(--color-paper);color:var(--color-beacon)}button:disabled{opacity:.45;cursor:default}svg{width:15px;height:15px;flex:none}header>button{align-self:start;border:0}.manager-toolbar,.manager-progress{display:flex;align-items:center;gap:12px;flex-wrap:wrap;padding:12px 28px;border-block:1px solid var(--color-hairline);font-size:12px;color:var(--color-muted)}.manager-toolbar>span,.manager-progress>span{margin-right:auto}.manager-progress{color:var(--color-beacon);border-top:0}progress{flex-basis:100%;width:100%;height:3px;accent-color:var(--color-beacon)}.manager-body{min-height:100px;overflow:auto;padding:0 28px}.save-row{padding:20px 0;border-bottom:1px solid var(--color-hairline)}.row-heading,.source-row{display:flex;align-items:start;justify-content:space-between;gap:16px}.row-heading label,.source-row label{display:flex;gap:10px;align-items:start;min-width:0}.row-heading strong{font-size:15px;font-weight:500}.row-heading>span,.source-status{font-size:12px;white-space:nowrap;color:var(--color-beacon)}input[type=checkbox]{flex:none;margin-top:3px;accent-color:var(--color-beacon);width:15px;height:15px}.source-row{margin:14px 0 0 25px}.source-path{font:11px/1.7 var(--font-mono);overflow-wrap:anywhere}.source-path small{display:block;font:12px/1.8 var(--font-sans);color:var(--color-muted)}.row-actions{display:flex;gap:10px;margin:10px 0 0 25px}.row-actions button{border:0;color:var(--color-muted);padding-inline:0}.save-row>p{margin:10px 0 0 25px;overflow-wrap:anywhere}.manager-empty{padding:36px 0;text-align:center}footer{padding:18px 28px;border-top:1px solid var(--color-hairline)}.manager-form,.manager-submit{display:flex;align-items:end;gap:16px;flex-wrap:wrap}.manager-form>label:not(.upload-choice){display:grid;gap:7px;font-size:12px;flex:1;min-width:180px}input[type=password]{padding:9px 11px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper);font-size:12px;min-width:0}.upload-choice{display:flex;gap:7px;align-items:center;min-height:36px;font-size:12px;color:var(--color-muted)}.manager-submit{margin-top:14px;align-items:center}.manager-submit>p{flex:1;min-width:180px;font-size:11px}.primary{color:var(--color-beacon);border-color:var(--color-beacon)}.has-error{color:var(--color-alert)!important}:where(button,input):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:600px){header,footer,.manager-toolbar,.manager-progress{padding:16px}.manager-body{padding:0 16px}.source-row{margin-left:0;flex-wrap:wrap;gap:4px}.source-status{margin-left:25px}.manager-submit>p{flex-basis:100%}}
</style>
