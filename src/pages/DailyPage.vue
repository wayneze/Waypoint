<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from 'vue';
import { ArrowUpRight, CalendarDays, Check, ChevronDown, Ellipsis, Plus, Search, X } from '@lucide/vue';
import {
  allAccountTasks, characterList, currentTab, gameIdOf, gameResetEpoch,
  getCharBoardTitle, getCharBoardHint, isProjectingObsidian, resetAllDailies, resetToday,
  selectScope, selectedScope, showImportModal, showToast,
} from '../workspace/core';
import { gameMetaOf, isGameEnabled } from '../services/games';
import {
  hasJx3Characters, homeBriefingNotice, homeCharStats, homeMatrixStats,
  homeNowItems, homeSectionsFor, type HomePeriod,
} from '../workspace/daily';
import { handleProjectToObsidian } from '../workspace/bootstrap';
import { attachJx3CalendarSlot, findJx3CalendarTask, syncJx3DailyCalendar } from '../workspace/jx3/calendar';
import { JX3_CALENDAR_SLOTS } from '../services/jx3Schedule';
import { askConfirmation } from '../workspace/confirmation';
import GameStampIcon from '../components/GameStampIcon.vue';
import TodayBoard from '../components/today/TodayBoard.vue';
import TodayTaskAdd from '../components/today/TodayTaskAdd.vue';
import Onboarding from '../components/today/Onboarding.vue';
import TodayJx3 from '../components/today/TodayJx3.vue';
import TodayEnding from '../components/today/TodayEnding.vue';
import TodayStats from '../components/today/TodayStats.vue';
import '../styles/today.css';
const TodaySky = defineAsyncComponent(() => import('../components/today/TodaySky.vue'));
const period = ref<HomePeriod>('daily');
const pane = ref<'tasks' | 'briefing'>('tasks');
const query = ref('');
const adding = ref(false);
const operationsBusy = ref(false);
const tasks = ref<HTMLElement>();
const focusedChar = computed(() => characterList.value.find(char => char.id === selectedScope.value));
const sections = computed(() => homeSectionsFor(selectedScope.value));
const filteredSections = computed(() => homeSectionsFor(selectedScope.value, query.value));
const section = computed(() => filteredSections.value.find(section => section.id === period.value)!);
const daily = computed(() => sections.value[0]);
const percent = computed(() => daily.value.total ? Math.round(daily.value.done / daily.value.total * 100) : 0);
const dateLabel = computed(() => { gameResetEpoch.value; return new Date().toLocaleDateString('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }); });
const hasBriefing = computed(() => homeNowItems.value.length > 0 || isGameEnabled('sky') || hasJx3Characters.value);
const resetHint = computed(() => {
  gameResetEpoch.value;
  if (!focusedChar.value) return '各游戏按自己的重置时间轮转';
  const game = gameMetaOf(focusedChar.value.game_id);
  return period.value === 'weekly' ? game.weeklyReset + ' 复位' : period.value === 'daily' ? game.dailyReset + ' 复位' : '与万象记事保持同步';
});
watch(hasBriefing, value => { if (!value) pane.value = 'tasks'; });
watch(characterList, chars => { if (selectedScope.value !== 'all' && !chars.some(char => char.id === selectedScope.value)) selectScope('all'); });
watch([selectedScope, period, query], () => { if (tasks.value) tasks.value.scrollTop = 0; });
function closeMenu(event: Event) {
  const menu = (event.currentTarget as HTMLElement).closest('details'); menu?.removeAttribute('open');
  if (event instanceof KeyboardEvent) menu?.querySelector('summary')?.focus();
}
async function operate(action: () => Promise<unknown>) {
  if (operationsBusy.value) return;
  operationsBusy.value = true;
  try { await action(); } catch (error) { showToast('操作未完成', String(error), { persistent: true }); }
  finally { operationsBusy.value = false; }
}
async function reset() {
  const scope = selectedScope.value;
  if (!await askConfirmation({ title: '重新开始今日日常？', message: scope === 'all' ? '所有角色已完成的日常任务和光遇日常将恢复为待办。周常和万象记事保持各自的周期。' : `「${focusedChar.value?.name || '当前角色'}」已完成的日常任务和光遇日常将恢复为待办。`, confirmLabel: '重置日常' })) return;
  await operate(() => scope === 'all' ? resetAllDailies() : resetToday());
}
function slotAttached(key: (typeof JX3_CALENDAR_SLOTS)[number]['key']) {
  const slot = JX3_CALENDAR_SLOTS.find(slot => slot.key === key)!;
  return !!findJx3CalendarTask(allAccountTasks.value[selectedScope.value] || [], key, slot.mark);
}
async function attachSlot(key: (typeof JX3_CALENDAR_SLOTS)[number]['key']) {
  const id = selectedScope.value;
  await operate(async () => { if (await attachJx3CalendarSlot(id, key)) await syncJx3DailyCalendar(); });
}
function add() {
  if (period.value === 'memo') { currentTab.value = 'memo'; return; }
  pane.value = 'tasks'; adding.value = true;
}
</script>

<template>
  <section class="today-workspace" aria-label="今日待办">
    <header class="today-heading">
      <div><p class="today-date"><CalendarDays />{{ dateLabel }}</p><h1>今日待办<span>.</span></h1><p class="today-subtitle">日常、周常与惦记的事，一件件慢慢完成。</p></div>
      <div class="today-heading-tools">
        <button type="button" class="today-button is-primary" :disabled="period !== 'memo' && !characterList.length" @click="add"><Plus />{{ period === 'memo' ? '添加记事' : '添加待办' }}</button>
        <details class="today-actions" @keydown.esc.prevent.stop="closeMenu"><summary class="today-icon-button" aria-label="更多今日操作"><Ellipsis /></summary><div class="today-actions-menu" @click="closeMenu">
          <button type="button" :disabled="isProjectingObsidian" @click="handleProjectToObsidian('dailies')">{{ isProjectingObsidian ? '投影中…' : '投影到 Obsidian' }}</button>
          <button v-if="hasJx3Characters" type="button" :disabled="operationsBusy" @click="operate(() => syncJx3DailyCalendar(true))">同步剑三日历</button>
          <button type="button" :disabled="!characterList.length" @click="showImportModal = true">导入待办</button><hr /><button type="button" :disabled="operationsBusy || !characterList.length" @click="reset">重置{{ selectedScope === 'all' ? '所有角色' : '当前角色' }}日常…</button>
        </div></details>
      </div>
    </header>
    <nav class="today-scopes" aria-label="查看角色">
      <button type="button" :aria-current="selectedScope === 'all' ? 'page' : undefined" @click="selectScope('all'); pane = 'tasks'">全部角色<span>{{ homeMatrixStats.remainingDaily }}</span></button>
      <button v-for="char in characterList" :key="char.id" type="button" :title="char.name + (char.server ? ' · ' + char.server : '')" :aria-current="selectedScope === char.id ? 'page' : undefined" :aria-label="'查看角色：' + char.name" @click="selectScope(char.id); pane = 'tasks'"><GameStampIcon :game="gameMetaOf(char.game_id).stamp" class="w-3.5 h-3.5" /><span class="today-scope-name">{{ getCharBoardTitle(char) }}</span><span>{{ homeCharStats(char.id).dailyRemaining }}</span></button>
    </nav>
    <div class="today-progress-strip">
      <div class="today-progress-copy"><span>{{ focusedChar ? getCharBoardTitle(focusedChar) : '今天的进度' }}</span><small>{{ focusedChar ? getCharBoardHint(focusedChar) : `${characterList.length} 个角色 · ${daily.remaining} 项日常待办` }}</small></div>
      <div class="today-progress-meter" role="progressbar" aria-label="今日日常进度" :aria-valuemin="0" :aria-valuenow="daily.done" :aria-valuemax="daily.total"><i :style="{ width: percent + '%' }" /></div>
      <p class="today-progress-value"><b>{{ daily.done }}</b><span> / {{ daily.total }}</span><small>已完成</small></p>
      <span v-if="daily.total && !daily.remaining" class="today-progress-done"><Check />今日已清</span>
    </div>
    <div class="today-toolbar">
      <nav class="today-periods" aria-label="待办周期"><button v-for="tab in sections" :key="tab.id" type="button" :aria-pressed="period === tab.id" @click="period = tab.id; pane = 'tasks'">{{ tab.id === 'daily' ? '日常' : tab.id === 'weekly' ? '周常' : '万象' }}<span>{{ tab.remaining }}</span></button></nav>
      <label class="today-search"><Search /><input v-model="query" type="search" aria-label="搜索待办、角色或备注" placeholder="搜索待办、角色或备注" /><button v-if="query" type="button" aria-label="清空搜索" @click="query = ''"><X /></button></label>
      <nav v-if="hasBriefing" class="today-pane-switch" aria-label="今日视图"><button type="button" :aria-pressed="pane === 'tasks'" @click="pane = 'tasks'">待办</button><button type="button" :aria-pressed="pane === 'briefing'" @click="pane = 'briefing'">动态<span v-if="homeBriefingNotice" class="today-notice-dot" :title="homeBriefingNotice" /></button></nav>
    </div>
    <TodayTaskAdd :open="adding && period !== 'memo'" :scope="selectedScope" :period="period" @close="adding = false" @saved="period = $event; query = ''; pane = 'tasks'" />
    <div class="today-layout" :class="{ 'has-briefing': hasBriefing }">
      <div ref="tasks" class="today-tasks" :class="{ 'is-selected': pane === 'tasks' }" role="region" aria-label="今日待办清单" tabindex="0">
        <Onboarding />
        <details v-if="focusedChar && gameIdOf(focusedChar.id) === 'jx3'" class="today-calendar-add"><summary>添加官方日程<ChevronDown /></summary><div><button v-for="slot in JX3_CALENDAR_SLOTS" :key="slot.key" type="button" :disabled="operationsBusy || slotAttached(slot.key)" @click="attachSlot(slot.key)"><Check v-if="slotAttached(slot.key)" /><Plus v-else />{{ slot.label.replace('今日', '') }}</button></div></details>
        <TodayBoard :section="section" :query="query" :scope="selectedScope" @add="add" @clear="query = ''" />
        <TodayStats v-if="selectedScope === 'all'" />
      </div>
      <aside v-if="hasBriefing" class="today-briefing" :class="{ 'is-selected': pane === 'briefing' }" aria-label="游戏动态" tabindex="0">
        <header class="today-section-heading"><h2>旅程动态</h2><span v-if="homeBriefingNotice" class="today-notice">{{ homeBriefingNotice }}</span><ArrowUpRight v-else /></header>
        <div v-if="homeNowItems.length" class="today-live-list"><div v-for="item in homeNowItems" :key="item.id" :class="{ 'is-live': item.tone === 'alert' }"><i /><div><p>{{ item.title }}</p><strong>{{ item.hint }}</strong></div></div></div>
        <TodaySky v-if="isGameEnabled('sky')" /><TodayJx3 v-if="hasJx3Characters" /><TodayEnding v-if="isGameEnabled('sky')" />
      </aside>
    </div>
    <footer class="today-footer"><span>{{ section.remaining }} 项待办 · {{ section.done }} 项已完成</span><span>{{ resetHint }}</span></footer>
  </section>
</template>
