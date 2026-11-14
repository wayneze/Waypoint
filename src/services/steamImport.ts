import type { GameRecord, ObsidianGameGuide, SteamLibraryGame } from '../types';
import { recordSteamIds, type SteamActivity } from './gameActivity';
import { titleKey } from './localGames';
import { gameTitleKeys } from './gameIdentity';

export interface SteamRow {
  game: SteamLibraryGame;
  title: string;
  original: string;
  record?: GameRecord;
  suggestions: GameRecord[];
  guide?: ObsidianGameGuide;
  installed: boolean;
  inLibrary: boolean;
  linked: boolean;
  source?: string;
}
export type SteamFilter = 'all' | 'library' | 'new' | 'unlinked';
export type SteamInstallFilter = 'all' | 'installed' | 'uninstalled';
export type SteamSort = 'library' | 'installed' | 'recent' | 'playtime' | 'name';
export function needsSteamTitle(row: SteamRow): boolean { return !/\p{Script=Han}/u.test(row.title); }

export function buildSteamRows(games: SteamLibraryGame[], records: GameRecord[], guides: ObsidianGameGuide[], activity: SteamActivity): SteamRow[] {
  const installed = new Set(activity.installed.map(g => g.appid));
  const links = new Map(activity.links.map(l => [l.appid, l.record_id]));
  return games.map(game => {
    const exact = records.filter(r => recordSteamIds(r).includes(game.appid));
    const record = records.find(r => r.id === links.get(game.appid)) || (exact.length === 1 ? exact[0] : undefined);
    const localized = activity.titles[game.appid];
    const names = new Set([game.name, localized?.title || ''].flatMap(gameTitleKeys));
    const suggestions = record ? [] : records.filter(r => {
      const ids = recordSteamIds(r);
      const bound = activity.links.find(l => l.record_id === r.id);
      if ((bound && bound.appid !== game.appid) || (ids.length && !ids.includes(game.appid))) return false;
      return exact.includes(r) || [r.title, r.library?.metadata?.title || '', r.library?.metadata?.original_title || '', ...r.library?.metadata?.aliases || []]
        .some(n => gameTitleKeys(n).some(key => names.has(key)));
    });
    const linkedGuides = guides.filter(g => Object.values(g.links || {}).some(value => {
      try { const u = new URL(value); return u.hostname === 'store.steampowered.com' && u.pathname.split('/')[2] === String(game.appid); }
      catch { return false; }
    }));
    const namedGuides = linkedGuides.length ? [] : guides.filter(g => {
      const steam = Object.values(g.links || {}).flatMap(value => {
        try { const u = new URL(value); return u.hostname === 'store.steampowered.com' ? [Number(u.pathname.split('/')[2])] : []; } catch { return []; }
      });
      return (!steam.length || steam.includes(game.appid)) &&
        [g.title, g.original_title || '', ...g.aliases].some(n => gameTitleKeys(n).some(key => names.has(key)));
    });
    const guide = linkedGuides.length === 1 ? linkedGuides[0] : namedGuides.length === 1 ? namedGuides[0] : undefined;
    const personal = record?.title !== game.name && /\p{Script=Han}/u.test(record?.title || '') ? record?.title : undefined;
    const matchedChinese = record?.library?.metadata?.title;
    const suggestedChinese = suggestions.length === 1 && /\p{Script=Han}/u.test(suggestions[0]!.title) ? suggestions[0]!.title : undefined;
    const title = personal || (guide?.title && /\p{Script=Han}/u.test(guide.title) ? guide.title : '') || suggestedChinese
      || (matchedChinese && /\p{Script=Han}/u.test(matchedChinese) ? matchedChinese : '') || localized?.title || record?.title || guide?.title || game.name;
    return { game, title, original: title === game.name ? '' : game.name, record, suggestions, guide,
      installed: installed.has(game.appid), inLibrary: !!record || !!guide || suggestions.length > 0,
      linked: !!links.get(game.appid), source: localized?.source };
  });
}

export function selectSteamRows(rows: SteamRow[], query: string, filter: SteamFilter, install: SteamInstallFilter, sort: SteamSort, onlySelected: boolean, selected: number[]): SteamRow[] {
  const key = titleKey(query.trim());
  const ids = new Set(selected);
  return rows.filter(r => (!key || [r.title, r.game.name, String(r.game.appid), ...r.suggestions.map(s => s.title)].some(n => titleKey(n).includes(key)))
    && (filter === 'all' || (filter === 'library' ? r.inLibrary : filter === 'new' ? !r.inLibrary : !r.linked))
    && (install === 'all' || r.installed === (install === 'installed'))
    && (!onlySelected || ids.has(r.game.appid)))
    .sort((a, b) => {
      let order = 0;
      if (sort === 'library') order = Number(b.inLibrary) - Number(a.inLibrary) || Number(b.linked) - Number(a.linked);
      if (sort === 'installed') order = Number(b.installed) - Number(a.installed);
      if (sort === 'playtime') order = b.game.playtime_forever - a.game.playtime_forever;
      if (sort === 'recent' || sort === 'library' || sort === 'installed')
        order ||= (b.game.rtime_last_played || 0) - (a.game.rtime_last_played || 0);
      return order || a.title.localeCompare(b.title, 'zh-Hans-CN', { numeric: true }) || a.game.appid - b.game.appid;
    });
}
