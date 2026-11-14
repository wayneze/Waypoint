<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { BookOpen, Check, Image, ImagePlus, Maximize2, NotebookPen, Search, Settings2, Tags, Trash2, X } from '@lucide/vue';
import type { CoverPosition, GameCuration, GameMetadata, GameScalingProfile } from '../types';
import { sourceGameTaxonomy } from '../services/gameTaxonomy';
import { state, gameRecords } from '../workspace/core';
import { editingGameId, editingLibraryId, gameEditorSection, handleSaveGameRecord, libraryGuides, removingGame, singleAndGalgameGuides, type GameEditorSection } from '../workspace/guides/library';
import { metadataBusyIds } from '../workspace/guides/local';
import { isCoverSource } from '../services/gameCoverImage';
import { matchingGuides, sourceIdentity, withGameMetadata } from '../services/gameLibrary';
import { sanitizeAndEnrichGuide } from '../services/gameCovers';
import { loadGameMetadata, searchGameMetadata } from '../services/localGames';
import { posterSrc } from './sky/wikiImage';
import GameCoverEditor from './GameCoverEditor.vue';
import GameBackgroundEditor from './GameBackgroundEditor.vue';
import GameTaxonomyEditor from './GameTaxonomyEditor.vue';
import GameScalingEditor from './GameScalingEditor.vue';
import { describeError } from '../services/errors';

const dialog = ref<HTMLDialogElement | null>(null);
const content = ref<HTMLElement>();
const saving = ref(false), importing = ref(false), error = ref('');
const searching = ref(false), loadingMetadata = ref(false), searchError = ref(''), searched = ref(false);
const source = ref<GameMetadata['source']>('bangumi'), query = ref('');
const candidates = ref<GameMetadata[]>([]), pendingMetadata = ref<GameMetadata>();
const background = ref<string | null>(null), backgroundPosition = ref<CoverPosition>({ x: 50, y: 20 });
const backgroundEdited = ref(false);
const curation = ref<GameCuration>({}), curationEdited = ref(false);
const scaling = ref<GameScalingProfile[]>([]), scalingEdited = ref(false);
let generation = 0;
const sections = [
  { id: 'general', name: '基本信息', icon: Settings2, hint: '名称、类别、合集与攻略关联' },
  { id: 'metadata', name: '资料匹配', icon: Search, hint: '搜索或更新作品介绍与媒体资料' },
  { id: 'taxonomy', name: '标签与制作', icon: Tags, hint: '管理作品标签、开发公司、发行商和制作人员' },
  { id: 'cover', name: '书库封面', icon: Image, hint: '搜索、导入封面并调整展示位置' },
  { id: 'background', name: '详情背景', icon: ImagePlus, hint: '搜索游戏画面或使用自己的图片' },
  { id: 'scaling', name: '画面缩放', icon: Maximize2, hint: '为每个安装版本设置自动缩放与画面预设' },
  { id: 'progress', name: '游玩记录', icon: NotebookPen, hint: '状态、进度、评分与个人备忘' },
  { id: 'removal', name: '移除作品', icon: Trash2, hint: '核对书库记录和磁盘文件的移除范围' },
] as const;
const visibleSections = computed(() => sections.filter(s => s.id !== 'removal' || editingLibraryId.value));
const currentSection = computed(() => sections.find(s => s.id === gameEditorSection.value)!);
const record = computed(() => gameRecords.value.find(r => r.id === editingGameId.value));
const editedGame = computed(() => singleAndGalgameGuides.value.find(g => g.id === editingLibraryId.value || g.record_id === editingGameId.value));
const metadata = computed(() => pendingMetadata.value || record.value?.library?.metadata || undefined);
const metadataBusy = computed(() => !!editingGameId.value && metadataBusyIds.value.includes(editingGameId.value));
const blocked = computed(() => saving.value || importing.value || loadingMetadata.value || metadataBusy.value);
const collections = computed(() => [...new Set(singleAndGalgameGuides.value.map(game => game.series).filter((name): name is string => !!name))].sort((a,b) => a.localeCompare(b, 'zh-Hans-CN')));
const association = computed({
  get: () => state.newGame.guide_id == null ? 'auto' : state.newGame.guide_id === '' ? 'separate' : state.newGame.guide_id,
  set: (value: string) => { state.newGame.guide_id = value === 'auto' ? undefined : value === 'separate' ? '' : value; },
});
const matchedGuide = computed(() => {
  const matches = state.newGame.guide_id == null ? matchingGuides(state.newGame, libraryGuides.value)
    : libraryGuides.value.filter(guide => guide.id === state.newGame.guide_id);
  return matches.length === 1 ? matches[0] : undefined;
});
const sourceTaxonomy = computed(() => sourceGameTaxonomy(matchedGuide.value, metadata.value));
const automaticCover = computed(() => metadata.value?.cover || matchedGuide.value?.poster || sanitizeAndEnrichGuide({
  id: '', title: state.newGame.title, file_name: state.newGame.title, category: state.newGame.category,
  aliases: [], tags: [], genres: [], status: '', rating: 0, file_path: '', relative_path: '', modified: '',
}).poster);
function alreadyLinked(id: string) {
  return singleAndGalgameGuides.value.some(game => game.id === id && game.record_id && game.record_id !== editingGameId.value);
}
watch(() => state.showAddGameModal, async open => {
  generation++;
  error.value = ''; searchError.value = ''; searching.value = false; loadingMetadata.value = false;
  pendingMetadata.value = undefined; candidates.value = []; searched.value = false;
  if (open) {
    curation.value = JSON.parse(JSON.stringify(record.value?.library?.curation || {}));
    curationEdited.value = false;
    scaling.value = JSON.parse(JSON.stringify(record.value?.library?.scaling || []));
    scalingEdited.value = false;
    background.value = record.value?.library?.background ?? null;
    backgroundPosition.value = { ...(record.value?.library?.background_position || { x: 50, y: 20 }) };
    backgroundEdited.value = false;
    const linked = !metadata.value ? Object.values(editedGame.value?.links || {}).map(sourceIdentity).find(Boolean) : undefined;
    source.value = linked?.[0] || metadata.value?.source || 'bangumi';
    query.value = linked?.[1] || metadata.value?.original_title || state.newGame.title;
  }
  await nextTick();
  if (open) dialog.value?.showModal();
  else dialog.value?.close();
}, { immediate: true });
watch(() => state.newGame.title, (title, old) => { if (!query.value.trim() || query.value === old) query.value = title; });
watch(gameEditorSection, () => { if (content.value) content.value.scrollTop = 0; });
function close() { if (!saving.value && !importing.value) state.showAddGameModal = false; }
function switchSection(id: GameEditorSection) { gameEditorSection.value = id; }
function navigate(event: KeyboardEvent) {
  if (!['ArrowUp', 'ArrowDown', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  const items = visibleSections.value;
  const index = items.findIndex(s => s.id === gameEditorSection.value);
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? items.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + items.length) % items.length;
  switchSection(items[next]!.id);
  void nextTick(() => dialog.value?.querySelector<HTMLButtonElement>('#game-editor-tab-' + items[next]!.id)?.focus());
}
async function search() {
  if (searching.value || !query.value.trim()) return;
  const ticket = generation;
  searching.value = true; searched.value = false; searchError.value = ''; candidates.value = [];
  try {
    const result = await searchGameMetadata(source.value, query.value.trim());
    if (ticket === generation) { candidates.value = result; searched.value = true; }
  } catch (cause) { if (ticket === generation) searchError.value = describeError(cause).message; }
  finally { if (ticket === generation) searching.value = false; }
}
async function chooseMetadata(candidate: GameMetadata) {
  if (blocked.value) return;
  const ticket = generation;
  loadingMetadata.value = true; searchError.value = '';
  try {
    const full = await loadGameMetadata(candidate);
    if (ticket !== generation) return;
    pendingMetadata.value = full;
    if (!backgroundEdited.value) {
      const next = withGameMetadata(record.value?.library || { installations: [] }, full);
      background.value = next.background ?? null;
      backgroundPosition.value = { ...(next.background_position || { x: 50, y: 20 }) };
    }
  } catch (cause) { if (ticket === generation) searchError.value = describeError(cause).message; }
  finally { if (ticket === generation) loadingMetadata.value = false; }
}
function remove() { if (editedGame.value && !blocked.value) removingGame.value = editedGame.value; }
async function submit() {
  if (blocked.value || !state.newGame.title.trim()) return;
  error.value = '';
  if (state.newGame.poster.trim() && !isCoverSource(state.newGame.poster.trim())) {
    switchSection('cover'); error.value = '请选择图片或填写有效的图片网址'; return;
  }
  if (!Number.isFinite(state.newGame.rating) || state.newGame.rating < 0 || state.newGame.rating > 10) {
    switchSection('progress'); error.value = '个人评分需为 0 到 10 分'; return;
  }
  saving.value = true;
  try {
    await handleSaveGameRecord({ metadata: pendingMetadata.value,
      curation: curationEdited.value ? curation.value : undefined,
      scaling: scalingEdited.value ? scaling.value : undefined,
      background: backgroundEdited.value ? { background: background.value, background_position: { ...backgroundPosition.value } } : undefined });
  } catch (cause) { error.value = describeError(cause).message; }
  finally { saving.value = false; }
}
</script>

<template>
  <dialog ref="dialog" class="game-editor" aria-labelledby="game-editor-title" @keydown.esc.stop @cancel.prevent="close">
    <form novalidate @submit.prevent="submit">
      <header class="editor-header"><BookOpen /><div><p>{{ editingLibraryId ? state.newGame.title : '私人游戏书库' }}</p><h2 id="game-editor-title">{{ editingLibraryId ? '编辑作品' : '登记作品' }}</h2></div><button type="button" aria-label="关闭作品编辑" :disabled="saving || importing" @click="close"><X /></button></header>
      <div v-if="state.showAddGameModal" class="editor-body">
        <main ref="content" class="game-editor-content" role="tabpanel" id="game-editor-panel" :aria-labelledby="'game-editor-tab-' + gameEditorSection" tabindex="0">
          <div class="editor-section-title"><h3>{{ currentSection.name }}</h3><p>{{ currentSection.hint }}</p></div>
          <fieldset :disabled="saving || importing">
            <section v-show="gameEditorSection === 'general'" class="editor-fields">
              <label>游戏名称<input v-model="state.newGame.title" type="text" required placeholder="例如：白色相簿2 / 群星" autocomplete="off" /></label>
              <label>类别<select v-model="state.newGame.category"><option value="galgame">Galgame · 视觉小说</option><option value="single">单机游戏</option><option value="steam">Steam 游戏</option></select></label>
              <div class="game-editor-pair"><label>所属合集<input v-model="state.newGame.series" list="game-collections" placeholder="选择已有合集，或输入新名称" /><datalist id="game-collections"><option v-for="name in collections" :key="name" :value="name" /></datalist></label><label class="game-rating-field">合集内顺序<input v-model.number="state.newGame.series_order" type="number" min="1" max="4294967295" step="1" :disabled="!state.newGame.series.trim()" placeholder="自动" /></label></div>
              <p class="game-editor-help">同名合集会合并显示；留空后作为独立作品。<button v-if="state.newGame.series" type="button" class="text-action" @click="state.newGame.series = ''; state.newGame.series_order = undefined">移出合集</button></p>
              <label>关联攻略<select v-model="association"><option value="auto">自动匹配名称或别名</option><option value="separate">保持独立，不关联攻略</option><option v-for="guide in libraryGuides" :key="guide.id" :value="guide.id" :disabled="alreadyLinked(guide.id)">{{ guide.title }}{{ alreadyLinked(guide.id) ? ' · 已关联其他档案' : '' }}</option><option v-if="state.newGame.guide_id && !libraryGuides.some(guide => guide.id === state.newGame.guide_id)" :value="state.newGame.guide_id">原关联攻略暂未扫描到</option></select></label>
              <p class="game-editor-help">{{ matchedGuide && !alreadyLinked(matchedGuide.id) ? '将与《' + matchedGuide.title + '》合并显示，保留原攻略和书签。' : association === 'separate' ? '这份档案单独显示。' : '仅在唯一明确匹配时合并显示，也可以手动选择攻略。' }}</p>
            </section>
            <section v-show="gameEditorSection === 'metadata'" class="editor-fields">
              <article v-if="metadata" class="editor-current-metadata"><img v-if="metadata.cover" :src="posterSrc(metadata.cover)" alt="" referrerpolicy="no-referrer" /><div><small>{{ pendingMetadata ? '待保存的资料' : '当前匹配' }} · {{ metadata.source.toUpperCase() }} · {{ metadata.source_id }}</small><h4>{{ metadata.title }}</h4><p>{{ metadata.original_title }}</p><p>{{ [metadata.developer, metadata.release_date].filter(Boolean).join(' · ') }}</p><button type="button" class="text-action" :disabled="blocked" @click="chooseMetadata(metadata)">重新获取资料</button></div></article>
              <p class="game-editor-help">无需安装游戏，也可以匹配作品资料。资料、封面与背景可分别选择，最后统一保存。</p>
              <div class="editor-search"><select v-model="source" aria-label="资料来源" :disabled="searching"><option value="bangumi">Bangumi</option><option value="vndb">VNDB</option><option value="steam">Steam</option></select><input v-model="query" aria-label="搜索资料名称" placeholder="原名、中文名或来源 ID" @keydown.enter.prevent="search" /><button type="button" :disabled="searching || !query.trim()" @click="search"><Search />{{ searching ? '搜索中…' : '搜索资料' }}</button></div>
              <p v-if="metadataBusy" role="status" class="game-editor-help">这部作品正在补全资料，请等待保存结束。</p><p v-if="loadingMetadata" role="status" class="game-editor-help">正在读取完整资料与媒体…</p><p v-if="searchError" role="alert" class="editor-error">{{ searchError }}</p><p v-if="searched && !candidates.length" role="status" class="game-editor-help">未找到结果，试试原名、英文名或来源 ID。</p>
              <div class="editor-matches"><article v-for="candidate in candidates" :key="candidate.source + candidate.source_id"><img v-if="candidate.cover" :src="posterSrc(candidate.cover)" alt="" loading="lazy" referrerpolicy="no-referrer" /><div><h4>{{ candidate.title }}</h4><p>{{ candidate.original_title }}</p><small>{{ candidate.source.toUpperCase() }} · {{ candidate.source_id }} · {{ candidate.release_date }}</small><p>{{ candidate.description.slice(0, 130) }}</p><button type="button" :disabled="blocked" :aria-pressed="pendingMetadata?.source === candidate.source && pendingMetadata?.source_id === candidate.source_id" @click="chooseMetadata(candidate)">{{ pendingMetadata?.source === candidate.source && pendingMetadata?.source_id === candidate.source_id ? '已选用 · 待保存' : '选用这份资料' }}</button></div></article></div>
            </section>
            <section v-show="gameEditorSection === 'cover'"><GameCoverEditor v-model="state.newGame.poster" v-model:position="state.newGame.poster_position" :automatic="automaticCover" :title="state.newGame.title" @busy="importing = $event" /></section>
            <section v-show="gameEditorSection === 'background'"><GameBackgroundEditor v-model="background" v-model:position="backgroundPosition" :title="state.newGame.title" :metadata="metadata" :poster="state.newGame.poster || automaticCover" @update:model-value="backgroundEdited = true" @update:position="backgroundEdited = true" @busy="importing = $event" /></section>
            <section v-show="gameEditorSection === 'taxonomy'"><GameTaxonomyEditor v-model="curation" :source="sourceTaxonomy" @update:model-value="curationEdited = true" /></section>
            <section v-show="gameEditorSection === 'scaling'"><GameScalingEditor v-model="scaling" :installations="record?.library?.installations || []" @update:model-value="scalingEdited = true" /></section>
            <section v-show="gameEditorSection === 'progress'" class="editor-fields">
              <label>游玩状态<select v-model="state.newGame.status" aria-label="游玩状态"><option value="playing">正在游玩</option><option value="completed">已通关</option><option value="backlog">待玩 / 搁置</option></select></label>
              <div class="game-editor-pair"><label>当前进度<input v-model="state.newGame.progress" type="text" placeholder="第五章 / 某条角色路线" /></label><label class="game-rating-field">个人评分<input v-model.number="state.newGame.rating" type="number" min="0" max="10" step="0.1" /></label></div>
              <label>备忘<textarea v-model="state.newGame.notes" rows="6" placeholder="记下路线选择、下一步计划或剧情感受…"></textarea></label>
              <p class="game-editor-help">存档会在详情页“安装与存档”中自动识别，也可确认多个位置或手动选择目录。</p>
            </section>
            <section v-show="gameEditorSection === 'removal'" class="editor-fields editor-removal"><Trash2 /><h4>从书库移除这部作品</h4><p>默认保留磁盘上的游戏与攻略文件。下一步会列出关联文件，分别确认移除范围。</p><p>执行移除后，本窗口尚未保存的修改会放弃。</p><button type="button" class="editor-danger" :disabled="blocked || !editedGame" @click="remove">检查移除范围…</button></section>
          </fieldset>
        </main>
        <nav class="editor-navigation" role="tablist" aria-label="作品设置分类" aria-orientation="vertical" @keydown="navigate"><button v-for="item in visibleSections" :id="'game-editor-tab-' + item.id" :key="item.id" type="button" role="tab" aria-controls="game-editor-panel" :aria-selected="gameEditorSection === item.id" :tabindex="gameEditorSection === item.id ? 0 : -1" :class="{ danger: item.id === 'removal' }" :disabled="saving || importing" @click="switchSection(item.id)"><component :is="item.icon" /><span>{{ item.name }}</span></button><p>切换分类保留修改<br />点下方保存后生效</p></nav>
      </div>
      <footer class="editor-footer"><p v-if="error" role="alert">{{ error }}</p><span v-else>{{ saving ? '正在保存…' : '所有分类的修改统一保存' }}</span><button type="button" :disabled="saving || importing" @click="close">取消</button><button type="submit" class="game-editor-save" :disabled="blocked || !state.newGame.title.trim()"><Check />{{ saving ? '正在保存…' : editingLibraryId ? '保存修改' : '加入书库' }}</button></footer>
    </form>
  </dialog>
</template>

<style scoped>
.game-editor{position:fixed;inset:0;margin:auto;width:960px;max-width:calc(100% - 32px);height:720px;max-height:calc(100% - 32px);padding:0;border:1px solid var(--color-hairline);border-radius:8px;background:var(--color-canvas);color:var(--color-ink);overflow:hidden;box-shadow:0 24px 80px #0008}.game-editor[open],form{display:flex;flex-direction:column;min-height:0}form{flex:1}.game-editor::backdrop{background:#0e100ed4}.editor-header{display:flex;align-items:center;gap:14px;padding:20px 26px;border-bottom:1px solid var(--color-hairline);flex:none}.editor-header>svg{width:29px;height:29px;color:var(--color-beacon);stroke-width:1}.editor-header>div{min-width:0}.editor-header p{color:var(--color-faint);font-size:11px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:650px}.editor-header h2{font-size:22px;font-weight:500;margin-top:4px}.editor-header button{display:grid;place-items:center;margin-left:auto;min-width:32px;height:32px;color:var(--color-faint)}.editor-body{display:grid;grid-template-columns:minmax(0,1fr) 156px;min-height:0;flex:1}.game-editor-content{min-height:0;min-width:0;overflow-y:auto;padding:26px;scrollbar-gutter:stable}.editor-section-title{margin-bottom:24px}.editor-section-title h3{font-size:19px;font-weight:500}.editor-section-title p{font-size:12px;color:var(--color-muted);margin-top:6px;line-height:1.7}.editor-navigation{border-left:1px solid var(--color-hairline);padding:18px 10px;display:flex;flex-direction:column;gap:6px;background:var(--color-chrome);overflow:auto}.editor-navigation button{display:flex;align-items:center;gap:10px;min-height:42px;padding:10px 12px;border-radius:4px;color:var(--color-muted);font-size:12px;text-align:left;flex:none}.editor-navigation button:hover{background:var(--color-paper)}.editor-navigation button[aria-selected=true]{background:color-mix(in srgb,var(--color-beacon) 12%,transparent);color:var(--color-beacon)}.editor-navigation button.danger{margin-top:auto;color:var(--color-alert)}.editor-navigation p{font-size:10px;line-height:1.9;color:var(--color-faint);padding:10px 12px 0}.editor-navigation svg{width:15px;height:15px;flex:none}.editor-fields{display:grid;gap:17px}fieldset{min-width:0}label{display:flex;flex-direction:column;gap:7px;font-size:12px;color:var(--color-muted);min-width:0}input,select,textarea{min-width:0;width:100%;min-height:36px;padding:8px 10px;font-size:12px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-chrome);color:var(--color-ink)}input::placeholder,textarea::placeholder{color:var(--color-faint)}textarea{resize:vertical}.game-editor-pair{display:flex;gap:14px}.game-editor-pair>label{flex:1}.game-editor-pair>.game-rating-field{flex:0 0 110px}.game-editor-help{font-size:12px;color:var(--color-faint);line-height:1.8}.text-action{color:var(--color-beacon);font-size:12px}.game-editor-help .text-action{margin-left:8px}.editor-footer{display:flex;align-items:center;gap:12px;padding:16px 26px;border-top:1px solid var(--color-hairline);background:var(--color-chrome);flex:none}.editor-footer>span,.editor-footer>p{margin-right:auto;font-size:11px;color:var(--color-faint)}.editor-footer>p{color:var(--color-alert);max-width:60%;overflow-wrap:anywhere}.editor-footer button{display:inline-flex;align-items:center;gap:7px;padding:8px 12px;font-size:12px;white-space:nowrap;color:var(--color-muted)}.editor-footer .game-editor-save{color:var(--color-beacon);border:1px solid color-mix(in srgb,var(--color-beacon) 35%,transparent);border-radius:4px;background:color-mix(in srgb,var(--color-beacon) 8%,transparent)}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}svg{width:16px;height:16px}.editor-search{display:flex;gap:8px}.editor-search select{width:108px}.editor-search input{flex:1}.editor-search button,.editor-matches button,.editor-danger{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:8px 12px;border:1px solid var(--color-hairline);border-radius:4px;font-size:12px;white-space:nowrap}.editor-search button{color:var(--color-beacon)}.editor-matches{display:grid;gap:16px}.editor-matches article,.editor-current-metadata{display:flex;gap:16px;padding:16px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper)}.editor-matches img,.editor-current-metadata img{width:70px;height:94px;object-fit:cover;border-radius:3px;flex:none}.editor-matches article>div,.editor-current-metadata>div{min-width:0}.editor-matches h4,.editor-current-metadata h4{font-size:14px;font-weight:500;line-height:1.6}.editor-matches p,.editor-current-metadata p{font-size:12px;line-height:1.6;color:var(--color-muted);margin-block:5px}.editor-matches small,.editor-current-metadata small{font-size:10px;color:var(--color-faint)}.editor-current-metadata h4{margin-top:5px}.editor-matches button{margin-top:8px;color:var(--color-beacon)}.editor-matches button[aria-pressed=true]{border-color:var(--color-beacon)}.editor-error{font-size:12px;color:var(--color-alert);overflow-wrap:anywhere}.editor-removal{align-content:start}.editor-removal>svg{width:28px;height:28px;color:var(--color-alert)}.editor-removal h4{font-size:16px;font-weight:500}.editor-removal p{font-size:12px;line-height:1.8;color:var(--color-muted)}.editor-removal button{justify-self:start;color:var(--color-alert);border-color:var(--color-alert)}:where(button,input,select,textarea):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}.game-editor-content:focus-visible{outline:none}
@media(max-width:650px){.game-editor{max-width:calc(100% - 16px);max-height:calc(100% - 16px)}.editor-body{grid-template-columns:minmax(0,1fr) 116px}.game-editor-content{padding:18px 14px}.editor-header,.editor-footer{padding:14px 16px}.editor-navigation{padding:12px 6px}.editor-navigation button{gap:7px;padding:10px 8px;font-size:11px}.editor-navigation p{padding-inline:6px}.editor-search{flex-wrap:wrap}.editor-search input{min-width:120px}.editor-current-metadata img,.editor-matches img{width:50px;height:67px}.game-editor-pair{flex-wrap:wrap}.editor-footer>span{max-width:110px;line-height:1.6}.editor-header p{max-width:65vw}}@media(max-height:600px){.editor-header{padding-block:12px}.editor-header p{display:none}.editor-header h2{font-size:19px}.editor-footer{padding-block:10px}.editor-navigation{padding-block:8px;gap:2px}.editor-navigation button{min-height:36px;padding-block:7px}.editor-navigation p{display:none}}
</style>
