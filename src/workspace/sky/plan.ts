import { computed, ref } from 'vue';
import { setSkyGuideItems } from '../../services/skyCollection';
import { saveCollection } from './collection';
import { beijingDay } from '../../services/cycleReset';
import { currentSkyPeriods, periodKey, skyActivityDailies, skyPeriodGoals, skyTodoGroups, type SkyPeriodGoal, type SkyTodoGroup, type SkyTodoItem } from '../../services/skyPlan';
import { progressGet, setCharacterDailyChecks, setProgressEntries } from '../../services/progress';
import type { SkyCalendarItem } from '../../services/skyApi';
import { showToast, state } from '../core';
import { dailyCharacterId, saveDailyProgress as save, skyShard, isSavingDaily, refreshDailyChecks } from './daily';
import { skyCalendarFeed, skyDailyData } from './tabs';

export const skyPlanDay = computed(() => {
  void skyShard.value;
  return beijingDay();
});
export function skyGroupsFor(characterId: string) {
  return skyTodoGroups(skyDailyData.value, skyCalendarFeed.value, skyPlanDay.value, characterId, state.eventGuides);
}
export const skyTodayGroups = computed(() => skyGroupsFor(dailyCharacterId.value));
export const skyTodayProgress = computed(() => ({
  total: skyTodayGroups.value.length,
  done: skyTodayGroups.value.filter(g => g.done).length,
}));
export const skyActivities = computed(() => skyActivityDailies(skyDailyData.value, skyCalendarFeed.value, skyPlanDay.value, dailyCharacterId.value, state.eventGuides));
export const skyPeriods = computed(() => currentSkyPeriods(skyCalendarFeed.value, skyPlanDay.value));
export function goalsFor(row: SkyCalendarItem, id = dailyCharacterId.value) {
  return skyPeriodGoals(row, id, state.seasonalGuides, state.eventGuides);
}

export function toggleSkyGroup(group: SkyTodoGroup, characterId = dailyCharacterId.value, item?: SkyTodoItem) {
  if (!characterId || !group.ready) return;
  const keys = item ? item.keys : group.items.flatMap(i => i.keys);
  const done = item ? item.done : group.done;
  const day = group.day || skyPlanDay.value;
  return save(() => setCharacterDailyChecks(characterId, day, { [group.scope]: keys }, !done));
}

export function completeSkyToday() {
  const id = dailyCharacterId.value;
  if (!id) return;
  const groups = Object.fromEntries(skyTodayGroups.value.filter(g => g.ready).map(g => [g.scope, [] as string[]]));
  for (const g of skyTodayGroups.value) groups[g.scope]?.push(...g.items.flatMap(i => i.keys));
  return save(() => setCharacterDailyChecks(id, skyPlanDay.value, groups, true));
}

export async function resetSkyDailies(characterIds: string[]) {
  if (isSavingDaily.value) throw new Error('正在保存光遇进度，请稍后重置');
  isSavingDaily.value = true;
  try {
    for (const id of characterIds) {
      const groups: Record<string, string[]> = {};
      for (const group of skyGroupsFor(id)) (groups[group.scope] ||= []).push(...group.items.flatMap(i => i.keys));
      await setCharacterDailyChecks(id, skyPlanDay.value, groups, false);
    }
    refreshDailyChecks();
  } finally { isSavingDaily.value = false; }
}

export function setActivityDaily(row: SkyCalendarItem, included: boolean) {
  const id = dailyCharacterId.value;
  if (!id) return;
  return save(() => setProgressEntries('sky.event_daily_enabled', [[periodKey(row), included]], id));
}
export function isActivityDaily(row: SkyCalendarItem) {
  return skyActivities.value.some(e => (e.id || e.name) === row.title);
}

export function saveSkyCollectionItems(scope: string, guideId: string, ids: string[], obtained: boolean) {
  const id = dailyCharacterId.value;
  if (!id) { showToast('请先选择光遇角色', '收藏进度随角色保存'); return; }
  return saveCollection(() => setSkyGuideItems(id, scope, guideId, ids, obtained));
}

export function togglePeriodGoal(goal: SkyPeriodGoal, characterId = dailyCharacterId.value) {
  if (!characterId) return;
  if (goal.guideId && goal.itemId) {
    return saveCollection(() => setSkyGuideItems(characterId, goal.scope, goal.guideId!, [goal.itemId!], !goal.done));
  }
  return save(() => setProgressEntries(goal.scope, [[goal.id, !goal.done]], characterId));
}

// 国服每周日零点复位；只记录是否献祭，不改动探索中的光翼收藏。
export const skyWeekKey = computed(() => {
  const day = new Date(`${skyPlanDay.value}T00:00:00Z`);
  day.setUTCDate(day.getUTCDate() - day.getUTCDay());
  return day.toISOString().slice(0, 10);
});
export const skyEdenDone = computed(() => progressGet('sky.weekly', `${skyWeekKey.value}/eden`, dailyCharacterId.value) === true);
export function skyWeeklyGroup(characterId: string): SkyTodoGroup {
  const done = progressGet('sky.weekly', `${skyWeekKey.value}/eden`, characterId) === true;
  return { id: 'eden', title: '伊甸献祭', hint: '国服 · 按需完成 · 周日复位', scope: 'sky.weekly', day: skyWeekKey.value, done, ready: true, items: [{ id: 'eden', title: '伊甸献祭', keys: ['eden'], done }] };
}
export function toggleSkyEden() {
  const id = dailyCharacterId.value;
  const key = `${skyWeekKey.value}/eden`;
  if (id) return save(() => setProgressEntries('sky.weekly', [[key, !skyEdenDone.value]], id));
}
export const showSkySettings = ref(false);
