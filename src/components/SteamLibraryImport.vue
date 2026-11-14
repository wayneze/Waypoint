<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Check, Download, RefreshCw, Search, X } from '@lucide/vue';
import { isTauri } from '../services/api';
import { duration, timeLabel } from '../services/gameActivity';
import { buildSteamRows, needsSteamTitle, selectSteamRows, type SteamFilter, type SteamInstallFilter, type SteamSort } from '../services/steamImport';
import { gameRecords, currentTab, vaultList } from '../workspace/core';
import { libraryGuides } from '../workspace/guides/library';
import { steamLibrary, steamLibraryRefreshingId, syncSteamAccount, selectVaultGroup } from '../workspace/vault';
import { activityError, fillSteamNames, loadGameActivity, runSteamBatch, showSteamImport, steamActivity, steamBatch, steamImportAccountId, steamImportTargetId, steamNames, stopSteamBatch, stopSteamNames } from '../workspace/gameActivity';
import { posterSrc } from './sky/wikiImage';

const dialog = ref<HTMLDialogElement>();
const accountId = steamImportAccountId;
const query = ref('');
const selected = ref<number[]>([]);
const targets = ref<Record<number, string>>({});
const filter = ref<SteamFilter>('all');
const installation = ref<SteamInstallFilter>('all');
const sort = ref<SteamSort>('library');
const onlySelected = ref(false);
const page = ref(1);
const pageSize = 12;
const error = ref('');
const notice = ref('');
const brokenIcons = ref<number[]>([]);
const targetRecord = computed(() => gameRecords.value.find(r => r.id === steamImportTargetId.value));
const accounts = computed(() => vaultList.value.filter(v => v.platform.toLowerCase().includes('steam')));
const games = computed(() => steamLibrary.value.filter(g => g.vault_item_id === accountId.value));
const rows = computed(() => buildSteamRows(games.value, gameRecords.value, libraryGuides.value, steamActivity.value));
const filtered = computed(() => selectSteamRows(rows.value, query.value, filter.value, installation.value, sort.value, onlySelected.value, selected.value));
const pageCount = computed(() => Math.max(1, Math.ceil(filtered.value.length / pageSize)));
const visible = computed(() => filtered.value.slice((page.value - 1) * pageSize, page.value * pageSize));
const chosen = computed(() => rows.value.filter(r => selected.value.includes(r.game.appid)));
const pending = computed(() => chosen.value.filter(r => !r.linked));
const undecided = computed(() => targetRecord.value ? [] : pending.value.filter(r => !r.record && r.suggestions.length && !targets.value[r.game.appid]));
const hiddenSelected = computed(() => chosen.value.filter(r => !filtered.value.some(f => f.game.appid === r.game.appid)).length);
const allVisibleSelected = computed(() => visible.value.length > 0 && visible.value.every(r => selected.value.includes(r.game.appid)));
const someVisibleSelected = computed(() => visible.value.some(r => selected.value.includes(r.game.appid)));
const cache = computed(() => steamActivity.value.accounts.find(a => a.account_id === accountId.value));
const ownJob = computed(() => steamBatch.value.accountId === accountId.value);
const counts = computed(() => ({ library: rows.value.filter(r => r.inLibrary).length, new: rows.value.filter(r => !r.inLibrary).length, unlinked: rows.value.filter(r => !r.linked).length }));
watch([query, filter, installation, sort, onlySelected], () => { page.value = 1; });
watch(pageCount, count => { page.value = Math.min(page.value, count); });
watch(accountId, () => { selected.value = []; targets.value = {}; page.value = 1; notice.value = ''; error.value = ''; });
watch(games, items => { selected.value = selected.value.filter(id => items.some(g => g.appid === id)); });
watch(showSteamImport, async open => {
  await nextTick();
  if (!open) { dialog.value?.close(); return; }
  if (!accounts.value.some(a => a.id === accountId.value)) accountId.value = accounts.value[0]?.id || '';
  selected.value = []; targets.value = {}; onlySelected.value = false;
  filter.value = 'all'; installation.value = 'all';
  query.value = targetRecord.value?.title || '';
  dialog.value?.showModal();
  await loadGameActivity();
  if (showSteamImport.value) void fillSteamNames(visible.value.filter(needsSteamTitle).map(r => r.game.appid));
}, { immediate: true });
function close() { showSteamImport.value = false; }
function manage() { close(); selectVaultGroup('steam'); currentTab.value = 'vault'; }
function toggleVisible() {
  const ids = visible.value.map(r => r.game.appid);
  selected.value = allVisibleSelected.value ? selected.value.filter(id => !ids.includes(id)) : [...new Set([...selected.value, ...ids])];
}
async function sync() {
  const account = accountId.value;
  error.value = ''; notice.value = '';
  try {
    const result = await syncSteamAccount(account);
    await loadGameActivity();
    if (accountId.value !== account) return;
    notice.value = `已更新 ${result.length} 款库存和官方时长。选择需要收进书库的作品。`;
    void fillSteamNames(rows.value.filter(needsSteamTitle).map(r => r.game.appid));
  } catch (e) { if (accountId.value === account) error.value = String(e); await loadGameActivity(); }
}
function submit(importRecords: boolean) {
  error.value = ''; notice.value = '';
  const targetId = targetRecord.value?.id;
  const candidates = importRecords ? pending.value : chosen.value;
  if (targetRecord.value && (candidates.length !== 1 || candidates.some(r => r.record && r.record.id !== targetRecord.value!.id))) {
    error.value = '请选择一款尚未关联其他作品的 Steam 游戏。'; return;
  }
  const choices = candidates.map(r => ({ appid: r.game.appid, record_id: targetRecord.value?.id || r.record?.id || (targets.value[r.game.appid] === 'new' ? undefined : targets.value[r.game.appid]) || undefined,
    title: r.title, guide_id: r.guide?.id }));
  void runSteamBatch(accountId.value, choices, importRecords ? 'import' : 'sync').then(() => {
    if (targetId && targetRecord.value?.id === targetId && !steamBatch.value.errors.length) close();
  }).catch(e => { error.value = String(e); });
}
function selectTarget(appid: number) { selected.value = [appid]; }
function nextPage(delta: number) {
  page.value = Math.min(pageCount.value, Math.max(1, page.value + delta));
  void fillSteamNames(visible.value.filter(needsSteamTitle).map(r => r.game.appid));
}
</script>

<template>
  <dialog ref="dialog" class="steam-import" aria-labelledby="steam-import-title" @cancel.prevent="close" @click="($event.target === dialog) && close()">
    <header><div><h2 id="steam-import-title">{{ targetRecord ? '关联 Steam · ' + targetRecord.title : 'Steam 游戏库' }}</h2><p>{{ targetRecord ? '选择对应的 Steam 游戏，时长与成就会同步到当前作品。可以修改关键词查找。' : '选择入库、关联已有作品，或更新成就。' }}</p></div><button class="icon-button" aria-label="关闭 Steam 游戏库" @click="close"><X /></button></header>
    <div class="connection">
      <label class="account-label"><span>账号</span><select v-model="accountId" :disabled="!!steamLibraryRefreshingId"><option v-if="!accounts.length" value="">先添加 Steam 账号</option><option v-for="a in accounts" :key="a.id" :value="a.id">{{ a.account_name }}</option></select></label>
      <button :disabled="!accountId || !!steamLibraryRefreshingId || steamBatch.running || !isTauri()" @click="sync"><RefreshCw :class="{spinning:!!steamLibraryRefreshingId}" />{{ steamLibraryRefreshingId ? '正在刷新…' : '刷新库存与时长' }}</button>
      <button class="text-button" @click="manage">账号与连接设置</button>
      <small>上次更新 {{ timeLabel(cache?.updated_at) }}</small>
    </div>
    <div class="messages">
      <p v-if="!isTauri()" class="error" role="alert">请在桌面版同步 Steam。</p>
      <p v-if="error || cache?.error || activityError" class="error" role="alert">{{ error || cache?.error || activityError }}</p>
      <p v-if="notice" role="status">{{ notice }}</p>
      <div v-if="steamNames.running || steamNames.failed || steamNames.stopped" class="job">
        <span>{{ steamNames.running ? '正在补全中文名称' : steamNames.stopped ? '中文名称补全已停止' : '中文名称补全结束' }} {{ steamNames.done }} / {{ steamNames.total }}<template v-if="steamNames.failed"> · {{ steamNames.failed }} 项暂未取得</template></span>
        <button v-if="steamNames.running" class="text-button" @click="stopSteamNames">停止后续补全</button><small>可继续筛选和操作</small>
      </div>
      <div v-if="ownJob && (steamBatch.running || steamBatch.total || steamBatch.imported || steamBatch.errors.length)" class="job" role="status">
        <span>{{ steamBatch.running ? '正在处理' : steamBatch.stopped ? '已停止后续任务' : steamBatch.errors.length ? '部分同步失败' : '同步完成' }} · 入库 / 关联 {{ steamBatch.imported }} 部<template v-if="steamBatch.total"> · 成就已处理 {{ steamBatch.done }} / {{ steamBatch.total }}（成功 {{ steamBatch.synced }}）</template><template v-if="steamBatch.current"> · {{ steamBatch.current }}</template></span>
        <button v-if="steamBatch.running" class="text-button" @click="stopSteamBatch">停止后续同步</button><small v-if="steamBatch.running">关闭窗口后仍会继续</small>
      </div>
      <details v-if="ownJob && steamBatch.errors.length" class="errors"><summary>{{ steamBatch.errors.length }} 项未完成，查看原因</summary><p v-for="message in steamBatch.errors" :key="message">{{ message }}</p></details>
    </div>
    <div class="filters">
      <label class="search"><Search /><input v-model="query" type="search" aria-label="搜索 Steam 游戏" placeholder="搜索中文名、原名或 AppID" /></label>
      <label><span>排序</span><select v-model="sort" aria-label="Steam 游戏排序"><option value="library">书库已有优先</option><option value="installed">本机已安装优先</option><option value="recent">最近游玩</option><option value="playtime">累计时长</option><option value="name">名称顺序</option></select></label>
      <label><span>安装</span><select v-model="installation" aria-label="筛选安装状态"><option value="all">全部</option><option value="installed">本机已安装</option><option value="uninstalled">未检测到安装</option></select></label>
    </div>
    <nav class="scope" aria-label="Steam 书库状态筛选">
      <button v-for="item in [{id:'all',label:'全部',count:rows.length},{id:'library',label:'书库已有',count:counts.library},{id:'new',label:'书库未收录',count:counts.new},{id:'unlinked',label:'尚未关联',count:counts.unlinked}]" :key="item.id" :aria-pressed="filter === item.id" @click="filter = item.id as SteamFilter">{{ item.label }}<span>{{ item.count }}</span></button>
      <button class="names-button" :disabled="steamNames.running || !rows.length" @click="fillSteamNames(rows.filter(needsSteamTitle).map(r => r.game.appid), true)">补全中文名称</button>
    </nav>
    <div class="selection"><template v-if="!targetRecord"><label><input type="checkbox" :checked="allVisibleSelected" :indeterminate="someVisibleSelected && !allVisibleSelected" :disabled="!visible.length" @change="toggleVisible" />选择本页</label><button class="text-button" :disabled="!filtered.length" @click="selected = [...new Set([...selected, ...filtered.map(r => r.game.appid)])]">选择全部筛选结果（{{ filtered.length }}）</button></template><label><input v-model="onlySelected" type="checkbox" />只看已选</label><button class="text-button" :disabled="!selected.length" @click="selected = []">清空选择</button></div>
    <div class="games" :aria-busy="!!steamLibraryRefreshingId">
      <article v-for="row in visible" :key="row.game.appid" :class="{chosen:selected.includes(row.game.appid)}">
        <label class="game-check"><input v-if="targetRecord" type="radio" name="steam-target" :checked="selected.includes(row.game.appid)" :disabled="!!row.record && row.record.id !== targetRecord.id" :aria-label="'选择 ' + row.title" @change="selectTarget(row.game.appid)" /><input v-else v-model="selected" :value="row.game.appid" type="checkbox" :aria-label="'选择 ' + row.title" />
          <img v-if="row.game.img_icon_url && !brokenIcons.includes(row.game.appid)" :src="posterSrc(`https://cdn.cloudflare.steamstatic.com/steamcommunity/public/images/apps/${row.game.appid}/${row.game.img_icon_url}.jpg`)" @error="brokenIcons.push(row.game.appid)" alt="" loading="lazy" /><span v-else class="monogram" aria-hidden="true">{{ row.title.slice(0,1) }}</span>
          <span class="game-name"><strong>{{ row.title }}</strong><small v-if="row.original" class="original">{{ row.original }}</small><small>{{ duration(row.game.playtime_forever * 60) }} · {{ row.installed ? '本机已安装' : '未检测到安装' }} · {{ row.game.appid }}</small></span>
        </label>
        <div class="target">
          <span v-if="targetRecord">{{ row.record && row.record.id !== targetRecord.id ? '已属于：' + row.record.title : row.linked ? '已关联当前作品' : '关联到：' + targetRecord.title }}</span>
          <span v-else-if="row.linked" class="linked"><Check />已关联书库</span>
          <span v-else-if="row.record" class="linked">书库已有 · 待关联</span>
          <template v-else><span>{{ row.guide ? '书库已有攻略 · 待入库' : row.suggestions.length ? '发现同名档案 · 请确认' : '书库未收录' }}</span>
            <select v-if="!row.guide || row.suggestions.length" v-model="targets[row.game.appid]" :aria-label="row.title + '的入库方式'">
              <option :value="undefined">{{ row.suggestions.length ? '请选择关联目标' : '新建游玩档案' }}</option><option v-if="row.suggestions.length" value="new">单独新建档案</option>
              <optgroup v-if="row.suggestions.length" label="匹配的已有档案"><option v-for="r in row.suggestions" :key="r.id" :value="r.id">关联：{{ r.title }}</option></optgroup>
              <optgroup label="选择其他已有档案"><option v-for="r in gameRecords.filter(r => !row.suggestions.some(s => s.id === r.id))" :key="r.id" :value="r.id">关联：{{ r.title }}</option></optgroup>
            </select>
          </template>
          <small v-if="cache?.achievements[row.game.appid]?.updated_at">{{ cache.achievements[row.game.appid]!.status === 'unsupported' ? '无 Steam 成就' : `成就 ${cache.achievements[row.game.appid]!.items.filter(a => a.unlocked).length} / ${cache.achievements[row.game.appid]!.items.length}` }}{{ cache.achievements[row.game.appid]!.error ? ' · 上次缓存' : '' }}</small>
          <small v-if="row.source && !row.record && !row.guide">{{ row.source.includes('bgm.tv') ? '名称来源：Bangumi' : '中文名：Steam 商店' }}</small>
          <small v-else-if="!row.original && !row.record && !row.guide && steamActivity.titles[row.game.appid]?.checked_at">{{ steamActivity.titles[row.game.appid]?.error ? '中文资料暂不可用' : '暂无已核实的中文名称' }}</small>
        </div>
      </article>
      <div v-if="!visible.length" class="empty"><Search /><strong>{{ rows.length ? '没有符合条件的游戏' : '先刷新 Steam 游戏库' }}</strong><span>{{ rows.length ? '换个关键词，或调整书库与安装筛选。' : '刷新只更新库存和时长，选择后才会加入书库。' }}</span></div>
    </div>
    <div class="paging"><small>显示 {{ filtered.length ? (page - 1) * pageSize + 1 : 0 }}–{{ Math.min(page * pageSize, filtered.length) }} / {{ filtered.length }} 款</small><button :disabled="page <= 1" @click="nextPage(-1)">上一页</button><span>{{ page }} / {{ pageCount }}</span><button :disabled="page >= pageCount" @click="nextPage(1)">下一页</button></div>
    <footer>
      <div class="chosen-summary"><strong>已选 {{ chosen.length }} 款</strong><small>{{ pending.length }} 款待入库 / 关联<template v-if="hiddenSelected"> · {{ hiddenSelected }} 款不在当前筛选中</template></small><small v-if="undecided.length" class="error">{{ undecided.length }} 款需先选择关联目标</small></div>
      <small class="sync-hint">关联后自动同步时长与成就</small>
      <button :disabled="steamBatch.running || !!steamLibraryRefreshingId || !chosen.length || !cache?.updated_at" @click="submit(false)">同步所选时长与成就</button>
      <button class="primary" :disabled="steamBatch.running || !!steamLibraryRefreshingId || !pending.length || !!undecided.length || !cache?.updated_at" @click="submit(true)"><Download />{{ steamBatch.running ? '任务进行中' : `关联并同步 ${pending.length} 款` }}</button>
    </footer>
  </dialog>
</template>

<style scoped>
.steam-import{width:min(1120px,96vw);height:min(820px,92vh);max-height:92vh;max-width:96vw;margin:auto;padding:0;border:1px solid var(--color-hairline);border-radius:10px;background:var(--color-canvas);color:var(--color-ink);overflow:hidden}.steam-import[open]{display:flex;flex-direction:column}.steam-import::backdrop{background:#0009;backdrop-filter:blur(5px)}header{display:flex;justify-content:space-between;align-items:start;padding:20px 24px 12px;flex-shrink:0}h2{font:26px var(--font-display);margin:0}header p{font-size:12px;color:var(--color-muted);margin:6px 0 0}button,input,select{font:inherit;color:inherit}button,select{font-size:12px;border:1px solid var(--color-hairline);border-radius:4px;background:var(--color-paper);padding:7px 10px;min-height:32px}button{display:inline-flex;align-items:center;justify-content:center;gap:6px;cursor:pointer}button:disabled{opacity:.4;cursor:default}button:not(:disabled):hover{border-color:var(--color-beacon)}button:focus-visible,input:focus-visible,select:focus-visible,summary:focus-visible{outline:2px solid var(--color-beacon);outline-offset:2px}svg{width:15px;height:15px;flex-shrink:0}.icon-button{padding:6px}.connection,.filters,.scope,.selection,.paging,footer{display:flex;align-items:center;gap:10px;flex-wrap:wrap;padding-inline:24px;flex-shrink:0}.connection{padding-bottom:12px;border-bottom:1px solid var(--color-hairline)}.connection small{margin-left:auto;color:var(--color-muted);font-size:11px}.account-label,.filters label{display:flex;align-items:center;gap:7px;font-size:11px;color:var(--color-muted)}.account-label select{max-width:240px}.text-button{background:none;border:0;color:var(--color-beacon);padding:4px;font-size:11px}.messages{padding:0 24px;flex-shrink:0;max-height:120px;overflow:auto}.messages p{font-size:12px;line-height:1.7;margin:7px 0}.job{display:flex;align-items:center;gap:12px;flex-wrap:wrap;font-size:11px;color:var(--color-beacon);padding-top:5px}.job small{color:var(--color-muted)}.errors{font-size:12px;color:var(--color-alert,#d68e7b)}.errors summary{cursor:pointer;padding:6px 0}.error{color:var(--color-alert,#d68e7b)}.filters{padding-top:12px}.search{flex:1;min-width:180px;background:var(--color-paper);border:1px solid var(--color-hairline);border-radius:4px;padding:0 9px}.search input{min-width:0;width:100%;font-size:12px;padding:8px 0;background:transparent;border:0}.scope{padding-top:8px;padding-bottom:4px;gap:6px}.scope button{border:0;background:none;color:var(--color-muted);padding:7px 9px}.scope button[aria-pressed=true]{background:var(--color-paper);color:var(--color-beacon)}.scope button span{font:10px var(--font-mono);opacity:.75}.scope .names-button{margin-left:auto;font-size:11px;color:var(--color-beacon)}.selection{padding-top:3px;padding-bottom:6px;gap:16px;border-bottom:1px solid var(--color-hairline);font-size:11px;color:var(--color-muted)}.selection label{display:flex;align-items:center;gap:6px;cursor:pointer}.games{flex:1;min-height:0;overflow:auto;scrollbar-gutter:stable;padding:0 24px}.games article{display:flex;align-items:center;gap:20px;padding:13px 8px;border-bottom:1px solid var(--color-hairline);border-left:2px solid transparent}.games article.chosen{background:color-mix(in srgb,var(--color-beacon) 5%,transparent);border-left-color:var(--color-beacon)}.game-check{display:flex;align-items:center;gap:13px;flex:1;min-width:0;cursor:pointer}.game-check img,.monogram{width:40px;height:40px;object-fit:cover;border-radius:5px;flex-shrink:0}.monogram{display:grid;place-items:center;background:var(--color-paper);color:var(--color-beacon);font:23px var(--font-display)}.game-name{min-width:0}.game-name strong{font-size:14px;font-weight:500;line-height:1.5;display:block;overflow-wrap:anywhere}.game-name small{font-size:11px;color:var(--color-muted);display:block;line-height:1.7}.game-name .original{font-size:12px}.target{width:230px;max-width:38%;font-size:12px;color:var(--color-muted);display:flex;flex-direction:column;align-items:start;gap:5px}.target select{max-width:100%;font-size:11px;min-height:28px;padding:4px}.target small{font-size:10px;line-height:1.5}.linked{display:flex;align-items:center;gap:5px;color:var(--color-beacon)}.paging{justify-content:end;font-size:11px;border-top:1px solid var(--color-hairline);padding-block:7px}.paging small{margin-right:auto;color:var(--color-muted)}.paging button{font-size:11px;min-height:28px;padding:4px 9px}footer{padding-block:12px;border-top:1px solid var(--color-hairline);background:var(--color-paper);gap:10px}.chosen-summary{margin-right:auto;display:grid;gap:3px}.chosen-summary strong{font-size:13px;font-weight:500}.chosen-summary small{font-size:11px;color:var(--color-muted)}.sync-hint{font-size:11px;color:var(--color-muted)}.primary{background:var(--color-beacon);color:var(--color-canvas);border-color:var(--color-beacon)}.empty{display:grid;justify-items:center;align-content:center;gap:10px;min-height:150px;color:var(--color-muted);font-size:12px}.empty strong{font-size:15px;font-weight:500}.spinning{animation:spin 1s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}@media(prefers-reduced-motion:reduce){.spinning{animation:none}}@media(max-width:760px){header,.connection,.filters,.scope,.selection,.paging,footer,.messages{padding-inline:16px}.games{padding-inline:10px}.connection small{display:none}.target{width:180px;max-width:35%}.scope .names-button{margin-left:0}.chosen-summary{width:100%}.steam-import{height:96vh;max-height:96vh}.game-check{gap:8px}.game-check img,.monogram{width:32px;height:32px}.selection{gap:8px}.game-name strong{font-size:13px}}
@media(max-height:720px){header{padding-top:12px;padding-bottom:8px;align-items:center}h2{font-size:22px}header p{display:none}.connection{padding-bottom:8px}.filters{padding-top:8px}.scope{padding-top:4px;padding-bottom:0}.selection{padding-top:0;padding-bottom:4px}.games article{padding-block:8px}.paging{padding-block:4px}footer{padding-block:8px}.messages{max-height:72px}}
</style>
