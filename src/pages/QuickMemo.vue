<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { memoLinePreview, parseMemoLine } from '../services/memoCapture';
import { addCustomMemo } from '../services/api';
import { kvGet, kvSet } from '../services/kv';
import type { CustomMemo } from '../types';

const LAST_TAG_KEY = 'waypoint_memo_last_tag';
const draft = ref('');
const input = ref<HTMLInputElement | null>(null);
const errorText = ref('');

const fallbackTag = computed(() => kvGet(LAST_TAG_KEY) || '通用');
const parsed = computed(() => parseMemoLine(draft.value));
const preview = computed(() => memoLinePreview(parsed.value, fallbackTag.value));

onMounted(() => {
  input.value?.focus();
});

async function hide() {
  try {
    await getCurrentWindow().hide();
  } catch {
    /* 浏览器预览没有这扇窗 */
  }
}

async function submit() {
  const line = parsed.value;
  if (!line.title) {
    errorText.value = '写上事项再回车';
    return;
  }
  const tag = line.tag || fallbackTag.value;
  const memo: CustomMemo = {
    id: 'm_' + Date.now(),
    title: line.title,
    tag,
    cycle_type: line.cycle_type,
    cycle_val: line.cycle_val,
    has_progress: line.has_progress,
    current_progress: 0,
    target_progress: line.has_progress ? line.target_progress : 0,
    done: false,
    created_at: new Date().toISOString().replace('T', ' ').slice(0, 19),
  };
  try {
    await addCustomMemo(memo);
    kvSet(LAST_TAG_KEY, tag);
    draft.value = '';
    errorText.value = '';
    await hide();
  } catch (err) {
    errorText.value = String(err);
  }
}

async function onKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    event.preventDefault();
    draft.value = '';
    await hide();
    return;
  }
  if (event.key === 'Enter') {
    event.preventDefault();
    await submit();
  }
}
</script>

<template>
  <div class="flex h-screen w-screen items-center justify-center bg-transparent" @mousedown.self="hide">
    <form
      class="flex h-[52px] w-[560px] items-center gap-3 rounded-full border border-hairline bg-paper px-5 shadow-[0_16px_40px_rgba(0,0,0,0.45)]"
      @submit.prevent="submit"
    >
      <input
        ref="input"
        v-model="draft"
        type="text"
        placeholder="记一笔，回车即走"
        class="min-w-0 flex-1 bg-transparent text-[15px] text-ink placeholder:text-faint focus:outline-none"
        @keydown="onKeydown"
      >
      <span class="max-w-[168px] shrink-0 truncate text-[12px] text-faint">{{ errorText || preview }}</span>
    </form>
  </div>
</template>
