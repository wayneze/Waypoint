import { invoke } from '@tauri-apps/api/core';
import type { GameScalingProfile } from '../types';
import { installationKey } from './localGames';
export interface GameScalingStatus { record_id: string | null; directory: string | null; phase: 'idle' | 'waiting' | 'starting' | 'scaling' | 'paused' | 'error'; message: string }
export const scalingPhaseLabel = { idle: '未开启', waiting: '等待游戏窗口', starting: '正在应用', scaling: '缩放中', paused: '切出暂停 · 返回游戏自动恢复', error: '未能应用' };
export function scalingProfile(profiles: GameScalingProfile[] | undefined, directory: string): GameScalingProfile {
  return profiles?.find(p => installationKey(p.directory) === installationKey(directory)) || {
    directory, preset: 'fit', capture: 'auto', mode: 'fullscreen', auto_start: false,
  };
}
export const getGameScaling = () => invoke<GameScalingStatus>('get_game_scaling');
export const startGameScaling = (recordId: string, installationIndex: number) => invoke<void>('start_game_scaling', { recordId, installationIndex });
export const stopGameScaling = () => invoke<void>('stop_game_scaling');
