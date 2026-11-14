import { test } from 'vitest';
import { parseMingyiAchievementShot } from './jx3AchievementParser';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

function assertThrows(fn: () => void, needle: string) {
  try {
    fn();
  } catch (e: any) {
    assert(String(e).includes(needle), `expected error to include ${needle}, got ${e}`);
    return;
  }
  throw new Error(`expected throw containing ${needle}`);
}

test('jx3AchievementParser', () => {
  const lua = parseMingyiAchievementShot('return {0,1,2,10,24,12274}');
  assert(lua.format === 'lua-array', 'lua format');
  assert(lua.ids.join(',') === '0,1,2,10,24,12274', 'lua ids');

  const json = parseMingyiAchievementShot('[0,1,10]');
  assert(json.format === 'json-array', 'json format');
  assert(json.ids.join(',') === '0,1,10', 'json ids');

  assertThrows(
    () => parseMingyiAchievementShot('["achieve"] = { ["1001"] = true, ["3042"] = 1692800000 }'),
    '无法识别'
  );
  assertThrows(() => parseMingyiAchievementShot(''), '空内容');
});
