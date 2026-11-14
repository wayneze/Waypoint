<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { activeCharacter } from '../../workspace/core';
import {
  doneReps,
  evaluatedReputations,
  bothRecommendedReps,
  farmRecommendedReps,
  mapRecommendedReps,
  setReputationLane,
  setReputationGoal,
  setReputationMethod,
  itemRecommendedReps,
  unsetRecommendedReps,
  filteredEvaluatedReputations,
  queriedReputations,
  jx3CompletedAchieves,
  jx3ReputationFilter,
  jx3ReputationQuery
} from '../../workspace/jx3/progress';
import { Check, ChevronDown, Search, X } from '@lucide/vue';
import Jx3DungeonImage from './Jx3DungeonImage.vue';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
import { jx3ReputationIconUrl } from '../../services/jx3Artwork';

const REPUTATION_FILTERS = [
  { id: 'all' as const, label: '全部' },
  { id: 'farm' as const, label: '顺路打本' },
  { id: 'map' as const, label: '地图内容' },
  { id: 'both' as const, label: '多种内容' },
  { id: 'item' as const, label: '改用道具' },
  { id: 'unset' as const, label: '待核对' },
  { id: 'done' as const, label: '毕业' }
];

const REPUTATION_LANES = [
  { id: 'dungeon' as const, label: '打本' },
  { id: 'map' as const, label: '地图' },
  { id: 'both' as const, label: '都能' },
  { id: 'item' as const, label: '道具' }
];

const REPUTATION_GOALS = ['友好', '亲密', '敬重', '尊敬', '钦佩'] as const;

function reputationLaneLabel(action?: string) {
  if (action === 'farm') return '打本';
  if (action === 'map') return '地图';
  if (action === 'both') return '副本和地图';
  if (action === 'item') return '道具';
  if (action === 'done') return '已毕业';
  return '待分';
}

function reputationFilterCount(id: (typeof REPUTATION_FILTERS)[number]['id']) {
  if (id === 'farm') return farmRecommendedReps.value.length;
  if (id === 'map') return mapRecommendedReps.value.length;
  if (id === 'both') return bothRecommendedReps.value.length;
  if (id === 'item') return itemRecommendedReps.value.length;
  if (id === 'unset') return unsetRecommendedReps.value.length;
  if (id === 'done') return doneReps.value.length;
  return queriedReputations.value.length;
}
const totalDone = computed(() => evaluatedReputations.value.filter(rep => rep.smartAction === 'done').length);
const openRepId = ref('');
const reputationGroups = computed(() =>
  [false, true]
    .map((done) => ({
      done,
      rows: filteredEvaluatedReputations.value.filter(
        (rep) => (rep.smartAction === 'done') === done
      )
    }))
    .filter((group) => group.rows.length)
);
watch([activeCharacter, jx3ReputationFilter], () => {
  openRepId.value = '';
});
</script>

<template>
  <div class="jx3-panel">
    <header class="jx3-lead">
      <div>
        <h2>声望手札</h2>
        <p>还有成就或心愿就顺路提升；不想重复内容时，改用声望道具。</p>
      </div>
      <div class="jx3-number">
        <strong
          >{{ totalDone
          }}<small> / {{ evaluatedReputations.length }}</small></strong
        ><span>势力已毕业</span>
      </div>
    </header>
    <div class="jx3-toolbar">
      <nav class="jx3-segments" aria-label="声望获取方式">
        <button
          v-for="f in REPUTATION_FILTERS"
          :key="f.id"
          type="button"
          :aria-pressed="jx3ReputationFilter === f.id"
          @click="jx3ReputationFilter = f.id"
        >
          {{ f.label }}<small>{{ reputationFilterCount(f.id) }}</small>
        </button>
      </nav>
      <label class="jx3-search"
        ><Search :size="14" /><input
          v-model="jx3ReputationQuery"
          aria-label="搜索声望"
          placeholder="势力、地图或道具" /><button
          v-if="jx3ReputationQuery"
          type="button"
          aria-label="清空声望搜索"
          @click="jx3ReputationQuery = ''"
        >
          <X :size="14" /></button
      ></label>
    </div>
    <p v-if="jx3CompletedAchieves.size === 0" class="jx3-notice">
      尚未导入茗伊成就，仍可浏览声望目录并设置目标；导入后会自动对照已完成的等级成就。
    </p>
    <p v-if="!evaluatedReputations.length" class="jx3-empty-line">
      声望目录暂未载入。
    </p>
    <p v-else-if="!filteredEvaluatedReputations.length" class="jx3-empty-line">
      没有符合筛选的声望，试试其他方式或清空搜索。
    </p>
    <div v-else class="jx3-record-list">
      <component
        v-for="group in reputationGroups"
        :is="group.done ? 'details' : 'div'"
        :key="`${activeCharacter}/${group.done}`"
        :class="{ 'jx3-completed': group.done }"
        :open="group.done && jx3ReputationFilter === 'done' ? true : undefined"
      >
        <summary v-if="group.done">
          <Check :size="14" />已毕业 {{ group.rows.length }} 个势力<ChevronDown
            :size="14"
          />
        </summary>
        <article v-for="rep in group.rows" :key="rep.id" class="jx3-record">
          <button
            type="button"
            class="jx3-record-head"
            :aria-expanded="openRepId === rep.id"
            @click="openRepId = openRepId === rep.id ? '' : rep.id"
          >
            <Jx3BoxIcon :src="jx3ReputationIconUrl(rep)" size="jx3-reputation-badge" />
            <div class="jx3-record-copy">
              <h3>{{ rep.name }}</h3>
              <p>
                {{ rep.group
                }}<template v-if="rep.place"> · {{ rep.place }}</template>
              </p>
              <p>
                {{ reputationLaneLabel(rep.smartAction)
                }}<template v-if="rep.smartReason">
                  · {{ rep.smartReason }}</template
                >
              </p>
            </div>
            <div class="jx3-record-count">
              <strong>{{ rep.levelSource === 'unknown' ? '待同步' : '至少' + rep.currentLevel }}</strong
              ><small>{{ rep.levelSource === 'achievement' ? '成就推断 · ' : rep.levelSource === 'recorded' ? '已有记录 · ' : '' }}目标 {{ rep.maxLevelOverride || rep.maxLevel }}</small>
            </div>
            <ChevronDown :size="15" />
          </button>
          <div v-if="openRepId === rep.id" class="jx3-record-details">
            <Jx3DungeonImage v-if="rep.dungeon || rep.place" :name="rep.dungeon || rep.place || ''" size="jx3-reputation-scene" />
            <p class="text-muted">{{ rep.achievementCount ? `关联内容：${rep.achievementCount} 项成就，${jx3CompletedAchieves.size ? rep.missingAchievementCount + ' 项待补' : '尚未导入完成记录'}` : '此处成就关联尚未确认' }} · {{ rep.missingCollectCount }} 件已添加心愿待收集</p>
            <div class="jx3-options">
              <span>接下来</span>
              <button type="button" :aria-pressed="!rep.preferredMethod || rep.preferredMethod === 'auto'" @click="setReputationMethod(rep.id, 'auto')">自动建议</button>
              <button type="button" :aria-pressed="rep.preferredMethod === 'content'" @click="setReputationMethod(rep.id, 'content')">继续做内容</button>
              <button type="button" :aria-pressed="rep.preferredMethod === 'item'" @click="setReputationMethod(rep.id, 'item')">不再刷 · 用道具</button>
            </div>
            <p v-if="rep.smartAction === 'item'" class="jx3-notice">{{ rep.recommendedItem ? '目录可用道具：' + rep.recommendedItem : '目录未明确列出适用声望道具，使用前请核对道具说明与等级上限。' }}</p>
            <p class="text-faint">成就记录只证明曾达到该等级，无法提供当前精确声望值。</p>
            <div class="jx3-options">
              <span>毕业等级</span
              ><button
                v-for="level in REPUTATION_GOALS"
                :key="level"
                type="button"
                :aria-pressed="(rep.maxLevelOverride || rep.maxLevel) === level"
                @click="setReputationGoal(rep.id, level)"
              >
                {{ level }}
              </button>
            </div>
            <div class="jx3-options">
              <span>获取方式</span
              ><button
                v-for="lane in REPUTATION_LANES"
                :key="lane.id"
                type="button"
                :aria-pressed="(rep.laneOverride || rep.lane) === lane.id"
                @click="setReputationLane(rep.id, lane.id)"
              >
                {{ lane.label }}
              </button>
            </div>
            <p v-if="rep.rewardHighlight" class="text-muted">
              {{ rep.rewardHighlight }}
            </p>
          </div>
        </article>
      </component>
    </div>
  </div>
</template>
