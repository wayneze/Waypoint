import { invoke } from '@tauri-apps/api/core';
import type { GameRecord } from '../types';

export interface Achievement {
  id: string; name: string; description: string; icon: string; icon_gray: string;
  hidden: boolean; unlocked: boolean; unlocked_at?: number;
}
export interface AchievementCache {
  appid: number; updated_at?: number; status: 'ready' | 'unsupported' | 'private' | 'error';
  error?: string; items: Achievement[];
}
export interface SteamAccountCache {
  account_id: string; steamid64: string; updated_at?: number; error?: string;
  achievements: Record<string, AchievementCache>;
}
export interface SteamActivity {
  links: { record_id: string; appid: number }[];
  accounts: SteamAccountCache[];
  installed: { appid: number; directory: string }[];
  titles: Record<string, SteamTitle>;
}
export interface SteamTitle { title?: string; source?: string; checked_at: number; error?: string }
export const resolveSteamTitle = (appid: number, retry = false) => invoke<SteamTitle>('resolve_steam_title', { appid, retry });
export interface PlaySession {
  started_at: number; ended_at?: number; seconds: number; recovered: boolean; end_reason?: string;
}
export interface LocalPlaytime {
  record_id: string; total_seconds: number; last_played?: number; sessions: PlaySession[];
  active?: { session: PlaySession; tracking_error?: string };
}
export const getSteamActivity = () => invoke<SteamActivity>('get_steam_activity');
export const getLocalPlaytime = () => invoke<LocalPlaytime[]>('get_local_playtime');
export const refreshSteamAchievements = (vaultItemId: string, appid: number) =>
  invoke<AchievementCache>('refresh_steam_achievements', { vaultItemId, appid });
export const importSteamGames = (vaultItemId: string, choices: { appid: number; record_id?: string; title?: string; guide_id?: string }[]) =>
  invoke<string[]>('import_steam_games', { vaultItemId, choices });
export const unlinkSteamGame = (recordId: string) => invoke<void>('unlink_steam_game', { recordId });
export function recordSteamIds(record: GameRecord): number[] {
  const metadata = record.library?.metadata;
  return [...(record.library?.installations || []).map(i => Number(i.steam_id)),
    ...(metadata ? [metadata, ...(metadata.sources || [])].filter(s => s.source === 'steam').map(s => Number(s.source_id)) : []),
  ].filter(id => Number.isInteger(id) && id > 0);
}
export function duration(seconds: number): string {
  if (seconds < 60) return `${Math.floor(seconds)} 秒`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)} 分钟`;
  return `${Math.floor(seconds / 3600)} 小时 ${Math.floor(seconds % 3600 / 60)} 分钟`;
}
export function timeLabel(time?: number): string {
  return time ? new Date(time * 1000).toLocaleString('zh-CN', { hour12: false }) : '尚未同步';
}
