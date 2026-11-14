// 剑网3 声望、奇遇、茗伊成就对账。
import { describeError } from '../../services/errors';
import { ref, computed, watch } from 'vue';
import {
  type Character,
  type Jx3AdventureItem,
  type Jx3AdventureRequirement,
  type Jx3ReputationItem,
  type Jx3ReputationLevel,
  type MingyiRoleSnapshot,
  type MingyiScanResult,
  type TaskItem,
} from '../../types';
import { isTauri } from '../../services/api';
import {
  getJx3BoxReputations,
  getJx3BoxItemsByIds,
  getJx3BoxSerendipities,
  scanMingyiRoles,
} from '../../services/jx3Api';
import { pickFolder } from '../../services/systemApi';
import { kvGet, kvGetJson, kvSet } from '../../services/kv';
import { parseMingyiAchievementShot } from '../../services/jx3AchievementParser';
import {
  STORAGE_MINGYI_DATA_ROOT_KEY,
  buildJx3PlacePapers,
  computeAdventureProgress,
  evaluateReputationAction,
  filterJx3PlacePapers,
  loadJx3Adventures,
  jx3AdventuresFetchedAt,
  loadJx3CompletedAchievements,
  loadJx3ReputationCatalog,
  saveJx3Adventures,
  saveJx3CompletedAchievementsForCharacter,
  saveJx3ReputationCatalog,
  loadJx3CharReputationsMap,
  saveJx3CharReputationProgress,
  type Jx3CharReputationProgress,
} from '../../services/jx3Data';
import {
  ADVENTURE_CATALOG_TTL_MS,
  type Jx3BoxSerendipityRow,
  buildJx3AdventureCatalog,
  collectNeedlesFromRows,
  parseItemNamesPayload,
  rewardItemIds,
  parseSerendipityListPayload,
  relinkAdventureReputationRefs,
} from '../../services/jx3AdventureCatalog';
import {
  parseReputationCatalog,
  reputationCatalogPages,
  reputationItemsFromCatalog,
  type Jx3ReputationLane,
  standingsFromAchievements,
} from '../../services/jx3ProgressLedger';
import {
  activeCharObj,
  activeCharacter,
  characterList,
  gameIdOf,
  pushCharacterTask,
  resetRuleFor,
  showToast,
  showAchievementImportModal,
} from '../core';
import { jx3Collect } from './collect';
import { jx3AchievementIconIds } from '../../services/jx3Artwork';
import serendipities from '../../content/jx3/serendipities.json';
import { buildAdventureNameMaps, jx3AchievementGroups, jx3AchievementRecords, jx3AchievementById, knownAchievementPoints } from '../../services/jx3AchievementCatalog';

Object.assign(jx3AchievementIconIds, Object.fromEntries(jx3AchievementRecords.filter(row => row.IconID).map(row => [String(row.ID), row.IconID!])));

const charReputationsMap = ref<Record<string, Record<string, Jx3CharReputationProgress>>>(
  loadJx3CharReputationsMap()
);


export function buildCharacterReputations(
  charId: string,
  completedAchieves: Set<string>
): Jx3ReputationItem[] {
  const catalog = loadJx3ReputationCatalog();
  if (!catalog.length) return [];
  const charProgress = charReputationsMap.value[charId] || {};

  const standingLevels: Record<string, Jx3ReputationLevel> = {};
  if (completedAchieves.size > 0) {
    const standings = standingsFromAchievements(jx3AchievementRecords, completedAchieves);
    for (const s of standings) {
      standingLevels[s.name] = s.level;
    }
  }

  return catalog.map(item => {
    const prog = charProgress[item.id];
    const standingLevel = standingLevels[item.name];
    const levels: Jx3ReputationLevel[] = ['中立', '友好', '亲密', '敬重', '尊敬', '钦佩'];
    const saved = levels.includes(prog?.currentLevel as Jx3ReputationLevel) ? prog?.currentLevel : undefined;
    const currentLevel = levels[Math.max(0, levels.indexOf(saved!), levels.indexOf(standingLevel!))];
    const levelSource = standingLevel && levels.indexOf(standingLevel) >= levels.indexOf(saved!)
      ? 'achievement' as const : saved ? 'recorded' as const : 'unknown' as const;
    const laneOverride = prog?.laneOverride;
    const maxLevelOverride = prog?.maxLevelOverride;
    return {
      ...item,
      currentLevel,
      levelSource,
      preferredMethod: prog?.preferredMethod,
      laneOverride,
      maxLevelOverride,
    };
  });
}

export function syncActiveCharacterReputations() {
  const charId = activeCharacter.value;
  if (!charId) {
    jx3Reputations.value = loadJx3ReputationCatalog();
    return;
  }
  const items = buildCharacterReputations(charId, jx3CompletedAchieves.value);
  if (items.length) {
    jx3Reputations.value = items;
  }
  jx3Adventures.value = relinkAdventureReputationRefs(jx3Adventures.value, jx3Reputations.value);
}

export const jx3Reputations = ref<Jx3ReputationItem[]>(
  activeCharacter.value
    ? buildCharacterReputations(activeCharacter.value, loadJx3CompletedAchievements(activeCharacter.value))
    : loadJx3ReputationCatalog()
);
function resolvedAdventures(rows: Jx3BoxSerendipityRow[], old = loadJx3Adventures()) {
  const maps = buildAdventureNameMaps(collectNeedlesFromRows(rows).seriesPrefixes);
  const previous = new Map(old.map(item => [item.id, item]));
  return buildJx3AdventureCatalog(rows, maps, jx3Reputations.value).map(item => ({ ...item, reward: previous.get(item.id)?.reward || item.reward }));
}
export const jx3Adventures = ref<Jx3AdventureItem[]>(resolvedAdventures(serendipities));
export const jx3AdventureStatus = ref<'idle' | 'ok' | 'fail'>(jx3Adventures.value.length ? 'ok' : 'idle');
export const jx3AdventureQuery = ref('');
const adventureManualByCharacter = ref<Record<string, Record<string, true>>>(kvGetJson('waypoint_jx3_adventure_manual_by_character', {}));
export const jx3AdventureManualDone = computed(() => adventureManualByCharacter.value[activeCharacter.value] || {});
export type Jx3AdventureFilter = 'focus' | 'empty' | 'pet' | 'done';
export const jx3AdventureFilter = ref<Jx3AdventureFilter>('focus');

export const jx3CompletedAchieves = ref<Set<string>>(new Set());
export const jx3PlaceQuery = ref('');
export const jx3DungeonAchieveIds = new Set(jx3AchievementGroups.filter(group => group.sub === 11).flatMap(group => group.ids));
export const jx3AchievementSections = computed(() => jx3AchievementGroups.map(group => ({
  ...group, rows: group.ids.map(id => ({ id, name: jx3AchievementById.get(id)?.Name || id, point: jx3AchievementById.get(id)?.Point || 0, match: !jx3CompletedAchieves.value.size ? 'unlinked' : jx3CompletedAchieves.value.has(id) ? 'done' : 'missing' })),
  done: group.ids.filter(id => jx3CompletedAchieves.value.has(id)).length, total: group.ids.length,
})));
export const jx3SnapshotCoverage = computed(() => ({
  matched: [...jx3CompletedAchieves.value].filter(id => jx3AchievementById.has(id)).length,
  historical: [...jx3CompletedAchieves.value].filter(id => id !== '0' && !jx3AchievementById.has(id)).length,
  points: knownAchievementPoints(jx3CompletedAchieves.value),
}));

export const jx3ReputationFilter = ref<'all' | 'farm' | 'map' | 'both' | 'item' | 'unset' | 'done'>('all');
export const jx3ReputationQuery = ref('');

// 智能声望状态推导 (亲自打本 vs 安排声望道具)
export const evaluatedReputations = computed(() => {
  return jx3Reputations.value.map(rep => {
    const evalRes = evaluateReputationAction(rep, jx3CompletedAchieves.value, jx3Collect.value.items);
    return {
      ...rep,
      ...evalRes
    };
  });
});

function reputationMatchesQuery(rep: (typeof evaluatedReputations.value)[number], q: string): boolean {
  if (!q) return true;
  const hay = [rep.name, rep.group, rep.place, rep.dungeon, rep.gain, rep.recommendedItem, rep.rewardHighlight]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
  return hay.includes(q);
}

export const queriedReputations = computed(() => {
  const q = jx3ReputationQuery.value.trim().toLowerCase();
  return evaluatedReputations.value.filter(rep => reputationMatchesQuery(rep, q));
});

export const farmRecommendedReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'farm'));
export const mapRecommendedReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'map'));
export const bothRecommendedReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'both'));
export const itemRecommendedReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'item'));
export const unsetRecommendedReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'unset'));
export const doneReps = computed(() => queriedReputations.value.filter(r => r.smartAction === 'done'));

export const filteredEvaluatedReputations = computed(() => {
  if (jx3ReputationFilter.value === 'farm') return farmRecommendedReps.value;
  if (jx3ReputationFilter.value === 'map') return mapRecommendedReps.value;
  if (jx3ReputationFilter.value === 'both') return bothRecommendedReps.value;
  if (jx3ReputationFilter.value === 'item') return itemRecommendedReps.value;
  if (jx3ReputationFilter.value === 'unset') return unsetRecommendedReps.value;
  if (jx3ReputationFilter.value === 'done') return doneReps.value;
  return queriedReputations.value;
});

export function setReputationLane(id: string, lane: Jx3ReputationLane) {
  const charId = activeCharacter.value;
  if (!charId) return;
  const rep = jx3Reputations.value.find(item => item.id === id);
  if (rep) rep.laneOverride = lane;
  saveJx3CharReputationProgress(charId, id, { laneOverride: lane });
  if (!charReputationsMap.value[charId]) charReputationsMap.value[charId] = {};
  if (!charReputationsMap.value[charId][id]) {
    charReputationsMap.value[charId][id] = { laneOverride: lane };
  } else {
    charReputationsMap.value[charId][id].laneOverride = lane;
  }
}

export function toggleAdventureManual(reqId: string) {
  if (!activeCharacter.value) return;
  const next = { ...jx3AdventureManualDone.value };
  if (next[reqId]) delete next[reqId]; else next[reqId] = true;
  adventureManualByCharacter.value = { ...adventureManualByCharacter.value, [activeCharacter.value]: next };
  kvSet('waypoint_jx3_adventure_manual_by_character', JSON.stringify(adventureManualByCharacter.value));
}

export function setReputationMethod(id: string, preferredMethod: 'auto' | 'content' | 'item') {
  const charId = activeCharacter.value;
  if (!charId) return;
  saveJx3CharReputationProgress(charId, id, { preferredMethod });
  charReputationsMap.value = loadJx3CharReputationsMap();
  syncActiveCharacterReputations();
}

export function setReputationGoal(id: string, level: Jx3ReputationLevel) {
  const charId = activeCharacter.value;
  if (!charId) return;
  const rep = jx3Reputations.value.find(item => item.id === id);
  if (rep) rep.maxLevelOverride = level;
  saveJx3CharReputationProgress(charId, id, { maxLevelOverride: level });
  if (!charReputationsMap.value[charId]) charReputationsMap.value[charId] = {};
  if (!charReputationsMap.value[charId][id]) {
    charReputationsMap.value[charId][id] = { maxLevelOverride: level };
  } else {
    charReputationsMap.value[charId][id].maxLevelOverride = level;
  }
}

// 奇遇前置对撞达成度推导
export const adventureEvaluations = computed(() => {
  const repMap = new Map(jx3Reputations.value.map(r => [r.id, r]));
  return jx3Adventures.value.map(adv => {
    const progress = computeAdventureProgress(
      adv,
      jx3CompletedAchieves.value,
      repMap,
      new Set(Object.keys(jx3AdventureManualDone.value))
    );
    return {
      adv: { ...adv, requirements: progress.requirements, isTriggered: progress.completeMatch === 'done' },
      ...progress
    };
  });
});

type Jx3AdventureEval = (typeof adventureEvaluations.value)[number];

function adventureMatchesQuery(item: Jx3AdventureEval, q: string): boolean {
  if (!q) return true;
  const hay = [
    item.adv.name,
    item.adv.type,
    ...item.adv.requirements.map(r => r.name)
  ].join(' ').toLowerCase();
  return hay.includes(q);
}

function sortFocusAdventures(a: Jx3AdventureEval, b: Jx3AdventureEval): number {
  const score = (item: Jx3AdventureEval) => {
    if (item.missingRequirements.some(r => r.match === 'missing' || (r.type === 'reputation' && !r.completed && r.refId))) return 0;
    if (item.missingRequirements.some(r => r.match !== 'unlinked')) return 1;
    return 2;
  };
  return score(a) - score(b) || a.adv.name.localeCompare(b.adv.name, 'zh');
}

export const filteredAdventureEvaluations = computed(() => {
  const q = jx3AdventureQuery.value.trim().toLowerCase();
  return adventureEvaluations.value.filter(item => adventureMatchesQuery(item, q));
});

export const jx3AdventureFocusList = computed(() =>
  filteredAdventureEvaluations.value
    .filter(a => a.completeMatch !== 'done' && a.adv.type !== '宠物奇遇' && a.adv.requirements.length > 0)
    .slice()
    .sort(sortFocusAdventures)
);

export const jx3AdventureEmptyList = computed(() =>
  filteredAdventureEvaluations.value
    .filter(a => a.completeMatch !== 'done' && a.adv.type !== '宠物奇遇' && a.adv.requirements.length === 0)
    .slice()
    .sort((a, b) => a.adv.name.localeCompare(b.adv.name, 'zh'))
);

export const jx3AdventurePetList = computed(() =>
  filteredAdventureEvaluations.value
    .filter(a => a.completeMatch !== 'done' && a.adv.type === '宠物奇遇')
    .slice()
    .sort((a, b) => a.adv.name.localeCompare(b.adv.name, 'zh'))
);

export const jx3AdventureDoneList = computed(() =>
  filteredAdventureEvaluations.value
    .filter(a => a.completeMatch === 'done')
    .slice()
    .sort((a, b) => a.adv.name.localeCompare(b.adv.name, 'zh'))
);

export const jx3AdventureGapCount = computed(() =>
  adventureEvaluations.value.filter(a =>
    a.completeMatch !== 'done' && a.adv.type !== '宠物奇遇' && a.adv.requirements.length > 0
  ).length
);

export const jx3AdventureVisibleList = computed(() => {
  if (jx3AdventureFilter.value === 'empty') return jx3AdventureEmptyList.value;
  if (jx3AdventureFilter.value === 'pet') return jx3AdventurePetList.value;
  if (jx3AdventureFilter.value === 'done') return jx3AdventureDoneList.value;
  return jx3AdventureFocusList.value;
});

watch(jx3AdventureQuery, (raw) => {
  if (!raw.trim()) return;
  const buckets: Array<[Jx3AdventureFilter, number]> = [
    ['focus', jx3AdventureFocusList.value.length],
    ['empty', jx3AdventureEmptyList.value.length],
    ['pet', jx3AdventurePetList.value.length],
    ['done', jx3AdventureDoneList.value.length]
  ];
  const current = buckets.find(([id]) => id === jx3AdventureFilter.value);
  if (current && current[1] > 0) return;
  const hit = buckets.find(([, n]) => n > 0);
  if (hit) jx3AdventureFilter.value = hit[0];
});

export const jx3PlacePapers = computed(() => buildJx3PlacePapers(jx3CompletedAchieves.value));

export const filteredJx3PlacePapers = computed(() =>
  filterJx3PlacePapers(jx3PlacePapers.value, jx3PlaceQuery.value)
);

export const jx3DungeonDoneCount = computed(() => {
  let n = 0;
  for (const id of jx3DungeonAchieveIds) {
    if (jx3CompletedAchieves.value.has(id)) n++;
  }
  return n;
});

export const jx3PlaceGapCount = computed(() =>
  jx3PlacePapers.value.reduce((n, p) => n + p.missing.length, 0)
);

// 操作：将奇遇缺口项直接派生为今日/本周待办
export function addAdventureRequirementToTask(req: Jx3AdventureRequirement, advName: string) {
  if (req.match === 'unlinked' || (req.type !== 'achievement' && req.type !== 'reputation')) {
    showToast('还没对上', `${req.name} 不能当缺口派生`);
    return;
  }
  const newTask: TaskItem = {
    id: `adv_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
    character_id: activeCharacter.value,
    title: `【奇遇前置·${advName}】${req.name} (${req.guideNote || '去完成'})`,
    category: 'daily',
    reset_rule: resetRuleFor(activeCharacter.value, 'daily'),
    source_type: 'custom',
    tag: '奇遇攻坚',
    done: false,
    catalog_kind: 'adventure',
    catalog_id: req.refId || req.id
  };
  void pushCharacterTask(newTask, '已派生为待办任务', `已加入【${activeCharObj.value?.name || '当前角色'}】的日常清单`);
}

export function addJx3PlaceRowToTask(row: {
  match: string;
  name: string;
  id: string;
  adventureName?: string;
  guideNote?: string;
  req?: Jx3AdventureRequirement;
}, placeTitle: string) {
  if (row.match === 'unlinked') {
    showToast('还没对上真 ID', `${row.name} 不能当缺口派生`);
    return;
  }
  if (row.match === 'done') return;
  if (row.req) {
    addAdventureRequirementToTask(row.req, row.adventureName || placeTitle);
    return;
  }
  const newTask: TaskItem = {
    id: `ach_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
    character_id: activeCharacter.value,
    title: `【${placeTitle}】${row.name}`,
    category: 'daily',
    reset_rule: resetRuleFor(activeCharacter.value, 'daily'),
    source_type: 'custom',
    tag: '成就缺口',
    done: false,
    catalog_kind: 'achievement',
    catalog_id: row.id
  };
  void pushCharacterTask(newTask, '已派生为待办任务', `已加入【${activeCharObj.value?.name || '当前角色'}】的日常清单`);
}

watch(
  activeCharacter,
  (id) => {
    jx3CompletedAchieves.value = loadJx3CompletedAchievements(id || undefined);
    syncActiveCharacterReputations();
  },
  { immediate: true }
);

async function mapPool<T, R>(items: T[], limit: number, fn: (item: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length);
  let cursor = 0;
  const worker = async () => {
    while (cursor < items.length) {
      const idx = cursor++;
      out[idx] = await fn(items[idx]);
    }
  };
  const n = Math.max(1, Math.min(limit, items.length || 1));
  await Promise.all(Array.from({ length: n }, () => worker()));
  return out;
}

async function fetchAllSerendipityRows() {
  const first = parseSerendipityListPayload(await getJx3BoxSerendipities(1, 50));
  if (!first.list.length) return null;
  const rows = [...first.list];
  if (first.pages > 1) {
    const rest = await Promise.all(
      Array.from({ length: first.pages - 1 }, (_, i) => getJx3BoxSerendipities(i + 2, 50))
    );
    for (const raw of rest) rows.push(...parseSerendipityListPayload(raw).list);
  }
  return rows;
}

// 奖励名查不到只是那一行不显示奖励，不影响目录本身
async function fetchRewardItemNames(rows: Jx3BoxSerendipityRow[]): Promise<Record<string, string>> {
  const ids = Array.from(new Set(rows.flatMap(rewardItemIds)));
  const chunks: string[][] = [];
  for (let i = 0; i < ids.length; i += 50) chunks.push(ids.slice(i, i + 50));
  const pages = await mapPool(chunks, 3, chunk => getJx3BoxItemsByIds(chunk));
  return Object.assign({}, ...pages.map(parseItemNamesPayload));
}

async function fetchReputationCatalog() {
  const firstRaw = await getJx3BoxReputations(1);
  const rows = parseReputationCatalog(firstRaw);
  const pages = reputationCatalogPages(firstRaw);
  for (let page = 2; page <= pages; page++) {
    rows.push(...parseReputationCatalog(await getJx3BoxReputations(page)));
  }
  return rows;
}

async function refreshReputationStandings() {
  const catalog = await fetchReputationCatalog();
  const baseItems = reputationItemsFromCatalog(catalog, []);
  if (baseItems.length) saveJx3ReputationCatalog(baseItems);
  syncActiveCharacterReputations();
}

async function refreshJx3AdventureCatalog() {
  const rows = await fetchAllSerendipityRows();
  if (!rows?.length) return null;
  const catalog = resolvedAdventures(rows, jx3Adventures.value);
  const names = await fetchRewardItemNames(rows);
  for (let i = 0; i < catalog.length; i++) {
    const row = rows.find(row => `adv_${row.dwID}` === catalog[i].id);
    if (row) {
      const ids = rewardItemIds(row);
      const reward = [...new Set(ids.map(id => names[id]).filter(Boolean))];
      if (reward.length) catalog[i].reward = reward.join('、');
    }
  }
  saveJx3Adventures(catalog);
  return catalog;
}

function adventureCatalogFresh(): boolean {
  if (!jx3Adventures.value.length) return false;
  return Date.now() - jx3AdventuresFetchedAt() < ADVENTURE_CATALOG_TTL_MS;
}

let adventureCatalogLoading: Promise<void> | null = null;
export async function reconcileJx3Snapshot() {
  syncActiveCharacterReputations();
  await refreshReputationStandings();
}

export async function loadJx3AdventureCatalog(force = false) {
  if (!force && adventureCatalogFresh()) {
    jx3AdventureStatus.value = 'ok';
    return;
  }
  if (adventureCatalogLoading && !force) return adventureCatalogLoading;
  const hasCache = jx3Adventures.value.length > 0;
  jx3AdventureStatus.value = hasCache ? 'ok' : 'idle';
  adventureCatalogLoading = (async () => {
    try {
      const list = await refreshJx3AdventureCatalog();
      if (!list?.length) {
        jx3AdventureStatus.value = jx3Adventures.value.length ? 'ok' : 'fail';
        return;
      }
      jx3Adventures.value = list;
      jx3AdventureStatus.value = 'ok';
    } finally {
      adventureCatalogLoading = null;
    }
  })();
  if (hasCache && !force) return;
  await adventureCatalogLoading;
}

export const mingyiScan = ref<MingyiScanResult | null>(null);
export const mingyiScanBusy = ref(false);
export const mingyiScanError = ref('');
export const mingyiPasteOpen = ref(false);
export const rawAchievementInput = ref('');
export const mingyiDataRoot = ref(kvGet(STORAGE_MINGYI_DATA_ROOT_KEY) || '');

watch(mingyiDataRoot, (val) => kvSet(STORAGE_MINGYI_DATA_ROOT_KEY, val));

function applyMingyiShotToCharacter(characterId: string, ids: string[]) {
  const set = new Set(ids);
  saveJx3CompletedAchievementsForCharacter(characterId, set);
  if (characterId === activeCharacter.value) {
    jx3CompletedAchieves.value = set;
    syncActiveCharacterReputations();
  }
}

function matchMingyiRoleToCharacter(role: MingyiRoleSnapshot): Character | undefined {
  const jx3Chars = characterList.value.filter(c => c.game_id === 'jx3');
  const name = (role.name || '').trim();
  const server = (role.server || '').trim();
  const exact = jx3Chars.find(c => c.name === name && (!server || !c.server || c.server === server));
  if (exact) return exact;
  const byName = jx3Chars.filter(c => c.name === name);
  if (byName.length === 1) return byName[0];
  return undefined;
}

export function matchedMingyiCharacter(role: MingyiRoleSnapshot): Character | undefined {
  return matchMingyiRoleToCharacter(role);
}

export async function scanMingyiLocal(path?: string) {
  if (!isTauri()) {
    mingyiScanError.value = '请在桌面端扫描本机茗伊目录';
    showToast('仅桌面端', '浏览器调试读不到游戏目录');
    return;
  }
  mingyiScanBusy.value = true;
  mingyiScanError.value = '';
  try {
    const hint = (path ?? mingyiDataRoot.value).trim() || null;
    const result = await scanMingyiRoles(hint);
    mingyiScan.value = result;
    mingyiDataRoot.value = result.data_root;
    if (!result.roles.length) {
      showToast('未发现角色', result.warnings[0] || 'my#data 里没有 uid@edition 目录');
    }
  } catch (err: any) {
    mingyiScanError.value = (err ? describeError(err).message : '扫描失败');
    showToast('扫描失败', mingyiScanError.value);
  } finally {
    mingyiScanBusy.value = false;
  }
}

export async function handlePickMingyiFolder() {
  try {
    const selected = await pickFolder('选择剑网3 安装目录或 interface/my#data');
    if (!selected) return;
    mingyiDataRoot.value = selected;
    await scanMingyiLocal(selected);
  } catch (err: any) {
    showToast('选择目录失败', (err ? describeError(err).message : '无法打开系统选择器'));
  }
}

export async function openAchievementImportModal() {
  showAchievementImportModal.value = true;
  mingyiPasteOpen.value = false;
  rawAchievementInput.value = '';
  if (!mingyiScan.value && !mingyiScanBusy.value) {
    await scanMingyiLocal();
  }
}

export async function applyMingyiRole(role: MingyiRoleSnapshot) {
  if (role.shot_error) {
    showToast('这份快照读不了', role.shot_error);
    return;
  }
  const matched = matchMingyiRoleToCharacter(role);
  if (!matched) {
    showToast('对不上角色', `金库里没有「${role.name}」。先在账号金库加上同名角色再同步。`);
    return;
  }
  applyMingyiShotToCharacter(matched.id, role.acquired_ids);
  if (matched.id !== activeCharacter.value) {
    activeCharacter.value = matched.id;
  }
  await reconcileJx3Snapshot();
  void loadJx3AdventureCatalog(true);
  showToast(
    `已同步 ${role.name}`,
    `${role.acquired_count} 项已完成成就写入当前角色，声望推导和奇遇前置会跟着变`
  );
}

export function applyAllMingyiRoles() {
  const roles = mingyiScan.value?.roles || [];
  let n = 0;
  const missed: string[] = [];
  for (const role of roles) {
    if (role.shot_error || !role.acquired_ids.length) {
      missed.push(role.name || role.uid);
      continue;
    }
    const matched = matchMingyiRoleToCharacter(role);
    if (!matched) {
      missed.push(role.name || role.uid);
      continue;
    }
    applyMingyiShotToCharacter(matched.id, role.acquired_ids);
    n++;
  }
  if (n) {
    showToast(
      `已写入 ${n} 个角色`,
      missed.length ? `没对上：${missed.join('、')}` : '按角色名对上金库'
    );
  } else {
    showToast('没有写入', missed.length ? `没对上：${missed.join('、')}` : '扫描结果是空的');
  }
}

export function handleImportJx3Achievements() {
  const raw = rawAchievementInput.value.trim();
  if (!raw) {
    showToast('请粘贴快照', '格式是 return {0,1,2,...}，来自 achievement_acquire_shot.jx3dat');
    return;
  }
  if (gameIdOf(activeCharacter.value) !== 'jx3' || !activeCharacter.value) {
    showToast('先切到剑三角色', '粘贴导入写入当前角色');
    return;
  }
  try {
    const parsed = parseMingyiAchievementShot(raw);
    applyMingyiShotToCharacter(activeCharacter.value, parsed.ids);
    showToast(`已写入 ${parsed.ids.length} 项`, '当前角色成就清单已替换为这份快照');
  } catch (err: any) {
    showToast('格式不对', (err ? describeError(err).message : '需要 return {0,1,2,...}'));
  }
}
