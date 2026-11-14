<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Plus } from '@lucide/vue';
import {
  characterList, charQuickAddCategory, charQuickAddInputs, getCharBoardTitle, handleCharQuickAdd,
} from '../../workspace/core';
import type { HomePeriod } from '../../workspace/daily';

const props = defineProps<{ scope: string; period: HomePeriod; open: boolean }>();
const emit = defineEmits<{ close: []; saved: [period: 'daily' | 'weekly'] }>();
const owner = ref('');
const text = ref('');
const category = ref<'daily' | 'weekly'>('daily');
const saving = ref(false);
const error = ref('');
const input = ref<HTMLInputElement>();
const target = computed(() => characterList.value.find(char => char.id === owner.value));
watch(() => [props.scope, props.period, props.open] as const, async () => {
  if (props.scope !== 'all' || !target.value) owner.value = props.scope === 'all' ? characterList.value[0]?.id || '' : props.scope;
  category.value = props.period === 'weekly' ? 'weekly' : 'daily';
  if (props.open) { await nextTick(); input.value?.focus(); }
}, { immediate: true });
async function submit() {
  if (saving.value || !text.value.trim() || !target.value) return;
  const id = owner.value;
  const savedCategory = category.value;
  saving.value = true; error.value = '';
  try {
    charQuickAddInputs[id] = text.value;
    charQuickAddCategory[id] = savedCategory;
    await handleCharQuickAdd(id);
    text.value = '';
    emit('saved', savedCategory);
    input.value?.focus();
  } catch { error.value = '未能保存，内容已保留，请重试。'; }
  finally { saving.value = false; }
}
</script>

<template>
  <form v-if="open" class="today-add" aria-label="添加角色待办" @submit.prevent="submit">
    <fieldset :disabled="saving">
      <label><span>角色</span><select v-model="owner" aria-label="待办所属角色" required><option value="" disabled>选择角色</option><option v-for="char in characterList" :key="char.id" :value="char.id">{{ getCharBoardTitle(char) }}</option></select></label>
      <label><span>周期</span><select v-model="category" aria-label="待办周期"><option value="daily">每日</option><option value="weekly">每周</option></select></label>
      <label class="today-add-title"><span>事项</span><input ref="input" v-model="text" required aria-label="新待办标题" placeholder="写下要为这个角色做的事…" @keydown.enter="($event.isComposing || !text.trim()) && $event.preventDefault()" @keydown.esc.stop="emit('close')" /></label>
      <button type="submit" class="today-button is-primary" :disabled="saving || !target || !text.trim()"><Plus />{{ saving ? '保存中…' : '添加' }}</button><button type="button" class="today-button" @click="emit('close')">收起</button>
    </fieldset>
    <p v-if="error" role="alert">{{ error }}</p>
  </form>
</template>

<style scoped>
.today-add { padding: 14px 0; border-bottom: 1px solid var(--color-hairline); flex-shrink: 0; }
fieldset { display: flex; align-items: end; flex-wrap: wrap; gap: 10px; min-width: 0; }
label { display: grid; gap: 6px; min-width: 0; width: 150px; color: var(--color-muted); font-size: 12px; }
label:nth-child(2) { width: 80px; }
.today-add-title { flex: 1; min-width: 180px; }
input, select { height: 32px; width: 100%; min-width: 0; padding: 5px 8px; border: 1px solid var(--color-hairline); border-radius: 4px; background: var(--color-chrome); color: var(--color-ink); font-size: 13px; }
p { font-size: 12px; color: var(--color-alert); margin-top: 10px; }
@container (max-width: 680px) {
  fieldset { display: grid; grid-template-columns: minmax(90px, 120px) 64px minmax(120px, 1fr) auto auto; gap: 8px; }
  label, label:nth-child(2), .today-add-title { width: auto; min-width: 0; }
}
</style>
