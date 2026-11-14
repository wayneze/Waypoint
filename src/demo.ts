import type { CustomMemo, GameRecord, Jx3CollectItem, Jx3TokenSlot, TaskItem, VaultItem } from './types';
import fbMap from './content/jx3/fb-map.json';
import pendants from './content/jx3/pendants.json';
import tokenSets from './content/jx3/set-tokens.json';

// 独立开发入口；不加入正式构建，也不向普通预览或桌面数据库写入样例。
const demoOrigin = 'http://waypoint-demo.localhost:1422';
const marker = 'waypoint_demo';

function seedDemo() {
  if (!import.meta.env.DEV || import.meta.env.MODE !== 'demo'
    || location.origin !== demoOrigin || '__TAURI_INTERNALS__' in window) {
    throw new Error('请运行 pnpm demo，并打开 http://waypoint-demo.localhost:1422/demo.html');
  }
  if (localStorage.length && localStorage.getItem(marker) !== '1') {
    throw new Error('此预览地址已有非演示数据。请用新的浏览器配置打开演示，现有数据未改动。');
  }

  const vault: VaultItem[] = [
    {
      id: 'demo-jx3', platform: '剑网3', account_name: '江湖漫游者',
      server: '梦江南', notes: '演示账号 · 所有角色与资产均为虚构',
      assets: [{ id: 'demo-time', name: '游戏时间', qty: '28 天' }],
      characters: [
        { id: 'demo-jx3-a', name: '听风', sect: '纯阳', track_daily: true, starred: true,
          assets: [{ id: 'demo-gold-a', name: '金币', qty: '12,800' }] },
        { id: 'demo-jx3-b', name: '枕流', sect: '万花', track_daily: true,
          assets: [{ id: 'demo-gold-b', name: '金币', qty: '6,400' }] },
      ],
    },
    {
      id: 'demo-sky', platform: '光遇', account_name: '追光旅人', server: '国服',
      notes: '演示账号 · 不关联查询 ID', assets: [],
      characters: [{ id: 'demo-sky-a', name: '拾光', track_daily: true,
        assets: [
          { id: 'demo-candle', name: '白蜡烛', qty: '186' },
          { id: 'demo-heart', name: '爱心', qty: '42' },
        ] }],
    },
    {
      id: 'demo-nsh', platform: '逆水寒', account_name: '山河来信', server: '烟雨江南',
      notes: '演示账号', assets: [],
      characters: [{ id: 'demo-nsh-a', name: '照溪', sect: '素问', track_daily: true,
        assets: [{ id: 'demo-jade', name: '纹玉', qty: '1,280' }] }],
    },
  ];

  const taskLists: [string, string[], string[], number][] = [
    ['demo-jx3-a', ['大战日常', '门派勤修', '茶馆相约', '江湖行记', '整理背包'], ['团队秘境 · 预约', '侠行点奖励'], 2],
    ['demo-jx3-b', ['大战日常', '门派勤修', '家园照料', '生活技艺'], ['周常任务', '声望补齐计划'], 1],
    ['demo-sky-a', ['每日任务', '云野跑图', '收集季节蜡烛', '好友赠心'], ['暴风眼献祭', '整理兑换清单'], 2],
  ];
  const tasks = taskLists.map(([character, daily, weekly, completed]) => [
    character,
    [...daily, ...weekly].map((title, i): TaskItem => ({
      id: `demo-task-${character}-${i}`, character_id: character, title,
      category: i < daily.length ? 'daily' : 'weekly',
      reset_rule: i < daily.length ? (character.includes('jx3') ? '07:00' : '00:00') : '周一 07:00',
      source_type: 'custom', done: i < completed,
    })),
  ] as const);

  const works: [number, string, GameRecord['status'], string, number, string[]][] = [
    [1245620, '艾尔登法环', 'playing', '交界地探索 · 58 小时', 9, ['开放世界', '角色扮演']],
    [1145360, 'Hades', 'playing', '第 24 次逃脱', 9, ['动作', 'Roguelike']],
    [413150, '星露谷物语', 'playing', '第二年 · 春', 10, ['农场模拟', '休闲']],
    [367520, '空洞骑士', 'completed', '圣巢探索完成', 10, ['类银河战士恶魔城', '冒险']],
    [105600, '泰拉瑞亚', 'backlog', '等待下一场冒险', 0, ['沙盒', '建造']],
    [588650, '死亡细胞', 'playing', '二细胞挑战', 9, ['动作', 'Roguelike']],
    [504230, 'Celeste', 'completed', '山巅留念', 10, ['平台跳跃', '独立游戏']],
    [1091500, '赛博朋克 2077', 'backlog', '夜之城来信', 0, ['开放世界', '角色扮演']],
  ];
  const records: GameRecord[] = works.map(([appid, title, status, progress, rating, tags]) => ({
    id: `demo-game-${appid}`, title, category: 'single', status, progress, rating,
    notes: '演示游玩记录 · 非真实账号数据',
    poster: `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`,
    guide_id: '',
    library: {
      installations: [],
      metadata: {
        source: 'steam', source_id: String(appid), title, original_title: title,
        description: '演示书库中的作品，游玩状态与进度均为虚构。',
        cover: `https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/${appid}/library_600x900.jpg`,
        screenshots: [], developer: '', release_date: '', score: 0,
        url: `https://store.steampowered.com/app/${appid}/`, tags,
      },
    },
  }));
  const now = new Date().toISOString();
  const memos: CustomMemo[] = [
    { id: 'demo-memo-1', title: '周末秘境小队', tag: '剑网3', cycle_type: 'weekly', cycle_val: '6,0',
      note: '约好队友再出发\n- [x] 确认时间\n- [ ] 准备药品\n- [ ] 整理掉落清单', pinned: true,
      has_progress: false, current_progress: 0, target_progress: 0, done: false, created_at: now },
    { id: 'demo-memo-2', title: '攒蜡烛，等下一次相遇', tag: '光遇', cycle_type: 'once',
      note: '慢慢收集喜欢的装扮，不必一次完成。', has_progress: true,
      current_progress: 186, target_progress: 250, done: false, created_at: now },
    { id: 'demo-memo-3', title: '给存档留一份备份', tag: '单机', cycle_type: 'interval', cycle_val: '7d',
      note: '每周整理一次正在游玩的作品。', has_progress: false,
      current_progress: 0, target_progress: 0, done: false, created_at: now },
    { id: 'demo-memo-4', title: '游戏之外，也记得休息', tag: '生活', cycle_type: 'daily',
      note: '喝水、活动一下肩颈，看看窗外。', has_progress: false,
      current_progress: 0, target_progress: 0, done: false, created_at: now },
    { id: 'demo-memo-5', title: '通关后的几句话', tag: '单机', cycle_type: 'once',
      note: '把喜欢的片段写下来，留给以后的自己。', has_progress: false,
      current_progress: 0, target_progress: 0, done: true, created_at: now },
  ];
  // 物品资料复用公共目录，只有拥有状态是虚构的。
  const tokenSet = tokenSets[0]!;
  const collect: Jx3CollectItem[] = [
    ...Object.entries(tokenSet.pieces).map(([slot, piece], i): Jx3CollectItem => ({
      id: `demo-token-${slot}`, dungeonKey: tokenSet.dungeon, dungeonName: tokenSet.dungeon,
      name: piece.name, kind: 'token', obtained: i === 0, source: 'seed',
      tokenSeriesId: tokenSet.id, tokenSlot: slot as Jx3TokenSlot, boss: piece.boss,
      note: '演示收集计划',
    })),
    ...pendants.slice(0, 4).map((item, i): Jx3CollectItem => ({
      id: `demo-pendant-${i}`, dungeonKey: item.dungeon, dungeonName: item.dungeon,
      name: item.name, kind: 'pendant', obtained: i % 2 === 0, source: 'seed',
      catalogId: item.id, boss: item.boss, difficulty: item.difficulty,
      note: '演示收集计划',
    })),
  ];
  const entries: Record<string, unknown> = {
    [marker]: '1',
    waypoint_vault_items: vault,
    waypoint_game_records: records,
    waypoint_custom_memos: memos,
    waypoint_active_character: 'demo-jx3-a',
    waypoint_active_character_jx3: 'demo-jx3-a',
    waypoint_active_character_sky: 'demo-sky-a',
    waypoint_active_tab: 'daily',
    waypoint_jx3_subtab: 'collect',
    waypoint_onboarding_done: true,
    waypoint_progress_debug: {
      items: collect.map(item => ['jx3.collect', 'demo-jx3-a', item.id, item]),
      checks: [],
    },
    'waypoint_cache:__build': { body: JSON.stringify(__BUILD_ID__), fetched_at: Date.now() },
    'waypoint_cache:jx3.box_dungeons': { body: JSON.stringify(fbMap), fetched_at: Date.now() },
  };
  for (const [character, rows] of tasks) entries[`waypoint_tasks_${character}`] = rows;

  // 这个 origin 在首次写入前必须为空，只有本入口拥有的数据才可重置。
  localStorage.clear();
  for (const [key, value] of Object.entries(entries)) {
    localStorage.setItem(key, typeof value === 'string' ? value : JSON.stringify(value));
  }
}

try {
  seedDemo();
  await import('./main');
} catch (error) {
  document.body.textContent = error instanceof Error ? error.message : String(error);
}
