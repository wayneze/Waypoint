<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Check, Heart, Search, Shirt, X, ChevronLeft, ChevronRight, ArrowUpRight } from '@lucide/vue';
import { COLLECTION_KINDS } from '../../services/skyCollection';
import { collectionImageMode, collectionEntries, collectionStatus, collectionSource, collectionSearch, collectionKind, collectionCanEdit, collectionError, toggleCollectionOwned, toggleCollectionWanted } from '../../workspace/sky/collection';
import { activeSkyCharacter } from '../../workspace/vault';
import { state } from '../../workspace/core';
import { isLoadingSkySeasons, isLoadingSkyEvents } from '../../workspace/sky/tabs';
import { openExternalUrl } from '../../services/systemApi';
import ItemTile from './ItemTile.vue';
import { ensureCollectionMedia, collectionMediaLoading, collectionMediaError } from '../../services/skyCollectionMedia';

const page = ref(1);
const pageSize = 48;
const selectedKey = ref('');
const dialog = ref<HTMLDialogElement>();
const selected = computed(() => collectionEntries.value.find(e => e.key === selectedKey.value));
const statuses = computed(() => [
  { id: 'owned' as const, label: '已拥有', count: collectionEntries.value.filter(e => e.owned).length },
  { id: 'wanted' as const, label: '想要', count: collectionEntries.value.filter(e => e.wanted).length },
  { id: 'all' as const, label: '全部', count: collectionEntries.value.length },
  { id: 'missing' as const, label: '未拥有', count: collectionEntries.value.filter(e => !e.owned).length },
]);
const matching = computed(() => collectionEntries.value.filter(entry => {
  if (collectionStatus.value === 'owned' && !entry.owned) return false;
  if (collectionStatus.value === 'missing' && entry.owned) return false;
  if (collectionStatus.value === 'wanted' && !entry.wanted) return false;
  const source = collectionSource.value;
  if (source !== 'all' && source !== entry.source && source !== entry.source + ':' + entry.sourceId) return false;
  const search = collectionSearch.value.trim().toLocaleLowerCase();
  return !search || [entry.name, entry.sourceName, entry.spirit].join(' ').toLocaleLowerCase().includes(search);
}));
const kinds = computed(() => Object.entries(COLLECTION_KINDS).map(([id, label]) => ({ id, label,
  count: matching.value.filter(e => e.kind === id || (id === 'other' && !(e.kind in COLLECTION_KINDS))).length,
})).filter(k => k.count > 0 || k.id === collectionKind.value));
const filtered = computed(() => matching.value.filter(e => !collectionKind.value || e.kind === collectionKind.value
  || (collectionKind.value === 'other' && !(e.kind in COLLECTION_KINDS)))
  .sort((a, b) => (collectionStatus.value === 'owned' ? 0 : Number(a.owned) - Number(b.owned))
    || Number(!(collectionImageMode.value === 'wear' ? a.wear : a.icon)) - Number(!(collectionImageMode.value === 'wear' ? b.wear : b.icon))
    || a.name.localeCompare(b.name, 'zh-CN')));
const picturedCount = computed(() => filtered.value.filter(e => e.wear).length);
const pages = computed(() => Math.max(1, Math.ceil(filtered.value.length / pageSize)));
const visible = computed(() => filtered.value.slice((page.value - 1) * pageSize, page.value * pageSize));
const loading = computed(() => isLoadingSkySeasons.value || isLoadingSkyEvents.value);
watch(filtered, entries => { void ensureCollectionMedia(entries); }, { immediate: true });
watch([collectionStatus, collectionSource, collectionSearch, collectionKind, () => activeSkyCharacter.value?.id], () => { page.value = 1; });
watch(pages, count => { page.value = Math.min(page.value, count); });
watch(() => activeSkyCharacter.value?.id, () => { selectedKey.value = ''; });
watch(selectedKey, async key => {
  await nextTick();
  if (key && selected.value) dialog.value?.showModal(); else dialog.value?.close();
});
function resetFilters() { collectionSearch.value = ''; collectionSource.value = 'all'; collectionKind.value = ''; }
</script>

<template>
  <section class="sky-wardrobe" aria-label="外观图鉴">
    <div class="wardrobe-toolbar">
      <nav class="wardrobe-states" aria-label="收藏状态">
        <button v-for="s in statuses" :key="s.id" type="button" :aria-pressed="collectionStatus === s.id"
                :class="{ active: collectionStatus === s.id }" @click="collectionStatus = s.id">
          {{ s.label }}<span>{{ s.count }}</span>
        </button>
      </nav>
      <div class="wardrobe-tools">
        <label class="wardrobe-search"><Search :size="16" /><input v-model="collectionSearch" aria-label="搜索外观、先祖或来源" placeholder="搜索外观、先祖或来源" /></label>
        <select v-model="collectionSource" aria-label="外观来源">
          <option value="all">全部来源</option><option value="permanent">常驻先祖</option><option value="season">所有季节</option><option value="event">所有节日</option><option value="synced">其他已同步外观</option>
          <optgroup label="季节"><option v-for="g in state.seasonalGuides" :key="g.id" :value="'season:' + g.id">{{ g.season }}</option></optgroup>
          <optgroup label="节日"><option v-for="g in state.eventGuides" :key="g.id" :value="'event:' + g.id">{{ g.season }}</option></optgroup>
        </select>
      </div>
    </div>
    <nav class="wardrobe-kinds" aria-label="外观种类">
      <button type="button" :aria-pressed="!collectionKind" :class="{ active: !collectionKind }" @click="collectionKind = ''">全部种类 <small>{{ matching.length }}</small></button>
      <button v-for="k in kinds" :key="k.id" type="button" :aria-pressed="collectionKind === k.id" :class="{ active: collectionKind === k.id }" @click="collectionKind = k.id">{{ k.label }} <small>{{ k.count }}</small></button>
    </nav>
    <p v-if="collectionError" class="wardrobe-notice" role="alert">保存失败：{{ collectionError }}</p>
    <p v-if="loading" class="wardrobe-loading" role="status">正在更新季节与节日目录…</p>
    <p v-if="collectionMediaLoading" class="wardrobe-loading" role="status">正在关联实穿图片，首次载入后会保存在本地…</p>
    <p v-else-if="collectionMediaError" class="wardrobe-notice" role="status">{{ collectionMediaError }} <button type="button" @click="ensureCollectionMedia(filtered, true)">重试</button></p>
    <p v-if="collectionImageMode === 'wear' && filtered.length && !collectionMediaLoading" class="wardrobe-loading">{{ picturedCount }} 件已匹配实穿或展示图<template v-if="picturedCount < filtered.length"> · {{ filtered.length - picturedCount }} 件待匹配</template></p>
    <div v-if="visible.length" class="wardrobe-grid">
      <ItemTile v-for="entry in visible" :key="entry.key" :name="entry.name" :icon="entry.icon" :wear="entry.wear"
        :meta="entry.sourceName" :mode="collectionImageMode" :done="entry.owned" toggle :wanted="entry.wanted"
        :disabled="!collectionCanEdit" inspect @inspect="selectedKey = entry.key"
        @toggle="toggleCollectionOwned(entry)" @wish="toggleCollectionWanted(entry)" />
    </div>
    <div v-else class="wardrobe-empty">
      <Shirt :size="36" /><h3>{{ loading ? '目录正在载入' : collectionStatus === 'wanted' ? '把喜欢的外观留在这里' : '没有符合条件的外观' }}</h3>
      <p>{{ collectionStatus === 'wanted' ? '点物品旁的爱心即可加入想要；获得后也会保留。' : '自动同步与手动标记都会汇入这里，可以切换全部浏览目录。' }}</p>
      <button type="button" class="wp-game-action" @click="resetFilters(); collectionStatus = 'all'">浏览全部</button>
    </div>
    <footer class="wardrobe-footer">
      <span>{{ filtered.length }} 件外观<span v-if="collectionStatus === 'wanted'"> · 已获得的排在后面</span></span>
      <div v-if="pages > 1"><button type="button" :disabled="page === 1" aria-label="上一页外观" @click="page--"><ChevronLeft :size="17" /></button><span>{{ page }} / {{ pages }}</span><button type="button" :disabled="page === pages" aria-label="下一页外观" @click="page++"><ChevronRight :size="17" /></button></div>
    </footer>
    <dialog ref="dialog" class="wardrobe-dialog" aria-labelledby="outfit-detail-title" @close="selectedKey = ''" @click="($event.target === dialog) && (selectedKey = '')">
      <template v-if="selected">
        <div class="wardrobe-detail-heading"><span>外观详情</span><button type="button" aria-label="关闭外观详情" @click="selectedKey = ''"><X :size="20" /></button></div>
        <div class="wardrobe-detail-body">
          <ItemTile :name="selected.name" :icon="selected.icon" :wear="selected.wear" :source-url="selected.sourceUrl" @preview="selectedKey = ''" />
          <div class="wardrobe-detail-info">
            <small>{{ COLLECTION_KINDS[selected.kind] || '其他' }}</small><h2 id="outfit-detail-title">{{ selected.name }}</h2>
            <dl><div><dt>来源</dt><dd>{{ selected.sourceName }}</dd></div><div v-if="selected.spirit"><dt>先祖</dt><dd>{{ selected.spirit }}</dd></div>
              <div v-if="selected.method"><dt>获取方式</dt><dd>{{ selected.method }}</dd></div>
              <div><dt>价格</dt><dd>{{ selected.cost || '来源未提供价格' }}</dd></div><div v-if="selected.location"><dt>位置</dt><dd>{{ selected.location }}</dd></div>
              <div v-if="selected.period"><dt>所属活动档期</dt><dd>{{ selected.period }}</dd></div></dl>
            <p class="wardrobe-availability">{{ selected.source === 'permanent' ? '常驻先祖兑换' : selected.status === 'live' ? '所属季节或活动正在进行，具体获取条件以游戏内为准。' : '目录保留历史获取方式，当前是否开放请查看活动与回归。' }}</p>
            <div class="wardrobe-detail-actions"><button type="button" class="wp-game-action" :class="{ 'is-primary': selected.owned }" :disabled="!collectionCanEdit" :aria-pressed="selected.owned" @click="toggleCollectionOwned(selected)"><Check :size="16" />{{ selected.owned ? '已拥有' : '标记已拥有' }}</button>
              <button type="button" class="wp-game-action" :disabled="!collectionCanEdit" :aria-pressed="selected.wanted" @click="toggleCollectionWanted(selected)"><Heart :size="16" :fill="selected.wanted ? 'currentColor' : 'none'" />{{ selected.wanted ? '已加入想要' : '想要' }}</button></div>
            <button v-if="selected.sourceUrl" type="button" class="wp-text-action" @click="openExternalUrl(selected.sourceUrl)">查看来源 <ArrowUpRight :size="14" /></button>
            <p v-if="collectionError" role="alert">{{ collectionError }}</p>
          </div>
        </div>
      </template>
    </dialog>
  </section>
</template>

<style scoped>
.sky-wardrobe { padding-top: 4px; }
.wardrobe-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 16px; flex-wrap: wrap; }
.wardrobe-states, .wardrobe-tools, .wardrobe-kinds { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
.wardrobe-states button { padding: 9px 12px; border-radius: 7px; color: var(--color-muted); font-size: 14px; cursor: pointer; }
.wardrobe-states button.active { background: var(--color-raised); color: var(--color-ink); font-weight: 600; }
.wardrobe-states span { margin-left: 7px; font-size: 12px; font-variant-numeric: tabular-nums; color: var(--color-muted); }
.wardrobe-tools { gap: 10px; }
.wardrobe-search { display: flex; align-items: center; gap: 8px; color: var(--color-muted); border-bottom: 1px solid var(--color-hairline); padding: 8px 3px; }
.wardrobe-search input { background: transparent; width: 180px; color: var(--color-ink); font-size: 13px; }
.wardrobe-tools select { max-width: 190px; background: var(--color-paper); color: var(--color-ink); border: 1px solid var(--color-hairline); border-radius: 6px; padding: 8px; font-size: 13px; }
.wardrobe-kinds { border-bottom: 1px solid var(--color-hairline); padding: 13px 0; margin-bottom: 20px; gap: 5px; }
.wardrobe-kinds button { padding: 6px 9px; color: var(--color-muted); cursor: pointer; border-radius: 5px; font-size: 13px; }
.wardrobe-kinds button.active { color: var(--color-beacon); background: color-mix(in srgb, var(--color-beacon) 9%, transparent); }
.wardrobe-kinds small { font-size: 11px; margin-left: 3px; }
.wardrobe-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(152px, 1fr)); gap: 20px 16px; }
button:disabled { opacity: .45; cursor: not-allowed; } button:focus-visible, input:focus-visible, select:focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 3px; }
.wardrobe-footer { display: flex; justify-content: space-between; padding: 22px 0; color: var(--color-muted); font-size: 12px; }.wardrobe-footer div { display: flex; align-items: center; gap: 14px; }.wardrobe-footer button { padding: 6px; cursor: pointer; }
.wardrobe-empty { min-height: 250px; display: grid; place-content: center; justify-items: center; gap: 14px; text-align: center; color: var(--color-muted); }.wardrobe-empty h3 { color: var(--color-ink); font-size: 17px; }.wardrobe-empty p { font-size: 13px; }
.wardrobe-notice, .wardrobe-loading { font-size: 13px; color: var(--color-muted); padding: 8px 0; }.wardrobe-notice button { color: var(--color-beacon); cursor: pointer; }
.wardrobe-dialog { margin: auto; width: min(760px, calc(100vw - 36px)); max-height: calc(100dvh - 48px); padding: 22px; border-radius: 12px; border: 1px solid var(--color-hairline); background: var(--color-paper); color: var(--color-ink); overflow: auto; }
.wardrobe-dialog::backdrop { background: #0009; }.wardrobe-detail-heading { display: flex; justify-content: space-between; align-items: center; color: var(--color-muted); font-size: 13px; margin-bottom: 20px; }.wardrobe-detail-heading button { cursor: pointer; padding: 6px; }
.wardrobe-detail-body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: 30px; }.wardrobe-detail-info { min-width: 0; }.wardrobe-detail-info small { color: var(--color-muted); }.wardrobe-detail-info h2 { margin: 8px 0 24px; font-size: 24px; line-height: 1.4; }
.wardrobe-detail-info dl { font-size: 13px; }.wardrobe-detail-info dl div { margin: 16px 0; }.wardrobe-detail-info dt { color: var(--color-muted); margin-bottom: 5px; }.wardrobe-detail-info dd { line-height: 1.65; }.wardrobe-availability { color: var(--color-muted); font-size: 12px; line-height: 1.7; margin: 22px 0; }.wardrobe-detail-actions { display: flex; flex-wrap: wrap; gap: 8px; margin-bottom: 20px; }
@media(max-width: 700px) { .wardrobe-tools { width: 100%; }.wardrobe-search { flex: 1; }.wardrobe-search input { width: 100%; min-width: 80px; }.wardrobe-grid { grid-template-columns: repeat(auto-fill, minmax(128px, 1fr)); gap: 12px; }.wardrobe-detail-body { grid-template-columns: 1fr; }.wardrobe-detail-body :deep(.wp-tile) { max-width: 250px; margin: auto; } }
</style>
