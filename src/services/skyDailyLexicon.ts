/**
 * 光遇每日博文词库解析：按栏目切开正文，再套 daily-lexicon.json 抽任务/地图。
 * 解析不到就不编造任务或暮土点位。
 */

import lexiconRaw from '../content/sky/daily-lexicon.json';

export interface ParsedDailyQuest {
  id: string;
  title: string;
  category: 'social' | 'spirit' | 'meditation' | 'collect' | 'creature';
  categoryLabel: string;
  realm: string;
  detailSpot: string;
  tip: string;
}

export interface ParsedCandleGroup {
  realm: string;
  groupName: string;
  headline: string;
  spots: Array<{ index: number; label: string; desc: string }>;
}

export interface ParsedEventDaily {
  name: string;
  window?: string;
  how?: string;
}

export interface ParsedSkyDailyPost {
  quests: ParsedDailyQuest[];
  seasonal?: ParsedCandleGroup;
  treasure?: ParsedCandleGroup;
  activityName?: string;
  eventDaily?: ParsedEventDaily;
  imageHints: Partial<Record<'seasonCandle' | 'quests' | 'treasureCandle' | 'shard' | 'token', number>>;
}

type QuestRule = {
  id: string;
  title: string;
  any?: string[];
  all?: string[];
  category: ParsedDailyQuest['category'];
  categoryLabel: string;
  realm?: string;
  needRealm?: boolean;
  tip: string;
};

type LexiconFile = {
  realms: string[];
  sectionNames: Record<string, string[]>;
  imageRoles: Array<{ role: 'seasonCandle' | 'quests' | 'treasureCandle' | 'shard' | 'token'; any: string[] }>;
  quests: QuestRule[];
};

const LEX = lexiconRaw as LexiconFile;

const NOISE_SECTION = /落石|复刻|日程|飞行日|代币|日历|兑换树/;

function fold(s: string): string {
  return (s || '').replace(/\r/g, '').replace(/[ \t]+/g, ' ').trim();
}

function findRealm(text: string): string | undefined {
  if (!text) return undefined;
  for (const r of LEX.realms) {
    if (text.includes(r)) return r;
  }
  return undefined;
}

/** 国服季蜡地图：云野→雨林→霞谷→暮土→禁阁，每日一图。原点 2026-09-01=云野，已对 2026-08-31～09-08 游离长图。 */
const CN_SEASON_REALMS = ['云野', '雨林', '霞谷', '暮土', '禁阁'] as const;
const CN_SEASON_ORIGIN_UTC = Date.UTC(2026, 8, 1);

export function cnSeasonalCandleRealm(dateStr: string): string {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dateStr);
  if (!m) return CN_SEASON_REALMS[0];
  const day = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  const diff = Math.round((day - CN_SEASON_ORIGIN_UTC) / 86_400_000);
  return CN_SEASON_REALMS[((diff % 5) + 5) % 5];
}

function ruleHits(text: string, q: QuestRule): boolean {
  const allOk = !q.all?.length || q.all.every(k => text.includes(k));
  const anyOk = !q.any?.length || q.any.some(k => text.includes(k));
  if (!q.all?.length && !q.any?.length) return false;
  return allOk && anyOk;
}

function nextHeaderIndex(text: string, from: number): number {
  const slice = text.slice(from);
  const m = slice.search(/图\s*\d|=『|【|#|每日任务|季蜡|大蜡|落石点|日程提醒/);
  if (m < 0) return text.length;
  return from + m;
}

function isStandaloneHeader(text: string, index: number, header: string): boolean {
  const before = index === 0 ? '\n' : text[index - 1];
  const after = text[index + header.length] || '\n';
  const lineStartOk = index === 0 || before === '\n' || before === ' ' || /[·\-—|]/.test(before);
  const lineEndOk = /[:：\s\n]/.test(after);
  if (header === '季蜡' || header === '大蜡' || header === '大蜡烛' || header === '季节蜡烛') {
    const line = text.slice(Math.max(0, text.lastIndexOf('\n', index)), text.indexOf('\n', index) === -1 ? text.length : text.indexOf('\n', index));
    if (/每日任务/.test(line) && /季蜡/.test(line) && /大蜡/.test(line)) return false;
  }
  return lineStartOk && lineEndOk;
}

function sliceAfterHeaders(text: string, headers: string[], maxLen = 280): string {
  let best = '';
  for (const h of headers) {
    let from = 0;
    while (from < text.length) {
      const i = text.indexOf(h, from);
      if (i < 0) break;
      from = i + h.length;
      if (!isStandaloneHeader(text, i, h)) continue;
      const start = i + h.length;
      const end = Math.min(nextHeaderIndex(text, start), start + maxLen);
      const slice = fold(text.slice(start, end));
      if (slice.length > best.length) best = slice;
    }
  }
  return best;
}

function stripNoiseSections(text: string): string {
  return text.replace(/=『[^』]{1,20}』=[\s\S]*?(?==『|$)/g, block => {
    const head = block.slice(0, 24);
    return NOISE_SECTION.test(head) ? '\n' : block;
  });
}

function unlabeledSpot(realm: string, kind: 'season' | 'treasure'): ParsedCandleGroup['spots'] {
  const label = kind === 'season' ? '季蜡点位见配图' : '大蜡点位见配图';
  return [{ index: 1, label, desc: `${realm} · 正文未标出分堆，请对照长图` }];
}

function parseSpotsFromText(block: string, realm: string): ParsedCandleGroup['spots'] | null {
  if (!block) return null;
  const lines = block
    .split(/[\n／/|；;]/)
    .map(fold)
    .filter(Boolean);
  const numbered: ParsedCandleGroup['spots'] = [];
  for (const line of lines) {
    const m = line.match(/^([1-4一二三四①②③④])[.、:：\s]+(.+)$/);
    if (!m) continue;
    const map: Record<string, number> = {
      '1': 1, '2': 2, '3': 3, '4': 4,
      一: 1, 二: 2, 三: 3, 四: 4,
      '①': 1, '②': 2, '③': 3, '④': 4
    };
    const index = map[m[1]] || numbered.length + 1;
    numbered.push({ index, label: fold(m[2]).slice(0, 24), desc: `${realm} · ${fold(m[2])}` });
  }
  if (numbered.length >= 2) return numbered.slice(0, 4);
  return null;
}

function questFromRule(q: QuestRule, line: string, globalRealm: string): ParsedDailyQuest {
  const realm = q.needRealm
    ? findRealm(line) || q.realm || globalRealm || '未标明地图'
    : q.realm || findRealm(line) || globalRealm || '任意';
  return {
    id: q.id,
    title: q.title,
    category: q.category,
    categoryLabel: q.categoryLabel,
    realm,
    detailSpot: fold(line).slice(0, 80) || realm,
    tip: q.tip
  };
}

function parseNumberedQuestLines(block: string): ParsedDailyQuest[] {
  const lines = block
    .split(/\n|(?=[1-4一二三四①②③④][.、])/)
    .map(fold)
    .filter(Boolean);
  const out: ParsedDailyQuest[] = [];
  const used = new Set<string>();
  for (const line of lines) {
    const m = line.match(/^([1-4一二三四①②③④])[.、:：\s]+(.+)$/);
    if (!m) continue;
    const body = fold(m[2]);
    const rule = LEX.quests.find(q => ruleHits(body, q) && !used.has(q.id));
    if (rule) {
      used.add(rule.id);
      out.push(questFromRule(rule, body, findRealm(body) || ''));
    } else {
      out.push({
        id: `line_${out.length + 1}`,
        title: body.slice(0, 32),
        category: 'collect',
        categoryLabel: '解析',
        realm: findRealm(body) || '未标明地图',
        detailSpot: body,
        tip: ''
      });
    }
    if (out.length >= 4) break;
  }
  return out;
}

/** 社区日历条目：标题以接口原文为准，词库只补地图/分类/提示。 */
export function questFromCalendarText(questId: string, questCn: string, index: number): ParsedDailyQuest {
  const body = fold(questCn);
  const rule = LEX.quests.find(q => ruleHits(body, q));
  if (rule) {
    const q = questFromRule(rule, body, findRealm(body) || '');
    q.id = questId || rule.id;
    q.title = body;
    q.detailSpot = q.categoryLabel;
    return q;
  }
  return {
    id: questId || `cal_${index + 1}`,
    title: body,
    category: 'collect',
    categoryLabel: '每日',
    realm: findRealm(body) || '未标明地图',
    detailSpot: '每日',
    tip: ''
  };
}

function parseKeywordQuests(block: string): ParsedDailyQuest[] {
  const quests: ParsedDailyQuest[] = [];
  const used = new Set<string>();
  const globalRealm = findRealm(block) || '';
  for (const q of LEX.quests) {
    if (!ruleHits(block, q) || used.has(q.id)) continue;
    used.add(q.id);
    const nearby = q.any?.find(k => block.includes(k)) || q.all?.[0] || q.title;
    const i = block.indexOf(nearby);
    const window = i >= 0 ? fold(block.slice(Math.max(0, i - 12), i + 48)) : block.slice(0, 48);
    quests.push(questFromRule(q, window, globalRealm));
    if (quests.length >= 4) break;
  }
  return quests;
}

function findLabeledRealm(text: string, keys: string[]): string | undefined {
  for (const k of keys) {
    const re = new RegExp(`${k}[:：·\\s]*([\\u4e00-\\u9fa5]{2,6})`, 'g');
    let m: RegExpExecArray | null;
    while ((m = re.exec(text))) {
      const lineStart = text.lastIndexOf('\n', m.index);
      const line = text.slice(lineStart + 1, text.indexOf('\n', m.index) === -1 ? text.length : text.indexOf('\n', m.index));
      if (/每日任务/.test(line) && /季蜡/.test(line) && /大蜡/.test(line)) continue;
      const realm = findRealm(m[1]) || findRealm(m[0]);
      if (realm) return realm;
    }
  }
  return findRealm(sliceAfterHeaders(text, keys));
}

function parseCandleGroup(
  text: string,
  keys: string[],
  kind: 'season' | 'treasure'
): ParsedCandleGroup | undefined {
  const block = sliceAfterHeaders(text, keys);
  const realm = findLabeledRealm(text, keys);
  if (!realm) return undefined;
  const spots = parseSpotsFromText(block, realm) || unlabeledSpot(realm, kind);
  const label = kind === 'season' ? '季蜡' : '大蜡烛';
  return {
    realm,
    groupName: '解析',
    headline: `${realm} · ${label}`,
    spots
  };
}

function parseImageHints(text: string): ParsedSkyDailyPost['imageHints'] {
  const imageHints: ParsedSkyDailyPost['imageHints'] = {};
  for (const role of LEX.imageRoles) {
    for (const k of role.any) {
      const re = new RegExp(`图\\s*(\\d)\\s*[-—–:：·.\\s]*[^\\n]{0,8}${k}`);
      const m = text.match(re);
      if (m) {
        imageHints[role.role] = Math.max(0, parseInt(m[1], 10) - 1);
        break;
      }
    }
  }
  return imageHints;
}

export function parseSkyDailyPost(raw: string): ParsedSkyDailyPost {
  const text = fold(raw).replace(/<br\s*\/?>/gi, '\n');
  const imageHints = parseImageHints(text);

  const questHeaders = LEX.sectionNames.quests || ['每日任务', '日常任务'];
  const questBlock = sliceAfterHeaders(text, questHeaders, 400);
  const fullQuestText = stripNoiseSections(text);
  const questSource = questBlock || fullQuestText;
  let quests = parseNumberedQuestLines(questSource);
  if (quests.length < 2) {
    const keyed = parseKeywordQuests(questSource);
    if (keyed.length > quests.length) quests = keyed;
  }
  if (quests.length < 2 && fullQuestText !== questSource) {
    const numbered = parseNumberedQuestLines(fullQuestText);
    const keyed = parseKeywordQuests(fullQuestText);
    const retry = numbered.length >= keyed.length ? numbered : keyed;
    if (retry.length > quests.length) quests = retry;
  }

  const seasonal = parseCandleGroup(text, LEX.sectionNames.seasonal || ['季蜡', '季节蜡烛'], 'season');
  const treasure = parseCandleGroup(text, LEX.sectionNames.treasure || ['大蜡', '大蜡烛'], 'treasure');

  let activityName: string | undefined;
  const boxed = text.match(/『([^』]{2,12})』/);
  if (boxed && /日|节|季|兑换/.test(boxed[1])) activityName = boxed[1];

  return {
    quests: quests.slice(0, 4),
    seasonal,
    treasure,
    activityName,
    eventDaily: parseEventDaily(text, activityName),
    imageHints
  };
}

function parseEventDaily(text: string, boxedName?: string): ParsedEventDaily | undefined {
  const nameMatch = text.match(/『([^』]{2,16})』/);
  const name = boxedName || (nameMatch && /日|节|季|兑换/.test(nameMatch[1]) ? nameMatch[1] : undefined);
  const eventBlock = name
    ? sliceAfterHeaders(text, [name, `『${name}』`], 280)
    : '';
  const src = eventBlock || text;
  const windowMatch = src.match(/时间[:：]\s*([^\n]{6,40})/);
  const howMatch = src.match(/每日\s*(?:\d+\s*(?:枚|个)?\s*)?代币获取方式[:：]\s*([^\n]+)/);
  const howText = howMatch ? fold(howMatch[1]) : '';
  // 被微博列表截断的句子不能当作完整攻略。
  const how = howText && !/(?:\.{3}|…)\s*全文|展开全文/.test(howText) ? howText : undefined;
  if (!name && !how) return undefined;
  return {
    name: name || '活动日常',
    window: windowMatch ? fold(windowMatch[1]) : undefined,
    how
  };
}
