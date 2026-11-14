import { expect, test, vi } from 'vitest';

const storage = new Map<string, string>();
vi.stubGlobal('localStorage', {
  getItem: (key: string) => storage.get(key) ?? null,
  setItem: (key: string, value: string) => { storage.set(key, value); },
});

vi.mock('../core', async () => {
  const { ref, reactive } = await import('vue');
  return {
    gameRecords: ref([]), obsidianGameGuides: ref([]), obsidianVaultPath: ref(''),
    isScanningGuides: ref(false), showToast: vi.fn(),
    state: reactive({ showAddGameModal: false, newGame: {} }),
  };
});
vi.mock('../../services/systemApi', () => ({
  scanObsidianGameGuides: vi.fn(async () => []),
  readObsidianFile: vi.fn(async () => '# 攻略内容'),
  openInExplorer: vi.fn(), openInObsidian: vi.fn(), updateGuideFrontmatter: vi.fn(),
}));

test('persisted game records remain browsable after scanning and never read the save directory as a guide', async () => {
  const { gameRecords, obsidianVaultPath } = await import('../core');
  const { loadObsidianGuides, singleAndGalgameGuides } = await import('./library');
  const { handleOpenGuideReader, guideContent } = await import('./reader');
  const { readObsidianFile } = await import('../../services/systemApi');
  gameRecords.value = [{
    id: 'local-record', title: '游玩记录测试', category: 'single', status: 'playing',
    progress: '第三章', rating: 0, save_path: 'D:/Games/Test/saves', notes: '回来继续探索。',
  }];
  await loadObsidianGuides();
  expect(singleAndGalgameGuides.value.map(g => g.id)).toEqual(['local-record']);
  obsidianVaultPath.value = 'D:/Notes';
  await loadObsidianGuides();
  const entry = singleAndGalgameGuides.value[0]!;
  expect(entry.id).toBe('local-record');
  expect(entry.file_path).toBe('');
  expect(entry.rating).toBe(0);
  await handleOpenGuideReader(entry);
  expect(readObsidianFile).not.toHaveBeenCalled();
  expect(guideContent.value).toContain('第三章');
  expect(guideContent.value).toContain('回来继续探索。');
});

test('switching guides ignores an older read finishing after the newly selected guide', async () => {
  const { handleOpenGuideReader, guideContent, selectedGuide, isLoadingGuideContent } = await import('./reader');
  const { readObsidianFile } = await import('../../services/systemApi');
  let finishFirst!: (value: string) => void;
  vi.mocked(readObsidianFile).mockImplementationOnce(() => new Promise(resolve => { finishFirst = resolve; }));
  const guide = { id: 'first', title: 'First', file_path: 'first.md' } as import('../../types').ObsidianGameGuide;
  const first = handleOpenGuideReader(guide);
  vi.mocked(readObsidianFile).mockResolvedValueOnce('# Second');
  await handleOpenGuideReader({ ...guide, id: 'second', title: 'Second', file_path: 'second.md' });
  finishFirst('# First');
  await first;
  expect(selectedGuide.value?.id).toBe('second');
  expect(guideContent.value).toBe('# Second');
  expect(isLoadingGuideContent.value).toBe(false);
});

test('saved web guides reopen offline without losing body, files or the original game identity', async () => {
  const { saveGameLink, loadGameExtras, linksForGames } = await import('./extras');
  const { handleOpenResourceReader, guideContent, selectedGuide, guideReadError } = await import('./reader');
  const game: import('../../types').ObsidianGameGuide = {
    id: 'offline-game', title: 'Original Game', file_path: '', aliases: [], tags: [], genres: [],
    category: 'single', status: 'playing', rating: 0, relative_path: '', file_name: '', modified: '',
  };
  const url = 'https://example.org/guide';
  const link = { title: '攻略', url, site: 'Example', role: 'guide' as const, article: { title: '攻略', url, markdown: '# 路线攻略\n\n## 第一章\n\n选择左侧路线。' }, local_files: ['D:/Downloads/patch.zip'] };
  await Promise.all([saveGameLink(game.id, link), saveGameLink(game.id, { title: '另一份资料', url: 'https://example.org/other', site: 'Example', role: 'patch' })]);
  await saveGameLink(game.id, { title: '重命名攻略', url, site: 'Example', role: 'guide' });
  loadGameExtras();
  const saved = linksForGames([game.id]).find(item => item.url === url)!;
  expect(saved.article?.markdown).toBe(link.article.markdown);
  expect(saved.local_files).toEqual(link.local_files);
  expect(linksForGames([game.id])).toHaveLength(2);
  await handleOpenResourceReader(game, saved);
  expect(guideReadError.value).toBe('');
  expect(guideContent.value).toBe(link.article.markdown);
  expect(selectedGuide.value?.resource_game_id).toBe(game.id);
  expect(selectedGuide.value?.file_path).toBe('');
});

test('editing persists the same record and preserves personal values, bookmarks and saved links', async () => {
  const { gameRecords, state } = await import('../core');
  const { openGameRecordEditor, handleSaveGameRecord, singleAndGalgameGuides } = await import('./library');
  const { guideProgressMap } = await import('./reader');
  const { saveGameLink, gameLinks } = await import('./extras');
  const { getGameRecords, saveGameRecord } = await import('../../services/api');
  const kv = await import('../../services/kv');
  openGameRecordEditor();
  Object.assign(state.newGame, { title: '白色相簿2', progress: '第一章', rating: 0, notes: '短备忘' });
  await handleSaveGameRecord();
  const record = gameRecords.value[gameRecords.value.length - 1]!;
  record.library = { installations: [{ root: 'D:/Games', directory: 'D:/Games/One', executable: 'D:/Games/One/game.exe', candidates: [] }] };
  await saveGameRecord(record, false);
  guideProgressMap.value[record.id] = { guideId: record.id, routeTitle: '原书签', blockIdx: 1, updatedAt: 'today' };
  await saveGameLink(record.id, { title: '攻略', url: 'https://example.com/guide', site: '测试站', role: 'guide' });
  let guide = singleAndGalgameGuides.value.find(item => item.id === record.id)!;
  expect(guide.rating).toBe(0);
  expect(guide.summary).toBe('短备忘');
  openGameRecordEditor(guide);
  Object.assign(state.newGame, { title: '自己的游戏名', progress: '第二章', rating: 8.5, poster: 'https://example.com/poster.jpg', notes: '下一次继续', series: '个人合集', series_order: 2 });
  await handleSaveGameRecord({
    metadata: { source: 'vndb', source_id: 'v1', title: '匹配的资料', original_title: '',
      cover: '', description: '', screenshots: [], developer: '', release_date: '', score: 0, url: 'https://vndb.org/v1' },
    background: { background: 'https://example.com/background.jpg', background_position: { x: 25, y: 60 } },
    curation: { added_tags: ['我的标签'], hidden_tags: ['PC'], developers: ['开发公司'], publishers: [],
      staff: [{ name: '制作人甲', role: '制作人' }] },
    scaling: [{ directory: 'D:/Games/One', preset: 'anime', capture: 'auto', mode: 'fullscreen', auto_start: true }],
  });
  const reloaded = await getGameRecords();
  expect(reloaded).toHaveLength(1);
  expect(reloaded[0]).toMatchObject({ id: record.id, progress: '第二章', rating: 8.5, poster: 'https://example.com/poster.jpg', notes: '下一次继续' });
  expect(reloaded[0]?.library?.installations[0]?.directory).toBe('D:/Games/One');
  expect(reloaded[0]?.library?.collection).toEqual({ name: '个人合集', order: 2 });
  expect(reloaded[0]?.library).toMatchObject({ metadata: { source_id: 'v1' },
    background: 'https://example.com/background.jpg', background_position: { x: 25, y: 60 },
    curation: { added_tags: ['我的标签'], developers: ['开发公司'], publishers: [], staff: [{ name: '制作人甲', role: '制作人' }] } });
  expect(reloaded[0]?.library?.scaling).toEqual([
    { directory: 'D:/Games/One', preset: 'anime', capture: 'auto', mode: 'fullscreen', auto_start: true },
  ]);
  guide = singleAndGalgameGuides.value.find(item => item.id === record.id)!;
  expect(guide.poster).toBe('https://example.com/poster.jpg');
  expect(guide.series).toBe('个人合集');
  expect(guideProgressMap.value[record.id]?.routeTitle).toBe('原书签');
  expect(gameLinks.value[record.id]).toHaveLength(1);
  openGameRecordEditor(guide);
  state.newGame.progress = '不能保存';
  const write = vi.spyOn(kv, 'kvSetConfirmed').mockRejectedValueOnce(new Error('disk full'));
  await expect(handleSaveGameRecord()).rejects.toThrow('disk full');
  write.mockRestore();
  expect(gameRecords.value.find(item => item.id === record.id)?.progress).toBe('第二章');
  expect(state.showAddGameModal).toBe(true);
  state.showAddGameModal = false;
});

test('saved links serialize changes, deduplicate URLs and keep the old state on failed writes', async () => {
  const { saveGameLink, removeGameLink, gameLinks, loadGameExtras } = await import('./extras');
  const kv = await import('../../services/kv');
  const link = { title: '资料', url: 'https://example.com', site: '测试站', role: 'resource' as const };
  await Promise.all([saveGameLink('links-a', link), saveGameLink('links-b', link)]);
  await saveGameLink('links-a', { ...link, title: '更新标题', url: 'https://example.com/' });
  loadGameExtras();
  expect(gameLinks.value['links-a']).toEqual([{ ...link, title: '更新标题', url: 'https://example.com/' }]);
  expect(gameLinks.value['links-b']).toHaveLength(1);
  const write = vi.spyOn(kv, 'kvSetConfirmed').mockRejectedValueOnce(new Error('disk full'));
  await expect(removeGameLink('links-a', 'https://example.com/')).rejects.toThrow('disk full');
  write.mockRestore();
  expect(gameLinks.value['links-a']).toHaveLength(1);
  await removeGameLink('links-a', 'https://example.com/');
  loadGameExtras();
  expect(gameLinks.value['links-a']).toEqual([]);
  expect(gameLinks.value['links-b']).toHaveLength(1);
});

test('cover wall sorts recent successful opens and keeps completed games at the bottom', async () => {
  const { gameRecords } = await import('../core');
  const { filteredObsidianGuides, guideSortOrder, loadGameLibraryPreferences } = await import('./library');
  const { rememberGame, recentGames } = await import('./extras');
  const { customGameStatusMap } = await import('./status');
  gameRecords.value = ['recent-a', 'recent-b', 'recent-c', 'recent-d'].map(id => ({
    id, title: id, category: 'single', status: 'playing', rating: 0, progress: '',
  }));
  const time = vi.spyOn(Date, 'now').mockReturnValueOnce(100).mockReturnValueOnce(200);
  await Promise.all([rememberGame('recent-a'), rememberGame('recent-b')]);
  time.mockRestore();
  loadGameLibraryPreferences();
  guideSortOrder.value = 'recent';
  expect(recentGames.value).toMatchObject({ 'recent-a': 100, 'recent-b': 200 });
  expect(filteredObsidianGuides.value.map(item => item.id)).toEqual(['recent-b', 'recent-a', 'recent-c', 'recent-d']);
  customGameStatusMap.value['recent-b'] = '已通关';
  expect(filteredObsidianGuides.value.map(item => item.id)).toEqual(['recent-a', 'recent-c', 'recent-d', 'recent-b']);
});

test('one game combines its guide and record without losing guide bookmarks, record links or the original markdown', async () => {
  const { gameRecords, obsidianGameGuides, state } = await import('../core');
  const { singleAndGalgameGuides, pinnedGameIds, isGamePinned, openGameRecordEditor, handleSaveGameRecord } = await import('./library');
  const { guideProgressMap, handleOpenGuideReader, guideContent } = await import('./reader');
  const { gameSourceIds } = await import('../../services/gameLibrary');
  const { saveGameLink, linksForGames, removeGameLink } = await import('./extras');
  const { getEffectiveGameStatus, customGameStatusMap, setGameStatus } = await import('./status');
  const { saveGameRecord } = await import('../../services/api');
  const guide = {
    id: 'sg-guide', title: '命运石之门', aliases: ['STEINS;GATE'], category: 'galgame', status: '在玩',
    rating: 8.8, file_path: 'steins.md', file_name: 'steins', relative_path: 'steins.md', modified: '', tags: [], genres: [],
  };
  const record = {
    id: 'sg-record', title: 'STEINS;GATE (命运石之门)', category: 'galgame' as const, status: 'playing' as const,
    rating: 9.9, progress: '第六章', notes: '下一次再选', poster: 'data:image/jpeg;base64,/9j/2Q==', poster_position: { x: 10, y: 80 },
  };
  await saveGameRecord(record, true);
  obsidianGameGuides.value = [guide];
  gameRecords.value = [record];
  guideProgressMap.value[guide.id] = { guideId: guide.id, routeTitle: '真结局', updatedAt: 'today' };
  pinnedGameIds.value = [record.id];
  customGameStatusMap.value[record.id] = '已通关';
  await Promise.all([
    saveGameLink(guide.id, { title: '攻略', url: 'https://example.com/sg', site: '站点', role: 'guide' }),
    saveGameLink(record.id, { title: '补丁', url: 'https://example.com/patch', site: '站点', role: 'resource' }),
  ]);
  expect(singleAndGalgameGuides.value).toHaveLength(1);
  const merged = singleAndGalgameGuides.value[0]!;
  expect(merged).toMatchObject({ id: guide.id, record_id: record.id, rating: 9.9, progress_note: '第六章', poster_position: { x: 10, y: 80 } });
  expect(isGamePinned(merged)).toBe(true);
  expect(getEffectiveGameStatus(merged)).toBe('已通关');
  await setGameStatus(merged, '在玩');
  expect(linksForGames(gameSourceIds(merged))).toHaveLength(2);
  await handleOpenGuideReader(merged);
  expect(guideContent.value).toBe('# 攻略内容');
  expect(guideProgressMap.value[guide.id]?.routeTitle).toBe('真结局');
  openGameRecordEditor(merged);
  state.newGame.poster_position.x = 99;
  expect(gameRecords.value[0]?.poster_position?.x).toBe(10); // 草稿取消不能改动已保存封面。
  state.newGame.guide_id = '';
  await handleSaveGameRecord();
  expect(singleAndGalgameGuides.value).toHaveLength(2);
  expect(getEffectiveGameStatus(singleAndGalgameGuides.value.find(game => game.id === record.id))).toBe('在玩');
  expect(guideProgressMap.value[guide.id]?.routeTitle).toBe('真结局');
  expect(linksForGames([record.id])).toHaveLength(1);
  await removeGameLink(gameSourceIds(merged), 'https://example.com/sg');
  expect(linksForGames(gameSourceIds(merged))).toHaveLength(1);
  obsidianGameGuides.value = [];
});
