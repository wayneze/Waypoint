<script setup lang="ts">
import { nextTick, reactive, ref, useId, watch } from 'vue';
import { ChevronDown, Plus } from '@lucide/vue';
import type { CustomMemo } from '../../types';
import { availableMemoTags, createMemo, handleUpdateMemo, type MemoInput } from '../../workspace/memos';
import { characterList, getShortCharName } from '../../workspace/core';
import { formatWeekdays, parseWeekdays, WEEKDAY_LABELS } from '../../services/memoNote';

const props = defineProps<{ memo?: CustomMemo; defaultTag?: string }>();
const emit = defineEmits<{ saved: [tag: string]; cancel: [] }>();
const id = useId();
const titleInput = ref<HTMLInputElement>();
const form = ref<HTMLFormElement>();
const expanded = ref(!!props.memo);
const saving = ref(false);
const error = ref('');
const draft = reactive({
  title: props.memo?.title || '',
  tag: props.memo?.tag || props.defaultTag || '通用',
  cycle_type: props.memo?.cycle_type || 'once' as CustomMemo['cycle_type'],
  cycle_val: props.memo?.cycle_val || '',
  character_id: props.memo?.character_id || '',
  note: props.memo?.note || '',
  has_progress: props.memo?.has_progress || false,
  target_progress: props.memo?.target_progress || 5,
  weekdays: parseWeekdays(props.memo?.cycle_type === 'weekly' ? props.memo.cycle_val : ''),
});
const cycles = [
  { id: 'once', label: '一次性事项' }, { id: 'daily', label: '每日重复' },
  { id: 'weekly', label: '每周重复' }, { id: 'interval', label: '间隔重复' },
  { id: 'deadline', label: '设定截止日' },
] as const;
watch(() => props.defaultTag, tag => {
  if (!props.memo && !draft.title && !draft.note) draft.tag = tag || '通用';
});
watch(() => draft.cycle_type, cycle => {
  draft.cycle_val = cycle === 'interval' ? '3d' : '';
});
async function focus() {
  await nextTick();
  titleInput.value?.focus();
}
defineExpose({ focus });
if (props.memo) void focus();

async function submit() {
  if (saving.value) return;
  if (!form.value?.checkValidity()) {
    expanded.value = true;
    await nextTick();
    form.value?.reportValidity();
    return;
  }
  saving.value = true;
  error.value = '';
  try {
    const input: MemoInput = {
      title: draft.title, tag: draft.tag, cycle_type: draft.cycle_type,
      cycle_val: draft.cycle_type === 'weekly' ? formatWeekdays(draft.weekdays)
        : draft.cycle_type === 'interval' || draft.cycle_type === 'deadline' ? draft.cycle_val : '',
      character_id: draft.character_id, note: draft.note,
      has_progress: draft.has_progress, target_progress: draft.target_progress,
    };
    const ok = props.memo ? await handleUpdateMemo(props.memo, input) : await createMemo(input);
    if (!ok) { error.value = '未能保存，请检查提示后重试。'; return; }
    if (!props.memo) {
      draft.title = ''; draft.note = ''; draft.has_progress = false;
      draft.cycle_type = 'once'; draft.cycle_val = ''; draft.weekdays = [];
      draft.character_id = ''; draft.target_progress = 5;
      expanded.value = false;
      void focus();
    }
    emit('saved', input.tag.trim() || '通用');
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <form ref="form" class="memo-editor" novalidate :class="{ 'is-editing': memo, 'is-expanded': expanded }" :aria-label="memo ? '编辑记事' : '快速记录'" @submit.prevent="submit">
    <fieldset :disabled="saving">
      <div class="memo-compose-line">
        <Plus v-if="!memo" class="memo-compose-mark" aria-hidden="true" />
        <input ref="titleInput" v-model="draft.title" class="memo-title-input" required maxlength="500"
          :aria-label="memo ? '修改记事标题' : '记事标题'" :placeholder="memo ? '事项标题' : '记一笔，留给之后的自己…'"
          @keydown.enter="($event.isComposing || !draft.title.trim()) && $event.preventDefault()" @keydown.esc.stop="memo && emit('cancel')" />
        <button v-if="!memo" type="button" class="memo-options-toggle" :aria-expanded="expanded" :aria-controls="id + '-options'" @click="expanded = !expanded">
          <span>{{ expanded ? '收起选项' : '更多选项' }}</span><ChevronDown :class="{ 'is-open': expanded }" />
        </button>
        <button v-if="!memo" type="submit" class="memo-save" :disabled="!draft.title.trim() || saving">{{ saving ? '保存中…' : '记下' }}</button>
      </div>
      <div v-show="expanded" :id="id + '-options'" class="memo-editor-options">
        <div class="memo-fields">
          <label :for="id + '-tag'">分类<input :id="id + '-tag'" v-model="draft.tag" :list="id + '-tags'" placeholder="选择或输入分类" maxlength="50" /></label>
          <datalist :id="id + '-tags'"><option v-for="tag in availableMemoTags" :key="tag" :value="tag" /></datalist>
          <label :for="id + '-character'">关联角色<select :id="id + '-character'" v-model="draft.character_id">
            <option value="">不关联角色</option><option v-for="char in characterList" :key="char.id" :value="char.id">{{ getShortCharName(char) }}</option>
          </select></label>
          <label :for="id + '-cycle'">重复与日期<select :id="id + '-cycle'" v-model="draft.cycle_type">
            <option v-for="cycle in cycles" :key="cycle.id" :value="cycle.id">{{ cycle.label }}</option>
          </select></label>
        </div>
        <div v-if="draft.cycle_type === 'weekly'" class="memo-weekdays" role="group" aria-label="每周执行日">
          <span>执行日</span><label v-for="day in [1, 2, 3, 4, 5, 6, 0]" :key="day"><input v-model="draft.weekdays" type="checkbox" :value="day" />周{{ WEEKDAY_LABELS[day] }}</label><small>不选则整周可做</small>
        </div>
        <label v-if="draft.cycle_type === 'interval'" class="memo-cycle-value" :for="id + '-interval'">重复间隔
          <input :id="id + '-interval'" v-model="draft.cycle_val" required pattern="(?:[1-9]\d*(?:\.\d+)?|0\.\d*[1-9]\d*)[dh]" placeholder="例如 3d 或 48h" title="使用正数加 d（天）或 h（小时），例如 3d、48h" /><small>d 为天，h 为小时</small>
        </label>
        <label v-if="draft.cycle_type === 'deadline'" class="memo-cycle-value" :for="id + '-deadline'">截止日期<input :id="id + '-deadline'" v-model="draft.cycle_val" type="date" required /></label>
        <label class="memo-note-field" :for="id + '-note'">备注与步骤<textarea :id="id + '-note'" v-model="draft.note" rows="3" placeholder="写下细节、坐标或链接。用 - [ ] 写一条可勾选的步骤。" /></label>
        <div class="memo-editor-bottom">
          <label class="memo-count-toggle"><input v-model="draft.has_progress" type="checkbox" />按次数记录</label>
          <label v-if="draft.has_progress" class="memo-target" :for="id + '-target'">目标<input :id="id + '-target'" v-model.number="draft.target_progress" type="number" required min="1" max="9999" step="1" />次</label>
          <div v-if="memo" class="memo-edit-actions"><button type="button" @click="emit('cancel')">取消</button><button class="memo-save" type="submit" :disabled="!draft.title.trim() || saving">{{ saving ? '保存中…' : '保存修改' }}</button></div>
        </div>
      </div>
    </fieldset>
    <p v-if="error" class="memo-editor-error" role="alert">{{ error }}</p>
  </form>
</template>

<style scoped>
.memo-editor { border-bottom: 1px solid var(--color-hairline); }
.memo-editor fieldset { min-width: 0; }
.memo-compose-line { display: flex; align-items: center; gap: 12px; padding: 14px 0; }
.memo-compose-mark { width: 18px; height: 18px; color: var(--color-faint); flex-shrink: 0; }
.memo-title-input { flex: 1; min-width: 0; padding: 7px 0; font-size: 14px; color: var(--color-ink); background: transparent; }
.memo-title-input::placeholder { color: var(--color-faint); }
.memo-title-input:focus-visible { outline-offset: 4px; }
.memo-options-toggle, .memo-edit-actions > button { display: inline-flex; align-items: center; gap: 5px; min-height: 32px; padding: 5px 10px; border-radius: 4px; font-size: 12px; color: var(--color-muted); cursor: pointer; white-space: nowrap; }
.memo-options-toggle svg { width: 14px; height: 14px; transition: transform .15s; }
.memo-options-toggle .is-open { transform: rotate(180deg); }
.memo-options-toggle:hover, .memo-edit-actions > button:hover { background: var(--color-raised); color: var(--color-ink); }
.memo-save, .memo-edit-actions > .memo-save { min-height: 32px; padding: 5px 14px; border: 1px solid color-mix(in srgb, var(--color-beacon) 35%, transparent); border-radius: 4px; background: color-mix(in srgb, var(--color-beacon) 10%, transparent); color: var(--color-beacon); font-size: 12px; cursor: pointer; white-space: nowrap; }
.memo-save:disabled { opacity: .4; cursor: default; }
.memo-editor-options { display: grid; gap: 16px; padding: 4px 0 20px 30px; }
.memo-fields { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
.memo-fields label, .memo-note-field { display: grid; gap: 7px; font-size: 12px; color: var(--color-muted); min-width: 0; }
.memo-fields :is(input, select), textarea, .memo-cycle-value input, .memo-target input { width: 100%; min-width: 0; border: 1px solid var(--color-hairline); border-radius: 4px; background: var(--color-chrome); color: var(--color-ink); padding: 6px 9px; font-size: 13px; }
.memo-fields :is(input, select) { height: 34px; }
textarea { resize: vertical; min-height: 84px; max-height: 320px; line-height: 1.6; }
textarea::placeholder { color: var(--color-faint); }
.memo-weekdays, .memo-cycle-value, .memo-editor-bottom, .memo-target, .memo-count-toggle { display: flex; align-items: center; flex-wrap: wrap; gap: 12px; font-size: 12px; color: var(--color-muted); }
.memo-weekdays label, .memo-count-toggle { display: inline-flex; align-items: center; gap: 6px; cursor: pointer; }
.memo-weekdays small, .memo-cycle-value small { font-size: 12px; color: var(--color-faint); }
.memo-cycle-value input { width: 180px; }
.memo-target { gap: 7px; }
.memo-target input { width: 76px; font-variant-numeric: tabular-nums; }
.memo-edit-actions { display: flex; align-items: center; gap: 6px; margin-left: auto; }
.memo-editor-error { padding: 0 0 12px 30px; color: var(--color-alert); font-size: 12px; }
.is-editing { border: 0; }
.is-editing .memo-compose-line { padding-top: 0; }
.is-editing .memo-title-input { border-bottom: 1px solid var(--color-hairline); font-weight: 600; }
.is-editing .memo-editor-options { padding-left: 0; padding-bottom: 0; }
@container (max-width: 680px) {
  .memo-fields { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .memo-fields > label:last-child { grid-column: 1 / -1; }
  .memo-compose-line { gap: 8px; }
  .memo-editor-options { padding-left: 0; }
}
</style>
