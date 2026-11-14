// 探索页站点名单。默认值只是第一次的种子，之后增删只改本机列表。
import { kvGetJson, kvSetJson } from './kv';
import type { ObsidianGameGuide } from '../types';

export type ExploreRole = 'opinion' | 'guide' | 'resource' | 'patch' | 'tool';
export type ExploreScope = 'galgame' | 'single';

export interface ExploreSite {
  id: string;
  name: string;
  home: string;
  /** 含 {q} 时按游戏名打开搜索；没有就只开首页。 */
  search?: string;
  /** 能直接取回标题的接口。 */
  result?: 'wp' | 'bangumi' | 'steam' | 'steam-guides' | 'dfan' | 'dfan-guides' | 'seiya' | 'galgames' | 'loli' | 'touch' | 'qingju' | 'juij';
  role: ExploreRole;
  scope: ExploreScope;
  hidden: boolean;
  /** 同一角色里第一个未隐藏的站是搜索条的默认动作。 */
  order: number;
}

/** 发现页的站点分类；工具入口跨游戏类别共用。 */
export const EXPLORE_ROLES: { id: ExploreRole; label: string }[] = [
  { id: 'opinion', label: '口碑' },
  { id: 'resource', label: '资源' },
  { id: 'guide', label: '攻略' },
  { id: 'patch', label: '补丁' },
  { id: 'tool', label: '工具' },
];

const KEY = 'waypoint_explore_sites';

const SEED: ExploreSite[] = [
  { id: '2dfan', name: '2DFan', home: 'https://2dfan.com/', search: 'https://2dfan.com/search?keyword={q}', result: 'dfan', role: 'opinion', scope: 'galgame', hidden: false, order: 0 },
  { id: 'bangumi', name: 'Bangumi', home: 'https://bgm.tv/', search: 'https://bgm.tv/subject_search/{q}?cat=4', result: 'bangumi', role: 'opinion', scope: 'galgame', hidden: false, order: 1 },
  { id: 'hikari', name: 'HIKARI FIELD', home: 'https://store.hikarifield.co.jp/', role: 'opinion', scope: 'galgame', hidden: false, order: 2 },
  { id: 'vndb', name: 'VNDB', home: 'https://vndb.org/', search: 'https://vndb.org/v?sq={q}', role: 'opinion', scope: 'galgame', hidden: false, order: 3 },
  { id: 'steam', name: 'Steam', home: 'https://store.steampowered.com/', search: 'https://store.steampowered.com/search/?term={q}', result: 'steam', role: 'opinion', scope: 'single', hidden: false, order: 0 },
  { id: 'steamdb', name: 'SteamDB', home: 'https://steamdb.info/', search: 'https://steamdb.info/search/?a=app&q={q}', role: 'opinion', scope: 'single', hidden: false, order: 1 },
  { id: 'seiya', name: '誠也の部屋 · 日文攻略', home: 'https://seiya-saiga.com/game/kouryaku.html', search: 'https://seiya-saiga.com/game/kouryaku.html', result: 'seiya', role: 'guide', scope: 'galgame', hidden: false, order: 0 },
  { id: '2dfan-guides', name: '2DFan · 中文攻略', home: 'https://2dfan.com/', search: 'https://2dfan.com/search?keyword={q}', result: 'dfan-guides', role: 'guide', scope: 'galgame', hidden: false, order: 1 },
  { id: 'dldou', name: 'DLdou', home: 'https://dldou.com/', search: 'https://dldou.com/?s={q}', result: 'wp', role: 'guide', scope: 'galgame', hidden: false, order: 2 },
  { id: 'steam-guides', name: 'Steam 社区指南', home: 'https://steamcommunity.com/?subsection=guides', result: 'steam-guides', role: 'guide', scope: 'single', hidden: false, order: 0 },
  { id: 'tiangal', name: '天遊二次元', home: 'https://www.tiangal.com/', search: 'https://www.tiangal.com/search/{q}', result: 'wp', role: 'resource', scope: 'galgame', hidden: false, order: 0 },
  { id: 'qingju', name: '青桔网', home: 'https://x.qingju.org/', search: 'https://x.qingju.org/zh-CN/search?keyword={q}', result: 'qingju', role: 'resource', scope: 'galgame', hidden: false, order: 1 },
  { id: 'zi0', name: '梓澪の妙妙屋', home: 'https://gal.games/', search: 'https://gal.games/search?q={q}', result: 'galgames', role: 'resource', scope: 'galgame', hidden: false, order: 2 },
  { id: 'touchgal', name: 'TouchGal', home: 'https://www.touchgal.ink/', search: 'https://www.touchgal.ink/search', result: 'touch', role: 'resource', scope: 'galgame', hidden: false, order: 3 },
  { id: 'ttloli', name: '忧郁的loli', home: 'https://www.ttloli.com/', search: 'https://www.ttloli.com/?s={q}', result: 'loli', role: 'resource', scope: 'galgame', hidden: false, order: 4 },
  { id: 'kagura', name: 'KAGURA FAN', home: 'https://kagurafan.com/', search: 'https://kagurafan.com/?s={q}', result: 'wp', role: 'patch', scope: 'galgame', hidden: false, order: 0 },
  { id: 'juij', name: '非线性列车 · Juij', home: 'https://h.juij.fun/', search: 'https://h.juij.fun/search/?keyword={q}', result: 'juij', role: 'patch', scope: 'single', hidden: false, order: 0 },
  { id: 'luna', name: 'LunaTranslator', home: 'https://docs.lunatranslator.org/zh/', role: 'tool', scope: 'galgame', hidden: false, order: 0 },
  { id: 'dango', name: '团子翻译器', home: 'https://translator.dango.cloud/', role: 'tool', scope: 'galgame', hidden: false, order: 1 },
];

const ROLES = new Set<ExploreRole>(['opinion', 'guide', 'resource', 'patch', 'tool']);
const SCOPES = new Set<ExploreScope>(['galgame', 'single']);

export function isHttp(value: string): boolean {
  try { const url = new URL(value); return ['http:', 'https:'].includes(url.protocol) && !!url.hostname && !url.username && !url.password; } catch { return false; }
}

// 已发布版本的官方旧地址；仅迁移这些值，用户自定义地址、顺序和隐藏状态保留。
const PREVIOUS_DEFAULTS: Record<string, { home?: string[]; search?: string[] }> = {
  seiya: { home: ['https://seiya-saiga.com/'] },
  qingju: { home: ['https://qingju.org/'] },
  zi0: { home: ['https://zi0.cc/', 'https://zi6.cc/'], search: ['https://zi0.cc/?s={q}', 'https://zi6.cc/?s={q}'] },
  'steam-guides': { home: ['https://store.steampowered.com/'], search: ['https://store.steampowered.com/search/?term={q}'] },
  luna: { home: ['https://docs.lunatranslator.org/'] },
};

function cleanSite(raw: unknown, index: number): ExploreSite | null {
  if (!raw || typeof raw !== 'object') return null;
  const row = raw as Partial<ExploreSite>;
  const home = typeof row.home === 'string' ? row.home.trim() : '';
  const name = typeof row.name === 'string' ? row.name.trim() : '';
  if (!name || !isHttp(home)) return null;
  const search = typeof row.search === 'string' && isHttp(row.search.replace('{q}', 'x'))
    ? row.search.trim()
    : undefined;
  const result = row.result === 'wp' || row.result === 'bangumi' || row.result === 'steam' || row.result === 'steam-guides'
    || row.result === 'dfan' || row.result === 'dfan-guides' || row.result === 'seiya' || row.result === 'galgames'
    || row.result === 'loli' || row.result === 'touch' || row.result === 'qingju' || row.result === 'juij'
    ? row.result
    : undefined;
  const role = ROLES.has(row.role as ExploreRole) ? row.role as ExploreRole : 'resource';
  const scope = SCOPES.has(row.scope as ExploreScope) ? row.scope as ExploreScope : 'galgame';
  const id = typeof row.id === 'string' && row.id.trim() ? row.id.trim() : `site_${index}`;
  return {
    id,
    name,
    home,
    search,
    result,
    role,
    scope,
    hidden: row.hidden === true,
    order: Number.isFinite(row.order) ? Number(row.order) : index,
  };
}

export function loadExploreSites(): ExploreSite[] {
  const stored = kvGetJson<unknown>(KEY, null);
  if (!Array.isArray(stored)) {
    kvSetJson('waypoint_chinese_guide_site_added', true);
    return SEED.map(site => ({ ...site }));
  }
  const sites = stored.map(cleanSite).filter((site): site is ExploreSite => !!site).map(site => {
    const seed = SEED.find(item => item.id === site.id);
    if (!seed) return site;
    const previous = PREVIOUS_DEFAULTS[site.id];
    const official = site.home === seed.home || previous?.home?.includes(site.home);
    if (!official) return site;
    return {
      ...site,
      home: seed.home,
      search: !site.search || previous?.search?.includes(site.search) ? seed.search : site.search,
      result: seed.result,
    };
  });
  // One published addition: remember it separately so deleting it stays deleted.
  if (!kvGetJson('waypoint_chinese_guide_site_added', false)) {
    if (sites.length && !sites.some(site => site.id === '2dfan-guides')) sites.push({ ...SEED.find(site => site.id === '2dfan-guides')! });
    kvSetJson('waypoint_chinese_guide_site_added', true);
    saveExploreSites(sites);
  }
  return sites;
}

export function saveExploreSites(sites: ExploreSite[]) {
  kvSetJson(KEY, sites);
}

export function exploreQuery(guide: Pick<ObsidianGameGuide, 'title' | 'original_title' | 'aliases'>): string {
  const original = guide.original_title?.trim();
  if (original && original !== guide.title) return original;
  const alias = (guide.aliases || []).map(name => name.trim()).find(Boolean);
  return alias || guide.title;
}

export function exploreTerms(site: ExploreSite, query: string, guide?: ObsidianGameGuide): string[] {
  if (!guide) return [query.trim()];
  const names = [...new Set([guide.original_title, guide.title, ...guide.aliases].filter((name): name is string => !!name?.trim()).map(name => name.trim()))];
  if (!names.includes(query.trim())) return [query.trim()];
  const expanded = [...new Set(names.flatMap(name => {
    const normalized = name.normalize('NFKC');
    return [...Array.from(normalized.matchAll(/\(([^)]+)\)/g), m => m[1]!), normalized.replace(/\([^)]*\)/g, '').trim(), normalized];
  }).filter(Boolean))];
  const japanese = (name: string) => /[\u3040-\u30ff]/.test(name);
  const chinese = (name: string) => /[\u3400-\u9fff]/.test(name) && !japanese(name);
  const preferred = site.result === 'seiya' ? japanese : site.result === 'dfan-guides' || site.id === 'dldou' ? chinese : () => false;
  return [...expanded.filter(preferred), query.trim(), ...expanded].filter((name, index, all) => all.indexOf(name) === index).slice(0, 4);
}

export function directLink(guide: ObsidianGameGuide, siteId: string): string | undefined {
  const links = guide.links || {};
  const keys = siteId === 'bangumi' ? ['bangumi', 'bgm'] : [siteId];
  for (const key of keys) {
    const url = links[key];
    if (url && isHttp(url)) return url;
  }
  return undefined;
}

export function siteUrl(site: ExploreSite, query: string, guide?: ObsidianGameGuide): string {
  if (guide && site.id === 'steam-guides') {
    const steam = directLink(guide, 'steam');
    const appId = steam?.match(/^https?:\/\/(?:store\.steampowered\.com|steamcommunity\.com)\/app\/(\d+)/)?.[1];
    if (appId) return `https://steamcommunity.com/app/${appId}/guides/`;
  }
  if (guide) {
    const linked = directLink(guide, site.id);
    if (linked) return linked;
  }
  const q = query.trim();
  if (site.search && q) return site.search.split('{q}').join(encodeURIComponent(q));
  return site.home;
}

export function defaultSite(sites: ExploreSite[], role: ExploreRole, scope: ExploreScope): ExploreSite | undefined {
  return sites
    .filter(site => !site.hidden && site.role === role && site.scope === scope)
    .sort((a, b) => a.order - b.order)[0];
}

export function newExploreSite(partial: Pick<ExploreSite, 'name' | 'home' | 'role' | 'scope'> & { search?: string }): ExploreSite {
  return {
    id: `site_${Date.now().toString(36)}`,
    name: partial.name.trim(),
    home: partial.home.trim(),
    search: partial.search?.trim() || undefined,
    role: partial.role,
    scope: partial.scope,
    hidden: false,
    order: Date.now(),
  };
}

export function siteSearchLabel(site: ExploreSite): string {
  if (site.result === 'steam-guides') return '查找游戏的社区指南';
  if (site.result) return '可在这里查看结果';
  return site.search?.includes('{q}') ? '前往网站搜索' : '访问站点';
}
