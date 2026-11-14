<script setup lang="ts">
// 数据源状态条：显示这一页用到的网络数据最后成功时间、失败原因，并给重试。
// 全部正常且是今天的数据时不显示，避免占地方。
import { computed } from 'vue';
import { RefreshCw } from '@lucide/vue';
import { sourceStatus, type SourceStatus } from '../services/fetchCache';

const props = defineProps<{
  /** fetchCache 的缓存键，和 sourceStatus 对应 */
  sources: string[];
}>();
const emit = defineEmits<{ retry: [key: string] }>();

function ago(ts: number): string {
  if (!ts) return '';
  const min = Math.round((Date.now() - ts) / 60000);
  if (min < 1) return '刚刚';
  if (min < 60) return `${min} 分钟前`;
  const h = Math.round(min / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.round(h / 24)} 天前`;
}

const rows = computed(() =>
  props.sources
    .map(key => ({ key, s: sourceStatus[key] as SourceStatus | undefined }))
    .filter((r): r is { key: string; s: SourceStatus } => !!r.s && r.s.state !== 'ok')
);

function text(s: SourceStatus): string {
  if (s.state === 'loading') return '正在拉取…';
  if (s.state === 'refreshing') return `显示 ${ago(s.fetchedAt)}的数据，正在后台更新`;
  if (s.state === 'stale') return `更新失败，显示 ${ago(s.fetchedAt)}的数据 · ${s.error}`;
  if (s.state === 'partial') return `已更新，但${s.error}`;
  return `拉取失败 · ${s.error}`;
}
</script>

<template>
  <div v-if="rows.length" class="flex flex-col gap-1">
    <div
      v-for="{ key, s } in rows"
      :key="key"
      class="flex items-center gap-2 px-3 py-1.5 rounded-md border text-[12px]"
      :class="s.state === 'error' || s.state === 'stale' || s.state === 'partial' ? 'border-alert/35 bg-alert/8 text-alert' : 'border-hairline bg-raised/60 text-muted'">
      <span class="font-medium shrink-0">{{ s.label }}</span>
      <span class="min-w-0 truncate" :title="s.error">{{ text(s) }}</span>
      <button
        v-if="s.state === 'error' || s.state === 'stale' || s.state === 'partial'"
        type="button"
        class="ml-auto shrink-0 inline-flex items-center gap-1 text-[12px] hover:text-ink cursor-pointer"
        @click="emit('retry', key)">
        <RefreshCw class="w-3 h-3" />重试
      </button>
    </div>
  </div>
</template>
