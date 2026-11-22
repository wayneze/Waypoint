<script setup lang="ts">
import { version } from '../package.json';
import { computed, defineAsyncComponent, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { CalendarCheck, ChevronDown, ChevronRight, Database, Keyboard, Minus, Pin, Plus, ScrollText, Settings2, Shield } from '@lucide/vue';
import CharAvatar from './components/CharAvatar.vue';
import GameStampIcon from './components/GameStampIcon.vue';
import DatabaseRestoreStatus from './components/DatabaseRestoreStatus.vue';
import WorkspaceOverlays from './components/WorkspaceOverlays.vue';
import { CATALOG_NAV_GAMES, READER_NAV_GAMES, isGameEnabled } from './services/games';
import {
  activeCharObj,
  activeCharacter,
  characterChoices,
  currentTab,
  currentTabTitle,
  gameIdOf,
  gameRecords,
  gameStampOf,
  handleClose,
  handleMinimize,
  handleTitlebarMouseDown,
  handleToggleMaximize,
  handleTogglePin,
  isMaximized,
  isPinned,
  isSidebarCollapsed,
  toggleSidebarCollapse,
  uiZoomLevel,
  vaultList,
  zoomIn,
  zoomOut,
  zoomReset,
} from './workspace/core';
import { bootstrapWorkspace } from './workspace/bootstrap';
import { enabledGameIds, gameMetaOf } from './services/games';
import { dbInfo, panelHotkey, settingsSection, isRestoringDatabase } from './workspace/settings';
import { jx3ApiStatus } from './workspace/jx3/calendar';
import { memoPendingCount } from './workspace/memos';
import { singleAndGalgameGuides } from './workspace/guides/library';
import { homeMatrixStats as matrixStats } from './workspace/daily';
import { startSkyBoostSync } from './workspace/sky/boost';
import { startPriceMonitor } from './workspace/discovery';

// 页面按需加载，首屏只拉当前标签页。
const DailyPage = defineAsyncComponent(() => import('./pages/DailyPage.vue'));
const Jx3Page = defineAsyncComponent(() => import('./pages/Jx3Page.vue'));
const CollectionPage = defineAsyncComponent(() => import('./pages/CollectionPage.vue'));
const MemoPage = defineAsyncComponent(() => import('./pages/MemoPage.vue'));
const GalgamePage = defineAsyncComponent(() => import('./pages/GalgamePage.vue'));
const VaultPage = defineAsyncComponent(() => import('./pages/VaultPage.vue'));
const SettingsPage = defineAsyncComponent(() => import('./pages/SettingsPage.vue'));

const catalogNavGames = computed(() => {
  enabledGameIds.value;
  return CATALOG_NAV_GAMES.filter(game => isGameEnabled(game.id));
});
const readerNavGames = computed(() => {
  enabledGameIds.value;
  return READER_NAV_GAMES.filter(game => isGameEnabled(game.id));
});
const navGroups = computed(() => [
  {
    id: 'routine', label: '日程',
    items: [
      { id: 'daily', label: '今日待办', icon: CalendarCheck, stamp: undefined, count: matrixStats.value.remainingDaily },
      { id: 'memo', label: '万象记事', icon: ScrollText, stamp: undefined, count: memoPendingCount.value || null },
    ],
  },
  {
    id: 'games', label: '游戏旅程',
    items: [
      ...catalogNavGames.value.map(game => ({ id: game.id, label: game.label, icon: null, stamp: game.stamp, count: null })),
      ...readerNavGames.value.map(game => ({ id: game.id, label: game.navLabel!, icon: null, stamp: game.stamp, count: singleAndGalgameGuides.value.length || gameRecords.value.length || null })),
    ],
  },
  {
    id: 'tools', label: '工具',
    items: [
      { id: 'vault', label: '资产金库', icon: Shield, stamp: undefined, count: vaultList.value.length || null },
      { id: 'settings', label: '设置', icon: Settings2, stamp: undefined, count: null },
    ],
  },
]);
const currentNavGroup = computed(() => navGroups.value.find(group => group.items.some(item => item.id === currentTab.value)));
const currentNavItem = computed(() => currentNavGroup.value?.items.find(item => item.id === currentTab.value));

watch(enabledGameIds, (ids) => {
  const tab = currentTab.value;
  if (tab === 'daily' || tab === 'memo' || tab === 'vault' || tab === 'settings') return;
  if (!ids.includes(tab)) currentTab.value = 'daily';
}, { immediate: true });

const charMenuOpen = ref(false);
const charTriggerRef = ref<HTMLElement | null>(null);
const charMenuStyle = ref<Record<string, string>>({});
const statusDetailsRef = ref<HTMLDetailsElement | null>(null);

function placeCharMenu() {
  const el = charTriggerRef.value;
  if (!el) return;
  const r = el.getBoundingClientRect();
  // DOM rectangles are visual pixels; fixed menu offsets use zoomed CSS pixels.
  const scale = uiZoomLevel.value / 100;
  const rect = { top: r.top / scale, bottom: r.bottom / scale, left: r.left / scale, right: r.right / scale, width: r.width / scale };
  const height = window.innerHeight / scale;
  const menuH = Math.min(320, height - 24);
  if (isSidebarCollapsed.value) {
    const top = Math.min(rect.top, height - menuH - 8);
    charMenuStyle.value = {
      position: 'fixed',
      left: `${Math.round(rect.right + 8)}px`,
      top: `${Math.round(Math.max(8, top))}px`,
      minWidth: '196px',
      zIndex: '80'
    };
  } else {
    const below = rect.bottom + 6;
    const top = below + menuH > height - 8
      ? Math.max(8, rect.top - menuH - 6)
      : below;
    charMenuStyle.value = {
      position: 'fixed',
      left: `${Math.round(rect.left)}px`,
      top: `${Math.round(top)}px`,
      width: `${Math.round(rect.width)}px`,
      zIndex: '80'
    };
  }
}

function toggleCharMenu() {
  charMenuOpen.value = !charMenuOpen.value;
  if (charMenuOpen.value) void nextTick(placeCharMenu);
}

function pickCharacter(id: string) {
  activeCharacter.value = id;
  charMenuOpen.value = false;
}

function dismissShellMenus(e: MouseEvent) {
  const t = e.target as Node;
  if (!statusDetailsRef.value?.contains(t)) statusDetailsRef.value?.removeAttribute('open');
  if (!charMenuOpen.value) return;
  if (charTriggerRef.value?.contains(t)) return;
  const menu = document.getElementById('wp-char-menu');
  if (menu?.contains(t)) return;
  charMenuOpen.value = false;
}

function onShellMenuKey(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    charMenuOpen.value = false;
    statusDetailsRef.value?.removeAttribute('open');
  }
}

watch(isSidebarCollapsed, () => {
  charMenuOpen.value = false;
});
watch(uiZoomLevel, () => { if (charMenuOpen.value) void nextTick(placeCharMenu); });

let stopBoostSync: (() => void) | undefined;
let stopPriceMonitor: (() => void) | undefined;
onMounted(() => {
  bootstrapWorkspace();
  stopBoostSync = startSkyBoostSync();
  stopPriceMonitor = startPriceMonitor();
  document.addEventListener('mousedown', dismissShellMenus);
  window.addEventListener('keydown', onShellMenuKey);
  window.addEventListener('resize', placeCharMenu);
});

onBeforeUnmount(() => {
  stopBoostSync?.();
  stopPriceMonitor?.();
  document.removeEventListener('mousedown', dismissShellMenus);
  window.removeEventListener('keydown', onShellMenuKey);
  window.removeEventListener('resize', placeCharMenu);
});
</script>

<template>
  <div class="wp-shell wp-grain text-ink antialiased fixed inset-0 flex flex-col overflow-hidden border border-hairline" :class="{ 'is-sidebar-collapsed': isSidebarCollapsed }">
    <header 
      data-tauri-drag-region 
      @mousedown="handleTitlebarMouseDown"
      @dblclick="handleToggleMaximize"
      class="wp-titlebar z-40 flex items-center shrink-0 select-none cursor-default">
      
      <div class="wp-titlebar-brand pointer-events-none" data-tauri-drag-region>
        <img src="/waypoint-logo.svg" alt="Waypoint" />
        <span v-if="!isSidebarCollapsed" class="wp-titlebar-wordmark">Waypoint<span>.</span></span>
      </div>

      <div data-tauri-drag-region class="wp-titlebar-context pointer-events-none">
        <span class="wp-titlebar-section">{{ currentNavGroup?.label }}</span>
        <span class="wp-titlebar-slash" aria-hidden="true">/</span>
        <span class="wp-titlebar-location" :data-tone="currentNavItem?.stamp">
          <GameStampIcon v-if="currentNavItem?.stamp" :game="currentNavItem.stamp" />
          <span>{{ currentNavItem?.label || currentTabTitle }}</span>
        </span>
      </div>

      <div class="wp-titlebar-actions pointer-events-auto" data-tauri-no-drag @dblclick.stop>
        <button 
          type="button"
          @click="handleTogglePin" 
          class="wp-titlebar-pin"
          :aria-pressed="isPinned"
          :title="isPinned ? '已置顶' : '置顶（悬浮在游戏顶层打卡）'">
          <Pin :class="{ 'is-pinned': isPinned }" />
          <span>{{ isPinned ? '已置顶' : '置顶' }}</span>
        </button>
        <div class="wp-window-controls" data-tauri-no-drag>
        <button 
          type="button"
          @click="handleMinimize" 
          class="wp-window-button"
          title="最小化">
          <svg class="w-2.5 h-2.5" viewBox="0 0 10 10"><rect y="4.5" width="10" height="1" fill="currentColor"/></svg>
        </button>

        <button 
          type="button"
          @click="handleToggleMaximize" 
          class="wp-window-button"
          :title="isMaximized ? '还原' : '最大化'">
          <svg v-if="!isMaximized" class="w-2.5 h-2.5" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1"/></svg>
          <svg v-else class="w-2.5 h-2.5" viewBox="0 0 10 10"><path d="M2.5 2.5V0.5h7v7H7.5V9.5h-7v-7h2zm1 1h-2v5h5v-2H3.5v-3zm5-2h-5v1h4v4h1v-5z" fill="currentColor"/></svg>
        </button>

        <button 
          type="button"
          @click="handleClose" 
          class="wp-window-button is-close"
          title="关闭窗口 (最小化至系统托盘)">
          <svg class="w-2.5 h-2.5" viewBox="0 0 10 10"><path d="M1 1l8 8m0-8L1 9" stroke="currentColor" stroke-width="1.1" stroke-linecap="round"/></svg>
        </button>
        </div>
      </div>
    </header>

    <!-- 主体容器 (高度自适应 100% 视口，无多余外层滚动条) -->
    <div class="flex-1 min-h-0 min-w-0 flex overflow-hidden">
      
      <aside class="wp-sidebar" :inert="isRestoringDatabase" :class="{ 'is-collapsed': isSidebarCollapsed }" aria-label="工作台侧栏">
        <button type="button" class="wp-sidebar-edge"
          :aria-label="isSidebarCollapsed ? '展开侧栏' : '收起侧栏'"
          :aria-expanded="!isSidebarCollapsed" aria-controls="wp-main-navigation"
          :title="isSidebarCollapsed ? '点击展开侧栏 · Ctrl+B' : '点击收起侧栏 · Ctrl+B'"
          @click="toggleSidebarCollapse"><span aria-hidden="true" /></button>
        <div class="wp-sidebar-identity">
          <button v-if="characterChoices.length" ref="charTriggerRef" type="button"
            class="wp-character-switch" :data-tone="gameStampOf(activeCharacter)"
            :title="'切换角色 · ' + (activeCharObj?.name || '角色')"
            :aria-label="'切换角色：' + (activeCharObj?.name || '角色')"
            :aria-expanded="charMenuOpen" aria-haspopup="listbox" aria-controls="wp-char-menu"
            @click="toggleCharMenu">
            <CharAvatar size="md" :stamp="gameStampOf(activeCharacter)" :name="activeCharObj?.name || ''" :sect="activeCharObj?.sect" />
            <span v-if="!isSidebarCollapsed" class="wp-character-copy">
              <span class="wp-character-name">{{ activeCharObj?.name }}</span>
              <span class="wp-character-world">{{ gameMetaOf(gameIdOf(activeCharacter)).label }}{{ activeCharObj?.server ? ' · ' + activeCharObj.server : '' }}</span>
            </span>
            <ChevronDown v-if="!isSidebarCollapsed" class="wp-character-chevron" :class="{ 'is-open': charMenuOpen }" />
          </button>
          <button v-else type="button" class="wp-character-switch" title="去金库登记角色" aria-label="去金库登记角色" @click="currentTab = 'vault'">
            <span class="wp-character-empty"><Plus /></span>
            <span v-if="!isSidebarCollapsed" class="wp-character-copy">
              <span class="wp-character-name">添加角色</span>
              <span class="wp-character-world">从这里开始你的旅程</span>
            </span>
          </button>
        </div>
        <nav id="wp-main-navigation" class="wp-sidebar-navigation" aria-label="主导航">
          <div v-for="group in navGroups.filter(group => group.items.length)" :key="group.id"
            class="wp-nav-section" :class="{ 'is-tools': group.id === 'tools' }">
            <div v-if="!isSidebarCollapsed" class="wp-nav-section-label">{{ group.label }}</div>
            <button v-for="item in group.items" :key="item.id" type="button"
              class="wp-nav" :class="{ 'is-active': currentTab === item.id }" :data-tone="item.stamp"
              :aria-current="currentTab === item.id ? 'page' : undefined" :aria-label="item.label"
              :title="isSidebarCollapsed ? item.label + (item.count ? ' · ' + item.count : '') : undefined"
              @click="currentTab = item.id">
              <span class="wp-nav-icon">
                <component :is="item.icon" v-if="item.icon" />
                <GameStampIcon v-else-if="item.stamp" :game="item.stamp" />
              </span>
              <span v-if="!isSidebarCollapsed" class="wp-nav-label">{{ item.label }}</span>
              <span v-if="item.count !== null && (!isSidebarCollapsed || item.id === 'daily')" class="wp-nav-count"
                :class="{ 'is-clear': item.id === 'daily' && item.count === 0 }">{{ isSidebarCollapsed && item.count > 99 ? '99+' : item.count }}</span>
              <ChevronRight v-if="!isSidebarCollapsed && currentTab === item.id" class="wp-nav-current" />
            </button>
          </div>
        </nav>
      </aside>

      <Teleport to="body">
        <div
          v-if="charMenuOpen && characterChoices.length"
          id="wp-char-menu"
          class="wp-char-menu"
          role="listbox"
          :style="charMenuStyle">
          <button
            v-for="char in characterChoices"
            :key="char.id"
            type="button"
            role="option"
            class="wp-char-option"
            :class="char.id === activeCharacter ? 'is-active wp-stamp-' + gameStampOf(char.id) : ''"
            :aria-selected="char.id === activeCharacter"
            @click="pickCharacter(char.id)">
            <CharAvatar size="sm" :stamp="gameStampOf(char.id)" :name="char.name" :sect="char.sect" />
            <div class="min-w-0 flex-1">
              <div class="wp-char-option-name text-[13px] font-semibold truncate flex items-center gap-1">
                <span class="truncate">{{ char.name }}</span>
                <span v-if="char.starred" class="text-beacon text-[11px] shrink-0">★</span>
              </div>
              <div class="text-[11px] text-faint font-mono truncate">{{ gameMetaOf(char.game_id).label }}{{ char.server ? ' · ' + char.server : '' }}{{ char.sect ? ' · ' + char.sect : '' }}</div>
            </div>
          </button>
        </div>
      </Teleport>

      <main class="flex-1 min-w-0 h-full overflow-y-auto bg-canvas" :class="['daily', 'memo', 'jx3', 'sky', 'galgame', 'vault', 'settings'].includes(currentTab) ? 'wp-workspace-main' : 'p-3 sm:p-4'">
        <DatabaseRestoreStatus v-if="isRestoringDatabase" />
        <DailyPage v-else-if="currentTab === 'daily'" />
        <Jx3Page v-else-if="currentTab === 'jx3'" />
        <CollectionPage v-else-if="currentTab === 'sky'" />
        <MemoPage v-else-if="currentTab === 'memo'" />
        <GalgamePage v-else-if="currentTab === 'galgame'" />
        <VaultPage v-else-if="currentTab === 'vault'" />
        <SettingsPage v-else-if="currentTab === 'settings'" />
      </main>
    </div>

    <footer class="wp-statusbar" :inert="isRestoringDatabase" aria-label="工作台状态与显示">
      <details ref="statusDetailsRef" class="wp-status-details">
        <summary class="wp-status-trigger" title="查看资料与连接状态">
          <Database /><span>本地资料</span>
          <span class="wp-status-dot" :class="dbInfo ? 'is-ok' : 'is-idle'" />
          <ChevronDown class="wp-status-chevron" />
        </summary>
        <div class="wp-status-popover">
          <div class="wp-status-popover-heading"><span>工作台状态</span><span class="font-mono">v{{ version }}</span></div>
          <dl>
            <div><dt>本地数据库</dt><dd>{{ dbInfo ? '可用' : '尚未读取' }}</dd></div>
            <div v-if="dbInfo"><dt>资料大小</dt><dd>{{ Math.round(dbInfo.size_bytes / 1024).toLocaleString() }} KB</dd></div>
            <div v-if="isGameEnabled('jx3')"><dt>剑三资讯连接</dt><dd :class="jx3ApiStatus === 'fail' ? 'text-alert' : ''">{{ jx3ApiStatus === 'ok' ? '已连接' : jx3ApiStatus === 'fail' ? '连接失败' : '等待连接' }}</dd></div>
          </dl>
          <button type="button" @click="settingsSection = 'backup'; currentTab = 'settings'; statusDetailsRef?.removeAttribute('open')">资料与备份设置 <ChevronRight /></button>
        </div>
      </details>
      <span v-if="isGameEnabled('jx3') && jx3ApiStatus === 'fail'" class="wp-status-warning">剑三资讯暂未连接</span>
      <div class="wp-status-actions">
        <span class="wp-hotkey-hint" title="在其他窗口中呼出 Waypoint">
          <Keyboard /><span>快捷呼出</span><kbd>{{ panelHotkey }}</kbd>
        </span>
        <div class="wp-zoom-control" role="group" aria-label="界面缩放">
          <button type="button" @click="zoomOut" :disabled="uiZoomLevel <= 85" title="缩小 (Ctrl + -)" aria-label="缩小界面"><Minus /></button>
          <button type="button" class="wp-zoom-value" @click="zoomReset" title="重置为 100% (Ctrl + 0)" :aria-label="'当前缩放 ' + uiZoomLevel + '%，点击重置为 100%'">{{ uiZoomLevel }}<span>%</span></button>
          <button type="button" @click="zoomIn" :disabled="uiZoomLevel >= 125" title="放大 (Ctrl + +)" aria-label="放大界面"><Plus /></button>
        </div>
      </div>
    </footer>

    <WorkspaceOverlays />
  </div>
</template>
