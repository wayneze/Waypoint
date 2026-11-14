import type { Jx3DungeonPendant, Jx3SetTokenSeries, Jx3WorldBossSchedule } from '../types';
import pendantsCatalog from '../content/jx3/pendants.json';
import setTokensCatalog from '../content/jx3/set-tokens.json';
import worldBossCatalog from '../content/jx3/world-boss.json';
import { progressMap } from './progress';

export function cloneCatalog<T>(data: T): T {
  return JSON.parse(JSON.stringify(data)) as T;
}

export const JX3_PENDANTS_CATALOG: Jx3DungeonPendant[] = pendantsCatalog as Jx3DungeonPendant[];

export const JX3_SET_TOKENS_CATALOG: Jx3SetTokenSeries[] = setTokensCatalog as Jx3SetTokenSeries[];

export const JX3_WORLD_BOSS_SCHEDULE: Jx3WorldBossSchedule = worldBossCatalog as Jx3WorldBossSchedule;

export function compactQuery(q: string): string {
  return q.toLowerCase().replace(/[\s·・\-_/]/g, '');
}

export function scoreText(query: string, compact: string, fields: string[]): number {
  let best = 0;
  for (const field of fields) {
    const raw = (field || '').trim();
    if (!raw) continue;
    const low = raw.toLowerCase();
    const packed = compactQuery(raw);
    if (low === query || packed === compact) best = Math.max(best, 100);
    else if (low.startsWith(query) || packed.startsWith(compact)) best = Math.max(best, 80);
    else if (low.includes(query) || packed.includes(compact)) best = Math.max(best, 60);
  }
  return best;
}

const PENDANTS_SCOPE = 'jx3.pendants';
const TOKENS_SCOPE = 'jx3.tokens';

export function loadJx3Pendants(): Jx3DungeonPendant[] {
  const catalog = cloneCatalog(JX3_PENDANTS_CATALOG).map(p => ({
    ...p,
    obtained: false,
    cdCleared: false
  }));
  const stored = progressMap<{ obtained?: boolean; cdCleared?: boolean }>(PENDANTS_SCOPE);
  return catalog.map(p => {
    const match = stored[p.id];
    return match ? { ...p, ...match } : p;
  });
}

export function loadJx3SetTokens(): Jx3SetTokenSeries[] {
  const catalog = cloneCatalog(JX3_SET_TOKENS_CATALOG).map(series => {
    const pieces = { ...series.pieces };
    (Object.keys(pieces) as Array<keyof typeof pieces>).forEach(k => {
      pieces[k] = { ...pieces[k], obtained: false };
    });
    return { ...series, pieces };
  });
  const stored = progressMap<Record<string, boolean>>(TOKENS_SCOPE);
  return catalog.map(series => {
    const seriesSaved = stored[series.id];
    if (!seriesSaved) return series;
    const newPieces = { ...series.pieces };
    (Object.keys(newPieces) as Array<keyof typeof newPieces>).forEach(k => {
      if (seriesSaved[k] !== undefined) {
        newPieces[k] = { ...newPieces[k], obtained: seriesSaved[k] };
      }
    });
    return { ...series, pieces: newPieces };
  });
}

const HERO_PREFIX = /^(英雄|普通)/;

export function normalizeJx3DungeonLabel(raw: string): string {
  return (raw || '')
    .replace(/[·・]/g, '·')
    .replace(/\s+/g, '')
    .replace(HERO_PREFIX, '')
    .trim();
}
