export interface Jx3ExteriorItem {
  kind: string;
  id: number;
  name: string;
}

export interface Jx3ExteriorSnapshot {
  role: string;
  server: string;
  items: Jx3ExteriorItem[];
  path: string;
}

export interface Jx3ExteriorScan {
  client_root: string;
  addon_installed: boolean;
  snapshots: Jx3ExteriorSnapshot[];
  warning?: string;
}

export interface Jx3GoodsHit {
  id: string;
  name: string;
  alias: string;
  price?: number;
  price_source?: string;
  url: string;
}

export interface Jx3GoodsQuote {
  id: string;
  name: string;
  alias: string;
  price?: number;
  price_source?: string;
  note_band: string;
  url: string;
  updated: string;
  failed_at: string;
  image_url?: string;
  icon_url?: string;
}

export interface Character {
  id: string;
  name: string;
  game_id: string;
  server?: string;
  sect?: string;
  is_active?: boolean;
  starred?: boolean;
}

export type Jx3CatalogKind = 'pendant' | 'reputation' | 'adventure' | 'token' | 'achievement' | 'special' | 'appearance' | 'boss';
export type Jx3CalendarSlotKey = 'war' | 'mine' | 'school' | 'battle' | 'rescue' | 'card' | 'leader' | 'raid' | 'conn';

export interface TaskItem {
  id: string;
  character_id: string;
  title: string;
  category: 'daily' | 'weekly' | 'custom';
  reset_rule: string;
  source_type: 'api_jx3' | 'fixed' | 'custom';
  api_key?: string;
  tag?: string;
  done: boolean;
  catalog_kind?: Jx3CatalogKind;
  catalog_id?: string;
  schedule_time?: string;
}

export interface CollectionItem {
  id: string;
  map: '晨岛' | '云野' | '雨林' | '霞谷' | '暮土' | '禁阁' | string;
  name: string;
  action?: string;
  redCandles: number;
  wings: number;
  location?: string;
  keyItems?: string;
  coreItem?: string;        // 核心装扮道具名称 (如 "矮人面具")
  coreCost?: string;        // 核心装扮消耗 (如 "20爱心")
  status: 'locked' | 'unlocked';
  wingUnlocked?: boolean;   // 永久光翼节点是否已拿 (暴风眼升华锁)
  itemObtained?: boolean;   // 核心装扮是否已兑换
  note?: string;
}

// 季节向导具体道具项（支持逐项标记【换了没】）
export interface SeasonalItem {
  id: string;
  name: string;             // 道具名称，如 "挂机木质吊床"、"野外充能火盆"
  costDesc: string;         // 消耗描述，如 "55白蜡"、"13爱心"
  candles: number;          // 白蜡单价
  ascended: number;         // 红蜡单价
  hearts: number;           // 爱心单价
  obtained: boolean;        // 核心标记：【换了没】！true=已换, false=待换
  icon?: string;
  slot?: string;
  wear?: string;
  spirit?: string;
  kind?: string;
}

export interface SeasonalGuide {
  id: string;
  sourceUrl?: string;
  season: string;
  shortName?: string;
  realm?: string;
  loc: string;
  quests?: string;
  benefit?: string;
  items: string;
  itemList: SeasonalItem[];
  task?: string;
  priority: string;
  candles: number;
  ascended: number;
  hearts: number;
  selected: boolean;
  isCustom?: boolean;
  status?: string;
  start?: string;
  end?: string;
  icon?: string;
  /** 先祖 → 买齐整棵树的季蜡 */
  treeCandles?: Record<string, number>;
}

export interface Jx3DailyCalendar {
  date: string;
  war: string;     // 今日大战
  mine: string;    // 今日矿车
  school: string;  // 门派事件
  battle?: string;
  rescue?: string;
  leader?: string[];
  /** 今日秘境日常 */
  card?: string[];
  /** 今日福缘宠物 */
  lucky?: string[];
  /** 外观抽取，只有部分日子有 */
  draw?: string[];
  /** 本周公共任务 */
  weekly_conn?: string[];
  /** 本周团队秘境 */
  weekly_raid?: string[];
}

export interface MingyiRoleSnapshot {
  uid: string;
  jx3_id: number | null;
  name: string;
  server: string;
  region: string;
  edition: string;
  time: number | null;
  time_str: string | null;
  version: string | null;
  acquired_count: number;
  acquired_ids: string[];
  shot_path: string;
  info_path: string;
  shot_error: string | null;
}

export interface MingyiScanResult {
  data_root: string;
  roles: MingyiRoleSnapshot[];
  warnings: string[];
}

// 剑网3 成就百科项
export interface Jx3Achievement {
  id: string;
  name: string;
  point: number;   // 资历点数 (如 30 / 50 / 100)
  sub_name: string;// 分类 (如 "任务·安史之乱", "秘境·英雄荻花")
  desc: string;    // 达成条件
  done: boolean;   // 是否已完成
  wiki_url?: string;
}

export type Jx3AchieveMatch = 'done' | 'missing' | 'unlinked';

export interface Jx3PlaceAchieveRow {
  key: string;
  id: string;
  name: string;
  point?: number;
  match: Jx3AchieveMatch;
  source: 'place' | 'adventure';
  adventureName?: string;
  guideNote?: string;
  req?: Jx3AdventureRequirement;
}

export interface Jx3PlacePaper {
  id: string;
  title: string;
  kind: 'dungeon' | 'map';
  reputation?: Jx3ReputationItem;
  smartAction?: 'farm' | 'map' | 'both' | 'item' | 'unset' | 'done';
  smartReason?: string;
  dungeonPendantsObtained?: boolean;
  dungeonAchievementMatch: Jx3AchieveMatch | 'empty';
  rows: Jx3PlaceAchieveRow[];
  missing: Jx3PlaceAchieveRow[];
  unlinked: Jx3PlaceAchieveRow[];
  doneCount: number;
  linkedCount: number;
}

// 单机 & Galgame 进度档案
export interface CoverPosition { x: number; y: number }

export interface GameInstallation {
  steam_id?: string;
  root: string;
  directory: string;
  executable: string;
  candidates: string[];
}
export interface GameStaff { name: string; role: string; source_id?: string }
export interface GameTaxonomy {
  tags: string[];
  developers: string[];
  publishers: string[];
  staff: GameStaff[];
}
/** 标签按增删保存；制作信息未设置时自动使用来源，空数组表示手动清空。 */
export interface GameCuration {
  added_tags?: string[];
  hidden_tags?: string[];
  developers?: string[] | null;
  publishers?: string[] | null;
  staff?: GameStaff[] | null;
}
export interface GameMetadata {
  source: 'steam' | 'vndb' | 'bangumi';
  source_id: string;
  title: string;
  original_title: string;
  description: string;
  cover: string;
  screenshots: string[];
  developer: string;
  developers?: string[];
  publishers?: string[];
  staff?: GameStaff[];
  release_date: string;
  score: number;
  url: string;
  aliases?: string[];
  tags?: string[];
  platforms?: string[];
  info?: { label: string; value: string }[];
  sources?: { source: GameMetadata['source']; source_id: string; url: string }[];
  videos?: { title: string; url: string; thumbnail: string; page_url: string }[];
  music?: { id: string; title: string; relation: string; cover: string; url: string }[];
  background?: string;
  warnings?: string[];
  enriched?: boolean;
  media_version?: number;
}
export interface GameExtraMedia { kind: 'video' | 'audio'; title: string; url: string }
export interface GameScalingProfile {
  directory: string;
  preset: 'fit' | 'anime' | 'sharp' | 'pixel';
  capture: 'auto' | 'wgc' | 'gdi';
  mode: 'fullscreen' | 'window';
  auto_start: boolean;
}
export interface GameAlbum { info: { label: string; value: string }[]; tracks: { title: string; duration: string; disc: number; number: number }[] }
export interface GameLibraryDetails {
  installations: GameInstallation[];
  /** 未设置时沿用攻略合集；空名称表示明确移出合集。 */
  collection?: { name: string; order?: number } | null;
  preferred_launch?: string | null;
  save_sources?: GameSaveSource[];
  metadata?: GameMetadata | null;
  curation?: GameCuration | null;
  scaling?: GameScalingProfile[];
  /** null/undefined: official background or cover; empty string: no image; URL/data: user choice. */
  background?: string | null;
  background_position?: CoverPosition | null;
  extra_media?: GameExtraMedia[];
  matching?: { query: string; error: string; candidates: GameMetadata[] } | null;
}
export interface GameSaveSource { slot: string; path: string; kind: 'directory' | 'nscripter' }
export interface GameSaveCandidate { source: GameSaveSource; reason: string; status: 'ready' | 'missing' | 'empty' | 'unavailable'; automatic: boolean }
export interface GameSaveDiscovery { candidates: GameSaveCandidate[]; warnings: string[] }
export interface GameRecord {
  id: string;
  title: string;
  category: 'galgame' | 'single' | 'steam';
  status: 'playing' | 'completed' | 'backlog' | 'dropped';
  progress: string;
  rating: number;
  save_path?: string;
  notes?: string;
  poster?: string;
  poster_position?: CoverPosition;
  /** undefined 自动匹配，空字符串保持独立，否则关联该攻略 ID。 */
  guide_id?: string | null;
  library?: GameLibraryDetails;
}

export interface VaultAsset {
  id: string;
  name: string;
  qty?: string;
  note?: string;
}

export interface VaultCharacter {
  id: string;
  name: string;
  sect?: string;
  server?: string;
  assets: VaultAsset[];
  /** 挂到日常 · 周常。关掉后号仍留在金库。 */
  track_daily: boolean;
  /** 光遇查询ID（设置 → 精灵 → 工具指南）。仅光遇角色使用。 */
  query_id?: string;
  /** 标星的常用号：金库里排前、高亮，侧栏也排前。 */
  starred?: boolean;
}

export interface VaultItem {
  id: string;
  platform: string;
  account_name: string;
  server?: string;
  notes?: string;
  /** Steam 登录号的 17 位 SteamID64。非 Steam 卡不使用。 */
  steamid64?: string;
  assets: VaultAsset[];
  characters: VaultCharacter[];
  /** 本机解不开这条记录的密文时的原因（换了电脑或系统账户）。有值时只读。 */
  locked?: string;
}

/** 金库 Steam 账号卡下的 GetOwnedGames 缓存，独立于单机档案。 */
export interface SteamLibraryGame {
  vault_item_id: string;
  appid: number;
  name: string;
  playtime_forever: number;
  playtime_2weeks?: number;
  rtime_last_played?: number;
  img_icon_url?: string;
}

/** 投影到 Obsidian `Waypoint/账号金库.md` 的金库行。 */
export interface WaypointVaultExportItem {
  id: string;
  platform: string;
  account_name: string;
  server?: string;
  notes?: string;
  steamid64?: string;
  assets: VaultAsset[];
  characters: VaultCharacter[];
}

export type WaypointProjectionScope =
  | 'vault'
  | 'dailies'
  | 'memos'
  | 'games'
  | 'jx3'
  | 'sky'
  | 'all';

export interface WaypointProjectionSnapshot {
  scopes: WaypointProjectionScope[];
  characters: Character[];
  tasks: TaskItem[];
  vault: WaypointVaultExportItem[];
  memos: CustomMemo[];
  gameRecords: Array<Pick<GameRecord, 'id' | 'title' | 'category' | 'status' | 'progress' | 'rating'>>;
  jx3: {
    pendants: Array<{ id: string; name: string; dungeon?: string; obtained: boolean; cdCleared: boolean }>;
    reputations: Array<{ id: string; name: string; currentLevel: string }>;
    tokens: Array<{ id: string; seriesName: string; pieces: Record<string, boolean> }>;
    achievements: string[];
  };
  sky: {
    spirits: Array<{ id: string; map: string; name: string; unlocked: boolean; wingUnlocked: boolean; itemObtained: boolean }>;
    seasons: Array<{
      id: string;
      season: string;
      selected: boolean;
      items: Array<{ id: string; name: string; obtained: boolean }>;
    }>;
  };
}

export interface WaypointProjectionResult {
  root: string;
  relative_root: string;
  files: string[];
  counts: {
    vault: number;
    tasks: number;
    memos: number;
    game_records: number;
    characters: number;
  };
}

export interface ObsidianFile {
  name: string;
  relative_path: string;
  full_path: string;
  size: number;
  modified: string;
}

export interface DbInfo {
  path: string;
  dir: string;
  size_bytes: number;
  exists: boolean;
  /** 上次导入/恢复前留下的回滚点时间；没有则为空 */
  undo_restore_at?: string | null;
}

export interface WebdavConfig {
  url: string;
  username: string;
  password?: string;
}

export interface WebdavStatus {
  configured: boolean;
  url: string;
  username: string;
  has_password: boolean;
  last_sync_time?: string;
  remote_exists: boolean;
  remote_size?: number;
  remote_modified?: string;
  remote_error?: string | null;
}

export interface WebdavSyncResult {
  success?: boolean;
  /** 云端在本机上次同步后被别的设备改过，这次没有上传 */
  conflict?: boolean;
  remote_modified?: string;
  last_sync_time?: string | null;
  raw_size?: number;
  compressed_size?: number;
  restored_size?: number;
  sync_time?: string;
}

export interface BackupProgress {
  phase: 'checking' | 'packing' | 'uploading' | 'verifying' | 'publishing';
  sent: number;
  total: number;
}

export interface RestoreProgress {
  phase: 'reading' | 'downloading' | 'decrypting' | 'validating' | 'saving_undo' | 'replacing' | 'reloading';
  received: number;
  /** 0 表示服务器没有提供文件大小。 */
  total: number;
}

export interface ObsidianGameGuide {
  /** 系列仅用于聚合展示，每部作品仍拥有独立攻略与档案。 */
  series?: string;
  series_order?: number;
  resource_url?: string;
  resource_game_id?: string;
  id: string;
  title: string;
  original_title?: string;
  aliases: string[];
  release_date?: string;
  year?: string;
  developer?: string | null;
  publisher?: string | null;
  taxonomy?: GameTaxonomy;
  category: 'galgame' | 'single' | 'mobile' | string;
  status: string;
  rating_vndb?: number;
  rating_personal?: number;
  rating_bangumi?: number;
  rating: number;
  routes_total?: number;
  routes_cleared?: number;
  progress_note?: string;
  tags: string[];
  genres: string[];
  file_path: string;
  relative_path: string;
  file_name: string;
  modified: string;
  poster?: string;
  summary?: string;
  record_id?: string;
  poster_position?: CoverPosition;
  personal_notes?: string;
  /** 笔记前言 links 直链。键是站点名，值是 https 地址。 */
  links?: Record<string, string>;
}

export interface GuideProgressBookmark {
  guideId: string;
  routeTitle: string;
  blockIdx?: number;
  lineIdx?: number;
  stepText?: string;
  updatedAt: string;
}

export interface CustomMemo {
  id: string;
  title: string;
  tag: string;
  cycle_type: 'once' | 'daily' | 'weekly' | 'interval' | 'deadline';
  cycle_val?: string;
  note?: string;
  has_progress: boolean;
  current_progress: number;
  target_progress: number;
  done: boolean;
  last_completed_at?: string;
  created_at: string;
  character_id?: string;
  pinned?: boolean;
}

// ==================== 剑网3 专属数据架构 ====================

export type Jx3ReputationLevel = '中立' | '友好' | '亲密' | '敬重' | '尊敬' | '钦佩';

export interface Jx3DungeonPendant {
  id: string;
  name: string;
  type: '背部挂件' | '腰部挂件' | '特殊坐骑' | '宠物/特效';
  dungeon: string;           // 掉落副本 (如 "英雄荻花圣殿")
  difficulty: '5人普通' | '5人英雄' | '10人普通' | '25人普通' | '25人英雄';
  boss: string;              // 掉落首领 (如 "卫栖梧 / 慕容追风 / 沙利亚")
  cdRule: '每日07:00' | '周一/五07:00' | '周一07:00' | '无CD';
  reputationId?: string;     // 关联声望势力 ID (如 "dashaman")
  relatedAchieveIds: string[]; // 关联的该副本成就 ID 列表 (如无伤/全通)
  obtained: boolean;         // 角色是否已拥有该挂件
  cdCleared: boolean;        // 本周期是否已刷打卡
  icon?: string;
  desc?: string;
}

export interface Jx3ReputationItem {
  id: string;
  name: string;              // 声望名称 (如 "大萨满 / 轩辕社 / 天策府")
  iconPath?: string;         // 游戏内势力徽记路径
  group: string;
  dungeon?: string;          // 关联掉落声望的副本 (如 "英雄荻花圣殿")
  currentLevel: Jx3ReputationLevel;
  levelSource?: 'achievement' | 'recorded' | 'unknown';
  preferredMethod?: 'auto' | 'content' | 'item';
  maxLevel: Jx3ReputationLevel;
  recommendedItem: string;   // 推荐喂养的声望道具 (如 "迟来的钦佩 / 遗失的尊敬 / 翰墨声望书")
  itemGain: number;          // 道具单次增加声望值 (如 1000 / 3000)
  rewardHighlight: string;   // 尊敬/钦佩核心奖励 (称号/知交/附魔/挂件)
  // 智能推导状态 (运行时动态计算)
  dungeonAchievementsDone?: boolean; // 副本关联成就全部完成（仅已对账的真 ID）
  dungeonAchievementMatch?: Jx3AchieveMatch | 'empty';
  dungeonPendantsObtained?: boolean;  // 副本关联挂件已全部拥有
  place?: string;
  gain?: string;
  lane?: 'dungeon' | 'map' | 'both' | 'item' | 'unset';
  laneOverride?: 'dungeon' | 'map' | 'both' | 'item' | 'unset';
  maxLevelOverride?: Jx3ReputationLevel;
  smartAction?: 'farm' | 'map' | 'both' | 'item' | 'unset' | 'done';
  smartReason?: string;      // 智能决策原因说明
}

export type Jx3AdventureReqType =
  | 'achievement'
  | 'reputation'
  | 'point'
  | 'item'
  | 'quest'
  | 'other';

export type Jx3AdventureSeriesMode = 'all' | 'any';

export interface Jx3AdventureRequirement {
  id: string;
  type: Jx3AdventureReqType;
  name: string;              // 游戏表 szFront 原句
  refId?: string;            // 真成就数字 ID 或声望 ID；假键不算对账
  placeId?: string;          // 挂到哪张地点纸
  minLevel?: Jx3ReputationLevel;
  targetVal?: number;
  currentVal?: number;
  seriesMode?: Jx3AdventureSeriesMode;
  seriesIds?: string[];      // 对决！/五十胜！这类前缀展开后的真 ID
  reputationIds?: string[];
  unresolvedAlternatives?: boolean;
  unresolvedRequired?: boolean;
  completed: boolean;        // 本地对撞是否已达成
  match?: Jx3AchieveMatch;
  guideNote?: string;
}

export interface Jx3AdventureItem {
  id: string;
  name: string;
  type: '绝世奇遇' | '百家奇遇' | '宠物奇遇' | '阵营奇遇';
  reward: string;
  triggerDaily: string;
  minPoint: number;
  completeAchieveId?: string; // 「完成奇遇《名字》」数字 ID
  coverUrl?: string; // 魔盒彩色奖励图
  requirements: Jx3AdventureRequirement[];
  isTriggered: boolean;
}

export interface Jx3SetTokenSeries {
  id: string;
  seriesName: string;        // 套装名称 (如 "儒风套 / 朔雪套 / 秦风套 / 燕云套")
  level: string;             // 对应年代 (如 "95年代 / 90年代 / 80年代")
  dungeon: string;           // 主要产出副本 (如 "永王行宫·花月仙踪 / 仙侣庭园")
  pinyin: string;            // 拼音简写方便打本秒搜 (如 "rf / rufeng")
  aliases?: string[];        // 常见易错字、玩家别名 (如 ["溯雪", "溯雪套"])
  pieces: {
    head: { name: string; boss: string; obtained: boolean };
    body: { name: string; boss: string; obtained: boolean };
    belt: { name: string; boss: string; obtained: boolean };
    wrist: { name: string; boss: string; obtained: boolean };
    shoes: { name: string; boss: string; obtained: boolean };
  };
}

export type Jx3TokenSlot = 'head' | 'body' | 'belt' | 'wrist' | 'shoes';
export type Jx3CollectKind = 'pendant' | 'special' | 'appearance' | 'token';
export type Jx3CollectSource = 'seed' | 'box' | 'custom';

/** 特殊收集心愿单项。进度是「这个本我勾了几件」，不是官方掉落表。 */
export interface Jx3CollectItem {
  id: string;
  dungeonKey: string;
  dungeonName: string;
  era?: string;
  kind: Jx3CollectKind;
  name: string;
  obtained: boolean;
  source: Jx3CollectSource;
  catalogId?: string;
  tokenSlot?: Jx3TokenSlot;
  tokenSeriesId?: string;
  boxItemId?: string;
  iconId?: number;
  note?: string;
  boss?: string;
  difficulty?: string;
  type?: string;
  cdRule?: string;
  cdCleared?: boolean;
  /** 本周期已打的切点。剑三 07:00，5 人每日，10/25 人周一。过点即视为未打。 */
  cdClearedAt?: string;
  reputationId?: string;
  relatedAchieveIds?: string[];
}

export interface Jx3CollectGroup {
  dungeonKey: string;
  dungeonName: string;
  era?: string;
  eraLevel?: string;
  icon?: string;
  bosses: string[];
  items: Jx3CollectItem[];
  obtainedCount: number;
  totalCount: number;
}

export interface Jx3BoxDungeonMap {
  mapId: string;
  mode: string;
}

export interface Jx3BoxDungeon {
  key: string;
  name: string;
  era: string;
  eraLevel: string;
  icon?: string;
  maps: Jx3BoxDungeonMap[];
  bosses: string[];
}

export interface Jx3BoxItemHit {
  id: string;
  name: string;
  iconId?: number;
  typeLabel?: string;
  getType?: string;
  getSource?: string;
  quality?: number;
  source?: string;
  desc?: string;
  belongSchool?: string;
  requireLevel?: number;
  inferredDungeon?: {
    dungeonKey: string;
    dungeonName: string;
    era?: string;
    boss?: string;
  };
}

export interface Jx3DynamicSetPiece {
  slot: Jx3TokenSlot;
  slotLabel: string;
  name: string;
  hit?: Jx3BoxItemHit;
  dungeonKey?: string;
  dungeonName?: string;
  era?: string;
  boss?: string;
  obtained?: boolean;
  inCollect?: boolean;
}

export interface Jx3DynamicSet {
  seriesName: string;
  level?: string;
  pieces: Record<Jx3TokenSlot, Jx3DynamicSetPiece>;
}

export interface Jx3CollectPinnedDungeon {
  dungeonKey: string;
  dungeonName: string;
  era?: string;
  eraLevel?: string;
}

export interface Jx3CollectStore {
  items: Jx3CollectItem[];
  pinnedDungeons: Jx3CollectPinnedDungeon[];
  collapsed: Record<string, boolean>;
}

export interface Jx3WorldBossSlot {
  weekday: number;           // 0-6，与 Date.getDay() 一致
  weekdayLabel: string;
  startHour: number;
  endHour: number;
}

export interface Jx3WorldBossSchedule {
  idleName: string;
  idleMap: string;
  drops: string[];
  slots: Jx3WorldBossSlot[];
}

export interface Jx3WorldBossStoredDay {
  weekday: number;
  dateStr: string;
  names: string[];
}

export interface Jx3WorldBossStoredWeek {
  weekStart: string;
  days: Jx3WorldBossStoredDay[];
}

export interface Jx3WorldBossDay {
  weekday: number;
  weekdayLabel: string;
  dateStr: string;
  startHour: number;
  endHour: number;
  timeDesc: string;
  names: string[];
  namesLine: string;
  statusText: string;
  isToday: boolean;
  inWindow: boolean;
  ended: boolean;
  isPast: boolean;
}

export interface Jx3WorldBossWeek {
  weekStart: string;
  cadence: string;
  headerHint: string;
  isBossDay: boolean;
  inWindow: boolean;
  days: Jx3WorldBossDay[];
}

export interface Jx3MapEvent {
  map: string;
  stage: string;
  site: string;
  desc: string;
  time: string;
}

export interface Jx3MapFeed {
  name: string;
  map: string;
  events: Jx3MapEvent[];
}

export interface Jx3MapRefreshInfo {
  map: string;
  nextText: string;
  dueSoon: boolean;
  approaching: boolean;
  inWindow: boolean;
}




