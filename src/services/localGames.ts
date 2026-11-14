import { invoke } from '@tauri-apps/api/core';
import type { GameAlbum, GameInstallation, GameMetadata, GameRecord } from '../types';

export interface ScannedGame { title: string; installation: GameInstallation; steam_id?: string }
export interface GameScan { games: ScannedGame[]; warnings: string[]; complete: boolean; visited: number }
export const installationKey = (path: string) => path.replace(/\//g, '\\').replace(/\\+$/, '').toLocaleLowerCase();
export function gameExecutableLabel(path: string) {
  const name = path.split(/[\\/]/).pop()?.replace(/\.exe$/i, '') || path;
  if (/民间步兵|民间步/.test(name)) return `民间步兵 · ${name}`;
  if (/原版汉化/.test(name)) return `原版汉化 · ${name}`;
  if (/语音版/.test(name)) return `语音版 · ${name}`;
  if (/汉化组/.test(name)) return `汉化版 · ${name}`;
  if (/_cn|chs|汉化/i.test(name)) return `汉化版 · ${name}`;
  if (/_jp/i.test(name)) return `日文版 · ${name}`;
  return name;
}
export interface GameLaunchOption { key: string; label: string; available: boolean; installationIndex?: number }
export function gameLaunchOptions(installations: GameInstallation[], hasSteam: boolean, steamInstalled: boolean, available: Record<string, boolean>, checked: boolean): GameLaunchOption[] {
  return [
    ...(hasSteam ? [{ key: 'steam', label: steamInstalled ? 'Steam 客户端 · 已安装' : 'Steam 客户端 · 安装 / 启动', available: true }] : []),
    ...installations.map((item, installationIndex) => ({
      key: `local:${installationKey(item.directory)}`,
      label: `本地 · ${item.directory.split(/[\\/]/).filter(Boolean).pop()} · ${item.executable.split(/[\\/]/).pop()}${checked && !available[item.directory] ? ' · 目录不可用' : ''}`,
      available: checked && !!available[item.directory], installationIndex,
    })),
  ];
}
export function preferredLaunchOption(options: GameLaunchOption[], preferred?: string | null): string {
  // A removed preference requires an explicit choice; never silently launch another edition.
  return preferred ? options.some(option => option.key === preferred) ? preferred : '' : options.find(option => option.available)?.key || options[0]?.key || '';
}
export function launchGameOption(recordId: string, option: GameLaunchOption) {
  if (!option.available) return Promise.reject(new Error('这个启动方式当前不可用，请选择其他安装。'));
  return option.key === 'steam' ? invoke<void>('launch_steam_game', { recordId })
    : launchLibraryGame(recordId, option.installationIndex!);
}
export const titleKey = (title: string) => title.normalize('NFKC').toLocaleLowerCase().replace(/[^\p{L}\p{N}]/gu, '');
export function installedRecord(game: ScannedGame, records: GameRecord[]) {
  return records.find(r => r.library?.installations.some(i => installationKey(i.directory) === installationKey(game.installation.directory)));
}
function titleVariants(title: string): string[] {
  const normalized = title.normalize('NFKC').replace(/\[[^\]]*]/g, '').trim();
  return [...new Set([normalized, normalized.replace(/\([^)]*\)/g, ''), ...Array.from(normalized.matchAll(/\(([^)]+)\)/g), m => m[1]!)].map(titleKey).filter(Boolean))];
}
export function rankMetadata(titles: string[], candidates: GameMetadata[]) {
  const keys = titles.flatMap(titleVariants);
  return candidates.map(metadata => {
    const names = [metadata.title, metadata.original_title, ...metadata.aliases || []].flatMap(titleVariants);
    let score = 0;
    for (const key of keys) for (const name of names) {
      if (key === name) score = Math.max(score, 1);
      // 合集父目录 + 完整作品名，或省略系列前缀的完整副标题。
      // 只接受较长的完整后缀；前缀包含不能把本篇匹配到 0、2、FD 等续作。
      else if (Math.min(key.length, name.length) >= 7 && (key.endsWith(name) || name.endsWith(key))) score = Math.max(score, .94);
    }
    return { metadata, score };
  }).sort((a, b) => b.score - a.score);
}
export function confidentMetadata(titles: string[], candidates: GameMetadata[]) {
  const ranked = rankMetadata(titles, candidates), top = ranked[0];
  return top && top.score >= .94 && (!ranked[1] || top.score - ranked[1].score >= .05) ? top.metadata : undefined;
}
export const scanGameDirectory = (root: string) => invoke<GameScan>('scan_game_directory', { root });
export const cancelGameScan = () => invoke<void>('cancel_game_scan');
export const searchGameMetadata = (source: GameMetadata['source'], query: string) =>
  invoke<GameMetadata[]>('search_game_metadata', { source, query });
export const loadGameMetadata = (metadata: Pick<GameMetadata, 'source' | 'source_id'>) =>
  invoke<GameMetadata>('load_game_metadata', { source: metadata.source, sourceId: metadata.source_id });
export const loadGameAlbum = (subjectId: string) => invoke<GameAlbum>('load_game_album', { subjectId });
export const bangumiConnected = () => invoke<boolean>('bangumi_connected');
export const connectBangumi = (token: string) => invoke<void>('connect_bangumi', { token });
export const installationAvailable = (directory: string) => invoke<boolean>('game_installation_available', { directory });
export const launchLibraryGame = (recordId: string, installationIndex: number) =>
  invoke<void>('launch_library_game', { recordId, installationIndex });
