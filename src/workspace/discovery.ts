import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '../services/api';
import { hydrateKv, kvGetJson, kvSetConfirmed } from '../services/kv';
import { defaultPriceRule, eligibleOffer, gamePrices, marketConnected, marketKeys, marketRegions, moneyLabel, regionInfo, sameMarketGame, validPreferences, type DiscoveryFeedback, type DiscoveryFeedbackKind, type DiscoveryPreferences, type MarketGame, type PriceRule, type PriceWatch } from '../services/discovery';
import { showToast } from './core';

const KEY = 'waypoint_price_watches';
const PREFERENCES_KEY = 'waypoint_discovery_preferences';
const FEEDBACK_KEY = 'waypoint_discovery_feedback';
export const discoveryPreferences = ref<DiscoveryPreferences | null>(null);
export const discoveryFeedback = ref<DiscoveryFeedback[]>([]);
export const marketCountry = computed(()=>discoveryPreferences.value?.country || 'JP');
export const discoveryReady = ref(false);
export const priceWatches = ref<PriceWatch[]>([]);
export const marketIsConnected = ref(false);
export const checkingPrices = ref(false);
export const priceMonitorError = ref('');
let writeJob: Promise<void> = Promise.resolve();
let feedbackJob: Promise<void> = Promise.resolve();
let monitorJob: Promise<void> | undefined;
let hydration: Promise<void> | undefined;
export function hydrateDiscovery() {
  return hydration ??= (async () => {
    await hydrateKv();
    const stored=kvGetJson<unknown>(KEY,[]);
    const prefs=kvGetJson<unknown>(PREFERENCES_KEY,null);
    const feedback=kvGetJson<unknown>(FEEDBACK_KEY,[]);
    discoveryFeedback.value=Array.isArray(feedback)?feedback.filter(row=>row && typeof row.title==='string'
      && ['owned','uninterested'].includes(row.kind) && Array.isArray(row.keys) && row.keys.length
      && row.keys.every((key:unknown)=>typeof key==='string' && key.length>0)):[];
    discoveryPreferences.value=validPreferences(prefs)?prefs:null;
    priceWatches.value = Array.isArray(stored) ? stored.filter(w=>w?.game?.id && w?.rule && typeof w.rule.enabled==='boolean').map(w=>
      marketRegions.some(r=>r.id===w.rule.country) && typeof w.rule.currency==='string' ? w
        : {game:w.game,rule:defaultPriceRule(marketCountry.value),error:'原提醒未记录地区与币种，请重新设置目标价。'}
    ) : [];
    marketIsConnected.value = isTauri() ? await marketConnected() : false;
    discoveryReady.value=true;
  })().catch(error=>{hydration=undefined;throw error;});
}
export async function saveDiscoveryPreferences(value: DiscoveryPreferences) {
  if(!validPreferences(value))throw new Error('请选择感兴趣的类型，或明确选择浏览全部类型。');
  await hydrateDiscovery();
  const next={...value,interests:value.all?[]:[...new Set(value.interests)]};
  await kvSetConfirmed(PREFERENCES_KEY,JSON.stringify(next));
  discoveryPreferences.value=next;
}
function mutate(update: (rows: PriceWatch[]) => PriceWatch[]): Promise<void> {
  const next = writeJob.catch(() => {}).then(async () => {
    await hydrateDiscovery();
    const rows = update(priceWatches.value);
    await kvSetConfirmed(KEY, JSON.stringify(rows));
    priceWatches.value = rows;
  });
  writeJob = next;
  return next;
}
function mutateFeedback(update:(rows:DiscoveryFeedback[])=>DiscoveryFeedback[]):Promise<void> {
  const next=feedbackJob.catch(()=>{}).then(async()=>{
    await hydrateDiscovery();
    const rows=update(discoveryFeedback.value);
    await kvSetConfirmed(FEEDBACK_KEY,JSON.stringify(rows));
    discoveryFeedback.value=rows;
  });
  feedbackJob=next;
  return next;
}
export function saveDiscoveryFeedback(game:MarketGame,kind:DiscoveryFeedbackKind) {
  return mutateFeedback(rows=>{
    const keys=marketKeys(game);
    const existing=rows.filter(row=>row.keys.some(key=>keys.has(key)));
    for(const row of existing)for(const key of row.keys)keys.add(key);
    return [{keys:[...keys],title:game.title,kind},...rows.filter(row=>!existing.includes(row))];
  });
}
export const restoreDiscoveryGame=(entry:DiscoveryFeedback)=>mutateFeedback(rows=>rows.filter(row=>!row.keys.some(key=>entry.keys.includes(key))));
export function savePriceWatch(game: MarketGame, rule?: PriceRule) {
  return mutate(rows => {
    const old = rows.find(w=>sameMarketGame(w.game,game));
    const next: PriceWatch = { ...old,
      game: old && !rule ? { ...game,steam_id:old.game.steam_id || game.steam_id,itad_id:old.game.itad_id || game.itad_id,product_title:old.game.product_title } : game,
      rule: rule || old?.rule || defaultPriceRule(marketCountry.value) };
    if (rule && (JSON.stringify(rule) !== JSON.stringify(old?.rule)
      || game.steam_id !== old?.game.steam_id || game.itad_id !== old?.game.itad_id)) {
      next.alerted = undefined;
      next.checked_at = undefined;
      next.error = undefined;
      next.quotes = undefined;
    }
    return old ? rows.map(w=>w===old?next:w) : [...rows,next];
  });
}
export const removePriceWatch = (id: string) => mutate(rows=>rows.filter(w=>w.game.id!==id));
const watchIdentity = (w: PriceWatch) => JSON.stringify([w.rule,w.game.steam_id,w.game.itad_id]);
export function checkPriceWatches(force = false): Promise<void> {
  if (monitorJob) return monitorJob;
  monitorJob = (async () => {
    checkingPrices.value = true;
    priceMonitorError.value = '';
    try {
      await hydrateDiscovery();
      const due = priceWatches.value.filter(w=>w.rule.enabled && (force || !w.checked_at || Date.now()-Date.parse(w.checked_at)>=30*60_000));
      for (const watch of due) {
        const id=watch.game.id, identity=watchIdentity(watch);
        try {
          const quotes=await gamePrices(watch.game,marketIsConnected.value,watch.rule.country,true);
          const current=priceWatches.value.find(w=>w.game.id===id);
          if (!current || watchIdentity(current)!==identity) continue;
          const best=quotes.offers.filter(o=>eligibleOffer(o,current.rule)).sort((a,b)=>a.price.amount-b.price.amount)[0];
          const notify=best && (!current.alerted || current.alerted.currency!==best.price.currency || best.price.amount < current.alerted.amount);
          if (notify) {
            // 通知失败不能记成已发送；旧价格保留，下一次检查可重试。
            await invoke('trigger_notification',{title:`${current.game.title} 达到关注价`,
              body:`${best.shop} · ${moneyLabel(best.price)} · ${regionInfo(current.rule.country).label}报价。打开航标「想玩与提醒」查看版本及购买入口。`});
            showToast('想玩的游戏降价了',`${current.game.title} · ${best.shop} ${moneyLabel(best.price)}`);
          }
          await mutate(rows=>rows.map(w=>w.game.id===id && watchIdentity(w)===identity ? {
            ...w,game:{...w.game,itad_id:quotes.itad_id || w.game.itad_id},quotes,checked_at:quotes.checked_at,error:'',
            alerted:best && notify ? {amount:best.price.amount,currency:best.price.currency} : w.alerted,
          } : w));
        } catch(error) {
          await mutate(rows=>rows.map(w=>w.game.id===id && watchIdentity(w)===identity?{...w,error:String(error),checked_at:new Date().toISOString()}:w));
        }
      }
    } catch(error) { priceMonitorError.value=String(error); }
    finally { checkingPrices.value=false; }
  })().finally(()=>{monitorJob=undefined;});
  return monitorJob;
}
/** 应用生命周期唯一所有者；只监控显式开启的关注项，退出后停止。 */
export function startPriceMonitor() {
  let disposed=false;
  let timer: ReturnType<typeof setInterval> | undefined;
  const run=(force=false)=>{if(!disposed && isTauri()) void checkPriceWatches(force);};
  const onFocus=()=>run();
  void hydrateDiscovery().then(()=>{
    if(disposed) return;
    run(); timer=setInterval(()=>run(true),30*60_000);
    window.addEventListener('focus',onFocus);
  }).catch(error=>{priceMonitorError.value=String(error);});
  return ()=>{disposed=true;if(timer) clearInterval(timer);window.removeEventListener('focus',onFocus);};
}
