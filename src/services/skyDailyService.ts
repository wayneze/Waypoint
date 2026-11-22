/**
 * 光遇今日任务：条目走社区日历 JSON。
 * 季蜡地图走国服五图日轮（云野→雨林→霞谷→暮土→禁阁），配图走分类后的季蜡长图。
 * 微博正文通常只有「图1 - 季蜡」，没有地图名。
 */

import { fetchSkyDailyFromBackend, fetchSkyQuestCalendar, type SkyQuestCalendar } from './skyApi';
import { cnSeasonalCandleRealm, parseSkyDailyPost, questFromCalendarText, type ParsedEventDaily } from './skyDailyLexicon';
import { cacheFromThisBuild, cacheGet, cacheSet, reportSource, sourceStatus } from './fetchCache';
import { beijingDay } from './cycleReset';
import { dailyChecked, setCharacterDailyChecks } from './progress';

export interface SkyDailyQuestItem {
  id: string;
  title: string;
  category: 'social' | 'spirit' | 'meditation' | 'collect' | 'creature';
  categoryLabel: string;
  realm: string;
  detailSpot: string;
  tip: string;
  imageUrl: string;
  completed: boolean;
}

export interface SkyCandleSpot {
  index: number;
  label: string;
  desc: string;
  imageUrl: string;
  completed?: boolean;
}

export interface SkyDailyCandleGroup {
  realm: string;
  groupName: string;
  headline: string;
  spots: SkyCandleSpot[];
}

export interface SkyEventDaily {
  id?: string;
  guidePending?: boolean;
  name: string;
  window?: string;
  how?: string;
  imageUrl: string;
  completed: boolean;
}

export interface SkyDailyGuidePayload {
  dateStr: string;
  dayOfWeekStr: string;
  quests: SkyDailyQuestItem[];
  seasonalCandles: SkyDailyCandleGroup;
  treasureCandles: SkyDailyCandleGroup[];
  eventDaily?: SkyEventDaily;
  /** 保留博文明确标注的图片用途，由活动身份匹配；兑换树不作为每日代币位置图。 */
  activityImages?: Array<{ caption: string; url: string }>;
  overviewImages: {
    questsImg: string;
    seasonCandleImg: string;
    treasureCandleImg: string;
    shardImg?: string;
    tokenImg?: string;
    tokenOnceImg?: string;
    calendarImg?: string;
  };
  sourcePostText?: string;
  fetchedAt: number;
  isFallback: boolean;
  parseNote?: string;
}

export function getTodayDayOfWeek(): string {
  return new Intl.DateTimeFormat('zh-CN', { timeZone: 'Asia/Shanghai', weekday: 'long' }).format(new Date());
}

// 今日任务图文按天缓存；角色打卡复用 progress_items，旧 daily_checks 只供归属合并。
const CACHE_KEY_PREFIX = 'sky.daily.';
const QUEST_SCOPE = 'sky.quest';
const CANDLE_SCOPE = 'sky.season_candle';
const EVENT_SCOPE = 'sky.event_daily';

function emptyGroup(realm: string, kind: '季蜡' | '大蜡烛'): SkyDailyCandleGroup {
  return {
    realm,
    groupName: '未解析',
    headline: `${kind} · 未能从博文解析地图`,
    spots: []
  };
}

function emptyPayload(todayStr: string, note: string): SkyDailyGuidePayload {
  return {
    dateStr: todayStr,
    dayOfWeekStr: getTodayDayOfWeek(),
    fetchedAt: Date.now(),
    isFallback: true,
    parseNote: note,
    sourcePostText: '',
    overviewImages: {
      questsImg: '',
      seasonCandleImg: '',
      treasureCandleImg: ''
    },
    quests: [],
    seasonalCandles: emptyGroup('未标明', '季蜡'),
    treasureCandles: []
  };
}

export async function loadTodaySkyDailyGuide(force = false): Promise<SkyDailyGuidePayload> {
  const todayStr = beijingDay();
  const cacheKey = `${CACHE_KEY_PREFIX}${todayStr}`;
  const entry = cacheFromThisBuild(cacheKey) ? cacheGet<SkyDailyGuidePayload>(cacheKey) : null;
  const cached = entry?.data.dateStr === todayStr ? entry.data : undefined;
  if (!force && cached && Date.now() - cached.fetchedAt < (cached.parseNote ? 5 : 15) * 60_000) {
    if (!sourceStatus['sky.daily']) {
      reportSource('sky.daily', '光遇今日任务', cached.parseNote ? 'partial' : 'ok', cached.parseNote, cached.fetchedAt);
    }
    return cached;
  }

  const [post, questCalendar] = await Promise.all([fetchSkyDailyFromBackend(), fetchSkyQuestCalendar()]);
  const weibo = post?.success && post.date_str === todayStr ? post : null;
  const calendar = questCalendar?.items.every(item => item.date === todayStr) ? questCalendar : null;
  if (weibo || calendar?.items.length) {
    const payload = buildPayloadFromBackend(
      weibo || { post_text: '', all_images: [] }, todayStr, calendar
    );
    const missing = [!weibo && '微博当天图文未发布或暂不可用', payload.parseNote].filter(Boolean).join('；');
    payload.parseNote = missing || undefined;
    if (!weibo && cached?.sourcePostText) {
      reportSource('sky.daily', '光遇今日任务', 'stale', '微博暂不可用，已保留今天的图文', cached.fetchedAt);
      return cached;
    }
    cacheSet(cacheKey, payload);
    reportSource('sky.daily', '光遇今日任务', missing ? 'partial' : 'ok', missing, payload.fetchedAt);
    return payload;
  }

  if (cached) {
    reportSource('sky.daily', '光遇今日任务', 'stale', '今日任务日历和微博暂不可用', cached.fetchedAt);
    return cached;
  }
  reportSource('sky.daily', '光遇今日任务', 'error', '今日任务日历和微博都没拉到');

  return emptyPayload(todayStr, '未能拉到今日任务日历，微博也没有博文。任务与地图不会沿用旧日文案。');
}

function pickImage(images: string[], hint: number | undefined): string {
  return hint == null ? '' : images[hint] || '';
}

function buildPayloadFromBackend(
  raw: {
    post_text: string;
    all_images: string[];
  },
  todayStr: string,
  calendar: SkyQuestCalendar | null
): SkyDailyGuidePayload {
  const parsed = parseSkyDailyPost(raw.post_text || '');
  const images = raw.all_images || [];
  const seasonImg = calendar?.seasonImage || pickImage(images, parsed.imageHints.seasonCandle);
  const weiboQuestsImg = pickImage(images, parsed.imageHints.quests);
  const shardImg = calendar?.shardImage || pickImage(images, parsed.imageHints.shard);
  // 活动代币图只认正文明说"代币"的那张。兑换树、复刻先祖动作展示不是代币位置，没有就不显示
  const tokenImg = calendar?.eventImage
    || (parsed.imageHints.token != null ? images[parsed.imageHints.token] || '' : '');
  const questsImg = calendar?.image || weiboQuestsImg;

  const calendarItems = calendar && calendar.items.length ? calendar.items : null;
  const quests: SkyDailyQuestItem[] = calendarItems
    ? calendarItems.map((item, idx) => ({
        ...questFromCalendarText(item.quest_id, item.quest_cn, idx),
        imageUrl: questsImg,
        completed: false
      }))
    : parsed.quests.map(q => ({
        ...q,
        imageUrl: questsImg,
        completed: false
      }));

  const seasonRealm = cnSeasonalCandleRealm(todayStr);
  const parsedSeasonal = parsed.seasonal;
  const seasonalSpots = (parsedSeasonal?.spots?.length ? parsedSeasonal.spots : [1, 2, 3, 4].map(index => ({
    index,
    label: `第 ${index} 堆`,
    desc: `${seasonRealm} · 第 ${index} 堆`
  })));
  const seasonal: SkyDailyCandleGroup = {
    realm: seasonRealm,
    groupName: '轮换',
    headline: `${seasonRealm} · 季蜡`,
    spots: seasonalSpots.slice(0, 4).map((s, idx) => ({
      index: idx + 1,
      label: /^第\s*\d+\s*堆$/.test(s.label) || s.label.includes('点位见配图') ? `第 ${idx + 1} 堆` : s.label,
      desc: s.desc.includes(seasonRealm) ? s.desc : `${seasonRealm} · ${s.desc}`,
      imageUrl: seasonImg,
      completed: false
    }))
  };

  const eventParsed: ParsedEventDaily | undefined = parsed.eventDaily;
  const eventDaily: SkyEventDaily | undefined =
    (eventParsed?.how || tokenImg)
      ? {
          name: eventParsed?.name || '活动日常',
          window: eventParsed?.window,
          how: eventParsed?.how,
          imageUrl: tokenImg,
          completed: false
        }
      : undefined;

  const notes: string[] = [];
  if (quests.length === 0) notes.push('今日任务条目未拉到，请对照任务长图。');
  if (!seasonImg) notes.push('季蜡配图未拉到。');

  return {
    dateStr: todayStr,
    dayOfWeekStr: getTodayDayOfWeek(),
    fetchedAt: Date.now(),
    isFallback: false,
    parseNote: notes.join(' ') || undefined,
    sourcePostText: raw.post_text,
    overviewImages: {
      seasonCandleImg: seasonImg,
      questsImg,
      treasureCandleImg: '',
      shardImg,
      tokenImg
    },
    quests,
    seasonalCandles: seasonal,
    treasureCandles: [],
    activityImages: Array.from((raw.post_text || '').matchAll(/图\s*(\d+)\s*[-—–:：·.]\s*([^\n]+)/g))
      .map(match => ({ caption: match[2].trim(), url: images[Number(match[1]) - 1] || '' }))
      .filter(image => image.url && /攻略|代币|兑换|路线|位置|任务/.test(image.caption)),
    eventDaily
  };
}

function questKeys(quest: SkyDailyQuestItem): string[] {
  return [quest.id, quest.title];
}

export const SKY_DAILY_SCOPES = [QUEST_SCOPE, CANDLE_SCOPE, EVENT_SCOPE];
export interface SkyDailySelection { quests: boolean; candles: boolean; event: boolean }

export function syncSkyDailyChecks(payload: SkyDailyGuidePayload, characterId: string) {
  const quests = dailyChecked(QUEST_SCOPE, payload.dateStr, characterId);
  const candles = dailyChecked(CANDLE_SCOPE, payload.dateStr, characterId);
  const events = dailyChecked(EVENT_SCOPE, payload.dateStr, characterId);
  payload.quests.forEach(q => { q.completed = questKeys(q).some(k => quests.has(k)); });
  payload.seasonalCandles.spots.forEach(s => { s.completed = candles.has(String(s.index)); });
  if (payload.eventDaily) payload.eventDaily.completed = events.has(payload.eventDaily.name);
}

/** 手动清单与代练同步共用范围解释；未选项目完全不写入。 */
export function completeSkyDailySelection(payload: SkyDailyGuidePayload, characterId: string, selection: SkyDailySelection, events: SkyEventDaily[] = payload.eventDaily ? [payload.eventDaily] : []) {
  const groups: Record<string, string[]> = {};
  if (selection.quests) groups[QUEST_SCOPE] = payload.quests.flatMap(questKeys);
  if (selection.candles) groups[CANDLE_SCOPE] = payload.seasonalCandles.spots.map(s => String(s.index));
  if (selection.event) groups[EVENT_SCOPE] = events.map(e => e.id || e.name);
  return setCharacterDailyChecks(characterId, payload.dateStr, groups, true);
}
