<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { GameStamp } from '../services/games';
import { jx3SchoolIconUrl } from '../services/jx3School';
import { posterSrc } from './sky/wikiImage';

const props = withDefaults(defineProps<{
  stamp: GameStamp;
  name: string;
  sect?: string;
  size?: 'sm' | 'md';
}>(), { size: 'md' });

// 剑三有门派图标就用图标（走本地盘缓存），其余用名字首字配游戏色
const iconUrl = computed(() => props.stamp === 'jx3' ? jx3SchoolIconUrl(props.sect) : '');
const failed = ref(false);
watch(iconUrl, () => { failed.value = false; });
const mark = computed(() => Array.from(props.name.trim() || '?')[0]);
</script>

<template>
  <span
    class="shrink-0 rounded-md overflow-hidden flex items-center justify-center border font-semibold"
    :class="[
      size === 'sm' ? 'w-7 h-7 text-[13px]' : 'w-9 h-9 text-[15px]',
      iconUrl && !failed ? 'bg-canvas border-hairline' : 'bg-raised border-hairline text-stamp-' + stamp,
    ]"
    :title="sect || name"
  >
    <img
      v-if="iconUrl && !failed"
      :src="posterSrc(iconUrl)"
      alt=""
      class="w-full h-full object-contain"
      @error="failed = true"
    />
    <template v-else>{{ mark }}</template>
  </span>
</template>
