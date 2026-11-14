<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Check, ChevronDown, Package, Plus, Search, Trash2, X } from '@lucide/vue';
import { activeCharacter, activeCharObj } from '../../workspace/core';
import type { Jx3CollectKind } from '../../types';
import {
  jx3Collect, jx3CollectAdd, jx3CollectGroups, jx3CollectKindFilter, jx3CollectQuery,
  selectJx3CollectDungeon, removeJx3CollectItem, jx3CollectProgress, jx3CollectGroupCd,
  toggleJx3CollectCd, toggleJx3CollectItem, unpinJx3CollectDungeon,
} from '../../workspace/jx3/collect';
import { jx3BoxDungeonThumbUrl, jx3CollectItemIconUrl, JX3_COLLECT_KIND_LABEL } from '../../services/jx3Collect';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
import Jx3CollectAdd from './Jx3CollectAdd.vue';
import Jx3DungeonMap from './Jx3DungeonMap.vue';
import { jx3CategoryImage } from '../../services/jx3Artwork';
const kinds: Array<{ id: 'all' | Jx3CollectKind; label: string }> = [
  { id: 'all', label: '全部' }, { id: 'pendant', label: '挂件' },
  { id: 'special', label: '特殊' }, { id: 'appearance', label: '外观' }, { id: 'token', label: '牌子' },
];
const showAdd = ref(false);
const view = ref<'all' | 'single'>('all');
const era = ref('all');
const eras = computed(() => [...new Set(jx3CollectGroups.value.map(group => group.era).filter((value): value is string => !!value))]);
const visibleGroups = computed(() => jx3CollectGroups.value.filter(group => era.value === 'all' || group.era === era.value));
const selectedGroup = computed(() => visibleGroups.value.find(group => group.dungeonKey === jx3CollectAdd.dungeonKey));
const displayedGroups = computed(() => view.value === 'all' ? visibleGroups.value : selectedGroup.value ? [selectedGroup.value] : []);
function sections(group: (typeof visibleGroups.value)[number]) {
  return [false, true].map(done => ({ done, rows: group.items.filter(item => item.obtained === done) })).filter(section => section.rows.length);
}
watch(visibleGroups, groups => {
  if (groups.length && !groups.some(group => group.dungeonKey === jx3CollectAdd.dungeonKey))
    selectJx3CollectDungeon(groups[0].dungeonKey, groups[0].dungeonName);
}, { immediate: true });
watch(activeCharacter, () => { era.value = 'all'; showAdd.value = false; jx3CollectAdd.itemQuery = ''; });
</script>

<template>
  <div class="jx3-panel jx3-collection">
    <div class="jx3-toolbar">
      <nav class="jx3-segments" aria-label="收集品类">
        <button v-for="kind in kinds" :key="kind.id" type="button" :aria-pressed="jx3CollectKindFilter === kind.id" @click="jx3CollectKindFilter = kind.id">{{ kind.label }}</button>
      </nav>
      <label class="jx3-search"><Search :size="14" /><input v-model="jx3CollectQuery" aria-label="搜索已有心愿" placeholder="心愿、秘境或首领" />
        <button v-if="jx3CollectQuery" type="button" aria-label="清空心愿搜索" @click="jx3CollectQuery = ''"><X :size="14" /></button>
      </label>
      <div class="flex items-center gap-3">
        <small>{{ jx3CollectProgress.obtained }} / {{ jx3CollectProgress.total }} 已获得</small>
        <button type="button" class="jx3-button is-primary" :aria-expanded="showAdd" :disabled="activeCharObj?.game_id !== 'jx3'" @click="showAdd = !showAdd"><Plus :size="14" />添加心愿</button>
      </div>
    </div>
    <Jx3CollectAdd v-if="showAdd" />
    <nav class="jx3-segments" aria-label="副本浏览模式">
      <button type="button" :aria-pressed="view === 'all'" @click="view = 'all'">全部副本</button>
      <button type="button" :aria-pressed="view === 'single'" @click="view = 'single'">单个副本</button>
    </nav>
      <nav v-if="eras.length > 1" class="jx3-segments" aria-label="按收集版本筛选">
        <button type="button" :aria-pressed="era === 'all'" @click="era = 'all'">全部版本</button>
        <button v-for="item in eras" :key="item" type="button" :aria-pressed="era === item" @click="era = item">{{ item }}</button>
      </nav>
    <div v-if="!visibleGroups.length" class="jx3-collect-empty">
      <Package :size="28" /><h3>{{ jx3Collect.items.length ? '没有符合筛选的心愿' : (activeCharObj?.name || '当前角色') + '还没有副本心愿' }}</h3>
      <p>添加想收集的物品，按秘境记录自己的进度。</p>
      <button type="button" class="jx3-button" :disabled="activeCharObj?.game_id !== 'jx3'" @click="showAdd = true"><Plus :size="14" />添加第一个心愿</button>
    </div>
    <template v-else>
      <nav v-if="view === 'single'" class="jx3-dungeon-nav" aria-label="收集秘境">
        <button v-for="group in visibleGroups" :key="group.dungeonKey" type="button"
                :aria-current="selectedGroup?.dungeonKey === group.dungeonKey ? 'true' : undefined"
                @click="selectJx3CollectDungeon(group.dungeonKey, group.dungeonName)">
          <span>{{ group.dungeonName }}</span><small>{{ group.obtainedCount }}/{{ group.totalCount }}</small>
        </button>
      </nav>
      <section v-for="selectedGroup in displayedGroups" :key="selectedGroup.dungeonKey" class="jx3-collect-detail" :aria-label="selectedGroup.dungeonName + '收集清单'">
        <header class="jx3-dungeon-scene">
          <Jx3BoxIcon :src="jx3BoxDungeonThumbUrl(selectedGroup.icon)" size="jx3-scene-image" />
          <div class="jx3-scene-copy">
            <p>{{ selectedGroup.era || '秘境收集' }}<span>{{ selectedGroup.eraLevel ? selectedGroup.eraLevel + '级' : '' }}</span></p>
            <h2>{{ selectedGroup.dungeonName }}</h2>
            <p>{{ selectedGroup.obtainedCount }} 件已获得 · {{ selectedGroup.totalCount - selectedGroup.obtainedCount }} 件待收集</p>
            <div class="jx3-scene-actions">
              <button v-if="jx3CollectGroupCd(selectedGroup.dungeonKey).rule !== 'none'" type="button" class="jx3-button"
                      :aria-pressed="!jx3CollectGroupCd(selectedGroup.dungeonKey).open" @click="toggleJx3CollectCd(selectedGroup.dungeonKey)">
                <Check v-if="!jx3CollectGroupCd(selectedGroup.dungeonKey).open" :size="13" />{{ jx3CollectGroupCd(selectedGroup.dungeonKey).open ? '本周期未打' : '本周期已打' }}
              </button>
              <button v-if="jx3Collect.pinnedDungeons.some(pin => pin.dungeonKey === selectedGroup?.dungeonKey)" type="button" class="wp-text-action" @click="unpinJx3CollectDungeon(selectedGroup.dungeonKey)">取消秘境置顶</button>
            </div>
          </div>
        </header>
        <div class="jx3-collect-content" :class="{ 'is-overview': view === 'all' }">
          <div class="min-w-0">
            <h3 class="jx3-collect-title">物品心愿<small>{{ selectedGroup.items.length }} 件</small></h3>
            <p v-if="!selectedGroup.items.length" class="jx3-empty-line">还没有物品，点击「添加心愿」开始。</p>
            <component v-for="section in sections(selectedGroup)" :is="section.done ? 'details' : 'div'"
                       :key="`${activeCharacter}/${selectedGroup.dungeonKey}/${section.done}/${section.done ? section.rows.map(item => item.id).join(',') : ''}`"
                       :class="{ 'jx3-completed': section.done }">
              <summary v-if="section.done"><Check :size="14" />已获得 {{ section.rows.length }} 件<ChevronDown :size="14" /></summary>
              <div class="jx3-item-grid">
                <article v-for="item in section.rows" :key="item.id" class="jx3-item-card" :class="{ 'is-done': item.obtained }">
                  <div class="jx3-item-art">
                    <Jx3BoxIcon v-if="jx3CollectItemIconUrl(item)" :src="jx3CollectItemIconUrl(item)" size="jx3-item-icon" />
                    <Jx3BoxIcon v-else :src="jx3CategoryImage(item.kind === 'special' ? '杂闻' : '装备')" size="jx3-item-category" />
                  </div>
                  <div class="jx3-item-copy">
                    <small>{{ JX3_COLLECT_KIND_LABEL[item.kind] }}</small><h4>{{ item.name }}</h4>
                    <p v-if="item.boss || item.note">{{ item.boss || item.note }}</p>
                    <button type="button" class="jx3-item-check" :aria-pressed="item.obtained" :aria-label="(item.obtained ? '取消获得：' : '获得：') + item.name" @click="toggleJx3CollectItem(item.id)">
                      <span><Check v-if="item.obtained" :size="12" /></span>{{ item.obtained ? '已获得' : '记为获得' }}
                    </button>
                  </div>
                  <button type="button" class="jx3-remove" :aria-label="'移出心愿：' + item.name" @click="removeJx3CollectItem(item.id)"><Trash2 :size="13" /></button>
                </article>
              </div>
            </component>
          </div>
          <Jx3DungeonMap v-if="view === 'single'" :dungeon-key="selectedGroup.dungeonKey" :complete="selectedGroup.totalCount > 0 && selectedGroup.totalCount === selectedGroup.obtainedCount" />
        </div>
      </section>
    </template>
  </div>
</template>
