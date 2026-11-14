import { expect, test } from 'vitest';
import { buildGameLibrary, buildGameShelves, libraryCardTitle, matchingGuides, withGameMetadata } from './gameLibrary';
import { emptyTaxonomyFilter, matchesGameTaxonomy } from './gameTaxonomy';
import { sanitizeAndEnrichGuide } from './gameCovers';
import type { GameRecord, ObsidianGameGuide } from '../types';

const guide: ObsidianGameGuide = { id: 'guide', title: 'STEINS;GATE', aliases: ['命运石之门'], category: 'galgame', status: '在玩', rating: 8, tags: [], genres: [], file_path: 'guide.md', file_name: 'guide', relative_path: 'guide.md', modified: '' };
const record: GameRecord = { id: 'record', title: '命运石之门', category: 'galgame', status: 'playing', rating: 9, progress: '' };

test('identity matching avoids sequels and ambiguous names, supports explicit links and survives missing scans', () => {
  expect(buildGameLibrary([guide], [record])).toHaveLength(1);
  expect(buildGameLibrary([guide], [{ ...record, title: '命运石之门0' }])).toHaveLength(2);
  expect(buildGameLibrary([{ ...guide, title: '沙耶之歌（Steam 重制版）', aliases: [] }], [{ ...record, title: '另一款游戏（Steam 重制版）' }])).toHaveLength(2);
  expect(buildGameLibrary([guide, { ...guide, id: 'other' }], [record])).toHaveLength(3);
  expect(buildGameLibrary([guide], [record, { ...record, id: 'other' }])).toHaveLength(3);
  expect(buildGameLibrary([guide], [{ ...record, guide_id: '' }])).toHaveLength(2);
  const linked = { ...record, title: '我的名字', guide_id: guide.id };
  expect(buildGameLibrary([guide], [linked])[0]?.record_id).toBe(record.id);
  expect(buildGameLibrary([], [linked])[0]?.id).toBe(record.id);
  expect(buildGameLibrary([guide], [linked])[0]?.id).toBe(guide.id);
});

test('series members match their source identity without borrowing another volume cover or progress', () => {
  const first = { ...guide, id: 'first', title: '壳之少女', aliases: [], series: '少女三部曲', series_order: 1, poster: 'https://t.vndb.org/cv/53/94153.jpg', links: { vndb: 'https://vndb.org/v810' } };
  const second = { ...first, id: 'second', title: '虚之少女', series_order: 2, poster: 'https://t.vndb.org/cv/62/88062.jpg', links: { vndb: 'https://vndb.org/v5922' } };
  const imported = { ...record, title: '本地安装文件夹', library: { installations: [], metadata: {
    source: 'vndb' as const, source_id: 'v5922', title: 'Kara no Shoujo - The Second Episode', original_title: '虚ノ少女',
    description: '', cover: '', screenshots: [], developer: '', release_date: '', score: 0, url: 'https://vndb.org/v5922',
  } } };
  expect(matchingGuides(imported, [first, second])).toEqual([second]);
  expect(matchingGuides({ ...imported, title: first.title }, [first])).toEqual([]);
  expect(matchingGuides({ title: '少女三部曲' }, [first, second])).toEqual([]);
  const merged = buildGameLibrary([first, second].map(sanitizeAndEnrichGuide), [imported]);
  expect(merged).toHaveLength(2);
  expect(merged[0]?.record_id).toBeUndefined();
  expect(merged[1]).toMatchObject({ id: 'second', record_id: 'record', series: '少女三部曲', series_order: 2, poster: second.poster });

  // 真实旧攻略的模板编号与已匹配档案冲突；扫描归一化必须使用现有核验身份。
  const oldGuide = { ...guide, title: '命运石之门', original_title: 'Steins;Gate',
    links: { vndb: 'https://vndb.org/v200', bangumi: 'https://bgm.tv/subject/12007' } };
  const currentRecord = { ...record, title: 'STEINS;GATE (命运石之门)', library: {
    ...imported.library, metadata: { ...imported.library.metadata!, source_id: 'v2002', title: '命运石之门', original_title: 'STEINS;GATE' },
  } };
  const normalized = sanitizeAndEnrichGuide(oldGuide);
  expect(buildGameLibrary([normalized], [currentRecord])).toMatchObject([{ id: 'guide', record_id: 'record', file_path: 'guide.md' }]);
  expect(oldGuide.links.vndb).toBe('https://vndb.org/v200');
  const spinoff = sanitizeAndEnrichGuide({ ...oldGuide, title: '命运石之门 比翼恋理的爱人',
    original_title: 'STEINS;GATE 比翼恋理のだーりん', aliases: ['命运石之门'],
    poster: 'https://example.com/darling.jpg', file_name: 'darling.md', links: { vndb: 'https://vndb.org/v6618' } });
  expect(spinoff).toMatchObject({ poster: 'https://example.com/darling.jpg', links: { vndb: 'https://vndb.org/v6618' } });
  expect(matchingGuides(currentRecord, [spinoff])).toEqual([]);
});

test('series shelves retain every filtered work, count the full library and order members independently', () => {
  const first = { ...guide, id: 'first', series: '系列', series_order: 1 };
  const second = { ...guide, id: 'second', series: '系列', series_order: 2 };
  const standalone = { ...guide, id: 'standalone' };
  const all = [second, standalone, first];
  const grouped = buildGameShelves(all, all);
  expect(grouped.map(s => s.games.map(g => g.id))).toEqual([['first', 'second'], ['standalone']]);
  expect(buildGameShelves([second], all)[0]).toMatchObject({ total: 2, games: [second] });
  expect(buildGameShelves([], all)).toEqual([]);
  const joined = { ...record, guide_id: 'first', library: { installations: [], collection: { name: '自选合集', order: 3 } } };
  expect(buildGameLibrary(all, [joined]).find(g => g.id === 'first')).toMatchObject({ series: '自选合集', series_order: 3 });
  const removed = { ...joined, library: { ...joined.library, collection: { name: '' } } };
  expect(buildGameLibrary(all, [removed]).find(g => g.id === 'first')?.series).toBeUndefined();
  expect(first.series).toBe('系列');
  expect(libraryCardTitle({ title: '美好的每一天不连续存在（Subarashiki Hibi ~Furenzoku Exist~）' })).toBe('美好的每一天不连续存在');
  expect(libraryCardTitle({ title: 'STEINS;GATE (命运石之门)' })).toBe('命运石之门');
  expect(libraryCardTitle({ title: '沙耶之歌（Steam 重制版）' })).toBe('沙耶之歌（Steam 重制版）');
  expect(libraryCardTitle({ title: '美少女万华镜2 - 勿忘草与永远的少女', series: '美少女万华镜' }, true)).toBe('勿忘草与永远的少女');
});

test('one taxonomy resolves old source fields and personal edits, survives refresh and powers combined filters', () => {
  const metadata = { source: 'bangumi' as const, source_id: '3154', title: guide.title, original_title: '',
    description: '', cover: '', screenshots: [], developer: '', release_date: '', score: 0, url: '',
    tags: ['PC', '悬疑', 'galgame'], info: [
      { label: '开发', value: 'Nitro+ / 5pb.' }, { label: '发行商', value: 'Spike Chunsoft' },
      { label: '剧本', value: '林直孝 / 下倉バイオ' }, { label: '原画', value: 'huke' },
    ] };
  const library = { installations: [], metadata, curation: {
    hidden_tags: ['pc'], added_tags: ['我喜欢', 'GALGAME'],
    developers: ['我的公司'], publishers: [], staff: [{ name: '林直孝', role: '剧本' }, { name: 'huke', role: '原画' }],
  } };
  const entry = () => buildGameLibrary([{ ...guide, tags: ['Galgame', '旧标签'] }], [{ ...record, library }])[0]!;
  expect(entry().taxonomy).toEqual({ tags: ['GALGAME', '旧标签', '悬疑', '我喜欢'],
    developers: ['我的公司'], publishers: [], staff: library.curation.staff });
  const filters = { ...emptyTaxonomyFilter(), tags: ['悬疑', '我喜欢'], companies: ['我的公司'], people: ['huke'], staff_role: '原画' };
  expect(matchesGameTaxonomy(entry().taxonomy!, filters)).toBe(true);
  expect(matchesGameTaxonomy(entry().taxonomy!, { ...filters, staff_role: '剧本' })).toBe(false);
  expect(matchesGameTaxonomy(entry().taxonomy!, { ...filters, excluded: ['悬疑'] })).toBe(false);
  expect(matchesGameTaxonomy(entry().taxonomy!, { ...filters, tags: ['不存在', '悬疑'], tag_mode: 'any' })).toBe(true);
  expect(matchesGameTaxonomy(entry().taxonomy!, { ...filters, company_role: 'publisher' })).toBe(false);
  const refreshed = withGameMetadata(library, { ...metadata, tags: ['PC', '新增源标签'], developer: '新公司' });
  const next = buildGameLibrary([guide], [{ ...record, library: refreshed }])[0]!;
  expect(next.tags).toEqual(['新增源标签', '我喜欢', 'GALGAME']);
  expect(next.taxonomy?.developers).toEqual(['我的公司']);
  expect(next.taxonomy?.publishers).toEqual([]);
  expect(next.taxonomy?.staff).toEqual(library.curation.staff);
  const automatic = buildGameLibrary([guide], [{ ...record, library: { ...refreshed, curation: {} } }])[0]!;
  expect(automatic.taxonomy?.developers).toEqual(['新公司', 'Nitro+', '5pb.']);
  expect(automatic.taxonomy?.staff).toContainEqual({ name: 'huke', role: '原画' });
  expect(automatic.taxonomy?.publishers).toEqual(['Spike Chunsoft']);
});

test('scanned guides with null company credits still load alongside records and other guides', () => {
  const scanned = { ...guide, developer: null, publisher: null };
  const works = buildGameLibrary([scanned, { ...guide, id: 'no-company', title: '未填写公司', aliases: [] }], [record]);
  expect(works).toHaveLength(2);
  expect(works.find(g => g.record_id === record.id)?.taxonomy).toEqual({
    tags: [], developers: [], publishers: [], staff: [],
  });
});
