import { test } from 'vitest';
import { memoLinePreview, parseMemoLine } from './memoCapture.ts';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('memoCapture', () => {
  const interval = parseMemoLine('打百炼 3d #逆水寒');
  assert(interval.title === '打百炼', `title ${interval.title}`);
  assert(interval.tag === '逆水寒', 'tag');
  assert(interval.cycle_type === 'interval' && interval.cycle_val === '3d', 'interval');
  assert(memoLinePreview(interval, '通用') === '逆水寒 · 每 3d', memoLinePreview(interval, '通用'));

  const count = parseMemoLine('攒寒铁 x10');
  assert(count.title === '攒寒铁', 'count title');
  assert(count.has_progress && count.target_progress === 10, 'count target');
  assert(!count.tag, 'count has no tag');

  const deadline = parseMemoLine('战令 截止 2026-09-30 #剑网3');
  assert(deadline.title === '战令', 'deadline title');
  assert(deadline.cycle_type === 'deadline' && deadline.cycle_val === '2026-09-30', 'deadline');
  assert(deadline.tag === '剑网3', 'deadline tag');

  const weekly = parseMemoLine('每周 跑商');
  assert(weekly.title === '跑商' && weekly.cycle_type === 'weekly' && !weekly.cycle_val, 'weekly');

  const weekend = parseMemoLine('攻防 每周五六日 #剑网3');
  assert(weekend.title === '攻防' && weekend.cycle_val === '5,6,0', `weekend ${weekend.cycle_val}`);
  assert(memoLinePreview(weekend, '通用') === '剑网3 · 周五六日', memoLinePreview(weekend, '通用'));

  const hours = parseMemoLine('秘境 48h');
  assert(hours.cycle_type === 'interval' && hours.cycle_val === '48h', 'hours');

  const plain = parseMemoLine('买马草');
  assert(plain.title === '买马草' && plain.cycle_type === 'once' && !plain.tag, 'plain');
  assert(memoLinePreview(plain, '光遇') === '光遇 · 一次', 'fallback tag');

  const laterWins = parseMemoLine('刷本 3d 每周');
  assert(laterWins.cycle_type === 'weekly' && !laterWins.cycle_val, 'later token wins');
});
