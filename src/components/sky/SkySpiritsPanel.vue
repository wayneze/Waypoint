<script setup lang="ts">
import { computed, ref } from 'vue';
import SkySeasonsPanel from './SkySeasonsPanel.vue';
import { collectionCanEdit } from '../../workspace/sky/collection';
import { ATLAS_ACTIONS } from '../../services/skyAtlas';
import { vWikiImg } from './wikiImage';
import {
  collectionMapFilter,
  collectionSortBy,
  currentMapStats,
  filteredCollectionItems,
  nextMap,
  obtainedCoreItemsCount,
  prevMap,
  toggleSpiritGraduated,
  toggleSpiritItem,
  toggleSpiritWing,
  totalWings,
  unlockedSpiritsCount,
  unlockedWings,
} from '../../workspace/sky/spirits';
import { hideTooltip, state, triggerTooltip } from '../../workspace/core';

defineProps<{ source: 'permanent' | 'season' }>();
const showCompleted = ref(false);
const completed = computed(() => filteredCollectionItems.value.filter(i => i.status === 'unlocked'));
const visible = computed(() => filteredCollectionItems.value.filter(i => showCompleted.value || i.status !== 'unlocked'));
const portraits = new Map(ATLAS_ACTIONS.filter(action => action.constellationId && action.image)
  .map(action => [action.constellationId, action.image]));
const failedPortraits = ref<Record<string, boolean>>({});
</script>

<template>
  <div class="space-y-4">
    <SkySeasonsPanel v-if="source === 'season'" />
    <template v-else>
    <div class="wp-panel-filters flex flex-wrap items-center gap-2">
      <span class="text-[13px] text-muted font-mono shrink-0">{{ unlockedWings }}/{{ totalWings }} 翼 · {{ obtainedCoreItemsCount }}/{{ state.collectionItems.length }} 装扮 · {{ unlockedSpiritsCount }} 毕业</span>
      <div class="flex-1 min-w-0"></div>
      <div class="flex items-center gap-0.5 overflow-x-auto no-scrollbar">
        <button v-for="m in [
                  { key: 'all', label: '全图' },
                  { key: '晨岛', label: '晨岛' },
                  { key: '云野', label: '云野' },
                  { key: '雨林', label: '雨林' },
                  { key: '霞谷', label: '霞谷' },
                  { key: '暮土', label: '暮土' },
                  { key: '禁阁', label: '禁阁' }
                ]" :key="m.key"
                type="button"
                @click="collectionMapFilter = m.key as any"
                :class="collectionMapFilter === m.key ? 'bg-raised text-ink font-semibold' : 'text-muted hover:text-ink hover:bg-raised/70'"
                class="px-2 py-1 rounded-md text-[13px] transition cursor-pointer shrink-0">
          {{ m.label }}
        </button>
      </div>
      <select v-model="collectionSortBy" class="bg-canvas border border-hairline text-[13px] text-ink rounded-md px-1.5 py-1 focus:outline-none cursor-pointer shrink-0">
        <option value="default">星图顺序</option>
        <option value="missing_wing">待拿光翼</option>
        <option value="missing_item">待换装扮</option>
        <option value="locked_only">待毕业</option>
        <option value="red_asc">红蜡低到高</option>
        <option value="red_desc">红蜡高到低</option>
        <option value="wings_desc">光翼优先</option>
      </select>
    </div>

    <div class="wp-spirit-list">
      <div v-for="item in visible" :key="item.id"
           @mouseenter="triggerTooltip($event, item)"
           @mouseleave="hideTooltip"
           class="wp-spirit-entry">
        <img v-if="portraits.get(item.id) && !failedPortraits[item.id]" v-wiki-img="portraits.get(item.id) || ''" :alt="item.name"
             class="wp-spirit-portrait" @wikifail="failedPortraits[item.id] = true" />
        <div class="wp-spirit-description">
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0">
              <h3 class="text-[16px] font-semibold text-ink leading-snug truncate">{{ item.name }}</h3>
              <p class="text-[13px] text-faint mt-0.5">{{ item.map }} · {{ item.action }}</p>
            </div>
            <button type="button" :disabled="!collectionCanEdit" @click.stop="toggleSpiritGraduated(item)"
                    class="text-[13px] shrink-0 transition cursor-pointer"
                    :class="item.status === 'unlocked' ? 'text-done' : 'text-faint hover:text-ink'">
              {{ item.status === 'unlocked' ? '毕业' : '待毕业' }}
            </button>
          </div>
          <p class="text-[13px] text-muted mt-1 leading-snug" :title="item.location">{{ item.location }}</p>
        </div>
        <div class="wp-spirit-progress">
          <button type="button" :disabled="!collectionCanEdit" @click.stop="toggleSpiritWing(item)"
                  class="wp-progress-toggle" :aria-pressed="item.wingUnlocked"
                  :class="item.wingUnlocked ? 'text-done' : 'text-muted hover:text-ink'"
                  :title="`永久翼 +${item.wings}`">
            <span class="wp-progress-check" aria-hidden="true">{{ item.wingUnlocked ? '✓' : '' }}</span>
            <span class="min-w-0 truncate">永久翼 +{{ item.wings }}</span>
            <span class="shrink-0">{{ item.wingUnlocked ? '已拿' : `${item.redCandles}红蜡` }}</span>
          </button>
          <button type="button" :disabled="!collectionCanEdit" @click.stop="toggleSpiritItem(item)"
                  class="wp-progress-toggle" :aria-pressed="item.itemObtained"
                  :class="item.itemObtained ? 'text-done' : 'text-muted hover:text-ink'"
                  :title="item.coreItem ? `${item.coreItem} (${item.coreCost})` : item.keyItems">
            <span class="wp-progress-check" aria-hidden="true">{{ item.itemObtained ? '✓' : '' }}</span>
            <span class="min-w-0 truncate">{{ item.coreItem || '核心装扮' }}</span>
            <span class="shrink-0">{{ item.itemObtained ? '已换' : (item.coreCost || '需爱心') }}</span>
          </button>
        </div>
      </div>
    </div>

    <button v-if="completed.length" type="button" class="wp-text-action text-sm" :aria-expanded="showCompleted" @click="showCompleted = !showCompleted">{{ showCompleted ? '收起' : '显示' }}已毕业先祖 · {{ completed.length }}</button>
    <div v-if="collectionMapFilter !== 'all'" class="flex items-center justify-between px-0.5">
      <button type="button" @click="prevMap" class="text-[13px] text-muted hover:text-ink transition cursor-pointer">上一地图</button>
      <span class="text-[13px] text-faint">{{ collectionMapFilter }} · {{ currentMapStats.totalSpirits }} 位</span>
      <button type="button" @click="nextMap" class="text-[13px] text-muted hover:text-ink transition cursor-pointer">下一地图</button>
    </div>
    </template>
  </div>
</template>
