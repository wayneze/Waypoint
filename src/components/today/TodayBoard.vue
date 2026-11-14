<script setup lang="ts">
import { computed, reactive } from 'vue';
import { Check, ChevronDown, Search } from '@lucide/vue';
import HomeLineRow from '../HomeLineRow.vue';
import type { HomeLine, HomePeriodSection } from '../../workspace/daily';
import { weeklyDueText } from '../../workspace/daily';
import { currentTab, handleDeleteMatrixTask, handleRenameTask, handleToggleMatrixTask, showToast } from '../../workspace/core';
import { handleDeleteMemo, handleProgressChange, handleRenameMemo, handleToggleMemo, handleToggleMemoStep } from '../../workspace/memos';
import { toggleSkyGroup } from '../../workspace/sky/plan';
import { askConfirmation } from '../../workspace/confirmation';
import GameStampIcon from '../GameStampIcon.vue';

const props = defineProps<{ section: HomePeriodSection; query: string; scope: string }>();
const emit = defineEmits<{ add: []; clear: [] }>();
const busy = reactive(new Set<string>());
const doneOpen = computed(() => props.query.trim() ? true : undefined);
async function run(line: HomeLine, action: () => unknown | Promise<unknown>) {
  if (busy.has(line.id)) return;
  busy.add(line.id);
  try { await action(); }
  catch (error) { showToast('未能保存更改', String(error), { persistent: true }); }
  finally { busy.delete(line.id); }
}
async function toggle(line: HomeLine) {
  await run(line, () => line.kind === 'sky' ? toggleSkyGroup(line.group, line.charId)
    : line.kind === 'task' ? handleToggleMatrixTask(line.task) : handleToggleMemo(line.memo));
}
async function rename(line: HomeLine, title: string) {
  if (line.kind === 'sky') return;
  await run(line, () => line.kind === 'task' ? handleRenameTask(line.task, title, line.charId) : handleRenameMemo(line.memo, title));
}
async function remove(line: HomeLine) {
  if (line.kind === 'sky' || !await askConfirmation({ title: '删除这条待办？', message: `「${line.title}」将被永久删除。${line.kind === 'memo' ? '万象记事中的同一条记录也会删除。' : '如需每天继续做，请使用完成按钮。'}`, confirmLabel: '删除待办' })) return;
  await run(line, () => line.kind === 'task' ? handleDeleteMatrixTask(line.task.id, line.charId) : handleDeleteMemo(line.memo.id));
}
async function progress(line: HomeLine, delta: number) {
  if (line.kind === 'memo') await run(line, () => handleProgressChange(line.memo, delta));
}
async function step(line: HomeLine, index: number) {
  if (line.kind === 'memo') await run(line, () => handleToggleMemoStep(line.memo, index));
}
</script>

<template>
  <div class="today-board">
    <header class="today-section-heading"><h2>{{ section.label }}</h2><span>{{ section.remaining }} 项待办</span></header>
    <section v-for="group in section.groups" :key="group.gameId" class="today-group" :data-game="group.stamp" :aria-label="group.label + section.label">
      <header class="today-group-heading"><GameStampIcon :game="group.stamp" class="w-4 h-4" /><h3>{{ group.label }}</h3><span>{{ group.lines.length }}</span><p v-if="group.calendar" :title="group.calendar">{{ group.calendar }}</p><small v-if="section.id === 'weekly' && weeklyDueText(group.gameId)">{{ weeklyDueText(group.gameId) }}</small></header>
      <HomeLineRow v-for="line in group.lines" :key="line.id" :line="line" :busy="busy.has(line.id)" @toggle="toggle(line)" @rename="rename(line, $event)" @delete="remove(line)" @progress="progress(line, $event)" @step="step(line, $event)" />
    </section>
    <div v-if="!section.remaining && (!query.trim() || !section.total)" class="today-empty">
      <Search v-if="query.trim()" /><Check v-else />
      <h3>{{ query.trim() ? '没有找到对应的待办' : section.total ? '这一栏的事，已经做好了' : '给今天留一点自己的安排' }}</h3>
      <p>{{ query.trim() ? '可以搜索事项、角色或备注，已完成项也会一起查找。' : section.done ? '已完成的记录收在下方，随时可以恢复。' : section.id === 'memo' ? '万象记事中的心愿、截止事项会出现在这里。' : '添加一件小事，之后按游戏的重置时间自动轮转。' }}</p>
      <button v-if="query.trim()" type="button" @click="emit('clear')">清除搜索</button>
      <button v-else-if="section.id === 'memo'" type="button" @click="currentTab = 'memo'">前往万象记事</button>
      <button v-else type="button" @click="emit('add')">添加待办</button>
    </div>
    <details v-if="section.completed.length" :key="section.id + scope" class="today-completed" :open="doneOpen">
      <summary><Check /><span>已完成</span><b>{{ section.done }}</b><small>展开回看或恢复</small><ChevronDown /></summary>
      <HomeLineRow v-for="line in section.completed" :key="line.id" :line="line" :busy="busy.has(line.id)" @toggle="toggle(line)" @rename="rename(line, $event)" @delete="remove(line)" @progress="progress(line, $event)" @step="step(line, $event)" />
    </details>
  </div>
</template>
