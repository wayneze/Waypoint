// 剑网3 日历、诛恶、世界首领与加号录入。
import { ref, reactive, computed, watch, shallowRef } from 'vue';
import {
  type Jx3CalendarSlotKey,
  type Jx3DailyCalendar,
  type Jx3MapEvent,
  type Jx3MapFeed,
  type Jx3MapRefreshInfo,
  type Jx3WorldBossStoredWeek,
  type Jx3WorldBossWeek,
  type TaskItem,
} from '../../types';
import { updateTaskScheduleTime, updateTaskTitle } from '../../services/api';
import { getJx3Celebs, getJx3DailyCalendar, searchJx3BoxAchievements } from '../../services/jx3Api';
import { parseAchievementSearchPayload } from '../../services/jx3AdventureCatalog';
import { kvGet, kvSet } from '../../services/kv';
import { reportSource } from '../../services/fetchCache';
import { isGameEnabled } from '../../services/games';
import { beijingDay } from '../../services/cycleReset';
import {
  JX3_CALENDAR_SLOTS,
  jx3CalendarCategoryImage,
  type Jx3CatalogHit,
  calculateJx3WorldBossWeek,
  calculateJx3ZhueClock,
  daysUntilJx3Weekday,
  extractJx3TaskScheduleTime,
  filterJx3ZhueEvents,
  findJx3TimedBossEvent,
  formatJx3Date,
  loadJx3WorldBossWeek,
  resolveJx3TaskVisual,
  saveJx3WorldBossWeek,
  searchJx3Catalog,
  startOfJx3Week,
  upsertJx3WorldBossDay,
} from '../../services/jx3Schedule';
import { JX3_WORLD_BOSS_SCHEDULE } from '../../services/jx3CollectData';
import {
  activeCharacter,
  allAccountTasks,
  characterList,
  newTaskId,
  pushCharacterTask,
  resetRuleFor,
  showToast,
  syncActiveCharacterState,
} from '../core';
import { jx3BoxDungeons, jx3Collect, jx3Pendants, jx3SetTokens } from './collect';
const progressModule = shallowRef<typeof import('./progress')>();
watch(() => isGameEnabled('jx3'), enabled => {
  if (enabled) void import('./progress').then(module => { progressModule.value = module; });
}, { immediate: true });
import { jx3AchievementIconIds } from '../../services/jx3Artwork';

function jx3CalendarSlotTitle(prefix: string, value: string, current: string): string {
  const note = current.match(/(\s*[（(][^）)]+[）)])\s*$/)?.[1] || '';
  return `【${prefix}】${value}${note}`;
}

export function findJx3CalendarTask(list: TaskItem[], apiKey: string, titleMark: string) {
  return list.find(t => t.api_key === apiKey)
    || list.find(t => t.source_type === 'api_jx3' && t.title.includes(titleMark))
    || (titleMark !== '门派日常' && titleMark !== '世界首领'
      ? list.find(t => t.title.includes(titleMark))
      : undefined);
}

export type Jx3SubTab = 'daily' | 'collect' | 'reputation' | 'adventures' | 'achievements' | 'exterior';

function migrateJx3SubTab(raw: string | null): Jx3SubTab {
  if (raw === 'pendants' || raw === 'tokens' || raw === 'collect') return 'collect';
  if (raw === 'achievements' || raw === 'reputation' || raw === 'adventures' || raw === 'daily' || raw === 'exterior') return raw;
  return 'daily';
}

const jx3SubFromQuery = new URLSearchParams(window.location.search).get('jx3');
export const jx3SubTab = ref<Jx3SubTab>(
  migrateJx3SubTab(jx3SubFromQuery || kvGet('waypoint_jx3_subtab'))
);
export function setJx3SubTab(tab: Jx3SubTab) {
  jx3SubTab.value = tab;
  kvSet('waypoint_jx3_subtab', tab);
}

export const jx3WorldBossStored = ref<Jx3WorldBossStoredWeek | null>(loadJx3WorldBossWeek());
export const jx3WorldBoss = ref<Jx3WorldBossWeek>(calculateJx3WorldBossWeek(new Date(), jx3WorldBossStored.value));
export const jx3YuncongEvents = ref<Jx3MapEvent[]>([]);
export const jx3PifengEvents = ref<Jx3MapEvent[]>([]);
export const jx3ChutianEvents = ref<Jx3MapEvent[]>([]);
export const jx3MapRefreshTick = ref(0);
export const jx3AddDraft = reactive({
  text: '',
  category: 'daily' as 'daily' | 'weekly',
  schedule_time: '',
  highlight: 0
});

export const jx3BoxAchievementHits = ref<Jx3CatalogHit[]>([]);
export const jx3AchievementSearching = ref(false);

let achievementSearchTimer: ReturnType<typeof setTimeout> | null = null;
export function scheduleJx3AchievementSearch(keyword: string) {
  if (achievementSearchTimer) clearTimeout(achievementSearchTimer);
  const q = extractJx3TaskScheduleTime(keyword).title.trim();
  if (q.length < 2) {
    jx3BoxAchievementHits.value = [];
    jx3AchievementSearching.value = false;
    return;
  }
  achievementSearchTimer = setTimeout(async () => {
    jx3AchievementSearching.value = true;
    try {
      const raw = await searchJx3BoxAchievements(q, 1);
      const parsed = parseAchievementSearchPayload(raw);
      if (parsed.achievements.length) {
        const hits: Jx3CatalogHit[] = parsed.achievements.slice(0, 10).map(ach => {
          const boss = findJx3TimedBossEvent(ach.Name || '') || findJx3TimedBossEvent(ach.Desc || '');
          const hint = [
            ach.Desc || ach.ShortDesc || '',
            ach.Point ? `${ach.Point}点资历` : ''
          ].filter(Boolean).join(' · ');
          return {
            kind: 'achievement',
            id: String(ach.ID || ''),
            title: ach.Name || '',
            hint: hint || '官方成就',
            tag: '成就',
            category: boss?.scheduleTime.includes('周') ? 'weekly' : (jx3AddDraft.category || 'daily'),
            score: 75,
            iconId: ach.IconID,
            schedule_time: boss?.scheduleTime
          };
        });
        for (const ach of parsed.achievements) {
          if (ach.ID != null && ach.IconID) jx3AchievementIconIds[String(ach.ID)] = ach.IconID;
        }
        jx3BoxAchievementHits.value = hits;
      } else {
        jx3BoxAchievementHits.value = [];
      }
    } catch (_) {
      jx3BoxAchievementHits.value = [];
    } finally {
      jx3AchievementSearching.value = false;
    }
  }, 220);
}

export const jx3CatalogHits = computed(() => {
  const raw = jx3AddDraft.text.trim();
  const q = extractJx3TaskScheduleTime(raw).title.trim();
  const localHits = searchJx3Catalog(q, {
    pendants: jx3Pendants.value,
    reputations: progressModule.value?.jx3Reputations.value || [],
    adventures: progressModule.value?.jx3Adventures.value || [],
    tokens: jx3SetTokens.value,
    achievements: [],
    collectItems: jx3Collect.value.items,
    dungeons: jx3BoxDungeons.value
  });
  const liveAchievements = jx3BoxAchievementHits.value;
  if (!liveAchievements.length) return localHits;

  const seenTitles = new Set(localHits.map(h => h.title));
  const merged = [...localHits];
  for (const ach of liveAchievements) {
    if (!seenTitles.has(ach.title)) {
      seenTitles.add(ach.title);
      merged.push(ach);
    }
  }
  return merged.slice(0, 10);
});

export function jx3TaskVisual(task: TaskItem) {
  return resolveJx3TaskVisual(task, {
    collectItems: jx3Collect.value.items,
    pendants: jx3Pendants.value,
    tokens: jx3SetTokens.value,
    reputations: progressModule.value?.jx3Reputations.value || [],
    dungeons: jx3BoxDungeons.value
  });
}

export function jx3TaskIconUrl(task: TaskItem): string {
  return jx3TaskVisual(task).iconUrl;
}

export function jx3CalendarSlotIconUrl(slotKey: Jx3CalendarSlotKey): string {
  return jx3CalendarCategoryImage(slotKey);
}

export const jx3CalendarSlotHits = computed(() => {
  const raw = jx3AddDraft.text.trim();
  const q = extractJx3TaskScheduleTime(raw).title.trim().toLowerCase();
  if (!q) return JX3_CALENDAR_SLOTS;
  return JX3_CALENDAR_SLOTS.filter(s => s.label.includes(q) || s.mark.includes(q) || s.key.includes(q));
});

watch(() => jx3AddDraft.text, (val) => {
  jx3AddDraft.highlight = 0;
  scheduleJx3AchievementSearch(val);
});

export function jx3AddHitCount() {
  return jx3CalendarSlotHits.value.length + jx3CatalogHits.value.length;
}

export function moveJx3AddHighlight(delta: number) {
  const total = jx3AddHitCount();
  if (!total) {
    jx3AddDraft.highlight = 0;
    return;
  }
  jx3AddDraft.highlight = (jx3AddDraft.highlight + delta + total) % total;
}

export async function submitJx3CustomTask(text = jx3AddDraft.text, category = jx3AddDraft.category, customScheduleTime?: string) {
  let title = text.trim();
  if (!title) return;
  const charId = activeCharacter.value;
  if (!charId) {
    showToast('没有当前角色', '先在金库登记剑三角色');
    return;
  }
  let schedule_time = (customScheduleTime !== undefined ? customScheduleTime : jx3AddDraft.schedule_time).trim();
  const extracted = extractJx3TaskScheduleTime(title);
  if (!schedule_time && extracted.scheduleTime) {
    schedule_time = extracted.scheduleTime;
    title = extracted.title;
  } else if (schedule_time && extracted.scheduleTime && extracted.title) {
    title = extracted.title;
  }
  await pushCharacterTask({
    id: newTaskId('jx3'),
    character_id: charId,
    title,
    category,
    schedule_time: schedule_time || undefined,
    reset_rule: resetRuleFor(charId, category),
    source_type: 'custom',
    tag: category === 'weekly' ? '自建周常' : '自建日常',
    done: false
  }, '已添加', title);
  jx3AddDraft.text = '';
  jx3AddDraft.schedule_time = '';
}

export async function attachJx3CalendarSlot(charId: string, slotKey: Jx3CalendarSlotKey, notify = true) {
  const slot = JX3_CALENDAR_SLOTS.find(s => s.key === slotKey);
  if (!slot) return false;
  const list = allAccountTasks.value[charId] || [];
  if (findJx3CalendarTask(list, slot.key, slot.mark)) {
    if (notify) showToast('已经挂过', slot.label);
    return false;
  }
  const placeholder = slot.key === 'leader' ? '周三周五名单' : '待同步';
  const schedule_time = slot.key === 'leader' ? '每周三、周五 20:00 - 21:00' : undefined;
  await pushCharacterTask({
    id: newTaskId(`jx3_${slot.key}`),
    character_id: charId,
    title: `【${slot.label}】${placeholder}`,
    category: slot.category,
    schedule_time,
    reset_rule: resetRuleFor(charId, slot.category),
    source_type: 'api_jx3',
    api_key: slot.key,
    tag: slot.tag,
    done: false
  }, notify ? '已挂日历槽' : undefined, notify ? slot.label : undefined);
  return true;
}

export async function attachJx3CatalogHit(hit: Jx3CatalogHit, customScheduleTime?: string) {
  const charId = activeCharacter.value;
  if (!charId) {
    showToast('没有当前角色', '先在金库登记剑三角色');
    return;
  }
  const list = allAccountTasks.value[charId] || [];
  if (list.some(t => t.catalog_kind === hit.kind && t.catalog_id === hit.id && !t.done)) {
    showToast('已经挂过', hit.title);
    return;
  }
  let schedule_time = (customScheduleTime !== undefined ? customScheduleTime : (hit.schedule_time || jx3AddDraft.schedule_time)).trim();
  if (!schedule_time) {
    const extracted = extractJx3TaskScheduleTime(jx3AddDraft.text);
    if (extracted.scheduleTime) {
      schedule_time = extracted.scheduleTime;
    }
  }
  const targetCategory = (jx3AddDraft.category === 'weekly' || hit.category === 'weekly' || (schedule_time && schedule_time.includes('周'))) ? 'weekly' : hit.category;
  await pushCharacterTask({
    id: newTaskId(`jx3_${hit.kind}`),
    character_id: charId,
    title: `【${hit.tag}】${hit.title}`,
    category: targetCategory,
    schedule_time: schedule_time || undefined,
    reset_rule: resetRuleFor(charId, targetCategory),
    source_type: 'custom',
    tag: hit.tag,
    done: false,
    catalog_kind: hit.kind,
    catalog_id: hit.id
  }, '已引用图鉴', `${hit.tag} · ${hit.title}`);
  jx3AddDraft.text = '';
  jx3AddDraft.schedule_time = '';
}

export async function submitJx3Add(preferHighlight = false) {
  const slotHits = jx3CalendarSlotHits.value;
  const catalogHits = jx3CatalogHits.value;
  const total = slotHits.length + catalogHits.length;
  if (preferHighlight && total) {
    const idx = Math.min(jx3AddDraft.highlight, total - 1);
    if (idx < slotHits.length) {
      const charId = activeCharacter.value;
      if (!charId) {
        showToast('没有当前角色', '先在金库登记剑三角色');
        return;
      }
      const attached = await attachJx3CalendarSlot(charId, slotHits[idx].key);
      if (attached) await syncJx3DailyCalendar();
      jx3AddDraft.text = '';
      jx3AddDraft.schedule_time = '';
      return;
    }
    await attachJx3CatalogHit(catalogHits[idx - slotHits.length]);
    return;
  }
  await submitJx3CustomTask();
}

export const yuncangFeed = computed<Jx3MapFeed>(() => ({
  name: '云从社',
  map: '银霜口',
  events: jx3YuncongEvents.value
}));

export const pifengFeed = computed<Jx3MapFeed>(() => ({
  name: '披风会',
  map: '河西瀚漠',
  events: jx3PifengEvents.value
}));

export const chutianFeed = computed<Jx3MapFeed>(() => ({
  name: '楚天社',
  map: '楚州',
  events: jx3ChutianEvents.value
}));

export const yuncangZhueClock = computed<Jx3MapRefreshInfo | null>(() => {
  jx3MapRefreshTick.value;
  return calculateJx3ZhueClock('云从社');
});

export const pifengZhueClock = computed<Jx3MapRefreshInfo | null>(() => {
  jx3MapRefreshTick.value;
  return calculateJx3ZhueClock('披风会');
});

export const chutianZhueClock = computed<Jx3MapRefreshInfo | null>(() => {
  jx3MapRefreshTick.value;
  return calculateJx3ZhueClock('楚天社', new Date(), jx3ChutianEvents.value);
});

export const jx3ZhueAlert = computed(() =>
  !!(yuncangZhueClock.value?.inWindow || pifengZhueClock.value?.inWindow || chutianZhueClock.value?.inWindow)
);

export const jx3ZhueApproaching = computed(() =>
  !!(yuncangZhueClock.value?.approaching || pifengZhueClock.value?.approaching || chutianZhueClock.value?.approaching)
);

export function refreshJx3WorldBossClock() {
  jx3MapRefreshTick.value += 1;
  jx3WorldBoss.value = calculateJx3WorldBossWeek(new Date(), jx3WorldBossStored.value);
}

function rememberJx3WorldBossDay(weekday: number, dateStr: string, names: string[]) {
  const weekStart = formatJx3Date(startOfJx3Week(new Date()));
  const next = upsertJx3WorldBossDay(jx3WorldBossStored.value, weekday, dateStr, names, weekStart);
  jx3WorldBossStored.value = next;
  saveJx3WorldBossWeek(next);
  refreshJx3WorldBossClock();
}

async function collectJx3WorldBossWeek(today: { date: string; leader?: string[] }): Promise<string[]> {
  const now = new Date();
  const weekStart = formatJx3Date(startOfJx3Week(now));
  const stored = jx3WorldBossStored.value;
  if (stored && stored.weekStart === weekStart) {
    const allKnown = JX3_WORLD_BOSS_SCHEDULE.slots.every(slot => {
      const d = stored.days.find(x => x.weekday === slot.weekday);
      return d && d.names.length > 0;
    });
    if (allKnown) {
      return stored.days.flatMap(d => d.names);
    }
  }

  const fetched = await Promise.all(JX3_WORLD_BOSS_SCHEDULE.slots.map(async slot => {
    if (stored && stored.weekStart === weekStart) {
      const d = stored.days.find(x => x.weekday === slot.weekday);
      if (d && d.names.length > 0) {
        return { weekday: slot.weekday, dateStr: d.dateStr, names: d.names };
      }
    }
    const offset = daysUntilJx3Weekday(now, slot.weekday);
    if (offset === 0) return { weekday: slot.weekday, dateStr: today.date, names: today.leader || [] };
    if (offset == null) return null;
    const cal = await getJx3DailyCalendar(offset).catch(() => null);
    if (!cal?.leader?.length) return null;
    return { weekday: slot.weekday, dateStr: cal.date, names: cal.leader };
  }));
  for (const row of fetched) {
    if (row?.names.length) rememberJx3WorldBossDay(row.weekday, row.dateStr, row.names);
  }
  return (jx3WorldBossStored.value?.days || []).flatMap(d => d.names);
}

let lastMapEventsAt = 0;
let mapEventsJob: Promise<void> | undefined;
export async function refreshJx3MapEvents(force = false) {
  if (mapEventsJob) return mapEventsJob;
  if (!force && Date.now() - lastMapEventsAt < 15 * 60 * 1000) {
    return;
  }
  const job = (async () => {
    const [yun, pi, chu] = await Promise.all([
      getJx3Celebs('云从社'),
      getJx3Celebs('披风会'),
      getJx3Celebs('楚天社'),
    ]);
    if (yun !== null) jx3YuncongEvents.value = filterJx3ZhueEvents(yun);
    if (pi !== null) jx3PifengEvents.value = filterJx3ZhueEvents(pi);
    if (chu !== null) jx3ChutianEvents.value = filterJx3ZhueEvents(chu);
    if (yun !== null && pi !== null && chu !== null) lastMapEventsAt = Date.now();
  })();
  mapEventsJob = job;
  try {
    await job;
  } finally {
    if (mapEventsJob === job) mapEventsJob = undefined;
  }
}

export const jx3ApiStatus = ref<'unknown' | 'ok' | 'fail'>('unknown');
/** 最近一次拉到的当日日历，给今日页和日课面板展示秘境、福缘、本周团本 */
export const jx3TodayCalendar = ref<Jx3DailyCalendar | null>(null);

let lastDailySyncDate = '';
let lastDailySyncAt = 0;
let lastDailyAttemptAt = 0;
let dailySyncJob: Promise<void> | undefined;

// 同步 JX3API 真实日程。失败不改标题、不报连通。
export function syncJx3DailyCalendar(notify = false): Promise<void> {
  if (dailySyncJob) return dailySyncJob;
  const todayStr = beijingDay(new Date(Date.now() - 7 * 60 * 60_000));
  if (!notify && jx3ApiStatus.value === 'ok' && lastDailySyncDate === todayStr && Date.now() - lastDailySyncAt < 15 * 60 * 1000) {
    return Promise.resolve();
  }
  if (!notify && lastDailyAttemptAt && Date.now() - lastDailyAttemptAt < 60_000) {
    return Promise.resolve();
  }
  lastDailyAttemptAt = Date.now();
  const job = (async () => {
    let sourceLoaded = false;
    try {
      const daily = await getJx3DailyCalendar();
      if (!daily || daily.date !== todayStr || !daily.war || !daily.mine || !daily.school) {
        const reason = daily?.date && daily.date !== todayStr
          ? `JX3API 返回 ${daily.date} 的日历，等待今日数据`
          : 'JX3API 未返回当日大战、矿车或门派';
        jx3ApiStatus.value = 'fail';
        reportSource('jx3.calendar', '剑三日历', 'error', reason);
        if (notify) showToast('剑三日历同步失败', reason);
        return;
      }
      sourceLoaded = true;
      jx3ApiStatus.value = 'ok';
      jx3TodayCalendar.value = daily;
      reportSource('jx3.calendar', '剑三日历', 'ok');
      lastDailySyncDate = todayStr;
      lastDailySyncAt = Date.now();
      const weekNames = await collectJx3WorldBossWeek(daily);
      const leaderLine = weekNames.length ? weekNames.join('、') : (daily.leader || []).join('、');
      const values: Record<Jx3CalendarSlotKey, string> = {
        war: daily.war,
        mine: daily.mine,
        school: daily.school,
        battle: daily.battle || '',
        rescue: daily.rescue || '',
        card: (daily.card || []).join('、'),
        leader: leaderLine,
        raid: (daily.weekly_raid || []).join('、'),
        conn: (daily.weekly_conn || []).join('、')
      };
      const writes: Promise<void>[] = [];
      for (const c of characterList.value.filter(ch => ch.game_id === 'jx3')) {
        const list = allAccountTasks.value[c.id] || [];
        for (const slot of JX3_CALENDAR_SLOTS) {
          const value = values[slot.key];
          const task = findJx3CalendarTask(list, slot.key, slot.mark);
          if (!task) continue;
          if (slot.key === 'leader' && !task.schedule_time) {
            task.schedule_time = '每周三、周五 20:00 - 21:00';
            writes.push(updateTaskScheduleTime(task.id, task.schedule_time, c.id));
          }
          if (!value && slot.key !== 'leader') continue;
          const next = jx3CalendarSlotTitle(slot.label, value || '今日无', task.title);
          if (task.title === next) continue;
          task.title = next;
          writes.push(updateTaskTitle(task.id, next, c.id));
        }
      }
      if (writes.length) {
        await Promise.all(writes);
        syncActiveCharacterState();
      }
      if (notify) showToast('剑三日历已同步', `${daily.war} · ${daily.mine} · ${daily.school}`);
    } catch (err) {
      if (!sourceLoaded) {
        jx3ApiStatus.value = 'fail';
        reportSource('jx3.calendar', '剑三日历', 'error', err instanceof Error ? err.message : String(err));
      }
      if (notify) showToast(sourceLoaded ? '剑三日历已到，任务清单写入失败' : '剑三日历同步失败', String(err));
      console.warn('Failed to sync JX3 daily:', err);
    } finally {
      void refreshJx3MapEvents();
    }
  })();
  dailySyncJob = job;
  void job.finally(() => {
    if (dailySyncJob === job) dailySyncJob = undefined;
  });
  return job;
}
