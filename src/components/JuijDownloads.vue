<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';
import { Copy, ExternalLink, ArrowDownToLine, ChevronUp } from '@lucide/vue';
import { findJuijDownloads, resolveJuijDownload, juijFailure, type JuijDownload, type JuijDestination, type JuijFailure } from '../services/juijDownloads';
import { openResourceWeb } from '../services/systemApi';
import { showToast } from '../workspace/core';

const props = defineProps<{ article: string; gameId?: string; gameTitle?: string }>();
type Download = JuijDownload & { state: 'idle' | 'loading' | 'ready' | 'failed'; destination?: JuijDestination; error?: JuijFailure };
const expanded = ref(false);
const loading = ref(false);
const error = ref<JuijFailure>();
const downloads = ref<Download[]>([]);
const now = ref(Date.now());
const downloadRetryAt = ref(0);
let wakeup: ReturnType<typeof setTimeout> | undefined;
function blocked(failure?: JuijFailure) { return (failure?.retryAt || 0) > now.value; }
function remember(failure: JuijFailure) {
  now.value = Date.now();
  if (wakeup) clearTimeout(wakeup);
  const deadlines = [failure.retryAt || 0, downloadRetryAt.value, error.value?.retryAt || 0, ...downloads.value.map(d => d.error?.retryAt || 0)].filter(t => t > now.value);
  if (deadlines.length) wakeup = setTimeout(() => remember(failure), Math.min(Math.min(...deadlines) - now.value + 50, 2_147_483_647));
}
function retryTime(failure: JuijFailure) {
  return failure.retryAt ? `可于 ${new Date(failure.retryAt).toLocaleString()} 再次尝试；届时仍以站点响应为准。` : '';
}
let generation = 0;
watch(() => props.article, () => { generation++; expanded.value = false; loading.value = false; error.value = undefined; downloads.value = []; });
onBeforeUnmount(() => { generation++; if (wakeup) clearTimeout(wakeup); });

async function resolve(link: Download) {
  now.value = Date.now();
  if (link.state === 'loading' || blocked(link.error) || downloadRetryAt.value > now.value || link.error?.kind === 'verification') return;
  const token = generation;
  link.state = 'loading';
  link.error = undefined;
  link.destination = undefined;
  try {
    const destination = await resolveJuijDownload(link.page);
    if (token !== generation) return;
    link.destination = destination;
    link.state = 'ready';
  } catch (cause) {
    if (token !== generation) return;
    link.state = 'failed';
    link.error = juijFailure(cause);
    if (link.error.kind === 'rate_limited') downloadRetryAt.value = link.error.retryAt || 0;
    remember(link.error);
  }
}

async function toggle() {
  expanded.value = !expanded.value;
  now.value = Date.now();
  if (!expanded.value || downloads.value.length || loading.value || blocked(error.value) || error.value?.kind === 'verification') return;
  const token = generation;
  loading.value = true;
  error.value = undefined;
  try {
    const links = await findJuijDownloads(props.article);
    if (token !== generation) return;
    downloads.value = links.map(link => ({ ...link, state: 'idle' }));
    if (downloads.value.length === 1) void resolve(downloads.value[0]!);
  } catch (cause) {
    if (token === generation) { error.value = juijFailure(cause); remember(error.value); }
  } finally {
    if (token === generation) loading.value = false;
  }
}

async function copy(url: string) {
  try { await navigator.clipboard.writeText(url); showToast('已复制链接', '可以粘贴到浏览器中打开。'); }
  catch { showToast('复制失败', '可以选中下方地址手动复制。'); }
}
function open(url: string) {
  openResourceWeb(url, props.gameTitle || 'Juij 资料', props.gameId || '', props.article).catch(error => showToast('没打开', String(error)));
}

</script>

<template>
  <div class="juij-downloads">
    <button type="button" class="juij-toggle" :aria-expanded="expanded" @click="toggle"><ChevronUp v-if="expanded" /><ArrowDownToLine v-else />{{ expanded ? '收起下载链接' : '获取下载链接' }}</button>
    <div v-if="expanded" class="juij-body">
      <p v-if="loading" role="status">正在读取文章的下载入口…</p>
      <div v-else-if="error" role="alert">
        <p>{{ error.message }} {{ retryTime(error) }}</p>
        <button v-if="error.kind !== 'verification'" type="button" :disabled="blocked(error)" @click="expanded = false; toggle()">{{ blocked(error) ? '等待后重试' : '重试' }}</button>
        <button type="button" @click="open(article)">查看原文</button>
      </div>
      <template v-else>
        <div v-for="link in downloads" :key="link.page" class="juij-link">
          <div class="juij-link-heading"><strong>{{ link.label }}</strong><button v-if="(link.state === 'idle' || link.state === 'failed') && link.error?.kind !== 'verification'" type="button" :disabled="blocked(link.error) || downloadRetryAt > now" @click="resolve(link)">{{ blocked(link.error) || downloadRetryAt > now ? '等待后重试' : link.state === 'failed' ? '重试' : '获取链接' }}</button><span v-else-if="link.state === 'loading'" role="status">正在获取下载地址…</span></div>
          <p v-if="link.error" role="alert">{{ link.error.message }} {{ retryTime(link.error) }}</p>
          <div class="juij-actions"><button type="button" :disabled="downloadRetryAt > now" @click="open(link.page)"><ExternalLink />打开下载页</button></div>
          <template v-if="link.destination">
            <p>{{ link.destination.kind === 'share' ? '已取得网盘分享页，需要在网盘中继续下载。' : '已取得外部地址，是否可下载请以目标页面为准。' }}</p>
            <p class="juij-address">{{ link.destination.url }}</p>
            <div class="juij-actions"><button type="button" @click="open(link.destination!.url)"><ExternalLink />{{ link.destination.kind === 'share' ? '打开网盘分享页' : '打开外部地址' }}</button><button type="button" @click="copy(link.destination!.url)"><Copy />复制链接</button></div>
          </template>
        </div>
        <p class="juij-note">安装方法、版本与解压密码请查看原文。</p>
      </template>
    </div>
  </div>
</template>

<style scoped>
.juij-downloads { margin:0 0 10px; }
.juij-toggle, .juij-actions button { display:inline-flex; align-items:center; gap:6px; }
button { color:var(--color-beacon); font-size:11px; cursor:pointer; min-height:30px; }
button:hover { text-decoration:underline; }
button:disabled { color:var(--color-faint); cursor:not-allowed; text-decoration:none; }
button:focus-visible { outline:2px solid var(--color-beacon); outline-offset:3px; }
svg { width:13px; height:13px; flex-shrink:0; }
.juij-body { margin:7px 0; padding:10px 14px; border-left:2px solid var(--color-beacon); background:var(--color-paper); border-radius:0 4px 4px 0; }
p, .juij-link-heading { font-size:11px; color:var(--color-muted); line-height:1.8; overflow-wrap:anywhere; }
p > button { margin-left:10px; }
.juij-link + .juij-link { margin-top:12px; padding-top:12px; border-top:1px solid var(--color-hairline); }
.juij-link-heading { display:flex; align-items:center; justify-content:space-between; gap:8px; flex-wrap:wrap; }
.juij-link-heading strong { font-weight:500; }
.juij-link-heading span, .juij-note { color:var(--color-faint); }
.juij-address { margin-top:7px; user-select:text; }
.juij-actions { display:flex; flex-wrap:wrap; gap:8px 18px; margin-top:3px; }
.juij-note { margin-top:8px; font-size:10px; }
</style>
