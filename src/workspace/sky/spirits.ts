// 光遇常驻星盘先祖：地图分页、筛选排序与光翼/装扮打卡。
import { ref, computed, watch } from 'vue';
import type { CollectionItem } from '../../types';
import { kvGet, kvSet } from '../../services/kv';
import { loadSkySpirits, setSkySpiritProgress } from '../../services/skyData';
import { progressRevision } from '../../services/progress';
import { activeSkyCharacter } from '../vault';
import { saveCollection } from './collection';
import { state } from '../core';

// 光遇常驻星盘先祖统计与地图分页 (晨岛/云野/雨林/霞谷/暮土/禁阁/全图)
export const SKY_MAP_LIST = ['晨岛', '云野', '雨林', '霞谷', '暮土', '禁阁', 'all'] as const;
export type SkyMapName = typeof SKY_MAP_LIST[number];
export const initialSkyMap = new URLSearchParams(window.location.search).get('skyMap') as SkyMapName | null;
export const collectionMapFilter = ref<SkyMapName>(
  initialSkyMap || (kvGet('waypoint_sky_map_filter') as SkyMapName) || '晨岛'
);
watch(collectionMapFilter, (val) => {
  kvSet('waypoint_sky_map_filter', val);
});

export function prevMap() {
  const idx = SKY_MAP_LIST.indexOf(collectionMapFilter.value);
  if (idx <= 0 || idx === 6) {
    collectionMapFilter.value = '禁阁';
  } else {
    collectionMapFilter.value = SKY_MAP_LIST[idx - 1];
  }
}

export function nextMap() {
  const idx = SKY_MAP_LIST.indexOf(collectionMapFilter.value);
  if (idx >= 5) {
    collectionMapFilter.value = '晨岛';
  } else {
    collectionMapFilter.value = SKY_MAP_LIST[idx + 1];
  }
}

export const currentMapStats = computed(() => {
  const currentMap = collectionMapFilter.value;
  const items = currentMap === 'all' 
    ? state.collectionItems 
    : state.collectionItems.filter(i => i.map === currentMap);
  const totalW = items.reduce((s, i) => s + i.wings, 0);
  const unlockedW = items.filter(i => i.wingUnlocked).reduce((s, i) => s + i.wings, 0);
  const totalItems = items.length;
  const obtainedItems = items.filter(i => i.itemObtained).length;
  const graduatedSpirits = items.filter(i => i.status === 'unlocked').length;
  return {
    mapName: currentMap === 'all' ? '全图' : currentMap,
    totalSpirits: items.length,
    graduatedSpirits,
    totalW,
    unlockedW,
    totalItems,
    obtainedItems
  };
});

export const collectionSortBy = ref<'default' | 'missing_wing' | 'missing_item' | 'locked_only' | 'red_asc' | 'red_desc' | 'wings_desc'>('default');

export const unlockedWings = computed(() => {
  return state.collectionItems
    .filter(i => i.wingUnlocked)
    .reduce((s, i) => s + i.wings, 0);
});
export const totalWings = computed(() => {
  return state.collectionItems.reduce((s, i) => s + i.wings, 0);
});
export const obtainedCoreItemsCount = computed(() => {
  return state.collectionItems.filter(i => i.itemObtained).length;
});
export const unlockedSpiritsCount = computed(() => {
  return state.collectionItems.filter(i => i.status === 'unlocked').length;
});

function isSpiritFullyOwned(item: { status?: string; wingUnlocked?: boolean; itemObtained?: boolean }) {
  return item.status === 'unlocked';
}

export const filteredCollectionItems = computed(() => {
  let list = [...state.collectionItems];
  if (collectionMapFilter.value !== 'all') {
    list = list.filter(item => item.map === collectionMapFilter.value);
  }
  if (collectionSortBy.value === 'locked_only') {
    list = list.filter(item => item.status === 'locked');
  } else if (collectionSortBy.value === 'missing_wing') {
    list = list.filter(item => !item.wingUnlocked);
  } else if (collectionSortBy.value === 'missing_item') {
    list = list.filter(item => !item.itemObtained);
  } else if (collectionSortBy.value === 'red_asc') {
    list.sort((a, b) => a.redCandles - b.redCandles);
  } else if (collectionSortBy.value === 'red_desc') {
    list.sort((a, b) => b.redCandles - a.redCandles);
  } else if (collectionSortBy.value === 'wings_desc') {
    list.sort((a, b) => b.wings - a.wings);
  }
  if (
    collectionSortBy.value !== 'locked_only'
    && collectionSortBy.value !== 'missing_wing'
    && collectionSortBy.value !== 'missing_item'
  ) {
    const pending = list.filter(item => !isSpiritFullyOwned(item));
    const owned = list.filter(item => isSpiritFullyOwned(item));
    list = [...pending, ...owned];
  }
  return list;
});

watch([() => activeSkyCharacter.value?.id || '', progressRevision], ([characterId]) => {
  const records = new Map(loadSkySpirits(String(characterId)).map(s => [s.id, s]));
  for (const item of state.collectionItems) {
    const record = records.get(item.id);
    item.itemObtained = !!record?.itemObtained;
    item.wingUnlocked = !!record?.wingUnlocked;
    item.status = record?.unlocked ? 'unlocked' : 'locked';
  }
}, { immediate: true });

export function toggleSpiritWing(item: CollectionItem) {
  const id = activeSkyCharacter.value?.id || '';
  return saveCollection(() => setSkySpiritProgress(id, item.id, { wingUnlocked: !item.wingUnlocked }));
}
export function toggleSpiritItem(item: CollectionItem) {
  const id = activeSkyCharacter.value?.id || '';
  return saveCollection(() => setSkySpiritProgress(id, item.id, { itemObtained: !item.itemObtained }));
}
export function toggleSpiritGraduated(item: CollectionItem) {
  const id = activeSkyCharacter.value?.id || '';
  const obtained = item.status !== 'unlocked';
  return saveCollection(() => setSkySpiritProgress(id, item.id, { unlocked: obtained, wingUnlocked: obtained, itemObtained: obtained }));
}
