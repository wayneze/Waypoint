import { expect, test, vi } from 'vitest';
import { fetchSkyDailyFromBackend, fetchSkyQuestCalendar } from './skyApi';
import { loadTodaySkyDailyGuide } from './skyDailyService';

vi.mock('./skyApi', () => ({
  fetchSkyDailyFromBackend: vi.fn(),
  fetchSkyQuestCalendar: vi.fn(),
}));

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('skyDaily image roles follow the post caption', async () => {
  const { parseSkyDailyPost } = await import('./skyDailyLexicon.ts');
  const hints = parseSkyDailyPost([
    '图1 - 季节蜡烛', '图2 - 任务', '图3 - 大蜡烛', '图4 - 可能的落石位置',
    '图5 - 本周四复刻先祖预估兑换树', '图6 - 本周四复刻先祖动作展示', '图7 - 恶作剧之日活动兑换树', '图8 - 近期活动日历',
    '图12 - 红石具体位置',
  ].join('\n')).imageHints;
  assert(hints.seasonCandle === 0 && hints.quests === 1 && hints.treasureCandle === 2 && hints.shard === 3, JSON.stringify(hints));
  assert(hints.token === undefined, `emote showcase / exchange tree must not be the token image, got ${hints.token}`);
  assert(hints.shard === 3, `later redstone image caption must not replace the shard map, got ${hints.shard}`);
  const { imageHints: noShardCaption } = parseSkyDailyPost('图1 - 季蜡\n图2 - 每日任务\n图3 - 先祖兑换树');
  assert(noShardCaption.shard === undefined, `an unlabelled image must not be guessed as a shard map, got ${noShardCaption.shard}`);
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

test('换日后只接受当天有标签的落石图，暂时断线仍保留当天图文', async () => {
  vi.useFakeTimers();
  try {
    vi.setSystemTime(new Date('2026-11-22T00:01:00+08:00'));
    const old = {
      success: true, date_str: '2026-11-21', post_text: '每日任务\n图2 - 红石落点',
      all_images: ['old-quests', 'old-shard'],
    };
    vi.mocked(fetchSkyDailyFromBackend).mockResolvedValue(old);
    vi.mocked(fetchSkyQuestCalendar).mockResolvedValue({
      items: [{ date: '2026-11-22', quest_cn: '和好友击掌', quest_id: 'high-five' }],
      image: 'today-quests',
    });
    const withoutPost = await loadTodaySkyDailyGuide(true);
    expect(withoutPost.overviewImages).toMatchObject({ questsImg: 'today-quests', shardImg: '' });

    vi.mocked(fetchSkyQuestCalendar).mockResolvedValue({
      items: [{ date: '2026-11-22', quest_cn: '和好友击掌', quest_id: 'high-five' }],
      image: 'today-quests',
      shardImage: 'typed-shard',
    });
    expect((await loadTodaySkyDailyGuide(true)).overviewImages.shardImg).toBe('typed-shard');

    vi.mocked(fetchSkyQuestCalendar).mockResolvedValue(null);
    vi.mocked(fetchSkyDailyFromBackend).mockResolvedValue({
      ...old, date_str: '2026-11-22', post_text: '每日任务\n图1 - 红石落点\n图2 - 兑换树',
      all_images: ['today-shard', 'reward-tree'],
    });
    expect((await loadTodaySkyDailyGuide(true)).overviewImages.shardImg).toBe('today-shard');

    vi.mocked(fetchSkyDailyFromBackend).mockResolvedValue(null);
    vi.mocked(fetchSkyQuestCalendar).mockResolvedValue(null);
    expect((await loadTodaySkyDailyGuide(true)).overviewImages.shardImg).toBe('today-shard');
    vi.setSystemTime(new Date('2026-11-23T00:01:00+08:00'));
    expect((await loadTodaySkyDailyGuide(true)).isFallback).toBe(true);
  } finally {
    vi.useRealTimers();
  }
});
