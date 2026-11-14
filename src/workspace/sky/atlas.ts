// 光遇探索：神殿、小金人、动作打卡，大神同步小金人。
import { ref, reactive, computed, watch } from 'vue';
import { fetchSkyRoleWings, type SkyRealmWings } from '../../services/skyApi';
import { kvGet, kvSet } from '../../services/kv';
import {
  ATLAS_ACTION_COUNT,
  ATLAS_AREAS,
  ATLAS_LIGHT_COUNT,
  ATLAS_REALMS,
  ATLAS_REALM_FILTERS,
  ATLAS_SHRINE_COUNT,
  atlasActionById,
  atlasAreaById,
  atlasLightById,
  atlasLightHint,
  atlasRealmById,
  atlasShrineById,
  atlasShrineHint,
  applyDashenLightBuffs,
  claimSkyAtlasChecks,
  loadSkyAtlasChecks,
  saveSkyAtlasChecks,
  type AtlasAction,
  type SkyAtlasChecks,
} from '../../services/skyAtlas';
import { showToast, state } from '../core';
import { activeSkyCharacter, skyCharacters } from '../vault';
import { collectionMapFilter } from './spirits';
import { setSkySubTab, sinkChecked } from './tabs';

export const ATLAS_REALM_TABS = ATLAS_REALM_FILTERS;
export const atlasRealmFilter = ref<string>(
  kvGet('waypoint_sky_atlas_realm') || 'all'
);
watch(atlasRealmFilter, val => {
  kvSet('waypoint_sky_atlas_realm', val);
});

export type AtlasMode = 'flow' | 'lights' | 'shrines' | 'actions';
export const atlasMode = ref<AtlasMode>(
  (() => {
    const m = kvGet('waypoint_sky_atlas_mode');
    if (m === 'spirits' || m === 'actions') return 'actions';
    return (m as AtlasMode) || 'flow';
  })()
);
watch(atlasMode, val => {
  kvSet('waypoint_sky_atlas_mode', val);
});

export const atlasHideCollected = ref(false);
export const atlasQuery = ref('');
export type AtlasActionKind = 'all' | 'regular' | 'season';
export const atlasActionKind = ref<AtlasActionKind>('all');
export { activeSkyCharacter as atlasCharacter } from '../vault';
const atlasCharacterId = computed(() => activeSkyCharacter.value?.id || '');
const atlasChecks = reactive<SkyAtlasChecks>({ shrines: {}, lights: {}, actions: {} });
export const atlasDashenMaps = ref<SkyRealmWings[]>([]);
export const atlasDashenSummary = ref('');
const pendingSync = ref<{ characterId: string; name: string } | null>(null);
export const isSyncingAtlasDashen = computed(() => !!pendingSync.value);
export const atlasSyncingName = computed(() => pendingSync.value?.name || '');
export const isClaimingAtlasLegacy = ref(false);
const isSavingAtlasChecks = ref(false);
export const atlasCanEdit = computed(() => !!atlasCharacterId.value
  && !isClaimingAtlasLegacy.value && !isSavingAtlasChecks.value
  && pendingSync.value?.characterId !== atlasCharacterId.value);
export const atlasQueryId = computed(() => activeSkyCharacter.value?.query_id?.trim() || '');
const legacyChecks = ref(loadSkyAtlasChecks(''));
export const atlasLegacyCount = computed(() =>
  Object.values(legacyChecks.value).reduce((n, rows) => n + Object.keys(rows).length, 0));

function clearDashenSummary() {
  atlasDashenMaps.value = [];
  atlasDashenSummary.value = '';
}

watch(atlasCharacterId, id => {
  Object.assign(atlasChecks, id ? loadSkyAtlasChecks(id) : { shrines: {}, lights: {}, actions: {} });
  clearDashenSummary();
}, { immediate: true, flush: 'sync' });
watch(atlasQueryId, clearDashenSummary, { flush: 'sync' });

async function editAtlasChecks(change: (checks: SkyAtlasChecks) => void) {
  const character = activeSkyCharacter.value;
  if (!character || !atlasCanEdit.value) return;
  const { id, name } = character;
  const checks = loadSkyAtlasChecks(id);
  change(checks);
  isSavingAtlasChecks.value = true;
  try {
    await saveSkyAtlasChecks(checks, id);
    if (atlasCharacterId.value === id) {
      Object.assign(atlasChecks, checks);
      clearDashenSummary();
    }
    return name;
  } catch (e) {
    showToast('探索进度保存失败，原记录已保留', e instanceof Error ? e.message : String(e));
  } finally {
    isSavingAtlasChecks.value = false;
  }
}

export async function claimAtlasLegacy() {
  const character = activeSkyCharacter.value;
  if (!character || !atlasCanEdit.value || isSyncingAtlasDashen.value || !atlasLegacyCount.value) return;
  const { id, name } = character;
  isClaimingAtlasLegacy.value = true;
  try {
    await claimSkyAtlasChecks(id);
    legacyChecks.value = loadSkyAtlasChecks('');
    if (atlasCharacterId.value === id) {
      Object.assign(atlasChecks, loadSkyAtlasChecks(id));
      clearDashenSummary();
    }
    showToast('旧探索进度已归入角色', `${name} · 已合并，保留该角色已有勾选`);
  } catch (e) {
    showToast('归入失败，旧记录仍保留', e instanceof Error ? e.message : String(e));
  } finally {
    isClaimingAtlasLegacy.value = false;
  }
}

export function isAtlasShrineChecked(id: string) {
  return !!atlasChecks.shrines[id];
}

export function isAtlasLightChecked(id: string) {
  return !!atlasChecks.lights[id];
}

export function isAtlasActionChecked(id: string) {
  return !!atlasChecks.actions[id];
}

function toggleAtlasCheck(kind: keyof SkyAtlasChecks, id: string) {
  return editAtlasChecks(checks => {
    if (checks[kind][id]) delete checks[kind][id];
    else checks[kind][id] = true;
  });
}

export function toggleAtlasShrine(id: string) {
  return toggleAtlasCheck('shrines', id);
}

export function toggleAtlasLight(id: string) {
  return toggleAtlasCheck('lights', id);
}

export function toggleAtlasAction(id: string) {
  return toggleAtlasCheck('actions', id);
}

/**
 * 献祭重生只清空大世界收集的小金人，绝对不清空神殿与动作！
 */
export async function resetAtlasLightsOnly() {
  const name = await editAtlasChecks(checks => { checks.lights = {}; });
  if (name) showToast('献祭重生 · 小金人已重置', `${name} · 神殿与动作记录保留`);
}

export async function syncAtlasFromDashen() {
  const character = activeSkyCharacter.value;
  if (!character || !atlasCanEdit.value || isSyncingAtlasDashen.value) return;
  const { id: characterId, name } = character;
  const queryId = atlasQueryId.value;
  if (!queryId) {
    showToast('先在账号金库填查询ID', '光遇角色卡里填：设置 → 精灵 → 工具指南 → 查询ID');
    return;
  }
  pendingSync.value = { characterId, name };
  try {
    const snap = await fetchSkyRoleWings(queryId);
    // 请求归属在发起时固定；角色删除或查询 ID 改变后，旧结果不得写回。
    if (!skyCharacters.value.some(ch => ch.id === characterId && ch.query_id?.trim() === queryId)) {
      showToast('同步结果未写入', `${name} 的角色或查询ID已变更，请重新同步`);
      return;
    }
    const isCurrent = atlasCharacterId.value === characterId;
    const buffs = snap.buffs || [];
    const missingMaps = snap.maps.filter(m => m.collected < m.total);
    const mapHint = missingMaps.length
      ? missingMaps.map(m => `${m.name}缺 ${m.total - m.collected}`).join(' · ')
      : '各大图小金人已齐';
    if (!buffs.length) {
      if (isCurrent) {
        atlasDashenMaps.value = snap.maps;
        atlasDashenSummary.value = `${snap.collected}/${snap.total} · ${mapHint}`;
      }
      showToast(`${name} · 大神 ${snap.collected}/${snap.total}`, '接口没有逐颗列表，本地勾选没改');
      return;
    }
    const checks = loadSkyAtlasChecks(characterId);
    const applied = applyDashenLightBuffs(buffs, checks.lights);
    checks.lights = applied.lights;
    await saveSkyAtlasChecks(checks, characterId);
    if (atlasCharacterId.value === characterId && atlasQueryId.value === queryId) {
      Object.assign(atlasChecks, checks);
      atlasDashenMaps.value = snap.maps;
      atlasDashenSummary.value = `${applied.collected}/${ATLAS_LIGHT_COUNT} · ${mapHint}`;
    }
    showToast(`${name} · 已勾选 ${applied.collected}/${ATLAS_LIGHT_COUNT}`,
      applied.unreported ? `${mapHint}；${applied.unreported} 颗未返回可匹配状态，保留原勾选` : mapHint);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (atlasCharacterId.value === characterId) clearDashenSummary();
    showToast(`${name} · 大神同步失败`, msg);
  } finally {
    pendingSync.value = null;
  }
}

export function atlasDashenForRealm(realmName: string): SkyRealmWings | undefined {
  return atlasDashenMaps.value.find(m => m.name === realmName);
}

export function atlasDashenHint(realmName: string): string {
  const m = atlasDashenForRealm(realmName);
  if (!m) return '';
  if (m.collected < m.total) return `大神 ${m.collected}/${m.total} · 缺 ${m.total - m.collected}`;
  return `大神 ${m.collected}/${m.total}`;
}

export const atlasProgress = computed(() => {
  const shrineDone = Object.keys(atlasChecks.shrines).length;
  const lightDone = Object.keys(atlasChecks.lights).length;
  const actionDone = Object.keys(atlasChecks.actions).length;
  return {
    shrines: shrineDone,
    shrineTotal: ATLAS_SHRINE_COUNT,
    lights: lightDone,
    lightTotal: ATLAS_LIGHT_COUNT,
    actions: actionDone,
    actionTotal: ATLAS_ACTION_COUNT,
    areas: ATLAS_AREAS.length,
  };
});

export interface AtlasAreaCard {
  id: string;
  name: string;
  realmId: string;
  realmName: string;
  shrines: { id: string; hint: string; image: string; checked: boolean }[];
  lights: { id: string; order: number; hint: string; checked: boolean }[];
  actions: (AtlasAction & { checked: boolean })[];
  shrineDone: number;
  shrineTotal: number;
  lightDone: number;
  lightTotal: number;
  actionDone: number;
  actionTotal: number;
  /** 流程卡里栏的顺序。小金人全拿完时，动作在小金人上面。 */
  sections: ('shrines' | 'lights' | 'actions')[];
  /** 这张区域卡上的神殿、小金人、动作都已打卡。流程里整卡沉底。 */
  checked: boolean;
}

export const atlasAreaCards = computed<AtlasAreaCard[]>(() => {
  const q = atlasQuery.value.trim().toLowerCase();
  const hide = atlasHideCollected.value;
  const realmId = atlasRealmFilter.value;
  const mode = atlasMode.value;
  const actionKind = atlasActionKind.value;
  const cards: AtlasAreaCard[] = [];

  for (const realm of ATLAS_REALMS) {
    if (realmId !== 'all' && realm.id !== realmId) continue;
    const realmName = realm.nameZh;
    for (const areaId of realm.areas) {
      const area = atlasAreaById.get(areaId);
      if (!area) continue;
      const areaName = area.nameZh;
      const shrines = (area.shrines || []).flatMap(id => {
        const shrine = atlasShrineById.get(id);
        if (!shrine) return [];
        return [{
          id,
          hint: atlasShrineHint(shrine),
          image: shrine.image,
          checked: isAtlasShrineChecked(id),
        }];
      });
      const lights = (area.lights || []).flatMap(id => {
        const light = atlasLightById.get(id);
        if (!light) return [];
        return [{
          id,
          order: light.order,
          hint: atlasLightHint(light),
          checked: isAtlasLightChecked(id),
        }];
      });
      const actions = (area.actions || []).flatMap(actId => {
        const act = atlasActionById.get(actId);
        if (!act) return [];
        return [{
          ...act,
          checked: isAtlasActionChecked(actId),
        }];
      });

      const shrineDone = shrines.filter(s => s.checked).length;
      const lightDone = lights.filter(l => l.checked).length;
      const actionDone = actions.filter(a => a.checked).length;
      const matches = (text: string) => !q || `${realmName} ${areaName} ${text}`.toLowerCase().includes(q);
      const pendingShrines = mode === 'flow' || mode === 'shrines'
        ? sinkAtlasChecked(shrines.filter(s => (!hide || !s.checked) && matches(s.hint))) : [];
      const pendingLights = mode === 'flow' || mode === 'lights'
        ? sinkAtlasChecked(lights.filter(l => (!hide || !l.checked) && matches(l.hint))) : [];
      const pendingActions = mode === 'flow' || mode === 'actions'
        ? sinkAtlasChecked(actions.filter(a => (!hide || !a.checked)
          && (mode !== 'actions' || actionKind === 'all' || (actionKind === 'regular' ? a.season === '常驻' : a.season !== '常驻'))
          && matches(`${a.name} ${a.spiritName} ${a.season} ${a.hint} ${a.type}`))) : [];

      if (!pendingShrines.length && !pendingLights.length && !pendingActions.length) continue;

      const lightsDone = lights.length > 0 && lightDone === lights.length;
      const sections: AtlasAreaCard['sections'] = lightsDone
        ? ['shrines', 'actions', 'lights']
        : ['shrines', 'lights', 'actions'];
      const checked = shrineDone === shrines.length
        && lightDone === lights.length
        && actionDone === actions.length;
      cards.push({
        id: area.id,
        name: areaName,
        realmId: realm.id,
        realmName,
        shrines: pendingShrines,
        lights: pendingLights,
        actions: pendingActions,
        shrineDone,
        shrineTotal: shrines.length,
        lightDone,
        lightTotal: lights.length,
        actionDone,
        actionTotal: actions.length,
        sections,
        checked,
      });
    }
  }
  return cards;
});

export const atlasRealmGroups = computed(() => {
  const groups: { id: string; name: string; cards: AtlasAreaCard[] }[] = [];
  for (const card of atlasAreaCards.value) {
    const last = groups[groups.length - 1];
    if (last && last.id === card.realmId) last.cards.push(card);
    else groups.push({ id: card.realmId, name: card.realmName, cards: [card] });
  }
  return groups.map(group => ({ ...group, cards: sinkAtlasChecked(group.cards) }));
});

function sinkAtlasChecked<T extends { checked: boolean }>(list: T[]): T[] {
  return sinkChecked(list, item => item.checked);
}

export function atlasActionTitle(action: Pick<AtlasAction, 'name' | 'spiritName' | 'type'>): string {
  if (action.type === '先祖' || action.name === action.spiritName) return action.spiritName;
  return action.name;
}

export const currentRealmOverviewMaps = computed(() => {
  const rid = atlasRealmFilter.value;
  if (rid === 'all') {
    return [
      { name: '总览', url: '/sky/maps/world_overview.png' },
      { name: '星图', url: '/sky/maps/in_game_constellation.png' },
    ];
  }
  const realm = atlasRealmById.get(rid);
  if (!realm || !realm.overviewMaps?.length) return [];
  return realm.overviewMaps.map((url, idx) => ({
    name: realm.overviewMaps.length > 1 ? `地图 ${idx + 1}` : '地图',
    url,
  }));
});

export function openAtlasAction(action: { id: string; constellationId?: string; name: string }) {
  if (action.constellationId) {
    const item = state.collectionItems.find(s => s.id === action.constellationId);
    if (item) collectionMapFilter.value = item.map as typeof collectionMapFilter.value;
    setSkySubTab('spirits');
  } else {
    toggleAtlasAction(action.id);
  }
}
