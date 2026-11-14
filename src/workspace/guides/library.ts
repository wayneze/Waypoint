// 单机 / Galgame 书库：筛选、置顶、扫描。
import { describeError } from '../../services/errors';
import { ref, computed } from 'vue';
import { type GameCuration, type GameLibraryDetails, type GameMetadata, type GameRecord, type ObsidianGameGuide } from '../../types';
import { emptyTaxonomyFilter, matchesGameTaxonomy, type GameTaxonomySelection } from '../../services/gameTaxonomy';
import { buildGameLibrary, gameSourceIds, withGameMetadata } from '../../services/gameLibrary';
import { sanitizeAndEnrichGuide } from '../../services/gameCovers';
import { saveGameRecord, updateGameRecord } from '../../services/api';
import { scanObsidianGameGuides } from '../../services/systemApi';
import { kvGet, kvGetJson, kvSet } from '../../services/kv';
import { gameRecords, isScanningGuides, obsidianGameGuides, obsidianVaultPath, showToast, state } from '../core';
import { getEffectiveGameStatus, setGameStatus } from './status';
import { guideProgressMap, showOnlyBookmarked, selectedGuide, handleOpenGuideReader } from './reader';
import { loadGameExtras, recentGames, rememberGame } from './extras';

export const guideCategoryFilter = ref<'all' | 'pinned' | 'galgame' | 'single' | 'unmatched'>('all');
export const galgameShelf = ref<'library' | 'explore'>('library');
export const guideStatusFilter = ref<string>('all');
export const guideSearchQuery = ref<string>('');
export const gameTaxonomyFilter = ref(emptyTaxonomyFilter());
export const taxonomyFilterCount = computed(() => {
  const f = gameTaxonomyFilter.value;
  return f.tags.length + f.excluded.length + f.companies.length + f.people.length + Number(!!f.staff_role);
});
export function filterGameLibrary(selection: GameTaxonomySelection) {
  const f = emptyTaxonomyFilter();
  if (selection.kind === 'tag') f.tags = [selection.name];
  else if (selection.kind === 'person') { f.people = [selection.name]; f.staff_role = selection.role || ''; }
  else { f.companies = [selection.name]; f.company_role = selection.kind; }
  gameTaxonomyFilter.value = f;
  guideSearchQuery.value = ''; guideCategoryFilter.value = 'all'; guideStatusFilter.value = 'all';
  showOnlyBookmarked.value = false; galgameShelf.value = 'library';
}
export const galgameCardDensity = ref<'compact' | 'large'>('large');
export const galgameCoverMode = ref<'poster' | 'badge'>('poster');
export const seriesDisplayMode = ref<'collapsed' | 'expanded'>('collapsed');
const seriesExpansionOverrides = ref<Record<string, boolean>>({});

export function isSeriesExpanded(key: string) {
  return seriesExpansionOverrides.value[key] ?? seriesDisplayMode.value === 'expanded';
}
export function toggleSeriesExpanded(key: string) {
  seriesExpansionOverrides.value[key] = !isSeriesExpanded(key);
}
export function setSeriesDisplayMode(mode: 'collapsed' | 'expanded') {
  seriesDisplayMode.value = mode;
  seriesExpansionOverrides.value = {};
  kvSet('waypoint_series_display_mode', mode);
}
export function toggleGalgameCardDensity() {
  galgameCardDensity.value = galgameCardDensity.value === 'large' ? 'compact' : 'large';
  kvSet('waypoint_galgame_card_density', galgameCardDensity.value);
}
export function toggleGalgameCoverMode() {
  galgameCoverMode.value = galgameCoverMode.value === 'poster' ? 'badge' : 'poster';
  kvSet('waypoint_galgame_cover_mode', galgameCoverMode.value);
}

export const failedPosters = ref<Record<string, boolean>>({});
export const failedInlineImages = ref<Record<string, boolean>>({});

export const removingGame = ref<ObsidianGameGuide | null>(null);
const removedGuideIds = ref(kvGetJson<string[]>('waypoint_removed_guides', []));
export const libraryGuides = computed(() => obsidianGameGuides.value.filter(
  g => !removedGuideIds.value.includes(g.id) && (g.category === 'galgame' || g.category === 'single' || g.category === 'steam')
));
export const singleAndGalgameGuides = computed(() => buildGameLibrary(libraryGuides.value, gameRecords.value));

// 游戏置顶管理 (正在游玩 / 重点关注 - 本地缓存 + app_settings 持久化)
export const pinnedGameIds = ref<string[]>([]);

function guideSourceKeys(guide: ObsidianGameGuide): string[] {
  const record = gameRecords.value.find(item => item.id === guide.record_id);
  return [...gameSourceIds(guide), guide.title, guide.file_name, ...(record ? [record.title] : [])];
}

export function isGamePinned(guideOrId: ObsidianGameGuide | string | undefined | null): boolean {
  if (!guideOrId) return false;
  const guide = typeof guideOrId === 'string'
    ? singleAndGalgameGuides.value.find(game => guideSourceKeys(game).includes(guideOrId)) : guideOrId;
  const keys = guide ? guideSourceKeys(guide) : [guideOrId as string];
  return pinnedGameIds.value.some(id => keys.includes(id));
}

export async function togglePinGame(guide: ObsidianGameGuide) {
  const currentlyPinned = isGamePinned(guide);
  if (currentlyPinned) {
    pinnedGameIds.value = pinnedGameIds.value.filter(
      id => !guideSourceKeys(guide).includes(id)
    );
    showToast('已取消置顶', `《${guide.title}》恢复普通排序`);
  } else {
    if (!pinnedGameIds.value.includes(guide.id)) {
      pinnedGameIds.value.unshift(guide.id);
    }
    showToast('已置顶游戏', `《${guide.title}》会优先显示在书库中`);
  }
  kvSet('waypoint_pinned_games', JSON.stringify(pinnedGameIds.value));
}

// 攻略排序规则：'year_desc' (年份最新 2024->2002) | 'year_asc' (年代经典 2002->2024) | 'rating_desc' (评分优先) | 'modified_desc' (最近修改) | 'title_asc' (标题字母)
export const guideSortOrder = ref<'year_desc' | 'year_asc' | 'rating_desc' | 'modified_desc' | 'title_asc' | 'recent'>('year_desc');

export function saveGuideSortOrder() {
  kvSet('waypoint_guide_sort_order', guideSortOrder.value);
}


// 通关游戏自动沉底控制 (默认开启：已通关游戏自动排在列表最末尾)
export const sinkCompletedGames = ref(true);

export function loadGameLibraryPreferences() {
  loadGameExtras();
  galgameCardDensity.value = kvGet('waypoint_galgame_card_density') === 'compact' ? 'compact' : 'large';
  galgameCoverMode.value = kvGet('waypoint_galgame_cover_mode') === 'badge' ? 'badge' : 'poster';
  seriesDisplayMode.value = kvGet('waypoint_series_display_mode') === 'expanded' ? 'expanded' : 'collapsed';
  removedGuideIds.value = kvGetJson<string[]>('waypoint_removed_guides', []);
  const sort = kvGet('waypoint_guide_sort_order');
  guideSortOrder.value = sort && ['year_desc', 'year_asc', 'rating_desc', 'modified_desc', 'title_asc', 'recent'].includes(sort)
    ? sort as typeof guideSortOrder.value : 'year_desc';
  sinkCompletedGames.value = kvGetJson('waypoint_sink_completed', true);
  pinnedGameIds.value = kvGetJson<string[]>('waypoint_pinned_games', []);
}

loadGameLibraryPreferences();

export function toggleSinkCompleted() {
  sinkCompletedGames.value = !sinkCompletedGames.value;
  kvSet('waypoint_sink_completed', JSON.stringify(sinkCompletedGames.value));
  showToast(
    sinkCompletedGames.value ? '已开启【通关沉底】' : '已关闭【通关沉底】',
    sinkCompletedGames.value ? '已通关游戏将自动沉底到列表末尾' : '已恢复同级普通排序'
  );
}

// 计算属性：当前筛选并排序后的单机 & Galgame 攻略列表 (置顶项永远优先置顶，内部依选定规则排序)
export const filteredObsidianGuides = computed(() => {
  let list = [...singleAndGalgameGuides.value];

  if (guideCategoryFilter.value === 'pinned') {
    list = list.filter(g => isGamePinned(g));
  } else if (guideCategoryFilter.value === 'galgame') {
    list = list.filter(g => g.category === 'galgame');
  } else if (guideCategoryFilter.value === 'single') {
    list = list.filter(g => g.category === 'single' || g.category === 'steam');
  } else if (guideCategoryFilter.value === 'unmatched') {
    list = list.filter(g => gameRecords.value.some(r => r.id === g.record_id && r.library?.installations.length && (!r.library.metadata || r.library.matching)));
  }

  // 置顶是显示顺序，游玩状态独立筛选。
  if (guideStatusFilter.value !== 'all') {
    if (guideStatusFilter.value === '在玩') {
      list = list.filter(g => ['在玩', '游玩中'].includes(getEffectiveGameStatus(g)));
    } else {
      list = list.filter(g => getEffectiveGameStatus(g) === guideStatusFilter.value);
    }
  }

  // 3. 正在推线书签过滤
  if (showOnlyBookmarked.value) {
    list = list.filter(g => !!guideProgressMap.value[g.id]);
  }
  list = list.filter(g => g.taxonomy && matchesGameTaxonomy(g.taxonomy, gameTaxonomyFilter.value));

  // 4. 搜索框过滤
  const q = guideSearchQuery.value.trim().toLowerCase();
  if (q) {
    list = list.filter(g =>
      g.title.toLowerCase().includes(q) ||
      g.series?.toLowerCase().includes(q) ||
      (g.original_title && g.original_title.toLowerCase().includes(q)) ||
      (g.aliases && g.aliases.some(alias => alias.toLowerCase().includes(q))) ||
      (g.developer && g.developer.toLowerCase().includes(q)) ||
      (g.publisher && g.publisher.toLowerCase().includes(q)) ||
      g.taxonomy?.staff.some(s => `${s.name} ${s.role}`.toLowerCase().includes(q)) ||
      (g.year && g.year.includes(q)) ||
      (g.tags && g.tags.some(t => t.toLowerCase().includes(q))) ||
      (g.genres && g.genres.some(gen => gen.toLowerCase().includes(q)))
    );
  }

  // 5. 排序核心算法：置顶优先，其次通关沉底，再按年份/评分/修改时间排序
  list.sort((a, b) => {
    // 1. 置顶绝对第一优先
    const aPinned = isGamePinned(a);
    const bPinned = isGamePinned(b);
    if (aPinned && !bPinned) return -1;
    if (!aPinned && bPinned) return 1;

    // 2. 通关沉底 (未置顶时，已通关游戏自动沉底到末尾)
    if (sinkCompletedGames.value) {
      const aDone = getEffectiveGameStatus(a) === '已通关';
      const bDone = getEffectiveGameStatus(b) === '已通关';
      if (!aDone && bDone) return -1;
      if (aDone && !bDone) return 1;
    }

    if (guideSortOrder.value === 'recent') {
      return Math.max(0, ...gameSourceIds(b).map(id => recentGames.value[id] || 0))
        - Math.max(0, ...gameSourceIds(a).map(id => recentGames.value[id] || 0));
    }
    if (guideSortOrder.value === 'year_desc') {
      const yA = Number(a.year) || 0;
      const yB = Number(b.year) || 0;
      if (yB !== yA) return yB - yA;
      return (b.release_date || '').localeCompare(a.release_date || '');
    }
    if (guideSortOrder.value === 'year_asc') {
      const yA = Number(a.year) || 9999;
      const yB = Number(b.year) || 9999;
      if (yA !== yB) return yA - yB;
      return (a.release_date || '').localeCompare(b.release_date || '');
    }
    if (guideSortOrder.value === 'rating_desc') {
      const rA = Math.max(a.rating_vndb || 0, a.rating_bangumi || 0, a.rating || 0);
      const rB = Math.max(b.rating_vndb || 0, b.rating_bangumi || 0, b.rating || 0);
      if (rB !== rA) return rB - rA;
      return (b.year || '').localeCompare(a.year || '');
    }
    if (guideSortOrder.value === 'modified_desc') {
      return (b.modified || '').localeCompare(a.modified || '');
    }
    if (guideSortOrder.value === 'title_asc') {
      return a.title.localeCompare(b.title, 'zh-Hans-CN', { numeric: true });
    }
    return 0;
  });

  return list;
});

// 单机与 Galgame 专区分类统计数据
export const guideStats = computed(() => {
  const total = singleAndGalgameGuides.value.length;
  const galgame = singleAndGalgameGuides.value.filter(g => g.category === 'galgame').length;
  const single = singleAndGalgameGuides.value.filter(g => g.category === 'single' || g.category === 'steam').length;
  const bookmarked = singleAndGalgameGuides.value.filter(g => !!guideProgressMap.value[g.id]).length;
  const pinned = singleAndGalgameGuides.value.filter(g => isGamePinned(g)).length;
  const completed = singleAndGalgameGuides.value.filter(g => getEffectiveGameStatus(g) === '已通关').length;
  const unmatched = gameRecords.value.filter(r => r.library?.installations.length && (!r.library.metadata || r.library.matching)).length;
  return { total, galgame, single, bookmarked, pinned, completed, unmatched };
});

// 新增与编辑共用一个草稿，编辑始终保留原 ID、书签与资料关联。
export const editingGameId = ref<string | null>(null);
export const editingLibraryId = ref<string | null>(null);
export type GameEditorSection = 'general' | 'metadata' | 'taxonomy' | 'scaling' | 'cover' | 'background' | 'progress' | 'removal';
export const gameEditorSection = ref<GameEditorSection>('general');
let initialEditorStatus: GameRecord['status'] = 'playing';

export function openGameRecordEditor(guide?: ObsidianGameGuide, section: GameEditorSection = 'general') {
  const record = guide ? gameRecords.value.find(item => item.id === (guide.record_id || guide.id)) : undefined;
  editingGameId.value = record?.id || null;
  editingLibraryId.value = guide?.id || null;
  const effectiveStatus = getEffectiveGameStatus(guide);
  state.newGame = record
    ? { ...record, guide_id: record.guide_id ?? (guide?.file_path ? guide.id : undefined), save_path: record.save_path || '', notes: record.notes || '', poster: record.poster || '', poster_position: { ...(record.poster_position || { x: 50, y: 50 }) }, series: '', series_order: undefined }
    : {
      title: guide?.title || '', category: guide?.category === 'single' || guide?.category === 'steam' ? guide.category : 'galgame',
      status: guide ? effectiveStatus === '已通关' ? 'completed' : ['在玩', '游玩中'].includes(effectiveStatus) ? 'playing' : 'backlog' : 'playing',
      progress: guide?.progress_note || '', rating: guide?.rating_personal || 0, save_path: '', notes: guide?.personal_notes || '', poster: '',
      poster_position: { x: 50, y: 50 }, guide_id: guide?.file_path ? guide.id : undefined,
      series: '', series_order: undefined,
    };
  state.newGame.series = guide?.series || '';
  state.newGame.series_order = guide?.series_order;
  if (guide) state.newGame.status = effectiveStatus === '已通关' ? 'completed' : ['在玩', '游玩中'].includes(effectiveStatus) ? 'playing' : 'backlog';
  initialEditorStatus = state.newGame.status;
  gameEditorSection.value = section;
  state.showAddGameModal = true;
}

export async function handleSaveGameRecord(changes: {
  metadata?: GameMetadata;
  background?: Pick<GameLibraryDetails, 'background' | 'background_position'>;
  curation?: GameCuration;
  scaling?: GameLibraryDetails['scaling'];
} = {}) {
  if (!state.newGame.title.trim()) return;
  const editing = editingGameId.value;
  const collection = { name: state.newGame.series.trim(), order: state.newGame.series_order ? Number(state.newGame.series_order) : undefined };
  if (collection.order !== undefined && (!Number.isSafeInteger(collection.order) || collection.order < 1 || collection.order > 4_294_967_295)) throw new Error('合集顺序需为正整数');
  if (!collection.name) collection.order = undefined;
  function editedLibrary(current: GameLibraryDetails = { installations: [] }) {
    return { ...(changes.metadata ? withGameMetadata(current, changes.metadata) : current), collection, ...changes.background,
      ...(changes.curation ? { curation: changes.curation } : {}), ...(changes.scaling ? { scaling: changes.scaling } : {}) };
  }
  let record: GameRecord = {
    id: editing || `g_${crypto.randomUUID()}`,
    title: state.newGame.title.trim(),
    category: state.newGame.category,
    status: state.newGame.status,
    progress: state.newGame.progress.trim(),
    rating: state.newGame.rating ?? 0,
    save_path: state.newGame.save_path || undefined,
    notes: state.newGame.notes || undefined,
    poster: state.newGame.poster.trim() || undefined,
    poster_position: { ...state.newGame.poster_position },
    guide_id: state.newGame.guide_id,
    library: editedLibrary(gameRecords.value.find(item => item.id === editing)?.library),
  };
  const candidate = buildGameLibrary(libraryGuides.value, [...gameRecords.value.filter(item => item.id !== record.id), record]);
  if (record.guide_id && libraryGuides.value.some(game => game.id === record.guide_id)
    && !candidate.some(game => game.id === record.guide_id && game.record_id === record.id)) {
    throw new Error('这篇攻略已关联其他档案或当前无法读取，请选择其他攻略');
  }
  if (editing) record = await updateGameRecord(editing, current => ({ ...record, library: editedLibrary(current.library) }));
  else await saveGameRecord(record, true);
  gameRecords.value = editing
    ? gameRecords.value.map(item => item.id === editing ? record : item)
    : [...gameRecords.value, record];
  delete failedPosters.value[record.id];
  if (editingLibraryId.value) delete failedPosters.value[editingLibraryId.value];
  state.showAddGameModal = false;
  const updated = singleAndGalgameGuides.value.find(item => item.record_id === record.id);
  if (updated) delete failedPosters.value[updated.id];
  if (updated && state.newGame.status !== initialEditorStatus) {
    await setGameStatus(updated, record.status === 'playing' ? '在玩' : record.status === 'completed' ? '已通关' : '未开始');
  }
  void rememberGame(updated?.id || record.id).catch(() => showToast('最近游玩顺序未保存', '游戏档案已保存，稍后重新打开可更新顺序'));
  if (selectedGuide.value?.id === editingLibraryId.value) {
    if (updated) await handleOpenGuideReader(updated);
  }
  showToast(editing ? '游戏档案已更新' : '游戏档案已建立', `【${record.title}】已保存到本机`);
}

// 扫描并加载 Obsidian 游戏攻略库
export async function loadObsidianGuides(showFeedback = false) {
  if (!obsidianVaultPath.value) {
    obsidianGameGuides.value = [];
    if (showFeedback) {
      showToast('未配置 Obsidian 路径', '请先在【系统设置】中绑定你的 Obsidian 仓库');
    }
    return;
  }
  isScanningGuides.value = true;
  try {
    const list = await scanObsidianGameGuides(obsidianVaultPath.value);
    // 注入权威真实封面与评分，消除 Obsidian 模板复制产生的脏 URL
    obsidianGameGuides.value = list.map(g => sanitizeAndEnrichGuide(g));
    if (showFeedback) {
      showToast('攻略库扫描完成', `已实时同步 ${list.length} 篇游戏与视觉小说攻略`);
    }
  } catch (err: any) {
    console.warn('Failed to scan obsidian game guides:', err);
    if (showFeedback) {
      showToast('扫描提示', (err ? describeError(err).message : '未能读取 Obsidian 攻略目录'));
    }
  } finally {
    isScanningGuides.value = false;
  }
}
