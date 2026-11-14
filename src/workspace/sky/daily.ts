// 光遇今日：落石预测、每日任务、季蜡与活动日常打卡。
import { ref, computed } from 'vue';
import {
  syncSkyDailyChecks,
  SKY_DAILY_SCOPES,
} from '../../services/skyDailyService';
import { kvGet, kvSet } from '../../services/kv';
import { type ServerMode, type ShardInfo, calculateSkyShard } from '../../services/skyShards';
import { activeCharacter, showToast } from '../core';
import { activeSkyCharacter, skyCharacters } from '../vault';
import { claimDailyChecks, legacyDailyCheckCount } from '../../services/progress';
import { skyDailyData } from './tabs';

// 光遇落石状态与服务器模式 (纯离线官方数学算法预测)
export const skyServerMode = ref<ServerMode>(
  (kvGet('waypoint_sky_server_mode') as ServerMode) || 'netease'
);
export const skyShard = ref<ShardInfo>(calculateSkyShard(new Date(), skyServerMode.value));

export function setSkyServerMode(mode: ServerMode) {
  skyServerMode.value = mode;
  kvSet('waypoint_sky_server_mode', mode);
  skyShard.value = calculateSkyShard(new Date(), mode);
}

export const dailyCharacterId = computed({
  get: () => activeSkyCharacter.value?.id || '',
  set: (id: string) => { if (skyCharacters.value.some(ch => ch.id === id)) activeCharacter.value = id; },
});
export const isSavingDaily = ref(false);
export const dailyCanEdit = computed(() => !!dailyCharacterId.value && !isSavingDaily.value);
export const dailyLegacyCount = ref(legacyDailyCheckCount(SKY_DAILY_SCOPES));

export function refreshDailyChecks() {
  if (skyDailyData.value) syncSkyDailyChecks(skyDailyData.value, dailyCharacterId.value);
}

export async function saveDailyProgress(run: () => Promise<void>) {
  if (isSavingDaily.value) return;
  isSavingDaily.value = true;
  try {
    await run();
    refreshDailyChecks();
    return true;
  } catch (e) {
    showToast('日常保存失败，原记录已保留', String(e));
  } finally { isSavingDaily.value = false; }
}

export function claimDailyLegacy() {
  const id = dailyCharacterId.value;
  if (!id) return;
  return saveDailyProgress(async () => {
    await claimDailyChecks(SKY_DAILY_SCOPES, id);
    dailyLegacyCount.value = legacyDailyCheckCount(SKY_DAILY_SCOPES);
  });
}
