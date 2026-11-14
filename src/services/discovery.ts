import { invoke } from '@tauri-apps/api/core';
import type { GameMetadata, GameRecord, ObsidianGameGuide, SteamLibraryGame } from '../types';
import { sourceIdentity } from './gameLibrary';
import { cacheGet, cacheSet } from './fetchCache';
import options from '../data/discoveryOptions.json';

export const marketRegions = options.regions;
export const discoveryInterests = options.interests;
export type MarketRegion = 'JP' | 'US' | 'HK';
export interface DiscoveryPreferences { interests: string[]; all: boolean; country: MarketRegion }
export const regionInfo = (country: string) => marketRegions.find(r=>r.id===country)!;
export function validPreferences(value: unknown): value is DiscoveryPreferences {
  const v=value as DiscoveryPreferences | null;
  return !!v && marketRegions.some(r=>r.id===v.country) && typeof v.all==='boolean'
    && Array.isArray(v.interests) && (v.all || v.interests.length>0)
    && v.interests.every(id=>discoveryInterests.some(i=>i.id===id));
}
export const defaultPriceRule = (country: MarketRegion, enabled=false): PriceRule =>
  ({enabled,mode:'low',target:0,shop:'all',country,currency:regionInfo(country).currency});

export interface Money { amount: number; currency: string }
export interface PriceOffer {
  country: MarketRegion;
  shop_id: number; shop: string; price: Money; regular?: Money | null; store_low?: Money | null;
  discount: number; url: string; updated_at: string; observed_at: string; expires_at?: string | null; drm: string[];
}
export interface MarketGame {
  id: string; title: string; cover: string; url: string; steam_id?: string | null; itad_id?: string | null;
  metadata?: GameMetadata | null; offer?: PriceOffer | null;
  product_title?: string;
}
export interface GameQuotes { country: MarketRegion; offers: PriceOffer[]; itad_id?: string | null; checked_at: string; warning?: string | null }
export interface MarketPage { games: MarketGame[]; next_offset: number | null; warning?: string | null }
export type DiscoverySource = 'bangumi' | 'steam' | 'itad';
export const marketConnected = () => invoke<boolean>('market_connected');
export const connectMarket = (key: string) => invoke<void>('connect_market', { key });
export const discoverGames = (source: DiscoverySource, query: string, offset: number, country: MarketRegion, interests: string[] = []) =>
  invoke<MarketPage>('discover_games', { source, query, offset, country, interests });
export const discoverDeals = (offset: number, country: MarketRegion, interests: string[]) =>
  invoke<MarketPage>('discover_deals', { offset, country, interests });
const priceJobs = new Map<string, Promise<GameQuotes>>();
export function gamePrices(game: MarketGame, connected: boolean, country: MarketRegion, force = false): Promise<GameQuotes> {
  const key = `prices:${country}:${connected}:${game.itad_id || ''}:${game.steam_id || ''}`;
  const cached = cacheGet<GameQuotes>(key);
  if (!force && cached && Date.now() - cached.fetchedAt < 15 * 60_000) return Promise.resolve(cached.data);
  const running = priceJobs.get(key);
  if (running) return running;
  const job = invoke<GameQuotes>('game_prices', { steamId: game.steam_id || null, itadId: game.itad_id || null, country })
    .then(result => { cacheSet(key, result); return result; }).finally(() => { priceJobs.delete(key); });
  priceJobs.set(key, job);
  return job;
}
export const chineseText = (text: string) => /[\u3400-\u9fff]/.test(text) && !/[\u3040-\u30ff]/.test(text);
export const moneyLabel = (value?: Money | null) => value ? new Intl.NumberFormat('zh-CN', {
  style: 'currency', currency: value.currency, currencyDisplay:'code', maximumFractionDigits: 2,
}).format(value.amount) : '暂无记录';
export function quoteFresh(offer: PriceOffer, now = Date.now()): boolean {
  // 上游 timestamp 是价格变动时间；价格长期不变不代表报价失效。
  const updated = Date.parse(offer.observed_at), expiry = offer.expires_at ? Date.parse(offer.expires_at) : undefined;
  return Number.isFinite(updated) && updated <= now + 60_000 && now - updated <= 24 * 60 * 60_000
    && (expiry === undefined || Number.isFinite(expiry) && expiry > now);
}
export function eligibleOffer(offer: PriceOffer, rule: PriceRule, now = Date.now()) {
  if (offer.country!==rule.country || !quoteFresh(offer, now) || offer.price.currency !== rule.currency || !Number.isFinite(offer.price.amount) || offer.price.amount < 0) return false;
  if (rule.shop !== 'all' && offer.shop_id !== Number(rule.shop)) return false;
  return rule.mode === 'target' ? offer.price.amount <= rule.target
    : !!offer.store_low && offer.store_low.currency === rule.currency && Number.isFinite(offer.store_low.amount)
      && offer.store_low.amount >= 0 && offer.price.amount <= offer.store_low.amount;
}
export interface PriceRule { enabled: boolean; mode: 'target' | 'low'; target: number; shop: 'all' | '61' | '16' | '35'; country: MarketRegion; currency: string }
export interface PriceWatch {
  game: MarketGame; rule: PriceRule; checked_at?: string; error?: string;
  quotes?: GameQuotes; alerted?: { amount: number; currency: string };
}
export function marketKeys(game: MarketGame) {
  return new Set([game.id, ...(game.metadata ? [game.metadata, ...game.metadata.sources || []].map(s => `${s.source}:${s.source_id}`) : []),
    ...(game.steam_id ? [`steam:${game.steam_id}`] : []), ...(game.itad_id ? [`itad:${game.itad_id}`] : [])]);
}
export function sameMarketGame(a: MarketGame, b: MarketGame) {
  const keys=marketKeys(a);
  return [...marketKeys(b)].some(key=>keys.has(key));
}
export type DiscoveryFeedbackKind = 'uninterested' | 'owned';
export interface DiscoveryFeedback { keys: string[]; title: string; kind: DiscoveryFeedbackKind }
export function feedbackForGame(game: MarketGame, feedback: DiscoveryFeedback[]) {
  const keys=marketKeys(game);
  return feedback.find(row=>row.keys.some(key=>keys.has(key)));
}
/** 仅安装记录或 Steam 拥有列表能证明已入库；一篇攻略或个人待玩档案不能证明已拥有。 */
export function ownedMarketGame(game: MarketGame, records: GameRecord[], steam: SteamLibraryGame[]) {
  const keys = marketKeys(game);
  return steam.some(s => keys.has(`steam:${s.appid}`)) || records.some(r => {
    if (!r.library?.installations.length) return false;
    const meta = r.library.metadata;
    return r.library.installations.some(i => i.steam_id && keys.has(`steam:${i.steam_id}`))
      || !!meta && [meta, ...meta.sources || []].some(s => keys.has(`${s.source}:${s.source_id}`));
  });
}
/** 仅沿已确认的来源 ID 使用笔记中文信息，不用近似名称串接商品价格。 */
export function localizeMarketGame(game: MarketGame, guides: ObsidianGameGuide[]): MarketGame {
  const keys = marketKeys(game);
  const matched = guides.filter(g => Object.values(g.links || {}).some(url => {
    const identity = sourceIdentity(url); return identity && keys.has(identity.join(':'));
  }));
  if (matched.length !== 1) return game;
  const local = matched[0]!;
  const steam = Object.values(local.links || {}).map(sourceIdentity).find(s => s?.[0] === 'steam');
  return { ...game, title: local.title, cover: local.poster || game.cover, steam_id: game.steam_id || steam?.[1],
    metadata: game.metadata ? { ...game.metadata, title: local.title, description: local.summary || game.metadata.description } : game.metadata };
}
export function marketGuide(game: MarketGame): ObsidianGameGuide {
  const meta = game.metadata;
  return { id:game.id,title:game.title,original_title:meta?.original_title,aliases:meta?.aliases || [],category:meta?.source==='bangumi'||meta?.source==='vndb'?'galgame':'single',
    status:'已收藏',rating:meta?.score || 0,tags:meta?.tags || [],genres:[],file_path:'',relative_path:'',file_name:'',modified:'',
    poster:game.cover,summary:meta?.description,links:Object.fromEntries([
      ...(meta ? [meta,...meta.sources || []].map(s=>[s.source,s.url]) : []),
      ...(game.steam_id ? [['steam',`https://store.steampowered.com/app/${game.steam_id}/`]] : []),
    ]) };
}
export function guideMarketGame(guide: ObsidianGameGuide, metadata?: GameMetadata | null): MarketGame {
  const ids = [...Object.values(guide.links || {}).map(sourceIdentity).filter(s=>!!s),
    ...(metadata ? [metadata, ...metadata.sources || []].map(m=>[m.source,m.source_id] as const) : [])];
  const steam = ids.find(s=>s[0]==='steam');
  return {id:ids[0]?.join(':') || guide.id,title:guide.title,cover:guide.poster || '',url:guide.links?.steam || '',
    steam_id:steam?.[1],metadata:metadata || null};
}
