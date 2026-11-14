import type {
  Jx3AdventureItem,
  Jx3AdventureReqType,
  Jx3AdventureRequirement,
  Jx3AdventureSeriesMode,
  Jx3ReputationItem,
  Jx3ReputationLevel
} from '../types';

function isJx3AchieveId(id: string | undefined | null): boolean {
  return !!id && /^\d+$/.test(String(id).trim());
}

export const ADVENTURES_CACHE_KEY = 'jx3.adventures';
export const ADVENTURE_CATALOG_TTL_MS = 12 * 60 * 60 * 1000;

const REPUTATION_LEVELS: Jx3ReputationLevel[] = ['中立', '友好', '亲密', '敬重', '尊敬', '钦佩'];

export interface Jx3BoxSerendipityRow {
  dwID?: number;
  szName?: string | null;
  szFront?: string | null;
  nClassify?: number | null;
  bPerfect?: boolean | null;
  szReward?: string | null;
  RewdItem?: string | null;
  nCampCanSee?: number | null;
  szFirstPagePath?: string | null;
  szRewardPath?: string | null;
  szOpenRewardPath?: string | null;
  szRewardType?: string | null;
}

export const JX3_ADVENTURE_IMG_ROOT = 'https://img.jx3box.com/adventure/';

export function jx3AdventureAssetUrl(gamePath?: string | null, client = 'std'): string {
  let p = String(gamePath || '').trim().replace(/\\/g, '/').toLowerCase();
  if (!p) return '';
  p = p.replace(/^\/+/, '').replace(/^ui\/image\/adventure\//, '');
  p = p.replace(/\.tga$/i, '.png');
  if (!p.endsWith('.png')) return '';
  return `${JX3_ADVENTURE_IMG_ROOT}adventure/${client}/${p}`;
}

export function jx3AdventureCoverUrl(row: Pick<Jx3BoxSerendipityRow, 'szFirstPagePath' | 'szRewardPath' | 'szOpenRewardPath'>): string {
  return jx3AdventureAssetUrl(row.szOpenRewardPath)
    || jx3AdventureAssetUrl(row.szRewardPath)
    || jx3AdventureAssetUrl(row.szFirstPagePath);
}

export interface Jx3AdventureNameMaps {
  achieveNameIds: Record<string, string>;
  seriesIds: Record<string, string[]>;
  completeAchieveIds: Record<string, string>;
}

const POINT_WAN = /江湖资历达到\s*([\d.]+)\s*万/;
const POINT_RAW = /江湖资历达到\s*(\d+)\s*点/;
const REP_REACH = /声望达到(尊敬|钦佩|敬重|亲密|友好|友善)/;
const ALL_SERIES = /完成所有[^\[\n]*\[([^\]]+)\]成就/;
const COUNT_SERIES = /完成\s*(?:(\d+)|([一二两三四五六七八九十]))\s*个[^\[\n]*\[([^\]]+)\]成就/;
const ACHIEVE_BRACKETS = /\[([^\]]+)\]成就/g;
const TASK_BRACKETS = /(?:完成任务\[([^\]]+)\]|完成\[([^\]]+)\]任务)/;
const ITEM_OWN = /拥有\s*(\d+)?(?:块)?\[([^\]]+)\]/;
const GOLD = /金币达到\s*(\d+)\s*砖/;
const CN_COUNT: Record<string, number> = {
  一: 1, 二: 2, 两: 2, 三: 3, 四: 4, 五: 5, 六: 6, 七: 7, 八: 8, 九: 9, 十: 10
};

export function splitFront(front: string): string[] {
  return (front || '')
    .split(/\r?\n/)
    .map(s => s.replace(/\\n/g, '\n'))
    .flatMap(s => s.split('\n'))
    .map(s => s.trim())
    .filter(Boolean);
}

export function isEmptyFront(front: string | null | undefined): boolean {
  const t = (front || '').trim();
  return !t || t === '等待探索' || t === '无';
}

function newReq(
  id: string,
  type: Jx3AdventureReqType,
  name: string,
  extra: Partial<Jx3AdventureRequirement> = {}
): Jx3AdventureRequirement {
  return {
    id,
    type,
    name,
    completed: false,
    ...extra
  };
}

function reputationRef(
  rawName: string,
  reputations: Pick<Jx3ReputationItem, 'id' | 'name'>[]
): string | undefined {
  const n = rawName.replace(/主城|当前/g, '').trim();
  if (!n) return undefined;
  if (/^阵营$/.test(n)) return undefined;
  if (n.includes('浩气') || n.includes('恶人')) {
    return reputations.find(r => r.id === 'rep_faction' || /浩气|恶人|阵营/.test(r.name))?.id;
  }
  const hit = reputations.find(r => {
    const short = r.name.split(/[\/／(（]/)[0].trim();
    return r.name === n || short === n || r.name.includes(n) || n.includes(short);
  });
  return hit?.id;
}

function parseReputationLine(
  line: string,
  idBase: string,
  reputations: Pick<Jx3ReputationItem, 'id' | 'name'>[]
): Jx3AdventureRequirement[] | null {
  const levelHit = line.match(REP_REACH);
  const shortHit = line.match(/声望(尊敬|钦佩|敬重|亲密|友好|友善)/);
  const rawLevel = levelHit?.[1] || shortHit?.[1];
  const minLevel = (rawLevel === '友善' ? '友好' : rawLevel) as Jx3ReputationLevel | undefined;
  if (!minLevel || !REPUTATION_LEVELS.includes(minLevel)) return null;
  if (!line.includes('声望')) return null;

  const head = line
    .replace(REP_REACH, '')
    .replace(/声望(尊敬|钦佩|敬重|亲密|友好|友善).*$/, '')
    .replace(/声望/g, '')
    .trim();
  const parts = head.split(/[、，,]/).map(s => s.replace(/主城/g, '').trim()).filter(Boolean);
  if (!parts.length) {
    return [newReq(`${idBase}_rep`, 'reputation', line, {
      minLevel,
      match: 'unlinked'
    })];
  }
  return parts.map((part, i) => {
    const refId = reputationRef(part, reputations);
    const reputationIds = /当前阵营/.test(part)
      ? reputations.filter(rep => /^(浩气盟|恶人谷|浩气盟\s*\/\s*恶人谷)$/.test(rep.name)).map(rep => rep.id)
      : undefined;
    return newReq(`${idBase}_rep_${i}`, 'reputation', `${part}声望达到${minLevel}`, {
      refId,
      reputationIds,
      minLevel,
      match: refId || reputationIds?.length ? undefined : 'unlinked'
    });
  });
}

function resolveAchieveName(
  name: string,
  maps: Jx3AdventureNameMaps
): { refId?: string; seriesIds?: string[]; seriesMode?: Jx3AdventureSeriesMode } {
  const exact = maps.achieveNameIds[name] || maps.completeAchieveIds[name];
  if (exact) return { refId: exact };
  const series = maps.seriesIds[name];
  if (series?.length) return { seriesIds: series, seriesMode: 'all' };
  return {};
}

function parseCount(line: string): { n: number; prefix: string } | null {
  const hit = line.match(COUNT_SERIES);
  if (!hit) return null;
  const n = hit[1] ? Number(hit[1]) : (CN_COUNT[hit[2] || ''] || 0);
  const prefix = (hit[3] || '').trim();
  if (!n || !prefix) return null;
  return { n, prefix };
}

function parseAchieveLine(
  line: string,
  idBase: string,
  maps: Jx3AdventureNameMaps
): Jx3AdventureRequirement | null {
  const allHit = line.match(ALL_SERIES);
  if (allHit) {
    const prefix = allHit[1].trim();
    const ids = maps.seriesIds[prefix] || [];
    return newReq(`${idBase}_ach`, 'achievement', line, {
      seriesMode: 'all',
      seriesIds: ids,
      refId: ids.length === 1 ? ids[0] : undefined,
      match: ids.length ? undefined : 'unlinked'
    });
  }
  const countHit = parseCount(line);
  if (countHit) {
    const ids = maps.seriesIds[countHit.prefix] || [];
    return newReq(`${idBase}_ach`, 'achievement', line, {
      seriesMode: 'any',
      targetVal: countHit.n,
      seriesIds: ids,
      match: ids.length ? undefined : 'unlinked'
    });
  }

  const names: string[] = [];
  ACHIEVE_BRACKETS.lastIndex = 0;
  for (const m of line.matchAll(ACHIEVE_BRACKETS)) names.push(m[1].trim());
  if (!names.length) {
    const bare = line.match(/完成\[([^\]]+)\](?!任务)/);
    if (bare && !/任务/.test(line)) names.push(bare[1].trim());
  }
  if (!names.length) {
    const got = line.match(/获得\[([^\]]+)\]成就/);
    if (got) names.push(got[1].trim());
  }
  if (!names.length) return null;

  const resolved = names.map(n => ({ name: n, ...resolveAchieveName(n, maps) }));
  const linked = resolved.filter(r => isJx3AchieveId(r.refId) || (r.seriesIds && r.seriesIds.length));
  if (linked.length === 1) {
    const hit = linked[0];
    const seriesIds = hit.seriesIds || (hit.refId ? [hit.refId] : []);
    const anyOf = /或/.test(line) && names.length > 1;
    return newReq(`${idBase}_ach`, 'achievement', line, {
      refId: hit.refId,
      seriesIds: seriesIds.length > 1 ? seriesIds : undefined,
      seriesMode: anyOf ? 'any' : hit.seriesMode,
      targetVal: anyOf ? 1 : undefined,
      match: seriesIds.length || hit.refId ? undefined : 'unlinked',
      unresolvedAlternatives: /或/.test(line) && (/任务/.test(line) || names.length > linked.length),
      unresolvedRequired: !/或/.test(line) && (names.length > linked.length || /任务/.test(line))
    });
  }
  if (linked.length > 1) {
    const ids = linked.flatMap(r => r.seriesIds || (r.refId ? [r.refId] : []));
    return newReq(`${idBase}_ach`, 'achievement', line, {
      seriesIds: ids,
      seriesMode: /或/.test(line) ? 'any' : 'all',
      targetVal: /或/.test(line) ? 1 : undefined,
      unresolvedAlternatives: /或/.test(line) && names.length > linked.length,
      unresolvedRequired: !/或/.test(line) && names.length > linked.length
    });
  }
  return newReq(`${idBase}_ach`, 'achievement', line, { match: 'unlinked' });
}

export function parseSerendipityFront(
  front: string,
  idBase: string,
  maps: Jx3AdventureNameMaps,
  reputations: Pick<Jx3ReputationItem, 'id' | 'name'>[]
): Jx3AdventureRequirement[] {
  if (isEmptyFront(front)) return [];
  const out: Jx3AdventureRequirement[] = [];
  splitFront(front).forEach((line, idx) => {
    const rid = `${idBase}_${idx}`;
    const wan = line.match(POINT_WAN);
    if (wan) {
      out.push(newReq(`${rid}_pt`, 'point', line, {
        targetVal: Math.round(Number(wan[1]) * 10000),
        match: 'unlinked'
      }));
      return;
    }
    const rawPt = line.match(POINT_RAW);
    if (rawPt) {
      out.push(newReq(`${rid}_pt`, 'point', line, {
        targetVal: Number(rawPt[1]),
        match: 'unlinked'
      }));
      return;
    }
    if (line.includes('声望')) {
      const reps = parseReputationLine(line, rid, reputations);
      if (reps) {
        out.push(...reps);
        return;
      }
    }
    const ach = parseAchieveLine(line, rid, maps);
    if (ach) {
      out.push(ach);
      return;
    }
    const task = line.match(TASK_BRACKETS);
    if (task || (/完成/.test(line) && /任务/.test(line))) {
      out.push(newReq(`${rid}_q`, 'quest', line, { match: 'unlinked' }));
      return;
    }
    const gold = line.match(GOLD);
    if (gold) {
      out.push(newReq(`${rid}_gold`, 'item', line, {
        targetVal: Number(gold[1]),
        match: 'unlinked'
      }));
      return;
    }
    const item = line.match(ITEM_OWN) || /拥有\[/.test(line) || /获得\[/.test(line);
    if (item) {
      out.push(newReq(`${rid}_item`, 'item', line, { match: 'unlinked' }));
      return;
    }
    out.push(newReq(`${rid}_other`, 'other', line, { match: 'unlinked' }));
  });
  return out;
}

export function classifySerendipity(row: Jx3BoxSerendipityRow): Jx3AdventureItem['type'] {
  if (Number(row.nClassify) === 1) return '宠物奇遇';
  const front = row.szFront || '';
  if (/加入阵营|当前阵营|阵营声望/.test(front) || row.nCampCanSee) return '阵营奇遇';
  if (row.bPerfect) return '绝世奇遇';
  return '百家奇遇';
}

export function completeAdventureSearchName(name: string, type: Jx3AdventureItem['type']): string {
  return type === '宠物奇遇' ? `宠物奇缘·${name}` : name;
}

export function compactAdventureName(name: string): string {
  return (name || '').replace(/[·・.\s]/g, '');
}

export function parseCompleteAdventureHits(
  achievements: Array<{ ID?: number; Name?: string; Desc?: string; ShortDesc?: string }>
): Record<string, string> {
  const out: Record<string, string> = {};
  const pat = /完成奇遇[《「](.+?)[》」]/;
  const put = (key: string, id: string) => {
    const name = key.trim();
    if (!name) return;
    out[name] = id;
    out[compactAdventureName(name)] = id;
  };
  for (const a of achievements) {
    const text = `${a.Desc || ''}${a.ShortDesc || ''}`;
    const m = text.match(pat);
    const id = a.ID != null ? String(a.ID) : '';
    if (!isJx3AchieveId(id)) continue;
    if (m) put(m[1], id);
  }
  return out;
}

function minPointOf(reqs: Jx3AdventureRequirement[]): number {
  let max = 0;
  for (const r of reqs) {
    if (r.type === 'point' && r.targetVal && r.targetVal > max) max = r.targetVal;
  }
  return max;
}

/** `RewdItem` 是「表_编号;」串，比如 `5_87714;5_87715;`。 */
export function rewardItemIds(row: Pick<Jx3BoxSerendipityRow, 'RewdItem'>): string[] {
  return Array.from(new Set(
    String(row.RewdItem || '').split(/[;,]/).map(s => s.trim()).filter(s => /^\d+_\d+$/.test(s))
  ));
}

/** 奖励一行字：去重后列前两件，多的写件数。名字查不到的 id 不显示。 */
export function rewardSummary(ids: string[], itemNames: Record<string, string>): string {
  const names = Array.from(new Set(ids.map(id => itemNames[id]).filter((n): n is string => !!n)));
  if (!names.length) return '';
  return names.length > 2 ? `${names.slice(0, 2).join('、')} 等 ${names.length} 件` : names.join('、');
}

export function parseItemNamesPayload(raw: unknown): Record<string, string> {
  const list = raw && typeof raw === 'object' && Array.isArray((raw as { list?: unknown }).list)
    ? (raw as { list: Array<{ id?: unknown; Name?: unknown }> }).list
    : [];
  const out: Record<string, string> = {};
  for (const it of list) {
    const id = typeof it?.id === 'string' ? it.id : '';
    const name = typeof it?.Name === 'string' ? it.Name.trim() : '';
    if (id && name) out[id] = name;
  }
  return out;
}

export function buildJx3AdventureCatalog(
  rows: Jx3BoxSerendipityRow[],
  maps: Jx3AdventureNameMaps,
  reputations: Pick<Jx3ReputationItem, 'id' | 'name'>[],
  itemNames: Record<string, string> = {}
): Jx3AdventureItem[] {
  const seen = new Set<number>();
  const list: Jx3AdventureItem[] = [];
  for (const row of rows) {
    const dwID = Number(row.dwID);
    const name = (row.szName || '').trim();
    if (!Number.isFinite(dwID) || dwID <= 0 || !name) continue;
    if (seen.has(dwID)) continue;
    seen.add(dwID);
    const type = classifySerendipity(row);
    const searchName = completeAdventureSearchName(name, type);
    const completeAchieveId = maps.completeAchieveIds[searchName]
      || maps.completeAchieveIds[compactAdventureName(searchName)]
      || maps.completeAchieveIds[name]
      || maps.completeAchieveIds[compactAdventureName(name)];
    const requirements = parseSerendipityFront(row.szFront || '', `adv_${dwID}`, maps, reputations);
    list.push({
      id: `adv_${dwID}`,
      name,
      type,
      reward: rewardSummary(rewardItemIds(row), itemNames),
      triggerDaily: '',
      minPoint: minPointOf(requirements),
      completeAchieveId: isJx3AchieveId(completeAchieveId) ? completeAchieveId : undefined,
      coverUrl: jx3AdventureCoverUrl(row) || undefined,
      requirements,
      isTriggered: false
    });
  }
  list.sort((a, b) => {
    const ia = Number(a.id.slice(4));
    const ib = Number(b.id.slice(4));
    return ib - ia || a.name.localeCompare(b.name, 'zh');
  });
  return list;
}

export function collectNeedlesFromRows(rows: Jx3BoxSerendipityRow[]): {
  achieveNames: string[];
  seriesPrefixes: string[];
} {
  const names = new Set<string>();
  const prefixes = new Set<string>();
  for (const row of rows) {
    const front = row.szFront || '';
    for (const line of splitFront(front)) {
      const allHit = line.match(ALL_SERIES);
      if (allHit) prefixes.add(allHit[1].trim());
      const countHit = parseCount(line);
      if (countHit) prefixes.add(countHit.prefix);
      ACHIEVE_BRACKETS.lastIndex = 0;
      for (const m of line.matchAll(ACHIEVE_BRACKETS)) names.add(m[1].trim());
      const bare = line.match(/完成\[([^\]]+)\](?!任务)/);
      if (bare && !/任务/.test(line) && !/成就/.test(line)) names.add(bare[1].trim());
      const got = line.match(/获得\[([^\]]+)\]成就/);
      if (got) names.add(got[1].trim());
    }
  }
  for (const p of prefixes) names.delete(p);
  return { achieveNames: Array.from(names), seriesPrefixes: Array.from(prefixes) };
}

export function relinkAdventureReputationRefs(
  adventures: Jx3AdventureItem[],
  reputations: Pick<Jx3ReputationItem, 'id' | 'name'>[]
): Jx3AdventureItem[] {
  return adventures.map(adv => ({
    ...adv,
    requirements: (adv.requirements || []).map(req => {
      if (req.type !== 'reputation' || req.refId) return req;
      const head = req.name
        .replace(REP_REACH, '')
        .replace(/声望(尊敬|钦佩|敬重).*$/, '')
        .replace(/声望/g, '')
        .trim();
      const refId = reputationRef(head || req.name, reputations);
      return refId ? { ...req, refId, match: undefined } : req;
    })
  }));
}

export function parseAchievementSearchHits(
  achievements: Array<{ ID?: number; Name?: string }>,
  keyword: string
): { exactId?: string; prefixIds: string[] } {
  const exact: string[] = [];
  const prefix: string[] = [];
  for (const a of achievements) {
    const id = a.ID != null ? String(a.ID) : '';
    const name = (a.Name || '').trim();
    if (!isJx3AchieveId(id) || !name) continue;
    if (name === keyword) exact.push(id);
    if (name.startsWith(keyword)) prefix.push(id);
  }
  return { exactId: exact[0], prefixIds: prefix };
}

export function parseSerendipityListPayload(raw: unknown): {
  list: Jx3BoxSerendipityRow[];
  pages: number;
  total: number;
} {
  const root = (raw && typeof raw === 'object') ? raw as Record<string, unknown> : {};
  const listRaw = Array.isArray(root.list) ? root.list : [];
  const list = listRaw.filter(x => x && typeof x === 'object') as Jx3BoxSerendipityRow[];
  const total = Number(root.total) || list.length;
  const per = Number(root.per) || 50;
  const pages = Number(root.pages) || Math.max(1, Math.ceil(total / Math.max(1, per)));
  return { list, pages, total };
}

export function parseAchievementSearchPayload(raw: unknown): {
  achievements: Array<{ ID?: number; Name?: string; Desc?: string; ShortDesc?: string; Point?: number; IconID?: number }>;
  pages: number;
  total: number;
} {
  const root = (raw && typeof raw === 'object') ? raw as Record<string, unknown> : {};
  const data = (root.data && typeof root.data === 'object')
    ? root.data as Record<string, unknown>
    : root;
  const achievements = Array.isArray(data.achievements)
    ? data.achievements.filter(x => x && typeof x === 'object') as Array<{
      ID?: number; Name?: string; Desc?: string; ShortDesc?: string; Point?: number; IconID?: number;
    }>
    : [];
  const total = Number(data.total) || achievements.length;
  const per = Number(data.per_page || data.per) || 15;
  const pages = Math.max(1, Math.ceil(total / Math.max(1, per)));
  return { achievements, pages, total };
}
