<script setup lang="ts">
import { computed } from 'vue';
import { CalendarDays, ChevronDown } from '@lucide/vue';
import {
  chutianFeed,
  chutianZhueClock,
  jx3WorldBoss,
  pifengFeed,
  pifengZhueClock,
  yuncangFeed,
  yuncangZhueClock
} from '../../workspace/jx3/calendar';
import { jx3ZhueTimesText } from '../../services/jx3Schedule';
const entries = computed(
  () =>
    [
      {
        name: '银霜口',
        faction: '云从社',
        clock: yuncangZhueClock.value,
        feed: yuncangFeed.value
      },
      {
        name: '河西瀚漠',
        faction: '披风会',
        clock: pifengZhueClock.value,
        feed: pifengFeed.value
      },
      {
        name: '楚天社',
        faction: '楚天社',
        clock: chutianZhueClock.value,
        feed: chutianFeed.value
      }
    ] as const
);
</script>
<template>
  <aside class="jx3-agenda" aria-label="江湖活动时间">
    <header class="jx3-section-heading">
      <h3><CalendarDays :size="16" />江湖时刻</h3>
      <span>今日</span>
    </header>
    <div class="jx3-agenda-line" :class="{ 'is-live': jx3WorldBoss.inWindow }">
      <span>世界首领</span><strong>{{ jx3WorldBoss.headerHint }}</strong
      ><small>{{ jx3WorldBoss.cadence }}</small>
    </div>
    <div
      v-for="entry in entries"
      :key="entry.name"
      class="jx3-agenda-line"
      :class="{ 'is-live': entry.clock?.inWindow }"
    >
      <span>{{ entry.name }}</span
      ><strong>{{ entry.clock?.nextText || '时段待同步' }}</strong>
    </div>
    <details class="jx3-schedule-details">
      <summary>完整活动排期<ChevronDown :size="14" /></summary>
      <div class="jx3-schedule-body">
        <section>
          <header class="jx3-section-heading"><h3>世界首领</h3></header>
          <div
            v-for="day in jx3WorldBoss.days"
            :key="day.weekday"
            class="jx3-agenda-line"
            :class="{ 'is-live': day.inWindow }"
          >
            <span>{{ day.weekdayLabel }} · {{ day.timeDesc }}</span
            ><strong>{{ day.namesLine }}</strong>
          </div>
        </section>
        <section v-for="entry in entries" :key="entry.name">
          <header class="jx3-section-heading">
            <h3>{{ entry.name }}</h3>
          </header>
          <p class="text-faint text-[12px]">
            {{ entry.faction }} · 诛恶 {{ jx3ZhueTimesText(entry.faction) }}
          </p>
          <p
            v-for="(event, index) in entry.feed.events"
            :key="index"
            class="py-1"
          >
            <small>{{ event.time }}</small> {{ event.stage
            }}<small v-if="event.map"> · {{ event.map }}</small
            ><small v-if="event.site"> · {{ event.site }}</small>
          </p>
        </section>
      </div>
    </details>
  </aside>
</template>
