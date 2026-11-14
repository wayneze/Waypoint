<script setup lang="ts">
import { computed } from 'vue';
import { noteParts } from '../services/memoNote';
import { openExternalUrl } from '../services/systemApi';

const props = defineProps<{ note: string; disabled?: boolean }>();
const emit = defineEmits<{ toggleStep: [index: number] }>();
const parts = computed(() => noteParts(props.note));

async function openLink(url: string) {
  try {
    await openExternalUrl(url);
  } catch {
    /* 非法地址不打开 */
  }
}
</script>

<template>
  <div class="memo-note-body text-[13px] text-muted leading-relaxed select-text whitespace-pre-wrap break-words">
    <template v-for="(part, index) in parts" :key="index">
      <button
        v-if="part.kind === 'step'"
        type="button"
        :disabled="disabled"
        :aria-pressed="part.done"
        :aria-label="part.text"
        class="flex w-full items-start gap-1.5 py-0.5 text-left cursor-pointer"
        @click="emit('toggleStep', part.index)"
      >
        <span
          class="mt-0.5 inline-flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded border text-[10px]"
          :class="part.done ? 'border-done bg-done/15 text-done' : 'border-hairline text-transparent'"
        >✓</span>
        <span :class="part.done ? 'line-through text-faint' : 'text-ink'">{{ part.text }}</span>
      </button>
      <button
        v-else-if="part.kind === 'link'"
        type="button"
        class="text-beacon underline underline-offset-2 cursor-pointer"
        @click="openLink(part.url)"
      >{{ part.text }}</button>
      <span v-else>{{ part.text }}</span>
    </template>
  </div>
</template>
