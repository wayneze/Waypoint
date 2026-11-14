import { ref } from 'vue';
import type { GameSaveDiscovery, GameSaveSource } from '../types';
import { updateGameRecord } from '../services/api';
import { createSaveBackup, discoverGameSaves, uploadSaveBackup } from '../services/gameSaves';
import { gameRecords, showToast } from './core';

export const showGameSaveManager = ref(false);
export interface SaveRow {
  id: string; title: string; discovery: GameSaveDiscovery; selected: string[]; include: boolean;
  phase: 'pending' | 'saving' | 'uploading' | 'done' | 'failed'; error: string; backupId?: string;
}
export const gameSaveRows = ref<SaveRow[]>([]);
export const saveScan = ref({ running: false, done: 0, total: 0 });
export const saveBatch = ref({ running: false, done: 0, total: 0, stopped: false, current: '' });
let stop = false;
export function stopSaveBatch() { stop = true; }
export async function setGameSaveSources(id: string, sources: GameSaveSource[]) {
  if (!sources.length) throw new Error('请至少选择一个存档位置');
  const saved = await updateGameRecord(id, record => ({
    ...record, save_path: undefined,
    library: { ...record.library || { installations: [] }, save_sources: sources.map(source => {
      const key = (path: string) => path.replace(/\//g, '\\').replace(/\\+$/, '').toLocaleLowerCase();
      const previous = record.library?.save_sources || [];
      const same = previous.find(p => p.kind === source.kind && key(p.path) === key(source.path));
      // Selecting or relocating the same source must not invalidate its backup identity.
      const relocated = sources.length === 1 && previous.length === 1 && previous[0]?.kind === source.kind ? previous[0] : undefined;
      return { ...source, slot: same?.slot || relocated?.slot || source.slot };
    }) },
  }));
  gameRecords.value = gameRecords.value.map(record => record.id === id ? saved : record);
  return saved;
}
export async function scanLibrarySaves(automaticOnly = false) {
  if (saveScan.value.running || saveBatch.value.running) return;
  const records = gameRecords.value.filter(r => r.library?.installations.length || r.save_path || r.library?.save_sources?.length);
  saveScan.value = { running: true, done: 0, total: records.length };
  saveBatch.value = { running: false, done: 0, total: 0, stopped: false, current: '' };
  gameSaveRows.value = [];
  try {
    for (const record of records) {
      let discovery: GameSaveDiscovery;
      try { discovery = await discoverGameSaves(record.id, automaticOnly); }
      catch (e) { discovery = { candidates: [], warnings: [String(e)] }; }
      const selected = discovery.candidates.filter(c => c.automatic && c.status === 'ready').map(c => c.source.slot);
      gameSaveRows.value.push({ id: record.id, title: record.title, discovery, selected,
        include: selected.length > 0 && !discovery.warnings.length, phase: 'pending', error: '' });
      saveScan.value.done++;
    }
  } finally { saveScan.value.running = false; }
}
export async function runSaveBatch(password: string, upload: boolean, failedOnly = false) {
  if (saveBatch.value.running || saveScan.value.running) return;
  const rows = gameSaveRows.value.filter(row => row.include && row.selected.length && (!failedOnly || row.phase === 'failed'));
  if (!rows.length) throw new Error('请先勾选可备份的作品和存档位置');
  if (rows.some(row => !failedOnly || !row.backupId) && [...password].length < 10) throw new Error('请输入至少 10 个字符的备份密码');
  stop = false;
  saveBatch.value = { running: true, done: 0, total: rows.length, current: '', stopped: false };
  try {
    for (const row of rows) {
      if (stop) { saveBatch.value.stopped = true; break; }
      saveBatch.value.current = row.title; row.error = '';
      try {
        if (!failedOnly || !row.backupId) {
          row.backupId = undefined;
          row.phase = 'saving';
          const sources = row.discovery.candidates.filter(c => row.selected.includes(c.source.slot)).map(c => c.source);
          await setGameSaveSources(row.id, sources);
          row.backupId = (await createSaveBackup(row.id, password)).id;
        }
        if (upload) { row.phase = 'uploading'; await uploadSaveBackup(row.id, row.backupId!); }
        row.phase = 'done';
      } catch (e) { row.phase = 'failed'; row.error = String(e); }
      saveBatch.value.done++;
    }
  } finally {
    saveBatch.value.running = false; saveBatch.value.current = '';
    const failed = rows.filter(r => r.phase === 'failed').length;
    showToast(saveBatch.value.stopped ? '已停止后续备份' : failed ? '部分存档未备份成功' : '存档备份已完成',
      `${rows.filter(r => r.phase === 'done').length} 部成功${failed ? ` · ${failed} 部待重试` : ''}`);
  }
}
