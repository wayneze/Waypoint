import type { GameLibraryDetails, GameMetadata, GameRecord, ObsidianGameGuide } from '../types';
import { sanitizeAndEnrichGuide } from './gameCovers';
import { gameTitleKeys } from './gameIdentity';
import { curatedGameTaxonomy, sourceGameTaxonomy } from './gameTaxonomy';

export function withGameMetadata(current: GameLibraryDetails, metadata: GameMetadata): GameLibraryDetails {
  const old = current.metadata;
  const sameWork = old && ((old.source === metadata.source && old.source_id === metadata.source_id)
    || metadata.sources?.some(s => s.source === old.source && s.source_id === old.source_id)
    || old.sources?.some(s => s.source === metadata.source && s.source_id === metadata.source_id));
  // Refreshing one work preserves its media; selecting another work clears the old artwork.
  const merged = sameWork ? {
    ...metadata,
    screenshots: [...new Set([...metadata.screenshots, ...old.screenshots])],
    videos: metadata.videos?.length ? metadata.videos : old.videos,
    music: metadata.music?.length ? metadata.music : old.music,
    background: metadata.background || old.background,
  } : metadata;
  return { ...current, metadata: merged, matching: null,
    background: sameWork || !old ? current.background : null,
    background_position: sameWork || !old ? current.background_position : null };
}

export function matchingGuides(record: Pick<GameRecord, 'title' | 'library'>, guides: ObsidianGameGuide[]) {
  const meta = record.library?.metadata;
  const sources = meta ? [meta, ...meta.sources || []] : [];
  const identities = new Map(sources.map(source => [source.source, source.source_id]));
  const steamIds = [...new Set(record.library?.installations.map(i => i.steam_id).filter(Boolean))];
  if (!identities.has('steam') && steamIds.length === 1) identities.set('steam', steamIds[0]!);
  const linked = guides.filter(guide => Object.values(guide.links || {}).some(url => {
    const identity = sourceIdentity(url);
    return identity && identities.get(identity[0]) === identity[1];
  }));
  if (linked.length) return linked;
  const keys = new Set([record.title, meta?.title || '', meta?.original_title || '', ...meta?.aliases || []].flatMap(gameTitleKeys));
  return guides.filter(guide => !Object.values(guide.links || {}).some(url => {
    const identity = sourceIdentity(url);
    return identity && identities.has(identity[0]) && identities.get(identity[0]) !== identity[1];
  })).filter(guide => [guide.title, guide.original_title || '', ...guide.aliases]
    .some(title => gameTitleKeys(title).some(name => keys.has(name))));
}

export function sourceIdentity(value: string): ['vndb' | 'bangumi' | 'steam', string] | undefined {
  try {
    const url = new URL(value);
    if (!['http:', 'https:'].includes(url.protocol)) return;
    if (['vndb.org', 'www.vndb.org'].includes(url.hostname) && /^\/v\d+\/?$/.test(url.pathname))
      return ['vndb', url.pathname.split('/')[1]!];
    if (['bgm.tv', 'bangumi.tv', 'chii.in'].includes(url.hostname) && /^\/subject\/\d+\/?$/.test(url.pathname))
      return ['bangumi', url.pathname.split('/')[2]!];
    if (url.hostname === 'store.steampowered.com' && /^\/app\/\d+(\/|$)/.test(url.pathname))
      return ['steam', url.pathname.split('/')[2]!];
  } catch { /* 不是作品直链时，只参与名称匹配。 */ }
}

export interface GameShelf {
  key: string;
  series?: string;
  games: ObsidianGameGuide[];
  total: number;
}

/** 书封上保留一个名称；完整标题仍用于档案与悬停提示。 */
export function libraryCardTitle(game: Pick<ObsidianGameGuide, 'title' | 'series'>, member = false): string {
  let title = member && game.series && game.title.startsWith(game.series)
    ? game.title.slice(game.series.length).replace(/^\s*\d*(?:\.\d+)?\s*[-—·:：]?\s*/, '') || game.title
    : game.title;
  const alias = title.match(/^(.*?)\s*[（(]([^（）()]*)[）)]\s*$/);
  if (alias && !/(?:steam|switch|xbox|ps[345]|重制|重製|汉化|漢化|高清|完整版|限定版|版本|edition|remaster)/i.test(alias[2]!)) {
    if (/\p{Script=Han}/u.test(alias[1]!) && !/\p{Script=Han}/u.test(alias[2]!)) title = alias[1]!;
    else if (!/\p{Script=Han}/u.test(alias[1]!) && /\p{Script=Han}/u.test(alias[2]!)) title = alias[2]!;
  }
  return title.trim();
}

/** 沿用筛选后的排序；系列出现在首个成员的位置，内部按作品顺序排列。 */
export function buildGameShelves(visible: ObsidianGameGuide[], all: ObsidianGameGuide[]): GameShelf[] {
  const shelves: GameShelf[] = [];
  const series = new Map<string, GameShelf>();
  for (const game of visible) {
    const name = game.series?.trim();
    if (name) {
      let shelf = series.get(name);
      if (!shelf) {
        shelf = { key: `series:${name}`, series: name, games: [], total: all.filter(g => g.series?.trim() === name).length };
        series.set(name, shelf);
        shelves.push(shelf);
      }
      shelf.games.push(game);
    } else {
      let shelf = shelves[shelves.length - 1];
      if (!shelf || shelf.series) {
        shelf = { key: `standalone:${game.id}`, games: [], total: 0 };
        shelves.push(shelf);
      }
      shelf.games.push(game);
      shelf.total++;
    }
  }
  for (const shelf of series.values()) shelf.games.sort((a, b) =>
    (a.series_order ?? Infinity) - (b.series_order ?? Infinity)
    || (a.release_date || '').localeCompare(b.release_date || '')
    || a.title.localeCompare(b.title, 'zh-Hans-CN', { numeric: true }));
  return shelves;
}

export function gameSourceIds(guide: ObsidianGameGuide): string[] {
  return [...new Set([guide.id, guide.record_id].filter((id): id is string => !!id))];
}

/** 合并只生成展示视图；攻略 ID 继续拥有路线和书签，原档案仍独立保存。 */
export function buildGameLibrary(guides: ObsidianGameGuide[], records: GameRecord[]): ObsidianGameGuide[] {
  const assignments = new Map<string, GameRecord[]>();
  for (const record of records) {
    const candidates = record.guide_id == null
      ? matchingGuides(record, guides)
      : guides.filter(guide => guide.id === record.guide_id);
    if (candidates.length !== 1) continue;
    const id = candidates[0]!.id;
    assignments.set(id, [...(assignments.get(id) || []), record]);
  }
  // 多份档案指向同一攻略时保留独立入口，不能任意丢掉其中一份。
  const attached = new Set<string>();
  const merged = guides.map(guide => {
    const matches = assignments.get(guide.id);
    if (matches?.length !== 1) return guide;
    const record = matches[0]!;
    attached.add(record.id);
    return {
      ...guide, record_id: record.id, title: record.title,
      aliases: [...new Set([...guide.aliases, guide.title])],
      status: record.status === 'playing' ? '在玩' : record.status === 'completed' ? '已通关' : '未开始',
      rating: record.rating, rating_personal: record.rating,
      progress_note: record.progress || guide.progress_note,
      personal_notes: record.notes,
      poster: record.poster || guide.poster, poster_position: record.poster_position,
    };
  });
  const result = [...merged, ...records.filter(record => !attached.has(record.id)).map(record => ({
    ...sanitizeAndEnrichGuide({
      id: record.id, title: record.title, aliases: [], category: record.category,
      status: record.status === 'playing' ? '在玩' : record.status === 'completed' ? '已通关' : '未开始',
      rating: record.rating, tags: [], genres: [], file_path: '', relative_path: '',
      file_name: record.title, modified: '',
    }),
    record_id: record.id, rating: record.rating, rating_personal: record.rating,
    summary: record.notes, personal_notes: record.notes, progress_note: record.progress,
    poster_position: record.poster_position,
    ...(record.poster ? { poster: record.poster } : {}),
  }))];
  return result.map(guide => {
    const record = records.find(r => r.id === guide.record_id);
    const collection = record?.library?.collection;
    if (collection) guide = { ...guide, series: collection.name.trim() || undefined, series_order: collection.name.trim() ? collection.order : undefined };
    const meta = record?.library?.metadata;
    const taxonomy = curatedGameTaxonomy(sourceGameTaxonomy(guide, meta), record?.library?.curation);
    guide = { ...guide, taxonomy, tags: taxonomy.tags, developer: taxonomy.developers.join(' / '), publisher: taxonomy.publishers.join(' / ') };
    if (!meta) return guide;
    return {
      ...guide, poster: record?.poster || meta.cover || guide.poster,
      summary: meta.description || guide.summary,
      aliases: [...new Set([...guide.aliases, meta.title, meta.original_title, ...meta.aliases || []].filter(Boolean))],
      release_date: meta.release_date || guide.release_date,
      year: meta.release_date?.slice(0, 4) || guide.year,
      original_title: meta.original_title || guide.original_title,
      links: { ...guide.links, ...Object.fromEntries((meta.sources || []).map(s => [s.source, s.url])), [meta.source]: meta.url },
      ...(meta.source === 'vndb' ? { rating_vndb: meta.score } : meta.source === 'bangumi' ? { rating_bangumi: meta.score } : {}),
    };
  });
}
