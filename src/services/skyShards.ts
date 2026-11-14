/**
 * 光遇落石：
 * 国际服 = PlutoyDev/sky-shards（太平洋时区，含周日 DST）。
 * 国服 = 社区日历同一套半月+星期规则（北京时间）：ichozero/skyshard_calendar 的地图表与时间窗，
 * 与 wiki NetEase/Shard Eruptions、QQ 机器人今日落石一致。机器人能秒出图，是因为地图是 (日+2)%5 查表。
 */

export type ServerMode = 'global' | 'netease';

export interface ShardOccurrence {
  index: number;              // 第 1 / 2 / 3 次爆发
  start: Date;                // 天空开始异动变色
  land: Date;                 // 落石正式砸落着陆 (landOffset: +8m40s)
  end: Date;                  // 落石消散结束 (endOffset: +4h)
  timeRangeStr: string;       // 本地时间格式 "16:48 ~ 20:40"
}

export interface ShardInfo {
  serverMode: ServerMode;
  hasShard: boolean;
  dateStr: string;            // 本地日期 YYYY-MM-DD
  laDateStr: string;          // 太平洋基准日期 YYYY-MM-DD (仅国际服)
  isRed: boolean;             // true: 强落石(红石, 产升华烛+远古回忆光翼); false: 常规落石(黑石, 产大烛火)
  realm: string;              // 如 "雨林"
  mapName: string;            // 如 "神庙终点花园"
  locationDetail: string;     // 如 "雨林神庙出门左侧水池草地"
  rewardDescription: string;  // 如 "升华红蜡烛 ×2.5 (可进远古回忆拿光翼)"
  rewardCandles: number;      // 升华烛数量 (黑石为 0)
  /** 红石可进破碎季远古回忆；具体哪一场由游戏随机，这里不编。 */
  ancientMemory?: string;
  occurrences: ShardOccurrence[];
  /** 当前服是否有离线周期模型。国际服与国服均为 true。 */
  predictable: boolean;
  
  // 实时状态
  status: 'active' | 'falling' | 'upcoming' | 'all_ended' | 'no_shard';
  statusText: string;         // 状态文案
  currentOrNextOccurrence?: ShardOccurrence;
  countdownText: string;      // 动态倒计时
  
  // 下一次降临预报 (若今日无石或今日已结束)
  nextShardForecast?: {
    dateStr: string;
    dayDesc: string;          // 如 "明天 (周二)" 或 "9月9日"
    realm: string;
    mapName: string;
    isRed: boolean;
    rewardDescription: string;
    landTimeStr: string;      // 本地着陆时间
    countdownText: string;    // "距下次降临还剩 18小时25分"
  };
}

// 国际服排程参数 (PlutoyDev 核心排程组)
const SHARDS_INFO_GLOBAL = [
  {
    type: 'black',
    noShardWkDay: [6, 7], // 周六、周日无石
    intervalMinutes: 8 * 60,
    offsetMinutes: 1 * 60 + 50,
    maps: ['prairie.butterfly', 'forest.brook', 'valley.rink', 'wasteland.temple', 'vault.starlight'],
  },
  {
    type: 'black',
    noShardWkDay: [7, 1], // 周日、周一无石
    intervalMinutes: 8 * 60,
    offsetMinutes: 2 * 60 + 10,
    maps: ['prairie.village', 'forest.boneyard', 'valley.rink', 'wasteland.battlefield', 'vault.starlight'],
  },
  {
    type: 'red',
    noShardWkDay: [1, 2], // 周一、周二无石
    intervalMinutes: 6 * 60,
    offsetMinutes: 7 * 60 + 40,
    maps: ['prairie.cave', 'forest.end', 'valley.dreams', 'wasteland.graveyard', 'vault.jelly'],
    defRewardAC: 2.0,
  },
  {
    type: 'red',
    noShardWkDay: [2, 3], // 周二、周三无石
    intervalMinutes: 6 * 60,
    offsetMinutes: 2 * 60 + 20,
    maps: ['prairie.bird', 'forest.tree', 'valley.dreams', 'wasteland.crab', 'vault.jelly'],
    defRewardAC: 2.5,
  },
  {
    type: 'red',
    noShardWkDay: [3, 4], // 周三、周四无石
    intervalMinutes: 6 * 60,
    offsetMinutes: 3 * 60 + 30,
    maps: ['prairie.island', 'forest.sunny', 'valley.hermit', 'wasteland.ark', 'vault.jelly'],
    defRewardAC: 3.5,
  },
];

const OVERRIDE_REWARD_AC: Record<string, number> = {
  'forest.end': 2.5,
  'valley.dreams': 2.5,
  'forest.tree': 3.5,
  'vault.jelly': 3.5,
};

const MAP_METAS: Record<string, { realm: string; name: string; desc: string }> = {
  'prairie.butterfly': { realm: '云野', name: '蝴蝶平原', desc: '云野一图中央大圆顶与左侧蝴蝶丘陵' },
  'prairie.village': { realm: '云野', name: '幽光山洞 / 仙乡', desc: '云野三塔村入口或左侧幽光山洞水域' },
  'prairie.cave': { realm: '云野', name: '幽光山洞', desc: '云野一图左侧隐藏隐藏洞穴水潭周边' },
  'prairie.bird': { realm: '云野', name: '鸟巢', desc: '云野一图右侧隐藏浮岛鸟巢主浮岛' },
  'prairie.island': { realm: '云野', name: '圣岛', desc: '圣岛季海岛主岛巨鳐雕像或水母洞旁' },
  'forest.brook': { realm: '雨林', name: '静谧庭院 / 幽光森林', desc: '雨林二图潺潺溪水两侧树根下' },
  'forest.boneyard': { realm: '雨林', name: '密林遗迹 (骨场)', desc: '雨林水母图前巨大古代兽骨与水坑' },
  'forest.end': { realm: '雨林', name: '神庙终点花园', desc: '雨林终点神庙出门左侧开阔草坪与小池' },
  'forest.tree': { realm: '雨林', name: '大树屋 / 荧光森林', desc: '集结季大树屋入口周边草地与树桩' },
  'forest.sunny': { realm: '雨林', name: '秘密花园 (晴空林)', desc: '雨林二图右侧隐藏隐藏树洞水潭' },
  'valley.rink': { realm: '霞谷', name: '溜冰场', desc: '霞谷滑冰场冰面或下层水下湖底' },
  'valley.dreams': { realm: '霞谷', name: '圆梦村', desc: '圆梦村滑冰少女向导雕像与缆车站旁' },
  'valley.hermit': { realm: '霞谷', name: '雪隐峰', desc: '圆梦村乘第二段高空缆车到达的雪人山峰' },
  'wasteland.temple': { realm: '暮土', name: '边陲之地', desc: '暮土一图倾颓神庙大殿石阶周边' },
  'wasteland.battlefield': { realm: '暮土', name: '远古战场', desc: '暮土终点大门前巨大战场遗迹残垣' },
  'wasteland.graveyard': { realm: '暮土', name: '巨兽荒原 (四龙图)', desc: '暮土四龙巡逻区域巨大龙骨凹槽' },
  'wasteland.crab': { realm: '暮土', name: '黑水港湾 (沉船图)', desc: '暮土沉船右侧黑水蟹池管道旁' },
  'wasteland.ark': { realm: '暮土', name: '失落方舟', desc: '暮土魔法季方舟农田与断船周边' },
  'vault.starlight': { realm: '禁阁', name: '星光沙漠', desc: '小王子季星光沙漠玫瑰花园周边' },
  'vault.jelly': { realm: '禁阁', name: '水母湾 (淮安湾)', desc: '禁阁星光沙漠右侧水母港湾碎石滩' },
};

/** 国服：星期 → 五图轮换（云野→雨林→霞谷→暮土→禁阁），下标 (日+2)%5。周日=0。 */
const NETEASE_MAPS: Record<number, string[]> = {
  0: ['prairie.island', 'forest.sunny', 'valley.hermit', 'wasteland.ark', 'vault.jelly'],
  2: ['prairie.butterfly', 'forest.tree', 'valley.rink', 'wasteland.temple', 'vault.starlight'],
  3: ['prairie.village', 'forest.boneyard', 'valley.rink', 'wasteland.battlefield', 'vault.starlight'],
  5: ['prairie.bird', 'forest.tree', 'valley.dreams', 'wasteland.crab', 'vault.jelly'],
  6: ['prairie.cave', 'forest.end', 'valley.dreams', 'wasteland.graveyard', 'vault.jelly']
};

/** 国服落地时刻（北京时间整点过 8 分）；持续到下一整点。 */
const NETEASE_HOURS: Record<number, number[]> = {
  0: [7, 13, 19],
  2: [9, 14, 19],
  3: [9, 15, 21],
  5: [11, 17, 23],
  6: [10, 14, 22]
};

/**
 * 格式化时间段
 */
function formatTimeRange(land: Date, end: Date): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  const lH = pad(land.getHours());
  const lM = pad(land.getMinutes());
  const eH = pad(end.getHours());
  const eM = pad(end.getMinutes());
  return `${lH}:${lM} ~ ${eH}:${eM}`;
}

/**
 * 格式化剩余时间
 */
function formatCountdown(ms: number): string {
  if (ms <= 0) return '即将结束';
  if (ms < 60_000) return '不到1分钟';
  const totalMins = Math.floor(ms / (1000 * 60));
  const hours = Math.floor(totalMins / 60);
  const mins = totalMins % 60;
  if (hours > 0) {
    return `${hours}小时${mins}分`;
  }
  return `${mins}分钟`;
}

/**
 * 国际服单日排程计算 (PlutoyDev 核心数学模型)
 */
function calcGlobalDayShard(date: Date) {
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone: 'America/Los_Angeles',
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    weekday: 'short',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
    hour12: false
  });
  
  const parts = formatter.formatToParts(date);
  const partMap: Record<string, string> = {};
  for (const p of parts) partMap[p.type] = p.value;
  
  const year = parseInt(partMap.year, 10);
  const month = parseInt(partMap.month, 10);
  const day = parseInt(partMap.day, 10);
  
  // weekday: Mon=1, Tue=2, Wed=3, Thu=4, Fri=5, Sat=6, Sun=7
  const weekdayMap: Record<string, number> = { 'Mon': 1, 'Tue': 2, 'Wed': 3, 'Thu': 4, 'Fri': 5, 'Sat': 6, 'Sun': 7 };
  const dayOfWeek = weekdayMap[partMap.weekday] || 1;

  const isRed = day % 2 === 1;
  const realmIdx = (day - 1) % 5;
  const infoIndex = isRed ? (((day - 1) / 2) % 3) + 2 : (day / 2) % 2;

  const config = SHARDS_INFO_GLOBAL[infoIndex];
  const hasShard = !config.noShardWkDay.includes(dayOfWeek);

  if (!hasShard) {
    return {
      laDateStr: `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`,
      hasShard: false,
      isRed: false,
      realm: '遇境',
      mapName: '天空澄净 · 无落石',
      locationDetail: '今日全图无黑暗降临，伊甸风暴平静',
      rewardDescription: '今日全图无落石，无需跑图清理',
      rewardCandles: 0,
      ancientMemory: undefined,
      occurrences: []
    };
  }

  const mapKey = config.maps[realmIdx];
  const rewardAC = isRed ? (OVERRIDE_REWARD_AC[mapKey] || config.defRewardAC || 2.0) : 0;
  const meta = MAP_METAS[mapKey] || { realm: '云野', name: mapKey, desc: '' };

  const hour = parseInt(partMap.hour, 10) % 24;
  const laCurrentMinutes = hour * 60 + parseInt(partMap.minute, 10);
  const laCurrentSeconds = parseInt(partMap.second, 10);
  const msFromMidnight = (laCurrentMinutes * 60 + laCurrentSeconds) * 1000 + (date.getTime() % 1000);
  const startOfDayMs = date.getTime() - msFromMidnight;

  let firstStartMs = startOfDayMs + config.offsetMinutes * 60 * 1000;
  if (dayOfWeek === 7 && config.type === 'red') {
    const noon = new Date(startOfDayMs + 12 * 3600 * 1000);
    const first = new Date(firstStartMs);
    if (isLosAngelesDST(noon) !== isLosAngelesDST(first)) {
      firstStartMs += (isLosAngelesDST(first) ? -1 : 1) * 3600 * 1000;
    }
  }

  const occurrences: ShardOccurrence[] = [0, 1, 2].map(i => {
    const startMs = firstStartMs + i * config.intervalMinutes * 60 * 1000;
    const landMs = startMs + (8 * 60 + 40) * 1000;
    const endMs = startMs + 4 * 60 * 60 * 1000;
    const landDate = new Date(landMs);
    const endDate = new Date(endMs);
    return {
      index: i + 1,
      start: new Date(startMs),
      land: landDate,
      end: endDate,
      timeRangeStr: formatTimeRange(landDate, endDate)
    };
  });

  return {
    laDateStr: `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`,
    hasShard,
    isRed,
    realm: meta.realm,
    mapName: meta.name,
    locationDetail: meta.desc,
    rewardDescription: isRed ? `升华红蜡烛 ×${rewardAC}（红石可进破碎季远古回忆拿翼，具体哪一场由游戏随机）` : '黑石：常规大烛火',
    rewardCandles: rewardAC,
    ancientMemory: isRed ? '破碎季远古回忆（随机一场，不在此预报）' : undefined,
    occurrences
  };
}

function isLosAngelesDST(date: Date): boolean {
  const name = new Intl.DateTimeFormat('en-US', {
    timeZone: 'America/Los_Angeles',
    timeZoneName: 'short'
  })
    .formatToParts(date)
    .find(p => p.type === 'timeZoneName')?.value || '';
  return name.includes('DT');
}

type DayShardRaw = ReturnType<typeof calcGlobalDayShard>;

function beijingClock(date: Date) {
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone: 'Asia/Shanghai',
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    weekday: 'short',
    hour: 'numeric',
    minute: 'numeric',
    second: 'numeric',
    hour12: false
  });
  const parts = formatter.formatToParts(date);
  const partMap: Record<string, string> = {};
  for (const p of parts) partMap[p.type] = p.value;
  const weekdayMap: Record<string, number> = {
    Sun: 0, Mon: 1, Tue: 2, Wed: 3, Thu: 4, Fri: 5, Sat: 6
  };
  const hour = parseInt(partMap.hour, 10) % 24;
  const minute = parseInt(partMap.minute, 10);
  const second = parseInt(partMap.second, 10);
  const msFromMidnight =
    (hour * 60 + minute) * 60 * 1000 + second * 1000 + (date.getTime() % 1000);
  return {
    year: parseInt(partMap.year, 10),
    month: parseInt(partMap.month, 10),
    day: parseInt(partMap.day, 10),
    dayOfWeek: weekdayMap[partMap.weekday] ?? 0,
    startOfDayMs: date.getTime() - msFromMidnight
  };
}

function neteaseHasShard(day: number, dayOfWeek: number): boolean {
  if (dayOfWeek === 1 || dayOfWeek === 4) return false;
  if (day <= 15) return dayOfWeek !== 3 && dayOfWeek !== 5;
  return dayOfWeek !== 2 && dayOfWeek !== 6;
}

/**
 * 国服单日排程：半月+星期决定红/黑/无石，地图 (日+2)%5，北京时间 :08 落地、下一整点结束。
 */
function calcNeteaseDayShard(date: Date): DayShardRaw {
  const clock = beijingClock(date);
  const { year, month, day, dayOfWeek, startOfDayMs } = clock;
  const dateStr = `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
  const empty = (detail: string): DayShardRaw => ({
    laDateStr: dateStr,
    hasShard: false,
    isRed: false,
    realm: '遇境',
    mapName: '天空澄净 · 无落石',
    locationDetail: detail,
    rewardDescription: '今日全图无落石，无需跑图清理',
    rewardCandles: 0,
    ancientMemory: undefined,
    occurrences: []
  });

  if (!neteaseHasShard(day, dayOfWeek)) {
    return empty('国服今日无落石（半月+星期规则）');
  }

  const maps = NETEASE_MAPS[dayOfWeek];
  const hours = NETEASE_HOURS[dayOfWeek];
  if (!maps || !hours) return empty('国服今日无落石');

  const isRed = dayOfWeek === 0 || dayOfWeek === 5 || dayOfWeek === 6;
  const mapKey = maps[(day + 2) % 5];
  const meta = MAP_METAS[mapKey] || { realm: '云野', name: mapKey, desc: '' };
  const rewardAC = isRed ? (OVERRIDE_REWARD_AC[mapKey] || (dayOfWeek === 0 ? 3.5 : 2.5)) : 0;

  const occurrences: ShardOccurrence[] = hours.map((h, i) => {
    const landMs = startOfDayMs + (h * 60 + 8) * 60 * 1000;
    const endMs = startOfDayMs + (h + 1) * 60 * 60 * 1000;
    const startMs = landMs;
    const landDate = new Date(landMs);
    const endDate = new Date(endMs);
    return {
      index: i + 1,
      start: new Date(startMs),
      land: landDate,
      end: endDate,
      timeRangeStr: formatTimeRange(landDate, endDate)
    };
  });

  return {
    laDateStr: dateStr,
    hasShard: true,
    isRed,
    realm: meta.realm,
    mapName: meta.name,
    locationDetail: `${meta.desc}（北京时间）`,
    rewardDescription: isRed
      ? `升华红蜡烛 ×${rewardAC}（红石可进破碎季远古回忆拿翼，具体哪一场由游戏随机）`
      : '黑石：常规大烛火',
    rewardCandles: rewardAC,
    ancientMemory: isRed ? '破碎季远古回忆（随机一场，不在此预报）' : undefined,
    occurrences
  };
}

function calcDayShard(date: Date, mode: ServerMode): DayShardRaw {
  return mode === 'netease' ? calcNeteaseDayShard(date) : calcGlobalDayShard(date);
}

/**
 * 寻找下一次即将降落的落石 (跨天推算，最多向前巡查 14 天)
 */
export function findNextUpcomingShard(from: Date = new Date(), mode: ServerMode = 'global') {
  for (let d = 0; d < 14; d++) {
    const cur = new Date(from.getTime() + d * 24 * 3600 * 1000);
    const raw = calcDayShard(cur, mode);
    if (raw.hasShard && raw.occurrences.length > 0) {
      for (const occ of raw.occurrences) {
        if (occ.end.getTime() > from.getTime()) {
          const diffMs = occ.land.getTime() - from.getTime();
          
          const landDate = occ.land;
          const isSameDay = landDate.getFullYear() === from.getFullYear() &&
            landDate.getMonth() === from.getMonth() &&
            landDate.getDate() === from.getDate();

          const tomorrow = new Date(from.getFullYear(), from.getMonth(), from.getDate() + 1);
          const isTomorrow = landDate.getFullYear() === tomorrow.getFullYear() &&
            landDate.getMonth() === tomorrow.getMonth() &&
            landDate.getDate() === tomorrow.getDate();

          let dayDesc = '今日';
          if (isTomorrow) dayDesc = '明天';
          else if (!isSameDay) dayDesc = `${landDate.getMonth() + 1}月${landDate.getDate()}日`;

          return {
            raw,
            occurrence: occ,
            dayDesc,
            countdownText: diffMs > 0 ? `距下次降临还有 ${formatCountdown(diffMs)}` : '爆发进行中'
          };
        }
      }
    }
  }
  return null;
}

/**
 * 计算完整的光遇落石状态 (当天 + 状态机 + 下次预报)
 */
export function calculateSkyShard(date: Date = new Date(), mode: ServerMode = 'global'): ShardInfo {
  const dateStr = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  const raw = calcDayShard(date, mode);

  const nowMs = date.getTime();
  let status: ShardInfo['status'] = 'upcoming';
  let statusText = '等待降临';
  let countdownText = '';
  let activeOccurrence: ShardOccurrence | undefined;

  if (!raw.hasShard) {
    status = 'no_shard';
    statusText = '今日天空澄净 · 无落石';
    countdownText = '今日无落石爆发';
  } else {
    // 检查当日 occurrences
    let nextStartOcc: ShardOccurrence | null = null;
    let foundActive = false;

    for (const occ of raw.occurrences) {
      if (nowMs >= occ.land.getTime() && nowMs < occ.end.getTime()) {
        foundActive = true;
        activeOccurrence = occ;
        status = 'active';
        const remainMs = occ.end.getTime() - nowMs;
        statusText = `第 ${occ.index} 次爆发进行中`;
        countdownText = `距结束 ${formatCountdown(remainMs)}`;
        break;
      } else if (nowMs >= occ.start.getTime() && nowMs < occ.land.getTime()) {
        foundActive = true;
        activeOccurrence = occ;
        status = 'falling';
        const remainMs = occ.land.getTime() - nowMs;
        statusText = `天空变红 · 落石坠落中`;
        countdownText = `即将砸落地面 (预计 ${formatCountdown(remainMs)} 后着陆)`;
        break;
      } else if (nowMs < occ.start.getTime() && !nextStartOcc) {
        nextStartOcc = occ;
      }
    }

    if (!foundActive) {
      if (nextStartOcc) {
        status = 'upcoming';
        activeOccurrence = nextStartOcc;
        const diffMs = nextStartOcc.land.getTime() - nowMs;
        statusText = `距第 ${nextStartOcc.index} 次降临`;
        countdownText = `还有 ${formatCountdown(diffMs)}`;
      } else {
        status = 'all_ended';
        statusText = '今日降落已全部结束';
        countdownText = '今日落石已全部消散';
      }
    }
  }

  // 计算下一次降临预报
  const nextUpcoming = findNextUpcomingShard(date, mode);
  let nextShardForecast: ShardInfo['nextShardForecast'] = undefined;
  if (nextUpcoming) {
    nextShardForecast = {
      dateStr: nextUpcoming.occurrence.land.toLocaleDateString(),
      dayDesc: nextUpcoming.dayDesc,
      realm: nextUpcoming.raw.realm,
      mapName: nextUpcoming.raw.mapName,
      isRed: nextUpcoming.raw.isRed,
      rewardDescription: nextUpcoming.raw.rewardDescription,
      landTimeStr: nextUpcoming.occurrence.timeRangeStr,
      countdownText: nextUpcoming.countdownText
    };
  }

  return {
    serverMode: mode,
    hasShard: raw.hasShard,
    predictable: true,
    dateStr,
    laDateStr: raw.laDateStr,
    isRed: raw.isRed,
    realm: raw.realm,
    mapName: raw.mapName,
    locationDetail: raw.locationDetail,
    rewardDescription: raw.rewardDescription,
    rewardCandles: raw.rewardCandles,
    ancientMemory: raw.ancientMemory,
    occurrences: raw.occurrences,
    status,
    statusText,
    currentOrNextOccurrence: activeOccurrence,
    countdownText,
    nextShardForecast
  };
}

/** 时间段的显示状态来自同一次排程判定，不在组件里另取时钟。 */
export function shardOccurrenceState(shard: ShardInfo, index: number): 'active' | 'falling' | 'next' | 'upcoming' | 'ended' {
  const current = shard.currentOrNextOccurrence?.index;
  if (current === undefined || index < current) return 'ended';
  if (index > current) return 'upcoming';
  if (shard.status === 'active' || shard.status === 'falling') return shard.status;
  return 'next';
}
