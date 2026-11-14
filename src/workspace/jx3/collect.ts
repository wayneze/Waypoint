// 剑网3 收集：秘境、挂件、套装牌子。
import { ref, reactive, computed, watch } from 'vue';
import {
  type Jx3BoxDungeon,
  type Jx3BoxItemHit,
  type Jx3CollectItem,
  type Jx3CollectKind,
  type Jx3CollectStore,
  type Jx3DungeonPendant,
  type Jx3SetTokenSeries,
  type Jx3DynamicSet,
  type Jx3DynamicSetPiece,
} from '../../types';
import { getJx3BoxDungeonTree, searchJx3BoxItems } from '../../services/jx3Api';
import {
  JX3_COLLECT_KIND_LABEL,
  JX3_TOKEN_SLOTS,
  collectItemsToPendants,
  collectItemsToTokens,
  detectDynamicSets,
  groupJx3Collect,
  guessJx3CollectKind,
  inferHitDungeon,
  loadJx3CharCollect,
  saveJx3CharCollect,
  newJx3CollectItemId,
  parseJx3BoxDungeonTree,
  parseJx3BoxItemSearch,
  resolveJx3SearchTokens,
  saveJx3Collect,
  searchJx3BoxDungeons,
} from '../../services/jx3Collect';
import { clearJx3CollectCd, jx3CollectCdLabel, jx3CollectCdOpen, jx3CollectCdRule, markJx3CollectCd } from '../../services/jx3CollectCd';
import { showToast, activeCharacter } from '../core';
import fbMapSeed from '../../content/jx3/fb-map.json';
import { cacheGet, cacheSet, reportSource } from '../../services/fetchCache';

const BOX_DUNGEON_CACHE_KEY = 'jx3.box_dungeons';

function getInitialBoxDungeons(): Jx3BoxDungeon[] {
  try {
    const cached = cacheGet<unknown>(BOX_DUNGEON_CACHE_KEY);
    if (cached) {
      const list = parseJx3BoxDungeonTree(cached.data);
      if (list.length) return list;
    }
  } catch (_) {}
  try {
    return parseJx3BoxDungeonTree(fbMapSeed);
  } catch (_) {
    return [];
  }
}

export const jx3BoxDungeons = ref<Jx3BoxDungeon[]>(getInitialBoxDungeons());
export const jx3Collect = ref<Jx3CollectStore>(loadJx3CharCollect(activeCharacter.value, jx3BoxDungeons.value));
export const jx3Pendants = ref<Jx3DungeonPendant[]>(collectItemsToPendants(jx3Collect.value.items));

export const jx3SetTokens = ref<Jx3SetTokenSeries[]>(collectItemsToTokens(jx3Collect.value.items));

export function syncActiveCharacterCollect() {
  const charId = activeCharacter.value;
  jx3Collect.value = loadJx3CharCollect(charId, jx3BoxDungeons.value);
  jx3Pendants.value = collectItemsToPendants(jx3Collect.value.items);
  jx3SetTokens.value = collectItemsToTokens(jx3Collect.value.items);
}

watch(activeCharacter, () => {
  syncActiveCharacterCollect();
}, { immediate: true });

export const jx3CollectQuery = ref('');
export const jx3CollectKindFilter = ref<'all' | Jx3CollectKind>('all');
export const jx3CollectAdd = reactive({
  dungeonKey: '',
  dungeonQuery: '',
  itemQuery: '',
  kind: 'special' as Jx3CollectKind,
  customName: ''
});
export const jx3BoxItemHits = ref<Jx3BoxItemHit[]>([]);
export const jx3DynamicSets = ref<Jx3DynamicSet[]>([]);
export const jx3PendingDungeonHit = ref<Jx3BoxItemHit | null>(null);
export const jx3BoxItemSearching = ref(false);
export const jx3BoxDungeonStatus = ref<'idle' | 'ok' | 'fail'>('idle');

// 拉不到时还有本地种子或上次缓存可用，状态条只提示"旧数据"
function boxDungeonFailed() {
  if (!jx3BoxDungeons.value.length) jx3BoxDungeonStatus.value = 'fail';
  const cached = cacheGet(BOX_DUNGEON_CACHE_KEY);
  reportSource(BOX_DUNGEON_CACHE_KEY, '魔盒副本树', 'stale', '魔盒接口没返回副本树，用的是上次的数据', cached?.fetchedAt ?? 0);
}

function persistJx3Collect() {
  const charId = activeCharacter.value;
  if (charId) {
    saveJx3CharCollect(charId, jx3Collect.value);
  } else {
    saveJx3Collect(jx3Collect.value);
  }
  jx3Pendants.value = collectItemsToPendants(jx3Collect.value.items);
  jx3SetTokens.value = collectItemsToTokens(jx3Collect.value.items);
}

export const jx3CollectGroups = computed(() => {
  const groups = groupJx3Collect(jx3Collect.value, jx3BoxDungeons.value);
  const q = jx3CollectQuery.value.trim().toLowerCase();
  const kind = jx3CollectKindFilter.value;
  return groups
    .map(g => {
      const items = g.items.filter(it => {
        if (kind !== 'all' && it.kind !== kind) return false;
        if (!q) return true;
        const blob = `${it.name} ${g.dungeonName} ${g.era || ''} ${it.boss || ''} ${it.note || ''}`.toLowerCase();
        return blob.includes(q);
      });
      return {
        ...g,
        items,
        totalCount: items.length,
        obtainedCount: items.filter(i => i.obtained).length
      };
    })
    .filter(g => {
      if (g.items.length) return true;
      if (kind !== 'all' || q) return false;
      return jx3Collect.value.pinnedDungeons.some(p => p.dungeonKey === g.dungeonKey);
    });
});

export const jx3CollectProgress = computed(() => {
  const items = jx3Collect.value.items;
  return { obtained: items.filter(i => i.obtained).length, total: items.length };
});

export const jx3CollectDungeonHits = computed(() =>
  searchJx3BoxDungeons(jx3CollectAdd.dungeonQuery, jx3BoxDungeons.value)
);

export const jx3CollectActiveDungeon = computed(() =>
  jx3BoxDungeons.value.find(d => d.key === jx3CollectAdd.dungeonKey)
  || jx3Collect.value.pinnedDungeons.find(d => d.dungeonKey === jx3CollectAdd.dungeonKey)
  || null
);

export const jx3CollectActiveDungeonName = computed(() => {
  const loc = jx3CollectActiveDungeon.value;
  if (!loc) return '';
  return 'name' in loc ? loc.name : loc.dungeonName;
});

function collectDungeonLoc(loc: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string } | null | undefined): { dungeonKey: string; dungeonName: string; era?: string } | null {
  if (!loc) return null;
  if ('key' in loc) return { dungeonKey: loc.key, dungeonName: loc.name, era: loc.era };
  return { dungeonKey: loc.dungeonKey, dungeonName: loc.dungeonName, era: loc.era };
}

export function jx3CollectGroupCd(dungeonKey: string) {
  const dungeon = jx3BoxDungeons.value.find(d => d.key === dungeonKey);
  const item = jx3Collect.value.items.find(it => it.dungeonKey === dungeonKey);
  const rule = jx3CollectCdRule(dungeon, item);
  const open = jx3Collect.value.items.some(it => it.dungeonKey === dungeonKey && jx3CollectCdOpen(it, rule));
  return { rule, open, label: jx3CollectCdLabel(rule) };
}

export function toggleJx3CollectCd(dungeonKey: string) {
  const dungeon = jx3BoxDungeons.value.find(d => d.key === dungeonKey);
  const sample = jx3Collect.value.items.find(it => it.dungeonKey === dungeonKey);
  const rule = jx3CollectCdRule(dungeon, sample);
  if (rule === 'none') return;
  const open = jx3Collect.value.items.some(it => it.dungeonKey === dungeonKey && jx3CollectCdOpen(it, rule));
  jx3Collect.value = open
    ? markJx3CollectCd(jx3Collect.value, dungeonKey, rule)
    : clearJx3CollectCd(jx3Collect.value, dungeonKey);
  persistJx3Collect();
  const name = dungeon?.name || sample?.dungeonName || dungeonKey;
  showToast(open ? '本周期已打' : '本周期未打', `${name} · ${jx3CollectCdLabel(rule)}`);
}

export function toggleJx3CollectItem(id: string) {
  const item = jx3Collect.value.items.find(it => it.id === id);
  if (!item) return;
  item.obtained = !item.obtained;
  jx3Collect.value = {
    ...jx3Collect.value,
    items: [...jx3Collect.value.items]
  };
  persistJx3Collect();
  showToast(item.obtained ? '已勾选' : '已取消', `${JX3_COLLECT_KIND_LABEL[item.kind]} · ${item.name}`);
}

export function removeJx3CollectItem(id: string) {
  const item = jx3Collect.value.items.find(it => it.id === id);
  if (!item) return;
  jx3Collect.value.items = jx3Collect.value.items.filter(it => it.id !== id);
  persistJx3Collect();
  showToast('已移出收集', item.name);
}

export function selectJx3CollectDungeon(dungeonKey: string, dungeonName?: string) {
  const box = jx3BoxDungeons.value.find(d => d.key === dungeonKey);
  jx3CollectAdd.dungeonKey = dungeonKey;
  jx3CollectAdd.dungeonQuery = box?.name || dungeonName || dungeonKey;
}

export function pinJx3CollectDungeon(dungeon: { key?: string; dungeonKey?: string; name?: string; dungeonName?: string; era?: string; eraLevel?: string }) {
  const dungeonKey = dungeon.key || dungeon.dungeonKey || '';
  const dungeonName = dungeon.name || dungeon.dungeonName || dungeonKey;
  if (!dungeonKey) return false;
  const exists = jx3Collect.value.pinnedDungeons.some(p => p.dungeonKey === dungeonKey);
  if (!exists) {
    jx3Collect.value.pinnedDungeons = [
      ...jx3Collect.value.pinnedDungeons,
      { dungeonKey, dungeonName, era: dungeon.era, eraLevel: dungeon.eraLevel }
    ];
    persistJx3Collect();
    showToast('已加入收集本', dungeonName);
  }
  selectJx3CollectDungeon(dungeonKey, dungeonName);
  return true;
}

export function unpinJx3CollectDungeon(dungeonKey: string) {
  const pin = jx3Collect.value.pinnedDungeons.find(p => p.dungeonKey === dungeonKey);
  const leftover = jx3Collect.value.items.filter(it => it.dungeonKey === dungeonKey);
  jx3Collect.value.pinnedDungeons = jx3Collect.value.pinnedDungeons.filter(p => p.dungeonKey !== dungeonKey);
  if (jx3CollectAdd.dungeonKey === dungeonKey) jx3CollectAdd.dungeonKey = '';
  persistJx3Collect();
  if (leftover.length) showToast('已取消置顶', `${pin?.dungeonName || dungeonKey} 仍有 ${leftover.length} 件心愿`);
  else showToast('已移出收集本', pin?.dungeonName || dungeonKey);
}

function collectHasSameItem(dungeonKey: string, name: string, extra?: { boxItemId?: string; catalogId?: string; tokenSeriesId?: string; tokenSlot?: string }) {
  return jx3Collect.value.items.some(it => {
    if (it.dungeonKey !== dungeonKey) return false;
    if (extra?.boxItemId && it.boxItemId === extra.boxItemId) return true;
    if (extra?.catalogId && it.catalogId === extra.catalogId && !it.tokenSlot) return true;
    if (extra?.tokenSeriesId && extra.tokenSlot && it.tokenSeriesId === extra.tokenSeriesId && it.tokenSlot === extra.tokenSlot) return true;
    return it.name === name;
  });
}

export function addJx3CollectItem(item: Omit<Jx3CollectItem, 'id' | 'obtained'> & { id?: string; obtained?: boolean }) {
  if (collectHasSameItem(item.dungeonKey, item.name, item)) {
    showToast('已经加过', item.name);
    jx3CollectAdd.itemQuery = '';
    jx3BoxItemHits.value = [];
    selectJx3CollectDungeon(item.dungeonKey, item.dungeonName);
    return false;
  }
  const next: Jx3CollectItem = {
    ...item,
    id: item.id || newJx3CollectItemId(item.kind === 'token' ? 'tok' : 'c'),
    obtained: !!item.obtained
  };
  jx3Collect.value.items = [...jx3Collect.value.items, next];
  if (!jx3Collect.value.pinnedDungeons.some(p => p.dungeonKey === next.dungeonKey)) {
    jx3Collect.value.pinnedDungeons = [
      ...jx3Collect.value.pinnedDungeons,
      { dungeonKey: next.dungeonKey, dungeonName: next.dungeonName, era: next.era }
    ];
  }
  persistJx3Collect();
  showToast('已加入收集', `${JX3_COLLECT_KIND_LABEL[next.kind]} · ${next.name}`);
  return true;
}

export function addJx3CollectBoxHit(hit: Jx3BoxItemHit, dungeon?: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string }, kind?: Jx3CollectKind) {
  const hitLoc = hit.inferredDungeon ? {
    dungeonKey: hit.inferredDungeon.dungeonKey,
    dungeonName: hit.inferredDungeon.dungeonName,
    era: hit.inferredDungeon.era
  } : null;
  const loc = collectDungeonLoc(dungeon) || hitLoc || collectDungeonLoc(jx3CollectActiveDungeon.value);
  if (!loc) {
    jx3PendingDungeonHit.value = hit;
    showToast('请选择归属秘境', `为「${hit.name}」点选一个秘境即可加入`);
    return false;
  }
  const ok = addJx3CollectItem({
    dungeonKey: loc.dungeonKey,
    dungeonName: loc.dungeonName,
    era: loc.era,
    kind: kind || guessJx3CollectKind(hit),
    name: hit.name,
    source: 'box',
    boxItemId: hit.id,
    iconId: hit.iconId,
    boss: hit.inferredDungeon?.boss,
    note: [hit.typeLabel, hit.getType, hit.getSource].filter(Boolean).join(' · ') || undefined
  });
  if (ok) {
    jx3PendingDungeonHit.value = null;
    jx3CollectAdd.itemQuery = '';
    jx3BoxItemHits.value = [];
    jx3DynamicSets.value = [];
    selectJx3CollectDungeon(loc.dungeonKey, loc.dungeonName);
  }
  return ok;
}

export function assignPendingHitDungeon(dungeon: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string }) {
  if (!jx3PendingDungeonHit.value) return false;
  const hit = jx3PendingDungeonHit.value;
  return addJx3CollectBoxHit(hit, dungeon);
}

export function clearPendingHitDungeon() {
  jx3PendingDungeonHit.value = null;
}

export function addJx3DynamicSetAll(
  set: Jx3DynamicSet,
  fallbackDungeon?: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string }
) {
  let addedCount = 0;
  const fallback = collectDungeonLoc(fallbackDungeon) || collectDungeonLoc(jx3CollectActiveDungeon.value);

  for (const slot of JX3_TOKEN_SLOTS) {
    const piece = set.pieces[slot];
    if (!piece || piece.inCollect) continue;
    const pieceLoc = piece.dungeonKey ? {
      dungeonKey: piece.dungeonKey,
      dungeonName: piece.dungeonName || piece.dungeonKey,
      era: piece.era
    } : fallback;

    if (!pieceLoc) continue;

    const ok = addJx3CollectItem({
      dungeonKey: pieceLoc.dungeonKey,
      dungeonName: pieceLoc.dungeonName,
      era: pieceLoc.era,
      kind: 'token',
      name: piece.name,
      source: piece.hit ? 'box' : 'seed',
      boxItemId: piece.hit?.id,
      iconId: piece.hit?.iconId,
      tokenSlot: slot,
      tokenSeriesId: set.seriesName,
      boss: piece.boss
    });
    if (ok) {
      piece.inCollect = true;
      addedCount++;
    }
  }

  if (addedCount > 0) {
    showToast(`已加入 ${set.seriesName}`, `共添加 ${addedCount} 件心愿到对应秘境`);
    jx3CollectAdd.itemQuery = '';
    jx3BoxItemHits.value = [];
    jx3DynamicSets.value = [];
  } else {
    showToast('未添加新部位', '所含部位已在心愿清单或缺少秘境归属');
  }
  return addedCount > 0;
}

export function addJx3DynamicPiece(
  piece: Jx3DynamicSetPiece,
  seriesName: string,
  fallbackDungeon?: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string }
) {
  const pieceLoc = piece.dungeonKey ? {
    dungeonKey: piece.dungeonKey,
    dungeonName: piece.dungeonName || piece.dungeonKey,
    era: piece.era
  } : collectDungeonLoc(fallbackDungeon) || collectDungeonLoc(jx3CollectActiveDungeon.value);

  if (!pieceLoc) {
    if (piece.hit) {
      jx3PendingDungeonHit.value = piece.hit;
    }
    showToast('请选择归属秘境', `为「${piece.name}」点选秘境即可加入`);
    return false;
  }

  const ok = addJx3CollectItem({
    dungeonKey: pieceLoc.dungeonKey,
    dungeonName: pieceLoc.dungeonName,
    era: pieceLoc.era,
    kind: 'token',
    name: piece.name,
    source: piece.hit ? 'box' : 'seed',
    boxItemId: piece.hit?.id,
    iconId: piece.hit?.iconId,
    tokenSlot: piece.slot,
    tokenSeriesId: seriesName,
    boss: piece.boss
  });

  if (ok) {
    piece.inCollect = true;
  }
  return ok;
}

export function addJx3CollectCustom(name: string, kind: Jx3CollectKind = jx3CollectAdd.kind, dungeon?: Jx3BoxDungeon | { dungeonKey: string; dungeonName: string; era?: string }) {
  const title = name.trim();
  if (!title) return false;
  const inferred = inferHitDungeon({ name: title }, jx3BoxDungeons.value);
  const loc = collectDungeonLoc(dungeon) || inferred || collectDungeonLoc(jx3CollectActiveDungeon.value);
  if (!loc) {
    showToast('先选一个副本', '从魔盒名单点选后再手写');
    return false;
  }
  const ok = addJx3CollectItem({
    dungeonKey: loc.dungeonKey,
    dungeonName: loc.dungeonName,
    era: loc.era,
    kind,
    name: title,
    source: 'custom',
    boss: inferred?.boss
  });
  if (ok) {
    jx3CollectAdd.customName = '';
    jx3CollectAdd.itemQuery = '';
    jx3BoxItemHits.value = [];
    jx3DynamicSets.value = [];
  }
  return ok;
}

let boxItemSearchTimer: ReturnType<typeof setTimeout> | null = null;
export function scheduleJx3BoxItemSearch(keyword: string) {
  jx3CollectAdd.itemQuery = keyword;
  if (boxItemSearchTimer) clearTimeout(boxItemSearchTimer);
  const q = keyword.trim();
  if (q.length < 1) {
    jx3BoxItemHits.value = [];
    jx3DynamicSets.value = [];
    jx3BoxItemSearching.value = false;
    return;
  }
  jx3BoxItemSearching.value = true;
  // 本地图鉴若能匹配该套装，0 延迟秒出卡片
  if (q.length >= 2) {
    const instantSets = detectDynamicSets([], jx3BoxDungeons.value, jx3Collect.value.items, q);
    if (instantSets.length) jx3DynamicSets.value = instantSets;
  }
  boxItemSearchTimer = setTimeout(async () => {
    const searchTerms = resolveJx3SearchTokens(q);
    const searchPromises = searchTerms.map(term => searchJx3BoxItems(term));
    const results = await Promise.all(searchPromises);
    if (jx3CollectAdd.itemQuery.trim() !== q) return;

    const allHitsMap = new Map<string, Jx3BoxItemHit>();
    for (let i = 0; i < searchTerms.length; i++) {
      const raw = results[i];
      const termHits = parseJx3BoxItemSearch(raw, jx3BoxDungeons.value, searchTerms[i]);
      for (const h of termHits) {
        if (!allHitsMap.has(h.id)) {
          allHitsMap.set(h.id, h);
        }
      }
    }
    const combinedHits = Array.from(allHitsMap.values());
    combinedHits.sort((a, b) => {
      const aRaw = a.name.includes(q) ? 2 : (searchTerms.some(t => a.name.includes(t)) ? 1 : 0);
      const bRaw = b.name.includes(q) ? 2 : (searchTerms.some(t => b.name.includes(t)) ? 1 : 0);
      if (aRaw !== bRaw) return bRaw - aRaw;
      return 0;
    });

    jx3BoxItemHits.value = combinedHits;
    jx3DynamicSets.value = detectDynamicSets(combinedHits, jx3BoxDungeons.value, jx3Collect.value.items, q);
    jx3BoxItemSearching.value = false;
  }, 280);
}

let boxDungeonLoading: Promise<void> | null = null;
let lastBoxDungeonCheck = 0;
export async function loadJx3BoxDungeonTree(force = false) {
  // 如果已有完整数据且不是强制刷新，24 小时内无需重复打远端
  if (!force && jx3BoxDungeons.value.length > 0 && Date.now() - lastBoxDungeonCheck < 24 * 60 * 60 * 1000) {
    jx3BoxDungeonStatus.value = 'ok';
    return;
  }
  if (boxDungeonLoading && !force) return boxDungeonLoading;
  boxDungeonLoading = (async () => {
    try {
      const raw = await getJx3BoxDungeonTree();
      if (!raw) {
        boxDungeonFailed();
        return;
      }
      const list = parseJx3BoxDungeonTree(raw);
      if (!list.length) {
        boxDungeonFailed();
        return;
      }
      jx3BoxDungeons.value = list;
      jx3BoxDungeonStatus.value = 'ok';
      reportSource(BOX_DUNGEON_CACHE_KEY, '魔盒副本树', 'ok');
      lastBoxDungeonCheck = Date.now();
      cacheSet(BOX_DUNGEON_CACHE_KEY, raw);
      syncActiveCharacterCollect();
    } catch (_) {
      boxDungeonFailed();
    }
  })();
  try {
    await boxDungeonLoading;
  } finally {
    boxDungeonLoading = null;
  }
}