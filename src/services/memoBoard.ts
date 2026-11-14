import type { CustomMemo } from '../types';

/** 未完成记事进日常 · 周常。挂了角色的只出现在该号；没挂角色的只出现在总览。 */
export function memoVisibleOnBoard(
  memo: CustomMemo,
  scope: string,
  cycle?: 'daily' | 'weekly' | 'other',
  includeCompleted = false,
): boolean {
  if (memo.done && !includeCompleted) return false;
  const onScope = memo.character_id
    ? scope === 'all' || memo.character_id === scope
    : scope === 'all';
  if (!onScope) return false;
  if (!cycle) return true;
  if (cycle === 'daily') return memo.cycle_type === 'daily';
  if (cycle === 'weekly') return memo.cycle_type === 'weekly';
  return memo.cycle_type !== 'daily' && memo.cycle_type !== 'weekly';
}
