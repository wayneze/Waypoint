<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Maximize2, Settings2, Square } from '@lucide/vue';
import { gameRecords, showToast } from '../workspace/core';
import { isTauri } from '../services/api';
import { installationKey } from '../services/localGames';
import { getGameScaling, scalingPhaseLabel, scalingProfile, startGameScaling, stopGameScaling, type GameScalingStatus } from '../services/gameScaling';
const props = defineProps<{ recordId: string; installationIndex: number }>();
const emit = defineEmits<{ settings: [] }>();
const status = ref<GameScalingStatus>({ record_id: null, directory: null, phase: 'idle', message: '' });
const busy = ref(false), error = ref('');
let unlisten: UnlistenFn | undefined, disposed = false;
onMounted(async () => {
  if (!isTauri()) return;
  try {
    const off = await listen<GameScalingStatus>('game-scaling-updated', e => { status.value = e.payload; });
    if (disposed) { off(); return; }
    unlisten = off;
    status.value = await getGameScaling();
  } catch (e) { error.value = String(e); }
});
onUnmounted(() => { disposed = true; unlisten?.(); });
const record = computed(() => gameRecords.value.find(r => r.id === props.recordId));
const installation = computed(() => record.value?.library?.installations[props.installationIndex]);
const profile = computed(() => scalingProfile(record.value?.library?.scaling, installation.value?.directory || ''));
const running = computed(() => !['idle','error'].includes(status.value.phase));
const thisGame = computed(() => status.value.record_id === props.recordId
  && installationKey(status.value.directory || '') === installationKey(installation.value?.directory || ''));
const presets = { fit: '基础适屏', anime: 'Anime4K', sharp: 'FSR', pixel: '最近邻' };
async function apply() {
  busy.value = true; error.value = '';
  try {
    await startGameScaling(props.recordId, props.installationIndex);
    status.value = await getGameScaling();
    showToast('缩放已准备', '返回游戏后自动应用；Ctrl + Alt + F10 可停止');
  } catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
async function stop() {
  busy.value = true;
  try { await stopGameScaling(); } catch (e) { error.value = String(e); }
  finally { busy.value = false; }
}
</script>
<template>
  <section class="scaling-panel" aria-label="画面缩放">
    <header><h3><Maximize2 />画面缩放</h3><button type="button" @click="emit('settings')"><Settings2 />设置预设</button></header>
    <p>{{ presets[profile.preset] }} · {{ profile.mode === 'window' ? '放大窗口' : '全屏等比' }} · {{ profile.auto_start ? '随启动自动应用' : '手动开启' }}</p>
    <div class="scaling-actions"><button type="button" :disabled="busy || running || !installation" @click="apply"><Maximize2 />{{ busy ? '处理中…' : '立即应用' }}</button><button v-if="running" type="button" :disabled="busy" @click="stop"><Square />停止缩放{{ thisGame ? '' : '（其他版本或作品）' }}</button><span v-if="thisGame" role="status">{{ scalingPhaseLabel[status.phase] }}</span></div>
    <p>游戏需为窗口模式。切出暂停，返回继续；Ctrl + Alt + F10 停止。</p>
    <p v-if="error || (thisGame && status.message)" role="alert" class="scaling-error">{{ error || status.message }}</p>
  </section>
</template>
<style scoped>
.scaling-panel{display:grid;gap:12px;padding:22px 0;border-bottom:1px solid var(--color-hairline);font-size:12px;color:var(--color-muted);line-height:1.8}header,.scaling-actions{display:flex;align-items:center;gap:10px;flex-wrap:wrap}header{justify-content:space-between}h3{font:18px var(--font-display);color:var(--color-ink);display:flex;align-items:center;gap:9px}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:34px;padding:6px 10px;border:1px solid var(--color-hairline);border-radius:4px;font-size:12px;color:var(--color-beacon);cursor:pointer}button:hover{background:var(--color-paper)}button:disabled{opacity:.4;cursor:default}svg{width:14px;height:14px}.scaling-actions span{font-size:11px;color:var(--color-beacon)}.scaling-error{color:var(--color-alert);overflow-wrap:anywhere}:where(button):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
