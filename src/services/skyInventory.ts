import catalog from '../data/skyOutfits.json';
import type { SeasonalGuide } from '../types';
import { progressGet, setProgressEntries } from './progress';

export interface SkyOwnedOutfit { id: string; name: string; kind: string }
export interface SkyBoostAssets {
  balances: Partial<Record<'candles' | 'season_candle' | 'heart' | 'prestige', number>>;
  outfits: SkyOwnedOutfit[] | null;
  warnings: string[];
}
export interface SkyInventory {
  outfits: SkyOwnedOutfit[];
  balances: SkyBoostAssets['balances'];
  outfitsAt?: string;
  balancesAt?: string;
}
export const SKY_OUTFIT_KINDS: Record<string, string> = {
  body: '服装', hair: '发型', hat: '帽饰', horn: '头饰', mask: '面具', prop: '道具',
  wing: '斗篷', face: '面饰', feet: '鞋子', neck: '颈饰', tail: '尾饰',
};
export const SKY_CURRENCIES = { candles: '白蜡', season_candle: '季蜡', heart: '爱心', prestige: '升华蜡烛' } as const;
const prefixes: Record<string, string> = { ...Object.fromEntries(Object.keys(SKY_OUTFIT_KINDS).map(k => [k, k])), wing: 'cape' };
const names: Record<string, string> = catalog.names;
const icons: Record<string, string> = catalog.icons;
const aliases: Record<string, string> = catalog.aliases;
const normalize = (name: string) => name.replace(/\.[^.]+$/, '').replace(/^(UiOutfit|CharSkyKid_)/i, '').replace(/[^a-z0-9]/gi, '').toLowerCase();

/** 供应商公开图鉴的命名映射，只用于显示；拥有与否来自返回的精确游戏物品。 */
export function describeSkyOutfit(outfit: SkyOwnedOutfit) {
  const suffix = outfit.name.match(/^CharSkyKid_[^_]+_(.+)$/)?.[1];
  const variants = [aliases[outfit.name], outfit.name, outfit.name.replace(/_\d{2}$/, ''),
    suffix && `${prefixes[outfit.kind] || outfit.kind}${suffix}`,
    suffix && `${prefixes[outfit.kind] || outfit.kind}${suffix.replace(/_\d{2}$/, '')}`].filter((s): s is string => !!s);
  const keys = variants.map(normalize);
  const label = keys.map(k => names[k]).find(Boolean);
  const icon = keys.map(k => icons[k]).find(Boolean);
  return {
    ...outfit, label: label || outfit.name.replace(/^CharSkyKid_/, '').replace(/_/g, ' '),
    image: icon ? `http://dp.mz997.com:92/exported_icons/${encodeURIComponent(icon)}` : '',
    kindLabel: SKY_OUTFIT_KINDS[outfit.kind] || '其他',
  };
}

export function loadSkyInventory(characterId: string): SkyInventory | undefined {
  return characterId ? progressGet<SkyInventory>('sky.inventory', 'owned', characterId) : undefined;
}

/** 缺失不代表清空；仅接受本次查询有明确值的余额，拥有记录按游戏内部名称合并。 */
export async function saveSkyInventory(characterId: string, assets: SkyBoostAssets, fetchedAt: string) {
  if (!assets.outfits && !Object.keys(assets.balances).length) return;
  const old = loadSkyInventory(characterId);
  const outfits = new Map((old?.outfits || []).map(item => [item.name, item]));
  for (const item of assets.outfits || []) outfits.set(item.name, item);
  const next: SkyInventory = {
    outfits: [...outfits.values()], balances: { ...old?.balances, ...assets.balances },
    outfitsAt: assets.outfits ? fetchedAt : old?.outfitsAt,
    balancesAt: Object.keys(assets.balances).length ? fetchedAt : old?.balancesAt,
  };
  await setProgressEntries('sky.inventory', [['owned', next]], characterId);
}

// 明确的游戏物品与 wiki 条目对应，不用种类、季节或模糊相似度猜测整组拥有状态。
const collectionLinks: Record<string, [string, string]> = {
  CharSkyKid_Body_AP21Coat: ['九色鹿季', '猎手 · 裤子'],
  CharSkyKid_Body_AP21LinenPants: ['九色鹿季', '采药人 · 裤子'],
  CharSkyKid_Body_AP21RoyalDress: ['九色鹿季', '公主 · 裤子'],
  CharSkyKid_Hair_AP21HunterBun: ['九色鹿季', '猎手 · 发型'],
  CharSkyKid_Horn_AP21Crown: ['九色鹿季', '领主 · 发饰'],
  CharSkyKid_Mask_AP21Eyebrows: ['九色鹿季', '领主 · 面具'],
  CharSkyKid_Mask_AP21PaintMakeup: ['九色鹿季', '公主 · 面具'],
  CharSkyKid_Wing_AP21LinenScarf: ['九色鹿季', '猎手 · 斗篷'],
  CharSkyKid_Prop_AP19Camera: ['拾光季', '拾光季向导 · 手持'],
  CharSkyKid_Body_MischiefMagicDress: ['恶作剧之日', '魔女裙'],
  CharSkyKid_Prop_MischiefWand: ['恶作剧之日', '魔法杖'],
  CharSkyKid_Mask_MischiefBeak: ['恶作剧之日', '鸦喙面具'],
  CharSkyKid_Wing_MischiefGoth: ['恶作剧之日', '哥特斗篷'],
  CharSkyKid_Hat_MischiefLeaf: ['恶作剧之日', '枯叶发饰'],
  CharSkyKid_Face_CurseSticker: ['恶作剧之日', '符咒贴纸头饰'],
  CharSkyKid_Body_Goth: ['恶作剧之日', '哥特螃蟹长袍'],
  CharSkyKid_Feet_GothBoots: ['恶作剧之日', '哥特长靴'],
  CharSkyKid_Hair_MischiefCat: ['恶作剧之日', '猫耳发型'],
};

export function skyOutfitCollectionMatch(outfit: SkyOwnedOutfit, guides: SeasonalGuide[]) {
  const explicit = collectionLinks[outfit.name];
  const label = describeSkyOutfit(outfit).label.split(' · ');
  const [season, name] = explicit || [label[1], label[0]];
  if (!season || !name) return undefined;
  const matches = guides.flatMap(guide => guide.season === season
    ? guide.itemList.filter(item => item.name === name).map(item => ({ guide, item })) : []);
  return matches.length === 1 ? matches[0] : undefined;
}

export async function syncSkyInventoryCollection(characterId: string, guides: SeasonalGuide[], scope: 'sky.season_wish' | 'sky.event_wish') {
  const owned = loadSkyInventory(characterId);
  if (!owned) return;
  const updates = new Map<string, { items: Record<string, boolean> }>();
  for (const outfit of owned.outfits) {
    const match = skyOutfitCollectionMatch(outfit, guides);
    if (!match) continue;
    const old = progressGet<{ items?: Record<string, boolean> }>(scope, match.guide.id, characterId) || {};
    if (old.items?.[match.item.id] !== undefined) continue;
    const record = updates.get(match.guide.id) || { ...old, items: { ...old.items } };
    record.items[match.item.id] = progressGet<boolean>('sky.outfit_status', outfit.name, characterId) ?? true;
    updates.set(match.guide.id, record);
  }
  if (updates.size) await setProgressEntries(scope, [...updates], characterId);
}
