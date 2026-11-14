import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { cacheGet, cacheSet } from './fetchCache';
import type { SkyCollectionEntry } from './skyCollection';

interface MediaCache {
  version: 1;
  images: Record<string, string>;
  checked: Record<string, number>;
}
interface MediaResult { images: Record<string, string>; checked: string[]; unavailable: number }
const CACHE_KEY = 'sky.collection.media';
const media = ref<MediaCache>({ version: 1, images: {}, checked: {} });
export const collectionMediaLoading = ref(false);
export const collectionMediaError = ref('');
const queue = new Map<string, { icon: string; kind: string }>();
const attempted = new Set<string>();
let hydrated = false;
const RETRY_AFTER = 24 * 60 * 60 * 1000;

export function collectionWear(entry: SkyCollectionEntry): string {
  return entry.wear || media.value.images[entry.icon] || '';
}

/** 图鉴与星盘共享图片关联；只解析当前列表中缺实穿的物品，已匹配结果存入图片目录缓存。 */
export async function ensureCollectionMedia(entries: SkyCollectionEntry[], retry = false) {
  if (!hydrated) {
    const saved = cacheGet<MediaCache>(CACHE_KEY)?.data;
    media.value = saved?.version === 1 ? saved : { version: 1, images: {}, checked: {} };
    hydrated = true;
  }
  if (retry && collectionMediaLoading.value) return;
  for (const entry of entries) {
    if (!entry.icon || entry.wear || media.value.images[entry.icon]) continue;
    if (!retry && attempted.has(entry.icon)) continue;
    if (!retry && Date.now() - (media.value.checked[entry.icon] || 0) < RETRY_AFTER) continue;
    queue.set(entry.icon, { icon: entry.icon, kind: entry.kind });
  }
  if (collectionMediaLoading.value || !queue.size || !('__TAURI_INTERNALS__' in window)) return;
  collectionMediaLoading.value = true;
  collectionMediaError.value = '';
  try {
    while (queue.size) {
      const items = [...queue.values()];
      queue.clear();
      for (const item of items) attempted.add(item.icon);
      const result = await invoke<MediaResult>('resolve_sky_collection_media', { items, retry });
      retry = false;
      const next: MediaCache = { version: 1, images: { ...media.value.images, ...result.images }, checked: { ...media.value.checked } };
      for (const icon of result.checked) next.checked[icon] = Date.now();
      media.value = next;
      cacheSet(CACHE_KEY, next);
      if (result.unavailable) collectionMediaError.value = '部分图片来源暂时不可用，可以重试';
    }
  } catch (error) {
    collectionMediaError.value = String(error);
    queue.clear();
  } finally {
    collectionMediaLoading.value = false;
  }
}
