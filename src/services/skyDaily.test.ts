import { expect, test } from 'vitest';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('skyDaily image roles follow the post caption', async () => {
  const { parseSkyDailyPost } = await import('./skyDailyLexicon.ts');
  const hints = parseSkyDailyPost([
    '图1 - 季节蜡烛', '图2 - 任务', '图3 - 大蜡烛', '图4 - 可能的落石位置',
    '图5 - 本周四复刻先祖预估兑换树', '图6 - 本周四复刻先祖动作展示', '图7 - 恶作剧之日活动兑换树', '图8 - 近期活动日历',
  ].join('\n')).imageHints;
  assert(hints.seasonCandle === 0 && hints.quests === 1 && hints.treasureCandle === 2 && hints.shard === 3, JSON.stringify(hints));
  assert(hints.token === undefined, `emote showcase / exchange tree must not be the token image, got ${hints.token}`);
  assert(parseSkyDailyPost('图5 - 活动代币位置').imageHints.token === 4, 'explicit token caption');
});

test('activity instructions accept token counts but never present truncated copy as a guide', async () => {
  const { parseSkyDailyPost } = await import('./skyDailyLexicon');
  const header = '=『恶作剧之日』=\n活动时间:2026.10.24 - 2026.11.13\n';
  expect(parseSkyDailyPost(header + '每日5枚代币获取方式:完成小游戏或在坩埚旁收集烛火').eventDaily?.how)
    .toBe('完成小游戏或在坩埚旁收集烛火');
  expect(parseSkyDailyPost(header + '每日代币获取方式:收集活动烛火').eventDaily?.how).toBe('收集活动烛火');
  expect(parseSkyDailyPost(header + '每日5枚代币获取方式:完成 ...全文').eventDaily?.how).toBeUndefined();
});
