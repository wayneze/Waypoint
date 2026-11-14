import atlasCatalog from '../content/sky/atlas.json';
import areaImages from '../content/sky/area-images.json';
import wingKeys from '../content/sky/wing-keys.json';
import { claimGlobalProgress, progressMap, replaceProgressScopes } from './progress';

export interface AtlasShrine {
  id: string;
  areaId: string;
  realmId: string;
  areaName: string;
  realmName: string;
  hint: string;
  image: string;
  position?: [number, number];
}

export interface AtlasLight {
  id: string;
  order: number;
  areaId: string;
  realmId: string;
  areaName: string;
  realmName: string;
  hint: string;
  position?: [number, number];
}

export interface AtlasAction {
  id: string;
  name: string;
  type: string;
  icon: string;
  image?: string;
  spiritName: string;
  season: string;
  areaId: string;
  realmId: string;
  realmName: string;
  areaName: string;
  hint: string;
  constellationId?: string;
  locationImage?: { preview: string; full: string };
  sourceUrl?: string;
}

export interface AtlasArea {
  id: string;
  name: string;
  nameZh: string;
  realmId: string;
  realmNameZh: string;
  shrines: string[];
  lights: string[];
  actions: string[];
}

export interface AtlasRealm {
  id: string;
  name: string;
  nameZh: string;
  overviewMaps: string[];
  areas: string[];
}

interface AtlasFile {
  realms: AtlasRealm[];
  areas: AtlasArea[];
  shrines: AtlasShrine[];
  lights: AtlasLight[];
  actions: AtlasAction[];
}

const catalog = atlasCatalog as unknown as AtlasFile;

export const ATLAS_REALMS: AtlasRealm[] = catalog.realms || [];
export const ATLAS_AREAS: AtlasArea[] = catalog.areas || [];
export const ATLAS_SHRINES: AtlasShrine[] = catalog.shrines || [];
export const ATLAS_LIGHTS: AtlasLight[] = catalog.lights || [];
export const ATLAS_ACTIONS: AtlasAction[] = catalog.actions || [];

export const atlasShrineById = new Map(ATLAS_SHRINES.map(s => [s.id, s]));
export const atlasLightById = new Map(ATLAS_LIGHTS.map(l => [l.id, l]));
export const atlasAreaById = new Map(ATLAS_AREAS.map(a => [a.id, a]));
export const atlasActionById = new Map(ATLAS_ACTIONS.map(a => [a.id, a]));
export const atlasRealmById = new Map(ATLAS_REALMS.map(r => [r.id, r]));

const areaImageById = new Map(
  (areaImages as { id: string; image: string }[]).map(row => [row.id, row.image]),
);

export function atlasAreaImage(areaId: string): string {
  return areaImageById.get(areaId) || '';
}

export const ATLAS_LIGHT_COUNT = ATLAS_LIGHTS.length;
export const ATLAS_SHRINE_COUNT = ATLAS_SHRINES.length;
export const ATLAS_ACTION_COUNT = ATLAS_ACTIONS.length;

export function atlasRealmName(id: string): string {
  const r = atlasRealmById.get(id);
  return r ? r.nameZh : id;
}

export function atlasShrineHint(shrine: AtlasShrine): string {
  return shrine.hint || `${shrine.areaName || ''} 地图石碑`;
}

export function atlasLightHint(light: AtlasLight): string {
  return light.hint || `小金人 #${light.order}`;
}

export const ATLAS_REALM_FILTERS = [
  { id: 'all', name: '全图' },
  ...ATLAS_REALMS.map(r => ({ id: r.id, name: r.nameZh })),
] as const;

const ATLAS_KINDS = ['shrines', 'lights', 'actions'] as const;
const ATLAS_SCOPE = (kind: typeof ATLAS_KINDS[number]) => `sky.atlas.${kind}`;

export interface SkyAtlasChecks {
  shrines: Record<string, true>;
  lights: Record<string, true>;
  actions: Record<string, true>;
}

export function loadSkyAtlasChecks(characterId: string): SkyAtlasChecks {
  return {
    shrines: progressMap<true>(ATLAS_SCOPE('shrines'), characterId),
    lights: progressMap<true>(ATLAS_SCOPE('lights'), characterId),
    actions: progressMap<true>(ATLAS_SCOPE('actions'), characterId),
  };
}

export function saveSkyAtlasChecks(checks: SkyAtlasChecks, characterId: string): Promise<void> {
  if (!characterId) throw new Error('请先选择光遇角色');
  return replaceProgressScopes(Object.fromEntries(ATLAS_KINDS.map(kind =>
    [ATLAS_SCOPE(kind), Object.entries(checks[kind])])), characterId);
}

export function claimSkyAtlasChecks(characterId: string): Promise<void> {
  return claimGlobalProgress(ATLAS_KINDS.map(ATLAS_SCOPE), characterId);
}

const WING_KEY_TO_LIGHT = wingKeys as Record<string, string>;

export function applyDashenLightBuffs(
  buffs: Array<{ name: string; collected: boolean }>,
  existing: Record<string, true>,
): { lights: Record<string, true>; collected: number; missing: number; unreported: number } {
  const lights: Record<string, true> = { ...existing };
  const reported = new Set<string>();
  for (const buff of buffs) {
    const id = WING_KEY_TO_LIGHT[buff.name];
    if (!id || !atlasLightById.has(id)) continue;
    reported.add(id);
    if (buff.collected) lights[id] = true;
    else delete lights[id];
  }
  const collected = Object.keys(lights).length;
  return {
    lights,
    collected,
    missing: Math.max(0, ATLAS_LIGHT_COUNT - collected),
    unreported: Math.max(0, ATLAS_LIGHT_COUNT - reported.size),
  };
}
