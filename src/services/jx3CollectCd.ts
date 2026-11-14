import type { Jx3BoxDungeon, Jx3CollectItem, Jx3CollectStore } from '../types';
import { lastDailyBoundary, lastWeeklyBoundary, periodStamp } from './cycleReset';

export type Jx3CollectCdRule = 'daily' | 'weekly' | 'none';

const JX3_RESET_HOUR = 7;

export function jx3CollectCdRule(dungeon?: Pick<Jx3BoxDungeon, 'maps'> | null, item?: Pick<Jx3CollectItem, 'cdRule'> | null): Jx3CollectCdRule {
  const written = (item?.cdRule || '').trim();
  if (written === '无CD') return 'none';
  if (written.includes('周一')) return 'weekly';
  if (written.includes('每日')) return 'daily';
  const modes = (dungeon?.maps || []).map(m => m.mode || '').filter(Boolean);
  if (!modes.length) return 'weekly';
  if (modes.some(mode => /10人|25人|团队/.test(mode))) return 'weekly';
  if (modes.every(mode => /5人/.test(mode) || mode === '')) return 'daily';
  return 'weekly';
}

export function jx3CollectCdLabel(rule: Jx3CollectCdRule): string {
  if (rule === 'daily') return '每日 07:00';
  if (rule === 'weekly') return '周一 07:00';
  return '无CD';
}

export function jx3CollectPeriodStamp(rule: Jx3CollectCdRule, now = new Date()): string {
  if (rule === 'none') return '';
  if (rule === 'weekly') return periodStamp(lastWeeklyBoundary(now, 1, JX3_RESET_HOUR, 0));
  return periodStamp(lastDailyBoundary(now, JX3_RESET_HOUR, 0));
}

export function jx3CollectCdOpen(item: Pick<Jx3CollectItem, 'cdClearedAt'>, rule: Jx3CollectCdRule, now = new Date()): boolean {
  if (rule === 'none') return false;
  const stamp = item.cdClearedAt || '';
  if (!stamp) return true;
  return stamp !== jx3CollectPeriodStamp(rule, now);
}

export function markJx3CollectCd(store: Jx3CollectStore, dungeonKey: string, rule: Jx3CollectCdRule, now = new Date()): Jx3CollectStore {
  const stamp = jx3CollectPeriodStamp(rule, now);
  return {
    ...store,
    items: store.items.map(item => item.dungeonKey === dungeonKey ? { ...item, cdClearedAt: stamp || undefined } : item),
  };
}

export function clearJx3CollectCd(store: Jx3CollectStore, dungeonKey: string): Jx3CollectStore {
  return {
    ...store,
    items: store.items.map(item => item.dungeonKey === dungeonKey ? { ...item, cdClearedAt: undefined } : item),
  };
}
