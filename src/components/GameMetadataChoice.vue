<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import type { CoverPosition, GameMetadata } from '../types';
import { loadGameMetadata } from '../services/localGames';
import { gameRecords } from '../workspace/core';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import { applyMetadata, metadataBusyIds } from '../workspace/guides/local';
import GameCoverEditor from './GameCoverEditor.vue';
import { isCoverSource } from '../services/gameCoverImage';

const props = defineProps<{ gameId: string; candidate: GameMetadata }>();
const workId = computed(() => singleAndGalgameGuides.value.find(item => item.id === props.gameId || item.record_id === props.gameId)?.record_id || props.gameId);
const emit = defineEmits<{ saved: []; cancel: [] }>();
const metadata = ref<GameMetadata>();
const poster = ref('');
const position = ref<CoverPosition>({ x: 50, y: 50 });
const loading = ref(false), saving = ref(false), importing = ref(false), error = ref('');
let generation = 0;
onBeforeUnmount(() => { generation++; });
watch(() => [props.gameId, props.candidate] as const, async () => {
  const ticket = ++generation;
  const game = singleAndGalgameGuides.value.find(item => item.id === props.gameId || item.record_id === props.gameId);
  const record = gameRecords.value.find(item => item.id === (game?.record_id || props.gameId));
  poster.value = record?.poster || '';
  position.value = { ...(record?.poster_position || { x: 50, y: 50 }) };
  loading.value = true; error.value = ''; metadata.value = undefined;
  try {
    const full = await loadGameMetadata(props.candidate);
    if (ticket === generation) metadata.value = full;
  } catch (cause) { if (ticket === generation) error.value = String(cause); }
  finally { if (ticket === generation) loading.value = false; }
}, { immediate: true });
async function save() {
  if (!metadata.value || saving.value || importing.value || metadataBusyIds.value.includes(workId.value)) return;
  const id = workId.value;
  if (poster.value.trim() && !isCoverSource(poster.value.trim())) { error.value = '请选择图片或填写有效的图片网址'; return; }
  saving.value = true; error.value = '';
  metadataBusyIds.value.push(id);
  try { await applyMetadata(id, metadata.value, { poster: poster.value, poster_position: { ...position.value } }); emit('saved'); }
  catch (cause) { error.value = String(cause); }
  finally { saving.value = false; metadataBusyIds.value = metadataBusyIds.value.filter(value => value !== id); }
}
</script>
<template>
  <section class="metadata-choice">
    <h3>确认《{{ candidate.title }}》的资料与封面</h3>
    <p v-if="loading" role="status">正在读取完整资料与在线封面…</p>
    <fieldset :disabled="saving"><GameCoverEditor v-if="metadata" v-model="poster" v-model:position="position" :automatic="metadata.cover" :title="metadata.original_title || metadata.title" @busy="importing = $event" /></fieldset>
    <p v-if="error" role="alert">{{ error }}</p>
    <div class="choice-actions"><button type="button" :disabled="saving || importing" @click="emit('cancel')">返回候选</button><button type="button" :disabled="!metadata || saving || importing || metadataBusyIds.includes(workId)" @click="save">{{ saving ? '保存中…' : '确认资料与封面' }}</button></div>
  </section>
</template>
<style scoped>
.metadata-choice{border:1px solid var(--color-hairline);padding:20px;margin-top:16px;border-radius:6px;background:var(--color-paper)}h3{font-size:16px;margin-bottom:18px}p{font-size:12px;margin-block:14px;color:var(--color-muted)}[role=alert]{color:var(--color-alert)}.choice-actions{display:flex;justify-content:end;gap:12px;margin-top:18px}button{padding:9px 14px;border:1px solid var(--color-hairline);border-radius:4px;font-size:12px;cursor:pointer}button:last-child{background:var(--color-beacon);color:var(--color-canvas)}button:disabled{opacity:.4;cursor:default}
</style>
