<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { FolderOpen, Search, X } from '@lucide/vue';
import { pickFolder } from '../services/systemApi';
import { cancelGameScan, installationKey, scanGameDirectory, type GameScan, type ScannedGame } from '../services/localGames';
import { singleAndGalgameGuides } from '../workspace/guides/library';
import { enrichImportedGames, importScannedGame, libraryImportTargetId, libraryRoots, metadataJob, metadataSource, saveLibraryRoots, showLibraryImport, showMetadataBatch, suggestedLibraryGame } from '../workspace/guides/local';
import type { GameRecord } from '../types';

const dialog = ref<HTMLDialogElement>();
const root = ref('');
const scanning = ref(false);
const saving = ref(false);
const error = ref('');
const receipt = ref('');
const result = ref<GameScan>();
const category = ref<GameRecord['category']>('galgame');
type ImportRow = ScannedGame & { selected: boolean; target: string; manualTarget: boolean };
const rows = ref<ImportRow[]>([]);
const selected = computed(() => rows.value.filter(r => r.selected));
const targetGame = computed(() => singleAndGalgameGuides.value.find(g => g.id === libraryImportTargetId.value || g.record_id === libraryImportTargetId.value));
watch(showLibraryImport, async open => {
  await nextTick();
  if (open) { rows.value = []; result.value = undefined; error.value = ''; receipt.value = ''; root.value = targetGame.value ? '' : libraryRoots.value[0] || ''; dialog.value?.showModal(); }
  else dialog.value?.close();
});
function close() { if (!saving.value && !scanning.value) showLibraryImport.value = false; }
async function chooseFolder() {
  try { const path = await pickFolder('选择游戏库目录'); if (path) root.value = path; } catch (e) { error.value = String(e); }
}
async function scan() {
  if (!root.value.trim() || scanning.value) return;
  scanning.value = true; error.value = ''; receipt.value = ''; rows.value = []; result.value = undefined;
  try {
    const path = root.value.trim();
    result.value = await scanGameDirectory(path);
    rows.value = result.value.games.map(game => ({ ...game, selected: !targetGame.value || result.value!.games.length === 1, target: targetGame.value?.id || suggestedLibraryGame(game)?.id || '', manualTarget: false }));
    if (!libraryRoots.value.some(p => installationKey(p) === installationKey(path))) await saveLibraryRoots([...libraryRoots.value, path]);
  } catch (e) { error.value = String(e); }
  finally { scanning.value = false; }
}
function rematch(row: ImportRow) {
  if (!targetGame.value && !row.manualTarget) row.target = suggestedLibraryGame(row)?.id || '';
}
async function forgetRoot(path: string) {
  try { await saveLibraryRoots(libraryRoots.value.filter(p => p !== path)); } catch (e) { error.value = String(e); }
}
async function importGames() {
  if (saving.value || !selected.value.length) return;
  saving.value = true; error.value = ''; receipt.value = '';
  const imported: { id: string; title: string; steam_id?: string }[] = [];
  try {
    for (const game of selected.value) {
      const id = await importScannedGame(game, game.target, category.value);
      const work = singleAndGalgameGuides.value.find(g => g.record_id === id);
      imported.push({ id, title: work?.title || game.title, steam_id: game.steam_id });
      game.target = work?.id || id; game.selected = false;
    }
    receipt.value = targetGame.value ? `已关联 ${imported.length} 个安装。` : `已入库 ${imported.length} 部。正在批量匹配资料，有歧义的项目会集中显示。`;
    showLibraryImport.value = false;
    if (!targetGame.value) showMetadataBatch.value = true;
  } catch (e) { error.value = `已保存 ${imported.length} 项，其余保留供重试。${String(e)}`; }
  finally {
    saving.value = false;
    if (imported.length && !targetGame.value) void enrichImportedGames(imported);
  }
}
</script>

<template>
  <dialog ref="dialog" class="library-import" aria-labelledby="library-import-title" @cancel.prevent="close">
    <header><div><p>{{ targetGame ? targetGame.title : '把故事收进书库' }}</p><h2 id="library-import-title">{{ targetGame ? '关联本地安装' : '添加游戏目录' }}</h2></div><button aria-label="关闭目录导入" :disabled="saving || scanning" @click="close"><X /></button></header>
    <div class="import-body">
      <p class="import-intro">{{ targetGame ? '选择这部作品的安装目录。勾选的安装会加入当前作品，已有封面、资料与记录会保留。' : '先匹配书库中已有的作品与攻略，再补充在线资料。名称不一致时，可在“关联作品”中手动选择；安装与攻略会保留在同一作品下。' }}</p>
      <label v-if="!targetGame" class="import-source">主要资料来源<select v-model="metadataSource" :disabled="saving"><option value="bangumi">Bangumi · 中文资料优先</option><option value="steam">Steam</option><option value="vndb">VNDB</option></select></label>
      <div v-if="libraryRoots.length" class="import-roots"><span v-for="path in libraryRoots" :key="path"><button :disabled="scanning || saving" :title="path" @click="root = path">{{ path }}</button><button :disabled="scanning || saving" :aria-label="'移除目录记录：' + path" title="仅移除目录记录，游戏档案与文件保留" @click="forgetRoot(path)"><X /></button></span></div>
      <form class="import-path" @submit.prevent="scan"><input v-model="root" :disabled="scanning || saving" aria-label="游戏库目录" placeholder="例如 D:\Games 或 \\服务器\游戏库" /><button type="button" :disabled="scanning || saving" @click="chooseFolder"><FolderOpen />选择</button><button :disabled="scanning || saving || !root.trim()"><Search />{{ scanning ? '正在扫描…' : '扫描目录' }}</button><button v-if="scanning" type="button" @click="cancelGameScan()">停止扫描</button></form>
      <p v-if="scanning" class="import-hint" role="status">正在读取目录，网络共享可能需要更久。停止后会保留已发现的结果。</p>
      <p v-if="error" role="alert" class="import-error">{{ error }}</p>
      <p v-if="receipt" role="status" class="import-success">{{ receipt }}</p>
      <template v-if="result">
        <div class="import-summary"><strong>{{ result.complete ? '扫描完成' : '扫描未完成' }} · {{ rows.length }} 部游戏</strong><label v-if="!targetGame">导入类别<select v-model="category" :disabled="saving"><option value="galgame">Galgame</option><option value="single">单机游戏</option><option value="steam">Steam</option></select></label></div>
        <p v-for="warning in result.warnings" :key="warning" class="import-error">{{ warning }}</p>
        <p class="import-hint">可修正文件夹昵称。多启动程序会保留备选；导入不会根据文件夹的“通关”字样修改进度。重扫按安装目录识别已有游戏。</p>
        <label v-if="rows.length" class="import-all"><input type="checkbox" :checked="rows.every(r => r.selected)" :disabled="saving" @change="rows.forEach(r => r.selected = ($event.target as HTMLInputElement).checked)" />选择全部 · 已选 {{ selected.length }}</label>
        <article v-for="row in rows" :key="row.installation.directory" class="import-row">
          <input v-model="row.selected" :disabled="saving" type="checkbox" :aria-label="'导入 ' + row.title" />
          <div><input v-model="row.title" :disabled="saving" class="import-title" aria-label="游戏名称" @change="rematch(row)" /><p :title="row.installation.directory">{{ row.installation.directory }}</p>
            <div class="import-row-options"><label>启动程序<select v-model="row.installation.executable" :disabled="saving"><option v-for="exe in row.installation.candidates" :key="exe" :value="exe">{{ exe.split(/[\\/]/).pop() }}</option></select></label><span v-if="row.installation.candidates.length > 1" class="import-hint">有 {{ row.installation.candidates.length }} 个版本，请确认</span>
              <span v-if="targetGame">关联到：{{ targetGame.title }}</span><label v-else>关联作品<select v-model="row.target" :disabled="saving" @change="row.manualTarget = true"><option value="">新建作品</option><option v-for="work in singleAndGalgameGuides" :key="work.id" :value="work.id">{{ work.title }}{{ work.file_path ? ' · 已有攻略' : '' }}</option></select></label>
            </div>
          </div>
        </article>
        <p v-if="!rows.length" class="import-hint">没有找到可用的游戏启动程序。可以选择更具体的安装目录。</p>
      </template>
    </div>
    <footer><p>{{ metadataJob.running ? `正在补全资料 ${metadataJob.done} / ${metadataJob.total}` : '已有进度、个人封面与评分会保留' }}</p><button :disabled="saving || scanning" @click="close">稍后继续</button><button class="primary" :disabled="saving || scanning || (!targetGame && metadataJob.running) || !selected.length || selected.some(r => !r.title.trim())" @click="importGames">{{ saving ? '正在保存…' : targetGame ? `关联 ${selected.length} 个安装` : `入库 ${selected.length} 部游戏` }}</button></footer>
  </dialog>
</template>

<style scoped>
.import-source{display:flex;align-items:center;gap:12px;font-size:12px;margin-bottom:20px}
.library-import{width:min(1060px,94vw);max-height:90dvh;padding:0;border:1px solid var(--color-hairline);border-radius:10px;background:var(--color-canvas);color:var(--color-ink);margin:auto;box-shadow:0 24px 100px #0008}.library-import[open]{display:flex;flex-direction:column}.library-import::backdrop{background:#080c0bd9;backdrop-filter:blur(5px)}
header,footer{display:flex;align-items:center;gap:16px;padding:20px 26px;flex-shrink:0}header{justify-content:space-between;border-bottom:1px solid var(--color-hairline)}header p{font-size:11px;color:var(--color-beacon)}h2{font:26px var(--font-display);margin-top:5px}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer;border:1px solid var(--color-hairline);padding:7px 11px;border-radius:4px;font-size:12px;white-space:nowrap}button:hover{background:var(--color-paper)}button:disabled{opacity:.4;cursor:default}svg{width:16px;height:16px}.import-body{overflow:auto;min-height:0;padding:22px 26px}.import-intro{font-size:13px;color:var(--color-muted);margin-bottom:16px}.import-path{display:flex;gap:8px}.import-path>input{flex:1;min-width:100px;font-family:var(--font-mono)}input:not([type=checkbox]),select{border:1px solid var(--color-hairline);padding:8px 10px;background:var(--color-paper);border-radius:4px;font-size:12px;color:var(--color-ink);min-width:0}input:focus,select:focus{outline:1px solid var(--color-beacon)}.import-roots{display:flex;gap:8px;flex-wrap:wrap;margin-bottom:14px}.import-roots>span{display:flex;max-width:100%}.import-roots button:first-child{overflow:hidden;text-overflow:ellipsis;display:block;max-width:380px;font-size:11px}.import-roots button:last-child{padding:5px}.import-hint{font-size:11px;color:var(--color-faint);line-height:1.7;margin:10px 0}.import-summary{display:flex;justify-content:space-between;align-items:center;margin-top:24px}.import-summary strong{font-weight:500;font-size:15px}.import-summary label{font-size:12px;display:flex;gap:8px;align-items:center}.import-all{display:flex;gap:8px;align-items:center;font-size:12px;padding:12px 0;border-bottom:1px solid var(--color-hairline)}.import-row{display:flex;gap:14px;padding:18px 0;border-bottom:1px solid var(--color-hairline)}.import-row>input{margin-top:10px;flex-shrink:0}.import-row>div{flex:1;min-width:0}.import-title{width:100%;font-size:14px!important}.import-row p{font-size:10px;color:var(--color-faint);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;margin:7px 0}.import-row-options{display:flex;flex-wrap:wrap;gap:8px 16px;align-items:center}.import-row-options label{font-size:11px;color:var(--color-muted);display:flex;align-items:center;gap:8px;max-width:100%}.import-row-options select{max-width:280px}.import-error{color:var(--color-alert);font-size:12px;margin:12px 0;overflow-wrap:anywhere}.import-success{color:var(--color-beacon);font-size:13px;margin:14px 0}footer{border-top:1px solid var(--color-hairline)}footer p{flex:1;font-size:11px;color:var(--color-faint)}.primary{background:var(--color-beacon);color:var(--color-canvas);border-color:var(--color-beacon)}@media(max-width:650px){header,footer,.import-body{padding:16px}.import-path{flex-wrap:wrap}.import-path>input{flex-basis:100%}.import-row-options label{width:100%}.import-row-options select{flex:1}footer{flex-wrap:wrap}footer p{flex-basis:100%}}
</style>
