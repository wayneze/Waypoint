<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ImagePlus, Search } from '@lucide/vue';
import type { CoverPosition, GameMetadata } from '../types';
import { coverPosition, detailBackground, importCoverImage, isCoverSource } from '../services/gameCoverImage';
import { posterSrc } from './sky/wikiImage';
import { searchGameMetadata } from '../services/localGames';

const props = defineProps<{ title: string; metadata?: GameMetadata; poster?: string }>();
const emit = defineEmits<{ busy: [value: boolean] }>();
const fileInput = ref<HTMLInputElement>();
const selected = defineModel<string | null>({ required: true });
const position = defineModel<CoverPosition>('position', { required: true });
const address = ref('');
const busy = ref(false), failed = ref(false), error = ref('');
const failedChoices = ref<Record<string, boolean>>({});
const loadedChoices = ref<Record<string, boolean>>({});
const searchSource = ref<'vndb' | 'steam'>('vndb');
const searchQuery = ref(props.metadata?.original_title || props.title);
watch(() => props.title, (title, old) => { if (!searchQuery.value.trim() || searchQuery.value === old) searchQuery.value = title; });
const searchResults = ref<GameMetadata[]>([]);
const searching = ref(false), searched = ref(false), searchError = ref('');
const loadingImages = ref('');
let searchGeneration = 0;
onBeforeUnmount(() => { searchGeneration++; });
function images(meta: GameMetadata) {
  return [...new Set([meta.background, ...meta.screenshots, meta.cover].filter((s): s is string => !!s))];
}
async function searchImages() {
  if (!searchQuery.value.trim() || searching.value) return;
  const ticket = ++searchGeneration;
  searching.value = true; searched.value = false; searchError.value = ''; searchResults.value = [];
  try {
    const results = await searchGameMetadata(searchSource.value, searchQuery.value.trim());
    if (ticket === searchGeneration) { searchResults.value = results; searched.value = true; }
  } catch (e) { if (ticket === searchGeneration) searchError.value = String(e); }
  finally { if (ticket === searchGeneration) searching.value = false; }
}
async function loadImages(meta: GameMetadata) {
  if (loadingImages.value) return;
  const ticket = searchGeneration;
  loadingImages.value = meta.source_id; searchError.value = '';
  try {
    const results = await searchGameMetadata(meta.source, meta.source_id);
    if (ticket === searchGeneration) searchResults.value = searchResults.value.map(item => item.source_id === meta.source_id ? results.find(r => r.source_id === meta.source_id) || item : item);
  } catch (e) { if (ticket === searchGeneration) searchError.value = String(e); }
  finally { loadingImages.value = ''; }
}
const preview = computed(() => detailBackground({ metadata: props.metadata, installations: [], background: selected.value }, props.poster));
const choices = computed(() => [...new Set([props.metadata?.background, props.poster, ...props.metadata?.screenshots || []].filter((s): s is string => !!s))]);
function choose(value: string | null) {
  selected.value = value; position.value = { x: 50, y: 20 }; failed.value = false; error.value = '';
}
function useAddress() {
  const url = address.value.trim();
  if (!isCoverSource(url)) { error.value = '请填写有效的图片网址。'; return; }
  choose(url);
}
async function useFile(file?: File) {
  if (!file || busy.value) return;
  busy.value = true; emit('busy', true); error.value = '';
  try { choose(await importCoverImage(file)); }
  catch (e) { error.value = String(e); }
  finally { busy.value = false; emit('busy', false); if (fileInput.value) fileInput.value.value = ''; }
}
function paste(event: ClipboardEvent) {
  const file = Array.from(event.clipboardData?.files || []).find(f => f.type.startsWith('image/'));
  if (file) { event.preventDefault(); void useFile(file); }
}
</script>

<template>
  <div class="background-editor" @paste="paste" @dragover.prevent @drop.prevent="useFile($event.dataTransfer?.files[0])">
      <div class="background-content">
        <div class="background-preview" tabindex="0" aria-label="背景预览，可粘贴或拖入图片">
          <img v-if="preview && !failed" :src="posterSrc(preview)" :style="{objectPosition: coverPosition(position)}" alt="详情背景预览" referrerpolicy="no-referrer" @error="failed = true" />
          <span v-else>{{ failed ? '图片未加载，请换图或重试' : '纯色背景' }}</span>
          <strong>{{ title }}</strong>
        </div>
        <p class="background-hint">自动使用官方背景，没有时使用封面。你选择的图片会保留，更新资料不会覆盖。</p>
        <div class="background-actions"><button type="button" :disabled="busy" :aria-pressed="selected === null" @click="choose(null)">自动背景</button><button type="button" :disabled="busy" :aria-pressed="selected === ''" @click="choose('')">纯色背景</button><button type="button" :disabled="busy" @click="fileInput?.click()"><ImagePlus />导入图片</button></div>
        <input ref="fileInput" class="background-file" type="file" accept="image/jpeg,image/png,image/webp,image/gif" aria-label="选择背景图片文件" @change="useFile(($event.target as HTMLInputElement).files?.[0])" />
        <p class="background-hint">支持拖入、Ctrl+V 粘贴；本地图片会自动压缩并随完整备份保存。</p>
        <div class="background-url"><label>图片网址<input v-model="address" type="url" placeholder="https://…" :disabled="busy" @keydown.enter.prevent="useAddress" /></label><button type="button" :disabled="busy || !address.trim()" @click="useAddress">预览网址</button></div>
        <div v-if="preview" class="background-position"><label>水平位置<input :value="position.x" type="range" min="0" max="100" aria-label="背景水平位置" @input="position = { ...position, x: Number(($event.target as HTMLInputElement).value) }" /></label><label>垂直位置<input :value="position.y" type="range" min="0" max="100" aria-label="背景垂直位置" @input="position = { ...position, y: Number(($event.target as HTMLInputElement).value) }" /></label><button v-if="failed" type="button" @click="failed = false">重试图片</button></div>
        <div v-if="choices.length" class="background-choices" aria-label="作品背景候选"><button v-for="(url,index) in choices" :key="url" type="button" :aria-pressed="selected === url" :aria-label="'选择背景图片 ' + (index + 1)" :disabled="busy" @click="choose(url)"><img v-if="!failedChoices[url]" :src="posterSrc(url)" alt="" loading="lazy" referrerpolicy="no-referrer" @load="loadedChoices[url] = true" @error="failedChoices[url] = true" /><span v-else class="background-unavailable">图片暂未载入</span><span v-if="!loadedChoices[url] && !failedChoices[url]" class="background-loading">图片载入中…</span><span>{{ url === metadata?.background ? '官方背景' : url === poster ? '作品封面' : '游戏画面' }}</span></button></div>
        <section class="background-search" aria-label="在线搜索背景">
          <h3>在线搜索背景</h3><p class="background-hint">按作品名或来源 ID 搜索，选择游戏画面或封面作为背景。</p>
          <div class="background-search-fields"><select v-model="searchSource" aria-label="背景搜索来源" :disabled="searching"><option value="vndb">VNDB</option><option value="steam">Steam</option></select><input v-model="searchQuery" aria-label="背景搜索关键词" placeholder="原名、中文名或来源 ID" :disabled="searching" @keydown.enter.prevent="searchImages" /><button type="button" :disabled="searching || !searchQuery.trim()" @click="searchImages"><Search />{{ searching ? '搜索中…' : '搜索背景' }}</button></div>
          <p v-if="searchError" role="alert" class="background-error">{{ searchError }}</p><p v-if="searched && !searchResults.length" role="status" class="background-hint">未找到作品，试试原名、来源 ID 或切换来源。</p>
          <article v-for="item in searchResults" :key="item.source + item.source_id" class="background-search-result"><header><div><h4>{{ item.title }}</h4><p>{{ item.source.toUpperCase() }} · {{ item.source_id }} · {{ item.release_date }}</p></div><button v-if="item.source === 'steam' && !item.screenshots.length" type="button" :disabled="!!loadingImages" @click="loadImages(item)">{{ loadingImages === item.source_id ? '读取中…' : '获取游戏画面' }}</button></header>
            <div class="background-choices" :aria-label="item.title + '搜索图片'"><button v-for="(url,index) in images(item)" :key="url" type="button" :aria-label="'使用搜索背景 ' + item.source_id + '-' + (index + 1)" :aria-pressed="selected === url" :disabled="busy" @click="choose(url)"><img v-if="!failedChoices[url]" :src="posterSrc(url)" alt="" loading="lazy" referrerpolicy="no-referrer" @load="loadedChoices[url] = true" @error="failedChoices[url] = true" /><span v-else class="background-unavailable">图片暂未载入</span><span v-if="!loadedChoices[url] && !failedChoices[url]" class="background-loading">图片载入中…</span><span>{{ url === item.cover ? '作品封面' : '游戏画面' }}</span></button></div>
          </article>
        </section>
        <p v-if="error" role="alert" class="background-error">{{ error }}</p>
      </div>
  </div>
</template>

<style scoped>
.background-editor{min-width:0}.background-content{min-width:0}.background-preview{position:relative;isolation:isolate;aspect-ratio:16/8;display:grid;place-items:center;overflow:hidden;border-radius:4px;background:var(--color-paper)}.background-preview img{width:100%;height:100%;object-fit:cover;position:absolute;inset:0;z-index:-2}.background-preview::after{content:'';position:absolute;inset:0;z-index:-1;background:linear-gradient(transparent 40%,#101410bb)}.background-preview strong{position:absolute;bottom:20px;left:20px;right:20px;font:22px var(--font-display);color:#fff}.background-preview span{color:var(--color-muted);font-size:12px}.background-hint{font-size:12px;line-height:1.7;color:var(--color-muted);margin:12px 0}.background-actions{display:flex;flex-wrap:wrap;gap:8px}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;min-height:36px;padding:7px 12px;border:1px solid var(--color-hairline);border-radius:4px;font-size:12px;cursor:pointer}button:hover{background:var(--color-paper)}button[aria-pressed=true]{border-color:var(--color-beacon);color:var(--color-beacon)}button:disabled{opacity:.45;cursor:default}svg{width:16px;height:16px}.background-file{display:none}.background-url{display:flex;align-items:end;gap:10px;margin-top:16px}.background-url label{flex:1}.background-position{display:flex;gap:20px;margin:18px 0}.background-position label{flex:1}label{display:grid;gap:6px;font-size:11px;color:var(--color-muted);min-width:0}input[type=url]{width:100%;background:var(--color-paper);border:1px solid var(--color-hairline);padding:9px;border-radius:4px;font-size:12px;color:var(--color-ink)}input[type=range]{width:100%;accent-color:var(--color-beacon)}.background-choices{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:10px;margin-top:18px}.background-choices button{position:relative;display:flex;flex-direction:column;min-width:0;padding:4px;overflow:hidden}.background-choices img{width:100%;aspect-ratio:16/10;object-fit:cover}.background-choices span{font-size:10px;color:var(--color-muted);padding:4px}.background-choices .background-loading{position:absolute;inset:4px 4px 26px;display:grid;place-items:center;background:var(--color-paper);font-size:11px;color:var(--color-faint)}.background-error{font-size:12px;color:var(--color-alert);margin-top:16px;overflow-wrap:anywhere}.background-search{margin-top:24px;padding-top:20px;border-top:1px solid var(--color-hairline)}.background-search h3{font-size:16px;font-weight:500}.background-search-fields{display:flex;gap:8px;flex-wrap:wrap}.background-search-fields input{flex:1;min-width:130px}.background-search-fields input,.background-search-fields select{background:var(--color-paper);border:1px solid var(--color-hairline);border-radius:4px;padding:8px;font-size:12px;color:var(--color-ink)}.background-search-result{margin-top:18px}.background-search-result header{display:flex;align-items:center;justify-content:space-between;gap:10px;flex-wrap:wrap}.background-search-result h4{font-size:13px}.background-search-result header p{margin-top:4px;font-size:11px;color:var(--color-muted)}.background-search-result .background-choices{margin-top:10px}.background-choices .background-unavailable{display:grid;place-items:center;width:100%;aspect-ratio:16/10;background:var(--color-paper);font-size:10px}:where(button,input,.background-preview):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:650px){.background-choices{grid-template-columns:repeat(2,minmax(0,1fr))}.background-url{flex-wrap:wrap}.background-preview strong{font-size:17px;bottom:12px;left:12px}}
</style>
