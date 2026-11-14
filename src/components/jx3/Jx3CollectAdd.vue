<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Jx3CollectKind, Jx3SetTokenSeries } from '../../types';
import {
  addJx3CollectBoxHit,
  addJx3CollectCustom,
  jx3BoxDungeons,
  jx3BoxItemHits,
  jx3BoxItemSearching,
  jx3DynamicSets,
  jx3PendingDungeonHit,
  assignPendingHitDungeon,
  clearPendingHitDungeon,
  addJx3DynamicSetAll,
  addJx3DynamicPiece,
  jx3Collect,
  jx3CollectActiveDungeonName,
  jx3CollectAdd,
  jx3CollectDungeonHits,
  pinJx3CollectDungeon,
  scheduleJx3BoxItemSearch
} from '../../workspace/jx3/collect';
import {
  detectDynamicSets,
  jx3BoxItemIconUrl,
  JX3_COLLECT_KIND_LABEL
} from '../../services/jx3Collect';
import { JX3_SET_TOKENS_CATALOG } from '../../services/jx3CollectData';
import { Plus, Search, Shirt, X } from '@lucide/vue';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';
const kinds: Jx3CollectKind[] = ['pendant', 'special', 'appearance', 'token'];
const setCatalogSearch = ref('');
const pendingDungeonSearch = ref('');
const pendingDungeonFiltered = computed(() => {
  const q = pendingDungeonSearch.value.trim().toLowerCase();
  return jx3BoxDungeons.value
    .filter((d) => !q || `${d.name} ${d.era || ''}`.toLowerCase().includes(q))
    .slice(0, 16);
});
const filteredSetCatalog = computed(() => {
  const q = setCatalogSearch.value.trim().toLowerCase();
  return JX3_SET_TOKENS_CATALOG.filter(
    (series) =>
      !q ||
      `${series.seriesName} ${series.level} ${series.dungeon} ${series.pinyin || ''}`
        .toLowerCase()
        .includes(q)
  );
});
function submitCustom() {
  addJx3CollectCustom(
    jx3CollectAdd.customName || jx3CollectAdd.itemQuery,
    jx3CollectAdd.kind
  );
}
function addSeries(series: Jx3SetTokenSeries) {
  const [dyn] = detectDynamicSets(
    [{ name: series.seriesName, id: series.id }],
    jx3BoxDungeons.value,
    jx3Collect.value.items
  );
  if (dyn) addJx3DynamicSetAll(dyn);
}
</script>

<template>
  <div class="jx3-add-panel grid gap-3">
    <div class="jx3-toolbar">
      <label class="jx3-search max-w-none"
        ><Search :size="14" /><input
          :value="jx3CollectAdd.itemQuery"
          aria-label="搜索新收集物品"
          placeholder="搜牌子、挂件、套装或任意物品"
          @input="
            scheduleJx3BoxItemSearch(($event.target as HTMLInputElement).value)
          "
          @keydown.enter.prevent="submitCustom" /><button
          v-if="jx3CollectAdd.itemQuery"
          type="button"
          aria-label="清空新物品搜索"
          @click="
            jx3CollectAdd.itemQuery = '';
            jx3BoxItemHits = [];
            jx3DynamicSets = [];
          "
        >
          <X :size="14" /></button></label
      ><button type="button" class="jx3-button" @click="submitCustom">
        <Plus :size="13" />手写添加
      </button>
    </div>
    <div class="jx3-options">
      <span>物品类型</span
      ><button
        v-for="kind in kinds"
        :key="kind"
        type="button"
        :aria-pressed="jx3CollectAdd.kind === kind"
        @click="jx3CollectAdd.kind = kind"
      >
        {{ JX3_COLLECT_KIND_LABEL[kind] }}
      </button>
    </div>
    <p class="text-muted text-[12px]">
      添加到：{{ jx3CollectActiveDungeonName || '按搜索结果自动匹配秘境' }}
    </p>
    <details class="jx3-catalog">
      <summary>指定其他秘境</summary>
      <label class="jx3-search"
        ><Search :size="14" /><input
          v-model="jx3CollectAdd.dungeonQuery"
          aria-label="搜索目标秘境"
          placeholder="秘境名称"
      /></label>
      <div class="jx3-options py-2">
        <button
          v-for="d in jx3CollectDungeonHits.length
            ? jx3CollectDungeonHits
            : jx3BoxDungeons.slice(0, 16)"
          :key="d.key"
          type="button"
          :aria-pressed="jx3CollectAdd.dungeonKey === d.key"
          @click="pinJx3CollectDungeon(d)"
        >
          {{ d.name }} <small>{{ d.era }}</small>
        </button>
      </div>
    </details>
    <div v-if="jx3PendingDungeonHit" class="jx3-notice grid gap-2">
      <div class="jx3-toolbar">
        <p>为「{{ jx3PendingDungeonHit.name }}」选择所属秘境</p>
        <button
          type="button"
          aria-label="取消选择所属秘境"
          @click="clearPendingHitDungeon"
        >
          <X :size="14" />
        </button>
      </div>
      <label class="jx3-search"
        ><Search :size="14" /><input
          v-model="pendingDungeonSearch"
          aria-label="筛选物品所属秘境"
          placeholder="搜索秘境"
      /></label>
      <div class="jx3-options">
        <button
          v-for="d in pendingDungeonFiltered"
          :key="d.key"
          type="button"
          @click="assignPendingHitDungeon(d)"
        >
          {{ d.name }}
        </button>
      </div>
    </div>
    <p v-if="jx3BoxItemSearching" role="status" class="text-faint">
      正在搜索物品及套装出处…
    </p>
    <section v-for="dyn in jx3DynamicSets" :key="dyn.seriesName">
      <header class="jx3-section-heading">
        <h3>
          {{ dyn.seriesName }} <small>{{ dyn.level }}</small>
        </h3>
        <button
          type="button"
          class="jx3-button"
          @click="addJx3DynamicSetAll(dyn)"
        >
          整套加入心愿
        </button>
      </header>
      <div
        v-for="(piece, slot) in dyn.pieces"
        :key="slot"
        class="jx3-requirement"
      >
        <div>
          <p>{{ piece.name }}</p>
          <small
            >{{ piece.slotLabel }} · {{ piece.dungeonName || '待定秘境' }} ·
            {{ piece.boss }}</small
          >
        </div>
        <span v-if="piece.inCollect" class="jx3-status is-done">已在清单</span
        ><button
          v-else
          type="button"
          class="jx3-button"
          @click="addJx3DynamicPiece(piece, dyn.seriesName)"
        >
          加入心愿
        </button>
      </div>
    </section>
    <div v-if="jx3BoxItemHits.length" class="max-h-64 overflow-y-auto">
      <div v-for="hit in jx3BoxItemHits" :key="hit.id" class="jx3-requirement">
        <Jx3BoxIcon :src="jx3BoxItemIconUrl(hit.iconId)" size="w-6 h-6" />
        <div class="min-w-0 flex-1">
          <p>{{ hit.name }}</p>
          <small
            >{{
              hit.inferredDungeon?.dungeonName ||
              hit.typeLabel ||
              hit.getType ||
              '掉落物品'
            }}<template v-if="hit.inferredDungeon?.boss">
              · {{ hit.inferredDungeon.boss }}</template
            ></small
          >
        </div>
        <button
          type="button"
          class="jx3-button"
          @click="addJx3CollectBoxHit(hit, undefined, jx3CollectAdd.kind)"
        >
          {{
            hit.inferredDungeon || jx3CollectAdd.dungeonKey
              ? '加入心愿'
              : '选本加入'
          }}
        </button>
      </div>
    </div>
    <details class="jx3-catalog">
      <summary><Shirt :size="14" />经典校服图鉴 · 展开查找套装</summary>
      <label class="jx3-search mb-3"
        ><Search :size="14" /><input
          v-model="setCatalogSearch"
          aria-label="搜索校服图鉴"
          placeholder="套装、年代、秘境或拼音"
      /></label>
      <div class="jx3-catalog-list">
        <section v-for="series in filteredSetCatalog" :key="series.id">
          <header class="jx3-section-heading">
            <div>
              <h3>{{ series.seriesName }}</h3>
              <small>{{ series.level }} · {{ series.dungeon }}</small>
            </div>
            <button type="button" class="jx3-button" @click="addSeries(series)">
              加入整套心愿
            </button>
          </header>
          <div class="grid grid-cols-2 gap-2">
            <p v-for="(piece, slot) in series.pieces" :key="slot">
              {{ piece.name }} <small>{{ piece.boss }}</small>
            </p>
          </div>
        </section>
        <p v-if="!filteredSetCatalog.length" class="jx3-empty-line">
          没有符合搜索的校服。
        </p>
      </div>
    </details>
  </div>
</template>
