// 万象记事。
import { optimistic } from '../services/optimistic';
import { ref, computed } from 'vue';
import { type CustomMemo } from '../types';
import { lastDailyBoundary, lastWeeklyBoundary, parseResetClock } from '../services/cycleReset';
import { isWeekdayActive, noteStepProgress, parseWeekdays, toggleNoteStep, weekdayLabel } from '../services/memoNote';
import { addCustomMemo, deleteCustomMemo, toggleCustomMemo, updateCustomMemo } from '../services/api';
import { kvSet } from '../services/kv';
import { type GameStamp, gameMetaOf } from '../services/games';
import { gameResetEpoch, newTaskId, showToast } from './core';

// 万象记事与周期追踪状态
export const customMemos = ref<CustomMemo[]>([]);
export const selectedMemoTag = ref<string>('all');
export const memoQuery = ref('');
export type MemoInput = Pick<CustomMemo, 'title' | 'tag' | 'cycle_type' | 'cycle_val' | 'note' | 'character_id' | 'has_progress' | 'target_progress'>;

// 计算属性：动态标签池
export const availableMemoTags = computed(() => {
  const defaultTags = ['逆水寒', '剑网3', '光遇', '单机', '生活', '通用'];
  const customTags = customMemos.value.map(m => m.tag).filter(t => !!t && !defaultTags.includes(t));
  return [...defaultTags, ...Array.from(new Set(customTags))];
});

export const filteredMemos = computed(() => {
  const query = memoQuery.value.trim().toLowerCase();
  return customMemos.value.filter(memo => {
    if (selectedMemoTag.value !== 'all' && memo.tag !== selectedMemoTag.value) return false;
    if (!query) return true;
    return `${memo.title} ${memo.note || ''} ${memo.tag}`.toLowerCase().includes(query);
  });
});

export const pendingMemos = computed(() =>
  [...filteredMemos.value.filter(m => !m.done)].sort((a, b) => Number(!!b.pinned) - Number(!!a.pinned))
);
export const completedMemos = computed(() => filteredMemos.value.filter(m => m.done));
export const memoPendingCount = computed(() => customMemos.value.filter(m => !m.done).length);

export const memoStats = computed(() => {
  const total = customMemos.value.length;
  const pending = customMemos.value.filter(m => !m.done).length;
  const completed = total - pending;
  return { total, pending, completed };
});

// 时间周期剩余时间格式化
export function getMemoRemainingText(memo: CustomMemo): { text: string; urgent: boolean; isDone: boolean; badgeClass: string } {
  if (memo.cycle_type === 'once') {
    if (memo.done) return { text: '已达成', urgent: false, isDone: true, badgeClass: 'bg-done/10 text-done border-done/35' };
    return { text: '一次', urgent: false, isDone: false, badgeClass: 'bg-raised text-muted border-hairline' };
  }

  const now = Date.now();
  gameResetEpoch.value;
  if (memo.cycle_type === 'daily') {
    const clock = parseResetClock(gameMetaOf(gameIdOfMemoTag(memo.tag)).dailyReset, '00:00');
    const hhmm = `${String(clock.hour).padStart(2, '0')}:${String(clock.minute).padStart(2, '0')}`;
    if (memo.done) {
      return { text: `今日已完成 · 下次 ${hhmm}`, urgent: false, isDone: true, badgeClass: 'bg-done/10 text-done border-done/35' };
    }
    return { text: `每日 · ${hhmm}`, urgent: true, isDone: false, badgeClass: 'bg-beacon/10 text-beacon border-beacon/35' };
  }

  if (memo.cycle_type === 'weekly') {
    const meta = gameMetaOf(gameIdOfMemoTag(memo.tag));
    const clock = parseResetClock(meta.weeklyReset, meta.dailyReset);
    const hhmm = `${String(clock.hour).padStart(2, '0')}:${String(clock.minute).padStart(2, '0')}`;
    const days = weekdayLabel(parseWeekdays(memo.cycle_val));
    if (memo.done) {
      return { text: `本周已完成 · 下次周${'日一二三四五六'[clock.weekday ?? 1]}${hhmm}`, urgent: false, isDone: true, badgeClass: 'bg-done/10 text-done border-done/35' };
    }
    const today = isWeekdayActive(memo.cycle_val);
    return {
      text: today ? `${days} · ${hhmm}` : `${days} · 非今日`,
      urgent: today,
      isDone: false,
      badgeClass: today ? 'bg-beacon/10 text-beacon border-beacon/35' : 'bg-raised text-faint border-hairline',
    };
  }

  if (memo.cycle_type === 'interval') {
    const val = memo.cycle_val || '3d';
    if (!memo.done || !memo.last_completed_at) {
      return { text: `每 ${val}`, urgent: true, isDone: false, badgeClass: 'bg-beacon/10 text-beacon border-beacon/35' };
    }
    const completedTime = new Date(memo.last_completed_at).getTime();
    let ms = 3 * 24 * 3600 * 1000;
    if (val.endsWith('d')) {
      const d = parseFloat(val);
      if (!isNaN(d)) ms = d * 24 * 3600 * 1000;
    } else if (val.endsWith('h')) {
      const h = parseFloat(val);
      if (!isNaN(h)) ms = h * 3600 * 1000;
    }
    const diff = completedTime + ms - now;
    if (diff <= 0) {
      return { text: `已到期 · 每 ${val}`, urgent: true, isDone: false, badgeClass: 'bg-alert/10 text-alert border-alert/35' };
    }
    const hours = Math.floor(diff / (3600 * 1000));
    if (hours >= 24) {
      const days = Math.floor(hours / 24);
      const remHours = hours % 24;
      return { text: `下次 ${days}天${remHours}时`, urgent: false, isDone: true, badgeClass: 'bg-done/10 text-done border-done/35' };
    }
    return { text: `下次 ${hours} 时`, urgent: false, isDone: true, badgeClass: 'bg-done/10 text-done border-done/35' };
  }

  if (memo.cycle_type === 'deadline') {
    if (!memo.cycle_val) return { text: '未设截止日期', urgent: false, isDone: memo.done, badgeClass: 'bg-raised text-faint border-hairline' };
    const target = new Date(memo.cycle_val).getTime();
    const diff = target - now;
    if (diff <= 0) {
      return { text: '已过期', urgent: true, isDone: memo.done, badgeClass: 'bg-alert/10 text-alert border-alert/35' };
    }
    const days = Math.floor(diff / (24 * 3600 * 1000));
    return {
      text: `剩 ${days} 天`,
      urgent: days <= 3,
      isDone: memo.done,
      badgeClass: days <= 3 ? 'bg-alert/10 text-alert border-alert/35' : 'bg-beacon/10 text-beacon border-beacon/35'
    };
  }

  return { text: '追踪', urgent: false, isDone: memo.done, badgeClass: 'bg-raised text-faint border-hairline' };
}

export function memoStampOf(tag: string): GameStamp {
  return gameMetaOf(gameIdOfMemoTag(tag)).stamp;
}

export function gameIdOfMemoTag(tag: string): string {
  if (tag === '剑网3') return 'jx3';
  if (tag === '光遇') return 'sky';
  if (tag === '逆水寒') return 'nsh';
  return 'other';
}

function memoResetBoundary(memo: CustomMemo, now: Date): Date | null {
  const meta = gameMetaOf(gameIdOfMemoTag(memo.tag));
  if (memo.cycle_type === 'daily') {
    const clock = parseResetClock(meta.dailyReset, '00:00');
    return lastDailyBoundary(now, clock.hour, clock.minute);
  }
  if (memo.cycle_type === 'weekly') {
    const clock = parseResetClock(meta.weeklyReset, meta.dailyReset);
    return lastWeeklyBoundary(now, clock.weekday ?? 1, clock.hour, clock.minute);
  }
  return null;
}

// 自动检测周期重置
export function checkMemoIntervalResets(nowDate = new Date()) {
  const now = nowDate.getTime();
  for (const memo of customMemos.value) {
    if (!memo.done || !memo.last_completed_at) continue;
    const completedTime = new Date(memo.last_completed_at).getTime();
    if (isNaN(completedTime)) continue;

    let shouldReset = false;
    if (memo.cycle_type === 'daily' || memo.cycle_type === 'weekly') {
      const boundary = memoResetBoundary(memo, nowDate);
      if (boundary && now >= boundary.getTime() && completedTime < boundary.getTime()) {
        shouldReset = true;
      }
    } else if (memo.cycle_type === 'interval') {
      const val = memo.cycle_val || '1d';
      let ms = 24 * 3600 * 1000;
      if (val.endsWith('d')) {
        const d = parseFloat(val);
        if (!isNaN(d)) ms = d * 24 * 3600 * 1000;
      } else if (val.endsWith('h')) {
        const h = parseFloat(val);
        if (!isNaN(h)) ms = h * 3600 * 1000;
      }
      if (now - completedTime >= ms) {
        shouldReset = true;
      }
    }

    if (shouldReset) {
      memo.done = false;
      memo.current_progress = 0;
      memo.last_completed_at = undefined;
      updateCustomMemo(memo);
    }
  }
}

// 复制坐标工具
export function copyMemoCoordinates(text: string) {
  const match = text.match(/\(?\d{2,4}[,\s，]+\d{2,4}\)?/);
  const target = match ? match[0] : text;
  navigator.clipboard.writeText(target);
  showToast('已复制坐标到剪贴板', target);
}

export async function createMemo(input: MemoInput): Promise<boolean> {
  const title = input.title.trim();
  if (!title) {
    showToast('请输入记事标题', '例如：沧州白鹰独珍戒指获取');
    return false;
  }

  const newMemo: CustomMemo = {
    ...input,
    id: newTaskId('m'),
    title,
    tag: input.tag.trim() || '通用',
    cycle_val: input.cycle_type === 'once' || input.cycle_type === 'daily' ? undefined : input.cycle_val || undefined,
    character_id: input.character_id || undefined,
    note: input.note?.trim() || undefined,
    current_progress: 0,
    target_progress: input.has_progress ? Math.max(1, Number(input.target_progress) || 1) : 0,
    done: false,
    created_at: new Date().toISOString().replace('T', ' ').slice(0, 19)
  };

  try {
    await addCustomMemo(newMemo);
    if (!customMemos.value.some(memo => memo.id === newMemo.id)) customMemos.value.unshift(newMemo);
    kvSet('waypoint_memo_last_tag', newMemo.tag);
    showToast('记事已创建', `${newMemo.title} · ${newMemo.tag}`);
    return true;
  } catch (e) {
    showToast('创建失败', String(e));
    return false;
  }
}

export async function handleToggleMemo(memo: CustomMemo) {
  try {
    const newDone = await toggleCustomMemo(memo.id);
    memo.done = newDone;
    memo.last_completed_at = newDone ? new Date().toISOString() : undefined;
    if (newDone && memo.has_progress) {
      memo.current_progress = memo.target_progress;
    }
  } catch (e) {
    showToast('切换失败', String(e));
  }
}

export async function handleProgressChange(memo: CustomMemo, delta: number) {
  const before = { current_progress: memo.current_progress, done: memo.done, last_completed_at: memo.last_completed_at };
  await optimistic(() => {
    memo.current_progress = Math.max(0, Math.min(memo.target_progress, memo.current_progress + delta));
    if (memo.current_progress >= memo.target_progress && !memo.done) {
      memo.done = true;
      memo.last_completed_at = new Date().toISOString();
    } else if (memo.current_progress < memo.target_progress && memo.done) {
      memo.done = false;
      memo.last_completed_at = undefined;
    }
  }, () => Object.assign(memo, before), () => updateCustomMemo(memo));
}

export async function handleDeleteMemo(id: string) {
  try {
    await deleteCustomMemo(id);
    customMemos.value = customMemos.value.filter(m => m.id !== id);
    showToast('记事已删除', '已从本地看板移除');
  } catch (e) {
    showToast('删除失败', String(e));
  }
}

const PIN_LIMIT = 3;

export async function handleToggleMemoPin(memo: CustomMemo) {
  const next = !memo.pinned;
  if (next && customMemos.value.filter(row => row.pinned && row.id !== memo.id).length >= PIN_LIMIT) {
    showToast('置顶已满', '最多钉 3 条，先取消一条');
    return;
  }
  await optimistic(() => { memo.pinned = next; }, () => { memo.pinned = !next; }, () => updateCustomMemo(memo));
}

export async function handleToggleMemoStep(memo: CustomMemo, index: number) {
  if (!memo.note) return;
  const updated = { ...memo, note: toggleNoteStep(memo.note, index) };
  const progress = noteStepProgress(updated.note);
  if (progress.total > 0 && progress.done === progress.total && !updated.done) {
    updated.done = true;
    updated.last_completed_at = new Date().toISOString();
  }
  await updateCustomMemo(updated);
  Object.assign(memo, updated);
}

export async function handleRenameMemo(memo: CustomMemo, title: string) {
  const next = title.trim();
  if (!next || next === memo.title) return;
  memo.title = next;
  await updateCustomMemo(memo);
  showToast('已修改', next);
}

export async function handleUpdateMemo(memo: CustomMemo, patch: Partial<CustomMemo>) {
  const title = (patch.title ?? memo.title).trim();
  if (!title) {
    showToast('请输入记事标题', '标题不能为空');
    return false;
  }
  const cycleType = patch.cycle_type ?? memo.cycle_type;
  const cycleVal = cycleType === 'interval' || cycleType === 'deadline' || cycleType === 'weekly'
    ? (patch.cycle_val ?? memo.cycle_val)
    : undefined;
  const hasProgress = patch.has_progress ?? memo.has_progress;
  const target = hasProgress
    ? Math.max(1, Number(patch.target_progress ?? memo.target_progress) || 1)
    : 0;
  const current = hasProgress ? Math.min(memo.current_progress, target) : memo.current_progress;
  const updated: CustomMemo = {
    ...memo, ...patch, title,
    tag: (patch.tag ?? memo.tag).trim() || '通用',
    cycle_type: cycleType, cycle_val: cycleVal,
    note: (patch.note ?? memo.note)?.trim() || undefined,
    character_id: (patch.character_id ?? memo.character_id) || undefined,
    has_progress: hasProgress, target_progress: target, current_progress: current,
  };
  try {
    await updateCustomMemo(updated);
    Object.assign(memo, updated);
    showToast('记事已更新', memo.title);
    return true;
  } catch (error) {
    showToast('更新失败', String(error));
    return false;
  }
}
