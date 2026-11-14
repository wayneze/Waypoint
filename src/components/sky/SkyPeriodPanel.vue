<script setup lang="ts">
import { goalsFor, skyPeriods, isActivityDaily } from '../../workspace/sky/plan';
import { isLoadingSkyNews, skyCalendarFeed, setSkySubTab } from '../../workspace/sky/tabs';
import { calendarRemain } from '../../services/skyCalendarService';
import { ArrowUpRight } from '@lucide/vue';
import { vWikiImg } from './wikiImage';
import SkyPeriodGoals from './SkyPeriodGoals.vue';
import SkyEventGuide from './SkyEventGuide.vue';
import SkyGuideImage from './SkyGuideImage.vue';
</script>

<template>
  <section aria-label="本期别错过" class="sky-period-panel">
    <div class="flex items-baseline justify-between gap-2 mb-2">
      <h3 class="text-[17px] font-semibold text-ink">本期别错过</h3>
      <span class="text-xs text-faint">{{ skyPeriods.length }} 个档期</span>
    </div>
    <p v-if="!skyCalendarFeed" class="text-sm text-muted py-5">{{ isLoadingSkyNews ? '正在读取活动与回归档期…' : '档期暂未获取，请刷新；当前活动状态未知。' }}</p>
    <p v-else-if="!skyPeriods.length" class="text-sm text-muted py-5">暂无已确认的国服进行中档期。</p>
    <article v-for="row in skyPeriods" :key="row.id" class="sky-period-row">
      <SkyGuideImage class="sky-period-cover" :src="row.icon" :alt="row.title">
        <span>{{ calendarRemain(row) }}</span>
      </SkyGuideImage>
      <div class="sky-period-body">
      <h4 class="text-[15px] font-semibold text-ink leading-relaxed">{{ row.title }}</h4>
      <div v-if="row.previews?.some(p => p.icon && p.slot !== '资讯')" class="sky-period-rewards" aria-label="本期兑换预览">
        <img v-for="preview in row.previews.filter(p => p.icon && p.slot !== '资讯').slice(0, 4)" :key="preview.name" v-wiki-img="preview.icon" :alt="preview.name" :title="preview.name" />
        <span>{{ goalsFor(row).filter(g => g.done).length }} / {{ goalsFor(row).length }}</span>
      </div>
      <SkyEventGuide v-if="row.kind === 'event' && !isActivityDaily(row)" :item="row" compact />
      <SkyPeriodGoals :item="row" />
      </div>
    </article>
    <button type="button" class="wp-text-action text-sm mt-3" @click="setSkySubTab('news')">全部活动与回归 <ArrowUpRight class="w-3.5 h-3.5" /></button>
  </section>
</template>

<style scoped>
.sky-period-row { border: 1px solid var(--color-hairline); border-radius: 12px; overflow: hidden; margin-top: 14px; background: var(--color-surface); }
.sky-period-cover { position: relative; height: 110px; overflow: hidden; display: grid; place-items: center; background: color-mix(in srgb, var(--game-color) 7%, var(--color-canvas)); color: var(--game-color); }
.sky-period-cover > span { position: absolute; top: 9px; right: 9px; padding: 3px 7px; border-radius: 4px; color: #fff; background: #101917d9; font-size: 11px; }
.sky-period-body { padding: 14px; }
.sky-period-rewards { display: flex; align-items: center; gap: 7px; margin-top: 12px; }
.sky-period-rewards img { width: 34px; height: 34px; border-radius: 5px; object-fit: contain; background: var(--color-canvas); }
.sky-period-rewards span { margin-left: auto; color: var(--color-muted); font: 11px var(--font-mono); }
</style>
