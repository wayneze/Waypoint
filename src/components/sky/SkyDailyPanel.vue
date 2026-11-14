<script setup lang="ts">
import { computed } from 'vue';
import { skyTodayGroups, skyTodayProgress, skyEdenDone, toggleSkyEden, skyPlanDay } from '../../workspace/sky/plan';
import { dailyCanEdit, dailyCharacterId, skyShard, skyServerMode, setSkyServerMode } from '../../workspace/sky/daily';
import { skyCalendarFeed, skyDailyData } from '../../workspace/sky/tabs';
import SkyTodoList from './SkyTodoList.vue';
import SkyPeriodPanel from './SkyPeriodPanel.vue';
import { shardOccurrenceState } from '../../services/skyShards';
import { Check, ChevronDown, Clock3, Gem, Mountain, MapPin, ArrowUpRight, Sparkles } from '@lucide/vue';
import { hotlinkImageSrc, vWikiImg } from './wikiImage';
import { openLightbox } from '../../workspace/guides/lightbox';

const pendingGroups = computed(() => skyTodayGroups.value.filter(group => !group.done));
const completedGroups = computed(() => skyTodayGroups.value.filter(group => group.done));
const completedKey = computed(() => `${dailyCharacterId.value}/${skyPlanDay.value}/${completedGroups.value.map(group => group.id).join(',')}`);
const shardLive = computed(() => skyShard.value.status === 'active' || skyShard.value.status === 'falling');
const shardSlots = computed(() => skyShard.value.occurrences.map(occ => ({ ...occ, state: shardOccurrenceState(skyShard.value, occ.index) })));
const slotLabels = { active: '进行中', falling: '降落中', next: '下一场', upcoming: '未开始', ended: '已结束' };
const shardImage = computed(() => skyServerMode.value === 'netease' && skyDailyData.value?.dateStr === skyPlanDay.value
  ? skyDailyData.value?.overviewImages.shardImg : undefined);
const progressPercent = computed(() => skyTodayProgress.value.total ? skyTodayProgress.value.done / skyTodayProgress.value.total * 100 : 0);
</script>

<template>
  <div class="sky-today-layout">
    <div class="sky-today-main">
      <section class="sky-shard" :class="{ 'is-live': shardLive }" aria-label="落石时段">
        <div class="sky-shard-content">
        <div class="sky-shard-heading">
          <Clock3 class="w-4 h-4" /><h3>落石时段</h3>
          <span class="sky-shard-status" role="status">{{ skyShard.status === 'active' ? '正在进行' : skyShard.status === 'falling' ? '正在降落' : skyShard.status === 'upcoming' ? '等待降临' : skyShard.status === 'all_ended' ? '今日已结束' : '今日无落石' }}</span>
          <select :value="skyServerMode" @change="setSkyServerMode(($event.target as HTMLSelectElement).value as 'netease' | 'global')" aria-label="落石服务器" class="bg-transparent text-xs text-muted ml-auto">
            <option value="netease">国服落石</option><option value="global">国际服落石</option>
          </select>
        </div>
        <div class="sky-shard-summary">
          <div>
            <p class="sky-shard-location"><Gem class="w-5 h-5" :class="{ 'text-alert': skyShard.isRed }" />{{ skyShard.hasShard ? `${skyShard.isRed ? '红石' : '黑石'} · ${skyShard.realm}` : '今日无落石' }}</p>
            <p v-if="skyShard.hasShard" class="sky-shard-map"><MapPin class="w-3.5 h-3.5" />{{ skyShard.mapName }}</p>
            <p v-if="skyShard.hasShard" class="text-xs text-muted mt-1.5">{{ skyShard.isRed ? `升华蜡烛 ×${skyShard.rewardCandles} · 远古回忆` : '常规大烛火' }}</p>
          </div>
          <div class="sky-shard-countdown">
            <small>{{ skyShard.statusText }}</small>
            <strong>{{ skyShard.countdownText }}</strong>
          </div>
        </div>
        <ol v-if="shardSlots.length" class="sky-shard-slots" aria-label="今日落石时间段">
          <li v-for="occ in shardSlots" :key="occ.index" :class="`is-${occ.state}`"
              :aria-current="occ.state === 'active' || occ.state === 'falling' ? 'time' : undefined">
            <span class="sky-shard-slot-label">第 {{ occ.index }} 场<span>{{ slotLabels[occ.state] }}</span></span>
            <strong>{{ occ.timeRangeStr }}</strong>
          </li>
        </ol>
        <p v-if="(skyShard.status === 'no_shard' || skyShard.status === 'all_ended') && skyShard.nextShardForecast" class="text-xs text-muted mt-3">下次 · {{ skyShard.nextShardForecast.dayDesc }} {{ skyShard.nextShardForecast.realm }} · {{ skyShard.nextShardForecast.countdownText }}</p>
        </div>
        <button v-if="shardImage && skyShard.hasShard" type="button" class="sky-shard-map-image" aria-label="查看今日落石位置图"
                @click="openLightbox(hotlinkImageSrc(shardImage), '今日落石 · 可能的位置')">
          <img v-wiki-img="shardImage" alt="今日落石位置参考图" />
          <span><MapPin class="w-3.5 h-3.5" />看落点 <ArrowUpRight class="w-3.5 h-3.5" /></span>
        </button>
      </section>
      <section class="sky-today-section" aria-label="今天要做">
        <div class="sky-today-section-heading">
          <div><h3>{{ pendingGroups.length ? '今天要做' : '今日日常已完成' }}</h3><p>{{ pendingGroups.length ? `还有 ${pendingGroups.length} 组 · 点图片看攻略` : '今日的光，已经收好' }}</p></div>
          <div class="sky-daily-meter" role="progressbar" :aria-valuenow="skyTodayProgress.done" :aria-valuemax="skyTodayProgress.total" aria-label="今日日常完成进度">
            <span>{{ skyTodayProgress.done }}<small> / {{ skyTodayProgress.total }}</small></span>
            <div><i :style="{ width: progressPercent + '%' }" /></div>
          </div>
        </div>
        <SkyTodoList v-if="pendingGroups.length" :groups="pendingGroups" />
        <div v-else class="sky-day-finished"><Sparkles class="w-8 h-8" /><span>今天可以慢慢逛了</span><small>已完成的任务和图片保留在下方</small></div>
        <p v-if="!skyCalendarFeed" class="text-xs text-muted pt-2">活动档期待更新，当前清单可能不完整。</p>
      </section>
      <details v-if="completedGroups.length" :key="completedKey" class="sky-completed">
        <summary class="sky-completed-summary">
          <Check class="w-4 h-4 text-done" />
          <span>已完成日常 <span class="font-mono">{{ completedGroups.length }}</span></span>
          <span class="sky-completed-names">{{ completedGroups.map(group => group.title).join(' · ') }}<small>展开看图</small></span>
          <ChevronDown class="w-4 h-4 sky-completed-chevron" />
        </summary>
        <SkyTodoList :groups="completedGroups" />
      </details>
    </div>
    <aside class="sky-period-aside">
      <SkyPeriodPanel />
      <section class="sky-week-section" aria-label="本周">
        <h3>每周旅程</h3>
        <button type="button" class="sky-eden-task" :disabled="!dailyCanEdit"
                :aria-pressed="skyEdenDone" @click="toggleSkyEden">
          <span class="sky-eden-icon"><Mountain class="w-6 h-6" /></span>
          <span class="flex-1"><strong>伊甸献祭</strong><small>{{ skyEdenDone ? '本周已完成' : '按需前往 · 周日重置' }}</small></span>
          <span class="sky-check" :class="{ 'is-done': skyEdenDone }"><Check class="w-4 h-4" /></span>
        </button>
      </section>
    </aside>
  </div>
</template>

<style scoped>
.sky-today-layout { display: grid; grid-template-columns: minmax(0, 1fr) 280px; gap: 28px; padding: 4px 2px; align-items: start; container-type: inline-size; }
.sky-today-main { min-width: 0; display: grid; gap: 26px; }
.sky-period-aside { min-width: 0; display: grid; gap: 26px; }
.sky-today-section-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 16px; }
.sky-today-section-heading h3 { font-size: 20px; font-weight: 600; color: var(--color-ink); }
.sky-today-section-heading p { font-size: 12px; color: var(--color-muted); margin-top: 4px; }
.sky-daily-meter { width: 96px; flex-shrink: 0; text-align: right; }
.sky-daily-meter > span { font: 24px var(--font-mono); color: var(--game-color); }
.sky-daily-meter small { font-size: 12px; color: var(--color-muted); }
.sky-daily-meter > div { margin-top: 6px; height: 4px; background: var(--color-hairline); border-radius: 4px; overflow: hidden; }
.sky-daily-meter i { display: block; height: 100%; background: var(--color-done); }
.sky-day-finished { display: grid; justify-items: center; align-content: center; gap: 12px; min-height: 180px; border: 1px solid var(--color-hairline); border-radius: 14px; color: var(--color-done); background: color-mix(in srgb, var(--color-done) 4%, transparent); }
.sky-day-finished span { font-size: 18px; color: var(--color-ink); }
.sky-day-finished small { font-size: 12px; color: var(--color-muted); }
.sky-shard { display: flex; border: 1px solid color-mix(in srgb, var(--game-color) 25%, var(--color-hairline)); border-radius: 14px; overflow: hidden; background: color-mix(in srgb, var(--game-color) 5%, var(--color-canvas)); }
.sky-shard-content { flex: 1; min-width: 0; padding: 16px 18px; }
.sky-shard.is-live { border-color: var(--color-alert); background: color-mix(in srgb, var(--color-alert) 9%, transparent); }
.sky-shard-heading { display: flex; align-items: center; gap: 8px; color: var(--game-color); }
.sky-shard-heading h3 { font-size: 13px; font-weight: 600; color: var(--color-ink); }
.sky-shard-status { font-size: 11px; padding: 3px 7px; border: 1px solid currentColor; border-radius: 4px; }
.sky-shard.is-live .sky-shard-heading, .sky-shard.is-live .sky-shard-countdown strong { color: var(--color-alert); }
.sky-shard-summary { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-top: 14px; }
.sky-shard-location { display: flex; align-items: center; gap: 7px; font-size: 20px; font-weight: 600; color: var(--color-ink); }
.sky-shard-map { display: flex; align-items: center; gap: 5px; font-size: 12px; color: var(--color-muted); margin-top: 6px; }
.sky-shard-countdown { display: grid; gap: 4px; }
.sky-shard-countdown small { font-size: 11px; color: var(--color-muted); }
.sky-shard-countdown strong { font-size: 20px; font-weight: 600; color: var(--game-color); font-variant-numeric: tabular-nums; }
.sky-shard-map-image { position: relative; flex: 0 0 132px; overflow: hidden; cursor: zoom-in; border-left: 1px solid var(--color-hairline); background: var(--color-surface); }
.sky-shard-map-image img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; object-position: top; }
.sky-shard-map-image span { position: absolute; bottom: 0; left: 0; right: 0; display: flex; align-items: center; justify-content: center; gap: 6px; background: linear-gradient(transparent, #101917f2); padding: 24px 6px 12px; color: white; font-size: 12px; }
.sky-shard-slots { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 8px; margin-top: 16px; }
.sky-shard-slots li { display: grid; gap: 7px; padding: 9px 10px; border: 1px solid var(--color-hairline); border-radius: 4px; color: var(--color-muted); }
.sky-shard-slot-label { display: flex; justify-content: space-between; gap: 4px; font-size: 11px; }
.sky-shard-slots strong { font-size: 12px; font-family: var(--font-mono); font-weight: 500; white-space: nowrap; }
.sky-shard-slots .is-active, .sky-shard-slots .is-falling { color: var(--color-alert); border-color: var(--color-alert); background: color-mix(in srgb, var(--color-alert) 13%, transparent); }
.sky-shard-slots .is-next { color: var(--game-color); border-color: color-mix(in srgb, var(--game-color) 45%, transparent); }
.sky-shard-slots .is-ended { color: var(--color-faint); }
.sky-completed { border: 1px solid var(--color-hairline); border-radius: 10px; padding: 0 14px; }
.sky-completed-summary { display: flex; align-items: center; gap: 10px; padding: 14px 0; cursor: pointer; list-style: none; color: var(--color-muted); font-size: 13px; }
.sky-completed-summary::-webkit-details-marker { display: none; }
.sky-completed-names { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 11px; color: var(--color-faint); text-align: right; }
.sky-completed-names small { margin-left: 12px; color: var(--game-color); }
.sky-completed[open] { padding-bottom: 16px; }
.sky-completed-chevron { flex-shrink: 0; transition: transform .15s; }
.sky-completed[open] .sky-completed-chevron { transform: rotate(180deg); }
.sky-week-section h3 { font-size: 14px; font-weight: 600; color: var(--color-muted); margin-bottom: 12px; }
.sky-eden-task { display: flex; align-items: center; gap: 12px; width: 100%; text-align: left; padding: 14px 12px; border: 1px solid var(--color-hairline); border-radius: 10px; cursor: pointer; }
.sky-eden-icon { color: var(--game-color); }
.sky-eden-task strong { display: block; font-size: 14px; font-weight: 500; color: var(--color-ink); }
.sky-eden-task small { display: block; font-size: 11px; color: var(--color-muted); margin-top: 4px; }
.sky-eden-task:disabled { opacity: .5; cursor: default; }
@media (max-width: 1200px) { .sky-today-layout { grid-template-columns: minmax(0, 1fr) 240px; gap: 20px; } .sky-shard-map-image { display: none; } }
@media (max-width: 1000px) { .sky-today-layout { grid-template-columns: minmax(0, 1fr); } .sky-period-aside { grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: start; } }
@container (max-width: 540px) { .sky-shard-content { padding: 12px; } .sky-shard-slots { gap: 5px; } .sky-shard-slot-label { flex-wrap: wrap; } .sky-shard-slots strong { font-size: 10px; } .sky-shard-slots li { padding: 7px; } .sky-shard-heading { gap: 5px; } .sky-period-aside { grid-template-columns: 1fr; } .sky-completed-names { display: none; } .sky-completed-chevron { margin-left: auto; } }
</style>
