import { expect, test } from 'vitest';
import { seasonEtaText, seasonGap, seasonGapText } from './skySeasonService.ts';
import type { SeasonalGuide, SeasonalItem } from '../types';

function item(spirit: string, kind: string, costDesc: string, candles: number, obtained = false): SeasonalItem {
  return { id: `${spirit}|${kind}`, name: `${spirit} · ${kind}`, spirit, costDesc, candles, ascended: 0, hearts: 0, obtained };
}

// 珍珠季嬉闹孩童整树 96 季蜡；面具 18、裤子 24，发型、斗篷是季卡
function pearl(itemList: SeasonalItem[]): SeasonalGuide {
  return {
    id: 'wiki-珍珠季', season: '珍珠季', loc: '', items: '', itemList, priority: '', candles: 0, ascended: 0, hearts: 0,
    selected: false, status: 'live', start: '2026-10-22', end: '2027-01-13', treeCandles: { 嬉闹孩童: 96 },
  };
}

test('season gap counts the whole tree and estimates days left', () => {
  const guide = pearl([
    item('嬉闹孩童', '面具', '18季蜡', 18),
    item('嬉闹孩童', '裤子', '24季蜡', 24),
    item('嬉闹孩童', '发型', '季卡', 0),
  ]);
  const gap = seasonGap(guide, '2026-11-01');
  expect(gap.wax).toBe(96);
  expect(gap.daysLeft).toBe(74);
  expect(gap.daysNeeded).toBe(20);
  expect(gap.daysNeededPass).toBe(16);
  expect(seasonGapText(gap)).toBe('约差 96季蜡');
  expect(seasonEtaText(gap)).toBe('每天做满约 20 天，季卡 16 天 · 剩 74 天');

  guide.itemList[0].obtained = true;
  expect(seasonGap(guide, '2026-11-01').wax).toBe(78);

  // 季蜡物品都换完，只剩季卡物品：不再要季蜡
  guide.itemList[1].obtained = true;
  const passOnly = seasonGap(guide, '2026-11-01');
  expect(passOnly.wax).toBe(0);
  expect(seasonGapText(passOnly)).toBe('1 件待换');
  expect(seasonEtaText(passOnly)).toBe('');
});

test('ended seasons get no day estimate', () => {
  const guide = { ...pearl([item('嬉闹孩童', '面具', '18季蜡', 18)]), status: 'ended' };
  const gap = seasonGap(guide, '2027-03-01');
  expect(gap.daysLeft).toBeNull();
  expect(seasonEtaText(gap)).toBe('');
});
