<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Check, ChevronDown, Copy, Minus, Pencil, Pin, Plus, Trash2 } from '@lucide/vue';
import type { CustomMemo } from '../../types';
import {
  copyMemoCoordinates, getMemoRemainingText, handleDeleteMemo, handleProgressChange,
  handleToggleMemo, handleToggleMemoPin, handleToggleMemoStep,
} from '../../workspace/memos';
import { characterList, getShortCharName } from '../../workspace/core';
import { noteStepProgress } from '../../services/memoNote';
import MemoEditor from './MemoEditor.vue';
import MemoNoteBody from '../MemoNoteBody.vue';

const props = defineProps<{ memo: CustomMemo; editing: boolean }>();
const emit = defineEmits<{ edit: []; close: [] }>();
const expanded = ref(false);
const busy = ref(false);
const error = ref('');
const deleteArmed = ref(false);
const editButton = ref<HTMLButtonElement>();
const character = computed(() => {
  const char = characterList.value.find(char => char.id === props.memo.character_id);
  return char ? getShortCharName(char) : '';
});
const timing = computed(() => getMemoRemainingText(props.memo));
const steps = computed(() => noteStepProgress(props.memo.note || ''));
const progress = computed(() => Math.min(100, props.memo.current_progress / Math.max(1, props.memo.target_progress) * 100));
const preview = computed(() => (props.memo.note || '').replace(/^\s*-\s+\[[ xX]\]\s?/gm, '').replace(/\n/g, ' · '));

async function run(action: () => Promise<unknown>) {
  if (busy.value) return;
  busy.value = true; error.value = '';
  try { await action(); } catch { error.value = '未能保存更改，请重试。'; }
  finally { busy.value = false; }
}
async function closeEditor() {
  deleteArmed.value = false;
  emit('close');
  await nextTick();
  editButton.value?.focus();
}
</script>

<template>
  <article class="memo-row" :class="{ 'is-done': memo.done, 'is-pinned': memo.pinned, 'is-editing': editing }" :aria-label="memo.title" :aria-busy="busy">
    <template v-if="!editing">
      <div class="memo-row-main">
        <button class="memo-complete" type="button" :disabled="busy" :aria-label="(memo.done ? '恢复事项：' : '完成事项：') + memo.title" :aria-pressed="memo.done" @click="run(() => handleToggleMemo(memo))"><Check /></button>
        <div class="memo-row-copy">
          <div class="memo-row-title"><Pin v-if="memo.pinned" class="memo-pin-mark" aria-label="已置顶" /><h3>{{ memo.title }}</h3></div>
          <p class="memo-row-meta"><span>{{ memo.tag }}</span><span v-if="character">{{ character }}</span><span :class="{ 'is-urgent': timing.urgent && !memo.done }">{{ timing.text }}</span><span v-if="steps.total">{{ steps.done }}/{{ steps.total }} 步</span></p>
          <button v-if="memo.note" type="button" class="memo-note-preview" :aria-expanded="expanded" :aria-controls="'memo-note-' + memo.id" @click="expanded = !expanded"><span>{{ preview }}</span><ChevronDown :class="{ 'is-open': expanded }" /></button>
        </div>
        <div class="memo-row-actions">
          <button type="button" :disabled="busy" :aria-label="(memo.pinned ? '取消置顶：' : '置顶事项：') + memo.title" :title="memo.pinned ? '取消置顶' : '置顶，最多 3 条'" :aria-pressed="!!memo.pinned" @click="run(() => handleToggleMemoPin(memo))"><Pin /></button>
          <button ref="editButton" type="button" :aria-label="'编辑事项：' + memo.title" title="编辑事项" @click="emit('edit')"><Pencil /><span>编辑</span></button>
        </div>
      </div>
      <div v-if="memo.has_progress" class="memo-row-progress">
        <div class="memo-progress-track" role="progressbar" :aria-label="memo.title + '完成次数'" :aria-valuemin="0" :aria-valuenow="memo.current_progress" :aria-valuemax="memo.target_progress"><span :style="{ width: progress + '%' }" /></div>
        <span class="memo-progress-value">{{ memo.current_progress }}<small> / {{ memo.target_progress }}</small></span>
        <button type="button" :disabled="busy || memo.current_progress <= 0" :aria-label="'减少次数：' + memo.title" @click="run(() => handleProgressChange(memo, -1))"><Minus /></button>
        <button type="button" :disabled="busy || memo.current_progress >= memo.target_progress" :aria-label="'增加次数：' + memo.title" @click="run(() => handleProgressChange(memo, 1))"><Plus /></button>
      </div>
      <div v-if="expanded && memo.note" :id="'memo-note-' + memo.id" class="memo-note-detail">
        <MemoNoteBody :note="memo.note" :disabled="busy" @toggle-step="run(() => handleToggleMemoStep(memo, $event))" />
        <button v-if="/\d{2,4}[,\s，]+\d{2,4}/.test(memo.note)" class="memo-copy-coordinates" type="button" @click="copyMemoCoordinates(memo.note!)"><Copy />复制坐标</button>
        <p class="memo-created">记录于 {{ memo.created_at.slice(0, 16).replace('T', ' ') }}</p>
      </div>
    </template>
    <template v-else>
      <MemoEditor :memo="memo" @saved="closeEditor" @cancel="closeEditor" />
      <div class="memo-delete-line">
        <template v-if="deleteArmed"><span>永久删除这条记事？</span><button type="button" :disabled="busy" @click="run(() => handleDeleteMemo(memo.id))">确认删除</button><button type="button" @click="deleteArmed = false">保留</button></template>
        <button v-else type="button" @click="deleteArmed = true"><Trash2 />删除记事</button>
      </div>
    </template>
    <p v-if="error" class="memo-row-error" role="alert">{{ error }}</p>
  </article>
</template>

<style scoped>
.memo-row { padding: 20px 2px; border-bottom: 1px solid var(--color-hairline); min-width: 0; }
.memo-row-main { display: flex; align-items: flex-start; gap: 14px; }
.memo-complete { display: grid; place-items: center; width: 19px; height: 19px; margin-top: 3px; flex-shrink: 0; border: 1px solid var(--color-faint); border-radius: 50%; color: transparent; cursor: pointer; }
.memo-complete:hover { color: var(--color-faint); border-color: var(--color-beacon); }
.memo-complete svg { width: 12px; height: 12px; }
.memo-row-copy { flex: 1; min-width: 0; }
.memo-row-title { display: flex; align-items: center; gap: 8px; min-width: 0; }
.memo-row-title h3 { font-size: 15px; line-height: 1.55; font-weight: 500; overflow-wrap: anywhere; user-select: text; }
.memo-pin-mark { width: 12px; height: 12px; flex-shrink: 0; color: var(--color-beacon); }
.memo-row-meta { display: flex; flex-wrap: wrap; gap: 5px 0; margin-top: 6px; font-size: 12px; color: var(--color-faint); font-variant-numeric: tabular-nums; }
.memo-row-meta > span + span::before { content: '·'; padding: 0 8px; color: var(--color-faint); }
.memo-row-meta .is-urgent { color: var(--color-beacon); }
.memo-row-actions { display: flex; gap: 4px; flex-shrink: 0; }
.memo-row-actions button, .memo-row-progress button { display: inline-flex; align-items: center; justify-content: center; gap: 5px; min-width: 28px; height: 28px; padding: 4px 6px; border-radius: 4px; font-size: 12px; color: var(--color-faint); cursor: pointer; }
.memo-row-actions button:hover, .memo-row-progress button:hover { color: var(--color-ink); background: var(--color-raised); }
.memo-row-actions svg, .memo-row-progress svg { width: 14px; height: 14px; }
.memo-row-actions [aria-pressed="true"] { color: var(--color-beacon); }
button:disabled { opacity: .4; cursor: default; }
.memo-note-preview { display: flex; align-items: center; gap: 12px; max-width: 100%; margin-top: 9px; color: var(--color-muted); font-size: 13px; text-align: left; cursor: pointer; }
.memo-note-preview > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.memo-note-preview > svg { width: 13px; height: 13px; flex-shrink: 0; transition: transform .15s; }
.memo-note-preview .is-open { transform: rotate(180deg); }
.memo-row-progress { display: flex; align-items: center; gap: 8px; max-width: 360px; margin: 12px 0 0 33px; }
.memo-progress-track { flex: 1; height: 3px; min-width: 40px; background: var(--color-raised); overflow: hidden; }
.memo-progress-track > span { display: block; height: 100%; background: var(--color-beacon); }
.memo-progress-value { min-width: 58px; text-align: right; font-size: 13px; color: var(--color-ink); font-variant-numeric: tabular-nums; }
.memo-progress-value small { font-size: 12px; color: var(--color-faint); }
.memo-note-detail { margin: 12px 0 0 33px; min-width: 0; }
.memo-copy-coordinates { display: inline-flex; align-items: center; gap: 5px; padding: 8px 0; color: var(--color-beacon); font-size: 12px; cursor: pointer; }
.memo-copy-coordinates svg { width: 13px; height: 13px; }
.memo-created { color: var(--color-faint); font-size: 12px; font-variant-numeric: tabular-nums; margin-top: 8px; }
.is-done .memo-complete { color: var(--color-done); background: color-mix(in srgb, var(--color-done) 12%, transparent); border-color: var(--color-done); }
.is-done h3 { color: var(--color-faint); text-decoration: line-through; }
.is-editing { padding-left: 33px; }
.memo-delete-line { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; margin-top: 16px; padding-top: 12px; border-top: 1px solid var(--color-hairline); font-size: 12px; color: var(--color-faint); }
.memo-delete-line button { display: inline-flex; align-items: center; gap: 6px; min-height: 28px; cursor: pointer; }
.memo-delete-line button:first-of-type { color: var(--color-alert); }
.memo-delete-line svg { width: 13px; height: 13px; }
.memo-row-error { color: var(--color-alert); font-size: 12px; margin: 10px 0 0 33px; }
@container (max-width: 680px) {
  .memo-row-actions button span { display: none; }
  .memo-row-main { gap: 10px; }
  .is-editing { padding-left: 0; }
}
</style>
