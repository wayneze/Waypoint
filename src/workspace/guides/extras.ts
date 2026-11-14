import { ref } from 'vue';
import { kvGetJson, kvSetConfirmed } from '../../services/kv';
import { isHttp, type ExploreRole } from '../../services/exploreSites';
import type { ResourceArticle } from '../../services/systemApi';
import { isTauri } from '../../services/api';
import { listen } from '@tauri-apps/api/event';

export interface GameLink {
  title: string; url: string; site: string; role: ExploreRole;
  article?: ResourceArticle;
  markdown_path?: string;
  saved_at?: string;
  version?: string;
  notes?: string;
  local_files?: string[];
}
const LINKS_KEY = 'waypoint_game_links';
const RECENT_KEY = 'waypoint_game_recent';
export const gameLinks = ref<Record<string, GameLink[]>>({});
export const recentGames = ref<Record<string, number>>({});
let linkWrite: Promise<void> = Promise.resolve();
let recentWrite: Promise<void> = Promise.resolve();
let listening = false;
export const resourceDownloadError = ref('');
const removingIds = new Set<string>();

export async function prepareGameRemoval(ids: string[]) {
  ids.forEach(id => removingIds.add(id));
  const release = () => ids.forEach(id => removingIds.delete(id));
  try { await Promise.all([linkWrite, recentWrite]); return release; }
  catch (error) { release(); throw error; }
}

export function loadGameExtras() {
  gameLinks.value = kvGetJson(LINKS_KEY, {});
  recentGames.value = kvGetJson(RECENT_KEY, {});
  if (isTauri() && !listening) {
    listening = true;
    void listen<{ gameId: string; sourceUrl: string; title: string; path?: string; success: boolean }>('resource-download', async ({ payload }) => {
      if (!payload.gameId || removingIds.has(payload.gameId) || kvGetJson<string[]>('waypoint_removed_guides', []).includes(payload.gameId)) return;
      if (!payload.success || !payload.path) { resourceDownloadError.value = '资料下载未完成，请在原网页重试。'; return; }
      try {
        await changeLinks([payload.gameId], links => {
          const existing = links.find(link => link.url === payload.sourceUrl);
          const item: GameLink = { ...existing, title: existing?.title || payload.title, url: payload.sourceUrl, site: existing?.site || new URL(payload.sourceUrl).hostname, role: existing?.role || 'resource', local_files: [...new Set([...(existing?.local_files || []), payload.path!])] };
          return existing ? links.map(link => link.url === item.url ? item : link) : [...links, item];
        });
      } catch (error) { resourceDownloadError.value = `文件已下载，但关联未保存：${String(error)}`; }
    }).catch(error => { listening = false; resourceDownloadError.value = String(error); });
  }
}

export function rememberGame(id: string) {
  const openedAt = Date.now();
  const job = recentWrite.catch(() => {}).then(async () => {
    if (removingIds.has(id)) return;
    const next = { ...recentGames.value, [id]: openedAt };
    await kvSetConfirmed(RECENT_KEY, JSON.stringify(next));
    recentGames.value = next;
  });
  recentWrite = job;
  return job;
}

export function linksForGames(ids: string[]): GameLink[] {
  const unique = new Map<string, GameLink>();
  for (const id of ids) for (const link of gameLinks.value[id] || []) {
    if (!unique.has(link.url)) unique.set(link.url, link);
  }
  return [...unique.values()];
}

function changeLinks(ids: string[], change: (links: GameLink[]) => GameLink[]) {
  const job = linkWrite.catch(() => {}).then(async () => {
    if (ids.some(id => removingIds.has(id))) return;
    const next = { ...gameLinks.value };
    for (const id of ids) next[id] = change(next[id] || []);
    await kvSetConfirmed(LINKS_KEY, JSON.stringify(next));
    gameLinks.value = next;
  });
  linkWrite = job;
  return job;
}

export function saveGameLink(id: string, link: GameLink) {
  if (!id || !link.title.trim() || !isHttp(link.url)) return Promise.reject(new Error('请选择游戏并填写有效的资料链接'));
  const url = new URL(link.url).href;
  return changeLinks([id], links => {
    const item = { ...link, title: link.title.trim(), url };
    const index = links.findIndex(existing => existing.url === url);
    return index < 0 ? [...links, item] : links.map((existing, i) => i === index ? { ...existing, ...item } : existing);
  });
}

export function removeGameLink(id: string | string[], url: string) {
  return changeLinks(Array.isArray(id) ? id : [id], links => links.filter(link => link.url !== url));
}
