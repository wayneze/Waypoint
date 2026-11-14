import { expect, test, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import { cacheGet, cacheSet } from './fetchCache';
import { discoverDeals, discoverGames, gamePrices, eligibleOffer, localizeMarketGame, marketGuide, ownedMarketGame, quoteFresh, validPreferences, type GameQuotes, type MarketGame, type PriceOffer, type PriceRule } from './discovery';
import type { GameRecord, ObsidianGameGuide } from '../types';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('./fetchCache', () => ({ cacheGet: vi.fn(() => null), cacheSet: vi.fn() }));
const now = Date.parse('2026-11-08T08:00:00Z');
const offer: PriceOffer = { country:'JP',shop_id:61,shop:'Steam',price:{amount:3000,currency:'JPY'},discount:50,url:'https://store.steampowered.com/app/1/',
  updated_at:'2026-09-01T00:00:00Z',observed_at:new Date(now).toISOString(),drm:['Steam'] };
const rule: PriceRule = {enabled:true,mode:'target',target:3000,shop:'all',country:'JP',currency:'JPY'};

test('only a current, comparable, in-scope price can trigger a target or store-low alert', () => {
  expect(quoteFresh(offer,now)).toBe(true); // 老变价时间不影响本次成功查询。
  expect(eligibleOffer(offer,rule,now)).toBe(true);
  expect(eligibleOffer({...offer,expires_at:new Date(now-1).toISOString()},rule,now)).toBe(false);
  expect(eligibleOffer({...offer,observed_at:new Date(now-25*3600_000).toISOString()},rule,now)).toBe(false);
  expect(eligibleOffer({...offer,price:{amount:3,currency:'USD'}},rule,now)).toBe(false);
  expect(eligibleOffer({...offer,country:'US'},rule,now)).toBe(false);
  expect(eligibleOffer({...offer,price:{amount:3,currency:'CNY'}},rule,now)).toBe(false);
  expect(eligibleOffer({...offer,price:{amount:NaN,currency:'JPY'}},rule,now)).toBe(false);
  expect(eligibleOffer(offer,{...rule,shop:'35'},now)).toBe(false);
  expect(eligibleOffer(offer,{...rule,mode:'low'},now)).toBe(false);
  expect(eligibleOffer({...offer,store_low:{amount:3000,currency:'JPY'}},{...rule,mode:'low'},now)).toBe(true);
  expect(eligibleOffer({...offer,store_low:{amount:3000,currency:'CNY'}},{...rule,mode:'low'},now)).toBe(false);
});

test('discovery requires an explicit preference and carries types and market into both feeds', async()=>{
  expect(validPreferences({interests:[],all:false,country:'JP'})).toBe(false);
  expect(validPreferences({interests:[],all:true,country:'JP'})).toBe(true);
  expect(validPreferences({interests:['unknown'],all:false,country:'JP'})).toBe(false);
  expect(validPreferences({interests:['visual_novel'],all:false,country:'CN'})).toBe(false);
  await discoverGames('bangumi','',0,'JP',['visual_novel']);
  expect(invoke).toHaveBeenLastCalledWith('discover_games',{source:'bangumi',query:'',offset:0,country:'JP',interests:['visual_novel']});
  await discoverDeals(25,'HK',['puzzle']);
  expect(invoke).toHaveBeenLastCalledWith('discover_deals',{offset:25,country:'HK',interests:['puzzle']});
});

test('prices isolate cache and concurrent requests by market', async()=>{
  const game:MarketGame={id:'steam:1',steam_id:'1',title:'Work',url:'',cover:''};
  let finish!: (value:GameQuotes)=>void;
  vi.mocked(invoke).mockReset().mockImplementationOnce(()=>new Promise<GameQuotes>(resolve=>{finish=resolve;}))
    .mockResolvedValue({country:'US',offers:[],checked_at:new Date().toISOString()});
  const japan=gamePrices(game,false,'JP');
  expect(gamePrices(game,false,'JP')).toBe(japan);
  const usa=gamePrices(game,false,'US');
  expect(invoke).toHaveBeenCalledTimes(2);
  expect(invoke).toHaveBeenLastCalledWith('game_prices',{steamId:'1',itadId:null,country:'US'});
  const result:GameQuotes={country:'JP',offers:[offer],checked_at:new Date().toISOString()};
  finish(result);await Promise.all([japan,usa]);
  expect(cacheSet).toHaveBeenCalledWith('prices:JP:false::1',result);
  vi.mocked(cacheGet).mockImplementation(key=>key==='prices:JP:false::1'?{data:result,fetchedAt:Date.now()}:null);
  await expect(gamePrices(game,false,'JP')).resolves.toBe(result);
  expect(invoke).toHaveBeenCalledTimes(2);
  await gamePrices(game,false,'US');
  expect(invoke).toHaveBeenCalledTimes(3);
});

test('Chinese note overlays require a unique exact work identity; a note or backlog is not ownership', () => {
  const game: MarketGame={id:'steam:1',steam_id:'1',title:'Volume One',cover:'',url:''};
  const guide=marketGuide(game);
  const cn: ObsidianGameGuide={...guide,id:'note1',title:'第一作',links:{steam:'https://store.steampowered.com/app/1/'}};
  expect(localizeMarketGame(game,[cn]).title).toBe('第一作');
  expect(localizeMarketGame({...game,id:'steam:2',steam_id:'2'},[cn]).title).toBe('Volume One');
  expect(localizeMarketGame(game,[cn,{...cn,id:'duplicate'}]).title).toBe('Volume One');
  const record: GameRecord={id:'record',title:'Volume One',category:'single',status:'backlog',progress:'',rating:0};
  expect(ownedMarketGame(game,[record],[])).toBe(false);
  expect(ownedMarketGame(game,[],[{vault_item_id:'account',appid:1,name:'Volume One',playtime_forever:0,playtime_2weeks:0,img_icon_url:''}])).toBe(true);
});
