<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ClipboardPaste, ImagePlus } from '@lucide/vue';
import type { CoverPosition } from '../types';
import { importCoverImage, coverPosition } from '../services/gameCoverImage';
import { posterSrc } from './sky/wikiImage';
import { describeError } from '../services/errors';
import { searchGameMetadata } from '../services/localGames';
import type { GameMetadata } from '../types';

const props = defineProps<{ automatic?: string; title: string }>();
const source = defineModel<string>({ required: true });
const position = defineModel<CoverPosition>('position', { required: true });
const emit = defineEmits<{ busy: [value: boolean] }>();
const fileInput = ref<HTMLInputElement | null>(null);
const busy = ref(false);
const error = ref('');
const failed = ref(false);
const onlineSource = ref<GameMetadata['source']>('bangumi');
const query = ref(props.title);
const results = ref<GameMetadata[]>([]);
const searching = ref(false);
const searchError = ref('');
let searchTicket = 0;
watch(() => props.title, title => { query.value = title; results.value = []; searchTicket++; searching.value = false; });
onBeforeUnmount(() => { searchTicket++; });
async function searchOnline() {
  if (!query.value.trim() || searching.value) return;
  const ticket = ++searchTicket;
  searching.value = true; searchError.value = '';
  try {
    const found = await searchGameMetadata(onlineSource.value, query.value.trim());
    if (ticket === searchTicket) {
      results.value = found.filter(item => item.cover);
      if (!results.value.length) searchError.value = '此来源没有封面，试试原名、英文名或切换来源。';
    }
  } catch (cause) { if (ticket === searchTicket) searchError.value = describeError(cause).message; }
  finally { if (ticket === searchTicket) searching.value = false; }
}
function chooseCover(url: string) { source.value = url; position.value = { x: 50, y: 50 }; }
const local = computed(() => source.value.startsWith('data:'));
const url = computed({ get: () => local.value ? '' : source.value, set: value => { source.value = value; } });
const preview = computed(() => source.value || props.automatic);
watch(preview, () => { failed.value = false; error.value = ''; });
async function useFile(file?: File) {
  if (!file || busy.value) return;
  busy.value = true; emit('busy', true); error.value = '';
  try {
    source.value = await importCoverImage(file);
    position.value = { x: 50, y: 50 };
  } catch (cause) { error.value = describeError(cause).message; }
  finally { busy.value = false; emit('busy', false); if (fileInput.value) fileInput.value.value = ''; }
}
function paste(event: ClipboardEvent) {
  const file = Array.from(event.clipboardData?.files || []).find(file => file.type.startsWith('image/'));
  if (file) { event.preventDefault(); void useFile(file); }
}
async function readClipboard() {
  if (busy.value) return;
  busy.value = true; emit('busy', true);
  let file: File | undefined;
  try {
    const items = await navigator.clipboard.read();
    for (const item of items) {
      const type = item.types.find(type => type.startsWith('image/'));
      if (type) { file = new File([await item.getType(type)], 'clipboard', { type }); break; }
    }
    if (!file) error.value = '剪贴板里没有图片，可以复制图片后在此处按 Ctrl+V';
  } catch { error.value = '无法读取剪贴板，请点击封面区域后按 Ctrl+V，或选择图片'; }
  finally { busy.value = false; emit('busy', false); }
  if (file) await useFile(file);
}
function reset() { source.value = ''; position.value = { x: 50, y: 50 }; error.value = ''; }
</script>

<template>
  <div class="cover-editor" @paste="paste" @dragover.prevent @drop.prevent="useFile($event.dataTransfer?.files[0])">
    <div class="cover-preview" tabindex="0" aria-label="封面预览，可粘贴或拖入图片">
      <img v-if="preview && !failed" :src="posterSrc(preview)" :style="{ objectPosition: coverPosition(position) }" alt="封面展示预览" referrerpolicy="no-referrer" @error="failed = true" />
      <div v-else class="cover-letter"><span>WAYPOINT</span><strong>{{ title || '下一段旅程' }}</strong><small>私人游戏书库</small></div>
    </div>
    <div class="cover-controls">
      <p>让这部游戏，一眼就能认出来</p>
      <div class="cover-online">
        <label>在线封面搜索<input v-model="query" aria-label="在线封面关键词" placeholder="原名、中文名或来源 ID" @keydown.enter.prevent="searchOnline" /></label>
        <div class="cover-actions"><select v-model="onlineSource" aria-label="在线封面来源"><option value="bangumi">Bangumi</option><option value="vndb">VNDB</option><option value="steam">Steam</option></select><button type="button" :disabled="searching || !query.trim()" @click="searchOnline">{{ searching ? '搜索中…' : '搜索在线封面' }}</button></div>
        <div v-if="automatic || results.length" class="cover-options" aria-label="在线封面候选">
          <button v-if="automatic" type="button" :aria-pressed="preview === automatic" @click="chooseCover(automatic)"><img :src="posterSrc(automatic)" alt="" loading="lazy" referrerpolicy="no-referrer" /><span>已匹配封面</span></button>
          <button v-for="item in results" :key="item.source + item.source_id" type="button" :aria-pressed="source === item.cover" @click="chooseCover(item.cover)"><img :src="posterSrc(item.cover)" alt="" loading="lazy" referrerpolicy="no-referrer" /><span>{{ item.title }}<small>{{ item.source.toUpperCase() }} · {{ item.release_date || item.source_id }}</small></span></button>
        </div>
        <p v-if="searchError" role="status">{{ searchError }}</p>
      </div>
      <div class="cover-actions"><button type="button" :disabled="busy" @click="fileInput?.click()"><ImagePlus />选择图片</button><button type="button" :disabled="busy" @click="readClipboard"><ClipboardPaste />粘贴图片</button></div>
      <input ref="fileInput" class="cover-file" type="file" accept="image/jpeg,image/png,image/webp,image/gif" aria-label="选择封面图片文件" @change="useFile(($event.target as HTMLInputElement).files?.[0])" />
      <p class="cover-hint">{{ busy ? '正在处理图片…' : local ? '本地图片已导入，将随完整备份保存。' : '也可拖入图片，或点击预览后按 Ctrl+V。' }}</p>
      <label>图片网址 <input v-model="url" type="url" :disabled="busy" placeholder="https://… 也可以直接填写图片网址" /></label>
      <div v-if="preview && !failed" class="cover-position">
        <label>水平位置<input v-model.number="position.x" type="range" min="0" max="100" /></label>
        <label>垂直位置<input v-model.number="position.y" type="range" min="0" max="100" /></label>
      </div>
      <button type="button" class="cover-reset" :disabled="busy" @click="reset">恢复自动封面与位置</button>
      <p v-if="failed" class="cover-hint">图片无法显示，请换一个地址或导入图片。</p>
      <p v-if="error" class="cover-error" role="alert">{{ error }}</p>
    </div>
  </div>
</template>

<style scoped>
.cover-editor { display:grid; grid-template-columns:130px minmax(0,1fr); gap:20px; align-items:start; }
.cover-preview { aspect-ratio:3/4; overflow:hidden; border:1px solid var(--color-hairline); border-radius:3px; background:var(--color-paper); }
.cover-preview img { display:block; width:100%; height:100%; object-fit:cover; }
.cover-letter { aspect-ratio:3/4; display:flex; flex-direction:column; padding:12px; background:#29281f; color:var(--color-beacon); border-left:4px solid #625837; }
.cover-letter strong { flex:1; display:flex; align-items:center; overflow:hidden; font-family:var(--font-display); font-size:18px; line-height:1.5; overflow-wrap:anywhere; }
.cover-letter span, .cover-letter small { font-size:9px; opacity:.7; letter-spacing:.08em; }
.cover-controls { min-width:0; display:grid; gap:10px; font-size:11px; color:var(--color-muted); }
.cover-actions { display:flex; gap:16px; flex-wrap:wrap; }
button { display:inline-flex; align-items:center; gap:6px; cursor:pointer; min-height:28px; color:var(--color-beacon); text-align:left; }
svg { width:14px; height:14px; }
.cover-file { display:none; }
.cover-hint { font-size:10px; line-height:1.7; color:var(--color-faint); }
label { display:flex; gap:7px; flex-direction:column; font-size:10px; min-width:0; }
input[type=url] { min-width:0; width:100%; padding:8px; border:1px solid var(--color-hairline); background:var(--color-chrome); border-radius:4px; font-size:11px; }
.cover-online{display:grid;gap:10px;padding-bottom:14px;border-bottom:1px solid var(--color-hairline)}.cover-online input,.cover-online select{padding:8px;border:1px solid var(--color-hairline);background:var(--color-chrome);border-radius:4px;min-width:0;color:var(--color-ink)}.cover-options{display:grid;grid-template-columns:repeat(auto-fill,minmax(85px,1fr));gap:8px;max-height:290px;overflow:auto}.cover-options button{display:flex;flex-direction:column;align-items:stretch;justify-content:start;border:1px solid var(--color-hairline);border-radius:4px;padding:5px;min-width:0}.cover-options [aria-pressed=true]{border-color:var(--color-beacon);background:color-mix(in srgb,var(--color-beacon) 9%,transparent)}.cover-options img{width:100%;height:110px;object-fit:contain;background:var(--color-chrome)}.cover-options span{font-size:10px;line-height:1.5;overflow-wrap:anywhere}.cover-options small{display:block;color:var(--color-faint);font-size:9px}
.cover-position { display:flex; gap:16px; }
.cover-position label { flex:1; }
input[type=range] { width:100%; min-width:0; accent-color:var(--color-beacon); }
.cover-reset { font-size:10px; color:var(--color-faint); }
.cover-error { color:var(--color-alert); overflow-wrap:anywhere; }
button:disabled { opacity:.5; cursor:default; }
:where(button,input,.cover-preview):focus-visible { outline:2px solid var(--color-beacon); outline-offset:3px; }
@media (max-width:560px) { .cover-editor { grid-template-columns:90px minmax(0,1fr); gap:12px; } }
</style>
