<script setup lang="ts">
import {
  ensureSkyCatalogs,
  ensureSkySubTabCatalog,
  isLoadingSkyDaily,
  isLoadingSkyNews,
  isLoadingSkyEvents,
  isLoadingSkySeasons,
  refreshSkyDaily,
  refreshSkyEvents,
  refreshSkyNews,
  refreshSkySeasons,
  setSkySubTab,
  skyDailyData,
  skySubTab,
} from '../workspace/sky/tabs';
import { dailyCanEdit } from '../workspace/sky/daily';
import { skyTodayProgress, completeSkyToday } from '../workspace/sky/plan';
import { totalWings, unlockedWings } from '../workspace/sky/spirits';
import { atlasCanEdit, atlasCharacter, atlasProgress, resetAtlasLightsOnly } from '../workspace/sky/atlas';
import { handleProjectToObsidian } from '../workspace/bootstrap';
import { isProjectingObsidian } from '../workspace/core';
import { RotateCw } from '@lucide/vue';
import { computed, onMounted, ref, watch } from 'vue';
import SkyDailyPanel from '../components/sky/SkyDailyPanel.vue';
import SkyNewsPanel from '../components/sky/SkyNewsPanel.vue';
import SkyAtlasPanel from '../components/sky/SkyAtlasPanel.vue';
import SkySpiritsPanel from '../components/sky/SkySpiritsPanel.vue';
import DataSourceBar from '../components/DataSourceBar.vue';
import GameWorkspace from '../components/GameWorkspace.vue';
import SkyDailyConnections from '../components/sky/SkyDailyConnections.vue';
import SkyOutfitsPanel from '../components/sky/SkyOutfitsPanel.vue';
import { collectionImageMode, spiritCollectionSource, restoreCollectionImageMode } from '../workspace/sky/collection';
import SkyBenefitsPanel from '../components/sky/SkyBenefitsPanel.vue';

const collectionTabs = [
  { id: 'outfits', label: '外观图鉴' },
  { id: 'permanent', label: '常驻先祖' },
  { id: 'season', label: '季节先祖' },
] as const;
const lastCollection = ref<'spirits' | 'outfits'>('outfits');
const mainTab = computed(() => skySubTab.value === 'spirits' || skySubTab.value === 'outfits' ? 'collection' : skySubTab.value);
const collectionTab = computed(() => skySubTab.value === 'outfits' ? 'outfits' : spiritCollectionSource.value);
function changeCollection(tab: typeof collectionTabs[number]['id']) {
  if (tab !== 'outfits') spiritCollectionSource.value = tab;
  setSkySubTab(tab === 'outfits' ? 'outfits' : 'spirits');
}
const tabs = computed(() => [
  { id: 'daily', label: '今日', count: `${skyTodayProgress.value.done}/${skyTodayProgress.value.total}` },
  { id: 'news', label: '活动与回归' },
  { id: 'collection', label: '收藏' },
  { id: 'atlas', label: '探索', count: `${atlasProgress.value.lights}/${atlasProgress.value.lightTotal}` },
  { id: 'benefits', label: '福利' },
]);
function changeMain(tab: string) {
  if (tab === 'collection') setSkySubTab(lastCollection.value);
  else if (tab === 'daily' || tab === 'news' || tab === 'atlas' || tab === 'benefits') setSkySubTab(tab);
}
const descriptions = {
  daily: '国服待办 · 活动日常与本期目标', news: '国服档期 · 回归先祖与后续情报',
  atlas: '大世界探索 · 光翼、神殿与动作', spirits: '先祖进度 · 常驻与季节',
  outfits: '自己的收藏 · 沿途的心愿',
  benefits: '网易官方 · 签到福利与优惠充值',
};
const refreshing = computed(() => mainTab.value === 'collection' ? isLoadingSkySeasons.value || isLoadingSkyEvents.value : skySubTab.value === 'daily' ? isLoadingSkyDaily.value : isLoadingSkyNews.value);
function refreshCurrent() {
  if (mainTab.value === 'collection') return Promise.all([refreshSkySeasons(true), refreshSkyEvents(true)]);
  if (skySubTab.value === 'daily') return Promise.all([refreshSkyDaily(true), refreshSkyNews(true)]);
  const key = SKY_SOURCES[skySubTab.value]?.[0];
  if (key) return retrySky(key);
}

// 每个子页用到的数据源（fetchCache 键）
const SKY_SOURCES: Record<string, string[]> = {
  daily: ['sky.daily', 'sky.calendar'],
  news: ['sky.calendar'],
  outfits: ['sky.seasons', 'sky.events'],
  spirits: ['sky.seasons'],
};

function retrySky(key: string) {
  if (key === 'sky.daily') return refreshSkyDaily(true);
  if (key === 'sky.calendar') return refreshSkyNews(true);
  if (key === 'sky.seasons') return refreshSkySeasons(true);
  if (key === 'sky.events') return refreshSkyEvents(true);
}

onMounted(() => {
  restoreCollectionImageMode();
  ensureSkyCatalogs();
});
const benefitsVisited = ref(skySubTab.value === 'benefits');
watch(skySubTab, tab => {
  if (tab === 'benefits') benefitsVisited.value = true;
  ensureSkySubTabCatalog(tab);
  if (tab === 'spirits' || tab === 'outfits') lastCollection.value = tab;
});
</script>

<template>
  <GameWorkspace title="光遇" subtitle="旅人手记 · 把沿途的光慢慢收集" game="sky"
                 :active="mainTab" :tabs="tabs" :class="{ 'is-benefits': skySubTab === 'benefits', 'is-daily': skySubTab === 'daily', 'is-collection': mainTab === 'collection' }" @change="changeMain">
    <template #summary>
      <span class="wp-game-summary-label">常驻永久翼</span>
      <span class="wp-game-summary-value">{{ unlockedWings }}<small> / {{ totalWings }}</small></span>
    </template>
    <template #context>
      <span v-if="skySubTab === 'atlas' && atlasCharacter" class="wp-context-owner">{{ atlasCharacter.name }}</span>
      <span v-if="skySubTab === 'daily' && skyDailyData" class="wp-context-owner">{{ skyDailyData.dateStr }} · {{ skyDailyData.dayOfWeekStr }}</span>
      <template v-if="skySubTab !== 'daily'">{{ descriptions[skySubTab] }}</template>
    </template>
    <template #actions>
      <button v-if="skySubTab === 'daily' && skyTodayProgress.done < skyTodayProgress.total"
              type="button" class="wp-game-action is-primary" :disabled="!dailyCanEdit" @click="completeSkyToday" title="手动完成所选角色全部日常；代练同步按订单范围执行">全部完成</button>
      <button v-if="SKY_SOURCES[skySubTab]" type="button" class="wp-game-action" :disabled="refreshing" @click="refreshCurrent">
        <RotateCw class="w-3.5 h-3.5" :class="{ 'animate-spin': refreshing }" />{{ refreshing ? '刷新中' : '刷新' }}
      </button>
    </template>
    <template v-if="skySubTab !== 'benefits'" #more>
      <button v-if="skySubTab === 'atlas'" type="button" @click="resetAtlasLightsOnly" :disabled="!atlasCanEdit"
              :title="atlasCharacter ? `只清空「${atlasCharacter.name}」的大世界小金人，保留神殿和动作` : '请先选择光遇角色'">献祭重置光翼</button>
      <button type="button" @click="handleProjectToObsidian('sky')" :disabled="isProjectingObsidian">
        {{ isProjectingObsidian ? '投影中…' : '投影本页到 Obsidian' }}
      </button>
    </template>
    <SkyDailyConnections v-show="skySubTab !== 'benefits'" class="mb-3" />
    <div v-if="mainTab === 'collection'" class="collection-toolbar">
      <nav class="collection-nav" aria-label="收藏分类">
        <button v-for="tab in collectionTabs" :key="tab.id" type="button" :aria-pressed="collectionTab === tab.id" @click="changeCollection(tab.id)">{{ tab.label }}</button>
      </nav>
      <div v-if="skySubTab === 'outfits' || spiritCollectionSource === 'season'" class="collection-image-mode" role="group" aria-label="图片展示方式">
        <button type="button" :aria-pressed="collectionImageMode === 'icon'" @click="collectionImageMode = 'icon'">图标</button>
        <button type="button" :aria-pressed="collectionImageMode === 'wear'" @click="collectionImageMode = 'wear'">实穿</button>
      </div>
    </div>
    <DataSourceBar :sources="SKY_SOURCES[skySubTab] || []" @retry="retrySky" />
    <SkyDailyPanel v-show="skySubTab === 'daily'" />
    <SkyNewsPanel v-if="skySubTab === 'news'" />
    <SkyAtlasPanel v-show="skySubTab === 'atlas'" />
    <SkySpiritsPanel v-if="skySubTab === 'spirits'" :source="spiritCollectionSource" />
    <SkyOutfitsPanel v-if="skySubTab === 'outfits'" />
    <SkyBenefitsPanel v-if="benefitsVisited" v-show="skySubTab === 'benefits'" :active="skySubTab === 'benefits'" />
  </GameWorkspace>
</template>

<style scoped>
.collection-toolbar { display: flex; align-items: center; gap: 12px; border-bottom: 1px solid var(--color-hairline); margin-bottom: 18px; }
.collection-nav { display: flex; gap: clamp(8px, 2vw, 20px); min-width: 0; }.collection-nav button { min-height: 44px; padding: 8px 2px; white-space: nowrap; border-bottom: 2px solid transparent; color: var(--color-muted); font-size: 14px; cursor: pointer; }.collection-nav button[aria-pressed="true"] { border-color: var(--color-beacon); color: var(--color-ink); }
.collection-image-mode { margin-left: auto; display: flex; padding: 3px; border-radius: 6px; background: var(--color-raised); }.collection-image-mode button { min-height: 32px; padding: 4px 12px; font-size: 12px; color: var(--color-muted); border-radius: 4px; cursor: pointer; }.collection-image-mode button[aria-pressed="true"] { background: var(--color-paper); color: var(--color-ink); }
.collection-toolbar button:focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 2px; }
.is-benefits :deep(.wp-game-content) { display: flex; flex-direction: column; overflow: hidden; padding: 0 2px 12px; scrollbar-gutter: auto; }
.is-daily :deep(.wp-game-heading) { padding-bottom: 12px; }
.is-daily :deep(.wp-game-heading p) { display: none; }
.is-daily :deep(.wp-game-heading h1) { font-size: 26px; }
.is-collection :deep(.wp-game-heading) { padding-bottom: 12px; }
.is-collection :deep(.wp-game-heading p) { display: none; }
.is-collection :deep(.wp-game-heading h1) { font-size: 26px; }
</style>
