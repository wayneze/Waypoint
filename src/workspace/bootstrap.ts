// 挂载初始化、日课换日、Obsidian 投影。
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { type CustomMemo, type TaskItem, type WaypointProjectionScope, type WaypointProjectionSnapshot } from '../types';
import {
  getAllTasks,
  getCustomMemos,
  getGameRecords,
  getSetting,
  getVaultItems,
  isTauri,
} from '../services/api';
import {
  exportWaypointProjection,
  getDbInfo,
  getHotkeyStatus,
  isAlwaysOnTop,
  scanObsidianVault,
  steamWebApiKeyIsConfigured,
} from '../services/systemApi';
import { kvGet, kvGetJson, kvSet, hydrateKv } from '../services/kv';
import { calculateSkyShard } from '../services/skyShards';
import { hydrateEnabledGames, hydrateGameResetOverrides, isGameEnabled } from '../services/games';
import {
  allAccountTasks,
  charQuickAddCategory,
  characterList,
  currentTab,
  selectedScope,
  gameRecords,
  isMaximized,
  isPinned,
  isSidebarCollapsed,
  isProjectingObsidian,
  obsidianVaultPath,
  setUiZoom,
  showToast,
  state,
  syncActiveCharacterState,
  toggleSidebarCollapse,
  vaultList,
  zoomIn,
  zoomOut,
  zoomReset,
} from './core';
import { jx3Pendants, jx3SetTokens, loadJx3BoxDungeonTree } from './jx3/collect';
import { refreshJx3MapEvents, refreshJx3WorldBossClock, syncJx3DailyCalendar } from './jx3/calendar';
import { reloadManualFromKv } from './jx3/exterior';
// 光遇模块只在启用光遇时按需加载，主包不打进去。
let skyModule: Promise<typeof import('./sky/tabs') & typeof import('./sky/daily')> | null = null;
function loadSky() {
  return (skyModule ??= Promise.all([import('./sky/tabs'), import('./sky/daily')]).then(([tabs, daily]) => ({ ...tabs, ...daily })));
}
import { loadGameActivity } from './gameActivity';
import { loadSteamLibrary, migrateSkyLegacyQueryId, steamWebApiKeyConfigured, steamWebApiKeySaved } from './vault';
import { loadCompletedRoutes, loadGuideBookmarks, loadGuideReaderPreferences, loadGuideRouteFocus } from './guides/reader';
import { loadCustomGameStatus } from './guides/status';
import { loadGameLibraryPreferences, loadObsidianGuides } from './guides/library';
import { checkMemoIntervalResets, customMemos } from './memos';
import { captureHotkey, dbInfo, hotkeyError, obsidianFiles, panelHotkey, refreshWebdavStatus } from './settings';
import { describeError } from '../services/errors';

async function buildWaypointProjectionSnapshot(scope: WaypointProjectionScope): Promise<WaypointProjectionSnapshot> {
  const { jx3CompletedAchieves, jx3Reputations } = await import('./jx3/progress');
  const tasks = Object.values(allAccountTasks.value).flat();
  return {
    scopes: [scope],
    characters: characterList.value.map(c => ({
      id: c.id,
      name: c.name,
      game_id: c.game_id,
      server: c.server,
      sect: c.sect,
      is_active: c.is_active
    })),
    tasks: tasks.map(t => ({
      id: t.id,
      character_id: t.character_id,
      title: t.title,
      category: t.category,
      reset_rule: t.reset_rule,
      source_type: t.source_type,
      api_key: t.api_key,
      tag: t.tag,
      done: t.done,
      catalog_kind: t.catalog_kind,
      catalog_id: t.catalog_id
    })),
    vault: vaultList.value.map(v => ({
      id: v.id,
      platform: v.platform,
      account_name: v.account_name,
      server: v.server,
      notes: v.notes,
      steamid64: v.steamid64,
      assets: v.assets,
      characters: v.characters
    })),
    memos: customMemos.value.map(m => ({ ...m })),
    gameRecords: gameRecords.value.map(r => ({
      id: r.id,
      title: r.title,
      category: r.category,
      status: r.status,
      progress: r.progress,
      rating: r.rating
    })),
    jx3: {
      pendants: jx3Pendants.value.map(p => ({
        id: p.id,
        name: p.name,
        dungeon: p.dungeon,
        obtained: !!p.obtained,
        cdCleared: !!p.cdCleared
      })),
      reputations: jx3Reputations.value.map(r => ({
        id: r.id,
        name: r.name,
        currentLevel: r.currentLevel
      })),
      tokens: jx3SetTokens.value.map(s => ({
        id: s.id,
        seriesName: s.seriesName,
        pieces: {
          head: !!s.pieces.head.obtained,
          body: !!s.pieces.body.obtained,
          belt: !!s.pieces.belt.obtained,
          wrist: !!s.pieces.wrist.obtained,
          shoes: !!s.pieces.shoes.obtained
        }
      })),
      achievements: Array.from(jx3CompletedAchieves.value)
    },
    sky: {
      spirits: state.collectionItems.map(s => ({
        id: s.id,
        map: s.map,
        name: s.name,
        unlocked: s.status === 'unlocked',
        wingUnlocked: !!s.wingUnlocked,
        itemObtained: !!s.itemObtained
      })),
      seasons: state.seasonalGuides.map(g => ({
        id: g.id,
        season: g.season,
        selected: !!g.selected,
        items: (g.itemList || []).map(item => ({
          id: item.id,
          name: item.name,
          obtained: !!item.obtained
        }))
      }))
    }
  };
}

const PROJECTION_DONE: Record<WaypointProjectionScope, string> = {
  vault: '已投影账号金库。其他页面未改动。',
  dailies: '已投影日常与周常。其他页面未改动。',
  memos: '已投影万象记事。其他页面未改动。',
  games: '已投影游戏档案摘要。攻略原文未改动。',
  jx3: '已投影剑网3图鉴进度。其他页面未改动。',
  sky: '已投影光遇进度。其他页面未改动。',
  all: '已投影全部航标笔记。'
};

export async function handleProjectToObsidian(scope: WaypointProjectionScope) {
  if (!obsidianVaultPath.value) {
    showToast('未配置 Obsidian 路径', '请先在【系统设置】中绑定你的 Obsidian 仓库');
    currentTab.value = 'settings';
    return;
  }
  isProjectingObsidian.value = true;
  try {
    const res = await exportWaypointProjection(await buildWaypointProjectionSnapshot(scope));
    const n = res.files?.length ?? 0;
    showToast('投影完成', `${PROJECTION_DONE[scope]} 写出 ${n} 篇。`);
    obsidianFiles.value = await scanObsidianVault(obsidianVaultPath.value);
  } catch (err: any) {
    showToast('投影失败', (err ? describeError(err).message : '未知错误'));
  } finally {
    isProjectingObsidian.value = false;
  }
}

export function handleProjectAllToObsidian() {
  return handleProjectToObsidian('all');
}

export async function loadAllTasks() {
  try {
    const all = await getAllTasks();
    const map: Record<string, TaskItem[]> = {};
    for (const c of characterList.value) {
      map[c.id] = [];
      if (!charQuickAddCategory[c.id]) charQuickAddCategory[c.id] = 'daily';
    }
    for (const t of all) {
      if (!map[t.character_id]) map[t.character_id] = [];
      map[t.character_id].push(t);
    }
    allAccountTasks.value = map;

    syncActiveCharacterState();
    if (isGameEnabled('jx3')) {
      void Promise.all([
        syncJx3DailyCalendar(),
        refreshJx3MapEvents(),
        loadJx3BoxDungeonTree(),
        import('./jx3/progress').then(({ loadJx3AdventureCatalog, reconcileJx3Snapshot }) =>
          Promise.all([loadJx3AdventureCatalog(), reconcileJx3Snapshot()]))
      ]).catch(err => {
        console.warn('Background sync JX3 failed:', err);
      });
    }
  } catch (err) {
    console.warn('Failed to load all tasks:', err);
  }
}

let lastConnectedSkyCatalogRefreshAt = 0;

function refreshWorkspaceClock() {
  checkMemoIntervalResets();
  if (document.visibilityState !== 'visible') return;
  if (isGameEnabled('jx3')) {
    void syncJx3DailyCalendar();
    void refreshJx3MapEvents();
  }
  if (isGameEnabled('sky')) {
    void loadSky().then(({ ensureSkyCatalogs, refreshSkyDaily }) => {
      if (Date.now() - lastConnectedSkyCatalogRefreshAt >= 15 * 60_000) {
        lastConnectedSkyCatalogRefreshAt = Date.now();
        return ensureSkyCatalogs();
      }
      return refreshSkyDaily(false);
    });
  }
}

async function onBackendCycleReset(games: string[]) {
  await loadAllTasks();
  if (isGameEnabled('jx3') && games.some(g => g.startsWith('剑网3'))) await syncJx3DailyCalendar();
  showToast('日课已按重置点换日', games.join('、'));
}

// 挂载初始化
export async function bootstrapWorkspace() {
  // 屏蔽网页默认右键菜单 (杜绝网页感)
  window.addEventListener('contextmenu', (e) => {
    const target = e.target as HTMLElement | null;
    if (!target || !target.closest('input, textarea, [contenteditable="true"]')) {
      e.preventDefault();
    }
  });

  // 屏蔽 F5 / Ctrl+R 刷新与 Ctrl+F 查找 (杜绝网页重载跳帧)
  window.addEventListener('keydown', (e) => {
    if (e.key === 'F5' || ((e.ctrlKey || e.metaKey) && (e.key === 'r' || e.key === 'R' || e.key === 'f' || e.key === 'F'))) {
      e.preventDefault();
    }
    // 键盘缩放快捷键 (Ctrl + +/-/0)
    if (e.ctrlKey || e.metaKey) {
      if (e.key === '=' || e.key === '+') {
        e.preventDefault();
        zoomIn();
      } else if (e.key === '-') {
        e.preventDefault();
        zoomOut();
      } else if (e.key === '0') {
        e.preventDefault();
        zoomReset();
      }
    }
  });

  await hydrateKv();
  reloadManualFromKv();
  hydrateGameResetOverrides();
  hydrateEnabledGames();

  // 恢复持久化 UI 缩放比例 (默认 100% 标准无溢出比例)
  try {
    const savedZoom = kvGet('ui_zoom');
    setUiZoom(savedZoom ? Number(savedZoom) : 100);
  } catch (_) {
    setUiZoom(100);
  }

  try {
    const hotkeys = await getHotkeyStatus();
    if (hotkeys) {
      panelHotkey.value = hotkeys.panel;
      captureHotkey.value = hotkeys.capture;
      kvSet('hotkey_panel', hotkeys.panel);
      kvSet('hotkey_capture', hotkeys.capture);
      hotkeyError.value = [hotkeys.panel_error, hotkeys.capture_error].filter(Boolean).join('；');
    }
    isPinned.value = await isAlwaysOnTop();
    if (isTauri()) {
      isMaximized.value = await getCurrentWindow().isMaximized();
      await getCurrentWindow().listen('tauri://resize', async () => {
        isMaximized.value = await getCurrentWindow().isMaximized();
      });
    }
  } catch (_) {}

  // 本机用户数据先就绪；外部目录与网络同步随后独立进行。
  await Promise.all([
    getVaultItems().then(async items => { vaultList.value = items; await migrateSkyLegacyQueryId(); await loadAllTasks(); }),
    getGameRecords().then(items => { gameRecords.value = items; }),
    getCustomMemos().then(items => { customMemos.value = items; checkMemoIntervalResets(); }),
  ]);
  if (isGameEnabled('galgame')) void Promise.all([loadSteamLibrary(), loadGameActivity()]);
  void steamWebApiKeyIsConfigured().then(configured => {
    steamWebApiKeyConfigured.value = configured;
    steamWebApiKeySaved.value = configured;
  });

  // 读取持久化设置与本地数据库信息
  try {
    const savedPath = await getSetting('obsidian_vault_path');
    if (savedPath) {
      obsidianVaultPath.value = savedPath;
      void scanObsidianVault(savedPath).then(files => { obsidianFiles.value = files; }).catch(err => console.warn('Obsidian scan failed:', err));
    }
    dbInfo.value = await getDbInfo();
    void refreshWebdavStatus();
  } catch (_) {}

  // 加载攻略进度书签
  try {
    loadGuideBookmarks();
    loadGuideReaderPreferences();
    loadGuideRouteFocus();
    loadCompletedRoutes();
    loadCustomGameStatus();
    loadGameLibraryPreferences();
    isSidebarCollapsed.value = kvGetJson('waypoint_sidebar_collapsed', isSidebarCollapsed.value);

    window.addEventListener('keydown', (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'b') {
        e.preventDefault();
        toggleSidebarCollapse();
      }
    });
  } catch (_) {}

  try {
    if (isTauri()) await listen<CustomMemo>('memo-added', (event) => {
      const memo = event.payload;
      if (!customMemos.value.some(row => row.id === memo.id)) {
        customMemos.value.unshift(memo);
      }
    });
    checkMemoIntervalResets();
    if (isTauri()) await listen<string[]>('cycle-reset', (event) => { void onBackendCycleReset(event.payload); });
    // 托盘菜单"今日日课剩 N 项"点进来
    if (isTauri()) await listen<string>('navigate', (event) => {
      currentTab.value = event.payload === 'today' ? 'daily' : event.payload;
      selectedScope.value = 'all';
    });
    setInterval(refreshWorkspaceClock, 60000);
    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible') refreshWorkspaceClock();
    });
    window.addEventListener('focus', refreshWorkspaceClock);
    window.addEventListener('online', refreshWorkspaceClock);

    // 光遇落石、剑三世界首领倒计时（每 30 秒刷新）
    setInterval(() => {
      if (isGameEnabled('sky')) {
        void loadSky().then(({ skyShard, skyServerMode }) => {
          skyShard.value = calculateSkyShard(new Date(), skyServerMode.value);
        });
      }
      if (isGameEnabled('jx3')) refreshJx3WorldBossClock();
    }, 30000);
  } catch (e) {
    console.warn('Failed to load custom memos:', e);
  }

  const skyDailyP = isGameEnabled('sky')
    ? loadSky().then(({ ensureSkyCatalogs }) => ensureSkyCatalogs())
    : Promise.resolve();
  await Promise.all([loadObsidianGuides(), skyDailyP]);
}
