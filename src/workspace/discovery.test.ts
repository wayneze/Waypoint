import { beforeEach, expect, test, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { feedbackForGame, gamePrices, type GameQuotes, type MarketGame, type PriceRule } from '../services/discovery';
import { kvGetJson, kvSetConfirmed } from '../services/kv';
import { checkPriceWatches, discoveryFeedback, discoveryPreferences, marketCountry, restoreDiscoveryGame, saveDiscoveryFeedback, saveDiscoveryPreferences, hydrateDiscovery, priceWatches, removePriceWatch, savePriceWatch } from './discovery';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => undefined) }));
vi.mock('./core', () => ({ showToast: vi.fn() }));
vi.mock('../services/api', () => ({ isTauri: () => true }));
vi.mock('../services/kv', () => ({hydrateKv:vi.fn(async()=>{}),kvGetJson:vi.fn(()=>[]),kvSetConfirmed:vi.fn(async()=>{})}));
vi.mock('../services/fetchCache', () => ({ cacheGet:()=>null,cacheSet:vi.fn() }));
vi.mock('../services/discovery', async original => ({
  ...await original<typeof import('../services/discovery')>(),
  marketConnected:vi.fn(async()=>false),gamePrices:vi.fn(),
}));
const game: MarketGame={id:'steam:1',steam_id:'1',title:'第一作',cover:'',url:''};
const rule: PriceRule={enabled:true,mode:'target',target:50,shop:'all',country:'JP',currency:'JPY'};
function quotes(amount=30):GameQuotes {
  const now=new Date().toISOString();
  return {country:'JP',checked_at:now,offers:[{country:'JP',shop_id:61,shop:'Steam',price:{amount,currency:'JPY'},discount:50,url:'https://store.steampowered.com/app/1/',
    updated_at:now,observed_at:now,drm:['Steam']}]};
}
beforeEach(async()=>{
  await hydrateDiscovery();
  priceWatches.value=[];
  discoveryPreferences.value=null;
  discoveryFeedback.value=[];
  vi.mocked(invoke).mockReset().mockResolvedValue(undefined);
  vi.mocked(gamePrices).mockReset().mockResolvedValue(quotes());
  vi.mocked(kvSetConfirmed).mockReset().mockResolvedValue(undefined);
  vi.mocked(kvGetJson).mockReset().mockImplementation((_key,fallback)=>fallback);
});
test('feedback persists by exact identity across feeds and restart; restore and failed writes preserve truthful state', async()=>{
  const metadata={source:'bangumi',source_id:'7',sources:[{source:'steam',source_id:'1'}]} as unknown as NonNullable<MarketGame['metadata']>;
  const linked={...game,id:'bangumi:7',steam_id:undefined,metadata};
  await saveDiscoveryFeedback(linked,'uninterested');
  expect(feedbackForGame(game,discoveryFeedback.value)?.kind).toBe('uninterested');
  expect(feedbackForGame({...game,id:'steam:2',steam_id:'2'},discoveryFeedback.value)).toBeUndefined();
  await Promise.all([saveDiscoveryFeedback(game,'owned'),saveDiscoveryFeedback({...game,id:'steam:2',steam_id:'2',title:'第二作'},'uninterested')]);
  expect(discoveryFeedback.value).toHaveLength(2);
  expect(feedbackForGame(linked,discoveryFeedback.value)?.kind).toBe('owned');
  const persisted=JSON.parse(vi.mocked(kvSetConfirmed).mock.calls.slice(-1)[0]![1]);
  vi.mocked(kvSetConfirmed).mockRejectedValueOnce(new Error('disk full'));
  await expect(restoreDiscoveryGame(discoveryFeedback.value[0]!)).rejects.toThrow('disk full');
  expect(discoveryFeedback.value).toEqual(persisted);
  vi.mocked(kvGetJson).mockImplementation((key,fallback)=>key==='waypoint_discovery_feedback'?persisted:fallback);
  vi.resetModules();
  const restarted=await import('./discovery');
  await restarted.hydrateDiscovery();
  expect(restarted.discoveryFeedback.value).toEqual(persisted);
  await restarted.restoreDiscoveryGame(restarted.discoveryFeedback.value[0]!);
  expect(restarted.discoveryFeedback.value).toHaveLength(1);
  expect(JSON.parse(vi.mocked(kvSetConfirmed).mock.calls.slice(-1)[0]![1])).toEqual(restarted.discoveryFeedback.value);
});
test('first use has no chosen interests; explicit preferences persist and failed saves preserve the selection', async()=>{
  expect(discoveryPreferences.value).toBeNull();
  expect(marketCountry.value).toBe('JP');
  await expect(saveDiscoveryPreferences({country:'JP',interests:[],all:false})).rejects.toThrow('请选择');
  expect(kvSetConfirmed).not.toHaveBeenCalled();
  const prefs={country:'JP' as const,interests:['visual_novel','puzzle'],all:false};
  await saveDiscoveryPreferences(prefs);
  expect(discoveryPreferences.value).toEqual(prefs);
  expect(kvSetConfirmed).toHaveBeenLastCalledWith('waypoint_discovery_preferences',JSON.stringify(prefs));
  vi.mocked(kvSetConfirmed).mockRejectedValueOnce(new Error('disk full'));
  await expect(saveDiscoveryPreferences({...prefs,country:'US'})).rejects.toThrow('disk full');
  expect(marketCountry.value).toBe('JP');
  vi.mocked(kvGetJson).mockImplementation((key,fallback)=>key==='waypoint_discovery_preferences'?prefs:fallback);
  vi.resetModules();
  const restarted=await import('./discovery');
  await restarted.hydrateDiscovery();
  expect(restarted.discoveryPreferences.value).toEqual(prefs);
});
test('legacy prices keep the wanted work but never reinterpret a CNY target as JPY', async()=>{
  const legacy={game,rule:{enabled:true,mode:'target',target:50,shop:'all'},quotes:quotes()};
  vi.mocked(kvGetJson).mockImplementation((key,fallback)=>key==='waypoint_price_watches'?[legacy]:fallback);
  vi.resetModules();
  const restarted=await import('./discovery');
  await restarted.hydrateDiscovery();
  expect(restarted.priceWatches.value[0]).toMatchObject({game,rule:{enabled:false,country:'JP',currency:'JPY',target:0}});
  expect(restarted.priceWatches.value[0]?.quotes).toBeUndefined();
  await restarted.checkPriceWatches(true);
  expect(gamePrices).not.toHaveBeenCalled();
});
test('saved watches restore from persistence and failed writes never publish a saved state', async()=>{
  await savePriceWatch(game,rule);
  const persisted=JSON.parse(vi.mocked(kvSetConfirmed).mock.calls.slice(-1)[0]![1]);
  expect(persisted[0]).toMatchObject({game,rule});
  vi.mocked(kvSetConfirmed).mockRejectedValueOnce(new Error('disk full'));
  await expect(removePriceWatch(game.id)).rejects.toThrow('disk full');
  expect(priceWatches.value).toHaveLength(1);
  // 重新加载模块模拟应用重启，读取同一个持久化合同。
  vi.mocked(kvGetJson).mockReturnValueOnce(persisted);
  vi.resetModules();
  const restarted=await import('./discovery');
  await restarted.hydrateDiscovery();
  expect(restarted.priceWatches.value).toEqual(persisted);
});
test('alerts deduplicate across repeated checks and notify again only when the price drops further', async()=>{
  await savePriceWatch(game,rule);
  await checkPriceWatches(true);
  await checkPriceWatches(true);
  expect(invoke).toHaveBeenCalledTimes(1);
  vi.mocked(gamePrices).mockResolvedValueOnce(quotes(60));
  await checkPriceWatches(true);
  await checkPriceWatches(true);
  expect(invoke).toHaveBeenCalledTimes(1);
  vi.mocked(gamePrices).mockResolvedValueOnce(quotes(20));
  await checkPriceWatches(true);
  expect(invoke).toHaveBeenCalledTimes(2);
  expect(priceWatches.value[0]?.alerted?.amount).toBe(20);
});
test('a notification failure retries without consuming the alert; disabling during a fetch prevents notification', async()=>{
  await savePriceWatch(game,rule);
  vi.mocked(invoke).mockRejectedValueOnce(new Error('notifications unavailable'));
  await checkPriceWatches(true);
  expect(priceWatches.value[0]?.alerted).toBeUndefined();
  expect(priceWatches.value[0]?.error).toContain('notifications unavailable');
  await checkPriceWatches(true);
  expect(priceWatches.value[0]?.alerted?.amount).toBe(30);
  let finish!: (value:GameQuotes)=>void;
  vi.mocked(gamePrices).mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve;}));
  const running=checkPriceWatches(true);
  await vi.waitFor(()=>expect(finish).toBeTypeOf('function'));
  await savePriceWatch(game,{...rule,enabled:false});
  finish(quotes(10));await running;
  expect(invoke).toHaveBeenCalledTimes(2);
});
test('adding to wanted again preserves an explicitly selected store edition and never enables reminders', async()=>{
  await savePriceWatch({...game,steam_id:'2',product_title:'Selected edition'}, {...rule,enabled:false});
  await savePriceWatch(game);
  expect(priceWatches.value[0]).toMatchObject({game:{steam_id:'2',product_title:'Selected edition'},rule:{enabled:false}});
  await checkPriceWatches(true);
  expect(gamePrices).not.toHaveBeenCalled();
});
test('saved reminders retain their market when preferences change and discard quotes when their edition or rule changes', async()=>{
  await savePriceWatch(game,rule);
  await saveDiscoveryPreferences({interests:['puzzle'],all:false,country:'US'});
  await checkPriceWatches(true);
  expect(gamePrices).toHaveBeenLastCalledWith(game,false,'JP',true);
  expect(priceWatches.value[0]?.quotes?.country).toBe('JP');
  await savePriceWatch(game,{...rule,country:'US',currency:'USD',target:5});
  expect(priceWatches.value[0]?.quotes).toBeUndefined();
  expect(priceWatches.value[0]?.alerted).toBeUndefined();
  await checkPriceWatches(true);
  expect(invoke).toHaveBeenCalledTimes(1); // 日区报价不能触发美区规则。
});
