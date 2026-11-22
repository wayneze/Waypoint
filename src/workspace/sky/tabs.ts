// 光遇页子分组，以及各分组数据（档期、今日、季节、节日）的加载与刷新。
import { ref, watch } from 'vue';
import { type SkyDailyGuidePayload, loadTodaySkyDailyGuide, syncSkyDailyChecks } from '../../services/skyDailyService';
import { beijingDay } from '../../services/cycleReset';
import { type SkyCalendarFeed, getSkyCalendarFeed } from '../../services/skyCalendarService';
import type { SkyEventCatalog, SkySeasonCatalog } from '../../services/skyApi';
import { kvGet, kvSet } from '../../services/kv';
import { getSkySeasonCatalog, loadSeasonWishStatus, mergeSeasonCatalog } from '../../services/skySeasonService';
import { getSkyEventCatalog, loadEventWishStatus, mergeEventCatalog } from '../../services/skyEventService';
import { showToast, state } from '../core';
import { activeSkyCharacter } from '../vault';
import { progressRevision } from '../../services/progress';
import { sourceStatus } from '../../services/fetchCache';
import { syncSkyInventoryCollection } from '../../services/skyInventory';

// 收藏只有外观图鉴与先祖星盘两个入口；季节、节日是目录来源。
type SkySubTab = 'daily' | 'news' | 'atlas' | 'spirits' | 'outfits' | 'benefits';
const SKY_SUB_TABS: SkySubTab[] = ['daily', 'news', 'atlas', 'spirits', 'outfits', 'benefits'];
function resolveSkySubTab(raw: string | null): SkySubTab {
  if (raw === 'seasons' || raw === 'events') return 'outfits';
  if (raw && (SKY_SUB_TABS as string[]).includes(raw)) return raw as SkySubTab;
  return 'daily';
}
export const initialSkyTab = new URLSearchParams(window.location.search).get('skyTab');
const storedSkyTab = kvGet('waypoint_sky_subtab');
if (storedSkyTab === 'radar') {
  kvSet('waypoint_sky_subtab', 'daily');
}
export const skySubTab = ref<SkySubTab>(
  initialSkyTab ? resolveSkySubTab(initialSkyTab) : resolveSkySubTab(storedSkyTab)
);

export function setSkySubTab(tab: SkySubTab) {
  skySubTab.value = tab;
  kvSet('waypoint_sky_subtab', tab);
}

const skyRefreshJobs: Partial<Record<string, Promise<void>>> = {};

function runSkyRefresh(key: string, run: () => Promise<void>): Promise<void> {
  const existing = skyRefreshJobs[key];
  if (existing) return existing;
  const job = run().finally(() => {
    if (skyRefreshJobs[key] === job) delete skyRefreshJobs[key];
  });
  skyRefreshJobs[key] = job;
  return job;
}

export function ensureSkySubTabCatalog(tab: SkySubTab): Promise<void> | void {
  if (tab === 'daily') return refreshSkyDaily(false);
  if (tab === 'news') return refreshSkyNews(false);
  if (tab === 'outfits' || tab === 'spirits') return Promise.all([refreshSkySeasons(false), refreshSkyEvents(false)]).then(() => {});
}

export async function ensureSkyCatalogs() {
  await Promise.all([refreshSkyDaily(), refreshSkyNews(), refreshSkySeasons(), refreshSkyEvents()]);
}

// 光遇档期：当前季节 / 国服回归先祖 / 国际服已公告回归
export const skyCalendarFeed = ref<SkyCalendarFeed | null>(null);
export const isLoadingSkyNews = ref(false);

export async function refreshSkyNews(force = false) {
  return runSkyRefresh(force ? 'news:force' : 'news', async () => {
    isLoadingSkyNews.value = true;
    try {
      const feed = await getSkyCalendarFeed(force, next => {
        skyCalendarFeed.value = next;
      });
      if (feed) {
        skyCalendarFeed.value = feed;
        if (force) {
          showToast('档期已更新', `${feed.as_of} · 季节与回归先祖`);
        }
      } else if (force) {
        showToast('刷新失败', '未能拉到季节或回归先祖档期');
      }
    } catch {
      if (force) {
        showToast('刷新失败', '未能拉到季节或回归先祖档期');
      }
    } finally {
      isLoadingSkyNews.value = false;
    }
  });
}

// 今日光遇日常图文导航响应式状态
export const skyDailyData = ref<SkyDailyGuidePayload | null>(null);
export const isLoadingSkyDaily = ref(false);
let lastSkyDailyAttemptDay = '';
let lastSkyDailyAttemptAt = 0;
watch(() => activeSkyCharacter.value?.id || '', id => {
  if (skyDailyData.value) syncSkyDailyChecks(skyDailyData.value, id);
}, { flush: 'sync' });

export async function refreshSkyDaily(force = false) {
  const today = beijingDay();
  if (skyRefreshJobs.daily) return skyRefreshJobs.daily;
  if (!force && lastSkyDailyAttemptDay === today && Date.now() - lastSkyDailyAttemptAt < 5 * 60_000) return;
  return runSkyRefresh('daily', async () => {
    lastSkyDailyAttemptDay = today;
    lastSkyDailyAttemptAt = Date.now();
    isLoadingSkyDaily.value = true;
    try {
      const data = await loadTodaySkyDailyGuide(force);
      if (data.dateStr !== beijingDay()) return;
      syncSkyDailyChecks(data, activeSkyCharacter.value?.id || '');
      skyDailyData.value = data;
      if (force) {
        const status = sourceStatus['sky.daily'];
        if (status.state === 'error' || status.state === 'stale') showToast('今日日常刷新失败', status.error);
        else showToast('今日日常已刷新', status.error || data.dateStr);
      }
    } catch (e) {
      if (force) showToast('刷新失败', '无法拉取今日任务数据');
    } finally {
      isLoadingSkyDaily.value = false;
    }
  });
}

export const isLoadingSkySeasons = ref(true);
export const isLoadingSkyEvents = ref(true);

function applySeasonCatalog(catalog: SkySeasonCatalog) {
  const customs = state.seasonalGuides.filter(g => g.isCustom);
  state.seasonalGuides = mergeSeasonCatalog(catalog, loadSeasonWishStatus(activeSkyCharacter.value?.id || ""), customs);
  void syncSkyInventoryCollection(activeSkyCharacter.value?.id || '', state.seasonalGuides, 'sky.season_wish').catch(e => showToast('收藏同步失败', String(e)));
}

function applyEventCatalog(catalog: SkyEventCatalog) {
  state.eventGuides = mergeEventCatalog(catalog, loadEventWishStatus(activeSkyCharacter.value?.id || ""));
  void syncSkyInventoryCollection(activeSkyCharacter.value?.id || '', state.eventGuides, 'sky.event_wish').catch(e => showToast('收藏同步失败', String(e)));
}

export async function refreshSkySeasons(force = false) {
  return runSkyRefresh(force ? 'seasons:force' : 'seasons', async () => {
    isLoadingSkySeasons.value = true;
    try {
      const catalog = await getSkySeasonCatalog(force, applySeasonCatalog);
      if (catalog) applySeasonCatalog(catalog);
    } catch (e) {
      console.warn('refreshSkySeasons failed', e);
    } finally {
      isLoadingSkySeasons.value = false;
    }
  });
}

export async function refreshSkyEvents(force = false) {
  return runSkyRefresh(force ? 'events:force' : 'events', async () => {
    isLoadingSkyEvents.value = true;
    try {
      const catalog = await getSkyEventCatalog(force, applyEventCatalog);
      if (catalog) applyEventCatalog(catalog);
    } catch (e) {
      console.warn('refreshSkyEvents failed', e);
    } finally {
      isLoadingSkyEvents.value = false;
    }
  });
}

// 已完成的沉底，各分组列表共用
export function sinkChecked<T>(list: T[], done: (item: T) => boolean): T[] {
  return [...list.filter(item => !done(item)), ...list.filter(done)];
}

// 刷新目录只读进度；角色切换或写入成功后重新投影，不反向覆盖存储。
watch([() => activeSkyCharacter.value?.id || '', progressRevision], ([id]) => {
  for (const [guides, wish] of [
    [state.seasonalGuides, loadSeasonWishStatus(String(id))],
    [state.eventGuides, loadEventWishStatus(String(id))],
  ] as const) {
    for (const guide of guides) {
      for (const item of guide.itemList || []) item.obtained = !!id && wish[guide.id]?.items?.[item.id] === true;
    }
  }
});
