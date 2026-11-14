<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { Bell, ChevronRight } from '@lucide/vue';
import type { SkyCalendarItem } from '../../services/skyApi';
import { calendarKindLabel, calendarLookSummary, calendarRange, calendarRemain, calendarStatusLabel } from '../../services/skyCalendarService';
import { getForecastContent, type SkyEventForecast } from '../../services/skyEventForecast';
import { openExternalUrl } from '../../services/systemApi';
import { hotlinkImageSrc } from './wikiImage';
import CalendarPreviews from './CalendarPreviews.vue';
import SkyEventGuide from './SkyEventGuide.vue';
import SkyPeriodGoals from './SkyPeriodGoals.vue';

const props = defineProps<{ item: SkyCalendarItem; forecast?: SkyEventForecast; active?: boolean; watched?: boolean; timing?: string }>();
const expanded = ref(false);
const content = ref<SkyCalendarItem | null>(null);
const loading = ref(false);
const failed = ref(false);
const displayItem = computed(() => content.value || props.item);
const referenceYear = computed(() => props.forecast?.history[0].start.slice(0, 4));
const statusLabel = computed(() => props.forecast ? '预计回归' : calendarStatusLabel(props.item));
let request = 0;
async function load(force = false) {
  const version = ++request;
  content.value = null;
  if (!props.forecast) return;
  loading.value = true;
  failed.value = false;
  const accept = (item: SkyCalendarItem) => { if (version === request) content.value = item; };
  const result = await getForecastContent(props.forecast, force, accept);
  if (version !== request) return;
  if (result) accept(result);
  failed.value = !result;
  loading.value = false;
}
watch(() => props.forecast?.history[0], () => { void load(); }, { immediate: true });
onBeforeUnmount(() => { request++; });
</script>

<template>
  <article class="wp-calendar-entry">
    <button type="button" class="wp-calendar-heading" :aria-expanded="expanded" @click="expanded = !expanded">
      <span class="wp-calendar-date">
        <template v-if="forecast"><span>{{ forecast.year }}</span><small>预计 · 未官宣</small></template>
        <template v-else><time :datetime="item.start">{{ item.start || '日期待定' }}</time><small>{{ timing || calendarRemain(item) || calendarStatusLabel(item) }}</small></template>
      </span>
      <img v-if="displayItem.icon" :src="hotlinkImageSrc(displayItem.icon)" alt="" class="w-16 h-16 shrink-0" referrerpolicy="no-referrer" @error="($event.target as HTMLImageElement).style.display='none'" />
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2 flex-wrap">
          <span class="wp-calendar-kind">{{ calendarKindLabel(item) }}{{ statusLabel ? ' · ' + statusLabel : '' }}</span>
          <span v-if="watched" class="text-[12px] text-done inline-flex items-center gap-0.5"><Bell class="w-3 h-3" />关注的先祖</span>
        </div>
        <h3 class="text-[16px] font-semibold text-ink leading-snug mt-1">{{ item.title }}</h3>
        <p class="text-[13px] text-faint font-mono tabular-nums">{{ forecast ? `预计 ${forecast.window}` : calendarRange(item) }}</p>
        <p v-if="forecast" class="text-[12px] text-muted mt-1">{{ referenceYear }} 届内容参考 · 本届外观、兑换与玩法待公布</p>
        <template v-else>
          <p v-if="item.season && item.kind !== 'season'" class="text-[13px] text-muted">{{ item.season }}</p>
          <p v-if="item.note" class="text-[13px] text-muted mt-0.5 line-clamp-3">{{ item.note }}</p>
        </template>
        <p v-if="calendarLookSummary(displayItem)" class="text-[12px] text-muted mt-0.5">{{ calendarLookSummary(displayItem) }}</p>
        <p v-else-if="loading" class="text-[12px] text-muted mt-0.5" role="status">正在载入往届外观与兑换…</p>
      </div>
      <ChevronRight class="w-4 h-4 text-muted shrink-0 mt-1 transition-transform" :class="{ 'rotate-90': expanded }" />
    </button>
    <slot name="actions" />
    <div v-if="expanded" class="px-3.5 pb-3.5 border-t border-hairline pt-3">
      <p v-if="forecast" class="text-[13px] text-muted">{{ referenceYear }} 届 · {{ item.region === 'intl' ? '国际服' : '国服' }} · {{ calendarRange(item) }}</p>
      <p v-if="loading && !content" role="status" class="text-[13px] text-muted py-3">正在载入往届内容…</p>
      <p v-if="failed" role="status" class="text-[13px] text-muted py-3">往届内容暂时获取失败。<button type="button" class="wp-text-action" @click="load(true)">重试</button></p>
      <SkyEventGuide v-if="item.kind === 'event'" :item="displayItem" :reference="!!forecast" :description="forecast ? content?.note : undefined" />
      <SkyPeriodGoals v-if="active && !forecast" :item="item" />
      <CalendarPreviews :item="displayItem" class="mt-4" />
      <details v-if="forecast" class="mt-4 text-[12px] text-muted">
        <summary class="cursor-pointer">时间预测依据 · 近 {{ forecast.history.length }} 届</summary>
        <div class="flex flex-wrap gap-x-6 gap-y-2 mt-2">
          <a v-for="edition in forecast.history" :key="edition.start" :href="edition.url" class="hover:text-beacon"
             @click.prevent="openExternalUrl(edition.url)">{{ edition.start }} — {{ edition.end }} ↗</a>
        </div>
      </details>
    </div>
  </article>
</template>
