import { getTodayDateStr as todayStr } from './skyDailyService';
/**
 * 光遇季节页目录：wiki 奇妙之旅名单 + 向导树种类/近季价格。
 * 打卡与心愿只存在本地，不写回 wiki。
 */
import { fetchSkySeasons, type SkySeasonCatalog, type SkySeasonCatalogItem } from './skyApi';
import type { SeasonalGuide, SeasonalItem } from '../types';
import { cachedFetch } from './fetchCache';
import { progressEntries, progressMap, progressReplace } from './progress';

const CACHE_KEY = 'sky.seasons';
const WISH_SCOPE = 'sky.season_wish';
const CUSTOM_SCOPE = 'sky.season_custom';

export type SeasonWishStatus = Record<string, { selected?: boolean; items?: Record<string, boolean> }>;



function statusLabel(status?: string): string {
  if (status === 'live') return '进行中';
  if (status === 'upcoming') return '未开';
  return '已常驻';
}

export function loadSeasonWishStatus(characterId = ""): SeasonWishStatus {
  return characterId ? progressMap(WISH_SCOPE, characterId) : {};
}

export function loadCustomSeasons(): SeasonalGuide[] {
  return progressEntries<SeasonalGuide>(CUSTOM_SCOPE).map(([, s]) => s).filter(s => s && s.isCustom);
}

export function saveCustomSeasons(seasons: SeasonalGuide[]) {
  progressReplace(CUSTOM_SCOPE, seasons.filter(s => s.isCustom).map(s => [s.id, {
    ...s, itemList: s.itemList.map(i => ({ ...i, obtained: false })),
  }] as [string, unknown]));
}

function catalogItemToGuide(row: SkySeasonCatalogItem, wish: SeasonWishStatus): SeasonalGuide {
  const st = wish[row.id] || {};
  const itemList: SeasonalItem[] = (row.items || []).map(it => ({
    id: it.id,
    name: it.name,
    costDesc: it.cost_desc || (it.slot === '季卡' ? '季卡' : ''),
    candles: it.candles || 0,
    ascended: it.ascended || 0,
    hearts: it.hearts || 0,
    obtained: st.items?.[it.id] === true,
    icon: it.icon || '',
    slot: it.slot,
    spirit: it.spirit,
    kind: it.kind,
    wear: it.wear || ''
  }));
  const candles = itemList.reduce((s, i) => s + (i.costDesc.includes('白蜡') ? i.candles : 0), 0);
  const hearts = itemList.reduce((s, i) => s + (i.costDesc.includes('爱心') ? i.hearts : 0), 0);
  return {
    id: row.id,
    sourceUrl: row.url,
    season: row.title,
    shortName: row.title,
    realm: row.realm || '',
    loc: row.loc || row.realm || '',
    quests: row.quests || '',
    benefit: row.slogan || '',
    items: itemList.map(i => i.name).join('、'),
    itemList,
    task: row.quests || '',
    priority: statusLabel(row.status),
    candles,
    ascended: 0,
    hearts,
    selected: false,
    status: row.status,
    start: row.start,
    end: row.end,
    icon: row.icon || '',
    treeCandles: row.tree_candles || {}
  };
}

export function mergeSeasonCatalog(catalog: SkySeasonCatalog, wish: SeasonWishStatus, customs: SeasonalGuide[]): SeasonalGuide[] {
  const wiki = (catalog.seasons || []).map(row => catalogItemToGuide(row, wish));
  return [...wiki, ...customs.map(g => ({ ...g, itemList: g.itemList.map(i => ({ ...i, obtained: wish[g.id]?.items?.[i.id] === true })) }))];
}

export async function getSkySeasonCatalog(
  force = false,
  onUpdate?: (catalog: SkySeasonCatalog) => void
): Promise<SkySeasonCatalog | null> {
  return cachedFetch<SkySeasonCatalog>({
    key: CACHE_KEY,
    label: '光遇季节目录',
    fetch: fetchSkySeasons,
    fresh: data => data.as_of === todayStr(),
    valid: data => Array.isArray(data.seasons) && data.seasons.length > 0,
    onUpdate,
    force
  });
}

/** 每日季蜡：四个每日任务各 1 根，四堆季节蜡烛堆合 1 根；季卡每天再多领 1 根（wiki 季节页「每日任务」）。 */
const DAILY_CANDLES = 5;
const DAILY_CANDLES_PASS = 6;

export interface SeasonGap {
  /** 还差的季蜡。先祖有待换物品时按买齐他整棵树、减去已换物品价格估，含染料、动作这些格子。 */
  wax: number;
  white: number;
  hearts: number;
  pending: number;
  /** 只在季节进行中给出 */
  daysLeft: number | null;
  daysNeeded: number | null;
  daysNeededPass: number | null;
}

function dayDiff(from: string, to: string): number {
  const a = Date.parse(`${from}T00:00:00`);
  const b = Date.parse(`${to}T00:00:00`);
  return Math.round((b - a) / 86400000);
}

export function seasonGap(guide: SeasonalGuide, today = todayStr()): SeasonGap {
  const items = guide.itemList || [];
  const pending = items.filter(i => !i.obtained);
  const trees = guide.treeCandles || {};
  let wax = 0;
  let white = 0;
  let hearts = 0;
  const treeSpirits = new Set<string>();
  for (const i of pending) {
    const desc = i.costDesc || '';
    // 只剩季卡物品时不再算树：季卡物品跟着节点送，不另花季蜡。
    if (desc.includes('季蜡')) {
      if (i.spirit && trees[i.spirit]) treeSpirits.add(i.spirit);
      else wax += i.candles;
    }
    if (desc.includes('白蜡')) white += i.candles;
    if (desc.includes('爱心') || desc.includes('季心')) hearts += i.hearts;
  }
  for (const spirit of treeSpirits) {
    const paid = items
      .filter(i => i.obtained && i.spirit === spirit && (i.costDesc || '').includes('季蜡'))
      .reduce((s, i) => s + i.candles, 0);
    wax += Math.max(0, trees[spirit] - paid);
  }
  const live = guide.status === 'live' && !!guide.end;
  const daysLeft = live ? Math.max(0, dayDiff(today, guide.end!) + 1) : null;
  return {
    wax,
    white,
    hearts,
    pending: pending.length,
    daysLeft,
    daysNeeded: live ? Math.ceil(wax / DAILY_CANDLES) : null,
    daysNeededPass: live ? Math.ceil(wax / DAILY_CANDLES_PASS) : null,
  };
}

export function seasonGapText(gap: SeasonGap): string {
  if (!gap.pending) return '';
  const parts: string[] = [];
  if (gap.wax) parts.push(`${gap.wax}季蜡`);
  if (gap.white) parts.push(`${gap.white}白蜡`);
  if (gap.hearts) parts.push(`${gap.hearts}心`);
  return parts.length ? `约差 ${parts.join(' / ')}` : `${gap.pending} 件待换`;
}

/** 进行中的季节：还要攒几天，和剩几天比。 */
export function seasonEtaText(gap: SeasonGap): string {
  if (!gap.pending || !gap.wax || gap.daysLeft === null || gap.daysNeeded === null) return '';
  return `每天做满约 ${gap.daysNeeded} 天，季卡 ${gap.daysNeededPass} 天 · 剩 ${gap.daysLeft} 天`;
}
