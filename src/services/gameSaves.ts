import { invoke } from '@tauri-apps/api/core';
import type { GameSaveDiscovery } from '../types';

export interface SaveVersion { id: string; created_at: number; bytes: number; remote: boolean }
export const discoverGameSaves = (recordId: string, automaticOnly = false) =>
  invoke<GameSaveDiscovery>('discover_game_saves', { recordId, automaticOnly });
export const saveStatus = (status: string) => ({
  ready: '可备份', missing: '尚未生成', empty: '未发现进度', unavailable: '无法读取',
}[status] || '待确认');
export const listSaveBackups = (recordId: string, remote: boolean) =>
  invoke<SaveVersion[]>('list_game_save_backups', { recordId, remote });
export const createSaveBackup = (recordId: string, password: string) =>
  invoke<SaveVersion>('create_game_save_backup', { recordId, password });
export const uploadSaveBackup = (recordId: string, id: string) =>
  invoke<void>('upload_game_save_backup', { recordId, id });
