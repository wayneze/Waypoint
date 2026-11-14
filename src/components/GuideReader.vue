<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import {
  clearGuideProgress, currentGuideRouteFocus, currentGuideRouteList, guideHasRoutes,
  guideSectionLabel, drawerWidth, GUIDE_ROUTE_FOCUS_OVERVIEW, getTableRouteColIdx,
  getTableStatusColIdx, guideProgressMap, handleOpenInObsidian, handleOpenGuideReader,
  isGuideRouteFocused, isGuideReaderMinimized, isLoadingGuideContent, isRouteCompleted,
  isTableRowRouteDone, jumpToBookmarkedProgress, minimizeGuideReader, parsedCurrentGuide,
  readerLayoutMode, restoreGuideReader, selectGuideRoute, selectedGuide, setGuideProgress,
  showGuideReader, startDrawerResize, toggleReaderLayoutMode, toggleRouteCompleted, guideReadError,
} from '../workspace/guides/reader';
import { formatGuideInline } from '../services/guideParser';
import { failedInlineImages, isGamePinned, togglePinGame, openGameRecordEditor } from '../workspace/guides/library';
import { getEffectiveGameStatus, setGameStatus } from '../workspace/guides/status';
import { getInlineImageTheme, openLightbox, toggleInlineImageTheme, lightboxState } from '../workspace/guides/lightbox';
import GuideLine from './GuideLine.vue';
import { openResourceWeb } from '../services/systemApi';
import GameLinks from './GameLinks.vue';
import { ArrowLeft, ArrowRight, BookOpen, Bookmark, Check, Circle, ExternalLink, List, Maximize2, Minimize2, PanelRight, Pencil, Pin, RotateCw, Search, X } from '@lucide/vue';

const panel = ref<HTMLElement | null>(null);
const contentsOpen = ref(false);
const contentsQuery = ref('');
const chapterIndex = ref(-1);
const bookmark = computed(() => selectedGuide.value ? guideProgressMap.value[selectedGuide.value.id] : undefined);
const chapters = computed(() => parsedCurrentGuide.value.blocks.flatMap((block, blockIdx) =>
  block.type === 'heading' && block.text && !(block.level === 1 && block.text === parsedCurrentGuide.value.title)
    ? [{ title: block.text, blockIdx, section: undefined }] : []));
const contents = computed(() => (guideHasRoutes.value ? currentGuideRouteList.value : chapters.value)
  .filter(item => item.title.toLocaleLowerCase().includes(contentsQuery.value.trim().toLocaleLowerCase())));
const currentRouteIndex = computed(() => currentGuideRouteList.value.findIndex(route => isGuideRouteFocused(route.title)));
const currentRoute = computed(() => currentGuideRouteList.value[currentRouteIndex.value]);
const completedCount = computed(() => selectedGuide.value ? currentGuideRouteList.value.filter(route => isRouteCompleted(selectedGuide.value!.id, route.title, route.section)).length : 0);
watch(() => selectedGuide.value?.id, () => { contentsQuery.value = ''; contentsOpen.value = false; chapterIndex.value = -1; });
watch([showGuideReader, isGuideReaderMinimized, readerLayoutMode], async () => {
  if (showGuideReader.value && !isGuideReaderMinimized.value) { await nextTick(); panel.value?.focus(); }
}, { immediate: true });
watch([isLoadingGuideContent, showGuideReader, () => selectedGuide.value?.id], () => {
  if (showGuideReader.value && !isLoadingGuideContent.value && !guideReadError.value && selectedGuide.value) {
    jumpToBookmarkedProgress(selectedGuide.value.id);
  }
}, { flush: 'post', immediate: true });
function navigate(title: string, blockIdx = -1) {
  contentsOpen.value = false;
  if (guideHasRoutes.value || blockIdx === -1) { selectGuideRoute(title); chapterIndex.value = -1; }
  else { chapterIndex.value = blockIdx; document.getElementById(`guide-block-${blockIdx}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' }); }
}
function resume() {
  if (!selectedGuide.value) return;
  contentsOpen.value = false;
  jumpToBookmarkedProgress(selectedGuide.value.id);
}
function followResourceLink(event: MouseEvent) {
  if (!selectedGuide.value?.resource_url) return;
  const anchor = (event.target as HTMLElement).closest('a');
  if (!anchor) return;
  event.preventDefault();
  const url = anchor.getAttribute('href') || '';
  if (/^https?:\/\//i.test(url)) void openResourceWeb(url, selectedGuide.value.title, selectedGuide.value.resource_game_id || '', selectedGuide.value.resource_url).catch(error => guideReadError.value = String(error));
}
function onKeydown(event: KeyboardEvent) {
  if (lightboxState.isOpen) return;
  if (event.key === 'Escape') { event.stopPropagation(); if (contentsOpen.value) contentsOpen.value = false; else showGuideReader.value = false; }
  if (event.key !== 'Tab' || readerLayoutMode.value !== 'modal') return;
  const focusable = [...panel.value!.querySelectorAll<HTMLElement>('button:not(:disabled), input, select, [tabindex="0"]')].filter(el => el.getClientRects().length);
  const first = focusable[0], last = focusable[focusable.length - 1];
  if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.value)) { event.preventDefault(); last?.focus(); }
  else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
}
</script>

<template>
  <div v-if="showGuideReader && isGuideReaderMinimized && selectedGuide" class="reader-dock">
    <button type="button" class="reader-dock-restore" @click="restoreGuideReader"><BookOpen /><span><strong>{{ selectedGuide.title }}</strong><small>{{ bookmark ? '继续 · ' + bookmark.routeTitle : '继续阅读' }}</small></span><Maximize2 /></button>
    <button type="button" aria-label="关闭攻略" @click="showGuideReader = false"><X /></button>
  </div>
  <div v-if="showGuideReader && !isGuideReaderMinimized && selectedGuide" class="reader-layer" :class="'is-' + readerLayoutMode" @click.self="readerLayoutMode === 'modal' && (showGuideReader = false)">
    <section ref="panel" class="reader-panel" :style="readerLayoutMode === 'sidecar' ? { width: `${drawerWidth}px` } : {}" role="dialog" :aria-modal="readerLayoutMode === 'modal' ? true : undefined" aria-labelledby="reader-title" tabindex="-1" @keydown="onKeydown">
      <div v-if="readerLayoutMode === 'sidecar'" class="reader-resize" @mousedown.prevent="startDrawerResize" title="拖动边缘调节阅读宽度"><span></span></div>
      <header class="reader-topbar">
        <div class="reader-identity"><BookOpen /><div><p>{{ (selectedGuide.file_path || selectedGuide.resource_url) ? '攻略阅读' : '游戏档案' }}</p><h1 id="reader-title" :title="selectedGuide.title">{{ selectedGuide.title }}</h1></div></div>
        <div class="reader-window-actions">
          <button type="button" @click="toggleReaderLayoutMode" :title="readerLayoutMode === 'sidecar' ? '沉浸阅读' : '靠右侧边'" :aria-label="readerLayoutMode === 'sidecar' ? '沉浸阅读' : '靠右侧边'"><Maximize2 v-if="readerLayoutMode === 'sidecar'" /><PanelRight v-else /></button>
          <button type="button" title="收起，稍后继续" aria-label="收起攻略" @click="minimizeGuideReader"><Minimize2 /></button>
          <button type="button" title="关闭 · Esc" aria-label="关闭攻略" @click="showGuideReader = false"><X /></button>
        </div>
      </header>
      <div class="reader-toolbar">
        <button type="button" class="reader-contents-toggle" :aria-expanded="contentsOpen" aria-controls="reader-contents" @click="contentsOpen = !contentsOpen"><List />目录</button>
        <label v-if="!selectedGuide.resource_url" class="reader-status"><span class="sr-only">游玩状态</span><select :value="getEffectiveGameStatus(selectedGuide) === '游玩中' ? '在玩' : getEffectiveGameStatus(selectedGuide)" @change="setGameStatus(selectedGuide, ($event.target as HTMLSelectElement).value)"><option value="未开始">未开始</option><option value="在玩">在玩中</option><option value="已通关">已通关</option><option value="已收藏">已收藏</option></select></label>
        <button v-if="!selectedGuide.resource_url" type="button" :aria-pressed="isGamePinned(selectedGuide)" :title="isGamePinned(selectedGuide) ? '取消置顶' : '置顶游戏'" @click="togglePinGame(selectedGuide)"><Pin />置顶</button>
        <span class="reader-toolbar-spacer"></span>
        <button v-if="!selectedGuide.resource_url" type="button" @click="openGameRecordEditor(selectedGuide)"><Pencil />编辑作品</button>
        <button v-if="bookmark" type="button" class="reader-resume" @click="resume"><Bookmark />回到书签</button>
        <button v-if="selectedGuide.file_path" type="button" class="reader-external" title="在 Obsidian 中打开" @click="handleOpenInObsidian(selectedGuide)"><ExternalLink /><span>Obsidian</span></button>
      </div>
      <div class="reader-body">
        <button v-if="contentsOpen" class="reader-nav-scrim" aria-label="收起目录" @click="contentsOpen = false"></button>
        <aside id="reader-contents" class="reader-contents" :class="{ 'is-open': contentsOpen }" aria-label="攻略目录">
          <div class="reader-contents-heading"><span>{{ guideHasRoutes ? '路线目录' : '章节目录' }}</span><small>{{ guideHasRoutes ? `${completedCount} / ${currentGuideRouteList.length}` : chapters.length + ' 章' }}</small></div>
          <div v-if="guideHasRoutes" class="reader-route-progress"><span :style="{ width: `${completedCount / currentGuideRouteList.length * 100}%` }"></span></div>
          <label v-if="contents.length > 6 || contentsQuery" class="reader-contents-search"><Search /><input v-model="contentsQuery" aria-label="筛选攻略目录" placeholder="找路线或章节" type="search" /></label>
          <nav>
            <button type="button" :aria-current="currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW && chapterIndex === -1 ? 'page' : undefined" @click="navigate(GUIDE_ROUTE_FOCUS_OVERVIEW)"><BookOpen /><span>阅读概览</span></button>
            <button v-for="(item, index) in contents" :key="item.blockIdx" type="button" :aria-current="(guideHasRoutes ? isGuideRouteFocused(item.title) : chapterIndex === item.blockIdx) ? 'page' : undefined" :class="{ 'is-complete': guideHasRoutes && isRouteCompleted(selectedGuide.id, item.title, item.section) }" @click="navigate(item.title, item.blockIdx)"><Check v-if="guideHasRoutes && isRouteCompleted(selectedGuide.id, item.title, item.section)" /><small v-else>{{ String(index + 1).padStart(2, '0') }}</small><span>{{ item.title }}</span></button>
            <p v-if="contentsQuery && !contents.length" class="reader-empty-contents">没有匹配的目录项</p>
          </nav>
          <div v-if="bookmark" class="reader-bookmark"><p><Bookmark />上次读到<button type="button" title="清除书签" aria-label="清除书签" @click="clearGuideProgress(selectedGuide.id)"><X /></button></p><button type="button" @click="resume"><strong>{{ bookmark.routeTitle }}</strong><span>{{ bookmark.stepText || '继续这条路线' }}</span><small>{{ bookmark.updatedAt }}</small></button></div>
        </aside>
        <div id="guide-reader-scroll" class="reader-scroll" @click="followResourceLink">
          <div v-if="isLoadingGuideContent" class="reader-message" role="status"><RotateCw class="animate-spin" /><h2>正在翻开攻略…</h2></div>
          <div v-else-if="guideReadError" class="reader-message" role="alert"><BookOpen /><h2>暂时无法打开攻略</h2><p>{{ guideReadError }}</p><button type="button" @click="handleOpenGuideReader(selectedGuide)">重新读取</button></div>
          <article v-else class="reader-article wp-guide-prose">
            <header class="reader-article-heading">
              <p>{{ (selectedGuide.file_path || selectedGuide.resource_url) ? '游戏攻略' : '游玩记录' }}<span v-if="selectedGuide.year"> / {{ selectedGuide.year }}</span></p>
              <h2>{{ currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW ? selectedGuide.title : currentRoute?.title }}</h2>
              <div v-if="guideHasRoutes">{{ currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW ? '阅读须知与路线总览' : '点击正文中的选项，记下读到的位置' }}</div>
            </header>
            <section v-if="selectedGuide.file_path && selectedGuide.record_id" class="reader-personal">
              <div><span>我的游玩档案</span><button v-if="!selectedGuide.resource_url" type="button" @click="openGameRecordEditor(selectedGuide, 'progress')">更新进度<Pencil /></button></div>
              <p v-if="selectedGuide.progress_note">{{ selectedGuide.progress_note }}</p>
              <p v-if="selectedGuide.personal_notes" class="reader-personal-note">{{ selectedGuide.personal_notes }}</p>
              <small>{{ selectedGuide.rating_personal ? '个人评分 ' + selectedGuide.rating_personal.toFixed(1) : '尚未评分' }}</small>
            </section>
            <GameLinks v-if="!selectedGuide.resource_url" :key="selectedGuide.id" :game-id="selectedGuide.id" :record-id="selectedGuide.record_id" class="reader-game-links" />
            <template v-if="currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW">
              <p v-if="parsedCurrentGuide.overview" class="wp-guide-note mb-6">{{ parsedCurrentGuide.overview }}</p>
              <p v-else-if="guideHasRoutes" class="text-[13px] text-muted mb-6">
                从目录选择一条路线，开始阅读。
              </p>
            </template>

            <template v-for="(block, bIdx) in parsedCurrentGuide.blocks" :key="bIdx">
              <div v-if="block.type === 'callout' && block.callout && isGuideRouteFocused(block.callout.title)"
                   :id="`guide-block-${bIdx}`"
                   class="mb-5 last:mb-0">
              <div class="reader-route">
                <div class="reader-route-status">
                  <span>{{ block.callout.sectionHeading || '路线攻略' }}</span>
                  <button type="button" @click="toggleRouteCompleted(selectedGuide, block.callout.sectionHeading || block.callout.title, block.callout.title)" :class="{ 'is-complete': isRouteCompleted(selectedGuide.id, block.callout.title, block.callout.sectionHeading) }">
                    <Check v-if="isRouteCompleted(selectedGuide.id, block.callout.title, block.callout.sectionHeading)" /><Circle v-else />
                    {{ isRouteCompleted(selectedGuide.id, block.callout.title, block.callout.sectionHeading) ? '已通关 · 点击撤销' : '标记这条路线通关' }}
                  </button>
                </div>

                <div class="wp-guide-route-body">
                  <GuideLine
                    v-for="(pLine, cIdx) in (block.callout.parsedLines || [])"
                    :id="`guide-line-${bIdx}-${cIdx}`"
                    :key="cIdx"
                    :line="pLine"
                    markable
                    :here="guideProgressMap[selectedGuide.id]?.blockIdx === bIdx && guideProgressMap[selectedGuide.id]?.lineIdx === cIdx"
                    :image-theme="getInlineImageTheme(pLine.imageUrl || '', pLine.imageAlt)"
                    :image-failed="!!failedInlineImages[pLine.imageUrl || '']"
                    @mark="setGuideProgress(selectedGuide.id, block.callout.title, bIdx, cIdx, pLine.raw)"
                    @open-image="openLightbox"
                    @toggle-theme="toggleInlineImageTheme"
                    @image-error="failedInlineImages[pLine.imageUrl || ''] = true"
                  />
                </div>
              </div>
              </div>

              <div v-else-if="currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW && block.type === 'table' && block.table"
                   :id="`guide-block-${bIdx}`"
                   class="reader-table overflow-x-auto my-5">
                <table class="w-full text-left text-[13px] border-collapse">
                  <thead>
                    <tr class="bg-raised border-b border-hairline text-muted">
                      <th v-for="(th, thIdx) in block.table.headers" :key="thIdx" class="px-3 py-2 font-medium">
                        <span v-html="formatGuideInline(th)"></span>
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(row, rIdx) in block.table.rows" :key="rIdx"
                        class="border-b border-hairline/70"
                        :class="getTableStatusColIdx(block.table.headers) !== -1 && isTableRowRouteDone(block.table.headers, row, selectedGuide.id) ? 'text-done' : ''">
                      <td v-for="(cell, cIdx) in row" :key="cIdx" class="px-3 py-2">
                        <template v-if="cIdx === getTableStatusColIdx(block.table.headers)">
                          <button type="button" @click.stop="toggleRouteCompleted(selectedGuide, row[getTableRouteColIdx(block.table.headers)] || cell)"
                                  class="text-[13px] cursor-pointer"
                                  :class="isTableRowRouteDone(block.table.headers, row, selectedGuide.id) ? 'text-done' : 'text-faint hover:text-ink'">
                            {{ isTableRowRouteDone(block.table.headers, row, selectedGuide.id) ? '已通关' : '未通关' }}
                          </button>
                        </template>
                        <template v-else-if="cIdx === getTableRouteColIdx(block.table.headers)">
                          <span v-html="formatGuideInline(cell)"></span>
                        </template>
                        <template v-else>
                          <span class="text-muted" v-html="formatGuideInline(cell)"></span>
                        </template>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-else-if="(!guideHasRoutes || currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW) && block.type === 'lines'" :id="`guide-block-${bIdx}`" class="mb-2">
                <GuideLine
                  v-for="(pLine, cIdx) in (block.lines || [])"
                  :key="cIdx"
                  :line="pLine"
                  :image-theme="getInlineImageTheme(pLine.imageUrl || '', pLine.imageAlt)"
                  :image-failed="!!failedInlineImages[pLine.imageUrl || '']"
                  @open-image="openLightbox"
                  @toggle-theme="toggleInlineImageTheme"
                  @image-error="failedInlineImages[pLine.imageUrl || ''] = true"
                />
              </div>

              <header v-else-if="!guideHasRoutes && block.type === 'heading' && block.text && !(block.level === 1 && block.text === parsedCurrentGuide.title)"
                      :id="`guide-block-${bIdx}`"
                      class="wp-guide-chapter"
                      :class="block.level === 1 ? 'is-h1' : (block.level && block.level >= 3 ? 'is-h3' : '')">
                <div class="wp-guide-kicker">{{ guideSectionLabel(block.level || 2) }}</div>
                <h3>{{ block.text }}</h3>
              </header>

              <ul v-else-if="(!guideHasRoutes || currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW) && block.type === 'list' && block.items" class="space-y-2 pl-1">
                <li v-for="(item, iIdx) in block.items" :key="iIdx" class="flex items-start gap-2">
                  <span class="text-faint">·</span>
                  <span v-html="formatGuideInline(item)"></span>
                </li>
              </ul>

              <p v-else-if="(!guideHasRoutes || currentGuideRouteFocus === GUIDE_ROUTE_FOCUS_OVERVIEW) && block.type === 'paragraph' && block.text"
                 :id="`guide-block-${bIdx}`"
                 class="wp-guide-body"
                 v-html="formatGuideInline(block.text)">
              </p>
            </template>
          </article>
          <footer v-if="!isLoadingGuideContent && !guideReadError && guideHasRoutes" class="reader-pagination"><button type="button" :disabled="currentRouteIndex < 0" @click="navigate(currentRouteIndex <= 0 ? GUIDE_ROUTE_FOCUS_OVERVIEW : currentGuideRouteList[currentRouteIndex - 1]!.title)"><ArrowLeft />{{ currentRouteIndex === 0 ? '阅读概览' : '上一条路线' }}</button><span>{{ currentRouteIndex < 0 ? '概览' : String(currentRouteIndex + 1).padStart(2, '0') }} / {{ currentGuideRouteList.length }}</span><button type="button" :disabled="currentRouteIndex >= currentGuideRouteList.length - 1" @click="navigate(currentGuideRouteList[currentRouteIndex + 1]!.title)">{{ currentRouteIndex < 0 ? '开始阅读' : '下一条路线' }}<ArrowRight /></button></footer>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.reader-layer { position:fixed; z-index:45; inset:42px 0 0; display:flex; justify-content:flex-end; pointer-events:none; }
.reader-layer.is-modal { inset:42px 0 0; padding:20px; align-items:center; justify-content:center; pointer-events:auto; background:#080b09b8; backdrop-filter:blur(5px); }
.reader-panel { position:relative; display:flex; flex-direction:column; height:100%; max-width:calc(100% - 76px); min-width:0; background:var(--color-canvas); border-left:1px solid var(--color-hairline); box-shadow:-15px 0 70px #0005; pointer-events:auto; outline:none; container-type:inline-size; }
.is-modal .reader-panel { width:1140px; max-width:100%; max-height:880px; border:1px solid var(--color-hairline); border-radius:9px; overflow:hidden; }
.reader-resize { position:absolute; left:-5px; top:0; bottom:0; width:10px; z-index:5; cursor:ew-resize; display:grid; place-items:center; }
.reader-resize span { width:2px; height:36px; background:var(--color-hairline); border-radius:2px; }
.reader-resize:hover span { height:70px; background:var(--color-beacon); }
.reader-topbar { display:flex; align-items:center; justify-content:space-between; gap:16px; padding:18px 23px 17px; border-bottom:1px solid var(--color-hairline); flex-shrink:0; }
.reader-identity { min-width:0; display:flex; align-items:center; gap:13px; }
.reader-identity > svg { width:21px; height:21px; color:var(--color-beacon); flex-shrink:0; }
.reader-identity > div { min-width:0; }
.reader-identity p { font-size:10px; letter-spacing:.1em; color:var(--color-faint); }
.reader-identity h1 { margin-top:3px; font-size:14px; font-weight:500; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.reader-window-actions { display:flex; gap:4px; flex-shrink:0; }
.reader-window-actions button, .reader-bookmark p button { width:30px; height:30px; display:grid; place-items:center; color:var(--color-faint); border-radius:4px; cursor:pointer; }
.reader-window-actions svg { width:16px; height:16px; }
.reader-window-actions button:hover { background:var(--color-raised); color:var(--color-ink); }
.reader-window-actions button:last-child:hover { color:var(--color-alert); }
.reader-toolbar { display:flex; align-items:center; gap:17px; padding:9px 23px; min-height:46px; border-bottom:1px solid var(--color-hairline); flex-shrink:0; font-size:11px; color:var(--color-faint); }
.reader-toolbar button { display:inline-flex; align-items:center; gap:6px; min-height:28px; cursor:pointer; white-space:nowrap; }
.reader-toolbar button[aria-pressed='true'], .reader-toolbar .reader-resume { color:var(--color-beacon); }
.reader-toolbar svg { width:13px; height:13px; }
.reader-toolbar-spacer { flex:1; }
.reader-status select { background:var(--color-canvas); color:var(--color-muted); font-size:11px; max-width:90px; padding:4px 0; cursor:pointer; }
.reader-toolbar .reader-contents-toggle { display:none; }
.reader-body { display:flex; flex:1; min-height:0; position:relative; }
.reader-contents { display:flex; flex-direction:column; width:216px; flex-shrink:0; background:var(--color-chrome); border-right:1px solid var(--color-hairline); min-height:0; }
.reader-contents-heading { display:flex; justify-content:space-between; align-items:center; padding:22px 18px 12px; font-size:11px; color:var(--color-muted); }
.reader-contents-heading small { font:10px var(--font-mono); color:var(--color-faint); }
.reader-route-progress { height:2px; background:var(--color-hairline); margin:0 18px 12px; }
.reader-route-progress span { display:block; height:100%; background:var(--color-done); }
.reader-contents-search { display:flex; gap:7px; align-items:center; margin:4px 14px 12px; color:var(--color-faint); }
.reader-contents-search svg { width:12px; height:12px; flex-shrink:0; }
.reader-contents-search input { width:100%; min-width:0; background:transparent; font-size:11px; padding:5px 0; }
.reader-contents nav { flex:1; min-height:0; overflow-y:auto; padding:0 10px 18px; }
.reader-contents nav button { display:flex; align-items:baseline; gap:9px; width:100%; padding:11px 9px; margin-bottom:2px; text-align:left; line-height:1.65; font-size:12px; color:var(--color-muted); border-left:2px solid transparent; cursor:pointer; }
.reader-contents nav button > svg, .reader-contents nav button > small { width:15px; flex-shrink:0; font:10px var(--font-mono); }
.reader-contents nav button > svg { height:12px; align-self:flex-start; margin-top:4px; }
.reader-contents nav button.is-complete { color:var(--color-done); }
.reader-contents nav button[aria-current] { color:var(--color-beacon); border-left-color:currentColor; background:color-mix(in srgb,var(--color-beacon) 7%,transparent); }
.reader-contents nav button:hover { background:var(--color-paper); }
.reader-empty-contents { font-size:11px; color:var(--color-faint); padding:15px 10px; }
.reader-bookmark { padding:14px 18px 20px; border-top:1px solid var(--color-hairline); }
.reader-bookmark p { display:flex; align-items:center; gap:6px; color:var(--color-beacon); font-size:10px; }
.reader-bookmark svg { width:12px; height:12px; }
.reader-bookmark p button { margin-left:auto; width:22px; height:22px; }
.reader-bookmark > button { display:flex; flex-direction:column; gap:5px; text-align:left; width:100%; cursor:pointer; }
.reader-bookmark strong { font-size:12px; font-weight:400; line-height:1.6; }
.reader-bookmark span, .reader-bookmark small { font-size:10px; color:var(--color-faint); overflow-wrap:anywhere; }
.reader-scroll { flex:1; min-width:0; min-height:0; overflow:auto; scroll-behavior:smooth; }
.reader-article { max-width:760px; padding:34px 38px 26px; margin-inline:auto; user-select:text; overflow-wrap:anywhere; }
.reader-article-heading { margin-bottom:28px; padding-bottom:24px; border-bottom:1px solid var(--color-hairline); }
.reader-article-heading > p { color:var(--color-beacon); font-size:10px; letter-spacing:.09em; margin-bottom:10px; }
.reader-article-heading h2 { font-family:var(--font-display); font-weight:400; font-size:28px; line-height:1.5; text-wrap:pretty; }
.reader-article-heading > div { margin-top:12px; color:var(--color-faint); font-size:11px; }
.reader-game-links { margin-bottom:24px; }
.reader-personal { border-left:2px solid var(--color-beacon); padding:12px 16px; margin-bottom:24px; background:var(--color-chrome); font-size:12px; }
.reader-personal > div { display:flex; justify-content:space-between; align-items:center; gap:10px; color:var(--color-beacon); font-size:10px; margin-bottom:8px; }
.reader-personal button { display:inline-flex; align-items:center; gap:5px; cursor:pointer; }
.reader-personal svg { width:12px; height:12px; }
.reader-personal p { white-space:pre-wrap; line-height:1.8; margin-block:6px; }
.reader-personal-note, .reader-personal small { color:var(--color-faint); }
.reader-route-status { display:flex; align-items:center; justify-content:space-between; gap:12px; padding-bottom:18px; color:var(--color-faint); font-size:11px; }
.reader-route-status button { display:flex; align-items:center; gap:6px; cursor:pointer; flex-shrink:0; }
.reader-route-status svg { width:13px; height:13px; }
.reader-route-status .is-complete { color:var(--color-done); }
.reader-pagination { display:flex; align-items:center; justify-content:space-between; gap:10px; margin:0 auto; padding:20px 38px 28px; max-width:760px; border-top:1px solid var(--color-hairline); font-size:11px; }
.reader-pagination button { display:flex; align-items:center; gap:7px; min-height:32px; cursor:pointer; color:var(--color-muted); }
.reader-pagination button:disabled { opacity:.25; cursor:default; }
.reader-pagination svg { width:13px; height:13px; }
.reader-pagination > span { font:10px var(--font-mono); color:var(--color-faint); }
.reader-message { display:flex; flex-direction:column; align-items:center; justify-content:center; gap:16px; min-height:250px; padding:30px; text-align:center; }
.reader-message > svg { width:28px; height:28px; color:var(--color-beacon); }
.reader-message h2 { font-size:17px; font-family:var(--font-display); }
.reader-message p { font-size:12px; color:var(--color-faint); overflow-wrap:anywhere; }
.reader-message button { color:var(--color-beacon); cursor:pointer; }
.reader-nav-scrim { display:none; }
.reader-dock { position:fixed; right:22px; bottom:45px; z-index:46; display:flex; align-items:center; max-width:calc(100% - 44px); padding:9px 12px; gap:12px; background:var(--color-paper); border:1px solid var(--color-hairline); border-left:3px solid var(--color-beacon); border-radius:5px; box-shadow:0 10px 30px #0005; }
.reader-dock button { display:flex; align-items:center; gap:12px; min-width:0; cursor:pointer; }
.reader-dock svg { width:15px; height:15px; color:var(--color-beacon); flex-shrink:0; }
.reader-dock span { min-width:0; max-width:240px; text-align:left; }
.reader-dock strong, .reader-dock small { display:block; font-size:12px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; font-weight:400; }
.reader-dock small { font-size:10px; color:var(--color-faint); margin-top:3px; }
.reader-panel :where(button,input,select):focus-visible, .reader-dock button:focus-visible { outline:2px solid var(--color-beacon); outline-offset:3px; }
.reader-panel button:hover:not(:disabled) { color:var(--color-beacon); }
@container (max-width:680px) {
  .reader-toolbar .reader-contents-toggle { display:inline-flex; }
  .reader-contents { display:none; position:absolute; top:0; left:0; bottom:0; z-index:3; width:min(270px,85%); box-shadow:15px 0 40px #0006; }
  .reader-contents.is-open { display:flex; }
  .reader-nav-scrim { display:block; position:absolute; inset:0; z-index:2; background:#0007; }
  .reader-article { padding:26px 25px; }
  .reader-article-heading h2 { font-size:25px; }
  .reader-pagination { padding:15px 25px 24px; }
}
@container (max-width:420px) {
  .reader-topbar { padding:12px 14px; gap:8px; }
  .reader-identity > svg { display:none; }
  .reader-toolbar { padding:7px 14px; gap:11px; }
  .reader-external span { display:none; }
  .reader-article { padding:23px 18px; }
  .reader-route-status { align-items:flex-start; flex-direction:column; }
}
@media (max-width:760px) { .reader-panel { max-width:100%; } .reader-layer.is-modal { padding:8px; } .reader-resize { display:none; } }
@media (max-height:600px) {
  .reader-layer.is-modal { padding:8px; }
  .reader-topbar { padding-block:10px; }
  .reader-contents-heading { padding-top:14px; }
  .reader-bookmark { padding-block:9px; }
  .reader-article { padding-top:20px; }
  .reader-article-heading { margin-bottom:18px; padding-bottom:15px; }
  .reader-article-heading > p { margin-bottom:5px; }
  .reader-article-heading h2 { font-size:23px; }
  .reader-article-heading > div { margin-top:7px; }
}
@media (prefers-reduced-motion:reduce) { .reader-scroll { scroll-behavior:auto; } }
</style>
