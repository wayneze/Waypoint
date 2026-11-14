import { ref } from 'vue';
import gamesCatalog from '../content/games.json';
import { normalizeDailyRule, normalizeWeeklyRule, parseResetClock } from './cycleReset';
import { kvGet, kvGetJson, kvSet } from './kv';

export type GameStamp = 'jx3' | 'sky' | 'nsh' | 'gal';
export type GameKind = 'generic' | 'catalog' | 'reader';

export interface GameMeta {
  id: string;
  label: string;
  stamp: GameStamp;
  kind: GameKind;
  dailyReset: string;
  weeklyReset: string;
  navLabel?: string;
  pageTitle?: string;
}

export interface GameResetOverride {
  dailyReset: string;
  weeklyReset: string;
}

const FALLBACK: Omit<GameMeta, 'id' | 'label'> = {
  stamp: 'gal',
  kind: 'generic',
  dailyReset: '00:00',
  weeklyReset: '周一 00:00'
};

export const OTHER_RESET_ID = 'other';

export const GAMES: GameMeta[] = gamesCatalog as GameMeta[];

const GAME_BY_ID = new Map(GAMES.map(game => [game.id, game]));
const RESET_OVERRIDE_KEY = 'waypoint_game_reset_v1';

function loadResetOverrides(): Record<string, GameResetOverride> {
  try {
    const raw = kvGet(RESET_OVERRIDE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, Partial<GameResetOverride>>;
    if (!parsed || typeof parsed !== 'object') return {};
    const out: Record<string, GameResetOverride> = {};
    for (const [id, row] of Object.entries(parsed)) {
      if (!row || typeof row !== 'object') continue;
      const daily = typeof row.dailyReset === 'string' ? normalizeDailyRule(row.dailyReset, '00:00') : '';
      const weekly = typeof row.weeklyReset === 'string' ? row.weeklyReset.trim() : '';
      if (!daily && !weekly) continue;
      out[id] = {
        dailyReset: daily || '00:00',
        weeklyReset: weekly ? normalizeWeeklyFromRule(weekly, daily || '00:00') : `周一 ${daily || '00:00'}`
      };
    }
    return out;
  } catch {
    return {};
  }
}

function normalizeWeeklyFromRule(rule: string, dailyFallback: string): string {
  const c = parseResetClock(rule, dailyFallback);
  return normalizeWeeklyRule(c.weekday ?? 1, `${String(c.hour).padStart(2, '0')}:${String(c.minute).padStart(2, '0')}`, rule);
}

let resetOverrides: Record<string, GameResetOverride> = loadResetOverrides();

function persistResetOverrides() {
  kvSet(RESET_OVERRIDE_KEY, JSON.stringify(resetOverrides));
}

export function hydrateGameResetOverrides() {
  resetOverrides = loadResetOverrides();
}

function catalogMeta(gameId: string): GameMeta {
  return GAME_BY_ID.get(gameId) || { id: gameId, label: gameId === OTHER_RESET_ID ? '其他' : gameId, ...FALLBACK };
}

export function gameMetaOf(gameId: string): GameMeta {
  const known = GAME_BY_ID.get(gameId);
  const base = catalogMeta(gameId);
  const override = resetOverrides[gameId] || (!known && gameId !== OTHER_RESET_ID ? resetOverrides[OTHER_RESET_ID] : undefined);
  if (!override) return base;
  return {
    ...base,
    dailyReset: override.dailyReset || base.dailyReset,
    weeklyReset: override.weeklyReset || base.weeklyReset
  };
}

export function gameResetIsCustom(gameId: string): boolean {
  return !!resetOverrides[gameId];
}

export function configurableResetGames(): GameMeta[] {
  return GAMES.filter(g => g.kind !== 'reader').map(g => gameMetaOf(g.id));
}

export function applyGameResetOverride(gameId: string, dailyReset: string, weeklyReset: string) {
  const base = catalogMeta(gameId);
  resetOverrides[gameId] = {
    dailyReset: normalizeDailyRule(dailyReset, base.dailyReset),
    weeklyReset: normalizeWeeklyFromRule(weeklyReset, dailyReset || base.dailyReset)
  };
  persistResetOverrides();
}

export function clearGameResetOverride(gameId: string) {
  delete resetOverrides[gameId];
  persistResetOverrides();
}

/** 网游账号才挂角色进日常板；Steam / Galgame 只有登录号。 */
export function gameHasRoster(gameId: string): boolean {
  const meta = gameMetaOf(gameId);
  if (meta.kind === 'reader') return false;
  const id = `${meta.id} ${meta.label}`.toLowerCase();
  if (id.includes('steam') || id.includes('galgame') || id.includes('视觉小说')) return false;
  return true;
}

export const CATALOG_NAV_GAMES = GAMES.filter(game => game.navLabel && game.kind === 'catalog');
export const READER_NAV_GAMES = GAMES.filter(game => game.navLabel && game.kind === 'reader');

/** 设置页可开关的游戏。没有独立页面的目录项不进这张表。 */
export const TOGGLEABLE_GAMES = GAMES.filter(game => !!game.navLabel);

const ENABLED_GAMES_KEY = 'enabled_games';
const DEFAULT_ENABLED_GAMES = TOGGLEABLE_GAMES.map(game => game.id);

function readEnabledGames(): string[] {
  const raw = kvGetJson<unknown>(ENABLED_GAMES_KEY, null);
  if (!Array.isArray(raw)) return [...DEFAULT_ENABLED_GAMES];
  const allowed = new Set(TOGGLEABLE_GAMES.map(game => game.id));
  const picked = raw.filter((id): id is string => typeof id === 'string' && allowed.has(id));
  return [...new Set(picked)];
}

export const enabledGameIds = ref<string[]>(readEnabledGames());

export function hydrateEnabledGames() {
  enabledGameIds.value = readEnabledGames();
}

export function isGameEnabled(gameId: string): boolean {
  enabledGameIds.value;
  return enabledGameIds.value.includes(gameId);
}

export function setGameEnabled(gameId: string, on: boolean) {
  if (!TOGGLEABLE_GAMES.some(game => game.id === gameId)) return;
  const next = new Set(enabledGameIds.value);
  if (on) next.add(gameId);
  else next.delete(gameId);
  const ordered = TOGGLEABLE_GAMES.map(game => game.id).filter(id => next.has(id));
  enabledGameIds.value = ordered;
  kvSet(ENABLED_GAMES_KEY, JSON.stringify(ordered));
}

const LEGACY_TABS: Record<string, string> = {
  collection: 'sky'
};

export function normalizeTab(tab: string): string {
  return LEGACY_TABS[tab] || tab;
}

/** 金库平台文案落到游戏目录 ID。未知平台原样返回。 */
export function resolveVaultGameId(platform: string): string {
  const raw = (platform || '').trim();
  if (!raw) return OTHER_RESET_ID;
  const p = raw.toLowerCase();
  for (const game of GAMES) {
    if (p === game.id || p.includes(game.id) || p.includes(game.label.toLowerCase())) {
      return game.id;
    }
  }
  if (/剑/.test(raw)) return 'jx3';
  if (/光遇/.test(raw)) return 'sky';
  if (/逆水寒/.test(raw)) return 'nsh';
  if (/galgame|视觉小说/.test(p)) return 'galgame';
  if (/steam/.test(p)) return 'steam';
  return raw;
}
