// 工作台共享核：页签、金库名册、任务、toast、窗口。
import { ref, reactive, computed, watch } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import {
  type Character,
  type CollectionItem,
  type GameRecord,
  type ObsidianGameGuide,
  type SeasonalGuide,
  type TaskItem,
  type VaultAsset,
  type VaultCharacter,
  type VaultItem,
} from '../types';
import {
  addTask,
  deleteTask,
  isTauri,
  toggleTask,
  rebaseTaskCycles,
  updateTaskTitle,
  updateTaskCategory,
} from '../services/api';
import { sendNotification, toggleAlwaysOnTop } from '../services/systemApi';
import { optimistic } from '../services/optimistic';
import { kvGet, kvGetJson, kvSet } from '../services/kv';
import { loadSkySpirits } from '../services/skyData';
import { loadCustomSeasons } from '../services/skySeasonService';
import { parseMarkdownTasks } from '../services/markdownParser';
import {
  CATALOG_NAV_GAMES,
  GAMES,
  type GameStamp,
  applyGameResetOverride,
  clearGameResetOverride,
  gameHasRoster,
  gameMetaOf,
  isGameEnabled,
  normalizeTab,
  resolveVaultGameId,
} from '../services/games';
import { charactersFromVault } from '../services/vault';

const SHELL_TAB_TITLES: Record<string, string> = {
  daily: '今日',
  memo: '万象记事 · 周期追踪',
  vault: '账号资产安全金库',
  settings: '系统设置'
};

// 当前活动 Tab (持久化恢复；旧 collection 对齐为 sky)。?tab= 只给本地核对用。
const tabFromQuery = new URLSearchParams(window.location.search).get('tab');
export const currentTab = ref(normalizeTab(tabFromQuery || kvGet('waypoint_active_tab') || 'daily'));
watch(currentTab, (val) => {
  kvSet('waypoint_active_tab', val);
});

export function newTaskId(prefix = 't') {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`;
}

export async function pushCharacterTask(task: TaskItem, toastTitle?: string, toastBody?: string) {
  if (!allAccountTasks.value[task.character_id]) {
    allAccountTasks.value[task.character_id] = [];
  }
  allAccountTasks.value[task.character_id].unshift(task);
  if (task.character_id === activeCharacter.value) {
    if (task.category === 'weekly') state.weeklyTasks.unshift(task);
    else state.dailyTasks.unshift(task);
  }
  await addTask(task);
  if (toastTitle) showToast(toastTitle, toastBody || task.title);
}

export const vaultList = ref<VaultItem[]>([]);

function stripIdentityParen(name: string): string {
  return name.replace(/[（(][^）)]*[）)]/g, '').trim();
}

export function vaultIdentityName(v: VaultItem): string {
  const account = (v.account_name || '').trim();
  if (account) return account;
  const first = v.characters?.[0]?.name;
  if (first) return first;
  return (v.platform || '').trim() || '未命名账号';
}

const rosterCharacters = computed(() => {
  const list: Character[] = [];
  for (const v of vaultList.value) {
    const gameId = resolveVaultGameId(v.platform);
    if (!gameHasRoster(gameId) || !isGameEnabled(gameId)) continue;
    list.push(...charactersFromVault(v, gameId));
  }
  return list.sort((a, b) => Number(!!b.starred) - Number(!!a.starred));
});
export const characterList = computed(() => rosterCharacters.value.filter(c => c.is_active));
const characterGame = computed(() => CATALOG_NAV_GAMES.find(game => game.id === currentTab.value)?.id);
// 游戏页只切换本游戏角色；未挂日常板的角色仍可用于收集与探索。
export const characterChoices = computed(() => characterGame.value
  ? rosterCharacters.value.filter(c => c.game_id === characterGame.value)
  : rosterCharacters.value);
const savedActiveChar = kvGet('waypoint_active_character');
export const activeCharacter = ref(savedActiveChar || '');
export const activeCharObj = computed(() => rosterCharacters.value.find(c => c.id === activeCharacter.value));
export const characterDisplayName = computed(() => activeCharObj.value ? activeCharObj.value.name : '当前角色');
export const selectedScope = ref<string>('all');

// 全账号任务字典: characterId -> TaskItem[]
export const allAccountTasks = ref<Record<string, TaskItem[]>>({});

// 各角色快速添加任务输入框与类型
export const charQuickAddInputs = reactive<Record<string, string>>({});
export const charQuickAddCategory = reactive<Record<string, 'daily' | 'weekly'>>({});

export function gameIdOf(id: string): string {
  if (!id) return '';
  const found = rosterCharacters.value.find(c => c.id === id);
  if (found?.game_id) return found.game_id;
  const prefix = id.split('_')[0];
  return prefix || id;
}

export function getShortCharName(char: Character): string {
  const sameGame = characterList.value.filter(c => c.game_id === char.game_id);
  if (sameGame.length <= 1) return gameMetaOf(char.game_id).label;
  return stripIdentityParen(char.name) || gameMetaOf(char.game_id).label;
}

export function getCharBoardTitle(char: Character): string {
  return stripIdentityParen(char.name) || char.name;
}

export function getCharBoardHint(char: Character): string {
  if (char.sect) return char.sect.split('·')[0]?.trim() || char.sect;
  return char.server || '';
}

export function resetRuleFor(charId: string, category: 'daily' | 'weekly'): string {
  const meta = gameMetaOf(gameIdOf(charId));
  return category === 'weekly' ? meta.weeklyReset : meta.dailyReset;
}

export function gameStampOf(id: string): GameStamp {
  return gameMetaOf(gameIdOf(id)).stamp;
}

export function stampTone(id: string): string {
  return `text-stamp-${gameStampOf(id)}`;
}

export function vaultStampOf(platform: string): GameStamp {
  const id = resolveVaultGameId(platform);
  const known = GAMES.some(g => g.id === id);
  return known ? gameMetaOf(id).stamp : 'gal';
}

const PLATFORM_COLORS = ['#7f9bb0', '#9a8456', '#7b8f6a', '#94707f', '#6f8a93', '#a07a5e'];

/** 金库卡的色条颜色：已知游戏用游戏色，其他平台按名字固定分一种色。 */
export function vaultPlatformColor(platform: string): string {
  const id = resolveVaultGameId(platform);
  if (GAMES.some(g => g.id === id)) return `var(--color-stamp-${gameMetaOf(id).stamp})`;
  let h = 0;
  for (const ch of (platform || '').trim().toLowerCase()) h = (h * 31 + ch.codePointAt(0)!) >>> 0;
  return PLATFORM_COLORS[h % PLATFORM_COLORS.length];
}

export function selectScope(scope: string) {
  selectedScope.value = scope;
  if (scope !== 'all') {
    activeCharacter.value = scope;
  }
}

export function getCharName(charId: string): string {
  const c = rosterCharacters.value.find(item => item.id === charId);
  return c ? c.name : charId;
}

export function getCharStats(charId: string) {
  const list = allAccountTasks.value[charId] || [];
  const daily = list.filter(t => t.category === 'daily');
  const doneD = daily.filter(t => t.done).length;
  const weekly = list.filter(t => t.category === 'weekly');
  const doneW = weekly.filter(t => t.done).length;
  const dailyPct = daily.length > 0 ? Math.round((doneD / daily.length) * 100) : 0;
  return {
    dailyTotal: daily.length,
    dailyDone: doneD,
    dailyRemaining: daily.length - doneD,
    dailyPercent: dailyPct,
    weeklyTotal: weekly.length,
    weeklyDone: doneW
  };
}

// 全账号综合完成度汇总统计
export const matrixStats = computed(() => {
  let totalDaily = 0;
  let doneDaily = 0;
  let totalWeekly = 0;
  let doneWeekly = 0;

  for (const c of characterList.value) {
    const list = allAccountTasks.value[c.id] || [];
    for (const t of list) {
      if (t.category === 'daily') {
        totalDaily++;
        if (t.done) doneDaily++;
      } else {
        totalWeekly++;
        if (t.done) doneWeekly++;
      }
    }
  }

  const dailyPercent = totalDaily > 0 ? Math.round((doneDaily / totalDaily) * 100) : 0;
  const weeklyPercent = totalWeekly > 0 ? Math.round((doneWeekly / totalWeekly) * 100) : 0;
  const remainingDaily = totalDaily - doneDaily;

  return {
    totalDaily,
    doneDaily,
    remainingDaily,
    dailyPercent,
    totalWeekly,
    doneWeekly,
    weeklyPercent
  };
});

// 单机档案
export const gameRecords = ref<GameRecord[]>([]);

// Obsidian 游戏攻略库状态与过滤器
export const obsidianGameGuides = ref<ObsidianGameGuide[]>([]);
export const isScanningGuides = ref<boolean>(false);



// 状态管理
export const state = reactive({
  
  // 新增金库表单
  newVault: {
    platform: '',
    account_name: '',
    server: '',
    notes: '',
    steamid64: '',
    assets: [] as VaultAsset[],
    characters: [] as VaultCharacter[]
  },
  showAddVaultModal: false,

  // 新增游戏档案表单
  newGame: { title: '', category: 'galgame' as 'galgame' | 'single' | 'steam', status: 'playing' as GameRecord['status'], progress: '', rating: 0, save_path: '', notes: '', poster: '', poster_position: { x: 50, y: 50 }, guide_id: undefined as string | null | undefined, series: '', series_order: undefined as number | undefined },
  showAddGameModal: false,
  gameCategoryFilter: 'all',
  gameStatusFilter: 'all',

  dailyTasks: [] as (TaskItem & { key?: string })[],
  weeklyTasks: [] as TaskItem[],

  // 光遇常驻星盘 37 位先祖 (持久化加载)
  collectionItems: loadSkySpirits().map(s => ({
    id: s.id,
    map: s.map,
    name: s.name,
    action: s.action,
    redCandles: s.redCandles,
    wings: s.wings,
    location: s.location,
    keyItems: s.keyItems,
    coreItem: s.coreItem || '',
    coreCost: s.coreCost || '',
    status: s.unlocked ? ('unlocked' as const) : ('locked' as const),
    wingUnlocked: s.wingUnlocked !== undefined ? s.wingUnlocked : s.unlocked,
    itemObtained: s.itemObtained !== undefined ? s.itemObtained : s.unlocked,
    note: s.note
  })) as CollectionItem[],

  seasonalGuides: loadCustomSeasons() as SeasonalGuide[],
  eventGuides: [] as SeasonalGuide[]
});

// 计算属性：完成率
export const doneDailyCount = computed(() => state.dailyTasks.filter(t => t.done).length);
export const dailyPercent = computed(() => state.dailyTasks.length === 0 ? 0 : Math.round((doneDailyCount.value / state.dailyTasks.length) * 100));

// Markdown 批量导入器状态
export const showAchievementImportModal = ref(false);
export const showImportModal = ref(false);
export const importRawText = ref(`| 任务名称 | 类型 | 周期 | 标签 |
| 刷声望-七秀敬仰 | daily | 07:00 | 声望 |
| 一之窟英雄清CD | weekly | 周一 | 团本 |
| 绝境战场周常三胜 | weekly | 周常 | PVP |`);

export const parsedImportTasks = computed(() => {
  const meta = gameMetaOf(gameIdOf(activeCharacter.value));
  return parseMarkdownTasks(importRawText.value, activeCharacter.value, {
    daily: meta.dailyReset,
    weekly: meta.weeklyReset,
  });
});

export async function applyImportTasks() {
  const tasks = parsedImportTasks.value;
  if (tasks.length === 0) return;
  if (!allAccountTasks.value[activeCharacter.value]) {
    allAccountTasks.value[activeCharacter.value] = [];
  }
  for (const t of tasks) {
    if (t.category === 'daily') {
      state.dailyTasks.push(t);
    } else {
      state.weeklyTasks.push(t);
    }
    allAccountTasks.value[activeCharacter.value].push(t);
    await addTask(t);
  }
  showToast('批量导入成功', `已将 ${tasks.length} 项任务写入当前角色日程`);
  showImportModal.value = false;
}

// 复制文本至剪贴板
export async function copyToClipboard(text: string, label: string) {
  try {
    await navigator.clipboard.writeText(text);
    showToast('已复制到剪贴板', `${label} 已安全复制，可直接粘贴！`);
  } catch (err) {
    console.error('Failed to copy: ', err);
  }
}

export const isProjectingObsidian = ref(false);

// 系统设置与 Obsidian 仓库联动
export const obsidianVaultPath = ref('');
export const gameResetEpoch = ref(0);

// UI 界面缩放控制系统 (默认 100% 标准比例，防视口溢出截断)
export const uiZoomLevel = ref<number>(100);

export function setUiZoom(level: number) {
  const clamped = Number.isFinite(level) ? Math.min(Math.max(Math.round(level), 85), 125) : 100;
  uiZoomLevel.value = clamped;
  if (clamped === 100) {
    document.documentElement.style.zoom = '';
  } else {
    document.documentElement.style.zoom = `${clamped}%`;
  }
  kvSet('ui_zoom', clamped.toString());
}

export function zoomIn() {
  setUiZoom(uiZoomLevel.value + 5);
}

export function zoomOut() {
  setUiZoom(uiZoomLevel.value - 5);
}

export function zoomReset() {
  setUiZoom(100);
}

// 游戏悬浮 Tooltip 引擎
export const tooltip = reactive({
  show: false,
  x: 0,
  y: 0,
  title: '',
  category: '',
  qualityBadge: '',
  desc: '',
  stats: [] as { label: string; val: string }[]
});

export function triggerTooltip(e: MouseEvent, data: any) {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  tooltip.x = Math.min(window.innerWidth - 320, rect.left);
  tooltip.y = rect.bottom + 8;
  
  tooltip.title = `${data.map} · ${data.name}`;
  tooltip.category = '光遇 · 常驻星图先祖';
  tooltip.qualityBadge = data.status === 'unlocked' ? '已全毕业' : (data.wingUnlocked ? '永久翼已拿' : '待毕业');
  const locDesc = data.location ? `【所在位置】${data.location}\n` : '';
  const coreDesc = data.coreItem ? `【核心特色装扮】${data.coreItem} (${data.coreCost || '消耗爱心'}) - ${data.itemObtained ? '✓已换取' : '待换取'}\n` : '';
  const itemDesc = data.keyItems ? `【其他兑换】${data.keyItems}\n` : '';
  tooltip.desc = `${locDesc}${coreDesc}${itemDesc}${data.note || '节点全拿后可解锁常驻星盘毕业礼，升华节点永久增加重生光之翼。'}`;
  tooltip.stats = [
    { label: '永久光翼', val: `+${data.wings} 翼 (${data.wingUnlocked ? '✓ 已取得' : '未取得'})` },
    { label: '升华红蜡', val: `${data.redCandles} 根红蜡烛` }
  ];
  tooltip.show = true;
}

export function hideTooltip() {
  tooltip.show = false;
}

// Toast 状态
export const toast = reactive({
  show: false,
  title: '',
  body: '',
  persistent: false,
});
export let toastTimer: any = null;

export function showToast(title: string, body: string, options: { persistent?: boolean } = {}) {
  toast.title = title;
  toast.body = body;
  toast.persistent = !!options.persistent;
  toast.show = true;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = null;
  if (toast.persistent) return;
  toastTimer = setTimeout(() => {
    toast.show = false;
  }, 4000);
}

// 同步当前激活角色的 tasks 到 state.dailyTasks 和 state.weeklyTasks
export function syncActiveCharacterState() {
  const activeList = allAccountTasks.value[activeCharacter.value] || [];
  state.dailyTasks = activeList.filter(t => t.category === 'daily');
  state.weeklyTasks = activeList.filter(t => t.category === 'weekly');
}

watch(activeCharacter, (val) => {
  const character = activeCharObj.value;
  if (character) {
    kvSet('waypoint_active_character', val);
    kvSet(`waypoint_active_character_${character.game_id}`, val);
  }
  if (currentTab.value === 'daily' && selectedScope.value !== 'all') {
    selectedScope.value = characterList.value.some(c => c.id === val) ? val : 'all';
  }
  syncActiveCharacterState();
}, { flush: 'sync' });

watch([currentTab, rosterCharacters], () => {
  // 设置在桌面端异步加载，因此在名册就绪后读取，同时接纳原有的单角色选择。
  const saved = rosterCharacters.value.find(c => c.id === kvGet('waypoint_active_character'));
  if (saved && !kvGet(`waypoint_active_character_${saved.game_id}`)) {
    kvSet(`waypoint_active_character_${saved.game_id}`, saved.id);
  }
  if (selectedScope.value !== 'all' && !characterList.value.some(c => c.id === selectedScope.value)) {
    selectedScope.value = 'all';
  }
  const preferred = characterGame.value
    ? kvGet(`waypoint_active_character_${characterGame.value}`)
    : currentTab.value === 'daily' && selectedScope.value !== 'all'
      ? selectedScope.value
      : activeCharacter.value || saved?.id;
  const choices = characterChoices.value;
  activeCharacter.value = (
    choices.find(c => c.id === preferred)
    || choices.find(c => c.id === activeCharacter.value)
    || choices.find(c => c.id === saved?.id)
    || choices[0]
  )?.id || '';
}, { immediate: true, flush: 'sync' });

// 切换任务状态 (矩阵看板中直接触发)
export async function handleToggleMatrixTask(task: TaskItem) {
  const sync = (done: boolean) => {
    task.done = done;
    if (task.character_id !== activeCharacter.value) return;
    const d = state.dailyTasks.find(t => t.id === task.id);
    if (d) d.done = done;
    const w = state.weeklyTasks.find(t => t.id === task.id);
    if (w) w.done = done;
  };
  const next = !task.done;
  await optimistic(() => sync(next), () => sync(!next), () => toggleTask(task.id, next, task.character_id));
}

// 切换单号聚焦中的任务状态
export async function handleToggleTask(task: TaskItem) {
  const sync = (done: boolean) => {
    task.done = done;
    const t = allAccountTasks.value[activeCharacter.value]?.find(item => item.id === task.id);
    if (t) t.done = done;
  };
  const next = !task.done;
  await optimistic(() => sync(next), () => sync(!next), () => toggleTask(task.id, next, activeCharacter.value));
}

export async function handleRenameTask(task: TaskItem, title: string, charId?: string) {
  const next = title.trim();
  if (!next || next === task.title) return;
  const owner = charId || task.character_id || activeCharacter.value;
  await updateTaskTitle(task.id, next, owner);
  task.title = next;
  const row = (allAccountTasks.value[owner] || []).find(t => t.id === task.id);
  if (row) row.title = next;
  const d = state.dailyTasks.find(t => t.id === task.id);
  if (d) d.title = next;
  const w = state.weeklyTasks.find(t => t.id === task.id);
  if (w) w.title = next;
  showToast('已修改', next);
}

export async function handleSwitchTaskCategory(task: TaskItem) {
  const next: 'daily' | 'weekly' = task.category === 'weekly' ? 'daily' : 'weekly';
  task.category = next;
  task.reset_rule = resetRuleFor(task.character_id || activeCharacter.value, next);
  const owner = task.character_id || activeCharacter.value;
  const list = allAccountTasks.value[owner];
  const row = list?.find(t => t.id === task.id);
  if (row) {
    row.category = next;
    row.reset_rule = task.reset_rule;
  }
  await updateTaskCategory(task.id, next, owner);
  showToast('已调整分类', `已改为${next === 'weekly' ? '周常' : '日常'}`);
}

// 矩阵看板为指定角色快速添加任务
export async function handleCharQuickAdd(charId: string) {
  const text = (charQuickAddInputs[charId] || '').trim();
  if (!text) return;
  const category = charQuickAddCategory[charId] || 'daily';
  const newTask: TaskItem = {
    id: `t_${Date.now()}_${Math.random().toString(36).slice(2, 5)}`,
    character_id: charId,
    title: text,
    category: category,
    reset_rule: resetRuleFor(charId, category),
    source_type: 'custom',
    tag: category === 'daily' ? '日常' : '周常',
    done: false
  };

  await addTask(newTask);
  if (!allAccountTasks.value[charId]) {
    allAccountTasks.value[charId] = [];
  }
  allAccountTasks.value[charId].push(newTask);
  if (charId === activeCharacter.value) {
    if (category === 'daily') state.dailyTasks.push(newTask);
    else state.weeklyTasks.push(newTask);
  }

  charQuickAddInputs[charId] = '';
  showToast('录入成功', `已为【${getCharName(charId)}】添加任务：${text}`);
}

// 矩阵看板删除指定角色的任务
export async function handleDeleteMatrixTask(taskId: string, charId: string, event?: Event) {
  if (event) event.stopPropagation();
  await deleteTask(taskId, charId);
  if (allAccountTasks.value[charId]) {
    allAccountTasks.value[charId] = allAccountTasks.value[charId].filter(t => t.id !== taskId);
  }
  if (charId === activeCharacter.value) {
    state.dailyTasks = state.dailyTasks.filter(t => t.id !== taskId);
    state.weeklyTasks = state.weeklyTasks.filter(t => t.id !== taskId);
  }
  showToast('已删除', '任务已从该角色移除');
}

// 单号聚焦中删除任务
export async function handleDeleteTask(taskId: string, category: 'daily' | 'weekly', event?: Event) {
  if (event) event.stopPropagation();
  if (category === 'daily') {
    state.dailyTasks = state.dailyTasks.filter(t => t.id !== taskId);
  } else {
    state.weeklyTasks = state.weeklyTasks.filter(t => t.id !== taskId);
  }
  if (allAccountTasks.value[activeCharacter.value]) {
    allAccountTasks.value[activeCharacter.value] = allAccountTasks.value[activeCharacter.value].filter(t => t.id !== taskId);
  }
  await deleteTask(taskId, activeCharacter.value);
  showToast('已删除', '任务已从数据库移除');
}

// 重置单个角色的今日日常
export async function resetCharacterDailies(charId: string) {
  if (gameIdOf(charId) === 'sky') {
    const { resetSkyDailies } = await import('./sky/plan');
    await resetSkyDailies([charId]);
  }
  const list = allAccountTasks.value[charId] || [];
  for (const t of list) {
    if (t.category === 'daily' && t.done) {
      t.done = false;
      await toggleTask(t.id, false, charId);
    }
  }
  if (charId === activeCharacter.value) {
    for (const t of state.dailyTasks) {
      t.done = false;
    }
  }
  showToast('日常已复位', `【${getCharName(charId)}】的今日日常已全部重置！`);
}

// 重置全账号所有角色的今日日常
export async function resetAllDailies() {
  const skyIds = characterList.value.filter(c => c.game_id === 'sky').map(c => c.id);
  if (skyIds.length) {
    const { resetSkyDailies } = await import('./sky/plan');
    await resetSkyDailies(skyIds);
  }
  for (const charId of Object.keys(allAccountTasks.value)) {
    const list = allAccountTasks.value[charId] || [];
    for (const t of list) {
      if (t.category === 'daily' && t.done) {
        t.done = false;
        await toggleTask(t.id, false, charId);
      }
    }
  }
  for (const t of state.dailyTasks) {
    t.done = false;
  }
  showToast('全账号已重置', '所有游戏与角色的今日日常均已复位！');
}

// 单号聚焦重置当前
export async function resetToday() {
  await resetCharacterDailies(activeCharacter.value);
}



// 触发系统弹窗测试
export async function triggerSampleToast() {
  showToast('通知测试', '能看到这条，说明应用内提示正常。系统通知稍后弹出。');
  await sendNotification('Waypoint · 通知测试', '能看到这条，说明系统通知已打开。');
}

// 窗口置顶控制 (悬浮伴侣模式)
export const isPinned = ref(false);
export async function handleTogglePin() {
  const next = !isPinned.value;
  isPinned.value = await toggleAlwaysOnTop(next);
  showToast(
    isPinned.value ? '已开启窗口置顶' : '已取消窗口置顶',
    isPinned.value ? 'Waypoint 现已悬浮于游戏与全屏窗口顶层 (可在游戏内打卡)' : '恢复正常系统窗口层级'
  );
}

// 原生一体化无边框窗口控制 (Windows 11 风格)
export const isMaximized = ref(false);

export const currentTabTitle = computed(() => {
  const tab = currentTab.value;
  if (SHELL_TAB_TITLES[tab]) return SHELL_TAB_TITLES[tab];
  const meta = gameMetaOf(tab);
  return meta.pageTitle || meta.navLabel || meta.label || '工作台';
});

export async function handleMinimize() {
  if (isTauri()) {
    try {
      await getCurrentWindow().minimize();
      return;
    } catch (e) {
      console.warn('Failed to minimize window:', e);
    }
  }
  showToast('桌面端交互', '在独立桌面客户端中，将最小化至 Windows 任务栏');
}

export async function handleToggleMaximize() {
  if (isTauri()) {
    try {
      await getCurrentWindow().toggleMaximize();
      isMaximized.value = await getCurrentWindow().isMaximized();
      return;
    } catch (e) {
      console.warn('Failed to toggle maximize:', e);
    }
  }
  isMaximized.value = !isMaximized.value;
  showToast('窗口尺寸', isMaximized.value ? '已模拟最大化窗口' : '已还原标准窗口');
}

export async function handleClose() {
  if (isTauri()) {
    try {
      // 优先隐退至系统托盘，维持后台 20:00 日常防漏定时守护
      await getCurrentWindow().hide();
      return;
    } catch (_) {
      try {
        await getCurrentWindow().close();
        return;
      } catch (e) {
        console.warn('Failed to close window:', e);
      }
    }
  }
  showToast('桌面端交互', '在独立桌面客户端中，窗口将隐藏并常驻系统托盘（按主面板热键唤出，默认 Alt+W）');
}

export function handleTitlebarMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  const target = e.target as HTMLElement | null;
  // 如果点击的是按钮、输入框、下拉框或明确禁止拖拽的区域，不触发移动
  if (target && target.closest('button, input, select, textarea, a, [data-tauri-no-drag]')) {
    return;
  }
  if (isTauri()) {
    try {
      getCurrentWindow().startDragging();
    } catch (_) {}
  }
}

export const isSidebarCollapsed = ref<boolean>(
  kvGetJson('waypoint_sidebar_collapsed', false)
);

export function toggleSidebarCollapse() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
  kvSet('waypoint_sidebar_collapsed', JSON.stringify(isSidebarCollapsed.value));
}

export async function saveGameResetTimes(changes: { gameId: string; dailyReset: string; weeklyReset: string }[]) {
  for (const { gameId, dailyReset, weeklyReset } of changes) {
    const defaults = GAMES.find(game => game.id === gameId);
    if (defaults?.dailyReset === dailyReset && defaults.weeklyReset === weeklyReset) {
      clearGameResetOverride(gameId);
    } else {
      applyGameResetOverride(gameId, dailyReset, weeklyReset);
    }
  }
  await rebaseTaskCycles();
  gameResetEpoch.value += 1;
  showToast('重置时间已保存', '所有修改已生效，当前任务勾选保持不变');
}

