<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Check, Search, X } from '@lucide/vue';
import { gameRecords } from '../workspace/core';
import { cancelMetadataQueue, enrichImportedGames, metadataBusyIds, metadataJob, metadataSource, searchRecordMetadata, showMetadataBatch } from '../workspace/guides/local';
import { posterSrc } from './sky/wikiImage';
import type { GameMetadata } from '../types';
import { bangumiConnected, connectBangumi } from '../services/localGames';
import { openExternalUrl } from '../services/systemApi';
import GameMetadataChoice from './GameMetadataChoice.vue';
import { openGameRecordEditor, singleAndGalgameGuides } from '../workspace/guides/library';

const dialog = ref<HTMLDialogElement>();
const scope = ref('pending');
const choice = ref<{ id: string; candidate: GameMetadata }>();
function editCover(id: string) {
  const game = singleAndGalgameGuides.value.find(item => (item.record_id || item.id) === id);
  if (game) { showMetadataBatch.value = false; openGameRecordEditor(game, 'cover'); }
}
const queries = ref<Record<string, string>>({});
const errors = ref<Record<string, string>>({});
const searching = ref<string[]>([]);
const connected = ref(false);
const token = ref('');
const connecting = ref(false);
const connectError = ref('');
const records = computed(() => gameRecords.value);
const pending = computed(() => records.value.filter(r => !r.library?.metadata || r.library.matching));
const visible = computed(() => scope.value === 'pending' ? pending.value : records.value);
const busy = (id: string) => metadataBusyIds.value.includes(id) || searching.value.includes(id);
watch(showMetadataBatch, async open => {
  await nextTick();
  if (open) { dialog.value?.showModal(); connected.value = await bangumiConnected().catch(() => false); }
  else { dialog.value?.close(); token.value = ''; }
});
async function connect() {
  connecting.value = true; connectError.value = '';
  try { await connectBangumi(token.value); connected.value = true; token.value = ''; }
  catch (e) { connectError.value = String(e); }
  finally { connecting.value = false; }
}
function run() {
  void enrichImportedGames(visible.value.map(r => ({ id: r.id, title: queries.value[r.id] || r.library?.matching?.query || r.title })), metadataSource.value, true);
}
async function search(id: string, title: string) {
  searching.value.push(id); errors.value[id] = '';
  try { await searchRecordMetadata(id, metadataSource.value, queries.value[id] || title); }
  catch (e) { errors.value[id] = String(e); }
  finally { searching.value = searching.value.filter(value => value !== id); }
}
</script>

<template>
  <dialog ref="dialog" class="metadata-batch" aria-labelledby="metadata-batch-title" @cancel.prevent="showMetadataBatch = false">
    <header><div><p>LIBRARY CURATION</p><h2 id="metadata-batch-title">一次整理，整库就绪</h2><span>别名与完整副标题自动识别；只在有歧义时确认。个人标题、评分与进度会保留。</span></div><button aria-label="关闭批量资料" @click="showMetadataBatch = false"><X /></button></header>
    <div class="batch-toolbar">
      <nav><button :aria-pressed="scope === 'pending'" @click="scope = 'pending'">待确认 {{ pending.length }}</button><button :aria-pressed="scope === 'all'" @click="scope = 'all'">全部档案 {{ records.length }}</button></nav>
      <select v-model="metadataSource" aria-label="批量资料来源" :disabled="metadataJob.running"><option value="bangumi">Bangumi 优先 · VNDB 补充</option><option value="steam">Steam</option><option value="vndb">VNDB</option></select>
      <button class="primary" :disabled="metadataJob.running || !visible.length || metadataBusyIds.length > 0" @click="run">{{ scope === 'all' ? '批量补全 / 更新' : '自动匹配待确认' }}</button>
    </div>
    <div v-if="metadataJob.total" class="batch-progress" role="status"><span>{{ metadataJob.running ? '正在匹配与补充媒体' : metadataJob.done < metadataJob.total ? '已停止，已保存的结果保留' : '本次匹配结束' }} · {{ metadataJob.done }} / {{ metadataJob.total }} · 已匹配 {{ metadataJob.matched }} · 待确认 {{ metadataJob.pending }}</span><button v-if="metadataJob.running" @click="cancelMetadataQueue">停止</button><progress :value="metadataJob.done" :max="metadataJob.total" /><p v-if="metadataJob.error" role="alert">{{ metadataJob.error }}</p></div>
    <div class="batch-body">
      <details v-if="metadataSource === 'bangumi'" class="batch-connection"><summary>{{ connected ? 'Bangumi 已连接 · 令牌仅保存在本机' : '连接 Bangumi：未登录搜索会缺少部分 Galgame 条目' }}</summary><p>公开条目无需登录。受限作品需要有访问权限的 Bangumi 账号；令牌只发送给 Bangumi API，保存在 Windows 凭据管理器，不进入备份。</p><a href="https://next.bgm.tv/demo/access-token" @click.prevent="openExternalUrl('https://next.bgm.tv/demo/access-token')">打开 Bangumi 令牌页面</a><form @submit.prevent="connect"><input v-model="token" type="password" autocomplete="off" aria-label="Bangumi Access Token" placeholder="粘贴 Access Token" :disabled="connecting" /><button :disabled="connecting || !token.trim()">{{ connecting ? '验证中…' : '验证并连接' }}</button></form><p v-if="connectError" role="alert">{{ connectError }}</p></details>
      <div v-if="!visible.length" class="batch-empty"><Check /><h3>{{ metadataJob.running ? '资料正在后台补全' : '没有需要逐个处理的作品' }}</h3><p>“全部档案”中可更新已有资料；只有攻略的作品也能在详情中直接匹配。</p></div>
      <article v-for="record in visible" :key="record.id" class="batch-record">
        <div class="batch-record-heading"><div><strong>{{ record.title }}</strong><span>{{ busy(record.id) ? '正在获取资料与媒体…' : record.library?.metadata ? `已关联 ${record.library.metadata.source.toUpperCase()} · ${record.library.metadata.title}` : '等待匹配' }}</span></div><button v-if="record.library?.metadata" @click="editCover(record.id)">更换封面</button></div>
        <form @submit.prevent="search(record.id, record.library?.matching?.query || record.title)"><input :value="queries[record.id] ?? record.library?.matching?.query ?? record.title" @input="queries[record.id] = ($event.target as HTMLInputElement).value" :aria-label="'匹配名称：' + record.title" :disabled="busy(record.id)" /><button :disabled="busy(record.id) || metadataJob.running"><Search />搜索候选</button></form>
        <p v-if="errors[record.id] || record.library?.matching?.error" class="batch-message" role="status">{{ errors[record.id] || record.library?.matching?.error }}</p>
        <GameMetadataChoice v-if="choice?.id === record.id" :key="choice.candidate.source + choice.candidate.source_id" :game-id="record.id" :candidate="choice.candidate" @cancel="choice = undefined" @saved="choice = undefined" /><div v-else-if="record.library?.matching?.candidates.length" class="batch-candidates">
          <button v-for="candidate in record.library.matching.candidates" :key="candidate.source + candidate.source_id" :disabled="busy(record.id) || metadataJob.running" @click="choice = { id: record.id, candidate }">
            <img v-if="candidate.cover" :src="posterSrc(candidate.cover)" alt="" loading="lazy" /><div><strong>{{ candidate.title }}</strong><span>{{ candidate.original_title }}</span><small>{{ candidate.release_date }} · {{ candidate.developer || candidate.source.toUpperCase() }}</small><b>选择资料与封面</b></div>
          </button>
        </div>
        <p v-if="record.library?.metadata?.warnings?.length" class="batch-message">{{ record.library.metadata.warnings.join('；') }}</p>
      </article>
    </div>
    <footer>关闭窗口后继续匹配；结果和待确认候选会保存。<button @click="showMetadataBatch = false">回到游戏库</button></footer>
  </dialog>
</template>

<style scoped>
.batch-connection{font-size:12px;padding:18px 0;border-bottom:1px solid var(--color-hairline);color:var(--color-muted);line-height:1.8}.batch-connection summary{cursor:pointer;color:var(--color-beacon)}.batch-connection a{display:inline-block;color:var(--color-beacon);margin:10px 0}.batch-connection p{margin-top:10px}
.metadata-batch{width:min(1120px,94vw);max-height:90dvh;margin:auto;padding:0;background:var(--color-canvas);color:var(--color-ink);border:1px solid var(--color-hairline);border-radius:8px;box-shadow:0 30px 100px #0009}.metadata-batch[open]{display:flex;flex-direction:column}.metadata-batch::backdrop{background:#080d10cc;backdrop-filter:blur(8px)}header{display:flex;justify-content:space-between;align-items:start;padding:26px 30px}header p{font:10px var(--font-mono);letter-spacing:.16em;color:var(--color-beacon)}h2{font:30px var(--font-display);margin:8px 0}header span,footer{font-size:12px;color:var(--color-muted)}button{display:inline-flex;gap:8px;align-items:center;justify-content:center;padding:9px 12px;border:1px solid var(--color-hairline);border-radius:4px;cursor:pointer;font-size:12px}button:hover{background:var(--color-paper)}button:disabled{opacity:.45;cursor:default}svg{width:16px;height:16px;flex-shrink:0}.batch-toolbar{display:flex;gap:12px;align-items:center;flex-wrap:wrap;padding:16px 30px;border-block:1px solid var(--color-hairline)}nav{display:flex;gap:8px;margin-right:auto}nav [aria-pressed=true]{color:var(--color-beacon);border-color:var(--color-beacon)}input,select{min-width:0;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper);padding:9px 12px;font-size:12px;color:var(--color-ink)}.primary{background:var(--color-beacon);color:var(--color-canvas)}.batch-progress{display:flex;flex-wrap:wrap;align-items:center;gap:12px;padding:12px 30px;font-size:12px;color:var(--color-beacon)}progress{width:100%;height:3px;accent-color:var(--color-beacon)}.batch-body{overflow:auto;min-height:0;padding:0 30px}.batch-record{padding:24px 0;border-bottom:1px solid var(--color-hairline)}.batch-record-heading{display:flex;align-items:center;justify-content:space-between;margin-bottom:14px}.batch-record-heading strong{font-size:17px;font-weight:500}.batch-record-heading span{display:block;font-size:11px;color:var(--color-muted);margin-top:5px}.batch-record-heading>svg{color:var(--color-beacon)}form{display:flex;gap:10px}input{flex:1}.batch-message{font-size:11px;color:var(--color-muted);line-height:1.8;margin-top:10px;overflow-wrap:anywhere}.batch-candidates{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:12px;margin-top:16px}.batch-candidates button{align-items:start;text-align:left;padding:12px;justify-content:start}.batch-candidates img{width:62px;height:88px;object-fit:cover;flex-shrink:0}.batch-candidates strong{font-size:13px;font-weight:500}.batch-candidates span,.batch-candidates small{display:block;color:var(--color-muted);font-size:10px;margin-top:5px;overflow-wrap:anywhere}.batch-candidates b{display:block;font-weight:500;color:var(--color-beacon);margin-top:10px}.batch-empty{padding:56px 10px;text-align:center;color:var(--color-muted)}.batch-empty>svg{width:30px;height:30px;margin:0 auto 15px;color:var(--color-beacon)}h3{font:23px var(--font-display);margin-bottom:12px}.batch-empty p{font-size:12px}footer{display:flex;gap:20px;align-items:center;justify-content:space-between;padding:16px 30px;border-top:1px solid var(--color-hairline)}:where(button,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:850px){.batch-candidates{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:600px){.batch-candidates{grid-template-columns:1fr}header,.batch-toolbar,footer{padding:16px}.batch-body{padding:0 16px}footer{flex-wrap:wrap}}
</style>
