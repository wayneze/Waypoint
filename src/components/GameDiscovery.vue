<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { ArrowLeft, ArrowUpRight, Bell, BookOpen, Check, EyeOff, Heart, Search, Settings2, X } from '@lucide/vue';
import type { ObsidianGameGuide } from '../types';
import { chineseText, discoveryInterests, regionInfo, discoverDeals, discoverGames, feedbackForGame, localizeMarketGame, marketGuide, moneyLabel, quoteFresh, sameMarketGame, type DiscoveryFeedback, type DiscoveryFeedbackKind, type DiscoverySource, type MarketGame } from '../services/discovery';
import { loadGameMetadata } from '../services/localGames';
import { openExternalUrl } from '../services/systemApi';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import { checkPriceWatches, checkingPrices, discoveryFeedback, discoveryPreferences, discoveryReady, marketCountry, hydrateDiscovery, marketIsConnected, priceMonitorError, priceWatches, removePriceWatch, restoreDiscoveryGame, saveDiscoveryFeedback, savePriceWatch } from '../workspace/discovery';
import { posterSrc } from './sky/wikiImage';
import GamePricePanel from './GamePricePanel.vue';
import MarketConnection from './MarketConnection.vue';
import DiscoveryPreferences from './DiscoveryPreferences.vue';
import ExplorePanel from './ExplorePanel.vue';
import { gameRecords } from '../workspace/core';
import { steamLibrary } from '../workspace/vault';
import { ownedMarketGame } from '../services/discovery';
const root=ref<HTMLElement>();
const view=ref<'discover'|'deals'|'wanted'|'resources'>('discover');
const resourceGame=ref<ObsidianGameGuide>();
const source=ref<DiscoverySource>('bangumi'),query=ref(''),games=ref<MarketGame[]>([]);
const chosen=ref<MarketGame>(),busy=ref(false),detailBusy=ref(false),error=ref(''),notice=ref(''),offset=ref(0);
const nextOffset=ref<number | null>(null),previousOffsets=ref<number[]>([]);
const settings=ref(false),preferencesOpen=ref(false),hideOwned=ref(true);
const feedbackOpen=ref(false),feedbackError=ref('');
const lastHidden=ref<DiscoveryFeedback>();
const region=computed(()=>regionInfo(marketCountry.value));
const selectedInterests=computed(()=>discoveryPreferences.value?.all?[]:discoveryPreferences.value?.interests || []);
const preferenceLabel=computed(()=>discoveryPreferences.value?.all?'全部类型':selectedInterests.value.map(id=>discoveryInterests.find(i=>i.id===id)?.label).join(' / '));
const failed=ref<Record<string,boolean>>({});
const failedCount=computed(()=>rows.value.filter(g=>g.cover && failed.value[g.id]).length);
function retryCovers(){failed.value={};}
const shop=ref('all'),budget=ref<number | ''>(''),saving=ref(false);
const landscapeCards=computed(()=>view.value==='deals' || view.value==='discover' && source.value==='steam');
const feedbackFor=(game:MarketGame)=>feedbackForGame(game,discoveryFeedback.value);
const isOwned=(game:MarketGame)=>feedbackFor(game)?.kind==='owned' || ownedMarketGame(game,gameRecords.value,steamLibrary.value);
const rows=computed(()=>{
  const rows=view.value==='wanted'?priceWatches.value.map(w=>({...w.game,offer:w.quotes?.offers.filter(o=>quoteFresh(o)&&o.price.currency===w.rule.currency && o.country===w.rule.country).sort((a,b)=>a.price.amount-b.price.amount)[0]})):games.value;
  return rows.map(g=>localizeMarketGame(g,singleAndGalgameGuides.value)).filter(g=>{
    if(view.value==='wanted')return true;
    if(feedbackFor(g)?.kind==='uninterested')return false;
    if(hideOwned.value && isOwned(g))return false;
    if(view.value!=='deals')return true;
    return (shop.value==='all'||g.offer?.shop_id===Number(shop.value))
      && (budget.value===''||!!g.offer&&g.offer.country===marketCountry.value && g.offer.price.currency===region.value.currency&&g.offer.price.amount<=Number(budget.value));
  });
});
const watchFor=(game:MarketGame)=>priceWatches.value.find(w=>sameMarketGame(w.game,game));
let generation=0,detailGeneration=0;
onBeforeUnmount(()=>{generation++;detailGeneration++;});
async function load() {
  if(view.value==='wanted' || view.value==='resources' || !discoveryPreferences.value)return;
  const ticket=++generation;busy.value=true;error.value='';notice.value='';games.value=[];retryCovers();
  try {
    const result=view.value==='deals'?await discoverDeals(offset.value,marketCountry.value,selectedInterests.value):await discoverGames(source.value,query.value,offset.value,marketCountry.value,selectedInterests.value);
    if(ticket===generation){games.value=result.games;nextOffset.value=result.next_offset;notice.value=result.warning || (!result.games.length?'没有更多符合条件的作品。':'');}
  }catch(e){if(ticket===generation)error.value=String(e);}
  finally{if(ticket===generation)busy.value=false;}
}
function resetSearch(){offset.value=0;previousOffsets.value=[];nextOffset.value=null;void load();}
function page(forward:boolean) {
  if(forward){if(nextOffset.value===null)return;previousOffsets.value.push(offset.value);offset.value=nextOffset.value;}
  else {offset.value=previousOffsets.value.pop() || 0;}
  void load();
}
watch([view,source,discoveryPreferences],()=>{generation++;busy.value=false;chosen.value=undefined;detailGeneration++;resetSearch();},{immediate:true});
watch([view,()=>chosen.value?.id],()=>root.value?.scrollIntoView({block:'start'}),{flush:'post'});
onMounted(async()=>{try{await hydrateDiscovery();}catch(e){error.value=String(e);}});
async function select(game:MarketGame) {
  chosen.value=game;
  const ticket=++detailGeneration;detailBusy.value=false;error.value='';
  if(!game.metadata || game.metadata.enriched)return;
  detailBusy.value=true;
  try {
    const full=await loadGameMetadata(game.metadata);
    if(ticket!==detailGeneration)return;
    const steam=full.source==='steam'?full.source_id:full.sources?.find(s=>s.source==='steam')?.source_id;
    const localized=localizeMarketGame({...game,metadata:full,cover:full.cover || game.cover,title:full.title,steam_id:steam || game.steam_id},singleAndGalgameGuides.value);
    chosen.value=localized;
  }catch(e){if(ticket===detailGeneration)error.value=`详情暂未取得：${String(e)}`;}
  finally{if(ticket===detailGeneration)detailBusy.value=false;}
}
async function want(game:MarketGame) {
  if(saving.value)return;
  saving.value=true;error.value='';
  try{await savePriceWatch(game);notice.value=`已将《${game.title}》加入想玩。可在购买与比价中设置提醒。`;}
  catch(e){error.value=String(e);}finally{saving.value=false;}
}
async function remove(id:string){try{await removePriceWatch(id);chosen.value=undefined;}catch(e){error.value=String(e);}}
async function dismiss(game:MarketGame,kind:DiscoveryFeedbackKind) {
  if(saving.value)return;
  saving.value=true;feedbackError.value='';
  try {
    await saveDiscoveryFeedback(game,kind);
    lastHidden.value=feedbackFor(game);
    chosen.value=undefined;detailGeneration++;
  }catch(e){feedbackError.value=String(e);}
  finally{saving.value=false;}
}
async function restore(entry:DiscoveryFeedback) {
  if(saving.value)return;
  saving.value=true;feedbackError.value='';
  try {
    await restoreDiscoveryGame(entry);
    if(lastHidden.value===entry)lastHidden.value=undefined;
  }catch(e){feedbackError.value=String(e);}
  finally{saving.value=false;}
}
watch(marketCountry,()=>{budget.value='';});
watch(marketIsConnected,()=>{if(view.value==='deals')resetSearch();});
async function open(url:string){try{await openExternalUrl(url);}catch(e){error.value=String(e);}}
function resources(game?:MarketGame){resourceGame.value=game?marketGuide(game):undefined;preferencesOpen.value=false;view.value='resources';}
const description=computed(()=>chosen.value?.metadata?.description?.replace(/<[^>]+>|\[\/?(?:url|spoiler|b|i|u)[^\]]*\]/g,'') || '');
</script>
<template>
  <section ref="root" class="game-discovery">
    <header class="discovery-heading"><div><h2>发现<span>.</span></h2><p>下一部作品，从这里遇见</p></div><div class="discovery-heading-actions"><button v-if="discoveryPreferences" type="button" class="discovery-button" :aria-expanded="preferencesOpen" @click="preferencesOpen=!preferencesOpen"><Settings2 />偏好设置</button><button type="button" class="discovery-button" :aria-expanded="feedbackOpen" @click="feedbackOpen=!feedbackOpen"><EyeOff />管理隐藏<span v-if="discoveryFeedback.length">{{ discoveryFeedback.length }}</span></button><button type="button" class="discovery-button" :aria-expanded="settings" @click="settings=!settings"><Settings2 />价格来源</button></div></header>
    <nav class="discovery-tabs" aria-label="发现页面分组">
      <button type="button" :aria-current="view==='discover'?'page':undefined" @click="view='discover'">发现游戏</button>
      <button type="button" :aria-current="view==='deals'?'page':undefined" @click="view='deals'">优惠</button>
      <button type="button" :aria-current="view==='resources'?'page':undefined" @click="resources()">攻略与资源</button>
      <button type="button" :aria-current="view==='wanted'?'page':undefined" @click="view='wanted'"><Heart />想玩与提醒<span>{{ priceWatches.length }}</span></button>
    </nav>
    <MarketConnection v-if="settings" />
    <section v-if="feedbackOpen" class="discovery-feedback" aria-label="管理已拥有与不感兴趣的作品">
      <header><div><h3>已拥有与不感兴趣</h3><p>你的选择保存在本机，发现和优惠共用。恢复推荐后将重新按偏好展示。</p></div><button type="button" aria-label="关闭隐藏管理" @click="feedbackOpen=false"><X /></button></header>
      <ul v-if="discoveryFeedback.length"><li v-for="entry in discoveryFeedback" :key="entry.keys[0]"><span>{{ entry.title }}</span><small>{{ entry.kind==='owned'?'已拥有':'不感兴趣' }}</small><button type="button" :disabled="saving" @click="restore(entry)">恢复推荐</button></li></ul>
      <p v-else>还没有手动标记的作品。</p>
    </section>
    <p v-if="feedbackError" class="discovery-error" role="alert">未能保存选择：{{ feedbackError }}</p>
    <p v-if="lastHidden" class="discovery-feedback-notice" role="status">《{{ lastHidden.title }}》已标记为{{ lastHidden.kind==='owned'?'已拥有':'不感兴趣' }}。<button type="button" :disabled="saving" @click="restore(lastHidden)">撤销</button></p>
    <p v-if="!discoveryReady" class="discovery-status" role="status">正在读取偏好…</p>
    <DiscoveryPreferences v-else-if="preferencesOpen || (!discoveryPreferences && view!=='resources')" :initial="!discoveryPreferences" cancellable @saved="preferencesOpen=false" @cancel="preferencesOpen=false" />
    <ExplorePanel v-if="view==='resources' && !preferencesOpen" :key="resourceGame?.id || 'all'" :game="resourceGame" embedded initial-role="guide" />
    <template v-if="discoveryReady && discoveryPreferences && !preferencesOpen && view!=='resources'">
    <div v-if="view!=='wanted' && !chosen" class="discovery-preference-summary"><span>{{ preferenceLabel }}</span><span>商店 {{ region.label }} · {{ region.currency }}</span></div>
    <template v-if="!chosen">
      <form v-if="view==='discover'" class="discovery-toolbar" @submit.prevent="resetSearch">
        <label class="discovery-select">作品来源<select v-model="source"><option value="bangumi">Bangumi · 中文游戏资料</option><option value="steam">Steam · {{ region.label }}</option></select></label>
        <label class="discovery-search"><Search /><input v-model="query" aria-label="搜索发现作品" placeholder="中文名或原名" /></label><button class="discovery-button" :disabled="busy">搜索</button>
      </form>
      <div v-else-if="view==='deals'" class="discovery-toolbar">
        <span class="discovery-muted">筛选本页</span><label class="discovery-select">商店<select v-model="shop"><option value="all">全部已覆盖商店</option><option value="61">Steam</option><option value="16">Epic</option><option value="35">GOG</option></select></label>
        <label class="discovery-select">预算上限 · {{ region.currency }}<input v-model.number="budget" type="number" min="0" step="1" placeholder="不限" aria-label="优惠预算上限" /></label>
        <button type="button" class="discovery-button" :disabled="busy" @click="load()">刷新优惠</button>
        <span class="discovery-muted">{{ marketIsConnected?'IsThereAnyDeal':'Steam · 跨店价格源未连接' }} · {{ region.label }}</span>
      </div>
      <div v-else class="discovery-toolbar"><p class="discovery-muted">收藏想玩的作品；提醒仅在航标运行时检查。</p><button type="button" class="discovery-button" :disabled="checkingPrices" @click="checkPriceWatches(true)"><Bell />{{ checkingPrices?'正在检查…':'检查关注价格' }}</button></div>
      <div v-if="view!=='wanted'" class="discovery-context"><label class="discovery-owned"><input v-model="hideOwned" type="checkbox" />隐藏已拥有（含 Steam 库与本地安装）</label>
      <p v-if="view==='discover'" class="discovery-caption">中文笔记与中文资料优先显示；缺少中文名时保留原名。各来源评分独立呈现。</p></div>
      <p v-if="busy" role="status" class="discovery-status">正在寻找作品…</p>
      <p v-if="error || priceMonitorError" class="discovery-error" role="alert">{{ error || priceMonitorError }}<button type="button" @click="load()">重试</button></p>
      <p v-if="notice" class="discovery-caption" role="status">{{ notice }}</p>
      <p v-if="failedCount" class="discovery-caption">{{ failedCount }} 张封面暂未加载。<button type="button" class="discovery-button" @click="retryCovers">重新加载封面</button></p>
      <div class="discovery-grid" :class="{'is-landscape':landscapeCards}">
        <article v-for="game in rows" :key="game.id" class="discovery-book">
          <button type="button" class="discovery-cover" :aria-label="'查看'+game.title" @click="select(game)"><img v-if="game.cover&&!failed[game.id]" :src="posterSrc(game.cover)" loading="lazy" alt="" referrerpolicy="no-referrer" @error="failed[game.id]=true" /><span v-else>{{ game.title }}</span><b v-if="game.offer?.discount">−{{ game.offer.discount }}%</b></button>
          <div class="discovery-book-meta"><span>{{ game.metadata?.release_date?.slice(0,4) || (game.offer?'当前促销':'作品') }}</span><span v-if="game.metadata?.score">{{ game.metadata.source==='bangumi'?'Bangumi':'VNDB' }} {{ game.metadata.score.toFixed(1) }}</span><span v-else-if="!chineseText(game.title)">原名</span></div>
          <h3><button type="button" @click="select(game)">{{ game.title }}</button></h3>
          <p>{{ game.metadata?.developer || game.offer?.shop || '查看作品详情与购买版本' }}</p>
          <div v-if="game.offer" class="discovery-price"><strong>{{ moneyLabel(game.offer.price) }}</strong><span>{{ game.offer.shop }}</span></div>
          <p v-if="view==='wanted'" class="discovery-muted">提醒地区：{{ regionInfo(watchFor(game)!.rule.country).label }} · {{ watchFor(game)?.rule.currency }}</p>
          <p v-if="view==='wanted'&&watchFor(game)?.rule.enabled" class="discovery-watch-label"><Bell />{{ watchFor(game)?.rule.mode==='low'?'达到该店史低提醒':'不高于 '+moneyLabel({amount:watchFor(game)!.rule.target,currency:watchFor(game)!.rule.currency})+' 提醒' }}</p>
          <p v-if="view==='wanted'&&watchFor(game)?.error" class="discovery-error">{{ watchFor(game)?.error }}</p>
          <p v-else-if="view==='wanted'&&watchFor(game)?.quotes?.warning" class="discovery-muted">{{ watchFor(game)?.quotes?.warning }}</p>
          <footer><div class="discovery-card-actions"><button type="button" :disabled="saving" @click="want(game)"><Check v-if="watchFor(game)" /><Heart v-else />{{ watchFor(game)?'已想玩':'想玩' }}</button><button type="button" @click="select(game)">详情与比价<ArrowUpRight /></button></div><div v-if="view!=='wanted'" class="discovery-card-actions discovery-dismiss"><button type="button" :disabled="saving" :aria-label="'不感兴趣：'+game.title" @click="dismiss(game,'uninterested')"><EyeOff />不感兴趣</button><button type="button" :disabled="saving || isOwned(game)" :aria-label="'已拥有：'+game.title" @click="dismiss(game,'owned')"><Check />已拥有</button></div></footer>
        </article>
      </div>
      <p v-if="!busy&&!error&&!rows.length" class="discovery-empty">{{ view==='wanted'?'还没有想玩的作品。浏览游戏后点「想玩」，再设置自己的目标价。':'没有符合当前条件的作品。' }}</p>
      <div v-if="view!=='wanted' && (nextOffset!==null || previousOffsets.length)" class="discovery-pagination"><button type="button" :disabled="busy||!previousOffsets.length" @click="page(false)">上一页</button><span>第 {{ previousOffsets.length+1 }} 页</span><button type="button" :disabled="busy||nextOffset===null" @click="page(true)">下一页</button></div>
    </template>
    <template v-else>
      <button type="button" class="discovery-back" @click="chosen=undefined;detailGeneration++"><ArrowLeft />返回{{ view==='deals'?'优惠':view==='wanted'?'想玩清单':'发现' }}</button>
      <div class="discovery-detail">
        <section class="discovery-work">
          <img v-if="chosen.cover&&!failed[chosen.id]" :src="posterSrc(chosen.cover)" alt="" referrerpolicy="no-referrer" @error="failed[chosen.id]=true" />
          <button v-else-if="chosen.cover" class="discovery-button" @click="retryCovers">重新加载封面</button>
          <div><p class="discovery-muted">{{ chosen.metadata?.developer }}</p><h2>{{ chosen.title }}</h2><p class="discovery-muted">{{ chosen.metadata?.original_title }}</p>
          <p v-if="detailBusy" role="status">正在补全作品资料…</p><p v-if="error" class="discovery-error" role="alert">{{ error }}</p>
          <p class="discovery-description">{{ description || '暂无中文简介，可前往来源查看作品资料。' }}</p>
          <div class="discovery-work-actions"><button type="button" class="discovery-button" :disabled="saving" @click="want(chosen)"><Heart />{{ watchFor(chosen)?'已加入想玩':'加入想玩' }}</button><button type="button" class="discovery-button" @click="resources(chosen)"><BookOpen />攻略与资源</button><button v-if="chosen.url" type="button" class="discovery-button" @click="open(chosen.url)">作品来源<ArrowUpRight /></button><button v-if="watchFor(chosen)" type="button" class="discovery-button" @click="remove(watchFor(chosen)!.game.id)"><X />移出想玩</button></div>
          <div v-if="view!=='wanted'" class="discovery-work-actions"><button type="button" class="discovery-button" :disabled="saving" @click="dismiss(chosen,'uninterested')"><EyeOff />不感兴趣</button><button type="button" class="discovery-button" :disabled="saving || isOwned(chosen)" @click="dismiss(chosen,'owned')"><Check />已拥有</button></div>
          <p v-if="notice" class="discovery-caption" role="status">{{ notice }}</p></div>
        </section>
        <GamePricePanel :game="chosen" @connect="settings=true" />
      </div>
    </template>
    </template>
    <footer class="discovery-attribution"><button type="button" @click="open('https://bgm.tv/')">Bangumi</button><span>·</span><button type="button" @click="open('https://store.steampowered.com/')">Steam</button><span>·</span><button type="button" @click="open('https://isthereanydeal.com/')">价格数据 IsThereAnyDeal</button></footer>
  </section>
</template>
<style scoped>
.game-discovery{padding:12px 0;container-type:inline-size}.discovery-heading{display:flex;justify-content:space-between;gap:16px;align-items:center}.discovery-heading>div{display:flex;align-items:baseline;gap:16px;flex-wrap:wrap}.discovery-heading-actions{display:flex;gap:8px;flex-wrap:wrap}.discovery-preference-summary{display:flex;justify-content:space-between;gap:10px;flex-wrap:wrap;font-size:12px;color:var(--color-beacon);margin:0 0 16px}.discovery-heading p{font-size:12px;color:var(--color-muted);margin-bottom:4px}.discovery-heading h2{font-family:var(--font-display);font-weight:400;font-size:26px}.discovery-heading h2 span{color:var(--color-beacon)}button{cursor:pointer}button:disabled{opacity:.45;cursor:default}svg{width:15px;height:15px;flex:none}.discovery-button{display:inline-flex;gap:7px;align-items:center;justify-content:center;border:1px solid var(--color-hairline);border-radius:4px;padding:8px 12px;font-size:12px;color:var(--color-muted);min-height:34px}.discovery-button:hover{color:var(--color-beacon);background:var(--color-paper)}.discovery-tabs{display:flex;gap:26px;flex-wrap:wrap;border-bottom:1px solid var(--color-hairline);margin:8px 0 16px}.discovery-tabs button{display:flex;gap:7px;align-items:center;padding:10px 0;border-bottom:2px solid transparent;color:var(--color-muted);font-size:13px}.discovery-tabs button[aria-current]{color:var(--color-beacon);border-color:currentColor}.discovery-tabs button:last-child{margin-left:auto}.discovery-tabs span{font:11px var(--font-mono)}.discovery-toolbar{display:flex;align-items:center;gap:12px;flex-wrap:wrap;margin-bottom:16px}.discovery-select{display:flex;gap:7px;align-items:center;font-size:12px;color:var(--color-muted)}.discovery-select input{width:110px}.discovery-select input,.discovery-select select{background:var(--color-paper);color:var(--color-ink);border:1px solid var(--color-hairline);border-radius:4px;padding:8px;max-width:100%}.discovery-search{display:flex;flex:1;min-width:170px;gap:8px;align-items:center;padding:9px 12px;border:1px solid var(--color-hairline);background:var(--color-chrome);border-radius:4px;color:var(--color-muted)}.discovery-search input{background:transparent;min-width:0;width:100%;font-size:13px;color:var(--color-ink)}.discovery-search:focus-within{border-color:var(--color-beacon)}.discovery-muted,.discovery-caption{font-size:12px;color:var(--color-muted);line-height:1.8}.discovery-caption{margin:8px 0 14px}.discovery-context{display:flex;justify-content:space-between;align-items:baseline;gap:6px 20px;flex-wrap:wrap;margin-bottom:18px}.discovery-context .discovery-caption,.discovery-context .discovery-owned{margin:0}.discovery-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(190px,1fr));gap:28px 24px}.discovery-book{min-width:0}.discovery-cover{display:block;width:100%;aspect-ratio:4/5;position:relative;overflow:hidden;border-radius:3px;background:var(--color-paper);text-align:left}.discovery-cover img{width:100%;height:100%;object-fit:cover;transition:transform .2s}.discovery-cover:hover img{transform:scale(1.025)}.discovery-cover>span{display:block;padding:22px;font-family:var(--font-display);font-size:23px}.discovery-cover b{position:absolute;bottom:10px;left:10px;background:var(--color-beacon);color:var(--color-canvas);font:13px var(--font-mono);padding:6px 9px;border-radius:3px}.discovery-book-meta{display:flex;justify-content:space-between;gap:8px;margin:12px 0 8px;color:var(--color-muted);font-size:11px}.discovery-book h3{font-size:16px;font-weight:500;line-height:1.5}.discovery-book h3 button{text-align:left}.discovery-book>p{font-size:12px;line-height:1.6;color:var(--color-muted);margin:6px 0 12px}.discovery-book footer{display:flex;justify-content:space-between;gap:8px;flex-wrap:wrap;border-top:1px solid var(--color-hairline);padding-top:10px;margin-top:14px}.discovery-book footer button{display:flex;gap:5px;align-items:center;color:var(--color-muted);font-size:12px;min-height:30px}.discovery-book footer button:hover{color:var(--color-beacon)}.discovery-price{display:flex;align-items:baseline;justify-content:space-between;gap:8px;margin-top:12px}.discovery-price strong{font-family:var(--font-mono);font-weight:500;font-size:18px;color:var(--color-beacon)}.discovery-price span{font-size:12px;color:var(--color-muted)}.discovery-watch-label{display:flex;gap:5px;align-items:center}.discovery-status,.discovery-empty{padding:30px 0;font-size:14px;color:var(--color-muted)}.discovery-error{color:var(--color-alert)!important;font-size:12px;line-height:1.8;overflow-wrap:anywhere}.discovery-error button{margin-left:12px;text-decoration:underline}.discovery-owned{display:flex;gap:7px;align-items:center;font-size:12px;color:var(--color-muted);margin:12px 0 18px}.discovery-pagination{display:flex;gap:20px;justify-content:center;align-items:center;padding:30px 0;font-size:12px;color:var(--color-muted)}.discovery-back{display:flex;align-items:center;gap:8px;font-size:12px;color:var(--color-muted);padding:5px 0;margin-bottom:24px}.discovery-detail{display:grid;grid-template-columns:minmax(0,1.1fr) minmax(340px,1fr);gap:36px}.discovery-work{display:flex;flex-direction:column;gap:22px;min-width:0}.discovery-work>img{max-height:340px;max-width:100%;align-self:start;object-fit:contain;border-radius:3px}.discovery-work h2{font-family:var(--font-display);font-size:26px;line-height:1.4;font-weight:500;margin:8px 0}.discovery-description{font-size:14px;line-height:1.9;white-space:pre-line;color:var(--color-muted);margin:20px 0;overflow-wrap:anywhere}.discovery-work-actions{display:flex;gap:10px;flex-wrap:wrap}.discovery-attribution{display:flex;gap:8px;flex-wrap:wrap;font-size:11px;color:var(--color-muted);margin-top:30px;padding-top:16px;border-top:1px solid var(--color-hairline)}:where(button,input,select):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}@container(max-width:850px){.discovery-detail{grid-template-columns:1fr}.discovery-work{flex-direction:row}.discovery-work>img{width:160px;max-height:230px}.discovery-grid{grid-template-columns:repeat(auto-fill,minmax(165px,1fr))}}@container(max-width:540px){.discovery-tabs{gap:14px}.discovery-tabs button:last-child{margin-left:0}.discovery-search{flex-basis:100%}.discovery-grid{grid-template-columns:repeat(2,minmax(0,1fr));gap:20px 16px}.discovery-work{flex-direction:column}.discovery-select{flex-wrap:wrap}}@media(prefers-reduced-motion:reduce){.discovery-cover img{transition:none}}
.discovery-book footer{flex-direction:column;gap:4px}
.discovery-card-actions{display:flex;justify-content:space-between;align-items:center;gap:8px;flex-wrap:wrap}
.discovery-dismiss{padding-top:4px}
.discovery-book .discovery-dismiss button{font-size:11px;color:var(--color-faint)}
.discovery-grid.is-landscape{grid-template-columns:repeat(auto-fill,minmax(290px,1fr));gap:28px 24px}
.is-landscape .discovery-cover{aspect-ratio:460/215}
.is-landscape .discovery-cover img{object-fit:contain}
.is-landscape .discovery-cover:hover img{transform:none}
.is-landscape .discovery-cover b{bottom:8px;left:8px;padding:4px 7px}
.discovery-feedback{margin:16px 0;padding:20px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper)}
.discovery-feedback header{display:flex;justify-content:space-between;gap:16px;align-items:start}
.discovery-feedback h3{font-size:15px;font-weight:500;margin-bottom:8px}
.discovery-feedback p{font-size:12px;color:var(--color-muted);line-height:1.8}
.discovery-feedback ul{list-style:none;margin-top:16px;max-height:320px;overflow:auto}
.discovery-feedback li{display:flex;align-items:center;gap:16px;padding:12px 0;border-top:1px solid var(--color-hairline);font-size:13px}
.discovery-feedback li>span{flex:1;min-width:0;overflow-wrap:anywhere}
.discovery-feedback small{color:var(--color-muted);white-space:nowrap}
.discovery-feedback li button,.discovery-feedback-notice button{color:var(--color-beacon);font-size:12px;flex:none}
.discovery-feedback-notice{font-size:12px;color:var(--color-muted);line-height:1.8;margin:12px 0}
.discovery-feedback-notice button{margin-left:12px;text-decoration:underline}
@container(max-width:680px){.discovery-grid.is-landscape{grid-template-columns:repeat(2,minmax(0,1fr));gap:24px 16px}.discovery-feedback li{gap:8px;flex-wrap:wrap}}
@container(max-width:380px){.discovery-grid.is-landscape{grid-template-columns:1fr}}
</style>
