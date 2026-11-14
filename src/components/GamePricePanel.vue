<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { ArrowUpRight, Bell, Check, Search } from '@lucide/vue';
import { defaultPriceRule, marketRegions, regionInfo, discoverGames, gamePrices, moneyLabel, quoteFresh, sameMarketGame, type GameQuotes, type MarketGame, type PriceRule } from '../services/discovery';
import { openExternalUrl } from '../services/systemApi';
import { checkPriceWatches, hydrateDiscovery, marketCountry, marketIsConnected, priceWatches, savePriceWatch } from '../workspace/discovery';
const props=defineProps<{game:MarketGame}>();
const emit=defineEmits<{connect:[]}>();
const binding=ref<MarketGame>();
const quotes=ref<GameQuotes>();
const busy=ref(false),error=ref(''),notice=ref(''),saving=ref(false);
const query=ref(''),results=ref<MarketGame[]>([]),searching=ref(false);
const rule=ref<PriceRule>(defaultPriceRule(marketCountry.value,true));
const ready=ref(false);
const region=computed(()=>regionInfo(rule.value.country));
const currencies=computed(()=>[...new Set([region.value.currency,...quotes.value?.offers.map(o=>o.price.currency)||[],rule.value.currency])]);
const saved=computed(()=>priceWatches.value.find(w=>sameMarketGame(w.game,props.game)));
const target=computed(()=>binding.value || saved.value?.game || props.game);
let generation=0;
let searchGeneration=0;
onBeforeUnmount(()=>{generation++;searchGeneration++;});
function reset(){
  generation++;searchGeneration++;searching.value=false;binding.value=undefined;quotes.value=undefined;results.value=[];notice.value='';error.value='';
  query.value=props.game.metadata?.original_title || props.game.title;
  rule.value={...(saved.value?.rule || defaultPriceRule(marketCountry.value,true))};
  void refresh();
}
watch(()=>props.game,()=>{if(ready.value)reset();});
onMounted(async()=>{try{await hydrateDiscovery();ready.value=true;reset();}catch(e){error.value=String(e);}});
watch(marketIsConnected,()=>{if(ready.value)void refresh();});
function changeRegion() {
  searchGeneration++;searching.value=false;results.value=[];
  rule.value.currency=region.value.currency;rule.value.target=0;
  notice.value='地区已切换，请重新设置目标价并保存。';void refresh();
}
async function refresh(force=false) {
  const ticket=++generation;
  quotes.value=undefined;error.value='';busy.value=false;
  if(!target.value.steam_id && !target.value.itad_id) return;
  busy.value=true;
  try {const result=await gamePrices(target.value,marketIsConnected.value,rule.value.country,force);if(ticket===generation) quotes.value=result;}
  catch(e){if(ticket===generation)error.value=String(e);}
  finally{if(ticket===generation)busy.value=false;}
}
async function search() {
  if(!query.value.trim()) return;
  const ticket=++searchGeneration;searching.value=true;error.value='';results.value=[];
  try {
    const {games:rows}=await discoverGames(marketIsConnected.value?'itad':'steam',query.value,0,rule.value.country);
    if(ticket===searchGeneration){results.value=rows;if(!rows.length)notice.value='没有找到商品，试试原名或英文名。';}
  }catch(e){if(ticket===searchGeneration)error.value=String(e);}
  finally{if(ticket===searchGeneration)searching.value=false;}
}
function bind(game:MarketGame) {
  binding.value={...props.game,steam_id:game.steam_id,itad_id:game.itad_id,url:game.url,product_title:game.title};
  notice.value=`本次选择：${game.title}。关注时将保存此商品对应关系。`;
  results.value=[];void refresh();
}
async function save() {
  if(saving.value) return;
  if(rule.value.mode==='target' && (!Number.isFinite(rule.value.target) || rule.value.target<0)) {error.value='请输入有效的目标价';return;}
  saving.value=true;error.value='';
  try {
    await savePriceWatch({...target.value,itad_id:quotes.value?.itad_id || target.value.itad_id},{...rule.value});
    notice.value=rule.value.enabled?'已保存提醒。航标运行时每 30 分钟检查，关机或退出后暂停。':'已暂停此作品的价格提醒。';
    if(rule.value.enabled) void checkPriceWatches(true);
  }catch(e){error.value=String(e);}
  finally{saving.value=false;}
}
async function open(url:string) {try{await openExternalUrl(url);}catch(e){error.value=String(e);}}
</script>
<template>
  <section class="price-panel" aria-label="购买与比价">
    <header><div><h3>购买与比价</h3><p>{{ region.label }} · 原币报价 · 同一商品版本</p></div><button type="button" :disabled="busy" @click="refresh(true)">刷新报价</button></header>
    <label class="price-region">商店地区<select v-model="rule.country" :disabled="!ready || saving" @change="changeRegion"><option v-for="area in marketRegions" :key="area.id" :value="area.id">{{ area.label }} · {{ area.currency }}</option></select></label>
    <p v-if="!marketIsConnected" class="price-note">Steam 所选地区可直接查询。<button type="button" @click="emit('connect')">连接价格来源</button>后比较 Epic、GOG 与商店历史低价。</p>
    <p v-if="target.product_title" class="price-note">对应商品：{{ target.product_title }}</p>
    <details :open="!target.steam_id && !target.itad_id" class="price-binding">
      <summary>{{ target.steam_id || target.itad_id ? '核对 / 更换商品版本' : '选择对应的商店商品' }}</summary>
      <p>核对本篇、续作与发行版本后选择；合集、DLC 和不同版本分别比较。</p>
      <form @submit.prevent="search"><label><Search /><input v-model="query" aria-label="商店商品名称" placeholder="可使用原名或英文名" /></label><button :disabled="searching">{{ searching?'搜索中…':'搜索商品' }}</button></form>
      <div class="price-candidates"><button v-for="item in results" :key="item.id" type="button" @click="bind(item)"><span>{{ item.title }}</span><small>使用此商品</small></button></div>
    </details>
    <p v-if="busy" role="status">正在核对当前地区报价…</p>
    <p v-if="error" class="price-error" role="alert">{{ error }}</p>
    <p v-if="notice" class="price-note" role="status">{{ notice }}</p>
    <template v-if="quotes">
      <p v-if="quotes.warning" class="price-note">{{ quotes.warning }}</p>
      <p v-if="!quotes.offers.length" class="price-note">已覆盖商店暂无可用报价，不代表免费或已下架。</p>
      <div class="price-offers">
        <article v-for="offer in quotes.offers" :key="offer.shop_id + offer.url">
          <div><strong>{{ offer.shop }}</strong><small>{{ offer.drm.join(' / ') || '商店版本' }} · {{ quoteFresh(offer)?'当前报价':'报价已过期，请到商店核对' }}</small></div>
          <div class="price-amount"><strong>{{ moneyLabel(offer.price) }}</strong><small v-if="offer.discount">优惠 {{ offer.discount }}%</small></div>
          <p>该店历史最低 {{ moneyLabel(offer.store_low) }}<small>{{ new Date(offer.observed_at).toLocaleString('zh-CN') }} 查询</small></p>
          <button type="button" :aria-label="'前往'+offer.shop+'查看商品'" @click="open(offer.url)">查看商品<ArrowUpRight /></button>
        </article>
      </div>
      <p class="price-note">报价覆盖 Steam / Epic / GOG，同币种比较。商店地区不保证版本未删改，请核对发行版本、内容说明与补丁；最终售价及激活地区以商店为准。</p>
    </template>
    <form v-if="target.steam_id || target.itad_id" class="price-watch" @submit.prevent="save">
      <h4><Bell />想玩与降价提醒</h4>
      <div class="price-rule">
        <label>提醒条件<select v-model="rule.mode"><option value="target">不高于目标价</option><option value="low">达到该店历史最低</option></select></label>
        <label>提醒币种<select v-model="rule.currency" @change="rule.target=0"><option v-for="currency in currencies" :key="currency" :value="currency">{{ currency }}</option></select></label>
        <label v-if="rule.mode==='target'">目标价 · {{ rule.currency }}<input v-model.number="rule.target" type="number" min="0" step="0.01" required /></label>
        <label>商店<select v-model="rule.shop"><option value="all">已覆盖商店</option><option value="61">Steam</option><option value="16">Epic</option><option value="35">GOG</option></select></label>
      </div>
      <label class="price-enabled"><input v-model="rule.enabled" type="checkbox" />开启本地提醒</label>
      <p class="price-note">仅以 {{ region.label }} 的 {{ rule.currency }} 有效报价触发；同一价位不会反复提醒，价格进一步降低时再通知。历史价缺失时不判定史低。</p>
      <button type="submit" class="price-save" :disabled="saving"><Check />{{ saving?'保存中…':saved?'更新关注条件':'保存关注条件' }}</button>
    </form>
  </section>
</template>
<style scoped>
.price-panel{container-type:inline-size}.price-panel header{display:flex;justify-content:space-between;align-items:center;gap:16px;margin-bottom:16px}.price-panel h3{font-family:var(--font-display);font-size:22px;font-weight:500}.price-panel header p,.price-note,.price-binding p{color:var(--color-muted);font-size:12px;line-height:1.8;margin:8px 0}.price-region{display:flex;gap:10px;align-items:center;font-size:12px;margin:12px 0}.price-region select{background:var(--color-paper);color:var(--color-ink);padding:8px;border:1px solid var(--color-hairline);border-radius:4px}.price-panel button{cursor:pointer;color:var(--color-beacon);font-size:12px;min-height:32px}.price-panel button:disabled{opacity:.5;cursor:default}.price-note button{text-decoration:underline;margin-inline:4px}.price-binding{padding:12px 0;border-block:1px solid var(--color-hairline);margin:16px 0}.price-binding summary{cursor:pointer;font-size:12px;color:var(--color-muted)}.price-binding form{display:flex;gap:10px;margin-top:12px}.price-binding form label{display:flex;gap:8px;align-items:center;flex:1;min-width:0;border:1px solid var(--color-hairline);padding:7px 10px}.price-binding input{min-width:0;width:100%;background:transparent;font-size:12px}.price-candidates button{display:flex;width:100%;text-align:left;justify-content:space-between;gap:12px;padding:10px 0;border-bottom:1px solid var(--color-hairline)}.price-candidates small{color:var(--color-muted);flex:none}.price-offers article{display:grid;grid-template-columns:minmax(110px,1fr) auto;gap:10px;padding:16px 0;border-bottom:1px solid var(--color-hairline)}.price-offers strong{font-size:15px;font-weight:500}.price-offers small{display:block;font-size:11px;color:var(--color-muted);margin-top:5px}.price-amount{text-align:right;color:var(--color-beacon)}.price-offers p{font-size:12px;color:var(--color-muted)}.price-offers button{display:flex;align-items:center;gap:5px;justify-self:end}.price-watch{margin-top:26px;padding-top:20px;border-top:1px solid var(--color-hairline)}.price-watch h4{display:flex;align-items:center;gap:7px;font-size:15px;font-weight:500;margin-bottom:18px}.price-rule{display:flex;gap:12px;flex-wrap:wrap}.price-rule label{display:flex;flex:1;min-width:130px;flex-direction:column;gap:7px;font-size:12px;color:var(--color-muted)}.price-rule input,.price-rule select{padding:8px;background:var(--color-paper);border:1px solid var(--color-hairline);border-radius:4px;color:var(--color-ink);min-width:0;width:100%}.price-enabled{display:flex;gap:8px;align-items:center;margin-top:16px;font-size:12px}.price-save{display:flex;gap:7px;align-items:center;border:1px solid var(--color-hairline);border-radius:4px;padding:7px 12px;margin-top:12px}.price-error{color:var(--color-alert);font-size:12px;white-space:pre-wrap}svg{width:15px;height:15px;flex:none}:where(button,input,select,summary):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
