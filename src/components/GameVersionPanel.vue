<script setup lang="ts">
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { FolderOpen, RefreshCw, SlidersHorizontal, X } from '@lucide/vue';
import { gameRecords } from '../workspace/core';
import { localPlaytime } from '../workspace/gameActivity';
import { updateLibraryDetails } from '../workspace/guides/local';
import { openInExplorer } from '../services/systemApi';
import { askConfirmation } from '../workspace/confirmation';
import { gameExecutableLabel } from '../services/localGames';

const props = defineProps<{ recordId: string; installationIndex: number }>();
type Switch = { id: string; label: string; current: 'original' | 'modified' | null; original: string; modified: string; files: string[] };
type Versions = { switches: Switch[]; tools: string[] };
const dialog = ref<HTMLDialogElement>(), versions = ref<Versions>(), busy = ref(false), error = ref('');
const installation = computed(() => gameRecords.value.find(r => r.id === props.recordId)?.library?.installations[props.installationIndex]);
const running = computed(() => !!localPlaytime.value.find(p => p.record_id === props.recordId)?.active);
async function load() {
  busy.value = true; error.value = '';
  try { versions.value = await invoke<Versions>('inspect_game_versions', { recordId: props.recordId, installationIndex: props.installationIndex }); }
  catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
function open() { versions.value = undefined; dialog.value?.showModal(); void load(); }
async function openDirectory() {
  if (!installation.value) return;
  try { await openInExplorer(installation.value.directory); } catch (e) { error.value = String(e); }
}
async function executable(value: string) {
  busy.value = true; error.value = '';
  try { await updateLibraryDetails(props.recordId, l => ({ ...l, installations: l.installations.map((i,n) => n === props.installationIndex ? { ...i, executable: value } : i) })); }
  catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
async function switchTo(item: Switch, target: 'original' | 'modified') {
  if (busy.value || running.value || item.current === target) return;
  if (!await askConfirmation({
    title: `切换为${item[target]}？`,
    message: '请先退出游戏。将切换下列资源，原文件会保留；共享目录的其他机器也会使用切换后的资源。',
    details: [{ label: '游戏目录', value: installation.value?.directory || '' }, { label: '资源', value: item.files.join('\n') }],
    confirmLabel: '切换并核验',
  })) return;
  busy.value = true; error.value = '';
  try { versions.value = await invoke<Versions>('switch_game_resources', { recordId: props.recordId, installationIndex: props.installationIndex, switchId: item.id, target }); }
  catch (e) { error.value = String(e); await load(); error.value = String(e); }
  finally { busy.value = false; }
}
</script>
<template>
  <button class="version-entry" @click="open"><SlidersHorizontal />版本与补丁</button>
  <dialog ref="dialog" class="version-dialog" aria-labelledby="version-title" @keydown.esc.stop @cancel.prevent="!busy && dialog?.close()">
    <header><div><p>GAME EDITION</p><h2 id="version-title">版本与补丁</h2></div><button aria-label="关闭版本与补丁" :disabled="busy" @click="dialog?.close()"><X /></button></header>
    <div class="version-body">
      <p class="directory">{{ installation?.directory }}</p>
      <label class="executable-label">启动版本<select aria-label="选择游戏启动版本" :value="installation?.executable" :disabled="busy || running" @change="executable(($event.target as HTMLSelectElement).value)"><option v-for="file in installation?.candidates" :key="file" :value="file">{{ gameExecutableLabel(file) }}</option></select></label>
      <p class="hint">选择后记住本作的启动程序。画面与补丁选项会读取当前文件状态。</p>
      <p v-if="running" role="status">游戏正在运行，退出后可切换版本。</p>
      <section v-for="item in versions?.switches" :key="item.id" class="switch-group">
        <h3>{{ item.label }}</h3><div class="switch-options"><button v-for="target in (['original','modified'] as const)" :key="target" :aria-pressed="item.current === target" :disabled="busy || running || !item.current" @click="switchTo(item,target)">{{ item[target] }}<span v-if="item.current === target">当前</span></button></div>
        <p v-if="!item.current" class="version-error">资源状态不完整，请检查目录后重新读取。</p>
      </section>
      <p v-if="versions && !versions.switches.length" class="hint">{{ versions.tools.length ? '检测到切换工具，当前尚未适配自动切换。可打开目录查看原工具。' : '未发现需要切换的资源方案；可以直接选择启动版本。' }}</p>
      <p v-if="error" class="version-error" role="alert">{{ error }}</p><p v-if="busy" role="status">正在处理并核验文件，请稍候…</p>
    </div>
    <footer><button :disabled="busy" @click="openDirectory"><FolderOpen />打开游戏目录</button><button :disabled="busy" @click="load"><RefreshCw />重新读取</button></footer>
  </dialog>
</template>
<style scoped>
button{display:inline-flex;align-items:center;justify-content:center;gap:7px;padding:8px 11px;min-height:34px;font-size:12px;border:1px solid var(--color-hairline);border-radius:4px;cursor:pointer;color:var(--color-muted)}button:hover{background:var(--color-paper);color:var(--color-beacon)}button:disabled{opacity:.45;cursor:default}svg{width:14px;height:14px;flex:none}.version-entry{border:0;padding-inline:4px}.version-dialog{margin:auto;width:min(620px,92vw);max-height:85dvh;padding:0;background:var(--color-canvas);color:var(--color-ink);border:1px solid var(--color-hairline);border-radius:8px;box-shadow:0 30px 100px #0009}.version-dialog::backdrop{background:#080d10cc;backdrop-filter:blur(8px)}.version-dialog[open]{display:flex;flex-direction:column}header{display:flex;align-items:start;justify-content:space-between;padding:24px 28px 12px}header p{font:10px var(--font-mono);letter-spacing:.16em;color:var(--color-beacon)}h2{font:28px var(--font-display);margin-top:8px}header>button{border:0}.version-body{padding:0 28px 24px;overflow:auto}.directory{font:11px/1.8 var(--font-mono);color:var(--color-muted);overflow-wrap:anywhere;margin-bottom:22px}.executable-label{display:grid;gap:10px;font-size:13px}select{padding:10px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper);min-width:0;font-size:12px;color:var(--color-ink)}.hint,.version-body>p:not(.directory){font-size:12px;line-height:1.8;color:var(--color-muted);margin-top:14px}.switch-group{padding-top:22px;margin-top:22px;border-top:1px solid var(--color-hairline)}h3{font-size:14px;font-weight:500;margin-bottom:12px}.switch-options{display:flex;gap:10px;flex-wrap:wrap}.switch-options [aria-pressed=true]{color:var(--color-beacon);border-color:var(--color-beacon);background:var(--color-paper)}.switch-options span{font-size:10px;opacity:.8}.version-error{color:var(--color-alert)!important;font-size:12px;line-height:1.8;margin-top:10px}footer{display:flex;justify-content:space-between;gap:12px;padding:16px 28px;border-top:1px solid var(--color-hairline)}:where(button,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:600px){header,footer{padding:16px}.version-body{padding:0 16px 20px}}
</style>
