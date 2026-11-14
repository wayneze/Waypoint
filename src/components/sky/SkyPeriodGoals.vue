<script setup lang="ts">
import { computed } from 'vue';
import { ArrowUpRight, Check } from '@lucide/vue';
import type { SkyCalendarItem } from '../../services/skyCalendarService';
import { openExternalUrl } from '../../services/systemApi';
import { dailyCanEdit } from '../../workspace/sky/daily';
import { goalsFor, isActivityDaily, setActivityDaily, togglePeriodGoal } from '../../workspace/sky/plan';

const props = defineProps<{ item: SkyCalendarItem }>();
const goals = computed(() => goalsFor(props.item));
const doneCount = computed(() => goals.value.filter(goal => goal.done).length);
</script>

<template>
  <details class="sky-period-goals">
    <summary>目标与兑换 · {{ doneCount }} / {{ goals.length }}</summary>
    <div class="sky-goal-grid">
      <button v-for="goal in goals" :key="goal.id" type="button" class="sky-goal"
              :disabled="!dailyCanEdit" :aria-pressed="goal.done" @click="togglePeriodGoal(goal)">
        <span class="sky-mini-check" :class="{ 'is-done': goal.done }"><Check class="w-3 h-3" /></span>
        <span :class="goal.done ? 'line-through text-faint' : 'text-ink'">{{ goal.title }}</span>
      </button>
    </div>
    <p v-if="item.kind === 'reprint' && !goals.some(goal => goal.itemId)" class="text-xs text-muted py-2">兑换清单待匹配，可先查看原页；不会把预览外观当作已拥有。</p>
    <label v-if="item.kind === 'event'" class="flex items-center gap-2 text-xs text-muted py-2 cursor-pointer">
      <input type="checkbox" class="accent-beacon" :checked="isActivityDaily(item)" :disabled="!dailyCanEdit"
             @change="setActivityDaily(item, ($event.target as HTMLInputElement).checked)" />
      加入每日待办
    </label>
    <button v-if="item.url && item.kind !== 'event'" type="button" class="wp-text-action text-xs mt-1" @click="openExternalUrl(item.url)">查看原页 <ArrowUpRight class="w-3 h-3" /></button>
  </details>
</template>

<style scoped>
.sky-period-goals { margin-top: 12px; }
.sky-period-goals summary { color: var(--color-muted); font-size: 12px; cursor: pointer; padding: 8px 0; }
.sky-goal-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(200px, 100%), 1fr)); gap: 4px 18px; margin-top: 8px; }
.sky-goal { display: flex; align-items: center; gap: 8px; text-align: left; min-width: 0; padding: 8px 0; font-size: 13px; cursor: pointer; }
.sky-goal:disabled { opacity: .5; cursor: default; }
.sky-mini-check { display: grid; place-items: center; border: 1px solid var(--color-hairline); border-radius: 5px; width: 18px; height: 18px; color: transparent; flex-shrink: 0; }
.sky-mini-check.is-done { color: var(--color-done); border-color: var(--color-done); }
</style>
