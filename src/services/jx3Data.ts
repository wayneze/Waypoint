import { normalizeJx3DungeonLabel } from './jx3CollectData';
import type {
  Jx3ReputationItem,
  Jx3AdventureItem,
  Jx3AdventureRequirement,
  Jx3ReputationLevel,
  Jx3AchieveMatch,
  Jx3PlaceAchieveRow,
  Jx3PlacePaper
} from '../types';
import { achievementsAtPlaces, jx3AchievementGroups, jx3AchievementById, knownAchievementPoints } from './jx3AchievementCatalog';
import reputationCatalog from '../content/jx3/reputation-catalog.json';
import { cacheFromThisBuild, cacheGet, cacheSet } from './fetchCache';
import { progressCharacters, progressEntries, progressMap, progressReplace, progressSet } from './progress';
import { ADVENTURES_CACHE_KEY, relinkAdventureReputationRefs } from './jx3AdventureCatalog';
import { reputationItemsFromCatalog, type Jx3ReputationLane } from './jx3ProgressLedger';

const REPUTATION_RANK: Record<Jx3ReputationLevel, number> = {
  '中立': 0,
  '友好': 1,
  '亲密': 2,
  '敬重': 3,
  '尊敬': 4,
  '钦佩': 5
};

function reputationMeets(current: Jx3ReputationLevel, minLevel: Jx3ReputationLevel): boolean {
  return REPUTATION_RANK[current] >= REPUTATION_RANK[minLevel];
}

export function reputationGoalLevel(rep: Pick<Jx3ReputationItem, 'maxLevel' | 'maxLevelOverride'>): Jx3ReputationLevel {
  return rep.maxLevelOverride || rep.maxLevel;
}

/** 茗伊快照是数字 ID。假键（req_* / ach_*）不算对账。 */
export function isJx3AchieveId(id: string | undefined | null): boolean {
  return !!id && /^\d+$/.test(id.trim());
}

export function matchJx3AchieveId(id: string | undefined | null, completed: Set<string>): Jx3AchieveMatch {
  const key = (id || '').trim();
  if (!isJx3AchieveId(key)) return 'unlinked';
  return completed.has(key) ? 'done' : 'missing';
}

export function evaluateReputationAction(
  rep: Jx3ReputationItem,
  completedAchieveIds: Set<string>,
  collectItems: Array<{ dungeonName: string; obtained: boolean }> = []
) {
  const places = (rep.place || rep.dungeon || '').split(' · ').filter(Boolean);
  const rows = achievementsAtPlaces(places, rep.lane === 'dungeon' ? 'dungeon' : rep.lane === 'map' ? 'map' : undefined);
  const missingAchievements = rows.filter(row => !completedAchieveIds.has(String(row.ID)));
  const wanted = collectItems.filter(item => places.some(place => normalizeJx3DungeonLabel(item.dungeonName) === normalizeJx3DungeonLabel(place)));
  const missingCollect = wanted.filter(item => !item.obtained).length;
  const dungeonAchievementMatch: Jx3AchieveMatch | 'empty' = !rows.length || !completedAchieveIds.size ? 'empty' : missingAchievements.length ? 'missing' : 'done';
  const contentDone = dungeonAchievementMatch === 'done' && missingCollect === 0;
  const common = {
    dungeonAchievementsDone: dungeonAchievementMatch === 'done', dungeonAchievementMatch,
    dungeonPendantsObtained: missingCollect === 0,
    achievementCount: rows.length, missingAchievementCount: missingAchievements.length,
    missingCollectCount: missingCollect, contentDone,
  };
  const result = (smartAction: 'farm' | 'map' | 'both' | 'item' | 'unset' | 'done', smartReason: string) => ({ ...common, smartAction, smartReason });
  if (rep.levelSource !== 'unknown' && reputationMeets(rep.currentLevel, reputationGoalLevel(rep)))
    return result('done', `声望已至少达到${reputationGoalLevel(rep)}`);
  if (rep.preferredMethod === 'item') return result('item', '已选择不再刷此处内容，优先核对可用声望道具');
  if (contentDone && rep.preferredMethod !== 'content') return result('item', '关联成就与已添加心愿均已完成，不必为声望重复刷内容');
  const lane = rep.laneOverride || rep.lane;
  const gaps = [missingAchievements.length && completedAchieveIds.size ? `${missingAchievements.length} 项成就` : '', missingCollect ? `${missingCollect} 件心愿` : ''].filter(Boolean).join('、');
  const reason = gaps ? `仍有${gaps}，可顺路提升声望` : '尚不能确认此处内容已清完，可手动选择改用道具';
  if (lane === 'dungeon') return result('farm', reason);
  if (lane === 'map') return result('map', reason);
  if (lane === 'both') return result('both', reason);
  if (lane === 'item') return result('item', rep.recommendedItem || '获取说明包含道具途径，使用前核对适用势力和等级');
  return result('unset', '获取方式与关联内容待核对');
}

function matchAdventureAchieve(req: Jx3AdventureRequirement, completed: Set<string>): Jx3AchieveMatch {
  const ids = (req.seriesIds && req.seriesIds.length)
    ? req.seriesIds.filter(isJx3AchieveId)
    : (isJx3AchieveId(req.refId) ? [String(req.refId)] : []);
  if (!ids.length) return 'unlinked';
  if (req.seriesMode === 'any') {
    const need = Math.max(1, req.targetVal || 1);
    const hit = ids.filter(id => completed.has(id)).length;
    return hit >= need ? 'done' : 'missing';
  }
  if (req.seriesMode === 'all' || ids.length > 1) {
    return ids.every(id => completed.has(id)) ? 'done' : 'missing';
  }
  return matchJx3AchieveId(ids[0], completed);
}

export function computeAdventureProgress(
  adv: Jx3AdventureItem,
  completedAchieveIds: Set<string>,
  reputationMap: Map<string, Jx3ReputationItem>,
  manualDone: ReadonlySet<string> = new Set()
): {
  total: number;
  completedCount: number;
  percentage: number;
  isReady: boolean;
  missingRequirements: Jx3AdventureRequirement[];
  unlinkedCount: number;
  completeMatch: Jx3AchieveMatch;
  requirements: Jx3AdventureRequirement[];
} {
  let completedCount = 0;
  let unlinkedCount = 0;
  const missingRequirements: Jx3AdventureRequirement[] = [];
  const completeMatch = matchJx3AchieveId(adv.completeAchieveId, completedAchieveIds);


  const requirements = adv.requirements.map(req => ({ ...req }));
  for (const req of requirements) {
    const marked = manualDone.has(req.id);
    if (req.type === 'achievement') {
      req.match = matchAdventureAchieve(req, completedAchieveIds);
      if (!completedAchieveIds.size || (req.match === 'missing' && req.unresolvedAlternatives) || (req.match === 'done' && req.unresolvedRequired)) req.match = 'unlinked';
      req.completed = req.match === 'done' || (req.match === 'unlinked' && marked);
      if (req.match === 'unlinked' && !marked) unlinkedCount++;
    } else if (req.type === 'reputation') {
      const refId = req.refId || '';
      const reps = (req.reputationIds || [refId]).map(id => reputationMap.get(id)).filter((rep): rep is Jx3ReputationItem => !!rep);
      const minLevel = req.minLevel || '尊敬';
      const met = reps.some(rep => rep.levelSource !== 'unknown' && reputationMeets(rep.currentLevel, minLevel));
      req.match = met ? 'done' : 'unlinked';
      req.completed = met || marked;
      if (!req.completed) unlinkedCount++;
    } else {
      req.match = 'unlinked';
      req.completed = marked || (req.type === 'point' && !!req.targetVal && knownAchievementPoints(completedAchieveIds) >= req.targetVal);
      if (!req.completed) unlinkedCount++;
    }

    if (req.completed) completedCount++;
    else missingRequirements.push(req);
  }

  const total = adv.requirements.length;
  const percentage = total > 0 ? Math.round((completedCount / total) * 100) : (completeMatch === 'done' ? 100 : 0);
  const isReady = completeMatch === 'done' || (total > 0 && completedCount === total && unlinkedCount === 0);

  return {
    total,
    completedCount,
    percentage,
    isReady,
    missingRequirements,
    unlinkedCount,
    completeMatch,
    requirements
  };
}

export function buildJx3PlacePapers(completedAchieveIds: Set<string>): Jx3PlacePaper[] {
  const buckets = new Map<string, { title: string; kind: 'dungeon' | 'map'; ids: Set<string> }>();
  for (const group of jx3AchievementGroups.filter(group => group.sub === 7 || group.sub === 11)) {
    for (const id of group.ids) {
      const record = jx3AchievementById.get(id);
      if (!record) continue;
      const title = record.SceneName || group.name.replace(/任务$/, '');
      const key = `${group.sub}:${title}`;
      const bucket = buckets.get(key) || { title, kind: group.sub === 11 ? 'dungeon' as const : 'map' as const, ids: new Set<string>() };
      bucket.ids.add(id);
      buckets.set(key, bucket);
    }
  }
  return [...buckets].map(([id, bucket]) => {
    const rows: Jx3PlaceAchieveRow[] = [...bucket.ids].map(id => {
      const record = jx3AchievementById.get(id)!;
      return { key: id, id, name: record.Name, point: record.Point, match: completedAchieveIds.size ? matchJx3AchieveId(id, completedAchieveIds) : 'unlinked', source: 'place' };
    });
    const missing = rows.filter(row => row.match === 'missing');
    const unlinked = rows.filter(row => row.match === 'unlinked');
    const doneCount = rows.filter(row => row.match === 'done').length;
    return { id, title: bucket.title, kind: bucket.kind, rows, missing, unlinked, doneCount, linkedCount: rows.length - unlinked.length,
      dungeonAchievementMatch: (unlinked.length ? 'empty' : missing.length ? 'missing' : 'done') as Jx3AchieveMatch | 'empty' };
  }).sort((a, b) => a.title.localeCompare(b.title, 'zh'));
}

export function filterJx3PlacePapers(papers: Jx3PlacePaper[], query: string): Jx3PlacePaper[] {
  const q = query.trim().toLowerCase();
  if (!q) return papers;
  return papers.filter(p => {
    const hay = [
      p.title,
      p.reputation?.name || '',
      p.reputation?.dungeon || '',
      ...p.rows.map(r => `${r.name} ${r.id} ${r.adventureName || ''}`)
    ].join(' ').toLowerCase();
    return hay.includes(q);
  });
}

const REPUTATION_SCOPE = 'jx3.reputation';
const ACHIEVEMENT_SCOPE = 'jx3.achievement';

const REPUTATIONS_CACHE_KEY = 'jx3.reputations';

export const STORAGE_MINGYI_DATA_ROOT_KEY = 'waypoint_jx3_mingyi_data_root';

export interface Jx3CharReputationProgress {
  currentLevel?: Jx3ReputationLevel;
  laneOverride?: Jx3ReputationLane;
  maxLevelOverride?: Jx3ReputationLevel;
  preferredMethod?: 'auto' | 'content' | 'item';
}

function achievementIds(characterId: string): Set<string> {
  return new Set(progressEntries(ACHIEVEMENT_SCOPE, characterId).map(([id]) => id));
}

/** 角色的已完成成就。角色化以前的全局记录存在 characterId = ''，只在还没有任何角色记录时使用。 */
export function loadJx3CompletedAchievements(characterId?: string): Set<string> {
  if (!characterId) return achievementIds('');
  const own = achievementIds(characterId);
  if (own.size) return own;
  const chars = progressCharacters(ACHIEVEMENT_SCOPE).filter(ch => ch !== '');
  return chars.length ? own : achievementIds('');
}

export function saveJx3CompletedAchievementsForCharacter(characterId: string, ids: Set<string> | string[]) {
  progressReplace(ACHIEVEMENT_SCOPE, Array.from(ids, id => [String(id), true] as [string, unknown]), characterId);
}

export function loadJx3ReputationCatalog(): Jx3ReputationItem[] {
  const stored = cacheFromThisBuild(REPUTATIONS_CACHE_KEY)
    ? cacheGet<Jx3ReputationItem[]>(REPUTATIONS_CACHE_KEY)?.data
    : undefined;
  return Array.isArray(stored) && stored.length ? stored : reputationItemsFromCatalog(reputationCatalog, []);
}

export function saveJx3ReputationCatalog(list: Jx3ReputationItem[]) {
  cacheSet(REPUTATIONS_CACHE_KEY, list);
}

export function loadJx3CharReputationsMap(): Record<string, Record<string, Jx3CharReputationProgress>> {
  const out: Record<string, Record<string, Jx3CharReputationProgress>> = {};
  for (const ch of progressCharacters(REPUTATION_SCOPE)) {
    out[ch] = progressMap<Jx3CharReputationProgress>(REPUTATION_SCOPE, ch);
  }
  return out;
}

export function saveJx3CharReputationProgress(
  characterId: string,
  repId: string,
  patch: Partial<Jx3CharReputationProgress>
) {
  if (!characterId || !repId) return;
  const current = progressMap<Jx3CharReputationProgress>(REPUTATION_SCOPE, characterId)[repId] || {};
  progressSet(REPUTATION_SCOPE, repId, { ...current, ...patch }, characterId);
}

export function loadJx3Adventures(): Jx3AdventureItem[] {
  const parsed = cacheGet<Jx3AdventureItem[]>(ADVENTURES_CACHE_KEY)?.data;
  if (!Array.isArray(parsed)) return [];
  return relinkAdventureReputationRefs(
    parsed.map(adv => ({
      ...adv,
      isTriggered: false,
      requirements: (adv.requirements || []).map(req => ({ ...req, completed: false }))
    })),
    loadJx3ReputationCatalog()
  );
}

/** 奇遇目录上次拉取时间（毫秒）。没有、或是上一版程序拉的（解析规则可能已变）为 0 */
export function jx3AdventuresFetchedAt(): number {
  if (!cacheFromThisBuild(ADVENTURES_CACHE_KEY)) return 0;
  return cacheGet(ADVENTURES_CACHE_KEY)?.fetchedAt ?? 0;
}

export function saveJx3Adventures(list: Jx3AdventureItem[]) {
  cacheSet(ADVENTURES_CACHE_KEY, list);
}
