import { computed, ref } from 'vue';
import type { GameLibraryDetails, GameMetadata, GameRecord, ObsidianGameGuide } from '../../types';
import { saveGameRecord, updateGameRecord } from '../../services/api';
import { kvGetJson, kvSetConfirmed } from '../../services/kv';
import { confidentMetadata, installedRecord, installationKey, loadGameMetadata, rankMetadata, searchGameMetadata, type ScannedGame } from '../../services/localGames';
import { matchingGuides, sourceIdentity, withGameMetadata } from '../../services/gameLibrary';
import { gameRecords } from '../core';
import { libraryGuides, singleAndGalgameGuides } from './library';
import { getEffectiveGameStatus } from './status';
import type { ExploreRole } from '../../services/exploreSites';
import { GAME_MEDIA_VERSION } from '../../services/gameMedia';

export const showLibraryImport = ref(false);
export const libraryImportTargetId = ref<string | null>(null);
export const showMetadataBatch = ref(false);
export const metadataSource = ref<GameMetadata['source']>('bangumi');
export const detailGameId = ref<string | null>(null);
export const detailResourceRole = ref<ExploreRole | null>(null);
export const detailTab = ref<'play' | 'resources' | 'metadata' | 'files' | 'prices'>('play');
export const detailGame = computed(() => singleAndGalgameGuides.value.find(g => g.id === detailGameId.value || g.record_id === detailGameId.value));
export const libraryRoots = ref<string[]>([]);
export const metadataJob = ref({ running: false, done: 0, total: 0, matched: 0, pending: 0, error: '' });
export const metadataBusyIds = ref<string[]>([]);
export const metadataErrors = computed(() => Object.fromEntries(gameRecords.value.filter(r => r.library?.matching?.error).map(r => [r.id, r.library!.matching!.error])));
let stopMetadata = false;

export function openGameDetail(game: ObsidianGameGuide, role: ExploreRole | null = null) {
  if (role) detailTab.value = 'resources';
  detailResourceRole.value = role;
  detailGameId.value = game.record_id || game.id;
}
export function openLibraryImport(targetId: string | null = null) {
  libraryImportTargetId.value = targetId;
  libraryRoots.value = kvGetJson<string[]>('waypoint_library_roots', []);
  showLibraryImport.value = true;
}
export async function saveLibraryRoots(roots: string[]) {
  await kvSetConfirmed('waypoint_library_roots', JSON.stringify(roots));
  libraryRoots.value = roots;
}
export async function updateLibraryDetails(id: string, update: (current: GameLibraryDetails) => GameLibraryDetails) {
  return updateLibraryRecord(id, current => ({ ...current, library: update(current.library || { installations: [] }) }));
}
export function suggestedLibraryGame(game: ScannedGame) {
  const installed = installedRecord(game, gameRecords.value);
  if (installed) return singleAndGalgameGuides.value.find(g => g.record_id === installed.id);
  const matches = matchingGuides({
    title: game.title,
    library: { installations: [{ ...game.installation, steam_id: game.steam_id || game.installation.steam_id }] },
  }, singleAndGalgameGuides.value);
  return matches.length === 1 ? matches[0] : undefined;
}
function newLibraryRecord(title: string, category: GameRecord['category'], guide?: ObsidianGameGuide): GameRecord {
  const status = getEffectiveGameStatus(guide);
  return {
    id: `g_${crypto.randomUUID()}`, title: guide?.title || title.trim(),
    category: guide?.category === 'single' || guide?.category === 'steam' ? guide.category : guide ? 'galgame' : category,
    status: status === '已通关' ? 'completed' : ['在玩', '游玩中'].includes(status) ? 'playing' : 'backlog',
    progress: guide?.progress_note || '', rating: guide?.rating_personal || 0, notes: guide?.personal_notes,
    guide_id: guide?.file_path ? guide.id : undefined,
  };
}
const libraryRecordWrites = new Map<string, Promise<string>>();
function updateLibraryRecord(gameId: string, update: (current: GameRecord) => GameRecord) {
  const key = singleAndGalgameGuides.value.find(g => g.id === gameId || g.record_id === gameId)?.id || gameId;
  // A guide's first personal setting and its metadata may finish together; create one attached record.
  const write = (libraryRecordWrites.get(key) || Promise.resolve()).catch(() => {}).then(async () => {
    const guide = singleAndGalgameGuides.value.find(g => g.id === key || g.record_id === key);
    const existing = gameRecords.value.find(r => r.id === (guide?.record_id || key));
    if (!existing && !guide?.file_path) throw new Error('要保存的作品已不存在，请重新打开作品。');
    const record = existing ? await updateGameRecord(existing.id, update)
      : update(newLibraryRecord(guide!.title, 'galgame', guide));
    if (!existing) await saveGameRecord(record, true);
    gameRecords.value = existing ? gameRecords.value.map(item => item.id === record.id ? record : item) : [...gameRecords.value, record];
    return record.id;
  });
  libraryRecordWrites.set(key, write);
  const clear = () => { if (libraryRecordWrites.get(key) === write) libraryRecordWrites.delete(key); };
  void write.then(clear, clear);
  return write;
}
export async function importScannedGame(game: ScannedGame, targetId: string, category: GameRecord['category']) {
  const target = singleAndGalgameGuides.value.find(g => g.id === targetId || g.record_id === targetId);
  const targetRecordId = target?.record_id || targetId;
  const installed = installedRecord(game, gameRecords.value);
  if (targetId && installed && installed.id !== targetRecordId) throw new Error(`这个目录已关联《${installed.title}》，不能重复关联到另一部作品。`);
  const existing = installed || gameRecords.value.find(r => r.id === targetRecordId);
  if (targetId && !existing && !target?.file_path) throw new Error('要关联的作品已不存在，请重新选择作品。');
  const record: GameRecord = existing ? JSON.parse(JSON.stringify(existing)) : newLibraryRecord(game.title, category, target);
  const attach = (library: GameLibraryDetails): GameLibraryDetails => {
    const installations = [...library.installations];
    const index = installations.findIndex(i => installationKey(i.directory) === installationKey(game.installation.directory));
    // 重扫保留用户选择的程序。
    if (index >= 0) installations[index] = { ...game.installation, executable: installations[index]!.executable };
    else installations.push({ ...game.installation, candidates: [...game.installation.candidates] });
    return { ...library, installations };
  };
  if (existing) {
    const updated = await updateGameRecord(record.id, latest => ({
      ...latest, guide_id: latest.guide_id ?? (target?.file_path ? target.id : undefined),
      library: attach(latest.library || { installations: [] }),
    }));
    gameRecords.value = gameRecords.value.map(r => r.id === updated.id ? updated : r);
  }
  else {
    record.library = attach({ installations: [] });
    await saveGameRecord(record, true);
    gameRecords.value = [...gameRecords.value, record];
  }
  return record.id;
}
export async function applyMetadata(gameId: string, metadata: GameMetadata, cover?: Pick<GameRecord, 'poster' | 'poster_position'>) {
  // 请求结束后读取最新档案，避免覆盖同时编辑的个人进度。
  return updateLibraryRecord(gameId, latest => ({ ...latest, ...cover,
    library: withGameMetadata(latest.library || { installations: [] }, metadata) }));
}
export async function selectGameMetadata(id: string, candidate: GameMetadata) {
  if (metadataBusyIds.value.includes(id)) throw new Error('这部作品正在批量补全，请等待当前结果保存后再操作。');
  metadataBusyIds.value.push(id);
  try { await applyMetadata(id, await loadGameMetadata(candidate)); }
  finally { metadataBusyIds.value = metadataBusyIds.value.filter(value => value !== id); }
}
const mediaAttempts = new Set<string>();
export async function ensureGameMedia(id: string) {
  const meta = gameRecords.value.find(r => r.id === id)?.library?.metadata;
  if (!meta || (meta.media_version || 0) >= GAME_MEDIA_VERSION || metadataBusyIds.value.includes(id)) return;
  const key = `${id}:${meta.source}:${meta.source_id}`;
  if (mediaAttempts.has(key)) return;
  mediaAttempts.add(key);
  await selectGameMetadata(id, meta);
}
export function cancelMetadataQueue() { stopMetadata = true; }
export async function searchRecordMetadata(id: string, source: GameMetadata['source'], query: string) {
  const candidates = await searchGameMetadata(source, query);
  const ranked = rankMetadata([query], candidates).map(item => item.metadata);
  await updateLibraryDetails(id, current => ({ ...current, matching: { query, candidates: ranked, error: ranked.length ? '请选择正确作品，也可以修改名称重新搜索。' : '没有找到候选，试试原名、别名或来源 ID。' } }));
}
export async function enrichImportedGames(games: { id: string; title: string; steam_id?: string }[], source = metadataSource.value, refresh = false) {
  if (metadataJob.value.running) return;
  stopMetadata = false;
  const queue = [...new Map(games.map(g => [g.id, g])).values()];
  metadataJob.value = { running: true, done: 0, total: queue.length, matched: 0, pending: 0, error: '' };
  async function worker() {
    while (queue.length && !stopMetadata) {
      const game = queue.shift()!;
      const record = gameRecords.value.find(r => r.id === game.id);
      if (!record || metadataBusyIds.value.includes(game.id)) { metadataJob.value.done++; continue; }
      const old = record.library?.metadata;
      if (!refresh && old?.enriched && (old.media_version || 0) >= GAME_MEDIA_VERSION && old.source === source) { metadataJob.value.done++; continue; }
      metadataBusyIds.value.push(game.id);
      let candidates: GameMetadata[] = [];
      try {
        let match: Pick<GameMetadata, 'source' | 'source_id'> | undefined = old?.source === source ? old : old?.sources?.find(s => s.source === source);
        if (source === 'steam' && game.steam_id) match = { source, source_id: game.steam_id };
        if (!match && record.guide_id) {
          const guide = libraryGuides.value.find(g => g.id === record.guide_id);
          const identity = Object.values(guide?.links || {}).map(sourceIdentity).find(id => id?.[0] === source);
          if (identity) match = { source: identity[0], source_id: identity[1] };
        }
        if (!match) {
          const names = [game.title, record.title, old?.original_title || '', old?.title || ''].filter(Boolean);
          let preferredError = '';
          try { candidates = await searchGameMetadata(source, game.title); }
          catch (e) { if (source !== 'bangumi') throw e; preferredError = String(e); }
          match = old ? candidates.find(c => c.sources?.some(s => s.source === old.source && s.source_id === old.source_id)) : undefined;
          match ||= confidentMetadata(names, candidates);
          // Bangumi 匿名搜索不包含部分 Galgame；备用来源仍需唯一作品身份，不能取搜索首项。
          if (!match && source === 'bangumi') {
            const alternatives = await searchGameMetadata('vndb', game.title).catch(() => [] as GameMetadata[]);
            const vn = confidentMetadata(names, alternatives);
            if (vn) match = vn.sources?.find(s => s.source === 'bangumi') || vn;
            candidates.push(...alternatives);
          }
          candidates = rankMetadata(names, candidates).map(item => item.metadata);
          if (!match && !candidates.length && preferredError) throw new Error(preferredError);
        }
        if (stopMetadata) break;
        if (match) {
          const full = await loadGameMetadata(match);
          if (stopMetadata) break;
          await applyMetadata(game.id, full);
          metadataJob.value.matched++;
        } else {
          await updateLibraryDetails(game.id, current => ({ ...current, matching: {
            query: game.title, candidates, error: candidates.length ? '候选仍有歧义，请在此确认作品。' : '没有找到候选，可修改名称重试。',
          } }));
          metadataJob.value.pending++;
        }
      } catch (e) {
        if (!stopMetadata) {
          try { await updateLibraryDetails(game.id, current => ({ ...current, matching: { query: game.title, candidates, error: String(e) } })); }
          catch (saveError) { metadataJob.value.error = `匹配结果保存失败：${String(saveError)}`; }
          metadataJob.value.pending++;
        }
      } finally {
        metadataBusyIds.value = metadataBusyIds.value.filter(id => id !== game.id);
        metadataJob.value.done++;
      }
    }
  }
  try {
    await Promise.all([worker(), worker()]);
  } finally { metadataJob.value.running = false; }
}
