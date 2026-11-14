import type { Jx3ReputationItem, Jx3ReputationLevel } from '../types';

export interface Jx3BoxAchievementRow {
  ID?: number;
  IconID?: number;
  Name?: string;
  Desc?: string;
  ShortDesc?: string;
  Sub?: number;
  Point?: number;
}

export interface Jx3ReputationStanding {
  id: string;
  name: string;
  group: string;
  level: Jx3ReputationLevel;
  achieveId: string;
  achieveName: string;
}

const LEVELS: Jx3ReputationLevel[] = ['中立', '友好', '亲密', '敬重', '尊敬', '钦佩'];
const LEVEL_RANK: Record<string, number> = {
  仇恨: -3,
  敌视: -2,
  疏远: -1,
  中立: 0,
  友善: 1,
  友好: 1,
  亲密: 2,
  敬重: 3,
  尊敬: 4,
  钦佩: 5,
  显赫: 6,
  崇敬: 7,
  崇拜: 8,
  传说: 9,
};

const REP_LEVEL = /(.+?)声望(?:达到|到达|到)(仇恨|敌视|疏远|中立|友善|友好|亲密|敬重|尊敬|钦佩|显赫|崇敬|崇拜|传说)/;

function clampLevel(rank: number): Jx3ReputationLevel {
  if (rank >= 5) return '钦佩';
  if (rank <= 0) return '中立';
  return LEVELS[rank] || '中立';
}

export function parseReputationStanding(row: Jx3BoxAchievementRow): {
  name: string;
  level: Jx3ReputationLevel;
} | null {
  const id = row.ID != null ? String(row.ID) : '';
  if (!/^\d+$/.test(id)) return null;
  const text = `${row.Desc || ''}${row.ShortDesc || ''}`.replace(/[。．]/g, '');
  const hit = text.match(REP_LEVEL);
  if (!hit) return null;
  const name = hit[1].trim();
  const rank = LEVEL_RANK[hit[2]];
  if (!name || rank == null || rank < 0) return null;
  return { name, level: clampLevel(rank) };
}

export function standingsFromAchievements(
  rows: Jx3BoxAchievementRow[],
  completed: Set<string>
): Jx3ReputationStanding[] {
  const best = new Map<string, Jx3ReputationStanding & { rank: number }>();
  for (const row of rows) {
    const parsed = parseReputationStanding(row);
    const id = row.ID != null ? String(row.ID) : '';
    if (!parsed || !completed.has(id)) continue;
    const prev = best.get(parsed.name);
    const done = completed.has(id);
    const rank = done ? (LEVEL_RANK[parsed.level] ?? 0) : 0;
    if (!prev) {
      best.set(parsed.name, {
        id: `rep_${parsed.name}`,
        name: parsed.name,
        group: '声望',
        level: done ? parsed.level : '中立',
        achieveId: done ? id : '',
        achieveName: done ? (row.Name || parsed.name) : '',
        rank,
      });
      continue;
    }
    if (!done || rank < prev.rank) continue;
    prev.level = parsed.level;
    prev.achieveId = id;
    prev.achieveName = row.Name || parsed.name;
    prev.rank = rank;
  }
  return Array.from(best.values())
    .map(({ rank: _rank, ...standing }) => standing)
    .sort((a, b) => a.name.localeCompare(b.name, 'zh'));
}

export type Jx3ReputationLane = 'dungeon' | 'map' | 'both' | 'item' | 'unset';

export interface Jx3BoxReputationRow {
  dwForceID?: number;
  szName?: string;
  szIconPath?: string;
  GroupName?: string;
  szMapNames?: string[];
  GainDesc?: Array<{ desc?: string; from?: string; to?: string }>;
}

const DUNGEON_GAIN = /首领|副本|秘境/;
const MAP_GAIN = /日常|周常|任务|历程|地图|阵营|战场|跑商|采集|探索/;
const ITEM_GAIN = /上缴|声望书|道具/;

export function reputationLaneFromGain(text: string): Jx3ReputationLane {
  const src = text || '';
  const dungeon = DUNGEON_GAIN.test(src);
  const map = MAP_GAIN.test(src);
  if (dungeon && map) return 'both';
  if (dungeon) return 'dungeon';
  if (map) return 'map';
  if (ITEM_GAIN.test(src)) return 'item';
  return 'unset';
}

const GAIN_LEVEL: Record<string, Jx3ReputationLevel> = {
  '4': '友好',
  '5': '亲密',
  '6': '敬重',
  '7': '尊敬',
  '8': '钦佩',
};

export function reputationCapFromGain(steps: Array<{ to?: string }> | undefined): Jx3ReputationLevel {
  let best = 0;
  for (const step of steps || []) {
    const rank = Number(step.to);
    if (Number.isFinite(rank) && rank > best) best = rank;
  }
  return GAIN_LEVEL[String(best)] || '钦佩';
}

const ITEM_NAME = /【([^】]+)】/g;

export function reputationItemNames(text: string): string[] {
  const names: string[] = [];
  for (const hit of text.matchAll(ITEM_NAME)) {
    const name = hit[1].trim();
    if (name && /声望|遗失的尊敬|迟来的钦佩|翰墨/.test(name) && !names.includes(name)) names.push(name);
  }
  return names;
}

export function parseReputationCatalog(raw: unknown): Jx3BoxReputationRow[] {
  const root = raw && typeof raw === 'object' ? raw as Record<string, unknown> : {};
  const list = Array.isArray(root.list) ? root.list : [];
  return list.filter(row => row && typeof row === 'object') as Jx3BoxReputationRow[];
}

export function reputationCatalogPages(raw: unknown): number {
  const root = raw && typeof raw === 'object' ? raw as Record<string, unknown> : {};
  const total = Number(root.total) || 0;
  const per = Number(root.per) || 50;
  return Number(root.pages) || Math.max(1, Math.ceil(total / Math.max(1, per)));
}

export interface Jx3ReputationOverride {
  lane?: Jx3ReputationLane;
  maxLevel?: Jx3ReputationLevel;
}

export function reputationItemsFromCatalog(
  rows: Jx3BoxReputationRow[],
  standings: Jx3ReputationStanding[],
  overrides: Record<string, Jx3ReputationOverride> = {}
): Jx3ReputationItem[] {
  const byName = new Map(standings.map(standing => [standing.name, standing]));
  const items: Jx3ReputationItem[] = [];
  for (const row of rows) {
    const name = String(row.szName || '').trim();
    const id = row.dwForceID != null ? `force_${row.dwForceID}` : '';
    if (!name || !id) continue;
    const gain = (row.GainDesc || []).map(step => String(step.desc || '').trim()).filter(Boolean).join(' ');
    const place = (row.szMapNames || []).map(map => String(map).trim()).filter(Boolean).join(' · ');
    const standing = byName.get(name);
    const lane = reputationLaneFromGain(gain);
    const cap = reputationCapFromGain(row.GainDesc);
    items.push({
      id,
      name,
      iconPath: row.szIconPath || undefined,
      group: String(row.GroupName || '声望'),
      place: place || undefined,
      dungeon: lane === 'dungeon' || lane === 'both' ? place || name : undefined,
      gain: gain || undefined,
      lane,
      laneOverride: overrides[id]?.lane,
      maxLevelOverride: overrides[id]?.maxLevel,
      currentLevel: standing?.level || '中立',
      maxLevel: cap,
      recommendedItem: reputationItemNames(gain).join(' / '),
      itemGain: 0,
      rewardHighlight: standing?.achieveName || '',
    });
  }
  return items.sort((a, b) => a.name.localeCompare(b.name, 'zh'));
}
