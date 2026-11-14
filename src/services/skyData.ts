import spiritsCatalog from '../content/sky/spirits.json';
import { progressMap, setProgressEntries } from './progress';

export interface SkySpirit {
  id: string;
  map: '晨岛' | '云野' | '雨林' | '霞谷' | '暮土' | '禁阁';
  name: string;
  action: string;
  redCandles: number;
  wings: number;
  location: string;
  keyItems: string;
  coreItem: string;
  coreCost: string;
  unlocked: boolean;
  wingUnlocked?: boolean;
  itemObtained?: boolean;
  note?: string;
}

function cloneCatalog<T>(data: T): T {
  return JSON.parse(JSON.stringify(data)) as T;
}

export const SKY_SPIRITS_CATALOG: SkySpirit[] = spiritsCatalog as SkySpirit[];

const SPIRITS_SCOPE = 'sky.spirits';

export function loadSkySpirits(characterId = ''): SkySpirit[] {
  const catalog = cloneCatalog(SKY_SPIRITS_CATALOG).map(spirit => ({
    ...spirit,
    unlocked: false,
    wingUnlocked: false,
    itemObtained: false
  }));
  const statusMap = progressMap<{ unlocked?: boolean; wingUnlocked?: boolean; itemObtained?: boolean }>(SPIRITS_SCOPE, characterId);
  return catalog.map(spirit => {
    const stored = characterId ? statusMap[spirit.id] : undefined;
    if (stored === undefined) return spirit;
    return {
      ...spirit,
      unlocked: stored.unlocked !== undefined ? stored.unlocked : false,
      wingUnlocked: stored.wingUnlocked !== undefined ? stored.wingUnlocked : false,
      itemObtained: stored.itemObtained !== undefined ? stored.itemObtained : false
    };
  });
}

export function setSkySpiritProgress(characterId: string, id: string, patch: Partial<Pick<SkySpirit, 'unlocked' | 'wingUnlocked' | 'itemObtained'>>) {
  const old = loadSkySpirits(characterId).find(spirit => spirit.id === id);
  return setProgressEntries(SPIRITS_SCOPE, [[id, { unlocked: !!old?.unlocked, wingUnlocked: !!old?.wingUnlocked, itemObtained: !!old?.itemObtained, ...patch }]], characterId);
}
