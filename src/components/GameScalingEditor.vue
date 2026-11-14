<script setup lang="ts">
import { computed, ref } from 'vue';
import type { GameInstallation, GameScalingProfile } from '../types';
import { scalingProfile } from '../services/gameScaling';
import { installationKey } from '../services/localGames';
const props = defineProps<{ installations: GameInstallation[] }>();
const model = defineModel<GameScalingProfile[]>({ required: true });
const selected = ref(0);
const installation = computed(() => props.installations[selected.value] || props.installations[0]);
const profile = computed(() => scalingProfile(model.value, installation.value?.directory || ''));
function update(changes: Partial<GameScalingProfile>) {
  if (!installation.value) return;
  const next = { ...profile.value, ...changes };
  model.value = [...model.value.filter(p => installationKey(p.directory) !== installationKey(next.directory)), next];
}
</script>
<template>
  <div class="scaling-editor">
    <template v-if="installations.length">
      <p>直接使用内置缩放核心，无需另外打开 Magpie。游戏先使用窗口模式，再放大到屏幕或独立缩放窗口。</p>
      <label v-if="installations.length > 1">安装版本<select v-model.number="selected" aria-label="缩放安装版本"><option v-for="(item,index) in installations" :key="item.directory" :value="index">{{ item.directory.split(/[\\/]/).pop() }} · {{ item.executable.split(/[\\/]/).pop() }}</option></select></label>
      <p class="install-path">{{ installation?.directory }}</p>
      <label class="scaling-auto"><input type="checkbox" :checked="profile.auto_start" aria-label="启动时自动缩放" @change="update({auto_start: ($event.target as HTMLInputElement).checked})" /><span>从 Waypoint 启动此版本时自动缩放<small>先等待游戏窗口，切出时暂停，返回游戏后继续。</small></span></label>
      <label>画面预设<select :value="profile.preset" aria-label="画面缩放预设" @change="update({preset: ($event.target as HTMLSelectElement).value as GameScalingProfile['preset']})"><option value="fit">基础适屏 · 低负载</option><option value="anime">Anime4K · 插画与视觉小说</option><option value="sharp">FSR · 清晰锐化</option><option value="pixel">最近邻 · 像素画面</option></select></label>
      <label>显示方式<select :value="profile.mode" aria-label="缩放显示方式" @change="update({mode: ($event.target as HTMLSelectElement).value as GameScalingProfile['mode']})"><option value="fullscreen">全屏等比适配</option><option value="window">放大窗口</option></select></label>
      <label>捕获方式<select :value="profile.capture" aria-label="缩放捕获方式" @change="update({capture: ($event.target as HTMLSelectElement).value as GameScalingProfile['capture']})"><option value="auto">自动 · 按窗口类型选择</option><option value="wgc">Windows Graphics Capture</option><option value="gdi">GDI · 旧引擎兼容</option></select></label>
      <p>保持原画面比例，不拉伸。遇到黑屏可切换捕获方式；已经全屏的游戏请先切回窗口模式。</p>
      <p class="shortcut">停止缩放：<kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>F10</kbd>，或在作品详情中点击“停止缩放”。</p>
      <p class="credit">缩放核心：Magpie 0.12.1 · GPL-3.0-or-later</p>
    </template>
    <p v-else>先在作品详情的“安装与存档”中关联本地安装，再为对应版本设置画面缩放。</p>
  </div>
</template>
<style scoped>
.scaling-editor{display:grid;gap:18px;font-size:12px;color:var(--color-muted);line-height:1.8}.scaling-editor label{display:grid;gap:7px}.scaling-editor select{width:100%;padding:9px 10px;min-height:38px;background:var(--color-chrome);border:1px solid var(--color-hairline);border-radius:4px;color:var(--color-ink);font-size:12px}.scaling-auto{display:flex!important;align-items:start;gap:10px!important;padding:14px;background:var(--color-paper);border-radius:4px;color:var(--color-ink)}.scaling-auto input{margin-top:5px;accent-color:var(--color-beacon)}small{display:block;color:var(--color-muted);font-size:11px}.install-path{font:11px/1.8 var(--font-mono);overflow-wrap:anywhere;color:var(--color-faint)}kbd{border:1px solid var(--color-hairline);border-radius:3px;padding:2px 5px;background:var(--color-paper);font-size:11px}.credit{font-size:11px;color:var(--color-faint);border-top:1px solid var(--color-hairline);padding-top:12px}:where(input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
