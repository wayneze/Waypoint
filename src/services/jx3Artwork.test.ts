import { expect, test } from 'vitest';
import { jx3AchievementIconIds, jx3CategoryImage, jx3ReputationIconUrl } from './jx3Artwork';
import { resolveJx3TaskVisual } from './jx3Schedule';
import { jx3BoxItemIconUrl } from './jx3Collect';

test('成就使用自己的 IconID，未知 ID 使用分类图；旧声望目录能显示已核验的势力徽记', () => {
  jx3AchievementIconIds['2526'] = 16;
  expect(resolveJx3TaskVisual({ catalog_kind: 'achievement', catalog_id: '2526', title: '全胜！25人英雄荻花圣殿' }).iconUrl)
    .toBe(jx3BoxItemIconUrl(16));
  expect(resolveJx3TaskVisual({ catalog_kind: 'achievement', catalog_id: '99999999' }).iconUrl)
    .toBe(jx3CategoryImage('剑侠录'));
  expect(jx3ReputationIconUrl({ name: '轩辕社' })).toBe('/jx3/art/reputation-xuanyuanshe.webp');
  expect(jx3ReputationIconUrl({ name: '未核验势力' })).toBe('');
});

test('手动副本待办与日历大战都保留对应场景图', () => {
  const dungeons = [{ key: '太极秘境', name: '太极秘境', era: '', eraLevel: '', icon: 'map/loading/loading_68_0.jpg', maps: [], bosses: [] }];
  const manual = resolveJx3TaskVisual({ title: '大战 · 太极秘境' }, { dungeons });
  const calendar = resolveJx3TaskVisual({ title: '【今日大战】太极秘境', api_key: 'war' }, { dungeons });
  expect(manual.sceneUrl).toBeTruthy();
  expect(manual.sceneUrl).toBe(calendar.sceneUrl);
  expect(manual.iconUrl).toBe(jx3CategoryImage('秘境'));
});
