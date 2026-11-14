<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { ArrowLeft, BookOpen, ExternalLink, FolderOpen, Maximize2, Pencil, Pin, Settings2 } from '@lucide/vue';
import { posterSrc } from './sky/wikiImage';
import { coverPosition, detailBackground } from '../services/gameCoverImage';
import { installationAvailable } from '../services/localGames';
import { openExternalUrl, openInExplorer } from '../services/systemApi';
import type { GameExtraMedia } from '../types';
import { gameRecords } from '../workspace/core';
import { detailGame, detailGameId, detailTab, detailResourceRole, ensureGameMedia, metadataBusyIds, openLibraryImport, updateLibraryDetails } from '../workspace/guides/local';
import { isGamePinned, openGameRecordEditor, togglePinGame, type GameEditorSection } from '../workspace/guides/library';
import { handleOpenGuideReader } from '../workspace/guides/reader';
import { getEffectiveGameStatus, setGameStatus } from '../workspace/guides/status';
import GameActivity from './GameActivity.vue';
import GameSaveBackups from './GameSaveBackups.vue';
import { loadGameActivity, openSteamImport, steamActivity } from '../workspace/gameActivity';
import { unlinkSteamGame } from '../services/gameActivity';
import GameLaunch from './GameLaunch.vue';
import GameScalingPanel from './GameScalingPanel.vue';
import GameMedia from './GameMedia.vue';
import GameAtmosphere from './GameAtmosphere.vue';
import ExplorePanel from './ExplorePanel.vue';
import GamePricePanel from './GamePricePanel.vue';
import MarketConnection from './MarketConnection.vue';
import { guideMarketGame } from '../services/discovery';
import { isCreditField, type GameTaxonomySelection } from '../services/gameTaxonomy';
const emit = defineEmits<{ back: []; filter: [selection: GameTaxonomySelection] }>();
const priceSettings = ref(false);
const atmosphere = ref<InstanceType<typeof GameAtmosphere>>();
const focused = ref(false);
const content = ref<HTMLElement>();
const tab = detailTab;
watch([tab, detailGameId], () => { if (content.value) content.value.scrollTop = 0; });
function playMedia(item: GameExtraMedia) { atmosphere.value?.play(item); content.value?.scrollTo({top:0,behavior:'smooth'}); }
const busy = ref(false);
const error = ref('');
const failedImages = ref<Record<string, boolean>>({});
const available = ref<Record<string, boolean>>({});
const checked = ref(false);
const record = computed(() => gameRecords.value.find(r => r.id === (detailGame.value?.record_id || detailGame.value?.id)));
const metadata = computed(() => record.value?.library?.metadata);
const marketGame = computed(() => detailGame.value ? guideMarketGame(detailGame.value, metadata.value) : undefined);
const installations = computed(() => record.value?.library?.installations || []);
const background = computed(() => detailBackground(record.value?.library, detailGame.value?.poster));
const steamLink = computed(() => steamActivity.value.links.find(l => l.record_id === record.value?.id));
const steamInstall = computed(() => steamActivity.value.installed.find(i => i.appid === steamLink.value?.appid));
const taxonomy = computed(() => detailGame.value?.taxonomy);
const tagsExpanded = ref(false);
const workInfo = computed(() => (metadata.value?.info || []).filter(i => !['中文名','别名','链接','英文名'].includes(i.label) && !isCreditField(i.label)));
const sources = computed(() => metadata.value?.sources?.length ? metadata.value.sources : metadata.value ? [metadata.value] : []);
const description = computed(() => (metadata.value?.description || detailGame.value?.summary || '').replace(/\[\/?(?:url|spoiler|b|i|u)[^\]]*\]/g, '').replace(/<[^>]+>/g, ''));
let generation = 0;
let installationGeneration = 0;
watch(detailGameId, async () => {
  ++generation;
  tagsExpanded.value = false;
  focused.value = false; failedImages.value = {}; priceSettings.value = false; error.value = ''; busy.value = false;
}, { immediate: true });
watch(() => record.value?.id, id => {
  if (!id) return;
  const ticket = generation;
  void ensureGameMedia(id).catch(e => { if (ticket === generation) error.value = `影音补全未成功，可在作品资料中重试：${String(e)}`; });
}, { immediate: true });

watch([installations, tab], async () => {
  const ticket = ++installationGeneration;
  if (tab.value !== 'files') return;
  available.value = {}; checked.value = false;
  for (const installation of installations.value) {
    const value = await installationAvailable(installation.directory).catch(() => false);
    if (ticket !== installationGeneration) return;
    available.value[installation.directory] = value;
  }
  if (ticket === installationGeneration) checked.value = true;
}, { immediate: true });
async function setExecutable(index: number, executable: string) {
  if (!record.value?.library) return;
  const current = record.value;
  try {
    await updateLibraryDetails(current.id, library => ({ ...library, installations: library.installations.map((item, i) => i === index ? { ...item, executable } : item) }));
  } catch (e) { error.value = String(e); }
}
async function openPath(path: string) { try { await openInExplorer(path); } catch (e) { error.value = String(e); } }
function edit(section: GameEditorSection = 'general') { atmosphere.value?.pause(); if (detailGame.value) openGameRecordEditor(detailGame.value, section); }
async function unlink() {
  if (!record.value || busy.value) return;
  const ticket = generation, id = record.value.id;
  busy.value = true;
  try { await unlinkSteamGame(id); await loadGameActivity(); }
  catch (e) { if (ticket === generation) error.value = String(e); }
  finally { if (ticket === generation) busy.value = false; }
}

</script>

<template>
  <section v-if="detailGame" class="game-detail" aria-label="作品工作区">
    <img v-if="background && !failedImages[background]" class="detail-background" :src="posterSrc(background)" :style="{objectPosition:coverPosition(record?.library?.background_position || {x:50,y:20})}" alt="" referrerpolicy="no-referrer" @error="failedImages[background] = true" />
    <div ref="content" class="detail-scroll">
    <header class="detail-heading" :class="{'is-focused':focused}">
      <button class="detail-back" aria-label="返回封面墙" title="返回封面墙" @click="emit('back')"><ArrowLeft /></button>
      <button class="detail-background-edit" @click="edit()"><Pencil />编辑作品</button>
      <div v-show="!focused" class="detail-heading-main">
      <div class="detail-identity">
        <img v-if="detailGame.poster && !failedImages[detailGame.poster]" class="detail-poster" @error="failedImages[detailGame.poster] = true" :src="posterSrc(detailGame.poster)" :style="{objectPosition: coverPosition(detailGame.poster_position)}" alt="" referrerpolicy="no-referrer" />
        <span v-else class="detail-monogram" aria-hidden="true">{{ detailGame.title.slice(0,1) }}</span>
        <div class="detail-title"><h1>{{ detailGame.title }}</h1><p v-if="metadata?.original_title && metadata.original_title !== detailGame.title">{{ metadata.original_title }}</p></div>
      </div>
      <div class="detail-heading-actions">
        <div class="detail-personal"><button :aria-pressed="isGamePinned(detailGame)" :aria-label="isGamePinned(detailGame) ? '取消置顶作品' : '置顶作品'" @click="togglePinGame(detailGame)"><Pin :fill="isGamePinned(detailGame) ? 'currentColor' : 'none'" /></button><select aria-label="当前作品游玩状态" :value="getEffectiveGameStatus(detailGame) === '游玩中' ? '在玩' : getEffectiveGameStatus(detailGame)" @change="setGameStatus(detailGame, ($event.target as HTMLSelectElement).value)"><option value="未开始">未开始</option><option value="在玩">在玩中</option><option value="已通关">已通关</option><option value="已收藏">已收藏</option></select></div>
        <GameLaunch :record-id="record?.id" :title="detailGame.title" @launching="atmosphere?.pause()" @setup="tab = 'files'" />
        <button v-if="installations.length" class="detail-link" @click="tab = 'files'"><Maximize2 />画面缩放</button>
      </div>
      </div>
      <GameAtmosphere :key="detailGame.id" ref="atmosphere" :metadata="metadata" :extra="record?.library?.extra_media" :title="detailGame.title" @focus="focused = $event" />
    </header>
    <nav class="detail-tabs" aria-label="作品内容">
      <button v-for="item in [{id:'play',name:'游玩'},{id:'resources',name:'攻略与资源'},{id:'metadata',name:'作品资料'},{id:'files',name:'安装与存档'},{id:'prices',name:'购买与比价'}] as const" :key="item.id" :aria-current="tab === item.id ? 'page' : undefined" @click="tab = item.id">{{ item.name }}</button>
    </nav>
    <div class="detail-content">
      <p v-if="error" role="alert" class="detail-error">{{ error }}</p>
      <p v-if="record && metadataBusyIds.includes(record.id)" role="status" class="detail-notice">正在补全这部作品的宣传片、画面与音乐关联…</p>
      <template v-if="tab === 'play'">
        <section class="detail-progress"><div><h2>最近进度</h2><p>{{ detailGame.progress_note || '还没有记录进度' }}</p><p v-if="detailGame.personal_notes" class="detail-muted">{{ detailGame.personal_notes }}</p></div><button class="detail-link" @click="edit('progress')">编辑进度与备忘</button></section>
        <div class="detail-reading"><BookOpen /><span>{{ detailGame.file_path ? '攻略与阅读进度' : '这部作品的攻略与资源' }}</span><button class="detail-link" @click="detailGame.file_path ? handleOpenGuideReader(detailGame) : tab = 'resources'">{{ detailGame.file_path ? '继续阅读' : '查看资料' }}</button></div>
        <GameActivity v-if="record" :key="record.id" :record-id="record.id" @install="tab = 'files'" />
        <p v-else class="detail-muted">建立个人档案后可记录本地时长，或关联 Steam 同步时长与成就。<button class="detail-link" @click="edit()">建立档案</button></p>
      </template>
      <ExplorePanel v-else-if="tab === 'resources'" :key="detailGame.id" :game="detailGame" :initial-role="detailResourceRole || undefined" />
      <section v-else-if="tab === 'prices' && marketGame">
        <button type="button" class="detail-link" :aria-expanded="priceSettings" @click="priceSettings = !priceSettings"><Settings2 />价格来源</button><MarketConnection v-if="priceSettings" /><GamePricePanel :game="marketGame" @connect="priceSettings = true" />
      </section>
      <section v-else-if="tab === 'files'" class="detail-installations">
        <header class="detail-section-heading"><h2>安装与关联</h2><span>共用当前作品档案</span></header>
        <template v-if="record">
          <article class="detail-install"><div class="detail-section-heading"><h3>Steam 客户端</h3><span>{{ steamLink ? steamInstall ? '已关联 · 已安装' : '已关联 · 未检测到安装' : '尚未关联' }}</span></div><p v-if="steamInstall">{{ steamInstall.directory }}</p><p class="detail-muted">Steam 使用客户端当前登录账号。关联后自动同步时长与成就。</p><div class="detail-install-actions"><button v-if="!steamLink" @click="openSteamImport(record.id)">关联 Steam 游戏</button><template v-else><button v-if="steamInstall" @click="openPath(steamInstall.directory)"><FolderOpen />打开目录</button><button :disabled="busy" @click="unlink">解除 Steam 关联</button></template></div></article>
          <article v-for="(installation,index) in installations" :key="installation.directory" class="detail-install"><div class="detail-section-heading"><h3>本地安装 {{ installations.length > 1 ? index + 1 : '' }}</h3><span :class="{offline: checked && !available[installation.directory]}">{{ !checked ? '检查中' : available[installation.directory] ? '目录可用' : '目录离线 / 无法访问' }}</span></div><p>{{ installation.directory }}</p><label>启动程序<select aria-label="本地启动程序" :value="installation.executable" @change="setExecutable(index, ($event.target as HTMLSelectElement).value)"><option v-if="!installation.candidates.includes(installation.executable)" :value="installation.executable">{{ installation.executable.split(/[\\/]/).pop() }} · 原选择</option><option v-for="exe in installation.candidates" :key="exe" :value="exe">{{ exe.split(/[\\/]/).pop() }}</option></select></label><button @click="openPath(installation.directory)"><FolderOpen />打开目录</button></article>
          <div class="detail-install-actions"><button @click="openLibraryImport(record.id)"><FolderOpen />关联本地安装</button><button v-if="record.save_path" @click="openPath(record.save_path)">打开存档位置</button></div>
          <GameScalingPanel v-for="(installation,index) in installations" :key="installation.directory" :record-id="record.id" :installation-index="index" @settings="edit('scaling')" />
          <GameSaveBackups :key="record.id" :record="record" />
        </template>
        <div v-else class="detail-install-actions"><button @click="openLibraryImport(detailGame.id)"><FolderOpen />关联本地安装</button><button class="detail-link" @click="edit()">编辑个人档案</button></div>
      </section>
      <section v-else-if="tab === 'metadata'" class="detail-metadata">
        <header class="detail-section-heading"><h2>作品资料</h2><button class="detail-link" @click="edit('metadata')">编辑与匹配资料</button></header>
        <p v-if="detailGame.release_date" class="detail-muted">发行日期：{{ detailGame.release_date }}</p>
        <div class="detail-section-heading taxonomy-heading"><h3>标签与制作</h3><button class="detail-link" @click="edit('taxonomy')">编辑标签与制作</button></div>
        <div v-if="taxonomy?.tags.length" class="detail-tags"><button v-for="tag in tagsExpanded ? taxonomy.tags : taxonomy.tags.slice(0,18)" :key="tag" :aria-label="'筛选标签：' + tag" @click="emit('filter', { kind: 'tag', name: tag })">{{ tag }}</button><button v-if="taxonomy.tags.length > 18" class="detail-link" @click="tagsExpanded = !tagsExpanded">{{ tagsExpanded ? '收起' : `展开全部 ${taxonomy.tags.length} 个标签` }}</button></div>
        <p v-else class="detail-muted">还没有标签，可以在编辑中添加。</p>
        <dl v-if="taxonomy" class="detail-credits"><div v-if="taxonomy.developers.length"><dt>开发 / 制作</dt><dd><button v-for="name in taxonomy.developers" :key="name" class="credit-link" :aria-label="'筛选开发公司：' + name" @click="emit('filter', {kind:'developer',name})">{{ name }}</button></dd></div><div v-if="taxonomy.publishers.length"><dt>发行商</dt><dd><button v-for="name in taxonomy.publishers" :key="name" class="credit-link" :aria-label="'筛选发行商：' + name" @click="emit('filter', {kind:'publisher',name})">{{ name }}</button></dd></div><div v-for="role in [...new Set(taxonomy.staff.map(s => s.role))]" :key="role"><dt>{{ role }}</dt><dd><button v-for="member in taxonomy.staff.filter(s => s.role === role)" :key="member.name" class="credit-link" :aria-label="'筛选制作人员：' + member.name" @click="emit('filter', {kind:'person',name:member.name,role})">{{ member.name }}</button></dd></div></dl>
        <p class="detail-description">{{ description || '还没有作品介绍，可在“编辑作品”的资料匹配中补充。' }}</p>
        <div v-if="sources.length" class="detail-sources"><a v-for="entry in sources" :key="entry.source + entry.source_id" :href="entry.url" @click.prevent="openExternalUrl(entry.url)">{{ entry.source === 'bangumi' ? 'Bangumi' : entry.source === 'steam' ? 'Steam' : 'VNDB' }} · {{ entry.source_id }}<ExternalLink /></a></div>
        <GameMedia v-if="record" :key="record.id" :record="record" :metadata="metadata" @play="playMedia" @background="edit('background')" />
        <section v-if="workInfo.length" class="detail-work-info"><h3>制作与发行</h3><dl><div v-for="info in workInfo" :key="info.label"><dt>{{ info.label === 'website' ? '官方网站' : info.label }}</dt><dd><a v-if="/^https?:\/\/\S+$/.test(info.value)" :href="info.value" @click.prevent="openExternalUrl(info.value)">{{ info.value }} <ExternalLink /></a><template v-else>{{ info.value }}</template></dd></div></dl></section>
        <p v-if="metadata?.aliases?.length" class="detail-muted">其他名称：{{ metadata.aliases.join(' ／ ') }}</p>
      </section>
    </div>
    </div>
  </section>
</template>

<style scoped>
.taxonomy-heading{margin-top:20px}.detail-tags button{background:var(--color-paper);padding:4px 8px;font-size:11px;color:var(--color-muted);border-color:transparent}.detail-tags button:hover{border-color:var(--color-beacon);color:var(--color-beacon)}.detail-credits{margin-top:16px}.detail-credits>div{display:grid;grid-template-columns:100px minmax(0,1fr);gap:12px;padding:7px 0;font-size:12px;line-height:1.8}.detail-credits dt{color:var(--color-faint);padding-top:5px}.detail-credits dd{display:flex;flex-wrap:wrap;gap:4px 12px;min-width:0}.detail-credits .credit-link{padding:4px 0;color:var(--color-beacon);border:0;text-align:left;overflow-wrap:anywhere}
.detail-background-edit{position:absolute;right:24px;top:16px;z-index:2;background:color-mix(in srgb,var(--color-canvas) 70%,transparent);backdrop-filter:blur(8px)}
.game-detail{position:relative;isolation:isolate;overflow:hidden;display:flex;flex-direction:column;height:100%;min-height:0;min-width:0;color:var(--color-ink);container-type:inline-size}.detail-scroll{flex:1;min-height:0;overflow-x:hidden;overflow-y:auto;display:flex;flex-direction:column}.detail-background{position:absolute;inset:0;width:100%;height:100%;object-fit:cover;object-position:center top;z-index:-2;pointer-events:none}.game-detail::before{content:'';position:absolute;inset:0;z-index:-1;pointer-events:none;background:linear-gradient(90deg,color-mix(in srgb,var(--color-canvas) 82%,transparent),color-mix(in srgb,var(--color-canvas) 12%,transparent) 80%),linear-gradient(0deg,var(--color-canvas),color-mix(in srgb,var(--color-canvas) 82%,transparent) 45%,transparent 100%)}.detail-heading{position:relative;isolation:isolate;display:flex;flex-direction:column;justify-content:end;gap:20px;height:clamp(300px,32cqw,440px);min-height:300px;padding:28px 24px 12px;flex-shrink:0}.detail-back{position:absolute;top:16px;left:24px;z-index:1;border:0;padding:6px 8px;min-height:32px;width:32px;background:transparent;color:var(--color-muted)}.detail-back svg{width:18px;height:18px}.detail-back:hover{background:color-mix(in srgb,var(--color-canvas) 35%,transparent);color:var(--color-ink)}.detail-heading-main{display:grid;grid-template-columns:minmax(0,1fr) 200px;align-items:end;gap:24px}.detail-heading-actions{display:flex;flex-direction:column;align-items:end;gap:24px;min-width:0}.detail-heading-actions :deep(.game-launch){width:100%;flex-direction:column;align-items:stretch;gap:8px}.detail-heading-actions :deep(.launch-main){min-height:40px}.detail-heading-actions :deep(.launch-versions){justify-content:end}.detail-heading-actions :deep(.launch-versions button){font-size:11px;min-height:30px;padding:5px 8px;max-width:100%}.detail-heading-actions :deep(.game-launch p){font-size:11px;line-height:1.5}.detail-heading :deep(.atmosphere-controls){margin:0;padding:8px 0 0;gap:8px}.detail-heading :deep(.atmosphere-label strong){max-width:400px}.detail-heading :deep(.atmosphere-picture){z-index:-2}.detail-heading :deep(.atmosphere-wash){z-index:-1}.detail-identity{display:flex;align-items:end;gap:18px;min-width:0}.detail-poster,.detail-monogram{width:clamp(120px,12cqw,170px);height:auto;aspect-ratio:3/4;object-fit:cover;flex-shrink:0;border-radius:3px}.detail-monogram{aspect-ratio:3/4;display:grid;place-items:center;font:30px var(--font-display);background:var(--color-paper);color:var(--color-beacon)}.detail-title{flex:1;min-width:0}h1{font:500 clamp(22px,2.3vw,30px)/1.3 var(--font-display);overflow-wrap:anywhere}.detail-title p{font-size:12px;line-height:1.5;color:var(--color-muted);margin-top:5px;overflow-wrap:anywhere}.detail-personal{align-self:end;display:flex;align-items:center;gap:4px;flex-shrink:0}.detail-personal button{border:0;padding:6px;color:var(--color-faint)}.detail-personal [aria-pressed=true]{color:var(--color-beacon)}.detail-personal select{background:transparent;border:0;font-size:11px;padding:5px;color:var(--color-muted)}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;border:1px solid var(--color-hairline);border-radius:4px;padding:7px 10px;font-size:12px;cursor:pointer;background:transparent;min-height:32px}button:hover{background:var(--color-paper)}button:disabled{opacity:.45;cursor:default}svg{width:14px;height:14px;flex-shrink:0}.detail-tabs{display:flex;align-items:center;gap:22px;flex-wrap:wrap;padding:0 24px;border-bottom:1px solid var(--color-hairline);flex-shrink:0}.detail-tabs button{border:0;border-bottom:2px solid transparent;border-radius:0;background:none;padding:10px 0;color:var(--color-muted);font-size:13px;white-space:nowrap}.detail-tabs button[aria-current]{color:var(--color-beacon);border-bottom-color:currentColor}.detail-content{flex:1 0 auto;min-height:180px;padding:20px 24px 36px}h2{font:22px var(--font-display)}h3{font:18px var(--font-display)}.detail-progress{display:flex;align-items:start;justify-content:space-between;gap:16px;padding-bottom:16px}.detail-progress h2{font:13px var(--font-sans);color:var(--color-muted);margin-bottom:8px}.detail-progress p{font-size:14px;white-space:pre-line;overflow-wrap:anywhere}.detail-progress .detail-link{flex-shrink:0}.detail-reading{display:flex;align-items:center;gap:9px;padding:12px 0;margin-bottom:24px;border-block:1px solid var(--color-hairline);font-size:12px;color:var(--color-muted)}.detail-reading .detail-link{margin-left:auto}.detail-link{display:inline-flex;align-items:center;gap:6px;color:var(--color-beacon);border:0;background:none;padding:4px 0;font-size:12px;cursor:pointer}.detail-section-heading{display:flex;align-items:center;justify-content:space-between;flex-wrap:wrap;gap:10px}.detail-section-heading>span{color:var(--color-muted);font-size:11px}.detail-section-heading>div{display:flex;gap:16px;flex-wrap:wrap}.detail-muted{font-size:12px!important;color:var(--color-muted);line-height:1.8;margin:12px 0;white-space:pre-line;overflow-wrap:anywhere}.detail-muted .detail-link{margin-left:8px}.detail-install{padding:20px 0;border-bottom:1px solid var(--color-hairline)}.detail-install>p:not(.detail-muted){font:11px/1.8 var(--font-mono);color:var(--color-muted);overflow-wrap:anywhere;margin:12px 0}.detail-install label{display:flex;gap:12px;align-items:center;font-size:12px;margin:14px 0;flex-wrap:wrap}.detail-install select{max-width:100%;flex:1}.detail-install-actions{display:flex;gap:10px;flex-wrap:wrap;margin-top:16px}.offline,.detail-error{color:var(--color-alert)}.detail-description{white-space:pre-line;font-size:13px;line-height:1.9;color:var(--color-muted);margin:18px 0}.detail-tags{display:flex;flex-wrap:wrap;gap:6px;margin-top:16px}.detail-tags span{background:var(--color-paper);padding:3px 7px;font-size:11px;border-radius:3px;color:var(--color-muted)}.detail-sources{display:flex;gap:14px;flex-wrap:wrap;margin:14px 0;font-size:12px;color:var(--color-beacon)}.detail-sources a{display:flex;align-items:center;gap:5px}input,select{background:var(--color-paper);border:1px solid var(--color-hairline);color:var(--color-ink);padding:7px 9px;border-radius:4px;min-width:0;font-size:12px}.detail-work-info{margin-top:26px;padding-top:22px;border-top:1px solid var(--color-hairline)}.detail-work-info dl>div{display:grid;grid-template-columns:90px minmax(0,1fr);gap:16px;padding:10px 0;border-bottom:1px solid var(--color-hairline);font-size:12px;line-height:1.8}.detail-work-info dt{color:var(--color-faint)}.detail-work-info dd{overflow-wrap:anywhere;white-space:pre-line}.detail-work-info a{color:var(--color-beacon)}.detail-work-info a svg{display:inline}.detail-error,.detail-notice{font-size:12px;margin-bottom:16px;overflow-wrap:anywhere}.detail-notice{color:var(--color-beacon)}:where(button,a,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
@container(max-width:650px){.detail-back{left:16px;top:12px}.detail-heading{padding:20px 16px 10px;min-height:300px}.detail-heading-main{grid-template-columns:minmax(0,1fr) 150px;gap:16px}.detail-heading-actions{gap:16px}.detail-identity{gap:12px}.detail-personal{gap:0}.detail-personal button{min-height:24px}.detail-tabs{gap:16px;padding:0 16px}.detail-tabs button{font-size:12px;padding:9px 0}.detail-content{padding:16px 16px 28px}.detail-progress{flex-wrap:wrap;gap:8px}.detail-title h1{font-size:21px}.detail-poster,.detail-monogram{width:110px;height:auto}.detail-section-heading{align-items:start}}
</style>
