<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  addJx3ExteriorManual,
  jx3ExteriorAddText,
  jx3ExteriorBusy,
  jx3ExteriorError,
  jx3ExteriorFailed,
  jx3ExteriorMissingSnapshot,
  jx3ExteriorPricedCount,
  jx3ExteriorRows,
  jx3ExteriorSuggestIndex,
  jx3ExteriorSuggests,
  jx3ExteriorTotalLabel,
  jx3ExteriorUnpriced,
  moveJx3ExteriorSuggest,
  pickJx3ExteriorSuggest,
  refreshJx3Exterior,
  refreshJx3ExteriorSuggests,
  removeJx3ExteriorManual
} from '../../workspace/jx3/exterior';
import { Plus, RefreshCw, Search, X } from '@lucide/vue';
import Jx3BoxIcon from '../Jx3BoxIcon.vue';

const query = ref('');
const status = ref('all');
const sort = ref('default');
const adding = ref(false);
const failedImages = ref<Record<string, boolean>>({});
const visibleRows = computed(() => {
  const q = query.value.trim().toLowerCase();
  const rows = jx3ExteriorRows.value.filter(
    (row) =>
      (status.value === 'all' || row.status === status.value) &&
      (!q || `${row.name} ${row.alias} ${row.band}`.toLowerCase().includes(q))
  );
  if (sort.value === 'price')
    rows.sort((a, b) => (b.price ?? -1) - (a.price ?? -1));
  if (sort.value === 'name')
    rows.sort((a, b) => a.name.localeCompare(b.name, 'zh-CN'));
  return rows;
});
</script>

<template>
  <div class="jx3-panel">
    <header class="jx3-lead">
      <div>
        <h2>衣冠藏品</h2>
        <p>
          已标价 {{ jx3ExteriorPricedCount }} 件 · 无市价
          {{ jx3ExteriorUnpriced }} 件 · 拉取失败 {{ jx3ExteriorFailed }} 件
        </p>
      </div>
      <div class="jx3-number">
        <strong>{{ jx3ExteriorTotalLabel }}</strong
        ><span>当前市价合计 · {{ jx3ExteriorRows.length }} 件外观</span>
      </div>
    </header>
    <div class="jx3-toolbar">
      <label class="jx3-search"
        ><Search :size="14" /><input
          v-model="query"
          aria-label="搜索持有外观"
          placeholder="名称、别名或系列" /><button
          v-if="query"
          type="button"
          aria-label="清空外观搜索"
          @click="query = ''"
        >
          <X :size="14" /></button
      ></label>
      <div class="jx3-segments">
        <select
          v-model="status"
          aria-label="按外观价格状态筛选"
          class="jx3-select"
        >
          <option value="all">全部价格状态</option>
          <option value="priced">已标价</option>
          <option value="unpriced">无市价</option>
          <option value="failed">拉取失败</option></select
        ><select v-model="sort" aria-label="外观排序" class="jx3-select">
          <option value="default">默认排序</option>
          <option value="price">价格从高到低</option>
          <option value="name">按名称</option></select
        ><button
          type="button"
          class="jx3-button"
          :disabled="jx3ExteriorBusy"
          @click="refreshJx3Exterior()"
        >
          <RefreshCw :size="13" />{{
            jx3ExteriorBusy ? '读取中…' : '刷新市价'
          }}</button
        ><button
          type="button"
          class="jx3-button is-primary"
          :aria-expanded="adding"
          @click="adding = !adding"
        >
          <Plus :size="13" />手记外观
        </button>
      </div>
    </div>
    <div v-if="adding" class="jx3-add-panel space-y-2">
      <div class="flex items-center gap-2">
        <input
          v-model="jx3ExteriorAddText"
          type="text"
          aria-label="手记外观名称或商品链接"
          placeholder="手记一件：别名、名称片段，或贴商品链接"
          class="flex-1 min-w-0 bg-transparent text-[14px] text-ink placeholder-faint focus:outline-none"
          @input="refreshJx3ExteriorSuggests()"
          @keydown.down.prevent="moveJx3ExteriorSuggest(1)"
          @keydown.up.prevent="moveJx3ExteriorSuggest(-1)"
          @keydown.enter.prevent="pickJx3ExteriorSuggest()"
          @keydown.esc="jx3ExteriorSuggests = []"
        />
        <button
          type="button"
          @click="addJx3ExteriorManual()"
          :disabled="jx3ExteriorBusy"
          class="text-[13px] text-beacon font-medium px-1.5 py-0.5 cursor-pointer shrink-0 flex items-center gap-1 disabled:opacity-50"
        >
          <Plus class="w-3.5 h-3.5" />
          添加
        </button>
      </div>
      <div v-if="jx3ExteriorSuggests.length" class="space-y-1">
        <button
          v-for="(hit, idx) in jx3ExteriorSuggests"
          :key="hit.key"
          type="button"
          @click="pickJx3ExteriorSuggest(hit)"
          :class="
            jx3ExteriorSuggestIndex === idx
              ? 'bg-raised text-ink'
              : 'text-muted hover:text-ink hover:bg-raised/70'
          "
          class="w-full text-left px-2 py-1.5 rounded-md text-[13px] transition cursor-pointer flex items-center justify-between gap-2"
        >
          <span class="min-w-0 truncate">
            {{ hit.name }}
            <span v-if="hit.alias" class="text-faint"> {{ hit.alias }}</span>
          </span>
          <span class="text-[12px] text-faint shrink-0">
            {{ hit.source === 'book' ? '价格簿' : '商品' }}
            <span v-if="hit.band"> · {{ hit.band }}</span>
            <span v-if="hit.price">
              · ¥{{ hit.price.toLocaleString('zh-CN')
              }}<template v-if="hit.priceSource">
                {{ hit.priceSource }}</template
              ></span
            >
          </span>
        </button>
      </div>
    </div>
    <p v-if="jx3ExteriorError" class="jx3-notice is-error">
      {{ jx3ExteriorError }}
    </p>
    <p
      v-if="jx3ExteriorMissingSnapshot && !jx3ExteriorRows.length"
      class="jx3-notice"
    >
      这个角色还没有外观快照。可以先手记一件，或进入游戏后刷新快照。
    </p>
    <p v-else-if="!jx3ExteriorRows.length" class="jx3-empty-line">
      还没有外观记录，点击「手记外观」开始收藏。
    </p>
    <p v-else-if="!visibleRows.length" class="jx3-empty-line">
      没有符合搜索与价格状态的外观。
    </p>
    <div v-else>
      <header class="jx3-section-heading">
        <h3>持有外观</h3>
        <span>{{ visibleRows.length }} 件 · 价格参考万宝楼</span>
      </header>
      <div class="jx3-exterior-grid">
      <article v-for="row in visibleRows" :key="row.key" class="jx3-exterior-row">
        <a v-if="row.imageUrl && !failedImages[row.imageUrl]" :href="row.url" target="_blank" rel="noopener noreferrer" class="jx3-exterior-art">
          <img :src="row.imageUrl" :alt="row.name" loading="lazy" referrerpolicy="no-referrer" @error="failedImages[row.imageUrl] = true" />
        </a>
        <a
          :href="row.url || undefined"
          target="_blank"
          rel="noopener noreferrer"
          class="min-w-0"
          ><Jx3BoxIcon :src="row.iconUrl" size="jx3-exterior-icon" /><h3>
            {{ row.name }} <small v-if="row.source === 'manual'">手记</small>
          </h3>
          <p v-if="row.alias || row.band">{{ row.alias }} {{ row.band }}</p>
          <p v-if="row.failedAt">
            最近拉取失败 · {{ row.failedAt.slice(0, 16).replace('T', ' ') }}
          </p></a
        >
        <div>
          <strong>{{
            row.status === 'priced' && row.price !== null
              ? '¥' + row.price.toLocaleString('zh-CN')
              : row.status === 'failed'
                ? '拉取失败'
                : '无市价'
          }}</strong
          ><small v-if="row.priceSource">{{ row.priceSource }}</small>
        </div>
        <button
          v-if="row.source === 'manual'"
          type="button"
          class="jx3-remove"
          :aria-label="'移出手记：' + row.name"
          @click="removeJx3ExteriorManual(row.name)"
        >
          <X :size="14" />
        </button>
      </article>
      </div>
    </div>
  </div>
</template>
