<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ExternalLink, Music2, Play, Plus, Search } from '@lucide/vue';
import type { GameAlbum, GameExtraMedia, GameMetadata, GameRecord } from '../types';
import { loadGameAlbum } from '../services/localGames';
import { openExternalUrl } from '../services/systemApi';
import { updateLibraryDetails } from '../workspace/guides/local';
import { posterSrc } from './sky/wikiImage';
import { mediaProvider, playableMedia as direct, searchGameVideos, type GameVideoSearchResult } from '../services/gameMedia';

const props = defineProps<{ record: GameRecord; metadata?: GameMetadata | null }>();
const active = ref('video');
const emit = defineEmits<{ play: [item: GameExtraMedia]; background: [] }>();
const albumId = ref('');
const albums = ref<Record<string, GameAlbum>>({});
const albumBusy = ref(false);
const error = ref('');
const add = ref(false);
const mediaTitle = ref('');
const mediaUrl = ref('');
const mediaKind = ref<GameExtraMedia['kind']>('audio');
const saving = ref(false);
const failed = ref<Record<string, boolean>>({});
const videos = computed(() => props.metadata?.videos || []);
const videoQuery = ref(`${props.record.title} OP`);
const videoResults = ref<GameVideoSearchResult[]>([]);
const videoSearching = ref(false), videoSearched = ref(false);
const videoSearchError = ref('');
let videoSearchGeneration = 0;
onBeforeUnmount(() => { videoSearchGeneration++; });
async function searchVideos() {
  if (videoSearching.value || !videoQuery.value.trim()) return;
  const ticket = ++videoSearchGeneration;
  videoSearching.value = true; videoSearched.value = false; videoSearchError.value = ''; videoResults.value = [];
  try {
    const results = await searchGameVideos(videoQuery.value.trim());
    if (ticket === videoSearchGeneration) { videoResults.value = results; videoSearched.value = true; }
  } catch (e) { if (ticket === videoSearchGeneration) videoSearchError.value = String(e); }
  finally { if (ticket === videoSearchGeneration) videoSearching.value = false; }
}
async function addVideo(item: GameVideoSearchResult) {
  if (saving.value) return;
  saving.value = true; error.value = '';
  try {
    await updateLibraryDetails(props.record.id, lib => ({ ...lib, extra_media: [
      ...(lib.extra_media || []).filter(m => m.url !== item.url), {kind:'video',title:item.title,url:item.url},
    ] }));
  } catch (e) { error.value = String(e); }
  finally { saving.value = false; }
}
const music = computed(() => props.metadata?.music || []);
const shots = computed(() => props.metadata?.screenshots || []);
const extra = computed(() => props.record.library?.extra_media || []);
const album = computed(() => albums.value[albumId.value]);
watch(() => props.record.id, () => { albumId.value = ''; error.value = ''; failed.value = {}; });
watch(active, () => { error.value = ''; });
async function open(url: string) { try { await openExternalUrl(url); } catch (e) { error.value = String(e); } }
function play(item: GameExtraMedia) {
  error.value = '';
  emit('play', item);
}
async function showAlbum(id: string) {
  albumId.value = id; error.value = '';
  if (albums.value[id]) return;
  albumBusy.value = true;
  try { albums.value[id] = await loadGameAlbum(id); }
  catch (e) { error.value = String(e); }
  finally { albumBusy.value = false; }
}

async function saveMedia() {
  saving.value = true; error.value = '';
  try {
    const url = new URL(mediaUrl.value.trim());
    if (url.protocol !== 'https:' || url.username || url.password) throw new Error('请填写不含登录凭据的 HTTPS 媒体地址。');
    const item: GameExtraMedia = { kind: mediaKind.value, title: mediaTitle.value.trim() || (mediaKind.value === 'audio' ? '音乐' : '宣传片'), url: url.href };
    await updateLibraryDetails(props.record.id, lib => ({ ...lib, extra_media: [...(lib.extra_media || []).filter(m => m.url !== item.url), item] }));
    add.value = false; mediaTitle.value = ''; mediaUrl.value = '';
  } catch (e) { error.value = String(e); }
  finally { saving.value = false; }
}
</script>

<template>
  <section class="game-media">
    <div class="media-heading"><div><p>PICTURES, MOTION & SOUND</p><h2>作品的另一面</h2></div><button @click="add = !add"><Plus />添加媒体链接</button></div>
    <nav aria-label="媒体分类"><button :aria-pressed="active === 'video'" @click="active = 'video'"><Play />宣传片 <span>{{ videos.length + extra.filter(m => m.kind === 'video').length }}</span></button><button :aria-pressed="active === 'music'" @click="active = 'music'"><Music2 />音乐 <span>{{ music.length + extra.filter(m => m.kind === 'audio').length }}</span></button><button :aria-pressed="active === 'images'" @click="active = 'images'">画面 <span>{{ shots.length }}</span></button></nav>
    <form v-if="add" class="media-add" @submit.prevent="saveMedia"><select v-model="mediaKind" aria-label="媒体类型"><option value="audio">音乐</option><option value="video">宣传片</option></select><input v-model="mediaTitle" aria-label="媒体名称" placeholder="名称" /><input v-model="mediaUrl" type="url" required aria-label="媒体地址" placeholder="HTTPS 音视频直链或播放页面" /><button :disabled="saving">{{ saving ? '保存中…' : '保存链接' }}</button><p>音视频直链在头部播放；Bilibili、YouTube 等视频页面使用应用内网页播放器。</p></form>
    <p v-if="error" class="media-error" role="alert">{{ error }}</p>
    <template v-if="active === 'video'">
      <p v-if="!videos.length" class="media-muted">暂未取得 Steam 或游戏官网的宣传片。可更新资料重试，或在下方搜索并选择 OP / PV。</p>
      <div class="video-grid"><article v-for="video in videos" :key="video.url"><button class="video-preview" :aria-label="'播放宣传片：' + video.title" @click="play({ kind: 'video', title: video.title, url: video.url })"><img v-if="video.thumbnail" :src="posterSrc(video.thumbnail)" alt="" loading="lazy" /><span><Play /></span></button><h3>{{ video.title }}</h3><a :href="video.page_url" @click.prevent="open(video.page_url)">{{ mediaProvider(video.page_url) }} <ExternalLink /></a></article></div>
      <section class="video-search" aria-label="在线搜索视频"><h3>寻找这部作品的 OP / PV</h3><p class="media-muted">Bilibili 搜索结果可能包含二创、合集或同名作品。先预览，再添加到当前作品；网页播放器的可用性取决于来源。</p>
        <form class="video-search-form" @submit.prevent="searchVideos"><input v-model="videoQuery" aria-label="视频搜索关键词" placeholder="作品名 + OP / PV" :disabled="videoSearching" /><button :disabled="videoSearching || !videoQuery.trim()"><Search />{{ videoSearching ? '搜索中…' : '搜索 Bilibili' }}</button><button type="button" @click="open('https://www.youtube.com/results?search_query=' + encodeURIComponent(videoQuery))">YouTube 搜索 <ExternalLink /></button></form>
        <p v-if="videoSearchError" role="alert" class="media-error">{{ videoSearchError }} <button @click="open('https://search.bilibili.com/all?keyword=' + encodeURIComponent(videoQuery))">打开 Bilibili 搜索</button></p><p v-if="videoSearched && !videoResults.length" role="status" class="media-muted">没有找到结果，试试原名或更准确的副标题。</p>
        <div class="video-search-results"><article v-for="item in videoResults" :key="item.url"><button class="video-preview" :aria-label="'预览搜索视频：' + item.title" @click="play({kind:'video',title:item.title,url:item.url})"><img :src="posterSrc(item.thumbnail)" alt="" loading="lazy" /><span><Play /></span></button><h3>{{ item.title }}</h3><p>{{ item.author }} · {{ item.duration }}</p><button :disabled="saving || extra.some(m => m.url === item.url)" @click="addVideo(item)">{{ extra.some(m => m.url === item.url) ? '已加入作品' : '加入作品视频' }}</button><button @click="open(item.page_url)">打开来源 <ExternalLink /></button></article></div>
      </section>
    </template>
    <template v-else-if="active === 'music'">
      <p class="media-muted">原声集、片头曲和片尾曲由 Bangumi 关联；可查看专辑与曲目。Bangumi 不提供音频，添加音源后可播放。</p>
      <div class="album-grid"><button v-for="item in music" :key="item.id" :aria-pressed="albumId === item.id" :disabled="albumBusy" @click="showAlbum(item.id)"><img v-if="item.cover" :src="posterSrc(item.cover)" alt="" loading="lazy" /><span><small>{{ item.relation }}</small><strong>{{ item.title }}</strong><b>查看曲目</b></span></button></div>
      <p v-if="!music.length" class="media-muted">此作品暂未取得关联音乐。可刷新 Bangumi 资料，或自行添加音源。</p>
      <section v-if="albumId" class="album-detail"><div class="album-title"><h3>{{ music.find(m => m.id === albumId)?.title }}</h3><a :href="music.find(m => m.id === albumId)?.url" @click.prevent="open(music.find(m => m.id === albumId)!.url)">专辑来源 <ExternalLink /></a></div><p v-if="albumBusy">正在获取曲目…</p><template v-else-if="album"><div class="album-info"><span v-for="info in album.info.filter(i => ['艺术家','作曲','作词','编曲','发售日期','发行商'].includes(i.label))" :key="info.label">{{ info.label }} · {{ info.value }}</span></div><ol><li v-for="(track,index) in album.tracks" :key="index"><small>{{ track.disc ? `CD${track.disc} · ` : '' }}{{ track.number }}</small><span>{{ track.title || '未命名曲目' }}</span><time>{{ track.duration }}</time></li></ol><p v-if="!album.tracks.length">来源尚未收录曲目。</p></template></section>
    </template>
    <template v-else>
      <div class="image-heading"><p class="media-muted">点击查看原图；在作品编辑中选择详情背景。</p><button @click="emit('background')">背景设置与本地图片</button><button v-if="Object.keys(failed).length" @click="failed = {}">重试图片</button></div>
      <p v-if="!shots.length" class="media-muted">尚未取得游戏画面。刷新资料可重试已关联的媒体来源。</p>
      <div class="shot-grid"><figure v-for="url in shots" :key="url"><a :href="url" @click.prevent="open(url)"><img v-if="!failed[url]" :src="posterSrc(url)" @error="failed[url] = true" alt="游戏画面，点击查看原图" loading="lazy" /><span v-else>图片未加载 · 打开原图</span></a></figure></div>
    </template>
    <div v-if="active !== 'images'" class="extra-media"><button v-for="item in extra.filter(m => m.kind === (active === 'music' ? 'audio' : 'video'))" :key="item.url" @click="play(item)"><Music2 v-if="item.kind === 'audio'" /><Play v-else /><span>{{ item.title }}<small>{{ direct(item.kind,item.url) ? '在头部播放' : '应用内播放页面' }}</small></span><ExternalLink v-if="!direct(item.kind,item.url)" /></button></div>
    <details v-if="metadata?.warnings?.length" class="media-warnings"><summary>部分媒体暂未取得 · 可在资料页刷新</summary><p v-for="warning in metadata.warnings" :key="warning">{{ warning }}</p></details>
  </section>
</template>

<style scoped>
.video-search{margin-top:28px;padding-top:20px;border-top:1px solid var(--color-hairline)}.video-search-form{display:flex;gap:8px;flex-wrap:wrap}.video-search-form input{flex:1;min-width:160px}.video-search-results{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:20px;margin-top:20px}.video-search-results article{min-width:0}.video-search-results h3{line-height:1.6;overflow-wrap:anywhere}.video-search-results p{font-size:11px;color:var(--color-muted);margin-bottom:10px}.video-search-results article>button:not(.video-preview){margin:0 6px 6px 0;font-size:11px}@media(max-width:650px){.video-search-results{grid-template-columns:1fr}}
.media-heading,.album-title,.image-heading{display:flex;justify-content:space-between;align-items:center;gap:16px;flex-wrap:wrap}.media-heading p{font:10px var(--font-mono);letter-spacing:.15em;color:var(--color-beacon)}h2{font:29px var(--font-display);margin-top:8px}h3{font-size:14px;font-weight:500;margin:12px 0}button{display:inline-flex;align-items:center;gap:8px;justify-content:center;cursor:pointer;border:1px solid var(--color-hairline);padding:8px 12px;border-radius:4px;font-size:12px}button:hover{background:var(--color-paper)}button:disabled{opacity:.4}svg{width:15px;height:15px;flex-shrink:0}nav{display:flex;gap:12px;margin:28px 0 24px}nav button[aria-pressed=true]{background:var(--color-beacon);color:var(--color-canvas);border-color:var(--color-beacon)}nav span{font:11px var(--font-mono);opacity:.65}.media-muted,.media-warnings{font-size:12px;line-height:1.8;color:var(--color-muted);margin:16px 0}.media-add{display:flex;gap:10px;flex-wrap:wrap;padding:20px;background:var(--color-chrome);margin-bottom:24px}input,select{border:1px solid var(--color-hairline);background:var(--color-paper);color:var(--color-ink);padding:9px 12px;min-width:0;font-size:12px}input{flex:1}input[type=url]{flex:2}.media-add p{width:100%;font-size:11px;color:var(--color-muted)}.media-error{color:var(--color-alert);font-size:12px;margin:14px 0}.video-grid,.shot-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:24px}.video-preview{position:relative;width:100%;aspect-ratio:16/9;padding:0;overflow:hidden;background:#101817}.video-preview img,.shot-grid img{width:100%;height:100%;object-fit:cover}.video-preview>span{position:absolute;display:grid;place-items:center;width:50px;height:50px;border-radius:50%;background:#0009;color:#fff;border:1px solid #fff5}.video-preview svg{width:24px;height:24px}a{display:inline-flex;gap:7px;align-items:center;font-size:11px;color:var(--color-beacon)}.album-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:14px}.album-grid button{text-align:left;align-items:start;justify-content:start;padding:14px}.album-grid button[aria-pressed=true]{border-color:var(--color-beacon)}.album-grid img{width:66px;height:66px;object-fit:cover;flex-shrink:0}.album-grid small{font-size:10px;color:var(--color-beacon)}.album-grid strong{display:block;font-size:12px;font-weight:500;margin:5px 0;line-height:1.5}.album-grid b{font-size:10px;font-weight:400;color:var(--color-muted)}.album-detail{margin-top:26px;border-top:1px solid var(--color-hairline);padding-top:12px}.album-info{display:flex;gap:8px 20px;flex-wrap:wrap;font-size:11px;color:var(--color-muted);margin-bottom:18px}.album-detail li{display:flex;align-items:center;gap:18px;padding:12px 0;border-bottom:1px solid var(--color-hairline);font-size:12px}.album-detail li small{min-width:65px;font-family:var(--font-mono);color:var(--color-faint)}.album-detail li span{flex:1}.album-detail time{font:11px var(--font-mono);color:var(--color-faint)}.shot-grid a{display:block;aspect-ratio:16/9;background:#131818;border-radius:4px;overflow:hidden}.extra-media{display:grid;gap:12px;margin-top:20px}.extra-media button{justify-content:start;text-align:left;padding:14px}.extra-media small{display:block;font-size:10px;color:var(--color-muted);margin-top:4px}.media-warnings{border-top:1px solid var(--color-hairline);padding-top:16px;font-size:11px;overflow-wrap:anywhere}.media-warnings summary{cursor:pointer}:where(button,input,select,a):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@media(max-width:900px){.album-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}@media(max-width:650px){.album-grid,.video-grid,.shot-grid{grid-template-columns:1fr}}
</style>
