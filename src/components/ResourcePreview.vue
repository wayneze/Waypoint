<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { BookOpen, BookmarkPlus, ExternalLink, FolderOpen, RotateCw, ScanLine } from '@lucide/vue';
import type { ObsidianGameGuide } from '../types';
import { currentResourceWeb, openExternalUrl, openInExplorer, openResourceWeb, pickOpenFile, pickSaveFile, readResourceArticle, writeResourceMarkdown, type ResourceArticle } from '../services/systemApi';
import { formatGuideInline, parseObsidianGuide } from '../services/guideParser';
import { gameSourceIds } from '../services/gameLibrary';
import { linksForGames, saveGameLink, type GameLink } from '../workspace/guides/extras';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import { handleOpenResourceReader } from '../workspace/guides/reader';
import JuijDownloads from './JuijDownloads.vue';
import { obsidianVaultPath } from '../workspace/core';

const props = defineProps<{ link: GameLink; game?: ObsidianGameGuide | null }>();
const isJuij = computed(() => /(^|\.)juij\.fun$/.test(new URL(props.link.url).hostname));
const emit = defineEmits<{ navigate: [link: GameLink]; saved: [] }>();
const selectedGame = ref('');
const target = computed(() => props.game || singleAndGalgameGuides.value.find(game => game.id === selectedGame.value));
const saved = computed(() => target.value ? linksForGames(gameSourceIds(target.value)).find(link => link.url === props.link.url) : undefined);
const article = ref<ResourceArticle>();
const busy = ref(false), saving = ref(false), error = ref(''), notice = ref('');
const version = ref(''), notes = ref('');
const qrInput = ref<HTMLInputElement>();
const scanning = ref(false);
const parsed = computed(() => article.value ? parseObsidianGuide(article.value.markdown) : undefined);
let generation = 0;
onBeforeUnmount(() => { generation++; });
async function load() {
  const token = ++generation, url = props.link.url;
  busy.value = true; error.value = '';
  try { const result = await readResourceArticle(url); if (token === generation) article.value = result; }
  catch (cause) { if (token === generation) error.value = String(cause); }
  finally { if (token === generation) busy.value = false; }
}
watch(() => [props.link.url, props.game?.id], () => {
  generation++; article.value = saved.value?.article || props.link.article;
  version.value = saved.value?.version || props.link.version || ''; notes.value = saved.value?.notes || props.link.notes || '';
  error.value = ''; notice.value = ''; busy.value = false;
  if (!article.value) void load();
}, { immediate: true });
async function save(body: boolean) {
  if (!target.value || saving.value) return;
  saving.value = true; error.value = ''; notice.value = '';
  try {
    await saveGameLink(target.value.record_id || target.value.id, { ...props.link, ...saved.value, ...(body && article.value ? { article: article.value } : {}), version: version.value, notes: notes.value, saved_at: new Date().toISOString() });
    notice.value = body ? '正文已保存，可以离线阅读；原文配图仍需联网。' : '来源已收录到当前游戏。';
    emit('saved');
  } catch (cause) { error.value = String(cause); }
  finally { saving.value = false; }
}
async function exportMarkdown() {
  if (!target.value || !article.value || saving.value) return;
  const game = target.value, body = { ...article.value }, link = { ...props.link, ...saved.value };
  const token = generation;
  saving.value = true; error.value = ''; notice.value = '';
  let written = '';
  try {
    const name = `${game.title} - ${body.title || '攻略'}`.replace(/[<>:"/\\|?*\x00-\x1f]/g, '_').slice(0, 110).replace(/[. ]+$/, '') + '.md';
    const directory = game.file_path ? game.file_path.replace(/[\\/][^\\/]+$/, '') : obsidianVaultPath.value;
    const initial = link.markdown_path || (directory ? `${directory.replace(/[\\/]+$/, '')}\\${name}` : name);
    const path = await pickSaveFile('保存攻略 Markdown（可选择笔记库目录）', 'Markdown (*.md)|*.md', initial);
    if (!path) return;
    written = await writeResourceMarkdown(path, body, game.title);
    await saveGameLink(game.record_id || game.id, { ...link, article: body, markdown_path: written, saved_at: new Date().toISOString() });
    emit('saved');
    if (token === generation) notice.value = `Markdown 已保存：${written}`;
  } catch (cause) { if (token === generation) error.value = `${written ? `文件已写入 ${written}，但资料关联失败：` : ''}${String(cause)}`; }
  finally { saving.value = false; }
}
async function web(url = props.link.url) {
  try { await openResourceWeb(url, target.value?.title || props.link.title, target.value?.record_id || target.value?.id || '', props.link.url); }
  catch (cause) { error.value = String(cause); }
}
async function external() { try { await openExternalUrl(props.link.url); } catch (cause) { error.value = String(cause); } }
async function current() {
  try { const url = await currentResourceWeb(); emit('navigate', { title: new URL(url).hostname + ' · 当前网页', site: new URL(url).hostname, role: props.link.role, url }); }
  catch (cause) { error.value = String(cause); }
}
function follow(event: MouseEvent) {
  const anchor = (event.target as HTMLElement).closest('a');
  if (!anchor) return;
  event.preventDefault();
  const url = anchor.getAttribute('href') || '';
  if (/^https?:\/\//i.test(url)) void web(url);
}
async function attachFile() {
  if (!target.value) return;
  try {
    const path = await pickOpenFile('关联已下载的游戏资料', '所有文件 (*.*)|*.*');
    if (!path) return;
    await saveGameLink(target.value.record_id || target.value.id, { ...props.link, ...saved.value, version: version.value, notes: notes.value, local_files: [...new Set([...(saved.value?.local_files || []), path])] });
  } catch (cause) { error.value = String(cause); }
}
async function openFile(path: string) { try { await openInExplorer(path); } catch (cause) { error.value = String(cause); } }
async function scanQr(event: Event) {
  const input = event.target as HTMLInputElement, file = input.files?.[0];
  input.value = '';
  if (!file || scanning.value) return;
  const token = generation;
  scanning.value = true; error.value = '';
  let bitmap: ImageBitmap | undefined;
  try {
    if (file.size > 10 * 1024 * 1024) throw new Error('二维码图片请控制在 10 MB 以内。');
    bitmap = await createImageBitmap(file);
    const ratio = Math.min(1, 1600 / Math.max(bitmap.width, bitmap.height));
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(bitmap.width * ratio)); canvas.height = Math.max(1, Math.round(bitmap.height * ratio));
    const context = canvas.getContext('2d');
    if (!context) throw new Error('无法读取图片，请重试。');
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
    const { default: jsQR } = await import('jsqr');
    const result = jsQR(pixels.data, pixels.width, pixels.height);
    if (!result) throw new Error('未识别到二维码，请选择清晰、完整的二维码图片。');
    let url: URL;
    try { url = new URL(result.data); } catch { throw new Error('二维码中没有网页链接。'); }
    if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) throw new Error('二维码中不是可打开的网页链接。');
    if (token === generation) emit('navigate', { title: `${url.hostname} · 二维码链接`, url: url.href, site: url.hostname, role: props.link.role });
  } catch (cause) { if (token === generation) error.value = String(cause); }
  finally { bitmap?.close(); scanning.value = false; }
}
function read() { if (target.value && saved.value?.article) void handleOpenResourceReader(target.value, saved.value); }
</script>
<template>
  <article class="resource-preview">
    <header><p>{{ saved?.article ? '已保存正文 · 可离线阅读' : '正文预览' }}<span>{{ link.site }}</span></p><h3>{{ article?.title || link.title }}</h3><small>{{ link.url }}</small></header>
    <div class="resource-tools"><button @click="web()"><BookOpen />应用内原网页</button><button @click="current">读取浏览中的网页</button><button :disabled="scanning" @click="qrInput?.click()"><ScanLine />{{ scanning ? '识别中…' : '识别二维码图片' }}</button><input ref="qrInput" hidden type="file" accept="image/*" aria-label="二维码图片" @change="scanQr" /><button aria-label="在外部浏览器打开" @click="external"><ExternalLink /></button><button :disabled="busy" aria-label="重新读取正文" @click="load"><RotateCw /></button></div>
    <p v-if="busy" class="resource-message" role="status">正在读取正文…</p><p v-if="error" class="resource-error" role="alert">{{ error }}</p>
    <div v-if="parsed" class="resource-body wp-guide-prose" @click="follow">
      <p v-if="parsed.overview" class="resource-paragraph">{{ parsed.overview }}</p>
      <template v-for="(block,index) in parsed.blocks" :key="index">
        <h4 v-if="block.type === 'heading'">{{ block.text }}</h4>
        <div v-else-if="block.table" class="resource-table"><table><thead><tr><th v-for="(cell,i) in block.table.headers" :key="i" v-html="formatGuideInline(cell)" /></tr></thead><tbody><tr v-for="(row,i) in block.table.rows" :key="i"><td v-for="(cell,j) in row" :key="j" v-html="formatGuideInline(cell)" /></tr></tbody></table></div>
        <ul v-else-if="block.items"><li v-for="(item,i) in block.items" :key="i" v-html="formatGuideInline(item)" /></ul>
        <div v-else-if="block.lines"><p v-for="(line,i) in block.lines" :key="i" v-html="line.formattedHtml" /></div>
        <p v-else class="resource-paragraph" v-html="formatGuideInline(block.text || '')" />
      </template>
    </div>
    <p v-else-if="!busy" class="resource-message">动态页面、登录和网盘操作可使用“应用内原网页”；链接仍可收录到本作。</p>
    <JuijDownloads v-if="isJuij" :article="link.url" :game-id="target?.record_id || target?.id" :game-title="target?.title" />
    <section v-if="link.role === 'patch' || link.role === 'resource'" class="resource-install-info"><label>适用版本<input v-model="version" placeholder="记录平台、游戏版本或补丁版本" /></label><label>安装备忘<textarea v-model="notes" rows="2" placeholder="作者说明、依赖或自己的安装备注" /></label><button v-for="path in saved?.local_files || []" :key="path" @click="openFile(path)"><FolderOpen />{{ path.split(/[\\/]/).pop() }}</button><button v-if="target" @click="attachFile">关联已下载文件</button><small>下载前选择保存位置；完成后记录文件路径，解压和安装由你处理。</small></section>
    <footer><label v-if="!game">收录到<select v-model="selectedGame" aria-label="选择游戏"><option value="">选择游戏</option><option v-for="item in singleAndGalgameGuides" :key="item.id" :value="item.id">{{ item.title }}</option></select></label><div><button v-if="article" class="resource-save" :disabled="!target || saving" @click="save(true)"><BookmarkPlus />{{ saving ? '保存中…' : saved?.article ? '更新已存正文' : '保存正文到本作' }}</button><button v-if="article" :disabled="!target || saving" @click="exportMarkdown">保存为 Markdown…</button><button :disabled="!target || saving" @click="save(false)">{{ saved ? '保存资料信息' : '仅收藏来源' }}</button><button v-if="saved?.article" @click="read"><BookOpen />打开攻略阅读器</button></div><p v-if="notice" role="status">{{ notice }}</p><p v-if="saved?.markdown_path" class="resource-path">Markdown：{{ saved.markdown_path }}<button @click="openFile(saved.markdown_path.replace(/[\\/][^\\/]+$/, ''))"><FolderOpen />打开所在目录</button></p><small v-if="article">Markdown 保留标题、表格、链接和配图地址；保存位置由你选择，配图需联网。</small></footer>
  </article>
</template>
<style scoped>
.resource-preview{min-width:0}header>p{display:flex;gap:12px;justify-content:space-between;font-size:11px;color:var(--color-beacon)}header>p span{color:var(--color-faint)}h3{font:23px var(--font-display);line-height:1.5;margin:14px 0 7px}header>small{display:block;color:var(--color-faint);font-size:10px;overflow-wrap:anywhere}.resource-tools{display:flex;gap:8px;flex-wrap:wrap;margin:16px 0}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;font-size:11px;color:var(--color-beacon);padding:7px 9px;border-radius:4px;cursor:pointer;min-height:32px}button:hover{background:var(--color-paper)}button:disabled{opacity:.4;cursor:default}svg{width:14px;height:14px;flex-shrink:0}.resource-message,.resource-error{font-size:12px;line-height:1.8;margin:16px 0;overflow-wrap:anywhere;color:var(--color-muted)}.resource-error{color:var(--color-alert)}.resource-body{max-height:560px;overflow:auto;padding-right:14px;font-size:13px;line-height:1.95;color:var(--color-muted);user-select:text;scrollbar-gutter:stable}.resource-body h4{font-size:16px;font-weight:500;color:var(--color-ink);margin:22px 0 12px}.resource-paragraph{white-space:pre-wrap;margin:12px 0}.resource-body :deep(img){max-width:100%;height:auto}.resource-table{overflow:auto;margin:16px 0}.resource-table table{width:100%;font-size:12px;border-collapse:collapse}.resource-table :is(th,td){border:1px solid var(--color-hairline);padding:8px;text-align:left;min-width:160px;vertical-align:top;overflow-wrap:anywhere}.resource-body ul{padding-left:20px}.resource-install-info{border-top:1px solid var(--color-hairline);margin-top:20px;padding-top:15px;display:flex;gap:12px;flex-wrap:wrap}.resource-install-info label{flex:1 1 180px}.resource-install-info small{width:100%;font-size:11px;color:var(--color-muted)}label{display:flex;flex-direction:column;gap:8px;font-size:11px;color:var(--color-muted)}input,textarea,select{background:var(--color-paper);border:1px solid var(--color-hairline);padding:9px;color:var(--color-ink);font-size:12px;min-width:0;border-radius:4px}footer{border-top:1px solid var(--color-hairline);padding-top:18px;margin-top:20px}footer>div{display:flex;gap:8px;flex-wrap:wrap;margin:10px 0}.resource-save{background:var(--color-beacon);color:var(--color-canvas)}footer p,footer small{display:block;font-size:11px;color:var(--color-muted);line-height:1.8}footer p{color:var(--color-beacon);margin:10px 0}:where(button,input,textarea,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
