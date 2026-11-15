<script setup lang="ts">
import { computed } from 'vue';
import { restoreProgress } from '../workspace/settings';

const labels = {
  reading: '正在读取备份文件…',
  downloading: '正在下载云端备份…',
  decrypting: '正在解密并解压备份…',
  validating: '正在校验数据库与图片…',
  saving_undo: '正在保留恢复前快照…',
  replacing: '正在替换本机资料…',
  reloading: '资料已替换，正在重新加载…',
};
const label = computed(() => restoreProgress.value ? labels[restoreProgress.value.phase] : '正在准备恢复…');
const downloading = computed(() => restoreProgress.value?.phase === 'downloading');
const bytesLabel = computed(() => {
  const progress = restoreProgress.value;
  if (!progress || !downloading.value) return '';
  const mib = (bytes: number) => (bytes / 1024 / 1024).toFixed(1);
  return progress.total
    ? `${mib(progress.received)} / ${mib(progress.total)} MiB`
    : `已下载 ${mib(progress.received)} MiB · 文件大小未知`;
});
</script>

<template>
  <section class="mx-auto flex min-h-full w-full max-w-lg flex-col justify-center gap-4 p-8 text-sm" role="status" aria-live="polite" aria-busy="true">
    <h1 class="text-base font-medium text-ink">正在恢复工作台资料</h1>
    <p class="text-muted">{{ label }}</p>
    <progress class="h-1.5 w-full accent-beacon" aria-label="备份恢复进度"
      :value="downloading && restoreProgress?.total ? restoreProgress.received : undefined"
      :max="downloading && restoreProgress?.total ? restoreProgress.total : 1" />
    <p v-if="bytesLabel" class="font-mono text-xs text-muted tabular-nums">{{ bytesLabel }}</p>
    <p class="text-xs leading-relaxed text-faint">下载完成后还会校验和替换资料。恢复成功后会自动重新加载，请等待此页面结束。</p>
  </section>
</template>
