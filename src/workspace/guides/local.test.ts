import { expect, test, vi } from 'vitest';
import { ref } from 'vue';
import type { GameRecord } from '../../types';
const storage = new Map<string, string>();
vi.stubGlobal('localStorage', { getItem: (k: string) => storage.get(k) ?? null, setItem: (k: string, v: string) => storage.set(k, v) });
vi.mock('../core', () => ({ gameRecords: ref([]), obsidianGameGuides: ref([]) }));
vi.mock('./library', async () => {
  const { computed } = await import('vue');
  const { gameRecords, obsidianGameGuides } = await import('../core');
  const { buildGameLibrary } = await import('../../services/gameLibrary');
  const libraryGuides = computed(() => obsidianGameGuides.value);
  return { libraryGuides, singleAndGalgameGuides: computed(() => buildGameLibrary(libraryGuides.value, gameRecords.value)) };
});
vi.mock('../../services/localGames', async importOriginal => ({
  ...await importOriginal<typeof import('../../services/localGames')>(),
  searchGameMetadata: vi.fn(),
  loadGameMetadata: vi.fn(),
}));

test('import and rescan are idempotent, metadata and personal edits keep their own values', async () => {
  const { gameRecords } = await import('../core');
  const { importScannedGame, applyMetadata } = await import('./local');
  const { saveGameRecord, getGameRecords } = await import('../../services/api');
  const { buildGameLibrary } = await import('../../services/gameLibrary');
  const original: GameRecord = { id: 'existing', title: '我的标题', category: 'galgame', status: 'playing', progress: '第六章', notes: '下次继续', rating: 9.5, poster: 'https://example.org/personal.jpg' };
  await saveGameRecord(original, true); gameRecords.value = [original];
  const game = { title: 'STEINS;GATE', installation: { root: '\\\\nas\\games', directory: '\\\\nas\\games\\SG', executable: '\\\\nas\\games\\SG\\cn.exe', candidates: ['\\\\nas\\games\\SG\\cn.exe'] } };
  const id = await importScannedGame(game, original.id, 'galgame');
  await importScannedGame({ ...game, installation: { ...game.installation, directory: '//NAS/games/sg/', executable: '\\\\nas\\games\\SG\\jp.exe' } }, '', 'galgame');
  await expect(importScannedGame(game, 'different-work', 'galgame')).rejects.toThrow('已关联');
  const metadata = { source: 'vndb' as const, source_id: 'v2002', title: 'STEINS;GATE', original_title: 'シュタインズ・ゲート', description: '故事简介', cover: 'https://example.org/source.jpg', screenshots: [], developer: 'MAGES', release_date: '2009-10-15', score: 9, url: 'https://vndb.org/v2002' };
  await applyMetadata(id, metadata);
  const records = await getGameRecords();
  expect(records).toHaveLength(1);
  expect(records[0]).toMatchObject(original);
  expect(records[0]?.library?.installations).toHaveLength(1);
  expect(records[0]?.library?.installations[0]?.executable).toBe(game.installation.executable);
  expect(buildGameLibrary([], records)[0]).toMatchObject({ title: original.title, poster: original.poster, rating: 9.5, summary: '故事简介', developer: 'MAGES' });
  const { updateGameRecord } = await import('../../services/api');
  await Promise.all([
    applyMetadata(id, { ...metadata, description: '新的介绍' }),
    updateGameRecord(id, current => ({ ...current, progress: '第七章' })),
  ]);
  const concurrent = (await getGameRecords())[0]!;
  expect(concurrent.progress).toBe('第七章');
  expect(concurrent.library?.metadata?.description).toBe('新的介绍');
  await applyMetadata(id, metadata, { poster: 'https://example.org/chosen-online.jpg', poster_position: { x: 25, y: 60 } });
  await applyMetadata(id, { ...metadata, description: '刷新资料' });
  expect((await getGameRecords())[0]).toMatchObject({
    poster: 'https://example.org/chosen-online.jpg', poster_position: { x: 25, y: 60 }, progress: '第七章',
  });
});

test('Bangumi batch matches aliases and subtitles, keeps ambiguous candidates, isolates failures and preserves media', async () => {
  const { gameRecords } = await import('../core');
  const { saveGameRecord, getGameRecords } = await import('../../services/api');
  const { enrichImportedGames, metadataJob, applyMetadata, ensureGameMedia, updateLibraryDetails } = await import('./local');
  const { confidentMetadata, searchGameMetadata, loadGameMetadata } = await import('../../services/localGames');
  const base = { source: 'bangumi' as const, source_id:'3154', title:'命运石之门', original_title:'STEINS;GATE', aliases:['石头门'], description:'中文介绍', cover:'', screenshots:[], developer:'MAGES', release_date:'2009-10-15', score:9, url:'https://bgm.tv/subject/3154' };
  const sequel = { ...base, source_id:'0', title:'命运石之门0', original_title:'STEINS;GATE 0', aliases:[] };
  const side = { ...base, source_id:'63973', title:'线形拘束', original_title:'STEINS;GATE 線形拘束のフェノグラム', aliases:[] };
  expect(confidentMetadata(['石头门'], [base,sequel])).toBe(base);
  expect(confidentMetadata(['STEINS;GATE 0'], [base,sequel])).toBe(sequel);
  expect(confidentMetadata(['STEINS;GATE'], [sequel])).toBeUndefined();
  expect(confidentMetadata(['線形拘束のフェノグラム'], [base,side])).toBe(side);
  expect(confidentMetadata(['石头门'], [base,{ ...base, source_id:'remake' }])).toBeUndefined();
  const records = ['石头门','歧义作品','连接失败'].map((title,index) => ({ id:`batch-${index}`,title,category:'galgame' as const,status:'playing' as const,rating:9.9,progress:'第七章',library:{ installations:[] } }));
  for (const r of records) await saveGameRecord(r,true);
  gameRecords.value = records;
  vi.mocked(searchGameMetadata).mockImplementation(async (_source,title) => {
    if (title === '连接失败') throw new Error('offline');
    if (_source === 'vndb') return [];
    return title === '石头门' ? [base,sequel] : [{ ...base,title:'歧义作品' },{ ...base,title:'歧义作品',source_id:'remake' }];
  });
  vi.mocked(loadGameMetadata).mockResolvedValue({ ...base,enriched:true,tags:['科幻'],sources:[{ source:'vndb',source_id:'v2002',url:'https://vndb.org/v2002' }] });
  await enrichImportedGames(records);
  expect(vi.mocked(searchGameMetadata).mock.calls[0]?.[0]).toBe('bangumi');
  expect(metadataJob.value).toMatchObject({ done:3,matched:1,pending:2,running:false });
  expect(gameRecords.value[0]).toMatchObject({ progress:'第七章',rating:9.9,library:{ metadata:{ tags:['科幻'] } } });
  expect(gameRecords.value[1]?.library?.matching?.candidates).toHaveLength(2);
  expect((await getGameRecords()).find(r => r.id === 'batch-2')?.library?.matching?.error).toContain('offline');
  await applyMetadata('batch-0',{ ...base,screenshots:['https://example.org/a.jpg'],videos:[{ title:'PV',url:'https://example.org/pv.mp4',thumbnail:'',page_url:'' }] });
  await applyMetadata('batch-0',{ ...base,warnings:['media offline'] });
  expect(gameRecords.value[0]?.library?.metadata?.screenshots).toEqual(['https://example.org/a.jpg']);
  await updateLibraryDetails('batch-0', lib => ({...lib,background:'',background_position:{x:25,y:70}}));
  vi.mocked(loadGameMetadata).mockResolvedValue({ ...base,media_version:3,videos:[{title:'PV',url:'https://example.org/new.mp4',thumbnail:'',page_url:''}] });
  vi.mocked(loadGameMetadata).mockClear();
  await Promise.all([ensureGameMedia('batch-0'), ensureGameMedia('batch-0')]);
  await ensureGameMedia('batch-0');
  expect(loadGameMetadata).toHaveBeenCalledTimes(1);
  expect(gameRecords.value[0]?.library).toMatchObject({ background:'',background_position:{x:25,y:70},metadata:{media_version:3,videos:[{url:'https://example.org/new.mp4'}]} });
  await applyMetadata('batch-0',sequel);
  expect(gameRecords.value[0]?.library?.metadata?.screenshots).toEqual([]);
  expect(gameRecords.value[0]?.library).toMatchObject({background:null,background_position:null});
});

test('directory imports target guide-only works, preserve their identity and reuse the same work on rescan', async () => {
  const { kvSetConfirmed } = await import('../../services/kv');
  await kvSetConfirmed('waypoint_game_records', '[]');
  const { gameRecords, obsidianGameGuides } = await import('../core');
  const { singleAndGalgameGuides } = await import('./library');
  const { suggestedLibraryGame, importScannedGame, enrichImportedGames } = await import('./local');
  const { searchGameMetadata, loadGameMetadata } = await import('../../services/localGames');
  const { getGameRecords } = await import('../../services/api');
  gameRecords.value = [];
  const guide = {
    id: 'guide-only', title: '命运石之门', original_title: 'STEINS;GATE', aliases: ['石头门'],
    category: 'galgame', status: '已通关', rating: 8, rating_personal: 9,
    progress_note: '已到真结局', file_path: 'notes/sg.md', file_name: 'sg.md', relative_path: 'sg.md',
    modified: '', tags: [], genres: [], poster: 'https://example.org/cover.jpg',
    links: { bangumi: 'https://bgm.tv/subject/3154', steam: 'https://store.steampowered.com/app/412830/' },
  };
  obsidianGameGuides.value = [guide, { ...guide, id: 'sequel', title: '命运石之门0', original_title: 'STEINS;GATE 0', aliases: [], links: {} }];
  const game = { title: '石头门', installation: { root: 'D:/Games', directory: 'D:/Games/SG', executable: 'D:/Games/SG/game.exe', candidates: ['D:/Games/SG/game.exe'] } };
  expect(suggestedLibraryGame(game)?.id).toBe(guide.id);
  expect(suggestedLibraryGame({ ...game, title: 'STEINS;GATE 0' })?.id).toBe('sequel');
  expect(suggestedLibraryGame({ ...game, title: '文件夹昵称' })).toBeUndefined();
  expect(suggestedLibraryGame({ ...game, title: '文件夹昵称', steam_id: '412830' })?.id).toBe(guide.id);
  const id = await importScannedGame({ ...game, title: '手动选择的文件夹昵称' }, guide.id, 'single');
  expect((await getGameRecords())[0]).toMatchObject({
    id, title: guide.title, guide_id: guide.id, category: 'galgame', status: 'completed', progress: guide.progress_note, rating: 9,
  });
  expect(singleAndGalgameGuides.value).toHaveLength(2);
  expect(singleAndGalgameGuides.value[0]).toMatchObject({ id: guide.id, record_id: id, file_path: guide.file_path, poster: guide.poster });
  expect(suggestedLibraryGame(game)?.record_id).toBe(id);
  await importScannedGame(game, guide.id, 'galgame');
  expect((await getGameRecords())).toHaveLength(1);
  expect((await getGameRecords())[0]?.library?.installations).toHaveLength(1);
  await expect(importScannedGame(game, 'sequel', 'galgame')).rejects.toThrow('已关联');
  vi.mocked(searchGameMetadata).mockClear();
  vi.mocked(loadGameMetadata).mockResolvedValue({
    source: 'bangumi', source_id: '3154', title: guide.title, original_title: 'STEINS;GATE',
    description: '', cover: '', screenshots: [], developer: '', release_date: '', score: 0, url: guide.links.bangumi,
  });
  await enrichImportedGames([{ id, title: '文件夹昵称' }]);
  expect(searchGameMetadata).not.toHaveBeenCalled();
  expect(loadGameMetadata).toHaveBeenLastCalledWith({ source: 'bangumi', source_id: '3154' });
  obsidianGameGuides.value.push({ ...guide, id: 'ambiguous' });
  expect(suggestedLibraryGame({ ...game, installation: { ...game.installation, directory: 'D:/Other' } })).toBeUndefined();
  obsidianGameGuides.value = [];
});

test('guide-only works accept metadata without an installation and keep the original guide identity', async () => {
  const { gameRecords, obsidianGameGuides } = await import('../core');
  const { kvSetConfirmed } = await import('../../services/kv');
  const { getGameRecords } = await import('../../services/api');
  const { applyMetadata, detailGame, detailGameId, updateLibraryDetails } = await import('./local');
  const { singleAndGalgameGuides } = await import('./library');
  await kvSetConfirmed('waypoint_game_records', '[]'); gameRecords.value = [];
  const guide = {
    id:'subahibi-guide',title:'美好的每一天～不连续的存在～',category:'galgame',status:'在玩',
    progress_note:'第二章',rating:8.8,rating_personal:9.5,personal_notes:'个人备忘',
    poster:'https://example.org/guide.jpg',file_path:'notes/subahibi.md',file_name:'subahibi.md',
    relative_path:'subahibi.md',modified:'',aliases:[],tags:[],genres:[],
  };
  obsidianGameGuides.value = [guide]; detailGameId.value = guide.id;
  const metadata = {
    source:'vndb' as const,source_id:'v3144',title:'美好的每一天',original_title:'素晴らしき日々',
    description:'作品介绍',cover:'https://example.org/source.jpg',screenshots:[],developer:'KeroQ',
    release_date:'2010-03-26',score:9,url:'https://vndb.org/v3144',media_version:3,
  };
  const [,id] = await Promise.all([
    updateLibraryDetails(guide.id, lib => ({...lib,background:'https://example.org/chosen.jpg'})),
    applyMetadata(guide.id, metadata),
  ]);
  expect(await getGameRecords()).toMatchObject([{
    id,guide_id:guide.id,title:guide.title,progress:'第二章',notes:'个人备忘',rating:9.5,status:'playing',
    library:{installations:[],metadata:{source_id:'v3144'},background:'https://example.org/chosen.jpg'},
  }]);
  expect(singleAndGalgameGuides.value).toHaveLength(1);
  expect(detailGame.value).toMatchObject({id:guide.id,record_id:id,file_path:guide.file_path,summary:'作品介绍'});
  await applyMetadata(guide.id, {...metadata,description:'刷新资料'});
  expect(await getGameRecords()).toHaveLength(1);
  expect(detailGame.value?.summary).toBe('刷新资料');
  detailGameId.value = null; obsidianGameGuides.value = [];
});
