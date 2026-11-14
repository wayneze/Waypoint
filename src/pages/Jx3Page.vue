<script setup lang="ts">
import { computed, onMounted } from 'vue';
import {
  farmRecommendedReps,
  jx3AdventureGapCount,
  jx3CompletedAchieves,
  jx3PlaceGapCount,
  loadJx3AdventureCatalog,
  reconcileJx3Snapshot,
  openAchievementImportModal,
} from '../workspace/jx3/progress';
import { handleProjectToObsidian } from '../workspace/bootstrap';
import { activeCharObj, isProjectingObsidian } from '../workspace/core';
import { jx3CollectProgress, loadJx3BoxDungeonTree } from '../workspace/jx3/collect';
import {
  jx3SubTab,
  jx3WorldBoss,
  jx3ZhueAlert,
  jx3ZhueApproaching,
  setJx3SubTab,
  syncJx3DailyCalendar,
} from '../workspace/jx3/calendar';
import { jx3ExteriorTotal, jx3ExteriorTotalLabel } from '../workspace/jx3/exterior';
import { ArrowDownToLine } from '@lucide/vue';
import Jx3DailyPanel from '../components/jx3/Jx3DailyPanel.vue';
import Jx3CollectPanel from '../components/jx3/Jx3CollectPanel.vue';
import Jx3ReputationPanel from '../components/jx3/Jx3ReputationPanel.vue';
import Jx3AdventuresPanel from '../components/jx3/Jx3AdventuresPanel.vue';
import Jx3ExteriorPanel from '../components/jx3/Jx3ExteriorPanel.vue';
import Jx3AchievementsPanel from '../components/jx3/Jx3AchievementsPanel.vue';
import DataSourceBar from '../components/DataSourceBar.vue';
import GameWorkspace from '../components/GameWorkspace.vue';
import { jx3CategoryImage } from '../services/jx3Artwork';
import Jx3BoxIcon from '../components/Jx3BoxIcon.vue';

const tabArt = { daily: '任务', collect: '秘境', reputation: '声望', adventures: '杂闻', exterior: '装备', achievements: '剑侠录' };

const tabs = computed(() => [
  { id: 'daily' as const, label: '日常', notice: jx3WorldBoss.value.inWindow || jx3ZhueAlert.value ? '活动进行中' : jx3WorldBoss.value.isBossDay || jx3ZhueApproaching.value ? '活动即将开始' : '' },
  { id: 'collect' as const, label: '特殊收集', count: `${jx3CollectProgress.value.obtained}/${jx3CollectProgress.value.total}` },
  { id: 'reputation' as const, label: '声望', count: farmRecommendedReps.value.length || undefined },
  { id: 'adventures' as const, label: '奇遇', count: jx3AdventureGapCount.value },
  { id: 'exterior' as const, label: '外观', count: jx3ExteriorTotal.value ? jx3ExteriorTotalLabel.value : undefined },
  { id: 'achievements' as const, label: '成就', count: jx3PlaceGapCount.value },
]);
const descriptions = {
  daily: '日常与周常 · 江湖活动时间', collect: '副本掉落 · 校服与特殊物品',
  reputation: '声望进度 · 值得补齐的奖励', adventures: '奇遇目录 · 前置与完成记录',
  exterior: '外观图鉴 · 持有与当前价格', achievements: '成就缺口 · 茗伊快照对账',
};

function retryJx3(key: string) {
  if (key === 'jx3.calendar') return syncJx3DailyCalendar(true);
  if (key === 'jx3.box_dungeons') return loadJx3BoxDungeonTree(true);
}

onMounted(() => {
  void loadJx3BoxDungeonTree();
  void loadJx3AdventureCatalog();
  void reconcileJx3Snapshot();
});
</script>

<template>
  <GameWorkspace class="jx3-workspace" title="剑网3" subtitle="江湖手札 · 每一份收获，都有来处" game="jx3"
                 :active="jx3SubTab" :tabs="tabs" @change="setJx3SubTab">
    <template #tab-icon="{ tab }">
      <Jx3BoxIcon :src="jx3CategoryImage(tabArt[tab.id])" size="jx3-tab-art" />
    </template>
    <template #summary>
      <span class="wp-game-summary-label">茗伊成就</span>
      <span class="wp-game-summary-value">{{ jx3CompletedAchieves.size }}<small> 已记录</small></span>
    </template>
    <template #context>
      <span v-if="activeCharObj?.game_id === 'jx3'" class="wp-context-owner">{{ activeCharObj.name }}</span>
      {{ descriptions[jx3SubTab] }}
    </template>
    <template #actions>
      <button type="button" @click="openAchievementImportModal" class="wp-game-action">
        <ArrowDownToLine class="w-3.5 h-3.5" />导入茗伊
      </button>
    </template>
    <template #more>
      <button type="button" @click="handleProjectToObsidian('jx3')" :disabled="isProjectingObsidian">
        {{ isProjectingObsidian ? '投影中…' : '投影本页到 Obsidian' }}
      </button>
    </template>
    <DataSourceBar :sources="jx3SubTab === 'daily' ? ['jx3.calendar'] : jx3SubTab === 'collect' ? ['jx3.box_dungeons'] : []" @retry="retryJx3" />
    <Jx3DailyPanel v-show="jx3SubTab === 'daily'" />
    <Jx3CollectPanel v-show="jx3SubTab === 'collect'" />
    <Jx3ReputationPanel v-show="jx3SubTab === 'reputation'" />
    <Jx3AdventuresPanel v-show="jx3SubTab === 'adventures'" />
    <Jx3ExteriorPanel v-show="jx3SubTab === 'exterior'" />
    <Jx3AchievementsPanel v-show="jx3SubTab === 'achievements'" />
  </GameWorkspace>
</template>

<style src="../styles/jx3.css"></style>
