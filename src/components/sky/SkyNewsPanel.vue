<script setup lang="ts">
import { computed, ref } from 'vue';
import { isLoadingSkyNews, skyCalendarFeed } from '../../workspace/sky/tabs';
import { calendarRange } from '../../services/skyCalendarService';
import type { SkyCalendarItem } from '../../services/skyCalendarService';
import { kvGetJson, kvSet } from '../../services/kv';
import { Bell, BellOff, X } from '@lucide/vue';
import { skyPeriods, skyPlanDay } from '../../workspace/sky/plan';
import { forecastSkyEvents, forecastReferenceItem } from '../../services/skyEventForecast';
import SkyCalendarCard from './SkyCalendarCard.vue';

const forecasts = computed(() => forecastSkyEvents(skyCalendarFeed.value, skyPlanDay.value));
const cnForecasts = computed(() => forecasts.value.filter(row => row.region === 'cn'));
const intlForecasts = computed(() => forecasts.value.filter(row => row.region === 'intl'));

function isNamedReprint(item: SkyCalendarItem) {
  return item.status !== 'inferred' && item.names.length > 0;
}

function isCnCalendarItem(item: SkyCalendarItem) {
  return item.region !== 'intl';
}

const newsNowItems = computed(() => {
  const feed = skyCalendarFeed.value;
  if (!feed) return [] as SkyCalendarItem[];
  const rows: SkyCalendarItem[] = [];
  if (feed.season && isCnCalendarItem(feed.season)) rows.push(feed.season);
  rows.push(...feed.events.filter(item => isCnCalendarItem(item) && (item.status === 'live' || item.status === 'upcoming')));
  rows.push(...feed.cn_now);
  const namedNext = feed.cn_next.filter(isNamedReprint);
  if (namedNext.length) {
    const firstStart = namedNext[0].start;
    rows.push(...namedNext.filter(item => item.start === firstStart));
  }
  return rows;
});

const activePeriodIds = computed(() => new Set(skyPeriods.value.map(row => row.id)));

const newsAheadItems = computed(() => {
  const feed = skyCalendarFeed.value;
  if (!feed) return [] as SkyCalendarItem[];
  const intlEvents = feed.events.filter(item => item.region === 'intl');
  return [...intlEvents, ...feed.intl_now, ...feed.intl_next];
});

const newsReprintGap = computed(() => {
  const feed = skyCalendarFeed.value;
  if (!feed || feed.cn_now.length || feed.cn_next.some(isNamedReprint)) return null;
  return feed.cn_next.find(item => !isNamedReprint(item)) || null;
});

/** 国际服开始至今多少天，不编国服日期。 */
function intlLag(item: SkyCalendarItem): string {
  const m = (item.start || '').match(/^(\d{4})-(\d{1,2})-(\d{1,2})/);
  if (!m) return '';
  const days = Math.floor((Date.now() - new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).getTime()) / 86400000);
  if (days < 0) return `国际服 ${-days} 天后开始`;
  if (days < 30) return `国际服 ${days} 天前开始`;
  return `国际服约 ${Math.round(days / 30)} 个月前`;
}

/** 关注的先祖名。后台 cycle.rs 读同一个键，国服点名和开始时各发一次通知。 */
const WATCH_KEY = 'waypoint_sky_watch_spirits';
const watched = ref<string[]>(kvGetJson<string[]>(WATCH_KEY, []));

function isWatched(name: string) {
  return watched.value.includes(name);
}

function toggleWatch(name: string) {
  watched.value = isWatched(name) ? watched.value.filter(n => n !== name) : [...watched.value, name];
  kvSet(WATCH_KEY, JSON.stringify(watched.value));
}

</script>

<template>
  <div class="space-y-5">
    <p v-if="!skyCalendarFeed && !isLoadingSkyNews" class="text-[13px] text-muted px-0.5">未能拉到季节或回归先祖档期。</p>
    <p v-else-if="isLoadingSkyNews && !skyCalendarFeed" class="text-[13px] text-muted px-0.5">正在同步档期…</p>

    <template v-if="skyCalendarFeed">
      <div v-if="newsNowItems.length || newsReprintGap || cnForecasts.length" class="space-y-2.5">
        <h4 class="text-[16px] font-semibold text-ink px-0.5">国服正在 / 即将</h4>
        <SkyCalendarCard v-for="item in newsNowItems" :key="item.id" :item="item"
                         :active="activePeriodIds.has(item.id)" :watched="item.names.some(isWatched)" />
        <div
          v-if="newsReprintGap"
          class="wp-record"
        >
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-[12px] font-semibold px-2 py-0.5 rounded-full border border-hairline text-muted">
              回归先祖 · 空档
            </span>
          </div>
          <h3 class="text-[16px] font-semibold text-ink leading-snug mt-1">国服当前没有回归先祖</h3>
          <p class="text-[13px] text-faint font-mono tabular-nums">{{ calendarRange(newsReprintGap) }}</p>
          <p class="text-[12px] text-muted mt-0.5">下一档尚未点名</p>
        </div>
        <SkyCalendarCard v-for="forecast in cnForecasts" :key="forecast.id"
                         :item="forecastReferenceItem(forecast)" :forecast="forecast" />
      </div>

      <div class="space-y-2.5">
        <h4 class="text-[16px] font-semibold text-ink px-0.5">国际服动态</h4>
        <p class="text-[12px] text-faint px-0.5 -mt-1.5">两服回归独立排期，仅展示国际服正在进行和已官宣即将开始的先祖；“预计”节日展示往届内容供参考。</p>
        <div v-if="watched.length" class="flex items-center gap-1.5 flex-wrap px-0.5">
          <span class="text-[12px] text-muted">关注中</span>
          <button
            v-for="name in watched"
            :key="'w-'+name"
            type="button"
            class="text-[12px] px-2 py-0.5 rounded-full border border-hairline text-ink inline-flex items-center gap-1 cursor-pointer hover:bg-raised/40 transition"
            :title="'取消关注' + name"
            @click="toggleWatch(name)"
          >{{ name }}<X class="w-3 h-3 text-muted" /></button>
        </div>
        <p v-if="!newsAheadItems.length && !intlForecasts.length" class="text-[13px] text-muted px-0.5">国际服暂无活动与回归情报。</p>
        <SkyCalendarCard v-for="item in newsAheadItems.filter(row => row.kind === 'event')" :key="item.id"
                         :item="item" :timing="intlLag(item)" />
        <SkyCalendarCard v-for="forecast in intlForecasts" :key="forecast.id"
                         :item="forecastReferenceItem(forecast)" :forecast="forecast" />
        <SkyCalendarCard v-for="item in newsAheadItems.filter(row => row.kind !== 'event')" :key="item.id"
                         :item="item">
          <template #actions>
          <div v-if="item.kind !== 'event' && item.names.length" class="px-3.5 pb-3 -mt-1 flex items-center gap-1.5 flex-wrap">
            <button
              v-for="name in item.names"
              :key="'wt-'+name"
              type="button"
              class="text-[12px] px-2 py-0.5 rounded-full border inline-flex items-center gap-1 cursor-pointer transition"
              :class="isWatched(name) ? 'border-done text-done' : 'border-hairline text-muted hover:bg-raised/40'"
              @click="toggleWatch(name)"
            >
              <component :is="isWatched(name) ? Bell : BellOff" class="w-3 h-3" />
              {{ isWatched(name) ? `已关注${name}，到国服时提醒` : `关注${name}` }}
            </button>
          </div>
          </template>
        </SkyCalendarCard>
      </div>
    </template>
  </div>
</template>
