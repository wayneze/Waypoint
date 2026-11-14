<script setup lang="ts">
// 今日页里的剑三一栏：日历里秘境、福缘、本周团本这些以前没展示的字段。
import { computed } from 'vue';
import { jx3TodayCalendar } from '../../workspace/jx3/calendar';
import { currentTab } from '../../workspace/core';
import { ArrowUpRight } from '@lucide/vue';
import GameStampIcon from '../GameStampIcon.vue';

const rows = computed(() => {
  const c = jx3TodayCalendar.value;
  if (!c) return [];
  return [
    { label: '大战', value: c.war },
    { label: '战场', value: c.battle || '' },
    { label: '秘境日常', value: (c.card || []).join('、') },
    { label: '福缘宠物', value: (c.lucky || []).join('、') },
    { label: '外观抽取', value: (c.draw || []).join('、') },
    { label: '本周团本', value: (c.weekly_raid || []).join('、') },
    { label: '本周公共', value: (c.weekly_conn || []).join('、') },
  ].filter(r => r.value);
});
</script>

<template>
  <section class="today-game-brief" data-game="jx3">
    <div class="today-brief-heading">
      <GameStampIcon game="jx3" class="w-4 h-4" />
      <h3>剑网3</h3>
      <button type="button" class="today-icon-button ml-auto" aria-label="前往剑网3日历" @click="currentTab = 'jx3'"><ArrowUpRight /></button>
    </div>
    <p v-if="!rows.length" class="text-xs text-faint">今日江湖资料待同步，可前往游戏页刷新。</p>
    <dl v-else class="today-calendar-rows">
      <div v-for="r in rows" :key="r.label">
        <dt>{{ r.label }}</dt>
        <dd>{{ r.value }}</dd>
      </div>
    </dl>
  </section>
</template>
