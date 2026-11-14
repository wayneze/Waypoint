import type { GameExtraMedia, GameMetadata } from '../types';
import { invoke } from '@tauri-apps/api/core';
export const GAME_MEDIA_VERSION = 3;
export interface GameVideoSearchResult { title: string; url: string; page_url: string; thumbnail: string; author: string; duration: string }
export const searchGameVideos = (query: string) => invoke<GameVideoSearchResult[]>('search_game_videos', { query });
export function mediaProvider(raw: string) {
  try {
    const host = new URL(raw).hostname;
    if (host === 'store.steampowered.com') return 'Steam 官方商店';
    if (host === 'youtu.be' || host.endsWith('youtube.com') || host.endsWith('youtube-nocookie.com')) return 'YouTube';
    if (host === 'bilibili.com' || host.endsWith('.bilibili.com')) return 'Bilibili';
  } catch { /* Display a neutral source label for old links. */ }
  return '游戏官网';
}
export function playableMedia(kind: GameExtraMedia['kind'], url: string) {
  try {
    if (new URL(url).protocol !== 'https:') return false;
    return (kind === 'audio' ? /\.(mp3|ogg|wav|m4a|aac|flac)(?:[?#]|$)/i : /\.(mp4|m4v|webm|ogv|m3u8)(?:[?#]|$)/i).test(url);
  } catch { return false; }
}
export function atmosphereSources(metadata?: GameMetadata | null, extra: GameExtraMedia[] = []) {
  const videos: GameExtraMedia[] = (metadata?.videos || []).map(video => ({ kind: 'video', title: video.title, url: video.url }));
  return [...videos, ...extra].filter((item, index, items) => playableMedia(item.kind, item.url) && items.findIndex(other => other.url === item.url) === index);
}
