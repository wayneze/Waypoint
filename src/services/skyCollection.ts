import type { SeasonalGuide, SeasonalItem } from '../types';
import type { SkyCalendarItem } from './skyApi';
import { matchEventGuide } from './skyEventService';
import { describeSkyOutfit, loadSkyInventory, skyOutfitCollectionMatch } from './skyInventory';
import { loadSkySpirits, setSkySpiritProgress } from './skyData';
import { progressGet, setProgressEntries } from './progress';

export type CollectionSource = 'season' | 'event' | 'permanent' | 'synced';
export interface SkyCollectionEntry {
  key: string;
  name: string;
  kind: string;
  icon: string;
  wear?: string;
  source: CollectionSource;
  sourceId: string;
  sourceName: string;
  sourceUrl?: string;
  scope: string;
  itemId: string;
  spirit?: string;
  cost: string;
  location?: string;
  period?: string;
  method?: string;
  status?: string;
  owned: boolean;
  wanted: boolean;
  wishKeys?: string[];
  synced?: boolean;
}
export const COLLECTION_KINDS: Record<string, string> = {
  wing: '斗篷', hair: '发型', body: '服装', mask: '面具', hat: '帽饰', horn: '头饰',
  face: '面饰', neck: '颈饰', feet: '鞋子', prop: '道具', tail: '尾饰', action: '动作', other: '其他',
};
const kindWords: [string, RegExp][] = [
  ['wing', /斗篷|披风|披肩|cape|wing/i], ['hair', /发型|头发|hair/i],
  ['feet', /鞋|靴|feet|shoes/i], ['body', /裤|裙|服装|长袍|礼服|衣|body|outfit/i],
  ['mask', /面具|mask/i], ['hat', /帽|hat/i], ['face', /面饰|眼镜|face/i],
  ['neck', /领结|围巾|项链|颈饰|neck/i], ['tail', /尾饰|tail/i],
  ['horn', /头饰|发饰|头巾|horn/i], ['action', /动作|表情|action|emote/i],
  ['prop', /道具|背饰|手持|乐器|家具|琴|鼓|笛|伞|灯|椅|桌|火盆|吊床|prop/i],
];
function kindOf(item: Pick<SeasonalItem, 'name' | 'kind'>): string {
  const value = item.kind || item.name;
  if (value in COLLECTION_KINDS) return value;
  return kindWords.find(([, pattern]) => pattern.test(value))?.[0] || 'other';
}
export function collectionKey(scope: string, sourceId: string, itemId: string): string {
  return JSON.stringify([scope, sourceId, itemId]);
}
function wanted(key: string, characterId: string): boolean {
  return !!characterId && progressGet('sky.collection_wanted', key, characterId) === true;
}
export async function setSkyGuideItems(characterId: string, scope: string, guideId: string, ids: string[], owned: boolean) {
  const record = progressGet<{ items?: Record<string, boolean> }>(scope, guideId, characterId) || {};
  await setProgressEntries(scope, [[guideId, { ...record, items: { ...record.items, ...Object.fromEntries(ids.map(id => [id, owned])) } }]], characterId);
}

/** 图鉴、活动卡片和星盘读取同一份收藏；订单快照只补充目录中未出现的物品。 */
export function skyCollectionEntries(characterId: string, seasons: SeasonalGuide[], events: SeasonalGuide[]): SkyCollectionEntry[] {
  const entries = new Map<string, SkyCollectionEntry>();
  for (const [source, scope, guides] of [
    ['season', 'sky.season_wish', seasons], ['event', 'sky.event_wish', events],
  ] as const) {
    for (const guide of guides) {
      const record = characterId ? progressGet<{ items?: Record<string, boolean> }>(scope, guide.id, characterId) : undefined;
      for (const item of guide.itemList) {
        const key = collectionKey(scope, guide.id, item.id);
        entries.set(key, { key, name: item.name, kind: kindOf(item), icon: item.icon || '', wear: item.wear,
          source, sourceId: guide.id, sourceName: guide.season, sourceUrl: guide.sourceUrl, scope, itemId: item.id,
          spirit: item.spirit === guide.season ? undefined : item.spirit,
          cost: item.costDesc === item.kind || Object.values(COLLECTION_KINDS).includes(item.costDesc) ? '' : item.costDesc,
          method: item.slot === '商城' ? '商城购买' : item.slot === '季卡' ? '季卡奖励' : '兑换获取',
          location: source === 'event' ? undefined : guide.loc,
          period: source === 'event' ? [guide.start, guide.end].filter(Boolean).join(' — ') : undefined, status: guide.status,
          owned: record?.items?.[item.id] === true, wanted: wanted(key, characterId) });
      }
    }
  }
  for (const spirit of loadSkySpirits(characterId)) {
    if (!spirit.coreItem) continue;
    const key = collectionKey('sky.spirits', spirit.id, 'core');
    entries.set(key, { key, name: spirit.coreItem, kind: kindOf({ name: spirit.coreItem }), icon: '',
      source: 'permanent', sourceId: spirit.id, sourceName: spirit.map, scope: 'sky.spirits', itemId: 'core',
      spirit: spirit.name, cost: spirit.coreCost, location: spirit.location,
      owned: !!spirit.itemObtained, wanted: wanted(key, characterId) });
  }
  for (const outfit of loadSkyInventory(characterId)?.outfits || []) {
    const display = describeSkyOutfit(outfit);
    const match = skyOutfitCollectionMatch(outfit, [...seasons, ...events]);
    const scope = match && seasons.includes(match.guide) ? 'sky.season_wish' : 'sky.event_wish';
    const key = match ? collectionKey(scope, match.guide.id, match.item.id) : collectionKey('sky.outfit_status', '', outfit.name);
    const existing = entries.get(key);
    if (existing && match) {
      const explicit = progressGet<{ items?: Record<string, boolean> }>(scope, match.guide.id, characterId)?.items?.[match.item.id];
      existing.owned = explicit ?? progressGet<boolean>('sky.outfit_status', outfit.name, characterId) ?? true;
      const rawKey = collectionKey('sky.outfit_status', '', outfit.name);
      existing.wishKeys = [...new Set([...(existing.wishKeys || [key]), rawKey])];
      existing.wanted ||= wanted(rawKey, characterId);
      existing.synced = true;
      existing.icon ||= display.image;
      existing.kind = outfit.kind;
    } else {
      const label = display.label.split(' · ');
      entries.set(key, { key, name: label[0] || display.label, kind: outfit.kind, icon: display.image,
        source: 'synced', sourceId: '', sourceName: label[1] || '已同步外观', scope: 'sky.outfit_status', itemId: outfit.name,
        cost: '', owned: progressGet<boolean>('sky.outfit_status', outfit.name, characterId) ?? true,
        wanted: wanted(key, characterId), synced: true });
    }
  }
  return [...entries.values()];
}
export function setSkyCollectionOwned(entry: SkyCollectionEntry, characterId: string, owned: boolean) {
  if (entry.scope === 'sky.outfit_status') return setProgressEntries(entry.scope, [[entry.itemId, owned]], characterId);
  if (entry.scope === 'sky.spirits') return setSkySpiritProgress(characterId, entry.sourceId, { itemObtained: owned });
  return setSkyGuideItems(characterId, entry.scope, entry.sourceId, [entry.itemId], owned);
}
export function setSkyCollectionWanted(entry: SkyCollectionEntry, characterId: string, value: boolean) {
  return setProgressEntries('sky.collection_wanted', (entry.wishKeys || [entry.key]).map(key => [key, value]), characterId);
}

/** 只把同档期、唯一对应的图鉴物品接到活动预览，不以近似名称猜测。 */
export function calendarCollectionEntries(row: SkyCalendarItem, entries: SkyCollectionEntry[], seasons: SeasonalGuide[], events: SeasonalGuide[]) {
  if (row.region === 'intl') return [];
  if (row.kind === 'event') {
    const guide = matchEventGuide(row, events);
    return guide ? entries.filter(e => e.source === 'event' && e.sourceId === guide.id) : [];
  }
  if (row.kind === 'season') {
    const guide = seasons.find(g => g.season === row.title && g.start === row.start);
    return guide ? entries.filter(e => e.source === 'season' && e.sourceId === guide.id) : [];
  }
  if (row.kind === 'reprint') return entries.filter(e => e.source === 'season' && !!e.spirit && row.names.includes(e.spirit));
  return [];
}
