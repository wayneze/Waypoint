<script setup lang="ts">
// 今日页：正在进行、7 天内结束的光遇季节/活动/复刻，带收藏页兑换清单里还没换的。数据直接读缓存，不拉网络。
import { computed } from 'vue';
import type { SkyCalendarItem } from '../../services/skyApi';
import { hotlinkImageSrc } from '../sky/wikiImage';
import { state } from '../../workspace/core';
import { seasonGap, seasonGapText } from '../../services/skySeasonService';
import { homeEndingItems } from '../../workspace/daily';

const items = computed(() => homeEndingItems.value.map(r => ({ ...r, ex: exchangeLeft(r.it) })));

/** 季节、活动按标题对上收藏页的兑换清单，给出还差什么；回归先祖没有兑换清单。 */
function exchangeLeft(it: SkyCalendarItem): { text: string; late: boolean } | null {
  if (it.kind === 'season') {
    const guide = state.seasonalGuides.find(g => !g.isCustom && g.season === it.title);
    if (!guide) return null;
    const gap = seasonGap(guide);
    const text = seasonGapText(gap);
    if (!text) return null;
    const late = gap.daysNeeded !== null && gap.daysLeft !== null && gap.daysNeeded > gap.daysLeft;
    return { text: late ? `${text}，做满也换不完` : text, late };
  }
  if (it.kind === 'event') {
    const guide = state.eventGuides.find(g => g.season === it.title);
    // 礼包是花钱买的，不算待换
    const pending = guide?.itemList.filter(i => !i.obtained && !/元$/.test(i.costDesc)).length ?? 0;
    return pending ? { text: `${pending} 件没换`, late: false } : null;
  }
  return null;
}

function leftText(n: number) {
  return n === 0 ? '今天结束' : `还剩 ${n} 天`;
}
</script>

<template>
  <div v-if="items.length" class="space-y-2">
    <div class="today-section-heading">
      <h4 class="text-[16px] font-semibold text-ink">快结束了</h4>
      <span class="text-[13px] font-mono tabular-nums text-alert">{{ items.length }}</span>
    </div>
    <div class="grid grid-cols-1 gap-3">
      <div v-for="{ it, left, ex } in items" :key="it.id" class="today-ending-card">
        <div class="wp-banner">
          <img v-if="it.icon" :src="hotlinkImageSrc(it.icon)" alt="" loading="lazy" referrerpolicy="no-referrer" />
          <span class="wp-banner-tag" :class="{ 'is-ending': left <= 1 }">{{ leftText(left) }}</span>
        </div>
        <div class="px-3.5 py-2.5">
          <p class="text-[14px] font-semibold text-ink leading-snug truncate">{{ it.title }}</p>
          <p class="text-[12px] text-faint mt-0.5 truncate">
            <span v-if="it.names.length">{{ it.names.join('、') }} · </span>{{ it.end }} 结束
          </p>
          <p v-if="ex" class="text-[12px] mt-0.5 truncate" :class="ex.late || left <= 1 ? 'text-alert' : 'text-muted'">{{ ex.text }}</p>
        </div>
      </div>
    </div>
  </div>
</template>
