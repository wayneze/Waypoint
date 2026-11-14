// 攻略阅读器、路线与书签。
import { describeError } from '../../services/errors';
import { ref, computed, nextTick } from 'vue';
import { type GuideProgressBookmark, type ObsidianGameGuide } from '../../types';
import { openInExplorer, openInObsidian, readObsidianFile, updateGuideFrontmatter } from '../../services/systemApi';
import { kvGet, kvSet } from '../../services/kv';
import { progressMap, progressSet } from '../../services/progress';
import { type ParsedGuide, parseObsidianGuide } from '../../services/guideParser';
import { gameRecords, showToast } from '../core';
import { getEffectiveGameStatus, setGameStatus } from './status';
import { linksForGames, rememberGame, type GameLink } from './extras';

export const selectedGuide = ref<ObsidianGameGuide | null>(null);
export const showGuideReader = ref<boolean>(false);
export const isLoadingGuideContent = ref<boolean>(false);
export const guideContent = ref<string>('');
export const guideReadError = ref('');
let guideRequest = 0;
export const GUIDE_ROUTE_FOCUS_OVERVIEW = '__overview__';
export const guideRouteFocusMap = ref<Record<string, string>>({});

// 攻略阅读器交互模式：'sidecar' (侧边伴游抽屉 - 默认推荐，左侧日常任务与网游完全可交互) | 'modal' (居中沉浸大屏)
export const readerLayoutMode = ref<'sidecar' | 'modal'>('sidecar');
// 是否最小化至右下角画中画胶囊 (不占用视野，随时可复原)
export const isGuideReaderMinimized = ref<boolean>(false);

export function toggleReaderLayoutMode() {
  readerLayoutMode.value = readerLayoutMode.value === 'sidecar' ? 'modal' : 'sidecar';
  kvSet('waypoint_guide_layout_mode', readerLayoutMode.value);
  showToast(
    '攻略布局已切换',
    readerLayoutMode.value === 'sidecar' ? '已切换为侧边模式' : '已切换为居中模式'
  );
}

export function minimizeGuideReader() {
  isGuideReaderMinimized.value = true;
}

export function restoreGuideReader() {
  isGuideReaderMinimized.value = false;
  showGuideReader.value = true;
}

// 侧边伴游抽屉宽度 (支持左侧边沿左右拖拽无极调节与宽屏/标准预设，默认 720px)
export const drawerWidth = ref(900);

export function loadGuideReaderPreferences() {
  readerLayoutMode.value = kvGet('waypoint_guide_layout_mode') === 'modal' ? 'modal' : 'sidecar';
  drawerWidth.value = Math.max(520, Math.min(1280, Number(kvGet('waypoint_guide_drawer_width')) || 900));
}

export function startDrawerResize(e: MouseEvent) {
  const startX = e.clientX;
  const startW = drawerWidth.value;

  function onMouseMove(ev: MouseEvent) {
    // 抽屉停靠在右侧，鼠标向左拖 (startX - ev.clientX > 0) 增加抽屉宽度
    const delta = startX - ev.clientX;
    const maxW = Math.max(520, window.innerWidth - 80);
    const newW = Math.min(maxW, Math.max(520, startW + delta));
    drawerWidth.value = newW;
  }

  function onMouseUp() {
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('mouseup', onMouseUp);
    kvSet('waypoint_guide_drawer_width', drawerWidth.value.toString());
  }

  window.addEventListener('mousemove', onMouseMove);
  window.addEventListener('mouseup', onMouseUp);
}

export const guideProgressMap = ref<Record<string, GuideProgressBookmark>>({});
export const showOnlyBookmarked = ref<boolean>(false);


// 当前打开攻略的结构化解析结果
export const parsedCurrentGuide = computed<ParsedGuide>(() => {
  return parseObsidianGuide(guideContent.value);
});

export const currentGuideRouteList = computed(() => {
  const list: Array<{ title: string; blockIdx: number; section?: string }> = [];
  parsedCurrentGuide.value.blocks.forEach((b, i) => {
    if (b.type === 'callout' && b.callout) {
      list.push({
        title: b.callout.title,
        blockIdx: i,
        section: b.callout.sectionHeading
      });
    }
  });
  return list;
});

export const currentGuideRouteFocus = computed(() => {
  const id = selectedGuide.value?.id;
  if (!id) return GUIDE_ROUTE_FOCUS_OVERVIEW;
  return guideRouteFocusMap.value[id] || GUIDE_ROUTE_FOCUS_OVERVIEW;
});

/** 没有路线折叠时，概览按章节排，不再把整篇落成纯文本。 */
export const guideHasRoutes = computed(() => currentGuideRouteList.value.length > 0);

export function guideSectionLabel(level: number): string {
  if (level <= 1) return '篇';
  if (level === 2) return '章';
  return '节';
}

function persistGuideRouteFocus() {
  const json = JSON.stringify(guideRouteFocusMap.value);
  kvSet('waypoint_guide_route_focus', json);
}

export function loadGuideRouteFocus() {
  try {
    const raw = kvGet('waypoint_guide_route_focus');
    if (raw) {
      const parsed = JSON.parse(raw);
      if (parsed && typeof parsed === 'object') {
        guideRouteFocusMap.value = parsed;
      }
    }
  } catch (e) {
    console.error('Failed to load guide route focus:', e);
  }
}

function findCalloutByFocusKey(key: string) {
  if (!key || key === GUIDE_ROUTE_FOCUS_OVERVIEW) return undefined;
  return currentGuideRouteList.value.find(r =>
    r.title === key || matchRouteTitles(r.title, key)
  );
}

export function isGuideRouteFocused(title: string) {
  const focus = currentGuideRouteFocus.value;
  if (focus === GUIDE_ROUTE_FOCUS_OVERVIEW) return false;
  return title === focus || matchRouteTitles(title, focus);
}

export function restoreGuideRouteFocus(guideId: string) {
  const saved = guideRouteFocusMap.value[guideId];
  if (saved === GUIDE_ROUTE_FOCUS_OVERVIEW) return;
  if (saved && findCalloutByFocusKey(saved)) return;

  const bm = guideProgressMap.value[guideId];
  if (bm?.blockIdx !== undefined) {
    const block = parsedCurrentGuide.value.blocks[bm.blockIdx];
    if (block?.type === 'callout' && block.callout) {
      guideRouteFocusMap.value = { ...guideRouteFocusMap.value, [guideId]: block.callout.title };
      persistGuideRouteFocus();
      return;
    }
    if (bm.routeTitle) {
      const hit = findCalloutByFocusKey(bm.routeTitle);
      if (hit) {
        guideRouteFocusMap.value = { ...guideRouteFocusMap.value, [guideId]: hit.title };
        persistGuideRouteFocus();
        return;
      }
    }
  }

  if (saved) {
    const next = { ...guideRouteFocusMap.value };
    delete next[guideId];
    guideRouteFocusMap.value = next;
    persistGuideRouteFocus();
  }
}

export function selectGuideRoute(title: string) {
  const id = selectedGuide.value?.id;
  if (!id) return;
  const key = title === GUIDE_ROUTE_FOCUS_OVERVIEW
    ? GUIDE_ROUTE_FOCUS_OVERVIEW
    : (findCalloutByFocusKey(title)?.title || title);
  if (guideRouteFocusMap.value[id] === key) {
    nextTick(() => {
      document.getElementById('guide-reader-scroll')?.scrollTo({ top: 0, behavior: 'smooth' });
    });
    return;
  }
  guideRouteFocusMap.value = { ...guideRouteFocusMap.value, [id]: key };
  persistGuideRouteFocus();
  nextTick(() => {
    document.getElementById('guide-reader-scroll')?.scrollTo({ top: 0 });
  });
}


export const completedRoutesMap = ref<Record<string, string[]>>({});

const ROUTES_SCOPE = 'guide.routes';
const PROGRESS_SCOPE = 'guide.progress';

export function loadCompletedRoutes() {
  completedRoutesMap.value = progressMap<string[]>(ROUTES_SCOPE);
}

export function extractNumeral(s: string): string {
  if (!s) return '';
  const m1 = s.match(/[ⅠⅡⅢⅣⅤⅥⅦⅧⅨⅩ]+/);
  if (m1) {
    const map: Record<string, string> = { 'Ⅰ':'1','Ⅱ':'2','Ⅲ':'3','Ⅳ':'4','Ⅴ':'5','Ⅵ':'6','Ⅶ':'7','Ⅷ':'8','Ⅸ':'9','Ⅹ':'10' };
    return map[m1[0]] || m1[0];
  }
  const m2 = s.match(/\b([0-9]+)\b/);
  if (m2) return m2[1];
  const m3 = s.match(/\b(I|II|III|IV|V|VI|VII|VIII|IX|X)\b/i);
  if (m3) {
    const map: Record<string, string> = { 'i':'1','ii':'2','iii':'3','iv':'4','v':'5','vi':'6','vii':'7','viii':'8','ix':'9','x':'10' };
    return map[m3[1].toLowerCase()] || m3[1];
  }
  return '';
}

export function normalizeRouteTitle(name: string): string {
  if (!name) return '';
  return name
    .replace(/[｜|]\s*折叠.*$/i, '')
    .replace(/[（\(]\s*折叠[^\)）]*[）\)]/gi, '')
    .replace(/[（\(][^）\)]*(?:视点|分支|选项|收录|救赎|结局|True|BE|GOOD|BAD)[^）\)]*[）\)]/gi, '')
    .replace(/（.*?）|\(.*?\)|【.*?】|\[.*?\]/g, ' ')
    .replace(/[线篇章条路线RouteENDEnd分支]+$/i, '')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase();
}

export function cleanRouteTitle(title: string): string {
  if (!title) return '';
  return title
    .replace(/[｜|]\s*折叠.*$/i, '')
    .replace(/[（\(]\s*折叠[^\)）]*[）\)]/gi, '')
    .replace(/[（\(][^）\)]*(?:视点|分支|选项|收录|救赎|结局)[^）\)]*[）\)]/gi, '')
    .replace(/[（\(].*?[）\)]/g, '')
    .replace(/[【\[].*?[】\]]/g, '')
    .trim();
}

export function matchRouteTitles(a: string, b: string): boolean {
  if (!a || !b) return false;
  const trimA = a.trim();
  const trimB = b.trim();
  if (trimA === trimB) return true;

  const na = normalizeRouteTitle(a);
  const nb = normalizeRouteTitle(b);
  if (!na || !nb) return false;
  if (na === nb) return true;

  // 严格数字/罗马数字一致性校验：若双方都包含编号，必须严格相同，严禁跨数字混淆
  const numA = extractNumeral(a);
  const numB = extractNumeral(b);
  if (numA && numB && numA !== numB) return false;

  // CJK 中文/日文前后缀严格匹配 (如 天女目瑛 vs 瑛线)
  const isCjkA = /[\u4e00-\u9fa5]/.test(na);
  const isCjkB = /[\u4e00-\u9fa5]/.test(nb);
  if (isCjkA && isCjkB) {
    if (na.endsWith(nb) || nb.endsWith(na)) {
      if (numA && numB && numA !== numB) return false;
      return true;
    }
    if (na.startsWith(nb) || nb.startsWith(na)) {
      if (numA && numB && numA !== numB) return false;
      return true;
    }
  }

  // 英文/拉丁路线标题匹配：首个实质词对齐，彻底杜绝任意子串乱包含
  const wordsA = na.split(/\s+/).filter(w => w.length >= 3 && !['the','and','for','with','its'].includes(w));
  const wordsB = nb.split(/\s+/).filter(w => w.length >= 3 && !['the','and','for','with','its'].includes(w));
  if (wordsA.length > 0 && wordsB.length > 0) {
    if (wordsA[0] === wordsB[0] && wordsA[0].length >= 4) {
      if (numA && numB && numA !== numB) return false;
      return true;
    }
  }

  return false;
}

export function isRouteCompleted(guideId: string, routeTitle: string, altTitle?: string): boolean {
  const routes = completedRoutesMap.value[guideId];
  if (!routes || !Array.isArray(routes)) return false;
  return routes.some(r => matchRouteTitles(r, routeTitle) || (altTitle ? matchRouteTitles(r, altTitle) : false));
}

export async function toggleRouteCompleted(guide: ObsidianGameGuide, routeTitle: string, altTitle?: string) {
  const guideId = guide.id;
  const clean = cleanRouteTitle(routeTitle);
  const cleanAlt = altTitle ? cleanRouteTitle(altTitle) : '';
  if (!completedRoutesMap.value[guideId]) {
    completedRoutesMap.value[guideId] = [];
  }

  // 判定当前路线是否已完成（检查已打卡列表）
  const idx = completedRoutesMap.value[guideId].findIndex(r => 
    matchRouteTitles(r, clean) || (cleanAlt ? matchRouteTitles(r, cleanAlt) : false)
  );

  // 同时检查当前正文表格中该路线当前是否已经为“已”
  let tableAlreadyDone = false;
  if (guideContent.value) {
    const tLines = guideContent.value.split(/\r?\n/);
    let curStatusIdx = 2;
    let curRouteIdx = 1;
    let inTable = false;
    for (let i = 0; i < tLines.length; i++) {
      const tl = tLines[i].trim();
      if (tl.startsWith('|') && tl.endsWith('|')) {
        if (i + 1 < tLines.length && tLines[i + 1].trim().startsWith('|') && tLines[i + 1].includes('---')) {
          const parts = tLines[i].split('|');
          const sIdx = parts.findIndex((p, idx) => {
            if (idx <= 0 || idx >= parts.length - 1) return false;
            const s = p.trim().toLowerCase();
            return s === '完成度' || s === '状态' || s === '进度' || s === '完成' || s === '通关' || s === '达成' || s === 'status' || s === 'clear';
          });
          if (sIdx !== -1) {
            curStatusIdx = sIdx;
            const rIdx = parts.findIndex((p, idx) => {
              if (idx <= 0 || idx >= parts.length - 1) return false;
              const s = p.trim().toLowerCase();
              return /路线|线路|结局|女主|章节|角色|分支|条目|卷|名称|标题|route|chapter|ending|title/i.test(s);
            });
            curRouteIdx = rIdx !== -1 ? rIdx : (curStatusIdx === 1 ? 2 : 1);
          }
          inTable = true;
          i++; // 跳过分隔行
          continue;
        }

        if (inTable && !tl.includes('---')) {
          const parts = tLines[i].split('|');
          if (curRouteIdx < parts.length && curStatusIdx < parts.length) {
            const rName = parts[curRouteIdx].trim();
            if (matchRouteTitles(rName, clean) || (cleanAlt && matchRouteTitles(rName, cleanAlt))) {
              const val = parts[curStatusIdx].trim().toLowerCase();
              if (val === '已' || val === '已通关' || val === '✓' || val === 'yes' || val === '100%') {
                tableAlreadyDone = true;
                break;
              }
            }
          }
        }
      } else {
        inTable = false;
      }
    }
  }

  const isCurrentlyDone = idx !== -1 || tableAlreadyDone;
  const isNowCompleted = !isCurrentlyDone;

  if (isNowCompleted) {
    if (idx === -1) {
      completedRoutesMap.value[guideId].push(clean);
    }
    showToast('路线已通关 ✓', `《${guide.title}》· 已完成【${clean}】路线！`);
  } else {
    completedRoutesMap.value[guideId] = completedRoutesMap.value[guideId].filter(r => 
      !matchRouteTitles(r, clean) && (!cleanAlt || !matchRouteTitles(r, cleanAlt))
    );
    showToast('已取消完成', `已将【${clean}】路线重置为未通关`);
  }

  progressSet(ROUTES_SCOPE, guideId, completedRoutesMap.value[guideId]?.length ? completedRoutesMap.value[guideId] : undefined);

  // 同步更新在内存中的 guideContent.value，严格基于表头与数据行边界更新，使正文表格立即联动
  if (guideContent.value) {
    const lines = guideContent.value.split(/\r?\n/);
    let modified = false;
    let statusColIdx = 2;
    let routeColIdx = 1;
    let inTable = false;

    for (let i = 0; i < lines.length; i++) {
      const l = lines[i].trim();
      if (l.startsWith('|') && l.endsWith('|')) {
        // 只有下一行是分隔线时，当前行才是表头
        if (i + 1 < lines.length && lines[i + 1].trim().startsWith('|') && lines[i + 1].includes('---')) {
          const parts = lines[i].split('|');
          const foundStatusIdx = parts.findIndex((p, pIdx) => {
            if (pIdx <= 0 || pIdx >= parts.length - 1) return false;
            const s = p.trim().toLowerCase();
            return s === '完成度' || s === '状态' || s === '进度' || s === '完成' || s === '通关' || s === '达成' || s === 'status' || s === 'clear';
          });
          if (foundStatusIdx !== -1) {
            statusColIdx = foundStatusIdx;
            const foundRouteIdx = parts.findIndex((p, pIdx) => {
              if (pIdx <= 0 || pIdx >= parts.length - 1) return false;
              const s = p.trim().toLowerCase();
              return /路线|线路|结局|女主|章节|角色|分支|条目|卷|名称|标题|route|chapter|ending|title/i.test(s);
            });
            routeColIdx = foundRouteIdx !== -1 ? foundRouteIdx : (statusColIdx === 1 ? 2 : 1);
          }
          inTable = true;
          i++; // 跳过分隔行
          continue;
        }

        // 数据行
        if (inTable && !l.includes('---')) {
          const parts = lines[i].split('|');
          if (routeColIdx < parts.length && statusColIdx < parts.length) {
            const rowRoute = parts[routeColIdx].trim();
            if (rowRoute && (matchRouteTitles(rowRoute, clean) || (cleanAlt && matchRouteTitles(rowRoute, cleanAlt)))) {
              const curCell = parts[statusColIdx];
              const curVal = curCell.trim();
              if (isNowCompleted) {
                const targetText = curVal.length > 2 ? '已通关' : '已';
                parts[statusColIdx] = curVal ? curCell.replace(curVal, targetText) : ` ${targetText} `;
                lines[i] = parts.join('|');
                modified = true;
              } else {
                const targetText = curVal.length > 2 ? '未通关' : '未';
                parts[statusColIdx] = curVal ? curCell.replace(curVal, targetText) : ` ${targetText} `;
                lines[i] = parts.join('|');
                modified = true;
              }
            }
          }
        }
      } else {
        inTable = false;
      }
    }
    if (modified) {
      guideContent.value = lines.join('\n');
    }
  }

  // 同步写回 Obsidian routes.cleared 以及 Markdown 表格行
  const clearedCount = completedRoutesMap.value[guideId].length;
  if (guide.file_path) {
    updateGuideFrontmatter(guide.file_path, undefined, clearedCount, clean, isNowCompleted);
  }

  // 全路线通关检测与自动联动
  const totalRoutes = guide.resource_url ? 0 : guide.routes_total || 0;
  if (isNowCompleted && totalRoutes > 0 && clearedCount >= totalRoutes) {
    if (getEffectiveGameStatus(guide) !== '已通关') {
      await setGameStatus(guide, '已通关');
      showToast('🎉 全路线通关达成！', `已完成《${guide.title}》全部 ${totalRoutes} 条路线，已自动标记为【已通关】！`);
    }
  }
}

export function getTableStatusColIdx(headers: string[]): number {
  if (!headers || !Array.isArray(headers)) return -1;
  return headers.findIndex(h => {
    const s = h.trim().toLowerCase();
    return /完成度|状态|进度|通关|达成|完成|status|clear|state/i.test(s);
  });
}

export function getTableRouteColIdx(headers: string[]): number {
  if (!headers || !Array.isArray(headers)) return 0;
  const idx = headers.findIndex(h => {
    const s = h.trim().toLowerCase();
    return /路线|线路|结局|女主|章节|角色|分支|条目|卷|名称|标题|route|chapter|ending|title/i.test(s);
  });
  if (idx !== -1) return idx;
  const statusIdx = getTableStatusColIdx(headers);
  return statusIdx === 0 ? 1 : 0;
}

export function isTableRowRouteDone(headers: string[], row: string[], guideId: string): boolean {
  if (!row || !Array.isArray(row)) return false;
  const routeCol = getTableRouteColIdx(headers);
  const routeName = row[routeCol] || '';

  // 1. 优先以实时响应式 completedRoutesMap 集合为权威依据
  if (isRouteCompleted(guideId, routeName)) {
    return true;
  }

  // 2. 检查单元格原始文字
  const statusCol = getTableStatusColIdx(headers);
  if (statusCol !== -1 && row[statusCol]) {
    const s = row[statusCol].trim().toLowerCase();
    if (s === '已' || s === '已通关' || s === '✓' || s === 'yes' || s === '100%') {
      return true;
    }
  }

  return false;
}


export function handleOpenResourceReader(game: ObsidianGameGuide, link: GameLink) {
  const owner = game.record_id || game.id;
  return handleOpenGuideReader({ ...game, id: `resource:${owner}:${link.url}`, title: link.title, file_path: '', resource_url: link.url, resource_game_id: owner, record_id: game.record_id });
}
export async function handleOpenGuideReader(guide: ObsidianGameGuide) {
  const request = ++guideRequest;
  selectedGuide.value = guide;
  showGuideReader.value = true;
  isGuideReaderMinimized.value = false;
  isLoadingGuideContent.value = true;
  guideContent.value = '';
  guideReadError.value = '';

  try {
    const record = gameRecords.value.find(item => item.id === (guide.record_id || guide.id));
    const stored = guide.resource_url ? linksForGames([guide.resource_game_id || '', guide.record_id || '']).find(link => link.url === guide.resource_url)?.article : undefined;
    if (guide.resource_url && !stored) throw new Error('这份资料只收藏了来源，尚未保存正文。');
    const raw = stored ? stored.markdown : guide.file_path
      ? await readObsidianFile(guide.file_path)
      : ['# 游玩记录', '## 当前进度', record?.progress || '尚未记录进度', '## 备忘', record?.notes || '尚未填写备忘'].join('\n\n');
    if (request !== guideRequest) return;
    guideContent.value = raw;
    void rememberGame(guide.resource_game_id || guide.id).catch(() => showToast('最近游玩顺序未保存', '内容已打开，稍后重新打开可更新顺序'));

    // 扫描 Markdown 表格中已有 | 已 | 标记的路线，与本地 completedRoutesMap 进行精准对账合并
    if (!completedRoutesMap.value[guide.id]) {
      completedRoutesMap.value[guide.id] = [];
    }
    const rawLines = raw.split(/\r?\n/);
    let tableStatusColIdx = 2;
    let tableRouteColIdx = 1;
    let inTable = false;
    for (let i = 0; i < rawLines.length; i++) {
      const trimmed = rawLines[i].trim();
      if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
        // 只有紧随其后是 | --- | 的行，才是表头行！绝不受备注中“通关”字样的干扰
        if (i + 1 < rawLines.length && rawLines[i + 1].trim().startsWith('|') && rawLines[i + 1].includes('---')) {
          const parts = rawLines[i].split('|');
          const foundStatusIdx = parts.findIndex((p, pIdx) => {
            if (pIdx <= 0 || pIdx >= parts.length - 1) return false;
            const s = p.trim().toLowerCase();
            return s === '完成度' || s === '状态' || s === '进度' || s === '完成' || s === '通关' || s === '达成' || s === 'status' || s === 'clear';
          });
          if (foundStatusIdx !== -1) {
            tableStatusColIdx = foundStatusIdx;
            const foundRouteIdx = parts.findIndex((p, pIdx) => {
              if (pIdx <= 0 || pIdx >= parts.length - 1) return false;
              const s = p.trim().toLowerCase();
              return /路线|线路|结局|女主|章节|角色|分支|条目|卷|名称|标题|route|chapter|ending|title/i.test(s);
            });
            tableRouteColIdx = foundRouteIdx !== -1 ? foundRouteIdx : (tableStatusColIdx === 1 ? 2 : 1);
          }
          inTable = true;
          i++; // 跳过分隔行
          continue;
        }

        if (inTable && !trimmed.includes('---')) {
          const parts = rawLines[i].split('|');
          if (tableStatusColIdx < parts.length && tableRouteColIdx < parts.length) {
            const statusVal = parts[tableStatusColIdx].trim().toLowerCase();
            if (statusVal === '已' || statusVal === '已通关' || statusVal === '✓' || statusVal === 'yes' || statusVal === '100%') {
              const routeCell = parts[tableRouteColIdx].trim();
              const clean = cleanRouteTitle(routeCell);
              if (clean && !completedRoutesMap.value[guide.id].some(r => matchRouteTitles(r, clean))) {
                completedRoutesMap.value[guide.id].push(clean);
              }
            }
          }
        }
      } else {
        inTable = false;
      }
    }

    restoreGuideRouteFocus(guide.id);
  } catch (err: any) {
    if (request !== guideRequest) return;
    guideReadError.value = err ? describeError(err).message : '文件不存在或无读取权限';
  } finally {
    if (request === guideRequest) isLoadingGuideContent.value = false;
  }
}

// 在系统默认程序 (Obsidian) 中打开
export async function handleOpenInObsidian(guide: ObsidianGameGuide) {
  try {
    await openInObsidian(guide.file_path);
    showToast('已在外部打开', `正在启动 Obsidian: ${guide.title}`);
  } catch (err: any) {
    await openInExplorer(guide.file_path);
  }
}

// 攻略书签持久化管理
export function loadGuideBookmarks() {
  guideProgressMap.value = progressMap<GuideProgressBookmark>(PROGRESS_SCOPE);
}

export async function setGuideProgress(guideId: string, routeTitle: string, blockIdx?: number, lineIdx?: number, stepText?: string) {
  if (selectedGuide.value?.id === guideId) {
    selectGuideRoute(routeTitle);
  }
  const cleanTitle = routeTitle
    .replace(/｜折叠.*/, '')
    .replace(/（折叠）/, '')
    .replace(/（.*?）|\(.*?\)/g, '')
    .trim();
  const cleanStep = stepText
    ? stepText.replace(/<[^>]+>/g, '').replace(/^\d+\.\s*/, '').trim().slice(0, 30)
    : undefined;

  const bookmark: GuideProgressBookmark = {
    guideId,
    routeTitle: cleanTitle,
    blockIdx,
    lineIdx,
    stepText: cleanStep,
    updatedAt: new Date().toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit' })
  };

  guideProgressMap.value[guideId] = bookmark;
  progressSet(PROGRESS_SCOPE, guideId, bookmark);
  showToast('攻略进度已标记', `已记录进度：${cleanTitle}${cleanStep ? ` · ${cleanStep}` : ''}`);
}

export async function clearGuideProgress(guideId: string) {
  delete guideProgressMap.value[guideId];
  progressSet(PROGRESS_SCOPE, guideId, undefined);
  showToast('进度标记已清除', '已重置该攻略的书签');
}

export function jumpToBookmarkedProgress(guideId: string) {
  const bm = guideProgressMap.value[guideId];
  if (!bm) return;
  if (bm.blockIdx !== undefined) {
    const block = parsedCurrentGuide.value.blocks[bm.blockIdx];
    if (block?.type === 'callout' && block.callout) {
      selectGuideRoute(block.callout.title);
    } else if (bm.routeTitle) {
      selectGuideRoute(bm.routeTitle);
    }
    nextTick(() => {
      setTimeout(() => {
        if (selectedGuide.value?.id !== guideId || !showGuideReader.value) return;
        const el = bm.lineIdx !== undefined
          ? document.getElementById(`guide-line-${bm.blockIdx}-${bm.lineIdx}`) || document.getElementById(`guide-block-${bm.blockIdx}`)
          : document.getElementById(`guide-block-${bm.blockIdx}`);
        if (el) {
          el.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }, 80);
    });
  } else if (bm.routeTitle) {
    selectGuideRoute(bm.routeTitle);
  }
}
