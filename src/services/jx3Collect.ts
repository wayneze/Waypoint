import type {
  Jx3DungeonPendant,
  Jx3SetTokenSeries,
  Jx3CollectItem,
  Jx3CollectStore,
  Jx3CollectGroup,
  Jx3CollectKind,
  Jx3CollectPinnedDungeon,
  Jx3BoxDungeon,
  Jx3BoxDungeonMap,
  Jx3BoxItemHit,
  Jx3TokenSlot,
  Jx3DynamicSet,
  Jx3DynamicSetPiece
} from '../types';
import { progressEntries, progressGet, progressReplace, progressSet } from './progress';
import { JX3_PENDANTS_CATALOG, JX3_SET_TOKENS_CATALOG, cloneCatalog, compactQuery, loadJx3Pendants, loadJx3SetTokens, normalizeJx3DungeonLabel, scoreText } from './jx3CollectData';
import dungeonScenes from '../content/jx3/dungeon-scenes.json';
import { jx3ArtworkUrl } from './jx3Artwork';

// 收集条目逐行存 jx3.collect；置顶副本和折叠状态存 jx3.collect.layout 一行。
// 全局清单 characterId = ''，角色清单用角色 id。
const COLLECT_SCOPE = 'jx3.collect';
const COLLECT_LAYOUT_SCOPE = 'jx3.collect.layout';

export const JX3_TOKEN_SLOTS: Jx3TokenSlot[] = ['head', 'body', 'belt', 'wrist', 'shoes'];

export const JX3_COLLECT_KIND_LABEL: Record<Jx3CollectKind, string> = {
  pendant: '挂件',
  special: '特殊',
  appearance: '外观',
  token: '牌子'
};

function compactDungeon(raw: string): string {
  return normalizeJx3DungeonLabel(raw).replace(/[·・\-_/]/g, '').toLowerCase();
}

function dungeonChunks(raw: string): string[] {
  return normalizeJx3DungeonLabel(raw).split(/[·&及和/,，]/).map(s => s.trim()).filter(s => s.length >= 2);
}

function applyDungeonAlias(label: string): string {
  const raw = normalizeJx3DungeonLabel(label)
    .replace(/^(特殊活动|苍生铸世|丝路风语|横刀断浪|奉天证道|世外蓬莱|剑胆琴心|安史之乱|巴蜀风云|风起稻香)[·]/, '');
  if (/花月仙踪/.test(raw)) return '永王行宫·花月别院';
  if (/夜守孤城/.test(raw)) return '太原之战·夜守孤城';
  if (/逐虎过江/.test(raw)) return '太原之战·逐虎驱狼';
  if (/巨冥湾/.test(raw)) return '尘归海·巨冥湾';
  if (/冰火岛/.test(raw) && !/荒血路|青莲狱/.test(raw)) return '冰火岛·荒血路';
  return raw;
}

export function parseJx3BoxDungeonTree(raw: unknown): Jx3BoxDungeon[] {
  if (!raw || typeof raw !== 'object') return [];
  const out: Jx3BoxDungeon[] = [];
  for (const [eraKey, pack] of Object.entries(raw as Record<string, unknown>)) {
    if (!pack || typeof pack !== 'object') continue;
    const rec = pack as Record<string, unknown>;
    const era = String(rec.name || eraKey);
    const eraLevel = String(rec.level ?? '');
    const dens = rec.dungeon;
    if (!dens || typeof dens !== 'object') continue;
    for (const [dKey, d] of Object.entries(dens as Record<string, unknown>)) {
      if (!d || typeof d !== 'object') continue;
      const row = d as Record<string, unknown>;
      const name = String(row.name || dKey);
      const maps: Jx3BoxDungeonMap[] = Array.isArray(row.maps)
        ? row.maps.map((m: any) => ({
          mapId: String(m?.map_id ?? m?.mapId ?? ''),
          mode: String(m?.mode ?? '')
        })).filter(m => m.mapId || m.mode)
        : [];
      const bosses = Array.isArray(row.boss)
        ? row.boss.map((b: unknown) => String(b || '').trim()).filter(Boolean)
        : [];
      const icon = String(row.icon || '').trim();
      out.push({ key: name, name, era, eraLevel, icon: icon || undefined, maps, bosses });
    }
  }
  return out;
}

function boxFieldText(value: unknown): string | undefined {
  if (value == null) return undefined;
  if (typeof value === 'string') {
    const t = value.trim();
    return t || undefined;
  }
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  if (Array.isArray(value)) {
    const parts = value.map(boxFieldText).filter((s): s is string => !!s);
    return parts.length ? [...new Set(parts)].join(' · ') : undefined;
  }
  if (typeof value === 'object') {
    const rec = value as Record<string, unknown>;
    return boxFieldText(rec.label ?? rec.name ?? rec.Name ?? rec.text ?? rec.title);
  }
  return undefined;
}

function stripBoxMarkup(text?: unknown): string {
  if (!text) return '';
  return String(text).replace(/<[^>]+>/g, '').replace(/text="([^"]+)"/g, '$1').trim();
}

export function parseJx3BoxItemSearch(raw: unknown, dungeons: Jx3BoxDungeon[] = [], keyword = ''): Jx3BoxItemHit[] {
  const root = raw as { code?: number; data?: { data?: unknown[] } } | null;
  if (!root || root.code !== 200 || !Array.isArray(root.data?.data)) return [];
  const hits: Jx3BoxItemHit[] = [];
  const qClean = keyword.trim().toLowerCase();
  for (const row of root.data.data) {
    if (!row || typeof row !== 'object') continue;
    const it = row as Record<string, unknown>;
    const id = String(it.id || '').trim();
    const name = String(it.Name || it.name || '').trim();
    if (!id || !name) continue;
    const iconRaw = it.IconID ?? it.iconId;
    const iconId = typeof iconRaw === 'number' ? iconRaw : Number(iconRaw);
    const hit: Jx3BoxItemHit = {
      id,
      name,
      iconId: Number.isFinite(iconId) && iconId > 0 ? iconId : undefined,
      typeLabel: boxFieldText(it.TypeLabel),
      getType: boxFieldText(it.GetType),
      getSource: boxFieldText(it.GetSource),
      quality: typeof it.Quality === 'number' ? it.Quality : undefined,
      source: boxFieldText(it.Source),
      desc: stripBoxMarkup(it.Desc) || undefined,
      belongSchool: boxFieldText(it.BelongSchool),
      requireLevel: typeof it.RequireLevel === 'number' ? it.RequireLevel : undefined
    };
    hit.inferredDungeon = inferHitDungeon(hit, dungeons);
    hits.push(hit);
  }
  if (qClean) {
    hits.sort((a, b) => {
      const aName = a.name.toLowerCase();
      const bName = b.name.toLowerCase();
      const aExact = aName.startsWith(qClean) ? 2 : (aName.includes(qClean) ? 1 : 0);
      const bExact = bName.startsWith(qClean) ? 2 : (bName.includes(qClean) ? 1 : 0);
      if (aExact !== bExact) return bExact - aExact;
      return 0;
    });
  }
  return hits.slice(0, 30);
}

export function jx3BoxItemIconUrl(iconId?: number): string {
  return iconId && iconId > 0 ? jx3ArtworkUrl(`https://icon.jx3box.com/icon/${iconId}.png`) : '';
}

export function jx3BoxDungeonThumbUrl(icon?: string): string {
  const path = String(icon || '').trim().replace(/^\/+/, '');
  if (!path) return '';
  const bundled = (dungeonScenes as Record<string, string>)[path];
  if (bundled) return bundled;
  if (/^https?:\/\//i.test(path)) return path;
  return `https://img.jx3box.com/${path}`;
}

/** 仅收录魔盒精确命中的游戏内物品 IconID。不对名字做批量搜索，也不用周边/时装/坐骑误伤。 */
const JX3_SEED_ITEM_ICON_IDS: Record<string, number> = {
  p_yehua: 1699,
  夜话白鹭: 1699,
  '夜话·白鹭': 1699,
  p_yemuxinghe: 7018,
  夜幕星河: 7018,
  p_zhanmaxingtian: 8626,
  斩马刑天: 8626,
  p_heitian: 2831,
  黑天: 2831
};

export function resolveJx3SeedItemIconId(key?: string): number | undefined {
  const k = String(key || '').trim();
  if (!k) return undefined;
  const id = JX3_SEED_ITEM_ICON_IDS[k];
  return id && id > 0 ? id : undefined;
}

export function resolveJx3CollectItemIconId(item?: {
  iconId?: number;
  catalogId?: string;
  id?: string;
  name?: string;
} | null): number | undefined {
  if (!item) return undefined;
  const seed = resolveJx3SeedItemIconId(item.catalogId)
    || resolveJx3SeedItemIconId(item.id)
    || resolveJx3SeedItemIconId(item.name);
  if (seed) return seed;
  if (item.iconId && item.iconId > 0) return item.iconId;
  return undefined;
}

export function jx3CollectItemIconUrl(item?: {
  iconId?: number;
  catalogId?: string;
  id?: string;
  name?: string;
} | null): string {
  return jx3BoxItemIconUrl(resolveJx3CollectItemIconId(item));
}

export function guessJx3CollectKind(hit: Jx3BoxItemHit): Jx3CollectKind {
  const blob = `${hit.typeLabel || ''}${hit.getType || ''}${hit.getSource || ''}${hit.source || ''}${hit.name}`;
  if (/挂件|背部|腰部/.test(blob)) return 'pendant';
  if (/外观|帽子|衣服|披风|发型|脸型|武器箱/.test(blob)) return 'appearance';
  if (/牌子|套装/.test(blob)) return 'token';
  return 'special';
}

export function scoreJx3DungeonMatch(query: string, dungeon: Jx3BoxDungeon): number {
  const q = query.trim();
  if (!q) return 0;
  return scoreText(q.toLowerCase(), compactQuery(q), [dungeon.name, dungeon.era, dungeon.bosses.join(' ')]);
}

export function searchJx3BoxDungeons(query: string, dungeons: Jx3BoxDungeon[]): Jx3BoxDungeon[] {
  const q = query.trim();
  if (!q) return [];
  return dungeons
    .map(d => ({ d, score: scoreJx3DungeonMatch(q, d) }))
    .filter(x => x.score > 0)
    .sort((a, b) => b.score - a.score || a.d.name.localeCompare(b.d.name, 'zh'))
    .slice(0, 12)
    .map(x => x.d);
}

export function resolveJx3BoxDungeon(label: string, dungeons: Jx3BoxDungeon[]): Jx3BoxDungeon | null {
  const parts = String(label || '').split(/[&及]/).map(s => s.trim()).filter(Boolean);
  const target = applyDungeonAlias(parts[0] || label);
  const compact = compactDungeon(target);
  if (!compact) return null;
  let best: Jx3BoxDungeon | null = null;
  let bestScore = 0;
  for (const d of dungeons) {
    const dc = compactDungeon(d.name);
    let score = 0;
    if (dc === compact || d.name === target || d.key === target) score = 100;
    else if (dc.includes(compact) || compact.includes(dc)) score = 80;
    else {
      const chunks = dungeonChunks(target);
      const hit = chunks.filter(c => d.name.includes(c) || compactDungeon(d.name).includes(compactDungeon(c)));
      if (hit.length) score = 40 + hit.length * 10 + Math.min(dc.length, compact.length);
    }
    if (score > bestScore) {
      bestScore = score;
      best = d;
    }
  }
  return bestScore >= 80 ? best : null;
}

function tokenPieceDungeonLabel(series: Jx3SetTokenSeries, slot: Jx3TokenSlot): string {
  const boss = series.pieces[slot].boss || '';
  const prefix = boss.split(/[·・]/)[0]?.trim() || '';
  const parts = series.dungeon.split(/[&及]/).map(s => s.trim()).filter(Boolean);
  if (prefix) {
    const hit = parts.find(p => p.includes(prefix) || prefix.includes(normalizeJx3DungeonLabel(p).split('·').pop() || ''));
    if (hit) return hit;
    for (const p of parts) {
      if (p.includes(prefix) || prefix.includes(p)) return p;
    }
    if (/仙侣/.test(prefix)) return parts.find(p => p.includes('仙侣')) || '永王行宫·仙侣庭园';
    if (/花月/.test(prefix)) return parts.find(p => /花月/.test(p)) || '永王行宫·花月别院';
    if (/夜守/.test(prefix)) return parts.find(p => /夜守/.test(p)) || series.dungeon;
    if (/逐虎/.test(prefix)) return parts.find(p => /逐虎/.test(p)) || series.dungeon;
    if (/稻香/.test(prefix)) return parts.find(p => /稻香/.test(p)) || series.dungeon;
    if (/秦皇/.test(prefix)) return parts.find(p => /秦皇/.test(p)) || series.dungeon;
    if (/锻刀/.test(prefix)) return parts.find(p => /锻刀/.test(p)) || series.dungeon;
    if (/战狂/.test(prefix)) return parts.find(p => /战狂/.test(p)) || series.dungeon;
    if (/千雷/.test(prefix)) return parts.find(p => /千雷/.test(p)) || series.dungeon;
    if (/荻花/.test(prefix)) return parts.find(p => p.includes('荻花')) || series.dungeon;
    if (/龙渊/.test(prefix)) return parts.find(p => p.includes('龙渊')) || series.dungeon;
    if (/烛龙/.test(prefix)) return parts.find(p => p.includes('烛龙')) || series.dungeon;
    if (/皇宫/.test(prefix)) return parts.find(p => p.includes('皇宫')) || series.dungeon;
    if (/大明宫/.test(prefix)) return parts.find(p => p.includes('大明宫')) || series.dungeon;
    if (/战兽/.test(prefix)) return parts.find(p => p.includes('战兽')) || series.dungeon;
    if (/西津渡/.test(prefix)) return parts.find(p => p.includes('西津渡')) || series.dungeon;
    if (/战宝/.test(prefix)) return parts.find(p => p.includes('战宝')) || series.dungeon;
    if (/黑牢/.test(prefix)) return parts.find(p => p.includes('黑牢')) || series.dungeon;
    if (/九老/.test(prefix)) return parts.find(p => p.includes('九老')) || series.dungeon;
    if (/冷龙/.test(prefix)) return parts.find(p => p.includes('冷龙')) || series.dungeon;
    if (/白帝/.test(prefix)) return parts.find(p => p.includes('白帝')) || series.dungeon;
    if (/一之窟/.test(prefix)) return parts.find(p => p.includes('一之窟')) || series.dungeon;
    if (/巨冥/.test(prefix)) return parts.find(p => p.includes('巨冥')) || series.dungeon;
    if (/冰火/.test(prefix)) return parts.find(p => p.includes('冰火')) || series.dungeon;
    if (/荒血/.test(prefix)) return parts.find(p => p.includes('荒血')) || '冰火岛·荒血路';
    if (/青莲/.test(prefix)) return parts.find(p => p.includes('青莲')) || '冰火岛·青莲狱';
    if (/达摩/.test(prefix)) return parts.find(p => p.includes('达摩')) || series.dungeon;
    if (/河阳/.test(prefix)) return parts.find(p => p.includes('河阳')) || '河阳之战';
    if (/范阳/.test(prefix)) return parts.find(p => p.includes('范阳')) || '范阳夜变';
  }
  return parts[0] || series.dungeon;
}

export function matchSlotKeyword(name: string): Jx3TokenSlot | undefined {
  if (/[冠帽头发盔]/.test(name)) return 'head';
  if (/[衣袍甲衫铠]/.test(name)) return 'body';
  if (/[带腰]/.test(name)) return 'belt';
  if (/[腕手袖]/.test(name)) return 'wrist';
  if (/[鞋履靴足]/.test(name)) return 'shoes';
  return undefined;
}

export const JX3_SLOT_LABELS: Record<Jx3TokenSlot, string> = {
  head: '头/帽',
  body: '衣/袍',
  belt: '腰带',
  wrist: '护腕',
  shoes: '鞋履'
};

export function inferHitDungeon(
  hit: Pick<Jx3BoxItemHit, 'name' | 'getType' | 'getSource' | 'desc' | 'typeLabel' | 'source' | 'requireLevel'>,
  dungeons: Jx3BoxDungeon[] = []
): { dungeonKey: string; dungeonName: string; era?: string; boss?: string } | undefined {
  // 1. 本地图鉴高精度匹配（夜话白鹭/黑天等，或者已知校服），作为快速增强
  for (const p of JX3_PENDANTS_CATALOG) {
    if (hit.name.includes(p.name) || p.name.includes(hit.name)) {
      const matched = resolveJx3BoxDungeon(p.dungeon, dungeons);
      return {
        dungeonKey: matched?.key || p.dungeon,
        dungeonName: matched?.name || p.dungeon,
        era: matched?.era,
        boss: p.boss
      };
    }
  }

  // 门派校服成衣通用命名规则：[套装名]·[门派前缀][部位]，如 文王·逸然衣、破虏·潜行衣、朔漠·归客履
  const dotPrefixMatch = /^([^·・\s]{2,6})[·・]/.exec(hit.name);
  const dotPrefix = dotPrefixMatch ? dotPrefixMatch[1].trim() : '';

  for (const s of JX3_SET_TOKENS_CATALOG) {
    const seriesRoot = s.seriesName.replace(/套$/, '').trim();
    const aliasRoots = (s.aliases || []).map(a => a.replace(/套$/, '').trim()).filter(a => a.length >= 2);
    const hitMatchesThisSeries = (dotPrefix && (dotPrefix === seriesRoot || aliasRoots.includes(dotPrefix)))
      || hit.name.includes(s.seriesName)
      || hit.name.includes(seriesRoot)
      || aliasRoots.some(a => hit.name.includes(a));
    if (hitMatchesThisSeries) {
      const slot = matchSlotKeyword(hit.name);
      if (slot) {
        const piece = s.pieces[slot];
        const dungeonLabel = tokenPieceDungeonLabel(s, slot);
        const matched = resolveJx3BoxDungeon(dungeonLabel, dungeons);
        return {
          dungeonKey: matched?.key || dungeonLabel,
          dungeonName: matched?.name || dungeonLabel,
          era: matched?.era,
          boss: piece.boss
        };
      }
      for (const sl of JX3_TOKEN_SLOTS) {
        const p = s.pieces[sl];
        if (hit.name.includes(p.name) || p.name.includes(hit.name)) {
          const dungeonLabel = tokenPieceDungeonLabel(s, sl);
          const matched = resolveJx3BoxDungeon(dungeonLabel, dungeons);
          return {
            dungeonKey: matched?.key || dungeonLabel,
            dungeonName: matched?.name || dungeonLabel,
            era: matched?.era,
            boss: p.boss
          };
        }
      }
      const parts = s.dungeon.split(/[&及]/).map(x => x.trim()).filter(Boolean);
      const mainLabel = parts[0] || s.dungeon;
      const matched = resolveJx3BoxDungeon(mainLabel, dungeons);
      return {
        dungeonKey: matched?.key || mainLabel,
        dungeonName: matched?.name || mainLabel,
        era: matched?.era
      };
    }
  }

  // 2. 通用魔盒全量秘境与首领倒排索引动态推断（覆盖魔盒全部数十万件物品，无硬编码）
  if (dungeons.length) {
    // 2.1 获取来源文本规范化（仅限真实来源字段，排除背景风物古诗文案，严禁诗句词汇误匹配首领）
    const sourceBlob = `${hit.getType || ''} ${hit.getSource || ''}`.toLowerCase();
    // 只有在 desc 明确属于掉落出处语句（包含“掉落”、“首领”、“击败”）时，才允许提取出处
    const descDropMatch = hit.desc && /掉落|击败|产出|首领/.test(hit.desc) ? hit.desc.toLowerCase() : '';
    const dropBlob = `${sourceBlob} ${descDropMatch}`.trim();

    // 2.2 首领名匹配（必须在真实来源文本 dropBlob 中匹配，绝不能在风物诗句中误匹配）
    if (dropBlob) {
      const STOP_BOSSES = new Set(['其它', '小怪', '神秘人', '无', '首领', '牡丹', '白鹭', '雷神', '赤幽']);
      for (const d of dungeons) {
        for (const b of d.bosses) {
          const bossClean = b.trim();
          if (bossClean.length < 2) continue;
          if (STOP_BOSSES.has(bossClean) && !/首领|击败/.test(dropBlob)) continue;
          if (dropBlob.includes(bossClean.toLowerCase())) {
            return {
              dungeonKey: d.key,
              dungeonName: d.name,
              era: d.era,
              boss: bossClean
            };
          }
        }
      }
    }

    // 2.3 秘境名称匹配（在来源文本中匹配，或在装备名称包含明确秘境前缀时匹配）
    const nameMatchesDungeon = dungeons.some(d => hit.name.includes(compactDungeon(d.name)));
    const targetDungeonBlob = `${dropBlob} ${nameMatchesDungeon ? hit.name.toLowerCase() : ''}`.trim();
    if (targetDungeonBlob) {
      for (const d of dungeons) {
        const compactName = compactDungeon(d.name).toLowerCase();
        if (compactName.length >= 2 && targetDungeonBlob.includes(compactName)) {
          return {
            dungeonKey: d.key,
            dungeonName: d.name,
            era: d.era
          };
        }
        const parts = d.name.split(/[·・\s]/).map(s => s.trim()).filter(s => s.length >= 2);
        for (const part of parts) {
          if (targetDungeonBlob.includes(part.toLowerCase())) {
            return {
              dungeonKey: d.key,
              dungeonName: d.name,
              era: d.era
            };
          }
        }
      }
    }
  }

  return undefined;
}

/**
 * 解析用户输入的搜索词，如果是已知套装的别名或正名，返回用于魔盒检索的所有关联词（正名根词 + 别名根词 + 原词去重）
 */
export function resolveJx3SearchTokens(keyword: string): string[] {
  const q = keyword.trim();
  if (!q) return [];
  const qLower = q.toLowerCase();
  const qRoot = qLower.replace(/套$/, '').trim();
  const terms = new Set<string>([q]);

  for (const series of JX3_SET_TOKENS_CATALOG) {
    const sName = series.seriesName.toLowerCase();
    const sRoot = sName.replace(/套$/, '').trim();
    const aliases = (series.aliases || []).map(a => a.toLowerCase());
    const aliasRoots = aliases.map(a => a.replace(/套$/, '').trim()).filter(a => a.length >= 2);

    const isMatch = (
      qLower.length >= 2 && (
        sName.includes(qLower) ||
        sRoot.includes(qRoot) ||
        aliases.some(a => a.includes(qLower) || qLower.includes(a)) ||
        aliasRoots.some(a => a.includes(qRoot) || qRoot.includes(a)) ||
        (series.pinyin && series.pinyin.split(/[\s/]+/).some(p => p.toLowerCase() === qLower || (qLower.length >= 2 && p.toLowerCase().includes(qLower))))
      )
    );

    if (isMatch) {
      if (sRoot) terms.add(sRoot);
      for (const a of aliasRoots) {
        if (a) terms.add(a);
      }
    }
  }

  return Array.from(terms);
}

export function detectDynamicSets(
  hits: Jx3BoxItemHit[],
  dungeons: Jx3BoxDungeon[] = [],
  collectItems: Jx3CollectItem[] = [],
  query = ''
): Jx3DynamicSet[] {
  const sets: Jx3DynamicSet[] = [];
  const processedSeries = new Set<string>();

  // 1. 优先检查已知校服目录（标准 5 部位与精确分流）
  const queryBlob = hits.map(h => h.name).join(' ').toLowerCase();
  const qClean = query.trim().toLowerCase();
  const qRoot = qClean.replace(/套$/, '').trim();
  for (const series of JX3_SET_TOKENS_CATALOG) {
    const seriesRoot = series.seriesName.replace(/套$/, '').trim();
    const aliases = (series.aliases || []).map(a => a.toLowerCase());
    const aliasRoots = aliases.map(a => a.replace(/套$/, '').trim()).filter(a => a.length >= 2);

    const queryMatched = qClean.length >= 2 && (
      series.seriesName.toLowerCase().includes(qClean) ||
      seriesRoot.toLowerCase().includes(qRoot) ||
      aliases.some(a => a.includes(qClean) || qClean.includes(a)) ||
      aliasRoots.some(a => a.includes(qRoot) || qRoot.includes(a)) ||
      (series.pinyin && series.pinyin.split(/[\s/]+/).some(p => p.toLowerCase() === qClean || (qClean.length >= 2 && p.toLowerCase().includes(qClean))))
    );
    const hitMatchesSeries = (h: Jx3BoxItemHit) => {
      const n = h.name.toLowerCase();
      const dotPrefixMatch = /^([^·・\s]{2,6})[·・]/.exec(n);
      const dotPrefix = dotPrefixMatch ? dotPrefixMatch[1].trim() : '';
      return (dotPrefix && (dotPrefix === seriesRoot.toLowerCase() || aliasRoots.includes(dotPrefix))) ||
        n.includes(series.seriesName.toLowerCase()) ||
        n.includes(seriesRoot.toLowerCase()) ||
        aliasRoots.some(a => n.includes(a));
    };
    const matched = queryMatched
      || hits.some(hitMatchesSeries)
      || (series.pinyin && series.pinyin.split(/[\s/]+/).some(p => p.length >= 2 && queryBlob.includes(p.toLowerCase())));
    if (matched && !processedSeries.has(series.seriesName) && !processedSeries.has(seriesRoot)) {
      processedSeries.add(series.seriesName);
      processedSeries.add(seriesRoot);
      for (const a of aliases) {
        processedSeries.add(a);
        processedSeries.add(a.replace(/套$/, ''));
      }
      const piecesRecord: Record<Jx3TokenSlot, Jx3DynamicSetPiece> = {} as any;
      for (const slot of JX3_TOKEN_SLOTS) {
        const p = series.pieces[slot];
        const dungeonLabel = tokenPieceDungeonLabel(series, slot);
        const resolved = resolveJx3BoxDungeon(dungeonLabel, dungeons);
        const existing = collectItems.find(it => it.kind === 'token' && (it.tokenSeriesId === series.id || it.tokenSeriesId === series.seriesName) && it.tokenSlot === slot);
        const relatedHit = hits.find(h => hitMatchesSeries(h) && matchSlotKeyword(h.name) === slot);
        piecesRecord[slot] = {
          slot,
          slotLabel: JX3_SLOT_LABELS[slot],
          name: relatedHit?.name || p.name,
          hit: relatedHit,
          dungeonKey: resolved?.key || dungeonLabel,
          dungeonName: resolved?.name || dungeonLabel,
          era: resolved?.era,
          boss: p.boss,
          obtained: existing ? existing.obtained : false,
          inCollect: !!existing
        };
      }
      sets.push({
        seriesName: series.seriesName,
        level: series.level,
        pieces: piecesRecord
      });
    }
  }

  // 2. 通用动态套装聚合：对魔盒返回的 hits 按名称前缀模式聚类
  const prefixMap = new Map<string, Array<{ slot: Jx3TokenSlot; hit: Jx3BoxItemHit }>>();
  for (const hit of hits) {
    const slot = matchSlotKeyword(hit.name);
    if (!slot) continue;
    const dotMatch = /^([^·・\s]{2,6})[·・]/.exec(hit.name);
    let prefix = dotMatch ? dotMatch[1].trim() : '';
    if (!prefix && hit.name.length >= 3) {
      prefix = hit.name.slice(0, 2);
    }
    if (!prefix || processedSeries.has(prefix) || processedSeries.has(`${prefix}套`)) continue;
    if (!prefixMap.has(prefix)) prefixMap.set(prefix, []);
    prefixMap.get(prefix)!.push({ slot, hit });
  }

  for (const [prefix, list] of prefixMap.entries()) {
    if (processedSeries.has(prefix) || processedSeries.has(`${prefix}套`)) continue;
    const uniqueSlots = new Set(list.map(x => x.slot));
    if (uniqueSlots.size >= 2) {
      processedSeries.add(prefix);
      processedSeries.add(`${prefix}套`);
      const piecesRecord: Record<Jx3TokenSlot, Jx3DynamicSetPiece> = {} as any;
      const setInferredFallback = list.map(x => x.hit.inferredDungeon).find(inf => inf?.dungeonName);
      for (const slot of JX3_TOKEN_SLOTS) {
        const found = list.find(x => x.slot === slot);
        const hit = found?.hit;
        const inf = hit?.inferredDungeon || setInferredFallback;
        const existing = collectItems.find(it => it.kind === 'token' && it.name.includes(prefix) && it.tokenSlot === slot);
        piecesRecord[slot] = {
          slot,
          slotLabel: JX3_SLOT_LABELS[slot],
          name: hit?.name || `${prefix}·${JX3_SLOT_LABELS[slot]}`,
          hit,
          dungeonKey: inf?.dungeonKey,
          dungeonName: inf?.dungeonName,
          era: inf?.era,
          boss: inf?.boss,
          obtained: existing ? existing.obtained : false,
          inCollect: !!existing
        };
      }
      sets.push({
        seriesName: `${prefix}套`,
        level: list[0]?.hit?.requireLevel ? `${list[0].hit.requireLevel}年代` : undefined,
        pieces: piecesRecord
      });
    }
  }

  return sets;
}

function collectItemDungeon(label: string, boxList: Jx3BoxDungeon[]): { dungeonKey: string; dungeonName: string; era?: string } {
  const matched = resolveJx3BoxDungeon(label, boxList);
  if (matched) return { dungeonKey: matched.key, dungeonName: matched.name, era: matched.era };
  const name = normalizeJx3DungeonLabel(label) || label.trim() || '未归组';
  return { dungeonKey: name, dungeonName: name };
}

function migrateCollectFromLegacy(boxList: Jx3BoxDungeon[] = []): Jx3CollectStore {
  const pendants = loadJx3Pendants();
  const tokens = loadJx3SetTokens();
  const items: Jx3CollectItem[] = [];
  const pinMap = new Map<string, Jx3CollectPinnedDungeon>();

  const pin = (dungeonKey: string, dungeonName: string, era?: string) => {
    if (!pinMap.has(dungeonKey)) pinMap.set(dungeonKey, { dungeonKey, dungeonName, era });
  };

  for (const p of pendants) {
    const loc = collectItemDungeon(p.dungeon, boxList);
    pin(loc.dungeonKey, loc.dungeonName, loc.era);
    items.push({
      id: p.id,
      dungeonKey: loc.dungeonKey,
      dungeonName: loc.dungeonName,
      era: loc.era,
      kind: 'pendant',
      name: p.name,
      obtained: !!p.obtained,
      source: 'seed',
      catalogId: p.id,
      note: p.desc,
      boss: p.boss,
      difficulty: p.difficulty,
      type: p.type,
      cdRule: p.cdRule,
      cdCleared: !!p.cdCleared,
      reputationId: p.reputationId,
      relatedAchieveIds: [...(p.relatedAchieveIds || [])],
      iconId: resolveJx3SeedItemIconId(p.id) || resolveJx3SeedItemIconId(p.name)
    });
  }

  for (const series of tokens) {
    for (const slot of JX3_TOKEN_SLOTS) {
      const piece = series.pieces[slot];
      const loc = collectItemDungeon(tokenPieceDungeonLabel(series, slot), boxList);
      pin(loc.dungeonKey, loc.dungeonName, loc.era);
      items.push({
        id: `${series.id}:${slot}`,
        dungeonKey: loc.dungeonKey,
        dungeonName: loc.dungeonName,
        era: loc.era,
        kind: 'token',
        name: piece.name,
        obtained: !!piece.obtained,
        source: 'seed',
        catalogId: series.id,
        tokenSlot: slot,
        tokenSeriesId: series.id,
        boss: piece.boss
      });
    }
  }

  return {
    items,
    pinnedDungeons: Array.from(pinMap.values()),
    collapsed: {}
  };
}

function sanitizeCollectStore(raw: unknown): Jx3CollectStore | null {
  if (!raw || typeof raw !== 'object') return null;
  const rec = raw as Record<string, unknown>;
  if (!Array.isArray(rec.items)) return null;
  const kinds: Jx3CollectKind[] = ['pendant', 'special', 'appearance', 'token'];
  const items: Jx3CollectItem[] = [];
  for (const row of rec.items) {
    if (!row || typeof row !== 'object') continue;
    const it = row as Record<string, unknown>;
    const id = String(it.id || '').trim();
    const dungeonKey = String(it.dungeonKey || '').trim();
    const dungeonName = String(it.dungeonName || dungeonKey).trim();
    const name = String(it.name || '').trim();
    const kind = kinds.includes(it.kind as Jx3CollectKind) ? it.kind as Jx3CollectKind : null;
    if (!id || !dungeonKey || !name || !kind) continue;
    items.push({
      id,
      dungeonKey,
      dungeonName,
      era: it.era ? String(it.era) : undefined,
      kind,
      name,
      obtained: !!it.obtained,
      source: it.source === 'box' || it.source === 'custom' || it.source === 'seed' ? it.source : 'custom',
      catalogId: it.catalogId ? String(it.catalogId) : undefined,
      tokenSlot: JX3_TOKEN_SLOTS.includes(it.tokenSlot as Jx3TokenSlot) ? it.tokenSlot as Jx3TokenSlot : undefined,
      tokenSeriesId: it.tokenSeriesId ? String(it.tokenSeriesId) : undefined,
      boxItemId: it.boxItemId ? String(it.boxItemId) : undefined,
      iconId: resolveJx3CollectItemIconId({
        iconId: typeof it.iconId === 'number' ? it.iconId : undefined,
        catalogId: it.catalogId ? String(it.catalogId) : undefined,
        id,
        name
      }),
      note: it.note ? String(it.note) : undefined,
      boss: it.boss ? String(it.boss) : undefined,
      difficulty: it.difficulty ? String(it.difficulty) : undefined,
      type: it.type ? String(it.type) : undefined,
      cdRule: it.cdRule ? String(it.cdRule) : undefined,
      cdCleared: it.cdCleared == null ? undefined : !!it.cdCleared,
      cdClearedAt: typeof it.cdClearedAt === 'string' && it.cdClearedAt ? it.cdClearedAt : undefined,
      reputationId: it.reputationId ? String(it.reputationId) : undefined,
      relatedAchieveIds: Array.isArray(it.relatedAchieveIds) ? it.relatedAchieveIds.map(x => String(x)) : undefined
    });
  }
  const pinnedDungeons: Jx3CollectPinnedDungeon[] = Array.isArray(rec.pinnedDungeons)
    ? rec.pinnedDungeons.map((p: any) => ({
      dungeonKey: String(p?.dungeonKey || '').trim(),
      dungeonName: String(p?.dungeonName || p?.dungeonKey || '').trim(),
      era: p?.era ? String(p.era) : undefined,
      eraLevel: p?.eraLevel ? String(p.eraLevel) : undefined
    })).filter((p: Jx3CollectPinnedDungeon) => p.dungeonKey)
    : [];
  const collapsed: Record<string, boolean> = {};
  if (rec.collapsed && typeof rec.collapsed === 'object') {
    for (const [k, v] of Object.entries(rec.collapsed as Record<string, unknown>)) {
      if (typeof v === 'boolean') collapsed[k] = v;
    }
  }
  return { items, pinnedDungeons, collapsed };
}

type CollectLayout = { pinnedDungeons?: unknown; collapsed?: unknown };

function readCollectStore(characterId: string): Jx3CollectStore | null {
  const layout = progressGet<CollectLayout>(COLLECT_LAYOUT_SCOPE, 'layout', characterId);
  const entries = progressEntries(COLLECT_SCOPE, characterId);
  if (!layout && !entries.length) return null;
  return sanitizeCollectStore({
    items: entries.map(([, item]) => item),
    pinnedDungeons: layout?.pinnedDungeons,
    collapsed: layout?.collapsed
  });
}

function writeCollectStore(characterId: string, store: Jx3CollectStore) {
  progressReplace(COLLECT_SCOPE, store.items.map(it => [it.id, it] as [string, unknown]), characterId);
  progressSet(COLLECT_LAYOUT_SCOPE, 'layout', {
    pinnedDungeons: store.pinnedDungeons,
    collapsed: store.collapsed
  }, characterId);
}

function emptyCollectStore(): Jx3CollectStore {
  return { items: [], pinnedDungeons: [], collapsed: {} };
}

export function loadJx3CharCollect(characterId: string, boxList: Jx3BoxDungeon[] = []): Jx3CollectStore {
  if (!characterId) return loadJx3Collect(boxList);
  const stored = readCollectStore(characterId);
  return stored ? relinkJx3CollectDungeons(stored, boxList) : emptyCollectStore();
}

export function saveJx3CharCollect(characterId: string, store: Jx3CollectStore) {
  if (!characterId) return;
  writeCollectStore(characterId, store);
}

export function loadJx3Collect(boxList: Jx3BoxDungeon[] = []): Jx3CollectStore {
  const stored = readCollectStore('');
  if (stored) return relinkJx3CollectDungeons(stored, boxList);
  return migrateCollectFromLegacy(boxList);
}

export function saveJx3Collect(store: Jx3CollectStore) {
  writeCollectStore('', store);
}

export function relinkJx3CollectDungeons(store: Jx3CollectStore, boxList: Jx3BoxDungeon[]): Jx3CollectStore {
  if (!boxList.length) return store;
  const boxKeys = new Set(boxList.map(d => d.key));
  const items = store.items.map(it => {
    let next = it;
    if (boxKeys.has(it.dungeonKey)) {
      const box = boxList.find(d => d.key === it.dungeonKey);
      if (box && (it.dungeonName !== box.name || it.era !== box.era)) {
        next = { ...it, dungeonName: box.name, era: box.era };
      }
    } else {
      const matched = resolveJx3BoxDungeon(it.dungeonName || it.dungeonKey, boxList);
      if (matched) {
        next = { ...it, dungeonKey: matched.key, dungeonName: matched.name, era: matched.era };
      }
    }
    const iconId = resolveJx3CollectItemIconId(next);
    if (iconId && next.iconId !== iconId) {
      next = { ...next, iconId };
    }
    return next;
  });
  const pinnedDungeons = store.pinnedDungeons.map(p => {
    if (boxKeys.has(p.dungeonKey)) {
      const box = boxList.find(d => d.key === p.dungeonKey);
      return box ? { dungeonKey: box.key, dungeonName: box.name, era: box.era, eraLevel: box.eraLevel } : p;
    }
    const matched = resolveJx3BoxDungeon(p.dungeonName || p.dungeonKey, boxList);
    if (!matched) return p;
    return { dungeonKey: matched.key, dungeonName: matched.name, era: matched.era, eraLevel: matched.eraLevel };
  });
  const seen = new Set<string>();
  const pins: Jx3CollectPinnedDungeon[] = [];
  for (const p of pinnedDungeons) {
    if (seen.has(p.dungeonKey)) continue;
    seen.add(p.dungeonKey);
    pins.push(p);
  }
  return { ...store, items, pinnedDungeons: pins };
}

export function collectItemsToPendants(items: Jx3CollectItem[]): Jx3DungeonPendant[] {
  const byId = new Map(JX3_PENDANTS_CATALOG.map(p => [p.id, p]));
  const out: Jx3DungeonPendant[] = [];
  const seen = new Set<string>();
  for (const it of items) {
    if (it.kind !== 'pendant') continue;
    const catalogId = it.catalogId || (byId.has(it.id) ? it.id : '');
    const seed = catalogId ? byId.get(catalogId) : undefined;
    const id = catalogId || it.id;
    if (seen.has(id)) continue;
    seen.add(id);
    out.push({
      id,
      name: it.name || seed?.name || id,
      type: (it.type as Jx3DungeonPendant['type']) || seed?.type || '背部挂件',
      dungeon: it.dungeonName || seed?.dungeon || '',
      difficulty: (it.difficulty as Jx3DungeonPendant['difficulty']) || seed?.difficulty || '25人英雄',
      boss: it.boss || seed?.boss || '',
      cdRule: (it.cdRule as Jx3DungeonPendant['cdRule']) || seed?.cdRule || '无CD',
      reputationId: it.reputationId || seed?.reputationId,
      relatedAchieveIds: it.relatedAchieveIds || seed?.relatedAchieveIds || [],
      obtained: !!it.obtained,
      cdCleared: !!it.cdCleared,
      icon: seed?.icon,
      desc: it.note || seed?.desc
    });
  }
  for (const seed of JX3_PENDANTS_CATALOG) {
    if (seen.has(seed.id)) continue;
    out.push({ ...cloneCatalog(seed), obtained: false, cdCleared: false });
  }
  return out;
}

export function collectItemsToTokens(items: Jx3CollectItem[]): Jx3SetTokenSeries[] {
  const catalog = cloneCatalog(JX3_SET_TOKENS_CATALOG);
  const byId = new Map(catalog.map(s => [s.id, s]));
  for (const it of items) {
    if (it.kind !== 'token' || !it.tokenSeriesId || !it.tokenSlot) continue;
    const series = byId.get(it.tokenSeriesId);
    if (!series) continue;
    series.pieces[it.tokenSlot].obtained = !!it.obtained;
  }
  return catalog;
}

export function groupJx3Collect(
  store: Jx3CollectStore,
  boxList: Jx3BoxDungeon[] = []
): Jx3CollectGroup[] {
  const boxMap = new Map(boxList.map(d => [d.key, d]));
  const groupMap = new Map<string, Jx3CollectGroup>();
  const ensure = (key: string, name: string, era?: string, eraLevel?: string) => {
    let g = groupMap.get(key);
    if (!g) {
      const box = boxMap.get(key);
      g = {
        dungeonKey: key,
        dungeonName: box?.name || name,
        era: box?.era || era,
        eraLevel: box?.eraLevel || eraLevel,
        icon: box?.icon,
        bosses: box?.bosses || [],
        items: [],
        obtainedCount: 0,
        totalCount: 0
      };
      groupMap.set(key, g);
    }
    return g;
  };
  for (const pin of store.pinnedDungeons) {
    ensure(pin.dungeonKey, pin.dungeonName, pin.era, pin.eraLevel);
  }
  for (const it of store.items) {
    const g = ensure(it.dungeonKey, it.dungeonName, it.era);
    g.items.push(it);
  }
  const groups = Array.from(groupMap.values()).map(g => {
    g.totalCount = g.items.length;
    g.obtainedCount = g.items.filter(i => i.obtained).length;
    g.items.sort((a, b) => {
      const obA = a.obtained ? 1 : 0;
      const obB = b.obtained ? 1 : 0;
      if (obA !== obB) return obA - obB;
      const ka = a.kind === 'token' ? 1 : 0;
      const kb = b.kind === 'token' ? 1 : 0;
      if (ka !== kb) return ka - kb;
      if (a.tokenSeriesId && b.tokenSeriesId && a.tokenSeriesId === b.tokenSeriesId) {
        return JX3_TOKEN_SLOTS.indexOf(a.tokenSlot || 'head') - JX3_TOKEN_SLOTS.indexOf(b.tokenSlot || 'head');
      }
      return a.name.localeCompare(b.name, 'zh');
    });
    return g;
  });
  groups.sort((a, b) => {
    const doneA = a.totalCount > 0 && a.obtainedCount === a.totalCount ? 1 : 0;
    const doneB = b.totalCount > 0 && b.obtainedCount === b.totalCount ? 1 : 0;
    if (doneA !== doneB) return doneA - doneB;
    const era = (a.eraLevel || '').localeCompare(b.eraLevel || '', 'zh');
    if (era) return era;
    return a.dungeonName.localeCompare(b.dungeonName, 'zh');
  });
  return groups;
}

export function newJx3CollectItemId(prefix = 'c') {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
}
