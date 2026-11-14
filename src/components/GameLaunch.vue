<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Check, ChevronDown, FolderPlus, Gamepad2, Monitor, Play } from '@lucide/vue';
import { gameRecords, showToast } from '../workspace/core';
import { launchingGames, steamActivity } from '../workspace/gameActivity';
import { updateLibraryDetails } from '../workspace/guides/local';
import { rememberGame } from '../workspace/guides/extras';
import { gameExecutableLabel, gameLaunchOptions, installationAvailable, launchGameOption, preferredLaunchOption } from '../services/localGames';
import GameVersionPanel from './GameVersionPanel.vue';

const props = defineProps<{ recordId?: string; title: string; compact?: boolean }>();
const emit = defineEmits<{ setup: []; launching: [] }>();
const saving = ref(false);
const available = ref<Record<string, boolean>>({});
const record = computed(() => gameRecords.value.find(r => r.id === props.recordId));
const link = computed(() => steamActivity.value.links.find(l => l.record_id === props.recordId));
const steamInstalled = computed(() => steamActivity.value.installed.some(i => i.appid === link.value?.appid));
const installations = computed(() => record.value?.library?.installations || []);
const options = computed(() => gameLaunchOptions(installations.value, !!link.value,
  steamInstalled.value, available.value, false));
const choice = computed(() => preferredLaunchOption(options.value, record.value?.library?.preferred_launch));
const busy = computed(() => saving.value || launchingGames.value.includes(props.recordId || ''));
const selected = computed(() => options.value.find(o => o.key === choice.value));
const label = computed(() => !options.value.length ? '关联安装' : !selected.value ? '请选择版本' : selected.value.key === 'steam' && !steamInstalled.value ? '通过 Steam 安装' : '开始游戏');
watch(() => props.recordId, () => { available.value = {}; });
function versionLabel(index?: number) {
  if (index === undefined) return 'Steam';
  const installation = installations.value[index];
  return installations.value.length === 1 && installation ? gameExecutableLabel(installation.executable) : installation?.directory.split(/[\\/]/).filter(Boolean).pop();
}
async function choose(key: string) {
  if (!record.value || busy.value) return;
  saving.value = true;
  try {
    await updateLibraryDetails(record.value.id, library => ({ ...library, preferred_launch: key }));
  } catch (e) { showToast('启动方式未保存', String(e)); }
  finally { saving.value = false; }
}
async function launch(key = choice.value) {
  if (!options.value.length) { emit('setup'); return; }
  if (!key) return;
  const item = options.value.find(o => o.key === key);
  const id = props.recordId;
  if (!id || !item || busy.value) return;
  const title = props.title;
  launchingGames.value.push(id);
  emit('launching');
  try {
    const directory = item.installationIndex === undefined ? undefined : installations.value[item.installationIndex]?.directory;
    const ready = !directory || await installationAvailable(directory);
    if (directory && props.recordId === id) available.value[directory] = ready;
    await launchGameOption(id, { ...item, available: ready });
    showToast(item.key === 'steam' ? '已交给 Steam' : '已启动本地程序', title);
    await rememberGame(id).catch(() => showToast('最近游玩顺序未保存', '启动请求已发出'));
  } catch (e) { showToast('游戏未启动', String(e)); }
  finally { launchingGames.value = launchingGames.value.filter(value => value !== id); }
}
</script>
<template>
  <div v-if="!compact || options.length" class="game-launch" :class="{'is-compact': compact}">
    <button class="launch-main" :disabled="busy || (!!options.length && !choice)" :aria-label="label + '：' + title" @click="launch()"><Play v-if="options.length" /><FolderPlus v-else />{{ saving ? '保存中…' : busy ? '正在启动…' : label }}</button>
    <label v-if="compact && options.length > 1" class="launch-picker" :title="selected ? '启动版本：' + versionLabel(selected.installationIndex) : '选择启动版本'">
      <ChevronDown aria-hidden="true" />
      <select class="launch-select" aria-label="启动版本" :value="choice" :disabled="busy" @change="choose(($event.target as HTMLSelectElement).value)">
        <option v-if="!choice" value="" disabled>选择版本</option>
        <option v-for="option in options" :key="option.key" :value="option.key">{{ versionLabel(option.installationIndex) }}</option>
      </select>
    </label>
    <div v-else-if="!compact && options.length" class="launch-versions" role="group" aria-label="启动版本">
      <button v-for="option in options" :key="option.key" :disabled="busy" :aria-pressed="choice === option.key" :title="option.label" @click="choose(option.key)">
        <Gamepad2 v-if="option.key === 'steam'" /><Monitor v-else />
        <span>{{ versionLabel(option.installationIndex) }}</span><Check v-if="choice === option.key" class="version-selected" />
      </button>
    </div>
    <GameVersionPanel v-if="!compact && recordId && selected?.installationIndex !== undefined" :key="recordId + ':' + selected.installationIndex" :record-id="recordId" :installation-index="selected.installationIndex" />
    <p v-if="options.length && !choice" role="status">原默认版本已移除，请选择要使用的版本。</p>
    <p v-else-if="selected?.installationIndex !== undefined && available[installations[selected.installationIndex]!.directory] === false" role="alert">本地目录不可用，请在“安装”检查路径。</p>
  </div>
</template>
<style scoped>
.game-launch{display:flex;align-items:center;gap:12px;flex-wrap:wrap;min-width:0}button{display:inline-flex;align-items:center;justify-content:center;gap:7px;padding:8px 13px;font-size:12px;cursor:pointer;border:1px solid transparent;border-radius:4px;min-height:35px;color:var(--color-muted);background:transparent}button:disabled{opacity:.5;cursor:default}button:hover:not(:disabled){background:var(--color-paper);color:var(--color-ink)}svg{width:15px;height:15px;flex-shrink:0}.launch-main{background:color-mix(in srgb,var(--color-beacon) 8%,var(--color-canvas));color:var(--color-beacon);border-color:color-mix(in srgb,var(--color-beacon) 45%,var(--color-hairline));font-weight:500}.launch-versions{display:flex;align-items:center;flex-wrap:wrap;gap:4px;min-width:0}.launch-versions button{text-align:left;max-width:100%;padding-inline:9px}.launch-versions span{overflow-wrap:anywhere}.launch-versions [aria-pressed=true]{background:var(--color-paper);color:var(--color-beacon)}.version-selected{width:12px;height:12px}p{flex-basis:100%;color:var(--color-alert);font-size:12px}:where(button):focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}
.is-compact{gap:2px}.is-compact .launch-main{padding:4px 0;font-size:12px;min-height:28px;border:0;background:transparent;color:var(--color-muted);font-weight:400}.is-compact .launch-main:hover:not(:disabled){background:var(--color-paper);color:var(--color-beacon)}.is-compact svg{width:13px;height:13px}.launch-picker{position:relative;display:flex;align-items:center;justify-content:center;width:24px;height:28px;border-radius:3px}.launch-picker:hover{background:var(--color-paper);color:var(--color-beacon)}.launch-picker:focus-within{outline:2px solid var(--color-beacon);outline-offset:2px}.launch-select{position:absolute;inset:0;width:100%;height:100%;opacity:0;cursor:pointer}.launch-select:disabled{cursor:default}</style>
