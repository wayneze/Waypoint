import { test } from 'vitest';
import {
  buildJx3AdventureCatalog,
  classifySerendipity,
  collectNeedlesFromRows,
  jx3AdventureAssetUrl,
  jx3AdventureCoverUrl,
  parseCompleteAdventureHits,
  parseItemNamesPayload,
  parseSerendipityFront,
  relinkAdventureReputationRefs,
  rewardItemIds,
  rewardSummary,
  type Jx3AdventureNameMaps
} from './jx3AdventureCatalog';
import { computeAdventureProgress } from './jx3Data';
import type { Jx3ReputationItem } from '../types';

function assert(cond: unknown, msg: string) {
  if (!cond) throw new Error(msg);
}

test('jx3AdventureCatalog', () => {
  const maps: Jx3AdventureNameMaps = {
    achieveNameIds: {
      '驯马宗师': '3169',
      '套马宗师': '3155',
      '套马·伯乐之才': '3156',
      '捐马助军': '3257',
      '日行一善': '4487',
      '大公忘私！千': '1701',
      '一心一意': '1571',
      '善为人师': '1096',
      '傲意战魂！第十四阶': '1456'
    },
    seriesIds: {
      '对决！': ['561', '562', '842'],
      '五十胜！': ['214', '810']
    },
    completeAchieveIds: {
      '三尺青锋': '4606',
      '塞外宝驹': '5179',
      '黑白路': '4608',
      '茶馆奇缘': '4610',
      '宠物奇缘·换房记': '9999'
    }
  };

  const reps: Pick<Jx3ReputationItem, 'id' | 'name'>[] = [
    { id: 'rep_cangjian', name: '藏剑山庄' },
    { id: 'rep_dashaman', name: '大萨满' },
    { id: 'rep_yangzhou', name: '扬州' },
    { id: 'rep_luoyang', name: '洛阳' },
    { id: 'rep_changan', name: '长安' },
    { id: 'rep_faction', name: '浩气盟 / 恶人谷' }
  ];

  const sanchi = parseSerendipityFront(
    '扬州、洛阳、长安主城声望达到尊敬\n拥有200块[浮屠陨铁]\n加入门派',
    'adv_2',
    maps,
    reps
  );
  assert(sanchi.length === 5, `三尺青锋 5 lines got ${sanchi.length}`);
  assert(sanchi.filter(r => r.type === 'reputation').map(r => r.refId).join(',') === 'rep_yangzhou,rep_luoyang,rep_changan', '三城声望对上');
  assert(sanchi.filter(r => r.type !== 'reputation').every(r => r.match === 'unlinked'), '陨铁/门派未对账');
  assert(!sanchi.some(r => r.name.includes('煞')), '不再写煞·天蛛');
  const friendly = parseSerendipityFront('昆仑声望达到友善', 'friendly', maps, [{ id: 'force_kunlun', name: '昆仑' }]);
  assert(friendly[0].type === 'reputation' && friendly[0].minLevel === '友好' && friendly[0].refId === 'force_kunlun', '游戏原文友善按友好等级匹配');

  const horse = parseSerendipityFront(
    '完成[套马宗师]成就\n完成[驯马宗师]成就\n完成[套马·伯乐之才]成就\n完成[捐马助军]\n江湖资历达到3万点',
    'adv_21',
    maps,
    reps
  );
  assert(horse.find(r => r.refId === '3169')?.type === 'achievement', '驯马宗师 3169');
  assert(horse.find(r => r.refId === '3257')?.type === 'achievement', '捐马助军 无「成就」后缀也对上');
  assert(horse.find(r => r.type === 'point')?.targetVal === 30000, '资历 3 万');
  assert(!horse.some(r => r.name.includes('良驹')), '不再写良驹！千匹');

  const heibai = parseSerendipityFront(
    '加入阵营\n完成三个对立阵营领袖的[对决！]成就\n当前阵营声望达到尊敬\n完成[傲意战魂！第十四阶]成就\n完成所有战场[五十胜！]成就',
    'adv_5',
    maps,
    reps
  );
  const duijue = heibai.find(r => r.name.includes('对决'));
  assert(duijue?.seriesMode === 'any' && duijue.targetVal === 3, '对决！任意 3');
  assert(duijue?.seriesIds?.join(',') === '561,562,842', '对决 series');
  const wushi = heibai.find(r => r.name.includes('五十胜'));
  assert(wushi?.seriesMode === 'all' && wushi.seriesIds?.length === 2, '五十胜全齐');
  assert(heibai.find(r => r.refId === '1456'), '傲意战魂 ID');
  assert(heibai.find(r => r.name.includes('阵营声望'))?.reputationIds?.includes('rep_faction'), '当前阵营按可确认的任一方声望判定，不随意指定阵营');
  const stale = parseSerendipityFront('扬州、洛阳、长安主城声望达到尊敬', 'adv_old', maps, [{ id: 'rep_cangjian', name: '藏剑山庄' }]);
  assert(stale.every(r => !r.refId), 'old catalog misses city refs');
  assert(relinkAdventureReputationRefs([{
    id: 'adv_old', name: '三尺青锋', type: '绝世奇遇', reward: '', triggerDaily: '', minPoint: 0, isTriggered: false, requirements: stale
  }], reps)[0].requirements.every(r => !!r.refId), 'relink fills city refs');

  const yinyang = parseSerendipityFront(
    '完成[日行一善]成就或枫华谷战乱[未完结]任务\n江湖资历达到3万点',
    'adv_3',
    maps,
    reps
  );
  assert(yinyang[0].refId === '4487', '阴阳两界对日行一善而不是摔死');
  assert(yinyang[0].type === 'achievement', '日行一善是成就');
  const mixed = {
    id: 'mixed', name: '混合前置', type: '百家奇遇' as const, reward: '', triggerDaily: '', minPoint: 0, isTriggered: false,
    requirements: yinyang.slice(0, 1)
  };
  assert(computeAdventureProgress(mixed, new Set(['1']), new Map()).requirements[0].match === 'unlinked', '未读取任务，不能因成就未完成就否定或条件');
  assert(computeAdventureProgress(mixed, new Set(['4487']), new Map()).requirements[0].match === 'done', '或条件中已知成就完成即可确认');
  mixed.requirements = parseSerendipityFront('完成[日行一善]成就及[未知条件]成就', 'and', maps, reps);
  assert(computeAdventureProgress(mixed, new Set(['4487']), new Map()).requirements[0].match === 'unlinked', '未解析的必需条件不能被已完成成就覆盖');

  const tea = parseSerendipityFront(
    '江湖资历达到2万\n当前金币达到20砖\n完成[大公忘私！千]成就\n完成[一心一意]成就',
    'adv_7',
    maps,
    reps
  );
  assert(tea.find(r => r.refId === '1701'), '茶馆达人换成大公忘私');
  assert(tea.find(r => r.type === 'item' && r.name.includes('金币')), '20 砖未对账');

  assert(jx3AdventureAssetUrl('ui\\Image\\Adventure\\catalogue\\duyaoxing.tga') ===
    'https://img.jx3box.com/adventure/adventure/std/catalogue/duyaoxing.png', 'catalogue tga 转 PNG');
  assert(jx3AdventureCoverUrl({
    szFirstPagePath: 'ui\\Image\\Adventure\\catalogue\\duyaoxing.tga',
    szRewardPath: 'ui\\Image\\Adventure\\reward\\dyx.tga',
    szOpenRewardPath: 'ui\\Image\\Adventure\\reward\\Open\\duyaoxing_open.tga'
  }).includes('/reward/open/duyaoxing_open.png'), '封面用彩色奖励图，图鉴是褐色素描');
  assert(!jx3AdventureAssetUrl('ui\\Image\\Adventure\\reward\\hrzx\\'), '门派目录没有单张 tga 就空着');

  const catalog = buildJx3AdventureCatalog([
    { dwID: 21, szName: '塞外宝驹', szFront: '完成[驯马宗师]成就', nClassify: 2, bPerfect: true, szFirstPagePath: 'ui\\Image\\Adventure\\catalogue\\saiwaibaoju.tga' },
    { dwID: 149, szName: '换房记', nClassify: 1, szFront: null },
    { dwID: 2, szName: '三尺青锋', szFront: '加入门派', nClassify: 2, bPerfect: true }
  ], maps, reps);
  assert(catalog.find(a => a.name === '塞外宝驹')?.coverUrl?.includes('saiwaibaoju.png'), '目录带上封面');
  assert(catalog[0].name === '换房记' || catalog.map(a => a.name).includes('换房记'), '宠物在目录里');
  const pet = catalog.find(a => a.name === '换房记')!;
  assert(pet.type === '宠物奇遇', 'nClassify 1 = 宠物');
  assert(pet.completeAchieveId === '9999', '宠物完成奇遇按 宠物奇缘·名');
  assert(buildJx3AdventureCatalog(
    [{ dwID: 149, szName: '换房记', nClassify: 1, szFront: null }],
    { ...maps, completeAchieveIds: { '宠物奇缘换房记': '5194' } },
    reps
  )[0].completeAchieveId === '5194', '宠物完成奇遇压缩名也能对上');
  assert(pet.requirements.length === 0, '宠物无 szFront');
  const horseItem = catalog.find(a => a.name === '塞外宝驹')!;
  assert(horseItem.completeAchieveId === '5179', '完成奇遇 ID');
  assert(horseItem.type === '绝世奇遇', 'bPerfect = 绝世');
  assert(classifySerendipity({ nClassify: 2, bPerfect: null, szFront: '加入阵营' }) === '阵营奇遇', '阵营前置');

  const progress = computeAdventureProgress(horseItem, new Set(['3169', '5179']), new Map());
  assert(progress.completeMatch === 'done', '完成奇遇进快照即完成');
  assert(horseItem.isTriggered === false, '计算不改写共享奇遇目录');
  assert(progress.completedCount === 1, '驯马宗师对上');

  const heibaiAdv = {
    id: 'adv_5',
    name: '黑白路',
    type: '阵营奇遇' as const,
    reward: '',
    triggerDaily: '',
    minPoint: 0,
    completeAchieveId: '4608',
    isTriggered: false,
    requirements: heibai
  };
  const heibaiProg = computeAdventureProgress(heibaiAdv, new Set(['561', '562', '842']), new Map());
  assert(heibaiProg.requirements.find(r => r.name.includes('对决'))?.match === 'done', '对决任意 3 已齐');
  assert(heibaiProg.requirements.find(r => r.name.includes('五十胜'))?.match === 'missing', '五十胜未齐');
  assert(heibaiProg.completeMatch === 'missing', '奇遇本身未完成');
  const gold = tea.find(r => r.type === 'item')!;
  const teaAdv = {
    id: 'adv_7', name: '茶馆', type: '绝世奇遇' as const, reward: '', triggerDaily: '', minPoint: 0, isTriggered: false, requirements: tea
  };
  const marked = computeAdventureProgress(teaAdv, new Set(), new Map(), new Set([gold.id]));
  assert(marked.completedCount === 1, '手记只算被点的那条');
  assert(marked.requirements.find(r => r.id === gold.id)?.completed === true && gold.completed === false, '手记只作用于本次计算，不污染目录');

  const needles = collectNeedlesFromRows([{ szFront: '完成所有战场[五十胜！]成就\n完成[驯马宗师]成就\n获得[京衙卫佩包]挂件' }]);
  assert(needles.seriesPrefixes.includes('五十胜！'), '系列前缀抽出');
  assert(needles.achieveNames.includes('驯马宗师'), '单条成就名抽出');
  assert(!needles.achieveNames.includes('五十胜！'), '系列前缀不进精确搜');
  assert(!needles.achieveNames.includes('京衙卫佩包'), '道具不进成就搜');

  const completeMap = parseCompleteAdventureHits([
    { ID: 4606, Name: '三尺青锋', Desc: '完成奇遇《三尺青锋》。', ShortDesc: '完成奇遇《三尺青锋》' },
    { ID: 1, Name: '噪声', Desc: '其它' },
    { ID: 5194, Name: '宠物奇缘· 换房记', Desc: '完成奇遇《宠物奇缘·换房记》' }
  ]);
  assert(completeMap['三尺青锋'] === '4606', '完成奇遇名字映射');
  assert(completeMap['宠物奇缘换房记'] === '5194', '宠物完成奇遇忽略点号空格');
  assert(!completeMap['噪声'], '没有完成奇遇描述的普通成就不能当作奇遇完成记录');
});

test('jx3 adventure reward comes from RewdItem ids', () => {
  const names = parseItemNamesPayload({
    list: [
      { id: '5_87714', Name: '武技殊影图·蹑云逐月·尘影' },
      { id: '5_87715', Name: '武技殊影图·迎风回浪·尘影' },
      { id: '5_87716', Name: '武技殊影图·凌霄揽胜·尘影' },
      { id: '5_87739', Name: '绾雾青丝' }
    ]
  });
  const ids = rewardItemIds({ RewdItem: '5_87714;5_87715;5_87716;5_87717;5_87714;' });
  assert(ids.length === 4, '去重并忽略尾分号');
  assert(rewardSummary(ids, names) === '武技殊影图·蹑云逐月·尘影、武技殊影图·迎风回浪·尘影 等 3 件', '多件只列前两件，查不到名的不计');
  assert(rewardSummary(rewardItemIds({ RewdItem: '5_87739;' }), names) === '绾雾青丝', '单件');
  assert(rewardSummary(rewardItemIds({ RewdItem: null }), names) === '', '没有奖励物品');
  const [adv] = buildJx3AdventureCatalog(
    [{ dwID: 9, szName: '徊影悟心', RewdItem: '5_87739;' }],
    { achieveNameIds: {}, seriesIds: {}, completeAchieveIds: {} },
    [],
    names
  );
  assert(adv.reward === '绾雾青丝', '目录带上奖励');
});
