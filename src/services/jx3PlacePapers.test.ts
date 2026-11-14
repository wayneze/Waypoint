import { expect, test } from 'vitest';
import { buildJx3PlacePapers, computeAdventureProgress, evaluateReputationAction } from './jx3Data';
import { achievementsAtPlaces, jx3AchievementById, jx3AchievementGroups } from './jx3AchievementCatalog';
import type { Jx3AdventureItem, Jx3ReputationItem } from '../types';

test('完整游戏菜单的 ID 均有记录；地点页直接对照快照，不局限于手写通关成就', () => {
  expect(jx3AchievementGroups.flatMap(group => group.ids).every(id => jx3AchievementById.has(id))).toBe(true);
  const paper = buildJx3PlacePapers(new Set(['2526'])).find(paper => paper.title === '荻花圣殿')!;
  expect(paper.rows.length).toBeGreaterThan(10);
  expect(paper.rows.find(row => row.id === '2526')?.match).toBe('done');
  expect(paper.rows.find(row => row.id === '2536')?.match).toBe('missing');
});

test('奇遇系列、资历和完成状态独立计算，不受成就页和另一角色覆盖', () => {
  const adv: Jx3AdventureItem = { id: 'a', name: '茶馆奇缘', type: '百家奇遇', reward: '', triggerDaily: '', minPoint: 0, completeAchieveId: '4610', isTriggered: false, requirements: [
    { id: 'series', type: 'achievement', name: '系列', seriesMode: 'any', seriesIds: ['2526', '2536'], targetVal: 1, completed: false },
    { id: 'points', type: 'point', name: '资历', targetVal: 10, completed: false },
  ] };
  const original = JSON.stringify(adv);
  const first = computeAdventureProgress(adv, new Set(['2526', '4610']), new Map());
  buildJx3PlacePapers(new Set());
  const other = computeAdventureProgress(adv, new Set(['1']), new Map());
  expect(first.completeMatch).toBe('done');
  expect(first.completedCount).toBe(2);
  expect(other.requirements[0].match).toBe('missing');
  expect(first.requirements[0].match).toBe('done');
  expect(JSON.stringify(adv)).toBe(original);
});

test('声望建议综合真实成就与所有心愿；内容清完或明确不再刷时改用道具', () => {
  const rep: Jx3ReputationItem = { id: 'test', name: '测试势力', group: '', place: '荻花圣殿', lane: 'dungeon', currentLevel: '中立', levelSource: 'unknown', maxLevel: '钦佩', recommendedItem: '', itemGain: 0, rewardHighlight: '' };
  const complete = new Set(achievementsAtPlaces(['荻花圣殿'], 'dungeon').map(row => String(row.ID)));
  const items = [{ dungeonName: '英雄荻花圣殿', obtained: false }];
  expect(evaluateReputationAction(rep, complete, items).smartAction).toBe('farm');
  items[0].obtained = true;
  expect(evaluateReputationAction(rep, complete, items).smartAction).toBe('item');
  expect(evaluateReputationAction(rep, new Set(), items).smartAction).toBe('farm');
  expect(evaluateReputationAction({ ...rep, preferredMethod: 'item' }, new Set(), items).smartAction).toBe('item');
  expect(evaluateReputationAction({ ...rep, preferredMethod: 'content' }, complete, items).smartAction).toBe('farm');
});
