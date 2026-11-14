import { test } from 'vitest';
import {
  parseReputationStanding,
  reputationItemsFromCatalog,
  reputationLaneFromGain,
  standingsFromAchievements,
} from './jx3ProgressLedger';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('jx3ProgressLedger', () => {
  assert(
    parseReputationStanding({ ID: 11883, Desc: '段氏声望达到钦佩' })?.name === '段氏',
    '达到'
  );
  assert(
    parseReputationStanding({ ID: 6684, Desc: '蓬莱声望到达钦佩' })?.level === '钦佩',
    '到达'
  );
  assert(
    parseReputationStanding({ ID: 514, Desc: '红衣教声望到友善' })?.level === '友好',
    '友善记成友好'
  );
  assert(parseReputationStanding({ ID: 11223, Desc: '云从社名望达到10级' }) == null, '名望不是声望等级');

  const standings = standingsFromAchievements([
    { ID: 503, Name: '洛阳贵客', Desc: '洛阳声望到尊敬' },
    { ID: 515, Name: '洛阳至交', Desc: '洛阳声望到钦佩' },
    { ID: 1346, Name: '丐帮挚友', Desc: '丐帮声望到尊敬' },
  ], new Set(['503', '515']));

  assert(standings.length === 1 && standings.find(s => s.name === '洛阳')?.level === '钦佩', '同势力取已完成的最高级');
  assert(!standings.some(s => s.name === '丐帮'), '没有等级证据时不凭空判为中立');

  assert(reputationLaneFromGain('击败阆风悬城的首领') === 'dungeon', '首领是打本');
  assert(reputationLaneFromGain('完成河西瀚漠的阵营日常任务') === 'map', '日常是地图');
  assert(reputationLaneFromGain('完成历程任务；击败英雄剑冢惊变的首领') === 'both', '任务加首领是都能');
  assert(reputationLaneFromGain('上缴任意马匹') === 'item', '上缴是道具');
  assert(reputationLaneFromGain('可通过【遗失的尊敬】、【迟来的钦佩】等道具提升') === 'item', '只有道具途径时归道具');
  assert(reputationLaneFromGain('五毒门派任务；击败英雄秘境小怪和首领掉落的道具圣兽之鳞换取。') === 'both', '门派任务加首领是都能');
  assert(reputationLaneFromGain('完成历程；或通过声望书提升') === 'map', '已有历程时声望书不抢分类');
  const catalog = reputationItemsFromCatalog(
    [{ dwForceID: 283, szName: '五更声', szIconPath: 'ui\\Image\\ReputationPanel\\icon\\wugengsheng.Tga', GroupName: '江湖势力', szMapNames: ['阆风悬城'], GainDesc: [{ from: '7', to: '8', desc: '击败阆风悬城的首领。' }] }],
    [{ id: 'rep_1', name: '五更声', group: '声望', level: '尊敬', achieveId: '1', achieveName: '五更声挚友' }]
  );
  assert(catalog[0]?.lane === 'dungeon' && catalog[0]?.place === '阆风悬城' && catalog[0]?.currentLevel === '尊敬' && catalog[0]?.maxLevel === '钦佩', '目录带地图、等级和上限');
  assert(catalog[0]?.iconPath?.endsWith('wugengsheng.Tga'), '势力真实徽记路径随目录保留');
  const capped = reputationItemsFromCatalog(
    [{ dwForceID: 1, szName: '敦煌张氏', GroupName: '江湖势力', GainDesc: [{ from: '3', to: '4', desc: '完成周常任务。' }] }],
    []
  );
  assert(capped[0]?.maxLevel === '友好' && capped[0]?.lane === 'map', '目录最高档是友好，不写成钦佩');
  const books = reputationItemsFromCatalog(
    [{ dwForceID: 2, szName: '昆仑', GroupName: '江湖势力', GainDesc: [{ from: '3', to: '5', desc: '可通过【遗失的尊敬】、【迟来的钦佩】等道具提升。' }] }],
    [],
    { force_2: { maxLevel: '友好' } }
  );
  assert(books[0]?.lane === 'item' && books[0]?.maxLevel === '亲密' && books[0]?.maxLevelOverride === '友好', '道具分类，毕业可改');
  assert(books[0]?.recommendedItem === '遗失的尊敬 / 迟来的钦佩', '道具名从获取说明抽出');
});
