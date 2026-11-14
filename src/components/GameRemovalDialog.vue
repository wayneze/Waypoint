<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { X, Trash2 } from '@lucide/vue';
import { previewLibraryRemoval, removeLibraryGame, type LibraryRemovalFile, type LibraryRemovalTarget } from '../services/api';
import { kvFlush } from '../services/kv';
import { progressFlush } from '../services/progress';
import { removingGame } from '../workspace/guides/library';
import { metadataBusyIds, metadataJob } from '../workspace/guides/local';
import { prepareGameRemoval } from '../workspace/guides/extras';
const dialog = ref<HTMLDialogElement>();
const files = ref<LibraryRemovalFile[]>([]);
const target = ref<LibraryRemovalTarget>();
const deleteGame = ref(false), deleteGuides = ref(false), confirming = ref(false), busy = ref(false), loading = ref(false), error = ref('');
let generation = 0;
watch(removingGame, async game => {
  const ticket = ++generation;
  error.value = ''; files.value = []; target.value = undefined; confirming.value = false;
  deleteGame.value = deleteGuides.value = false;
  if (!game) { dialog.value?.close(); return; }
  loading.value = true;
  await nextTick(); dialog.value?.showModal();
  try {
    const input = { record_id: game.record_id, guide_id: game.id, guide_path: game.file_path };
    const plan = await previewLibraryRemoval(input);
    if (ticket === generation) { files.value = plan.files; target.value = input; }
  } catch (cause) { if (ticket === generation) error.value = String(cause); }
  finally { if (ticket === generation) loading.value = false; }
});
const selected = computed(() => files.value.filter(file => file.kind === 'game' ? deleteGame.value : deleteGuides.value));
const gamePaths = computed(() => files.value.filter(f => f.kind === 'game'));
const guidePaths = computed(() => files.value.filter(f => f.kind === 'guide'));
const matching = computed(() => metadataJob.value.running || (!!removingGame.value?.record_id && metadataBusyIds.value.includes(removingGame.value.record_id)));
function close() { if (!busy.value) removingGame.value = null; }
async function submit() {
  if (!target.value || busy.value || matching.value) return;
  if (!confirming.value) { confirming.value = true; return; }
  busy.value = true; error.value = '';
  let release: (() => void) | undefined;
  try {
    release = await prepareGameRemoval([target.value.guide_id, target.value.record_id].filter((id): id is string => !!id));
    await Promise.all([kvFlush(), progressFlush()]);
    await removeLibraryGame(target.value, selected.value);
    window.location.reload();
  } catch (cause) { release?.(); error.value = String(cause); busy.value = false; confirming.value = false; }
}
</script>
<template>
  <dialog ref="dialog" class="removal" aria-labelledby="removal-title" @keydown.esc.stop @cancel.prevent="close">
    <header><div><p>管理书库</p><h2 id="removal-title">{{ confirming ? '确认删除范围' : '删除作品' }}</h2></div><button :disabled="busy" aria-label="关闭删除作品" @click="close"><X /></button></header>
    <div class="removal-body">
      <h3>{{ removingGame?.title }}</h3>
      <p>默认只从 Waypoint 书库移除作品、收藏资料和阅读记录，保留磁盘文件。保留的攻略不会在同步时自动重新加入。</p>
      <p v-if="loading" role="status">正在核对关联文件…</p>
      <fieldset :disabled="busy || confirming || loading">
        <label><input v-model="deleteGame" type="checkbox" :disabled="!gamePaths.length || gamePaths.some(f => f.blocked)" />同时删除游戏文件</label>
        <p v-if="!gamePaths.length" class="hint">没有可删除的本地安装目录。</p>
        <p v-for="file in gamePaths" :key="file.path" class="path">{{ file.path }}<small v-if="file.blocked">{{ file.blocked }}</small></p>
        <label><input v-model="deleteGuides" type="checkbox" :disabled="!guidePaths.length || guidePaths.some(f => f.blocked)" />同时删除攻略文件</label>
        <p v-if="!guidePaths.length" class="hint">没有关联的本地 Markdown 攻略。</p>
        <p v-for="file in guidePaths" :key="file.path" class="path">{{ file.path }}<small v-if="file.blocked">{{ file.blocked }}</small></p>
      </fieldset>
      <p v-if="deleteGame" class="warning">整个游戏目录及其中的存档、补丁和攻略都会随目录移除。目录外的存档不受影响。</p>
      <p v-if="selected.length">所选文件优先移入 Windows 回收站；无法回收时 Windows 会额外确认。失败时保留书库记录并显示原因。</p>
      <p v-if="confirming" class="warning">{{ selected.length ? `将移除作品并处理上方勾选的 ${selected.length} 个文件或目录。` : '仅移除软件内记录，不删除游戏和攻略文件。' }}</p>
      <p v-if="matching" role="status">资料匹配正在运行，保存结束后可删除。</p>
      <p v-if="error" role="alert" class="warning">{{ error }}</p>
    </div>
    <footer><button :disabled="busy" @click="confirming ? confirming = false : close()">{{ confirming ? '返回调整' : '取消' }}</button><button class="danger" :disabled="!target || loading || busy || matching" @click="submit"><Trash2 />{{ busy ? '正在移除…' : confirming ? '确认删除' : '检查删除范围' }}</button></footer>
  </dialog>
</template>
<style scoped>
.removal{position:fixed;inset:0;margin:auto;width:620px;max-width:calc(100vw - 32px);max-height:85vh;padding:0;background:var(--color-canvas);color:var(--color-ink);border:1px solid var(--color-hairline);border-radius:8px}.removal[open]{display:flex;flex-direction:column}.removal::backdrop{background:#101410bb}header,footer{display:flex;align-items:center;justify-content:space-between;padding:18px 24px;border-bottom:1px solid var(--color-hairline)}header p{font-size:11px;color:var(--color-muted)}h2{font:24px var(--font-display);margin-top:5px}h3{font-size:17px;margin-bottom:14px}.removal-body{padding:20px 24px;overflow:auto;min-height:0}p{font-size:12px;line-height:1.8;color:var(--color-muted);margin-bottom:12px}label{display:flex;align-items:center;gap:10px;margin:22px 0 8px;font-size:14px}.path{font:11px var(--font-mono);overflow-wrap:anywhere;background:var(--color-paper);padding:10px}.path small{display:block;color:var(--color-alert);margin-top:7px}.hint{color:var(--color-faint)}.warning{color:var(--color-alert)}footer{border:0;border-top:1px solid var(--color-hairline);justify-content:end;gap:12px}button{display:inline-flex;align-items:center;gap:7px;padding:8px 12px;border:1px solid var(--color-hairline);border-radius:4px;cursor:pointer;font-size:12px}button:disabled{opacity:.4;cursor:default}.danger{color:var(--color-alert);border-color:var(--color-alert)}svg{width:15px;height:15px}:where(button,input):focus-visible{outline:2px solid var(--color-beacon);outline-offset:3px}
</style>
