import { test } from 'vitest';
import { dailyChecked, progressEntries, progressReplace, progressSet, setCharacterDailyChecks } from './progress';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('progressReplace 删除缺席行，不会复活', () => {
  progressReplace('t.scope', [['a', 1], ['b', 2]], 'c1');
  progressReplace('t.scope', [['b', 3]], 'c1');
  const rows = progressEntries<number>('t.scope', 'c1');
  assert(rows.length === 1 && rows[0][0] === 'b' && rows[0][1] === 3, JSON.stringify(rows));
  assert(progressEntries('t.scope', '').length === 0, '角色之间互不影响');
  progressSet('t.scope', 'b', undefined, 'c1');
  assert(progressEntries('t.scope', 'c1').length === 0, 'undefined 即删除');
});

test('每日打卡按角色与天隔离，重复写入无额外记录', async () => {
  await setCharacterDailyChecks('c1', '2026-10-29', { 't.day': ['q1', 'q2'] }, true);
  await setCharacterDailyChecks('c1', '2026-10-29', { 't.day': ['q1'] }, true);
  assert(dailyChecked('t.day', '2026-10-29', 'c1').size === 2, '重复勾不增加记录');
  assert(dailyChecked('t.day', '2026-10-30', 'c1').size === 0, '别的日子为空');
  await setCharacterDailyChecks('c1', '2026-10-29', { 't.day': ['q1'] }, false);
  assert([...dailyChecked('t.day', '2026-10-29', 'c1')].join() === 'q2', 'uncheck');
});
