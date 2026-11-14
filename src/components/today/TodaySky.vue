<script setup lang="ts">
import { computed } from 'vue';
import { setSkySubTab, skyCalendarFeed } from '../../workspace/sky/tabs';
import { skyTodayGroups, skyTodayProgress, skyPeriods, goalsFor } from '../../workspace/sky/plan';
import { skyShard } from '../../workspace/sky/daily';
import { activeSkyCharacter } from '../../workspace/vault';
import { currentTab } from '../../workspace/core';
import { ArrowUpRight, Clock3 } from '@lucide/vue';
import GameStampIcon from '../GameStampIcon.vue';
const pendingGoals = computed(() => skyPeriods.value.reduce((n, row) => n + goalsFor(row).filter(g => !g.done).length, 0));
function open(tab: 'daily' | 'news') { currentTab.value = 'sky'; setSkySubTab(tab); }
</script>

<template>
  <section class="today-game-brief" data-game="sky">
    <div class="today-brief-heading">
      <GameStampIcon game="sky" class="w-4 h-4" /><h3>光遇</h3>
      <button type="button" class="today-icon-button ml-auto" @click="open('daily')" aria-label="前往光遇日常"><ArrowUpRight /></button>
    </div>
    <p class="text-xs text-muted">{{ activeSkyCharacter?.name || '未选择角色' }} · 国服</p>
    <button type="button" class="today-sky-summary" @click="open('daily')">
      <span class="text-xs text-faint">今天要做</span>
      <span class="today-sky-count">{{ skyTodayProgress.done }}<small> / {{ skyTodayProgress.total }}</small></span>
      <span v-for="group in skyTodayGroups" :key="group.id" class="flex justify-between gap-3 text-xs py-1" :class="group.done ? 'text-done' : 'text-muted'"><span>{{ group.title }}</span><span>{{ group.done ? '已完成' : group.hint }}</span></span>
    </button>
    <button type="button" class="today-brief-detail" @click="open('news')"><span>本期别错过</span><span>{{ skyCalendarFeed ? `${skyPeriods.length} 个档期 · ${pendingGoals} 项未完成` : '档期待更新' }}</span></button>
    <button type="button" class="today-shard-detail" :class="{ 'is-live': skyShard.status === 'active' || skyShard.status === 'falling' }" @click="open('daily')">
      <Clock3 class="w-4 h-4 shrink-0" />
      <div class="min-w-0">
        <span class="wp-timer-label">{{ skyShard.hasShard ? (skyShard.isRed ? '红石' : '黑石') + ' · ' + skyShard.realm : '落石' }} · {{ skyShard.statusText }}</span>
        <span class="wp-timer-value text-[16px]!">{{ skyShard.countdownText }}</span>
      </div>
    </button>
  </section>
</template>
