<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { ChevronRight, Trash2, X } from '@lucide/vue';
import { state } from '../../workspace/core';
import { collectionImageMode, collectionEntries, collectionCanEdit, toggleCollectionOwned, toggleCollectionWanted } from '../../workspace/sky/collection';
import { handleAddCustomSeason, handleDeleteCustomSeason, newSeasonForm, showAddSeasonModal } from '../../workspace/sky/guides';
import { vWikiImg, seasonIcon } from './wikiImage';
import { isLoadingSkySeasons } from '../../workspace/sky/tabs';
import ItemTile from './ItemTile.vue';
import { ensureCollectionMedia, collectionMediaLoading, collectionMediaError } from '../../services/skyCollectionMedia';
const search = ref('');
const page = ref(1);
const expanded = ref<string[]>([]);
const customDialog = ref<HTMLDialogElement>();
const seasons = computed(() => state.seasonalGuides.filter(g => g.season.includes(search.value.trim()))
  .map(guide => {
    const entries = collectionEntries.value.filter(e => e.source === 'season' && e.sourceId === guide.id);
    const groups = [...new Set(entries.map(e => e.spirit || '季节向导'))].map(name => ({
      name, items: entries.filter(e => (e.spirit || '季节向导') === name).sort((a, b) => Number(a.owned) - Number(b.owned)),
    }));
    return { guide, entries, groups };
  })
  .sort((a, b) => Number(b.guide.status === 'live') - Number(a.guide.status === 'live') ||
    Number(a.entries.length > 0 && a.entries.every(e => e.owned)) - Number(b.entries.length > 0 && b.entries.every(e => e.owned))));
const pages = computed(() => Math.max(1, Math.ceil(seasons.value.length / 8)));
const visible = computed(() => seasons.value.slice((page.value - 1) * 8, page.value * 8));
const expandedEntries = computed(() => visible.value.filter(s => expanded.value.includes(s.guide.id)).flatMap(s => s.entries));
watch(expandedEntries, entries => { void ensureCollectionMedia(entries); }, { immediate: true });
watch(search, () => { page.value = 1; });
watch(pages, count => { page.value = Math.min(page.value, count); });
watch(showAddSeasonModal, async open => { await nextTick(); if (open) customDialog.value?.showModal(); else customDialog.value?.close(); });
function toggle(id: string) { expanded.value = expanded.value.includes(id) ? expanded.value.filter(e => e !== id) : [...expanded.value, id]; }
</script>
<template>
  <section class="season-stars" aria-label="季节先祖">
    <div class="season-tools"><input v-model="search" aria-label="搜索季节" placeholder="搜索季节" /><button type="button" class="wp-game-action" @click="showAddSeasonModal = true">自定义季节</button></div>
    <p v-if="isLoadingSkySeasons && !seasons.length" class="text-sm text-muted" role="status">正在载入季节先祖…</p>
    <p v-else-if="!seasons.length" class="text-sm text-muted">没有符合条件的季节</p>
    <p v-if="collectionMediaLoading" class="text-sm text-muted" role="status">正在关联实穿图片，首次载入后会保存在本地…</p>
    <p v-else-if="collectionMediaError" class="text-sm text-muted" role="status">{{ collectionMediaError }} <button type="button" @click="ensureCollectionMedia(expandedEntries, true)">重试</button></p>
    <article v-for="{ guide, entries, groups } in visible" :key="guide.id" class="season-row">
      <button type="button" class="season-heading" :aria-expanded="expanded.includes(guide.id)" @click="toggle(guide.id)">
        <img v-if="seasonIcon(guide)" v-wiki-img="seasonIcon(guide)" alt="" /><span class="season-title"><strong>{{ guide.season }}</strong><small>{{ guide.realm }}<template v-if="guide.status === 'live'"> · 进行中</template></small></span>
        <span class="season-count">{{ entries.filter(e => e.owned).length }} / {{ entries.length }}<small>已拥有</small></span><ChevronRight :size="16" :class="{ 'rotate-90': expanded.includes(guide.id) }" />
      </button>
      <div v-if="expanded.includes(guide.id)" class="season-content">
        <div class="season-context"><span v-if="guide.loc">{{ guide.loc }}</span><span v-if="guide.quests">{{ guide.quests }}</span></div>
        <section v-for="group in groups" :key="group.name" class="season-spirit">
          <header class="season-spirit-heading"><h4>{{ group.name }}</h4><span>{{ group.items.filter(e => e.owned).length }} / {{ group.items.length }} 已拥有</span></header>
          <div class="season-items"><ItemTile v-for="entry in group.items" :key="entry.key"
            :name="entry.name.replace(group.name + ' · ', '')" :mode="collectionImageMode" :icon="entry.icon" :wear="entry.wear" :meta="entry.cost" :source-url="entry.sourceUrl"
            :done="entry.owned" toggle :wanted="entry.wanted" :disabled="!collectionCanEdit"
            @toggle="toggleCollectionOwned(entry)" @wish="toggleCollectionWanted(entry)" /></div>
        </section>
        <button v-if="guide.isCustom" type="button" class="wp-text-action" @click="handleDeleteCustomSeason(guide.id)"><Trash2 :size="14" />移除自定义季节</button>
      </div>
    </article>
    <div v-if="pages > 1" class="season-pagination"><button type="button" :disabled="page === 1" @click="page--">上一页</button><span>{{ page }} / {{ pages }} · {{ seasons.length }} 季</span><button type="button" :disabled="page === pages" @click="page++">下一页</button></div>
      <dialog ref="customDialog" class="season-custom-dialog" aria-label="自定义季节" @close="showAddSeasonModal = false">
        <div class="w-full max-w-lg p-5 rounded-md bg-paper border border-beacon/35 space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-hairline">
            <div class="text-[16px] font-semibold text-ink">自定义季节</div>
            <button @click="showAddSeasonModal = false" class="text-faint hover:text-ink cursor-pointer p-1">
              <X class="w-4 h-4" />
            </button>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-[13px]">
            <div class="sm:col-span-2">
              <label class="text-faint block mb-1">季节名称</label>
              <input v-model="newSeasonForm.season" placeholder="如: 晨岛 · 姆明季 / 联动季" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
            </div>
            <div>
              <label class="text-faint block mb-1">所属主地图</label>
              <select v-model="newSeasonForm.realm" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
                <option v-for="r in ['晨岛', '云野', '雨林', '霞谷', '暮土', '禁阁', '云巢', '遇境']" :key="r" :value="r">{{ r }}</option>
              </select>
            </div>
            <div>
              <label class="text-faint block mb-1">推荐优先级</label>
              <select v-model="newSeasonForm.priority" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
                <option value="S级首推">S级首推 ★★★★★</option>
                <option value="A级实用">A级实用 ★★★★☆</option>
                <option value="B级按需">B级按需 ★★★☆☆</option>
              </select>
            </div>
            <div class="sm:col-span-2">
              <label class="text-faint block mb-1">常驻入口位置</label>
              <input v-model="newSeasonForm.loc" placeholder="如: 云巢商业街二层 · 筑巢工坊" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
            </div>
            <div class="sm:col-span-2">
              <label class="text-faint block mb-1">世界效益与剧情</label>
              <input v-model="newSeasonForm.benefit" placeholder="如: 永久开启风道直飞 / 私人家园公寓" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
            </div>
            <div class="sm:col-span-2">
              <label class="text-faint block mb-1">核心道具名称</label>
              <input v-model="newSeasonForm.items" placeholder="如: 吊床、野外充能火盆、企鹅面具" class="w-full bg-paper border border-hairline rounded-md px-2.5 py-1.5 text-ink focus:outline-none focus:border-beacon/35">
            </div>
            <div class="sm:col-span-2 flex items-center space-x-3 pt-1 border-t border-hairline font-mono">
              <span class="text-faint">预估消耗:</span>
              <div class="flex items-center space-x-1">
                <span class="text-muted">白蜡</span>
                <input type="number" v-model="newSeasonForm.candles" class="w-14 bg-paper border border-hairline rounded px-1.5 py-0.5 text-center text-ink">
              </div>
              <div class="flex items-center space-x-1">
                <span class="text-beacon">红蜡</span>
                <input type="number" v-model="newSeasonForm.ascended" class="w-12 bg-paper border border-hairline rounded px-1.5 py-0.5 text-center text-beacon">
              </div>
              <div class="flex items-center space-x-1">
                <span class="text-alert">爱心</span>
                <input type="number" v-model="newSeasonForm.hearts" class="w-12 bg-paper border border-hairline rounded px-1.5 py-0.5 text-center text-alert">
              </div>
            </div>
          </div>

          <div class="flex items-center justify-end space-x-2 pt-3 border-t border-hairline">
            <button @click="showAddSeasonModal = false" class="px-3.5 py-1.5 rounded-md text-faint hover:text-ink cursor-pointer text-[13px]">取消</button>
            <button @click="handleAddCustomSeason" class="px-4 py-1.5 rounded-md bg-beacon/15 hover:bg-beacon/25 text-beacon border border-beacon/35 font-medium transition cursor-pointer text-[13px]">保存</button>
          </div>
        </div>
      </dialog>
  </section>
</template>
<style scoped>
.season-context { display: flex; gap: 16px; flex-wrap: wrap; color: var(--color-muted); font-size: 13px; padding-bottom: 20px; }
.season-spirit { display: grid; grid-template-columns: 140px minmax(0, 1fr); gap: 20px; padding: 22px 0; border-top: 1px solid var(--color-hairline); }.season-spirit-heading h4 { font-size: 14px; font-weight: 600; line-height: 1.6; }.season-spirit-heading span { display: block; color: var(--color-muted); font-size: 12px; margin-top: 8px; }.season-items { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 150px), 180px)); gap: 16px; align-items: stretch; }
@media(max-width: 850px) { .season-spirit { grid-template-columns: 1fr; gap: 14px; }.season-spirit-heading { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }.season-items { grid-template-columns: repeat(auto-fill, minmax(min(100%, 140px), 1fr)); } }
.season-tools { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 16px; }.season-tools input { background: transparent; border-bottom: 1px solid var(--color-hairline); padding: 8px; font-size: 13px; max-width: 60%; }
.season-row { border-bottom: 1px solid var(--color-hairline); }.season-heading { display: flex; gap: 16px; align-items: center; width: 100%; text-align: left; padding: 17px 4px; cursor: pointer; }.season-heading img { width: 48px; height: 48px; object-fit: contain; }.season-title { flex: 1; min-width: 0; }.season-title strong { font-size: 15px; }.season-title small { display: block; font-size: 12px; color: var(--color-muted); margin-top: 5px; }.season-count { font-size: 15px; font-variant-numeric: tabular-nums; text-align: right; }.season-count small { display: block; color: var(--color-muted); font-size: 11px; margin-top: 4px; }.season-content { padding: 16px 4px; }.season-pagination { display: flex; justify-content: space-between; padding: 18px 0; font-size: 13px; color: var(--color-muted); }.season-pagination button { cursor: pointer; }.season-pagination button:disabled { opacity: .4; }
.season-custom-dialog { margin: auto; padding: 0; border: 0; border-radius: 8px; background: var(--color-paper); color: var(--color-ink); max-height: 90dvh; max-width: calc(100vw - 32px); }.season-custom-dialog::backdrop { background: #0009; }
</style>
