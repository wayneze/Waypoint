import { ref, watch } from 'vue';
import { isGameEnabled } from '../../services/games';
import {
  applySkyBoostFeedback, fetchSkyBoostFeedback, loadSkyBoostBinding, supportsSkyBoostLink,
  loadSkyBoostCompletion, saveSkyBoostCompletion, skyBoostDay, type SkyBoostFeedback,
} from '../../services/skyBoost';
import { skyCharacters } from '../vault';
import { dailyCharacterId, isSavingDaily, refreshDailyChecks } from './daily';
import { skyDailyData, skyCalendarFeed } from './tabs';
import { skyActivityDailies } from '../../services/skyPlan';
import { saveSkyInventory, syncSkyInventoryCollection } from '../../services/skyInventory';
import { state } from '../core';

export const boostSyncing = ref(false);
export const boostResult = ref<{ characterId: string; feedback?: SkyBoostFeedback; message: string } | null>(null);

/** 跟随应用生命周期，切到总首页或关闭设置都不会停止同步。 */
export function startSkyBoostSync() {
  const sync = () => {
    if (isGameEnabled('sky') && document.visibilityState === 'visible') void syncSkyBoost();
  };
  const stop = watch([dailyCharacterId, () => skyDailyData.value?.dateStr], sync);
  const timer = setInterval(sync, 5 * 60_000);
  document.addEventListener('visibilitychange', sync);
  sync();
  return () => {
    stop();
    clearInterval(timer);
    document.removeEventListener('visibilitychange', sync);
  };
}

export async function syncSkyBoost(manual = false) {
  const characterId = dailyCharacterId.value;
  const payload = skyDailyData.value;
  const events = payload ? skyActivityDailies(payload, skyCalendarFeed.value, payload.dateStr, characterId, state.eventGuides) : [];
  if (!characterId || boostSyncing.value || isSavingDaily.value) return;
  const binding = loadSkyBoostBinding(characterId);
  if (!binding.link || (!manual && (!binding.autoSync || !supportsSkyBoostLink(binding.link)))) return;
  if (!manual && loadSkyBoostCompletion(characterId, binding)) return;
  boostSyncing.value = true;
  isSavingDaily.value = true;
  try {
    const feedback = await fetchSkyBoostFeedback(binding.link);
    if (!skyCharacters.value.some(ch => ch.id === characterId)
      || JSON.stringify(loadSkyBoostBinding(characterId)) !== JSON.stringify(binding)) {
      throw new Error('角色或订单设置已变更，本次反馈未写入');
    }
    const age = Date.now() - Date.parse(feedback.fetched_at);
    if (feedback.day !== skyBoostDay() || !Number.isFinite(age) || age < -60_000 || age > 120_000) {
      throw new Error('反馈日期不匹配，请重新同步');
    }
    let assetsMessage = '';
    if (feedback.assets) {
      await saveSkyInventory(characterId, feedback.assets, feedback.fetched_at);
      await syncSkyInventoryCollection(characterId, state.seasonalGuides, 'sky.season_wish');
      await syncSkyInventoryCollection(characterId, state.eventGuides, 'sky.event_wish');
      assetsMessage = [
        feedback.assets.outfits ? `已同步 ${feedback.assets.outfits.length} 件外观` : '',
        Object.keys(feedback.assets.balances).length ? '余额已更新' : '',
        ...feedback.assets.warnings,
      ].filter(Boolean).join('；');
    }
    let dailyMessage = '';
    let applied = false;
    try {
      applied = payload ? await applySkyBoostFeedback(payload, characterId, binding, feedback, new Date(), events) : false;
    } catch (e) { dailyMessage = e instanceof Error ? e.message : String(e); }
    refreshDailyChecks();
    const message = [dailyMessage || (applied
        ? (binding.included.event && !events.length
          ? '已同步所选项目；今日活动条目未提供，活动未勾选'
          : '已按订单包含项目同步')
        : feedback.completed ? '今日日常未加载，请刷新后同步' : '尚未完成，本地勾选未改动'), assetsMessage].filter(Boolean).join('；');
    if (applied && !dailyMessage) await saveSkyBoostCompletion(characterId, binding, feedback, message);
    boostResult.value = { characterId, feedback, message };
  } catch (e) {
    boostResult.value = { characterId, message: e instanceof Error ? e.message : String(e) };
  } finally {
    boostSyncing.value = false;
    isSavingDaily.value = false;
  }
}
