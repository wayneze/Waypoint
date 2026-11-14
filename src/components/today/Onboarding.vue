<script setup lang="ts">
// 首次使用引导：三步，每步做完自动打勾，可以整个跳过。
import { computed, ref } from 'vue';
import { characterList, currentTab, obsidianVaultPath } from '../../workspace/core';
import { enabledGameIds } from '../../services/games';
import { kvGet, kvGetJson, kvSet } from '../../services/kv';
import { ArrowUpRight, Check, ChevronDown } from '@lucide/vue';
import { settingsSection } from '../../workspace/settings';

const DISMISS_KEY = 'waypoint_onboarding_done';
const dismissed = ref(kvGetJson(DISMISS_KEY, false));
// 用户在设置页动过游戏开关才算这步做过
const gamesPicked = computed(() => (enabledGameIds.value, kvGet('enabled_games') !== null));

const steps = computed(() => [
  { id: 'vault', title: '登记一个游戏账号', hint: '在金库里加账号和角色，挂上日常板的角色才出现在今日页。', done: characterList.value.length > 0, tab: 'vault' },
  { id: 'games', title: '选要用的游戏', hint: '设置页可以关掉不玩的游戏，侧栏和提醒都会跟着收起来。', done: gamesPicked.value, tab: 'settings' },
  { id: 'obsidian', title: '绑定 Obsidian 仓库（可选）', hint: '攻略从仓库里读，进度可以写回仓库。', done: !!obsidianVaultPath.value, tab: 'settings' },
]);

const visible = computed(() => !dismissed.value && steps.value.some(s => !s.done));

function dismiss() {
  dismissed.value = true;
  kvSet(DISMISS_KEY, 'true');
}
function openStep(id: string, tab: string) {
  if (id === 'games') settingsSection.value = 'games';
  if (id === 'obsidian') settingsSection.value = 'connections';
  currentTab.value = tab;
}
</script>

<template>
  <div v-if="visible" class="today-onboarding">
    <div class="today-onboarding-heading">
      <div><h2>安排你的第一段旅程</h2><p>登记角色后，游戏里的日常就有了归处。</p></div>
      <button type="button" @click="dismiss">跳过引导</button>
    </div>
    <details :open="!characterList.length"><summary>设置进度 · {{ steps.filter(s => s.done).length }}/{{ steps.length }}<ChevronDown /></summary>
    <ol>
      <li v-for="(s, i) in steps" :key="s.id" :class="{ 'is-done': s.done }">
        <Check v-if="s.done" /><span v-else class="today-step-number">{{ i + 1 }}</span>
        <div><p>{{ s.title }}</p><small>{{ s.hint }}</small></div>
        <button v-if="!s.done" type="button" :aria-label="s.title" @click="openStep(s.id, s.tab)"><ArrowUpRight /></button>
      </li>
    </ol>
    </details>
  </div>
</template>
