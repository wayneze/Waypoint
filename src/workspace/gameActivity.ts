import { ref } from 'vue';
import { getGameRecords, isTauri } from '../services/api';
import { duration, getLocalPlaytime, getSteamActivity, importSteamGames, refreshSteamAchievements, resolveSteamTitle, type LocalPlaytime, type SteamActivity } from '../services/gameActivity';
import { steamLibrary, syncSteamAccount } from './vault';
import { gameRecords, showToast } from './core';

export const showSteamImport = ref(false);
export const steamImportTargetId = ref<string | null>(null);
export function openSteamImport(targetId: string | null = null) {
  steamImportTargetId.value = targetId;
  showSteamImport.value = true;
}
export const launchingGames = ref<string[]>([]);
export const steamActivity = ref<SteamActivity>({ links: [], accounts: [], installed: [], titles: {} });
export const localPlaytime = ref<LocalPlaytime[]>([]);
export const activityError = ref('');
export async function loadGameActivity() {
  if (!isTauri()) return;
  try {
    const [steam, local] = await Promise.all([getSteamActivity(), getLocalPlaytime()]);
    steamActivity.value = steam; localPlaytime.value = local; activityError.value = '';
  } catch (e) { activityError.value = String(e); }
}
export async function loadLocalPlaytime() {
  try { localPlaytime.value = await getLocalPlaytime(); }
  catch (e) { activityError.value = String(e); }
}

export const steamImportAccountId = ref('');
export const steamBatch = ref({ running: false, accountId: '', appids: [] as number[], total: 0, done: 0, synced: 0, imported: 0, current: '', errors: [] as string[], stopped: false });
let stopBatch = false;
export function stopSteamBatch() { stopBatch = true; }
export async function runSteamBatch(accountId: string, choices: { appid: number; record_id?: string; title?: string; guide_id?: string }[], mode: 'import' | 'sync') {
  if (steamBatch.value.running) throw new Error('已有 Steam 任务正在执行');
  stopBatch = false;
  const appids = [...new Set(choices.map(c => c.appid))];
  steamBatch.value = { running: true, accountId, appids, total: appids.length, done: 0, synced: 0, imported: 0, current: '保存书库', errors: [], stopped: false };
  try {
    if (mode === 'import') {
      await importSteamGames(accountId, choices);
      steamBatch.value.imported = choices.length;
      gameRecords.value = await getGameRecords();
      await loadGameActivity();
    }
    steamBatch.value.current = '同步官方时长';
    try { await syncSteamAccount(accountId); }
    catch (e) { steamBatch.value.errors.push(`时长更新失败：${String(e)}`); }
    await loadGameActivity();
    for (const appid of appids) {
      if (stopBatch) { steamBatch.value.stopped = true; break; }
      const name = steamActivity.value.titles[appid]?.title || steamLibrary.value.find(g => g.appid === appid)?.name || String(appid);
      steamBatch.value.current = name;
      try {
        const cache = await refreshSteamAchievements(accountId, appid);
        const account = steamActivity.value.accounts.find(a => a.account_id === accountId);
        if (account) account.achievements[appid] = cache;
        if (cache.error) steamBatch.value.errors.push(`${name}：${cache.error}`);
        else steamBatch.value.synced++;
      } catch (e) { steamBatch.value.errors.push(`${name}：${String(e)}`); }
      steamBatch.value.done++;
    }
  } catch (e) { steamBatch.value.errors.push(String(e)); }
  finally {
    steamBatch.value.running = false; steamBatch.value.current = '';
    showToast(steamBatch.value.errors.length ? 'Steam 任务有未完成项' : steamBatch.value.stopped ? 'Steam 后续同步已停止' : 'Steam 任务已完成',
      `入库 / 关联 ${steamBatch.value.imported} 部 · 成就成功 ${steamBatch.value.synced} / ${steamBatch.value.total}`);
  }
}

export const steamNames = ref({ running: false, total: 0, done: 0, failed: 0, stopped: false });
let stopNames = false;
export function stopSteamNames() { stopNames = true; }
export async function fillSteamNames(appids: number[], retry = false) {
  if (steamNames.value.running) return;
  const queue = [...new Set(appids)].filter(id => retry || !steamActivity.value.titles[id]?.checked_at);
  if (!queue.length) return;
  stopNames = false;
  steamNames.value = { running: true, total: queue.length, done: 0, failed: 0, stopped: false };
  async function worker() {
    while (queue.length && !stopNames) {
      const appid = queue.shift()!;
      try {
        const title = await resolveSteamTitle(appid, retry);
        steamActivity.value.titles[appid] = title;
        if (title.error) steamNames.value.failed++;
      } catch { steamNames.value.failed++; }
      steamNames.value.done++;
    }
  }
  try { await Promise.all([worker(), worker()]); }
  finally { steamNames.value.running = false; steamNames.value.stopped = stopNames && queue.length > 0; }
}
export function activitySummary(recordId: string): string {
  const link = steamActivity.value.links.find(l => l.record_id === recordId);
  const local = localPlaytime.value.find(p => p.record_id === recordId);
  if (local?.active) return `${local.active.tracking_error ? '计时暂停' : '正在计时'} · ${duration(local.active.session.seconds)}`;
  if (link) {
    const accounts = steamActivity.value.accounts.filter(a => a.updated_at && steamLibrary.value.some(g => g.vault_item_id === a.account_id && g.appid === link.appid));
    const account = accounts[0];
    if (account) {
      const game = steamLibrary.value.find(g => g.vault_item_id === account.account_id && g.appid === link.appid);
      const achievements = account.achievements[link.appid];
      return [`Steam ${duration((game?.playtime_forever || 0) * 60)}`,
        achievements?.updated_at && achievements.status !== 'unsupported' ? `成就 ${achievements.items.filter(a => a.unlocked).length}/${achievements.items.length}${achievements.error ? ' · 旧缓存' : ''}` : '',
        accounts.length > 1 ? '多账号' : ''].filter(Boolean).join(' · ');
    }
    return '已关联 Steam · 等待同步';
  }
  return local ? `本地 ${duration(local.total_seconds)}` : '';
}
