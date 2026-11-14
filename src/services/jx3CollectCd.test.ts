import { test } from 'vitest';
import { clearJx3CollectCd, jx3CollectCdOpen, jx3CollectCdRule, jx3CollectPeriodStamp, markJx3CollectCd } from './jx3CollectCd';
import type { Jx3CollectStore } from '../types';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('jx3CollectCd', () => {
  const monday = new Date(2026, 8, 21, 8, 0, 0);
  const sunday = new Date(2026, 8, 20, 23, 0, 0);
  assert(jx3CollectPeriodStamp('weekly', monday) === jx3CollectPeriodStamp('weekly', new Date(2026, 8, 21, 7, 0, 0)), '周一 07:00 起算本周');
  assert(jx3CollectPeriodStamp('weekly', sunday) !== jx3CollectPeriodStamp('weekly', monday), '周日还算上一周');
  assert(jx3CollectPeriodStamp('daily', new Date(2026, 8, 23, 6, 30, 0)) !== jx3CollectPeriodStamp('daily', new Date(2026, 8, 23, 7, 1, 0)), '07:00 前还是昨天');

  assert(jx3CollectCdRule({ maps: [{ mapId: '1', mode: '5人英雄' }] }) === 'daily', '5人每日');
  assert(jx3CollectCdRule({ maps: [{ mapId: '1', mode: '25人英雄' }] }) === 'weekly', '25人周一');
  assert(jx3CollectCdRule({ maps: [{ mapId: '1', mode: '5人普通' }, { mapId: '2', mode: '10人普通' }] }) === 'weekly', '有团队本按周一');
  assert(jx3CollectCdRule(null, { cdRule: '无CD' }) === 'none', '手写无CD');

  const store: Jx3CollectStore = {
    items: [{ id: 'a', dungeonKey: '荻花', dungeonName: '荻花', kind: 'special', name: '挂件', obtained: false, source: 'custom' }],
    pinnedDungeons: [],
    collapsed: {},
  };
  const marked = markJx3CollectCd(store, '荻花', 'weekly', monday);
  assert(jx3CollectCdOpen(marked.items[0], 'weekly', monday) === false, '本周已打');
  assert(jx3CollectCdOpen(marked.items[0], 'weekly', new Date(2026, 8, 28, 8, 0, 0)) === true, '下周一重置');
  assert(clearJx3CollectCd(marked, '荻花').items[0].cdClearedAt == null, '取消后本周未打');
});
