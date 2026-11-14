<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Check, ChevronDown, Minus, Pencil, Plus, X } from '@lucide/vue';
import { homeLineDone, type HomeLine } from '../workspace/daily';
import { jx3TaskIconUrl } from '../workspace/jx3/calendar';
import { isSavingDaily } from '../workspace/sky/daily';
import { noteStepProgress } from '../services/memoNote';
import GameStampIcon from './GameStampIcon.vue';
import Jx3BoxIcon from './Jx3BoxIcon.vue';
import MemoNoteBody from './MemoNoteBody.vue';

const props = defineProps<{ line: HomeLine; busy?: boolean }>();
const emit = defineEmits<{ toggle: []; delete: []; rename: [title: string]; progress: [delta: number]; step: [index: number] }>();
const editing = ref(false);
const expanded = ref(false);
const draft = ref('');
const input = ref<HTMLInputElement>();
const done = computed(() => homeLineDone(props.line));
const disabled = computed(() => props.busy || (props.line.kind === 'sky' && (!props.line.group.ready || isSavingDaily.value)));
const steps = computed(() => props.line.kind === 'memo' ? noteStepProgress(props.line.memo.note || '') : null);
async function edit() {
  draft.value = props.line.title; editing.value = true;
  await nextTick(); input.value?.focus(); input.value?.select();
}
function commit() {
  if (!draft.value.trim() || props.busy) return;
  emit('rename', draft.value.trim()); editing.value = false;
}
</script>

<template>
  <article class="today-line" :class="{ 'is-done': done }" :data-game="line.stamp" :aria-label="line.title" :aria-busy="busy">
    <div class="today-line-main">
      <button class="today-line-check" type="button" :disabled="disabled" :aria-label="(done ? '恢复待办：' : '完成待办：') + line.title" :aria-pressed="done" @click="emit('toggle')"><Check /></button>
      <Jx3BoxIcon v-if="line.kind === 'task'" :src="jx3TaskIconUrl(line.task)" size="w-6 h-6" />
      <div class="today-line-copy">
        <form v-if="editing" class="today-line-edit" @submit.prevent="commit">
          <input ref="input" v-model="draft" required aria-label="修改待办标题" @keydown.enter="$event.isComposing && $event.preventDefault()" @keydown.esc.stop="editing = false" />
          <button type="submit" :disabled="busy || !draft.trim()">保存</button><button type="button" aria-label="取消修改" @click="editing = false"><X /></button>
          <button type="button" class="today-line-delete" :disabled="busy" @click="emit('delete')">删除…</button>
        </form>
        <h3 v-else>{{ line.title }}</h3>
        <p class="today-line-meta"><GameStampIcon :game="line.stamp" class="w-3 h-3" /><span>{{ line.identity }}</span><span v-if="line.hint">{{ line.hint }}</span><span v-if="steps?.total">{{ steps.done }}/{{ steps.total }} 步</span></p>
      </div>
      <button v-if="!editing && line.kind !== 'sky'" type="button" class="today-line-action" :disabled="busy" :aria-label="'编辑待办：' + line.title" title="编辑待办" @click="edit"><Pencil /></button>
    </div>
    <div v-if="line.kind === 'memo' && line.memo.has_progress" class="today-line-progress">
      <span>次数</span><button type="button" :disabled="busy || line.memo.current_progress <= 0" :aria-label="'减少次数：' + line.title" @click="emit('progress', -1)"><Minus /></button><b>{{ line.memo.current_progress }}<small> / {{ line.memo.target_progress }}</small></b><button type="button" :disabled="busy || line.memo.current_progress >= line.memo.target_progress" :aria-label="'增加次数：' + line.title" @click="emit('progress', 1)"><Plus /></button>
    </div>
    <template v-if="line.kind === 'memo' && line.memo.note">
      <button class="today-line-note" type="button" :aria-expanded="expanded" :aria-controls="'today-note-' + line.id" @click="expanded = !expanded"><ChevronDown :class="{ 'is-open': expanded }" />{{ expanded ? '收起备注与步骤' : '查看备注与步骤' }}</button>
      <div v-if="expanded" :id="'today-note-' + line.id" class="today-line-note-body"><MemoNoteBody :note="line.memo.note" :disabled="busy" @toggle-step="emit('step', $event)" /></div>
    </template>
  </article>
</template>
