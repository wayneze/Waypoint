<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Archive, ArrowUpRight, ListTodo, Pin, Search, ScrollText, X } from '@lucide/vue';
import MemoEditor from '../components/memos/MemoEditor.vue';
import MemoRow from '../components/memos/MemoRow.vue';
import {
  availableMemoTags, completedMemos, customMemos, memoQuery, memoStats,
  pendingMemos, selectedMemoTag,
} from '../workspace/memos';
import { handleProjectToObsidian } from '../workspace/bootstrap';
import { isProjectingObsidian } from '../workspace/core';
import { kvGet } from '../services/kv';

const view = ref<'pending' | 'completed'>('pending');
const editingId = ref('');
const content = ref<HTMLElement>();
const composer = ref<InstanceType<typeof MemoEditor>>();
const defaultTag = computed(() => selectedMemoTag.value === 'all' ? kvGet('waypoint_memo_last_tag') || '通用' : selectedMemoTag.value);
const counts = computed(() => {
  const result = new Map<string, number>();
  for (const memo of customMemos.value) result.set(memo.tag, (result.get(memo.tag) || 0) + 1);
  return result;
});
const filtered = computed(() => !!memoQuery.value.trim() || selectedMemoTag.value !== 'all');
const groups = computed(() => view.value === 'completed'
  ? [{ id: 'completed', label: '已完成', items: completedMemos.value }]
  : [
    { id: 'pinned', label: '置顶', items: pendingMemos.value.filter(memo => memo.pinned) },
    { id: 'pending', label: '待办事项', items: pendingMemos.value.filter(memo => !memo.pinned) },
  ]);
const visibleCount = computed(() => view.value === 'pending' ? pendingMemos.value.length : completedMemos.value.length);
watch([view, selectedMemoTag, memoQuery], () => {
  editingId.value = '';
  if (content.value) content.value.scrollTop = 0;
});
function clearFilters() { selectedMemoTag.value = 'all'; memoQuery.value = ''; }
async function created(tag: string) {
  view.value = 'pending';
  memoQuery.value = '';
  if (selectedMemoTag.value !== 'all') selectedMemoTag.value = tag;
  await nextTick();
  if (content.value) content.value.scrollTop = 0;
}
async function focusComposer() {
  view.value = 'pending';
  await nextTick();
  composer.value?.focus();
}
</script>

<template>
  <section class="memo-workspace" aria-label="万象记事">
    <header class="memo-heading">
      <div><p class="memo-eyebrow">琐事与心愿，各有着落</p><h1>万象记事<span>.</span></h1><p class="memo-subtitle">记下惦记的事，按自己的节奏完成。</p></div>
      <div class="memo-heading-right">
        <p class="memo-summary"><strong>{{ memoStats.pending }}</strong><span>待办</span><i>/</i><strong>{{ memoStats.completed }}</strong><span>已完成</span></p>
        <button type="button" class="memo-project" :disabled="isProjectingObsidian" @click="handleProjectToObsidian('memos')">{{ isProjectingObsidian ? '投影中…' : '投影到 Obsidian' }}<ArrowUpRight /></button>
      </div>
    </header>
    <nav class="memo-categories" aria-label="记事分类">
      <button type="button" :aria-current="selectedMemoTag === 'all' ? 'page' : undefined" @click="selectedMemoTag = 'all'">全部<span>{{ memoStats.total }}</span></button>
      <button v-for="tag in availableMemoTags" :key="tag" type="button" :aria-current="selectedMemoTag === tag ? 'page' : undefined" @click="selectedMemoTag = tag">{{ tag }}<span>{{ counts.get(tag) || 0 }}</span></button>
    </nav>
    <div class="memo-browse-bar">
      <nav class="memo-views" aria-label="记事状态"><button type="button" :aria-pressed="view === 'pending'" @click="view = 'pending'"><ListTodo />待办<span>{{ pendingMemos.length }}</span></button><button type="button" :aria-pressed="view === 'completed'" @click="view = 'completed'"><Archive />已完成<span>{{ completedMemos.length }}</span></button></nav>
      <label class="memo-search"><Search aria-hidden="true" /><input v-model="memoQuery" type="search" aria-label="搜索记事标题、备注或分类" placeholder="搜索标题、备注或分类" /><button v-if="memoQuery" type="button" aria-label="清空搜索" @click="memoQuery = ''"><X /></button></label>
    </div>
    <MemoEditor ref="composer" :default-tag="defaultTag" @saved="created" />
    <div ref="content" class="memo-content" role="region" :aria-label="view === 'pending' ? '待办记事列表' : '已完成记事列表'" tabindex="0">
      <template v-for="group in groups" :key="group.id">
        <section v-if="group.items.length" class="memo-group" :aria-label="group.label">
          <header class="memo-group-heading"><Pin v-if="group.id === 'pinned'" /><h2>{{ group.label }}</h2><span>{{ group.items.length }}</span><p v-if="group.id === 'completed'">重复事项将在下个周期回到待办</p></header>
          <MemoRow v-for="memo in group.items" :key="memo.id" :memo="memo" :editing="editingId === memo.id" @edit="editingId = memo.id" @close="editingId = ''" />
        </section>
      </template>
      <div v-if="!visibleCount" class="memo-empty">
        <Search v-if="filtered" /><Archive v-else-if="view === 'completed'" /><ScrollText v-else />
        <h2>{{ filtered ? '没有找到对应的记事' : view === 'completed' ? '完成的事，会留在这里' : customMemos.length ? '惦记的事，暂时都做完了' : '把第一件惦记的事记下来' }}</h2>
        <p>{{ filtered ? '换一个关键词，或查看其他分类。' : view === 'completed' ? '可以在这里回看，也可以恢复为待办。' : '一件小事、一段心愿，或一份慢慢完成的清单。' }}</p>
        <button v-if="filtered" type="button" @click="clearFilters">清除筛选<ArrowUpRight /></button>
        <button v-else-if="view === 'pending'" type="button" @click="focusComposer">写下第一笔<ArrowUpRight /></button>
      </div>
    </div>
    <footer class="memo-footer"><span>{{ filtered ? '当前筛选' : '全部记事' }} · {{ visibleCount }} 条{{ view === 'pending' ? '待办' : '已完成' }}</span><span>每日与每周事项自动轮转</span></footer>
  </section>
</template>

<style scoped>
.memo-workspace { width: 100%; height: 100%; min-height: 0; max-width: 1440px; margin-inline: auto; display: flex; flex-direction: column; container-type: inline-size; }
.memo-heading { display: flex; justify-content: space-between; align-items: center; gap: 20px; padding-bottom: 24px; flex-shrink: 0; }
.memo-eyebrow { font-size: 12px; color: var(--color-faint); letter-spacing: .08em; }
.memo-heading h1 { font-size: 32px; line-height: 1.25; letter-spacing: -.04em; font-weight: 600; margin: 4px 0 8px; }
.memo-heading h1 > span { color: var(--color-beacon); }
.memo-subtitle { font-size: 12px; color: var(--color-muted); }
.memo-heading-right { display: grid; justify-items: end; gap: 14px; flex-shrink: 0; }
.memo-summary { display: flex; align-items: baseline; gap: 7px; font-size: 12px; color: var(--color-faint); white-space: nowrap; }
.memo-summary strong { color: var(--color-ink); font-size: 25px; line-height: 1; font-weight: 400; font-variant-numeric: tabular-nums; }
.memo-summary i { margin: 0 10px; font-style: normal; color: var(--color-hairline); }
.memo-project { display: flex; align-items: center; gap: 5px; min-height: 28px; font-size: 12px; color: var(--color-faint); cursor: pointer; }
.memo-project:hover { color: var(--color-ink); }
.memo-project svg { width: 13px; height: 13px; }
.memo-project:disabled { opacity: .5; cursor: default; }
.memo-categories { display: flex; gap: 20px; border-bottom: 1px solid var(--color-hairline); overflow-x: auto; flex-shrink: 0; }
.memo-categories button { display: flex; align-items: center; gap: 8px; padding: 10px 3px; border-bottom: 2px solid transparent; font-size: 13px; color: var(--color-muted); white-space: nowrap; cursor: pointer; }
.memo-categories button:hover { color: var(--color-ink); }
.memo-categories [aria-current="page"] { color: var(--color-beacon); border-color: var(--color-beacon); }
.memo-categories button span { color: var(--color-faint); font-size: 12px; font-variant-numeric: tabular-nums; }
.memo-browse-bar { display: flex; justify-content: space-between; align-items: center; gap: 16px; padding: 16px 0 12px; flex-shrink: 0; }
.memo-views { display: flex; gap: 4px; flex-shrink: 0; }
.memo-views button { display: inline-flex; align-items: center; gap: 7px; min-height: 32px; padding: 5px 10px; border-radius: 4px; font-size: 12px; color: var(--color-muted); cursor: pointer; }
.memo-views button[aria-pressed="true"] { color: var(--color-ink); background: var(--color-raised); }
.memo-views button span { color: var(--color-faint); font-variant-numeric: tabular-nums; }
.memo-views svg { width: 14px; height: 14px; }
.memo-search { display: flex; align-items: center; gap: 8px; width: clamp(200px, 30%, 320px); min-width: 0; height: 32px; padding: 0 9px; border: 1px solid var(--color-hairline); border-radius: 4px; color: var(--color-faint); background: var(--color-chrome); }
.memo-search > svg, .memo-search button svg { width: 14px; height: 14px; flex-shrink: 0; }
.memo-search input { min-width: 0; width: 100%; background: transparent; font-size: 12px; color: var(--color-ink); outline: none; }
.memo-search input::-webkit-search-cancel-button { display: none; }
.memo-search:focus-within { border-color: var(--color-beacon); }
.memo-search button { cursor: pointer; }
.memo-workspace > :deep(.memo-editor) { flex-shrink: 0; max-height: 55%; overflow-y: auto; scrollbar-gutter: stable; }
.memo-content { min-height: 0; flex: 1; overflow-y: auto; overscroll-behavior-y: contain; scrollbar-gutter: stable; padding-right: 8px; }
.memo-group-heading { display: flex; align-items: center; gap: 8px; padding: 22px 2px 0; color: var(--color-faint); font-size: 12px; }
.memo-group-heading h2 { color: var(--color-muted); font-weight: 500; }
.memo-group-heading > svg { width: 12px; height: 12px; color: var(--color-beacon); }
.memo-group-heading > span { font-variant-numeric: tabular-nums; }
.memo-group-heading p { margin-left: auto; font-size: 12px; }
.memo-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 260px; height: 100%; padding: 32px 16px; text-align: center; }
.memo-empty > svg { width: 28px; height: 28px; color: var(--color-faint); margin-bottom: 18px; stroke-width: 1.3; }
.memo-empty h2 { font-size: 17px; font-weight: 500; color: var(--color-ink); }
.memo-empty p { font-size: 13px; color: var(--color-faint); margin-top: 8px; }
.memo-empty button { display: flex; align-items: center; gap: 6px; min-height: 32px; margin-top: 18px; color: var(--color-beacon); font-size: 12px; cursor: pointer; }
.memo-empty button svg { width: 13px; height: 13px; }
.memo-footer { display: flex; justify-content: space-between; gap: 16px; padding: 10px 0; border-top: 1px solid var(--color-hairline); font-size: 12px; color: var(--color-faint); flex-shrink: 0; font-variant-numeric: tabular-nums; }
.memo-workspace :deep(:is(button, input, select, textarea)):focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 2px; }
.memo-content:focus-visible { outline: 1px solid var(--color-hairline); outline-offset: -1px; }
@container (max-width: 680px) {
  .memo-heading { gap: 12px; padding-bottom: 18px; }
  .memo-heading h1 { font-size: 27px; }
  .memo-heading-right { gap: 10px; }
  .memo-summary strong { font-size: 21px; }
  .memo-summary i { margin: 0 3px; }
  .memo-categories { gap: 16px; }
  .memo-browse-bar { gap: 10px; }
  .memo-search { width: min(44%, 260px); }
  .memo-group-heading p, .memo-footer > span:last-child { display: none; }
}
</style>
