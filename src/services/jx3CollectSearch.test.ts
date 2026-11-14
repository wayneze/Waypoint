import { expect, test } from 'vitest';
import {
  matchSlotKeyword,
  inferHitDungeon,
  detectDynamicSets,
  groupJx3Collect,
  parseJx3BoxItemSearch,
  resolveJx3SearchTokens,
  loadJx3CharCollect,
  loadJx3Collect,
  saveJx3CharCollect,
  saveJx3Collect,
} from './jx3Collect.ts';
import { progressEntries, progressReplace, progressRevision } from './progress';
import type { Jx3BoxDungeon, Jx3BoxItemHit, Jx3CollectStore } from '../types/index.ts';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('切换或重关联收藏只读取所属角色，不写共享区；缺少布局也能读回物品', () => {
  const dungeons: Jx3BoxDungeon[] = [{ key: '荻花圣殿', name: '荻花圣殿', era: '巴蜀风云', eraLevel: '80', maps: [], bosses: [] }];
  const make = (id: string): Jx3CollectStore => ({
    items: [{ id, name: id, dungeonKey: '英雄荻花圣殿', dungeonName: '英雄荻花圣殿', kind: 'special', source: 'box', obtained: false }],
    pinnedDungeons: [], collapsed: {},
  });
  saveJx3Collect(make('shared'));
  saveJx3CharCollect('collect-a', make('a'));
  progressReplace('jx3.collect', make('b').items.map(item => [item.id, item]), 'collect-b');
  const before = progressRevision.value;
  const shared = progressEntries('jx3.collect', '');
  const rawA = progressEntries('jx3.collect', 'collect-a');
  expect(loadJx3CharCollect('collect-a', dungeons).items[0]?.dungeonKey).toBe('荻花圣殿');
  expect(loadJx3CharCollect('collect-b', dungeons).items.map(item => item.id)).toEqual(['b']);
  expect(loadJx3CharCollect('collect-new', dungeons).items).toEqual([]);
  expect(loadJx3Collect(dungeons).items.map(item => item.id)).toEqual(['shared']);
  expect(progressEntries('jx3.collect', '')).toEqual(shared);
  expect(progressEntries('jx3.collect', 'collect-a')).toEqual(rawA);
  expect(progressRevision.value).toBe(before);
});

test('jx3CollectSearch', () => {
  // 1. 部位识别测试
  assert(matchSlotKeyword('破虏·潜行衣') === 'body', '破虏·潜行衣 识别为衣服');
  assert(matchSlotKeyword('破虏·潜行冠') === 'head', '破虏·潜行冠 识别为头部');
  assert(matchSlotKeyword('修罗·腰带') === 'belt', '修罗·腰带 识别为腰带');
  assert(matchSlotKeyword('定国·护腕') === 'wrist', '定国·护腕 识别为护腕');
  assert(matchSlotKeyword('雷神·鞋履') === 'shoes', '雷神·鞋履 识别为鞋履');

  // 2. 模拟全量秘境数据
  const mockDungeons: Jx3BoxDungeon[] = [
    {
      key: 'd_xianlv',
      name: '永王行宫·仙侣庭园',
      era: '剑胆琴心',
      eraLevel: '95',
      maps: [],
      bosses: ['屈焰阳', '森九绫', '李玚', '仲虚念', '李璘']
    },
    {
      key: 'd_huayue',
      name: '永王行宫·花月仙踪',
      era: '剑胆琴心',
      eraLevel: '95',
      maps: [],
      bosses: ['韩非池', '莫雨', '李白']
    },
    {
      key: 'd_dihua',
      name: '英雄荻花圣殿',
      era: '巴蜀风云',
      eraLevel: '80',
      maps: [],
      bosses: ['卫栖梧', '阿萨辛', '慕容追风', '沙利亚']
    },
    {
      key: 'd_zhulong',
      name: '烛龙殿',
      era: '巴蜀风云',
      eraLevel: '80',
      maps: [],
      bosses: ['索迪尔', '乌蒙贵', '雷神', '陆寻', '醉不归']
    },
    {
      key: 'd_lenglong',
      name: '冷龙峰',
      era: '雾海寻龙',
      eraLevel: '120',
      maps: [],
      bosses: ['葛木寒', '雨轻红', '喜雅', '鹰眼客', '赤幽明']
    },
    {
      key: 'd_heilao',
      name: '武狱黑牢',
      era: '群侠万变',
      eraLevel: '120',
      maps: [],
      bosses: ['时风', '乐临川', '牛波', '和正', '武云阙', '翁幼之']
    }
  ];

  // 3. 动态推断测试
  // 3.1 经典挂件自动推断
  const yehuaHit: Jx3BoxItemHit = {
    id: '1699',
    name: '夜话白鹭',
    typeLabel: '背部挂件'
  };
  const inferredYehua = inferHitDungeon(yehuaHit, mockDungeons);
  assert(inferredYehua?.dungeonName?.includes('荻花') === true, '夜话白鹭 自动推断为荻花圣殿');
  assert(inferredYehua?.boss === '卫栖梧 (三号首领)', '夜话白鹭 自动推断为卫栖梧');

  // 3.2 根据 Boss 名字模糊反向推断（即使是任意冷门物品）
  const customBossHit: Jx3BoxItemHit = {
    id: '99999',
    name: '未知秘境宝物',
    getType: '仙侣 李玚掉落'
  };
  const inferredBoss = inferHitDungeon(customBossHit, mockDungeons);
  assert(inferredBoss?.dungeonName?.includes('仙侣') === true, '根据 Boss 名动态推断为仙侣庭园');
  assert(inferredBoss?.boss === '李玚', '命中 Boss 李玚');

  // 3.3 蚩灵套单品精准推断测试（用户反馈的问题点）
  const chilingHit: Jx3BoxItemHit = {
    id: '20241',
    name: '蚩灵·逸然衣',
    requireLevel: 80,
    getType: '道具换取'
  };
  const inferredChiling = inferHitDungeon(chilingHit, mockDungeons);
  assert(inferredChiling?.dungeonName?.includes('荻花') === true, '蚩灵·逸然衣 自动推断为荻花圣殿');
  assert(inferredChiling?.boss?.includes('沙利亚') === true, '蚩灵·逸然衣 自动命中沙利亚');

  // 3.4 烛天套单品精准推断测试（严格验证绝对不能错误识别为荻花圣殿）
  const zhutianHit: Jx3BoxItemHit = {
    id: '30001',
    name: '烛天·逸然衣',
    requireLevel: 80,
    getType: '道具换取'
  };
  const inferredZhutian = inferHitDungeon(zhutianHit, mockDungeons);
  assert(inferredZhutian?.dungeonName?.includes('烛龙') === true, '烛天·逸然衣 必须精准推断为烛龙殿');
  assert(!inferredZhutian?.dungeonName?.includes('荻花'), '烛天·逸然衣 绝不能错误显示为荻花圣殿');
  assert(inferredZhutian?.boss?.includes('乌蒙贵') === true, '烛天衣服掉落为乌蒙贵');

  // 3.5 未知冷门散件（魔盒无出处且不在校服图谱），验证绝不硬编码强行归属
  const unknownScatteredHit: Jx3BoxItemHit = {
    id: '99998',
    name: '无名江湖散装',
    requireLevel: 80,
    getType: '道具换取'
  };
  const inferredUnknown = inferHitDungeon(unknownScatteredHit, mockDungeons);
  assert(inferredUnknown === undefined, '未知散件不瞎猜、绝不硬编码为荻花');

  // 4. 通用动态套装聚合测试（非硬编码：验证一个完全不在预设 JSON 里的全新自定义套装）
  const customDynamicHits: Jx3BoxItemHit[] = [
    {
      id: '901',
      name: '云梦·流光衣',
      getType: '仙侣庭园 掉落'
    },
    {
      id: '902',
      name: '云梦·流光冠',
      getType: '装备'
    },
    {
      id: '903',
      name: '云梦·流光带',
      getType: '装备'
    }
  ];

  // 对单品进行魔盒文本倒排推断，并测试同套继承
  customDynamicHits.forEach(h => {
    h.inferredDungeon = inferHitDungeon(h, mockDungeons);
  });
  assert(customDynamicHits[0].inferredDungeon?.dungeonName?.includes('仙侣') === true, '衣服根据文本掉落识别为仙侣');

  const customDynamicSets = detectDynamicSets(customDynamicHits, mockDungeons, []);
  assert(customDynamicSets.length === 1, '完全未预设的全新套装被自动聚合出来');
  assert(customDynamicSets[0].seriesName === '云梦套', '正确聚合为 云梦套');
  assert(customDynamicSets[0].pieces.body?.name === '云梦·流光衣', '衣服部件正确匹配单品');
  assert(customDynamicSets[0].pieces.head?.name === '云梦·流光冠', '头部部件正确匹配单品');
  assert(customDynamicSets[0].pieces.belt?.name === '云梦·流光带', '腰带部件正确匹配单品');
  assert(customDynamicSets[0].pieces.belt?.dungeonName?.includes('仙侣') === true, '腰带成功继承同套衣服已识别出的仙侣秘境');

  // 5. 经典校服套装整体聚合（验证蚩灵套与烛天套）
  const chilingHits: Jx3BoxItemHit[] = [
    { id: '201', name: '蚩灵·逸然衣', requireLevel: 80 },
    { id: '202', name: '蚩灵·逸然冠', requireLevel: 80 },
    { id: '203', name: '蚩灵·逸然带', requireLevel: 80 }
  ];
  const chilingSets = detectDynamicSets(chilingHits, mockDungeons, []);
  assert(chilingSets.length === 1, '蚩灵套被正确聚合');
  assert(chilingSets[0].seriesName === '蚩灵套', '聚合名为 蚩灵套');
  assert(chilingSets[0].pieces.body?.dungeonName?.includes('荻花') === true, '蚩灵衣服归属荻花');
  assert(chilingSets[0].pieces.body?.boss?.includes('沙利亚') === true, '蚩灵衣服掉落为沙利亚');
  assert(chilingSets[0].pieces.belt?.boss?.includes('辽东双雄') === true, '蚩灵腰带掉落为辽东双雄');

  const zhutianHits: Jx3BoxItemHit[] = [
    { id: '301', name: '烛天·逸然衣', requireLevel: 80 },
    { id: '302', name: '烛天·逸然冠', requireLevel: 80 }
  ];
  const zhutianSets = detectDynamicSets(zhutianHits, mockDungeons, []);
  assert(zhutianSets.length === 1, '烛天套被正确聚合');
  assert(zhutianSets[0].seriesName === '烛天套', '聚合名为 烛天套');
  assert(zhutianSets[0].pieces.body?.dungeonName?.includes('烛龙') === true, '烛天衣服精准归属烛龙殿');
  assert(!zhutianSets[0].pieces.body?.dungeonName?.includes('荻花'), '绝对不能是荻花圣殿');

  // 6. 已勾选心愿沉底测试
  const collectTestStore = {
    items: [
      { id: '1', name: '夜话白鹭', dungeonKey: 'd_dihua', kind: 'pendant' as const, obtained: true },
      { id: '2', name: '蚩灵·逸然衣', dungeonKey: 'd_dihua', kind: 'token' as const, obtained: false },
      { id: '3', name: '蚩灵·逸然冠', dungeonKey: 'd_dihua', kind: 'token' as const, obtained: true },
      { id: '4', name: '蚩灵·逸然带', dungeonKey: 'd_dihua', kind: 'token' as const, obtained: false }
    ],
    pinnedDungeons: [],
    collapsed: {}
  };
  const grouped = groupJx3Collect(collectTestStore as any, mockDungeons);
  const dihuaGroup = grouped.find(g => g.dungeonKey === 'd_dihua')!;
  assert(dihuaGroup.items.length === 4, '荻花心愿数量为 4');
  assert(dihuaGroup.items[0].obtained === false, '第 1 个是未勾选');
  assert(dihuaGroup.items[1].obtained === false, '第 2 个是未勾选');
  assert(dihuaGroup.items[2].obtained === true, '第 3 个已勾选沉底');
  assert(dihuaGroup.items[3].obtained === true, '第 4 个已勾选沉底');
  assert(dihuaGroup.items[2].name === '夜话白鹭', '挂件在沉底区前');
  assert(dihuaGroup.items[3].name === '蚩灵·逸然冠', '牌子在沉底区后');

  // 7. 破虏套搜索测试（验证搜“破虏”无论是否有单品均能秒出破虏套，且名字精准包含的单品排在最前）
  const instantPolu = detectDynamicSets([], mockDungeons, [], '破虏');
  assert(instantPolu.length === 1, '输入“破虏”秒出破虏套');
  assert(instantPolu[0].seriesName === '破虏套', '名称为破虏套');
  assert(instantPolu[0].pieces.body?.dungeonName?.includes('秦皇陵') === true, '破虏衣服出处包含秦皇陵');
  assert(instantPolu[0].pieces.belt?.dungeonName?.includes('稻香村') === true, '破虏腰带出处包含稻香村');

  // 测试魔盒搜索结果按名字相关性排序（破虏单品排在最前，排斥诗句干扰）
  const mockBoxRaw = {
    code: 200,
    data: {
      data: [
        { id: '1', Name: '西山秘宝·驰电掣风', Desc: '破虏寇' },
        { id: '2', Name: '长忌孤袭', Desc: '长忌破虏新策勋' },
        { id: '3', Name: '破虏·逸然衣', TypeLabel: '上衣' },
        { id: '4', Name: '破虏·逸然冠', TypeLabel: '帽子' }
      ]
    }
  };
  const parsedHits = parseJx3BoxItemSearch(mockBoxRaw, mockDungeons, '破虏');
  assert(parsedHits[0].name.startsWith('破虏'), '破虏单品经过相关性重排排在首位');
  assert(parsedHits[1].name.startsWith('破虏'), '破虏单品排在第二位');

  // 8. 别名与易错字容错联想测试（重点覆盖“溯雪”->“朔雪套”）
  // 8.1 搜索词解析关联：输入玩家高频错字“溯雪”，必须能够解析出官方正名“朔雪”
  const shuoxueTerms = resolveJx3SearchTokens('溯雪');
  assert(shuoxueTerms.includes('溯雪'), '检索词包含原词“溯雪”');
  assert(shuoxueTerms.includes('朔雪'), '检索词关联出官方正名“朔雪”');

  // 8.2 0 延迟秒出卡片：输入“溯雪”，本地图鉴立刻秒级识别出【朔雪套】（夜守 & 逐虎）
  const instantShuoxue = detectDynamicSets([], mockDungeons, [], '溯雪');
  assert(instantShuoxue.length === 1, '输入“溯雪”秒级识别出朔雪套');
  assert(instantShuoxue[0].seriesName === '朔雪套', '名称为官方正名 朔雪套');
  assert(instantShuoxue[0].pieces.head.name.includes('朔雪'), '头部为朔雪·冠/帽');
  assert(instantShuoxue[0].pieces.body.boss?.includes('逐虎') === true, '衣服归属逐虎');
  assert(instantShuoxue[0].pieces.head.boss?.includes('夜守') === true, '帽子归属夜守');

  // 8.3 魔盒单品关联：从魔盒拉回的正名单品（如《朔雪·逸然衣》），在搜“溯雪”时依然能精准挂载到卡片部位
  const mockShuoxueBoxHits: Jx3BoxItemHit[] = [
    { id: '1001', name: '朔雪·逸然衣', requireLevel: 90 },
    { id: '1002', name: '朔雪·逸然冠', requireLevel: 90 }
  ];
  const shuoxueSetsWithHits = detectDynamicSets(mockShuoxueBoxHits, mockDungeons, [], '溯雪');
  assert(shuoxueSetsWithHits.length === 1, '带单品也能成功识别为朔雪套');
  assert(shuoxueSetsWithHits[0].pieces.body.name === '朔雪·逸然衣', '衣服部件成功绑定真实的魔盒单品');
  assert(shuoxueSetsWithHits[0].pieces.head.name === '朔雪·逸然冠', '帽子部件成功绑定真实的魔盒单品');

  // 8.4 其它经典别名（燕虞、破鲁、嗤灵、迟冥）验证
  const instantYanyu = detectDynamicSets([], mockDungeons, [], '燕虞');
  assert(instantYanyu.length === 1 && instantYanyu[0].seriesName === '雁虞套', '输入“燕虞”秒识别“雁虞套”');
  const instantPoluAlias = detectDynamicSets([], mockDungeons, [], '破鲁');
  assert(instantPoluAlias.length === 1 && instantPoluAlias[0].seriesName === '破虏套', '输入“破鲁”秒识别“破虏套”');
  const instantChilingAlias = detectDynamicSets([], mockDungeons, [], '嗤灵');
  assert(instantChilingAlias.length === 1 && instantChilingAlias[0].seriesName === '蚩灵套', '输入“嗤灵”秒识别“蚩灵套”');
  const instantChimingAlias = detectDynamicSets([], mockDungeons, [], '迟冥');
  assert(instantChimingAlias.length === 1 && instantChimingAlias[0].seriesName === '驰冥套', '输入“迟冥”秒识别“驰冥套”');

  // 9. 展锋套测试（120年代初期 西津渡）
  const instantZhanfeng = detectDynamicSets([], mockDungeons, [], '展锋');
  assert(instantZhanfeng.length === 1, '输入“展锋”秒出展锋套');
  assert(instantZhanfeng[0].seriesName === '展锋套', '名称为展锋套');
  assert(instantZhanfeng[0].pieces.head.name.includes('展锋'), '头部包含展锋');
  assert(instantZhanfeng[0].pieces.head.boss?.includes('藤原佑野') === true, '帽子掉落为藤原佑野');
  assert(instantZhanfeng[0].pieces.body.boss?.includes('李重茂') === true, '衣服掉落为李重茂');
  assert(instantZhanfeng[0].pieces.belt.boss?.includes('苏凤楼') === true, '腰带掉落为苏凤楼');
  assert(instantZhanfeng[0].pieces.wrist.boss?.includes('刘展') === true, '护腕掉落为刘展');
  assert(instantZhanfeng[0].pieces.shoes.boss?.includes('韩敬青') === true, '鞋履掉落为韩敬青');

  // 9.1 魔盒展锋单品匹配（逸然衫、逸然护腰、逸然护手等）
  const mockZhanfengHits: Jx3BoxItemHit[] = [
    { id: '194740', name: '展锋·逸然护手', requireLevel: 120 },
    { id: '194741', name: '展锋·逸然护腰', requireLevel: 120 },
    { id: '194742', name: '展锋·逸然衫', requireLevel: 120 }
  ];
  const zhanfengSetsWithHits = detectDynamicSets(mockZhanfengHits, mockDungeons, [], '展锋');
  assert(zhanfengSetsWithHits.length === 1, '展锋单品成功聚合');
  assert(zhanfengSetsWithHits[0].pieces.body.name === '展锋·逸然衫', '衣服部件绑定展锋·逸然衫');
  assert(zhanfengSetsWithHits[0].pieces.belt.name === '展锋·逸然护腰', '腰带部件绑定展锋·逸然护腰');
  // 10. 揽江套与濯心套测试（120年代中后期 武狱黑牢 / 九老洞）
  // 10.1 揽江套：输入“揽江”秒级识别出【揽江套】（武狱黑牢），且 5 部位及首领准确
  const instantLanjiang = detectDynamicSets([], mockDungeons, [], '揽江');
  assert(instantLanjiang.length === 1, '输入“揽江”秒级识别出揽江套');
  assert(instantLanjiang[0].seriesName === '揽江套', '名称为揽江套');
  assert(instantLanjiang[0].pieces.head.name.includes('揽江'), '头部包含揽江');
  assert(instantLanjiang[0].pieces.head.boss?.includes('和正') === true, '帽子掉落为和正');
  assert(instantLanjiang[0].pieces.body.boss?.includes('武云阙') === true, '衣服掉落为武云阙');
  assert(instantLanjiang[0].pieces.belt.boss?.includes('乐临川') === true, '腰带掉落为乐临川');
  assert(instantLanjiang[0].pieces.wrist.boss?.includes('时风') === true, '护腕掉落为时风');
  assert(instantLanjiang[0].pieces.shoes.boss?.includes('牛波') === true, '鞋履掉落为牛波');

  // 10.2 魔盒揽江单品挂载（《揽江·逸然衣》、《揽江·逸然冠》等）
  const mockLanjiangHits: Jx3BoxItemHit[] = [
    { id: '104354', name: '揽江·逸然衣', requireLevel: 120 },
    { id: '104358', name: '揽江·逸然冠', requireLevel: 120 }
  ];
  const lanjiangSetsWithHits = detectDynamicSets(mockLanjiangHits, mockDungeons, [], '揽江');
  assert(lanjiangSetsWithHits.length === 1, '揽江单品成功关联');
  assert(lanjiangSetsWithHits[0].pieces.body.name === '揽江·逸然衣', '衣服部件绑定揽江·逸然衣');
  assert(lanjiangSetsWithHits[0].pieces.head.name === '揽江·逸然冠', '帽子部件绑定揽江·逸然冠');

  // 10.3 濯心套（九老洞）与“淮心”形近字容错
  const instantZhuoxin = detectDynamicSets([], mockDungeons, [], '濯心');
  assert(instantZhuoxin.length === 1, '输入“濯心”秒级识别出濯心套');
  const instantHuaixin = detectDynamicSets([], mockDungeons, [], '淮心');
  assert(instantHuaixin.length === 1 && instantHuaixin[0].seriesName === '濯心套', '输入形近错别字“淮心”也能秒级自动纠正识别出濯心套');
  const searchTermsHuaixin = resolveJx3SearchTokens('淮心');
  assert(searchTermsHuaixin.includes('濯心'), '搜淮心时自动派生出正名濯心进行魔盒检索');
  // 11. 防风物古诗词污染首领测试（彻底杜绝将古诗词如“牡丹架暖眠春昼”中的“牡丹”误判为荻花圣殿/百战首领）
  const mockHonghuiHits: Jx3BoxItemHit[] = [
    {
      id: '98369',
      name: '鸿辉·眠狸衣',
      getType: '道具换取',
      getSource: 'NPC商店 · 噗噗(万灵山庄)',
      desc: '牡丹架暖眠春昼，薄荷香浓醉晓晴。',
      requireLevel: 120
    },
    {
      id: '104369',
      name: '鸿辉·逸然衣',
      getType: '道具换取',
      getSource: 'NPC商店 · 董闻月(南诏段氏)',
      desc: '逸兴逍游天地阔，清风有梦意长留。',
      requireLevel: 120
    },
    {
      id: '104370',
      name: '鸿辉·逸然冠',
      getType: '道具换取',
      getSource: 'NPC商店 · 董闻月(南诏段氏)',
      desc: '逸兴逍游天地阔，清风有梦意长留。',
      requireLevel: 120
    }
  ];
  // 11. 鸿辉套与冷龙峰出处识别测试 & 防古诗“牡丹”污染荻花测试
  // 11.1 单品推断：古诗含有“牡丹”绝不能误识别为荻花圣殿或百战异闻录，且基于图谱权威精准归属【冷龙峰】
  const mianliInferred = inferHitDungeon(mockHonghuiHits[0], mockDungeons);
  assert(!mianliInferred?.dungeonName?.includes('荻花'), '鸿辉·眠狸衣 诗句含“牡丹”，绝不能误判为荻花圣殿！');
  assert(!mianliInferred?.dungeonName?.includes('百战'), '鸿辉·眠狸衣 绝不能误判为百战异闻录！');
  assert(mianliInferred?.dungeonName?.includes('冷龙峰') === true, '鸿辉·眠狸衣 精准归属于 冷龙峰');
  assert(mianliInferred?.boss?.includes('赤幽明') === true, '鸿辉·眠狸衣 衣服部位归属首领 赤幽明');

  // 11.2 非校服杂物含“牡丹”诗句，绝不瞎猜为荻花圣殿
  const unrelatedItemWithPeony = {
    id: '99999',
    name: '风物志·牡丹谱',
    getType: '任务奖励',
    getSource: '任务',
    desc: '绝代只西子，众芳惟牡丹。',
    requireLevel: 120
  };
  const unrelatedInferred = inferHitDungeon(unrelatedItemWithPeony, mockDungeons);
  assert(unrelatedInferred === undefined, '非荻花秘境产出的含牡丹诗句杂物保持未归属状态，绝不误判为荻花圣殿');

  // 11.3 套装秒出：搜“鸿辉”0延迟秒级识别【鸿辉套】（冷龙峰），首领完整映射
  const instantHonghui = detectDynamicSets([], mockDungeons, [], '鸿辉');
  assert(instantHonghui.length === 1, '搜“鸿辉”秒出鸿辉套');
  assert(instantHonghui[0].seriesName === '鸿辉套', '名称为鸿辉套');
  assert(instantHonghui[0].pieces.head.dungeonName?.includes('冷龙峰') === true, '鸿辉帽子归属冷龙峰');
  assert(instantHonghui[0].pieces.head.boss?.includes('鹰眼客') === true, '鸿辉帽子首领为鹰眼客');
  assert(instantHonghui[0].pieces.body.dungeonName?.includes('冷龙峰') === true, '鸿辉衣服归属冷龙峰');
  assert(instantHonghui[0].pieces.body.boss?.includes('赤幽明') === true, '鸿辉衣服首领为赤幽明');
  assert(instantHonghui[0].pieces.belt.boss?.includes('雨轻红') === true, '鸿辉腰带首领为雨轻红');
  assert(instantHonghui[0].pieces.wrist.boss?.includes('葛木寒') === true, '鸿辉护腕首领为葛木寒');
  assert(instantHonghui[0].pieces.shoes.boss?.includes('喜雅') === true, '鸿辉鞋履首领为喜雅');

  // 11.4 魔盒单品关联：单品聚合时挂载魔盒真实物品，且秘境与首领正确
  const honghuiSets = detectDynamicSets(mockHonghuiHits, mockDungeons, [], '鸿辉');
  assert(honghuiSets.length === 1, '鸿辉套带魔盒单品正常聚合');
  assert(honghuiSets[0].seriesName === '鸿辉套', '名称为鸿辉套');
  assert(honghuiSets[0].pieces.body?.name === '鸿辉·眠狸衣', '衣服挂载魔盒单品 鸿辉·眠狸衣');
  assert(honghuiSets[0].pieces.body?.dungeonName?.includes('冷龙峰') === true, '鸿辉套衣服归属冷龙峰');
  assert(honghuiSets[0].pieces.head?.dungeonName?.includes('冷龙峰') === true, '鸿辉套帽子归属冷龙峰');
  assert(!honghuiSets[0].pieces.body?.dungeonName?.includes('荻花'), '鸿辉套衣服绝不能是荻花圣殿！');
  assert(!honghuiSets[0].pieces.head?.dungeonName?.includes('荻花'), '鸿辉套帽子绝不能是荻花圣殿！');

  // 12. 文王套（永王行宫·仙侣庭园 & 永王行宫·花月别院）跨副本分流测试
  const mockWenwangHits: Jx3BoxItemHit[] = [
    {
      id: '7_104013',
      name: '文王·逸然衣',
      typeLabel: '上衣',
      getType: '道具换取',
      belongSchool: '段氏',
      requireLevel: 95
    },
    {
      id: '7_104011',
      name: '文王·逸然靴',
      typeLabel: '鞋子',
      getType: '道具换取',
      belongSchool: '段氏',
      requireLevel: 95
    }
  ];

  // 12.1 单品推断：衣服归属仙侣庭园李玚，鞋子归属花月别院李白
  const wenwangYiInferred = inferHitDungeon(mockWenwangHits[0], mockDungeons);
  assert(wenwangYiInferred?.dungeonName?.includes('仙侣庭园') === true, '文王·逸然衣 归属于 仙侣庭园');
  assert(wenwangYiInferred?.boss?.includes('李玚') === true, '文王·逸然衣 衣服归属首领 李玚');

  const wenwangXueInferred = inferHitDungeon(mockWenwangHits[1], mockDungeons);
  assert(wenwangXueInferred?.dungeonName?.includes('花月') === true, '文王·逸然靴 归属于 花月');
  assert(wenwangXueInferred?.boss?.includes('李白') === true, '文王·逸然靴 鞋履归属首领 李白');

  // 12.2 套装秒出：搜“文王”秒级出文王套，头/衣在仙侣，腰/腕/鞋在花月
  const instantWenwang = detectDynamicSets([], mockDungeons, [], '文王');
  assert(instantWenwang.length === 1, '搜“文王”秒出文王套');
  assert(instantWenwang[0].seriesName === '文王套', '名称为文王套');
  assert(instantWenwang[0].pieces.head.dungeonName?.includes('仙侣庭园') === true, '文王头归属仙侣庭园');
  assert(instantWenwang[0].pieces.body.dungeonName?.includes('仙侣庭园') === true, '文王衣归属仙侣庭园');
  assert(instantWenwang[0].pieces.belt.dungeonName?.includes('花月') === true, '文王腰归属花月别院');
  assert(instantWenwang[0].pieces.wrist.dungeonName?.includes('花月') === true, '文王腕归属花月别院');
  assert(instantWenwang[0].pieces.shoes.dungeonName?.includes('花月') === true, '文王鞋归属花月别院');

  // 13. 择芳套（白帝江关）
  const instantZefang = detectDynamicSets([], mockDungeons, [], '择芳');
  assert(instantZefang.length === 1, '搜“择芳”秒出择芳套');
  assert(instantZefang[0].pieces.body.dungeonName?.includes('白帝江关') === true, '择芳衣归属白帝江关');

  // 14. 鸣沙套与朔漠套（一之窟）
  const instantMingsha = detectDynamicSets([], mockDungeons, [], '鸣沙');
  assert(instantMingsha.length === 1, '搜“鸣沙”秒出鸣沙套');
  assert(instantMingsha[0].pieces.body.dungeonName?.includes('一之窟') === true, '鸣沙衣归属一之窟');
  assert(instantMingsha[0].pieces.body.boss?.includes('苏什') === true, '鸣沙衣首领为苏什');

  const instantShuomo = detectDynamicSets([], mockDungeons, [], '朔漠');
  assert(instantShuomo.length === 1, '搜“朔漠”秒出朔漠套');
  assert(instantShuomo[0].pieces.head.dungeonName?.includes('一之窟') === true, '朔漠头归属一之窟');

  // 15. 天韶套（世外蓬莱·冰火岛）
  const mockTianshaoHit: Jx3BoxItemHit = {
    id: '7_42010',
    name: '天韶·逸然衣',
    typeLabel: '上衣',
    getType: '道具换取',
    belongSchool: '藏剑',
    requireLevel: 100
  };
  const tianshaoInferred = inferHitDungeon(mockTianshaoHit, mockDungeons);
  assert(tianshaoInferred?.dungeonName?.includes('冰火') === true, '天韶·逸然衣 单品推断归属冰火岛');

  const instantTianshao = detectDynamicSets([], mockDungeons, [], '天韶');
  assert(instantTianshao.length === 1, '搜“天韶”秒出天韶套');
  assert(instantTianshao[0].seriesName === '天韶套', '名称为天韶套');
  assert(instantTianshao[0].pieces.body.dungeonName?.includes('冰火') === true, '天韶衣归属冰火岛');

  // 16. 夺夜套（范阳夜变）
  const mockDuoyeHit: Jx3BoxItemHit = {
    id: '7_104079',
    name: '夺夜·逸然衣',
    typeLabel: '上衣',
    getType: '道具换取',
    belongSchool: '藏剑',
    requireLevel: 100
  };
  const duoyeInferred = inferHitDungeon(mockDuoyeHit, mockDungeons);
  assert(duoyeInferred?.dungeonName?.includes('范阳') === true, '夺夜·逸然衣 单品推断归属范阳夜变');
  assert(duoyeInferred?.boss?.includes('安小逢') === true, '夺夜·逸然衣 衣服部位归属首领 安小逢');

  const instantDuoye = detectDynamicSets([], mockDungeons, [], '夺夜');
  assert(instantDuoye.length === 1, '搜“夺夜”秒出夺夜套');
  assert(instantDuoye[0].seriesName === '夺夜套', '名称为夺夜套');
  assert(instantDuoye[0].pieces.body.dungeonName?.includes('范阳') === true, '夺夜衣归属范阳夜变');
  assert(instantDuoye[0].pieces.head.dungeonName?.includes('范阳') === true, '夺夜头归属范阳夜变');
});
