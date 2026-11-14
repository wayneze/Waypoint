<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { ArrowLeft, ArrowRight, BookOpen, Compass, FolderOpen, HardDriveDownload, Pin, Plus, RefreshCw, Search, Settings2, X } from '@lucide/vue';
import { isTauri } from '../services/api';
import { buildGameShelves, libraryCardTitle, type GameShelf } from '../services/gameLibrary';
import { coverPosition } from '../services/gameCoverImage';
import { posterSrc } from '../components/sky/wikiImage';
import type { ObsidianGameGuide } from '../types';
import { currentTab, gameRecords, isScanningGuides } from '../workspace/core';
import { settingsSection } from '../workspace/settings';
import { activityError, activitySummary, loadGameActivity, loadLocalPlaytime, openSteamImport, steamActivity, steamBatch } from '../workspace/gameActivity';
import { detailGame, detailGameId, detailTab, metadataJob, openGameDetail, openLibraryImport, showMetadataBatch } from '../workspace/guides/local';
import { failedPosters, filteredObsidianGuides, galgameShelf, galgameCardDensity, galgameCoverMode, toggleGalgameCardDensity, toggleGalgameCoverMode, seriesDisplayMode, isSeriesExpanded, toggleSeriesExpanded, setSeriesDisplayMode, guideCategoryFilter, guideSearchQuery, guideSortOrder, guideStats, guideStatusFilter, isGamePinned, loadObsidianGuides, openGameRecordEditor, saveGuideSortOrder, singleAndGalgameGuides, sinkCompletedGames, toggleSinkCompleted } from '../workspace/guides/library';
import { getEffectiveGameStatus } from '../workspace/guides/status';
import { showOnlyBookmarked } from '../workspace/guides/reader';
import GameDetail from '../components/GameDetail.vue';
import GameCardActions from '../components/GameCardActions.vue';
import GameBadges from '../components/GameBadges.vue';
import GameDiscovery from '../components/GameDiscovery.vue';
import SteamLibraryImport from '../components/SteamLibraryImport.vue';
import GameLibraryImport from '../components/GameLibraryImport.vue';
import GameMetadataBatch from '../components/GameMetadataBatch.vue';
import GameRemovalDialog from '../components/GameRemovalDialog.vue';
import GameSaveManager from '../components/GameSaveManager.vue';
import { showGameSaveManager, saveBatch } from '../workspace/gameSaves';
import GameLibraryFilters from '../components/GameLibraryFilters.vue';
import { filterGameLibrary, gameTaxonomyFilter, taxonomyFilterCount } from '../workspace/guides/library';
import { emptyTaxonomyFilter, type GameTaxonomySelection } from '../services/gameTaxonomy';

const activityListeners: UnlistenFn[] = [];
let disposed = false;
onMounted(async () => {
  await loadGameActivity();
  if (!isTauri() || disposed) return;
  const callbacks = await Promise.all([
    listen('game-activity-updated', () => { void loadLocalPlaytime(); }),
    listen<string>('game-activity-error', event => { activityError.value = event.payload; }),
  ]);
  if (disposed) callbacks.forEach(fn => fn()); else activityListeners.push(...callbacks);
});
onUnmounted(() => { disposed = true; activityListeners.forEach(fn => fn()); });
const list = ref<HTMLElement>();
const installedOnly = ref(false);
const installedIds = computed(() => new Set([
  ...gameRecords.value.filter(r => r.library?.installations.length).map(r => r.id),
  ...steamActivity.value.links.filter(l => steamActivity.value.installed.some(i => i.appid === l.appid)).map(l => l.record_id),
]));
const visibleGames = computed(() => filteredObsidianGuides.value.filter(g => !installedOnly.value || installedIds.value.has(g.record_id || g.id)));
const shelves = computed(() => buildGameShelves(visibleGames.value, singleAndGalgameGuides.value));
watch([guideCategoryFilter, guideSearchQuery, guideStatusFilter, showOnlyBookmarked, installedOnly], () => { if (list.value) list.value.scrollTop = 0; });
watch(gameTaxonomyFilter, () => { if (list.value) list.value.scrollTop = 0; }, { deep: true });
let returnFocus: HTMLButtonElement | null = null;
function selectGame(game: ObsidianGameGuide, event: MouseEvent) {
  returnFocus = event.currentTarget as HTMLButtonElement;
  openGameDetail(game);
}
function setupLaunch(game: ObsidianGameGuide) {
  openGameDetail(game);
  detailTab.value = 'files';
}
function shelfMetadata(shelf: GameShelf) {
  const years = [...new Set(shelf.games.map(game => game.year).filter(Boolean))].sort();
  const developers = [...new Set(shelf.games.map(game => game.developer).filter(Boolean))];
  return [years.length > 1 ? `${years[0]}–${years[years.length - 1]}` : years[0], developers.join(' / ')].filter(Boolean).join(' · ');
}
async function toggleSeries(key: string) {
  toggleSeriesExpanded(key);
  await nextTick();
  Array.from(list.value?.querySelectorAll<HTMLButtonElement>('[data-series-key]') || [])
    .find(button => button.dataset.seriesKey === key)?.focus({ preventScroll: true });
}
function completedCount(games: ObsidianGameGuide[]) {
  return games.filter(game => getEffectiveGameStatus(game) === '已通关').length;
}
async function closeDetail() {
  detailGameId.value = null;
  await nextTick();
  returnFocus?.focus({ preventScroll: true });
}
function filterFromDetail(selection: GameTaxonomySelection) { installedOnly.value = false; filterGameLibrary(selection); void closeDetail(); }
const filtered = computed(() => !!guideSearchQuery.value || guideCategoryFilter.value !== 'all' || guideStatusFilter.value !== 'all' || showOnlyBookmarked.value || installedOnly.value || taxonomyFilterCount.value > 0);
const failedCoverCount=computed(()=>visibleGames.value.filter(g=>g.poster && failedPosters.value[g.id]).length);
function clearFilters() { guideSearchQuery.value = ''; guideCategoryFilter.value = 'all'; guideStatusFilter.value = 'all'; showOnlyBookmarked.value = false; installedOnly.value = false; gameTaxonomyFilter.value = emptyTaxonomyFilter(); }
function scope(value: 'all' | 'installed' | 'playing') { installedOnly.value = value === 'installed'; guideStatusFilter.value = value === 'playing' ? '在玩' : 'all'; }
function settings() { settingsSection.value = 'connections'; currentTab.value = 'settings'; }
</script>

<template>
  <SteamLibraryImport /><GameLibraryImport /><GameMetadataBatch /><GameRemovalDialog /><GameSaveManager />
  <section class="library-workspace" aria-label="游戏书库" @keydown.esc="closeDetail">
    <div v-show="!detailGame" class="library-home">
    <header class="library-heading">
      <nav aria-label="书库页面"><button :aria-current="galgameShelf === 'library' ? 'page' : undefined" @click="galgameShelf = 'library'"><BookOpen />我的书库</button><button :aria-current="galgameShelf === 'explore' ? 'page' : undefined" @click="galgameShelf = 'explore'"><Compass />发现</button></nav>
      <div class="library-tools" aria-label="书库管理">
        <button @click="openLibraryImport()"><FolderOpen />添加本地</button>
        <button @click="openSteamImport()"><RefreshCw />Steam 同步</button>
        <button @click="showMetadataBatch = true"><Search />匹配资料</button>
        <button @click="showGameSaveManager = true"><HardDriveDownload />{{ saveBatch.running ? `存档 ${saveBatch.done}/${saveBatch.total}` : '存档管理' }}</button>
        <button @click="openGameRecordEditor()"><Plus />登记作品</button>
      </div>
    </header>
    <div v-if="metadataJob.running || metadataJob.pending || metadataJob.error || steamBatch.running || steamBatch.errors.length" class="library-jobs" role="status">
      <button v-if="metadataJob.running || metadataJob.pending || metadataJob.error" @click="showMetadataBatch = true">{{ metadataJob.running ? `资料补全 ${metadataJob.done} / ${metadataJob.total}` : metadataJob.error || `${metadataJob.pending} 项资料待确认` }}</button>
      <button v-if="steamBatch.running || steamBatch.errors.length" @click="openSteamImport()">{{ steamBatch.running ? `Steam 同步 ${steamBatch.done} / ${steamBatch.total}` : `${steamBatch.errors.length} 项同步未完成` }}</button>
    </div>
    <div v-if="galgameShelf === 'library'" class="library-browser" aria-label="封面墙">
        <div class="library-filters">
          <label class="library-search"><Search /><input v-model="guideSearchQuery" type="search" placeholder="搜索书库" aria-label="搜索游戏" /><button v-if="guideSearchQuery" aria-label="清空搜索" @click="guideSearchQuery = ''"><X /></button></label>
          <nav class="library-scopes" aria-label="快捷筛选"><button :aria-pressed="!installedOnly && guideStatusFilter === 'all'" @click="scope('all')">全部</button><button :aria-pressed="installedOnly" @click="scope('installed')">已安装</button><button :aria-pressed="guideStatusFilter === '在玩'" @click="scope('playing')">在玩</button></nav>
          <span class="library-total">{{ visibleGames.length }} 部作品</span>
          <div class="library-selects">
            <select v-model="guideCategoryFilter" aria-label="作品分类"><option value="all">全部分类</option><option value="pinned">置顶作品</option><option value="galgame">Galgame</option><option value="single">单机游戏</option><option value="unmatched">待匹配资料</option></select>
            <select v-model="guideSortOrder" aria-label="作品排序" @change="saveGuideSortOrder"><option value="year_desc">最新发行</option><option value="year_asc">最早发行</option><option value="rating_desc">评分优先</option><option value="recent">最近游玩 / 阅读</option><option value="modified_desc">最近修改</option><option value="title_asc">标题顺序</option></select>
            <select v-model="guideStatusFilter" aria-label="游玩状态"><option value="all">全部状态</option><option value="在玩">在玩中</option><option value="已通关">已通关</option><option value="未开始">未开始</option><option value="已收藏">已收藏</option></select>
            <label class="library-check"><input v-model="showOnlyBookmarked" type="checkbox" />有书签</label>
          </div>
          <div class="library-count"><label><input type="checkbox" :checked="sinkCompletedGames" @change="toggleSinkCompleted" />通关沉底</label><button v-if="filtered" @click="clearFilters">重置</button></div>

        </div>
        <GameLibraryFilters />
        <div ref="list" class="library-list" aria-label="游戏封面墙">
          <div class="library-wall" :class="{'is-compact': galgameCardDensity === 'compact'}">
          <template v-for="shelf in shelves" :key="shelf.key">
            <article v-if="shelf.series && !isSeriesExpanded(shelf.key)" class="library-series-closed" :aria-label="shelf.series">
              <div class="library-series-cover" :style="{'--layers': Math.min(shelf.games.length,6) - 1}">
              <button class="library-book-stack" :aria-label="'向右展开' + shelf.series + '，' + shelf.games.length + '部作品'" :aria-expanded="false" @click="toggleSeries(shelf.key)">
                <span v-for="(game,index) in shelf.games.slice(0,6)" :key="game.id" class="library-stack-volume" :class="{'is-front': index === 0}" :style="{'--layer': index, zIndex:6-index}">
                  <img v-if="galgameCoverMode === 'poster' && game.poster && !failedPosters[game.id]" :src="posterSrc(game.poster)" :style="{objectPosition:coverPosition(game.poster_position)}" alt="" loading="lazy" referrerpolicy="no-referrer" @error="failedPosters[game.id] = true" />
                  <span v-else class="library-stack-type">{{ game.title }}</span><span v-if="index > 0" class="library-stack-number">{{ game.series_order ?? index+1 }}</span>
                </span>
              </button>
              <ul class="library-series-members" :aria-label="shelf.series + '作品列表'">
                <li v-for="game in shelf.games" :key="game.id">
                  <button :aria-label="'选择作品：' + game.title" :title="game.title" @click="selectGame(game, $event)"><span>{{ String(game.series_order ?? '·').padStart(2,'0') }}</span><span class="library-member-title">{{ libraryCardTitle(game, true) }}</span></button>
                </li>
              </ul>
              </div>
              <div class="library-card-heading">
                <header class="library-series-title"><h2 :title="shelf.series">{{ shelf.series }}</h2><button class="library-series-toggle" :data-series-key="shelf.key" :aria-label="'展开' + shelf.series" :aria-expanded="false" @click="toggleSeries(shelf.key)">展开<ArrowRight /></button></header>
                <p class="library-card-meta" :title="shelfMetadata(shelf)">{{ shelfMetadata(shelf) }}</p>
              </div>
              <div class="library-game-copy">
                <div class="library-game-status"><span>{{ shelf.games.length < shelf.total ? `显示 ${shelf.games.length} / ${shelf.total}` : `共 ${shelf.total}` }} 部 · 已通关 {{ completedCount(shelf.games) }}</span></div>
                <GameCardActions :games="shelf.games" @setup="setupLaunch" />
              </div>
            </article>
            <template v-else>
              <article v-for="game in shelf.games" :key="game.id" class="library-game" :aria-label="game.title">
                <button class="library-cover" :aria-label="'选择作品：' + game.title" @click="selectGame(game, $event)">
                  <img v-if="galgameCoverMode === 'poster' && game.poster && !failedPosters[game.id]" :src="posterSrc(game.poster)" :style="{ objectPosition: coverPosition(game.poster_position) }" alt="" loading="lazy" referrerpolicy="no-referrer" @error="failedPosters[game.id] = true" /><span v-else class="library-monogram">{{ game.title }}</span>
                </button>
                <div class="library-card-heading">
                  <div class="library-series-title"><button class="library-title" :title="game.title" @click="selectGame(game, $event)"><Pin v-if="isGamePinned(game)" />{{ libraryCardTitle(game, !!shelf.series) }}</button><button v-if="shelf.series" class="library-series-toggle" :data-series-key="shelf.key" :aria-label="'收起' + shelf.series" :title="shelf.series" :aria-expanded="true" @click="toggleSeries(shelf.key)">收起<ArrowLeft /></button></div>
                  <p class="library-card-meta" :title="[game.year, game.developer].filter(Boolean).join(' · ')">{{ [game.year, game.developer].filter(Boolean).join(' · ') }}</p>
                </div>
                <div class="library-game-copy">
                  <div class="library-game-status"><span>{{ getEffectiveGameStatus(game) }}</span><GameBadges :record-id="game.record_id" /></div>
                  <GameCardActions :games="[game]" @setup="setupLaunch" />
                  <p v-if="activitySummary(game.record_id || game.id)" class="library-activity">{{ activitySummary(game.record_id || game.id) }}</p>
                </div>
              </article>
            </template>
          </template>
          <div v-if="!visibleGames.length" class="library-empty"><BookOpen /><p>{{ guideStats.total ? '没有符合条件的作品' : '书库还没有作品' }}</p><button v-if="filtered" @click="clearFilters">清除筛选</button><button v-else @click="openGameRecordEditor()">登记第一部作品</button></div>
          </div>
        </div>
        <footer class="library-footer">          <div class="library-display" aria-label="书架显示">
            <div role="group" aria-label="合集显示"><button :aria-pressed="seriesDisplayMode === 'collapsed'" @click="setSeriesDisplayMode('collapsed')">合集折叠</button><button :aria-pressed="seriesDisplayMode === 'expanded'" @click="setSeriesDisplayMode('expanded')">全部展开</button></div>
            <div role="group" aria-label="封面大小"><button :aria-pressed="galgameCardDensity === 'large'" @click="galgameCardDensity !== 'large' && toggleGalgameCardDensity()">舒展</button><button :aria-pressed="galgameCardDensity === 'compact'" @click="galgameCardDensity !== 'compact' && toggleGalgameCardDensity()">紧凑</button></div>
            <button :aria-pressed="galgameCoverMode === 'badge'" @click="toggleGalgameCoverMode">{{ galgameCoverMode === 'poster' ? '文字书封' : '海报封面' }}</button>
          </div><button v-if="failedCoverCount" @click="failedPosters={}"><RefreshCw />重新加载 {{ failedCoverCount }} 张封面</button><button :disabled="isScanningGuides" @click="loadObsidianGuides(true)"><RefreshCw />{{ isScanningGuides ? '同步中…' : '同步攻略' }}</button><button @click="settings"><Settings2 />连接设置</button></footer>

    </div>
    <div v-else class="library-discover"><GameDiscovery /></div>
    </div>
    <div v-if="detailGame" class="library-detail">
      <GameDetail @back="closeDetail" @filter="filterFromDetail" />
    </div>
  </section>
</template>

<style scoped>
.library-workspace,.library-home,.library-detail{display:flex;flex-direction:column;width:100%;height:100%;min-height:0;min-width:0;color:var(--color-ink)}.library-workspace{container-type:inline-size}
.library-heading{display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap;padding:3px 0 12px;border-bottom:1px solid var(--color-hairline);flex-shrink:0}.library-heading nav,.library-tools{display:flex;gap:4px;align-items:center;flex-wrap:wrap}button{cursor:pointer;display:inline-flex;align-items:center;justify-content:center;gap:6px;color:var(--color-muted);font-size:12px;min-height:32px;padding:6px 9px;border-radius:4px}button:hover{color:var(--color-ink);background:var(--color-paper)}button:disabled{opacity:.4;cursor:default}svg{width:14px;height:14px;flex-shrink:0}.library-heading nav button{font-size:14px;padding:7px 12px}.library-heading [aria-current]{color:var(--color-beacon);background:var(--color-paper)}.library-tools button{padding-inline:8px}.library-jobs{display:flex;flex-wrap:wrap;gap:12px;padding:4px 12px;background:var(--color-paper);flex-shrink:0}.library-jobs button{color:var(--color-beacon);font-size:11px}
.library-browser{display:flex;flex-direction:column;flex:1;min-height:0;min-width:0}.library-filters{display:flex;align-items:center;flex-wrap:wrap;gap:10px 18px;padding:16px 0;flex-shrink:0}.library-search{display:flex;align-items:center;gap:8px;background:var(--color-paper);padding:0 9px;border:1px solid var(--color-hairline);border-radius:4px;flex:1;min-width:160px;max-width:360px}.library-search input{width:100%;min-width:0;background:none;font-size:13px;padding:8px 0;outline:none}.library-search button{padding:0;min-height:28px}.library-scopes{display:flex;gap:6px}.library-scopes button{font-size:12px;padding-inline:12px}.library-scopes [aria-pressed=true]{background:var(--color-paper);color:var(--color-beacon)}.library-total{margin-left:auto;color:var(--color-faint);font-size:11px}.library-selects{display:flex;gap:12px;align-items:center;flex-wrap:wrap;flex-basis:calc(100% - 180px)}.library-selects select{min-width:0;color:var(--color-muted);font-size:11px;border:0;background:var(--color-canvas);padding:4px 0;max-width:160px}.library-check,.library-count label{display:flex;align-items:center;gap:5px;color:var(--color-muted);font-size:11px}.library-count{display:flex;align-items:center;gap:12px;margin-left:auto;font-size:11px;color:var(--color-faint)}.library-count button{font-size:11px;min-height:20px;padding:0;color:var(--color-beacon)}
.library-list{flex:1;min-height:0;overflow-y:auto;scrollbar-gutter:stable;padding:4px 5px 24px}.library-wall{display:grid;grid-template-columns:repeat(auto-fill,minmax(min(100%,200px),240px));align-items:start;gap:24px}.library-wall.is-compact{grid-template-columns:repeat(auto-fill,minmax(min(100%,130px),150px));gap:24px 20px}.library-game,.library-series-closed{display:flex;flex-direction:column;min-width:0;text-align:left}
.library-cover{display:block;align-self:start;width:100%;aspect-ratio:3/4;padding:0;overflow:hidden;background:transparent;border-radius:3px;flex:none}.library-cover img{display:block;width:100%;height:100%;object-fit:cover;transition:filter .18s ease}.library-cover:hover img{filter:brightness(1.08)}.library-monogram{display:flex;align-items:center;justify-content:center;aspect-ratio:3/4;padding:20px;font:24px/1.6 var(--font-display);color:var(--color-beacon);background:var(--color-paper);border:1px solid var(--color-hairline)}
.library-card-heading{padding:8px 0}.library-title{display:block;flex:1;min-width:0;white-space:nowrap;text-overflow:ellipsis;overflow:hidden;padding:0;min-height:24px;text-align:left;color:var(--color-ink);font-size:14px;font-weight:500;line-height:24px}.library-title:hover{background:transparent;color:var(--color-beacon)}.library-title svg{display:inline;width:11px;height:11px;color:var(--color-beacon);margin-right:5px}.library-card-meta{font-size:11px;min-height:18px;line-height:18px;color:var(--color-faint);margin-top:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.library-game-copy{display:flex;flex-direction:column;gap:4px;min-width:0;border-top:1px solid var(--color-hairline);padding-top:6px}.library-game-status{display:flex;align-items:center;flex-wrap:wrap;gap:5px 10px;color:var(--color-faint);font-size:11px;line-height:20px}.library-game-status :deep(.game-badges){gap:8px;font-size:11px}.library-activity{font-size:11px;line-height:18px;color:var(--color-muted)}
.library-footer{flex-wrap:wrap;border-top:1px solid var(--color-hairline);display:flex;align-items:center;justify-content:end;gap:8px;padding:5px 0;flex-shrink:0}.library-footer button{font-size:11px}.library-empty{grid-column:1/-1;display:flex;align-items:center;justify-content:center;flex-direction:column;gap:12px;text-align:center;padding:50px 14px;color:var(--color-muted);font-size:13px}.library-empty>svg{width:28px;height:28px;color:var(--color-faint)}.library-empty button{color:var(--color-beacon)}.library-discover{overflow:auto;flex:1;min-height:0;padding-top:16px}.discovery-back{margin-bottom:12px}.library-detail :deep(.game-detail){flex:1}:where(button,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}
@container(max-width:760px){.library-wall:not(.is-compact){grid-template-columns:repeat(auto-fill,minmax(min(100%,180px),210px))}.library-tools button{font-size:11px;padding-inline:6px}.library-heading{gap:6px;padding-bottom:8px}.library-heading nav button{font-size:13px}.library-filters{gap:8px 12px;padding:12px 0}.library-selects{gap:8px;flex-basis:auto}.library-scopes button{padding-inline:8px}}

.library-display{display:flex;align-items:center;gap:10px;flex-wrap:wrap;margin-right:auto}.library-display>div{display:flex;gap:2px}.library-display button{font-size:11px;min-height:26px;padding:3px 8px}.library-display [aria-pressed=true]{color:var(--color-beacon);background:var(--color-paper)}
.library-series-title{display:flex;align-items:center;justify-content:space-between;gap:6px}.library-series-title h2{font-size:14px;line-height:24px;font-weight:500;color:var(--color-ink);min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.library-series-toggle{flex:none;padding:2px 4px;min-height:24px;font-size:10px;gap:3px;color:var(--color-faint)}.library-series-toggle svg{width:11px;height:11px}.library-series-toggle:hover{color:var(--color-beacon)}
.library-book-stack{position:relative;display:block;width:100%;aspect-ratio:3/4;padding:0;text-align:left;isolation:isolate;flex:none;background:transparent!important}.library-stack-volume{position:absolute;top:calc(var(--layer)*2px);bottom:calc(var(--layer)*2px);left:calc(var(--layer)*10px);width:calc(100% - var(--layers)*10px);overflow:hidden;border-radius:2px 5px 5px 2px;background:var(--color-paper);box-shadow:2px 1px 6px #0008,inset -1px 0 #fff3;transform-origin:left center;transition:transform .22s ease}.library-stack-volume img{display:block;width:100%;height:100%;object-fit:cover}.library-stack-volume.is-front{height:100%}.library-stack-volume::after{content:'';position:absolute;inset:0;pointer-events:none;background:linear-gradient(90deg,#0004,transparent 7%,#ffffff0a 8%,transparent 12%,transparent 94%,#0006);border:1px solid #ffffff12;border-radius:inherit}.library-stack-volume:not(.is-front)::after{background:linear-gradient(90deg,transparent 75%,#0005 93%,#ffffff30 96%,#0008)}.library-stack-number{position:absolute;bottom:14px;right:0;width:10px;text-align:center;color:#fff;font:9px var(--font-mono);text-shadow:0 1px 3px #000}.library-stack-type{display:flex;align-items:center;height:100%;padding:20px;font:22px/1.6 var(--font-display);color:var(--color-beacon)}.library-book-stack:hover .library-stack-volume,.library-book-stack:focus-visible .library-stack-volume{transform:translateX(calc(var(--layer)*2px))}.library-display button{white-space:nowrap}.library-series-cover{position:relative;isolation:isolate;width:100%}
.library-stack-type{height:100%}
.library-series-members{position:absolute;z-index:7;max-height:75%;overflow:auto;bottom:0;left:0;width:calc(100% - var(--layers)*10px);margin:0;padding:16px 8px 6px;background:linear-gradient(transparent,rgba(17,20,18,.25) 18%,rgba(17,20,18,.48));border:0;border-radius:0 0 3px 3px;list-style:none}
.library-series-members li{margin:0;padding:0}
.library-series-members button{display:flex;align-items:center;justify-content:start;gap:6px;width:100%;height:20px;min-height:20px;padding:0;text-align:left;font-size:10.5px;line-height:20px;color:rgba(255,255,255,.88);border:0;border-radius:0;background:transparent;text-shadow:0 1px 3px #000b}
.library-series-members button>span:first-child{font:9px var(--font-mono);color:var(--color-beacon);flex:none}
.library-member-title{white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0}
.library-series-members button:hover,.library-series-members button:focus-visible{color:#fff;background:#ffffff12}
</style>
