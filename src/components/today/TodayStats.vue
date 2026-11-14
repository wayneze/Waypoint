<script setup lang="ts">
// 近 30 天统计：连续打卡、本周日课完成率、每天打卡热力格、收集进度更新最多的图鉴。只读。
import { computed, onMounted, ref } from 'vue';
import { getActivityStats, type ActivityStats } from '../../services/systemApi';
import { localDay } from '../../services/fetchCache';

const DAYS = 30;
const stats = ref<ActivityStats | null>(null);
const open = ref(false);

const SCOPE_LABEL: Record<string, string> = {
  'task.daily': '日课',
  'task.weekly': '周常',
  'sky.quest': '光遇任务',
  'sky.season_candle': '季蜡',
  'sky.event_daily': '节日日常',
  'jx3.collect': '剑三收集',
  'jx3.achievement': '剑三成就',
  'jx3.reputation': '剑三声望',
  'jx3.tokens': '剑三牌子',
  'sky.spirits': '星盘先祖',
  'sky.season_wish': '季节心愿',
  'sky.event_wish': '节日心愿',
  'guide.routes': '攻略路线',
};

async function load() {
  stats.value = await getActivityStats(DAYS).catch(() => null);
}
onMounted(load);

function dayList(): string[] {
  const out: string[] = [];
  for (let i = DAYS - 1; i >= 0; i--) out.push(localDay(Date.now() - i * 86400000));
  return out;
}

const perDay = computed(() => {
  const map = new Map<string, number>();
  for (const [day, , n] of stats.value?.checks ?? []) map.set(day, (map.get(day) ?? 0) + n);
  return map;
});

const cells = computed(() => {
  const max = Math.max(1, ...perDay.value.values());
  return dayList().map(day => ({ day, n: perDay.value.get(day) ?? 0, level: Math.ceil(((perDay.value.get(day) ?? 0) / max) * 4) }));
});

const streak = computed(() => {
  const days = dayList().reverse();
  // 今天还没勾不算断
  let i = perDay.value.get(days[0]) ? 0 : 1;
  let n = 0;
  for (; i < days.length && perDay.value.get(days[i]); i++) n++;
  return n;
});

const weekRate = computed(() => {
  const s = stats.value;
  if (!s?.daily_tasks) return null;
  const now = new Date();
  const sinceMonday = (now.getDay() + 6) % 7;
  const days = dayList().slice(-(sinceMonday + 1));
  const done = (s.checks ?? []).filter(([d, scope]) => scope === 'task.daily' && days.includes(d)).reduce((a, [, , n]) => a + n, 0);
  return Math.min(100, Math.round((done / (s.daily_tasks * days.length)) * 100));
});

const topProgress = computed(() => {
  const sum = new Map<string, number>();
  for (const [, scope, n] of stats.value?.progress ?? []) sum.set(scope, (sum.get(scope) ?? 0) + n);
  return [...sum].sort((a, b) => b[1] - a[1]).slice(0, 4).map(([scope, n]) => ({ label: SCOPE_LABEL[scope] || scope, n }));
});
</script>

<template>
  <div v-if="stats" class="space-y-2">
    <button type="button" class="flex items-baseline justify-between gap-2 px-0.5 w-full cursor-pointer" @click="open = !open">
      <h4 class="text-[16px] font-semibold text-ink">近 {{ DAYS }} 天</h4>
      <span class="text-[12px] text-faint">连续 {{ streak }} 天 · {{ open ? '收起' : '展开' }}</span>
    </button>
    <div v-if="open" class="grid grid-cols-1 md:grid-cols-3 gap-2.5">
      <div class="wp-paper wp-stat">
        <span class="text-[12px] text-faint">连续打卡</span>
        <span class="wp-stat-value text-ink">{{ streak }}<span class="text-[14px] text-faint"> 天</span></span>
      </div>
      <div class="wp-paper wp-stat">
        <span class="text-[12px] text-faint">本周日课完成率</span>
        <span class="wp-stat-value text-ink">{{ weekRate ?? '—' }}<span v-if="weekRate != null" class="text-[14px] text-faint">%</span></span>
        <div class="wp-meter" :class="{ 'is-done': weekRate === 100 }"><span :style="{ width: (weekRate ?? 0) + '%' }" /></div>
      </div>
      <div class="wp-paper wp-stat">
        <span class="text-[12px] text-faint">图鉴更新</span>
        <span v-if="!topProgress.length" class="text-[13px] text-faint">这段时间没有动过</span>
        <span v-for="p in topProgress" :key="p.label" class="text-[13px] text-ink flex justify-between"><span>{{ p.label }}</span><span class="font-mono text-faint">{{ p.n }}</span></span>
      </div>
      <div class="wp-paper p-3.5 md:col-span-3">
        <div class="grid gap-1" style="grid-template-columns: repeat(30, minmax(0, 1fr))">
          <span
            v-for="c in cells" :key="c.day" :title="`${c.day} · ${c.n} 项`"
            class="aspect-square rounded-[3px]"
            :style="{ background: c.n ? `color-mix(in srgb, var(--color-done) ${20 + c.level * 20}%, var(--color-raised))` : 'var(--color-raised)' }" />
        </div>
        <div class="flex justify-between text-[11px] text-faint font-mono mt-1.5">
          <span>{{ cells[0]?.day }}</span><span>今天</span>
        </div>
      </div>
    </div>
  </div>
</template>
