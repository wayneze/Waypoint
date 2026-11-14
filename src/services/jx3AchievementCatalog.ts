import catalog from '../content/jx3/achievements.json';
import type { Jx3BoxAchievementRow } from './jx3ProgressLedger';
import type { Jx3AdventureNameMaps } from './jx3AdventureCatalog';
import { parseCompleteAdventureHits } from './jx3AdventureCatalog';

export interface Jx3AchievementRecord extends Jx3BoxAchievementRow {
  ID: number;
  Name: string;
  Sub?: number;
  Detail?: number | string;
  SceneName?: string;
  dwMapID?: number;
}

export const jx3AchievementRecords: Jx3AchievementRecord[] = catalog.achievements;
export const jx3AchievementById = new Map(jx3AchievementRecords.map(row => [String(row.ID), row]));
export const jx3AchievementMenus = catalog.menus;

function idsIn(value: unknown): string[] {
  if (Array.isArray(value)) return value.flatMap(idsIn);
  return typeof value === 'number' || (typeof value === 'string' && /^\d+$/.test(value)) ? [String(value)] : [];
}

export const jx3AchievementGroups = Object.entries(catalog.menus).flatMap(([sub, menu]) =>
  menu.children.map(child => ({
    id: String(child.id),
    name: child.name,
    category: menu.name,
    sub: Number(sub),
    detail: String(child.detail),
    ids: [...new Set(idsIn(child.achievements))],
  })).filter(group => group.ids.length)
);

export function buildAdventureNameMaps(prefixes: string[]): Jx3AdventureNameMaps {
  const achieveNameIds: Record<string, string> = {};
  const seriesIds: Record<string, string[]> = {};
  for (const row of jx3AchievementRecords) {
    achieveNameIds[row.Name] = String(row.ID);
  }
  for (const prefix of prefixes)
    seriesIds[prefix] = jx3AchievementRecords.filter(row => row.Name.startsWith(prefix)).map(row => String(row.ID));
  return { achieveNameIds, seriesIds, completeAchieveIds: parseCompleteAdventureHits(jx3AchievementRecords) };
}

export function knownAchievementPoints(completed: ReadonlySet<string>): number {
  return jx3AchievementRecords.reduce((sum, row) => sum + (completed.has(String(row.ID)) ? row.Point || 0 : 0), 0);
}

export function achievementsAtPlaces(places: string[], kind?: 'dungeon' | 'map'): Jx3AchievementRecord[] {
  const names = places.map(name => name.replace(/\s/g, '').replace(/^(英雄|普通)/, ''));
  const matches = (value: string) => names.some(name => name && (
    value === name || value.replace(/^(英雄|普通)/, '') === name
    || value.endsWith(`·${name}`) || name.endsWith(`·${value}`)
  ));
  const detailIds = new Set(jx3AchievementGroups
    .filter(group => group.sub === 7 && matches(group.name.replace(/任务$/, '')))
    .map(group => group.detail));
  return jx3AchievementRecords.filter(row => {
    if (kind === 'dungeon' && row.Sub !== 11) return false;
    if (kind === 'map' && row.Sub !== 7 && row.Sub !== 8) return false;
    return (row.Sub === 7 && detailIds.has(String(row.Detail)))
      || (!!row.SceneName && matches(row.SceneName));
  });
}
