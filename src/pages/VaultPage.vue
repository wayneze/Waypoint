<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { ArrowUpRight, Check, Copy, Ellipsis, LockKeyhole, Plus, Search, Shield, Star, X } from '@lucide/vue';
import SteamConnection from '../components/SteamConnection.vue';
import CharAvatar from '../components/CharAvatar.vue';
import GameStampIcon from '../components/GameStampIcon.vue';
import {
  copyToClipboard, currentTab, gameRecords, isProjectingObsidian, vaultList, vaultPlatformColor, vaultStampOf,
} from '../workspace/core';
import {
  filteredVaultList, handleDeleteVaultItem, handleOpenEditVault, handleToggleVaultStar,
  handleToggleVaultTrackDaily, openAddVaultModal, selectVaultGroup,
  selectedVaultGroup, selectedVaultGroupLabel, steamGamesOf, vaultGroups,
  handleExportVaultPlain, handleImportVaultPlain,
} from '../workspace/vault';
import { formatSteamPlaytime, isSteamVaultPlatform } from '../services/vault';
import { handleProjectToObsidian } from '../workspace/bootstrap';
import { openSteamImport, steamActivity, steamImportAccountId } from '../workspace/gameActivity';
import { libraryGuides } from '../workspace/guides/library';
import { buildSteamRows, selectSteamRows } from '../services/steamImport';
import { settingsSection } from '../workspace/settings';
import { resolveVaultGameId } from '../services/games';

const query = ref('');
const ledger = ref<HTMLElement | null>(null);
const searchResults = computed(() => {
  const term = query.value.trim().toLocaleLowerCase();
  return filteredVaultList.value.filter(item => !term || [
    item.platform, item.account_name, item.server,
    ...item.assets.map(asset => asset.name),
    ...item.characters.flatMap(ch => [ch.name, ch.server, ch.sect, ...ch.assets.map(asset => asset.name)]),
  ].filter(Boolean).join(' ').toLocaleLowerCase().includes(term));
});
const entries = computed(() => searchResults.value.map(account => ({
  account,
  characters: [...account.characters].sort((a, b) => Number(!!b.starred) - Number(!!a.starred)),
  games: selectSteamRows(buildSteamRows(steamGamesOf(account.id), gameRecords.value, libraryGuides.value, steamActivity.value), '', 'all', 'all', 'library', false, []),
})));
const visibleRosterCount = computed(() => searchResults.value.reduce((sum, item) => sum + item.characters.length, 0));
const rosterCount = computed(() => vaultList.value.reduce((sum, item) => sum + item.characters.length, 0));

watch([query, selectedVaultGroup], () => {
  if (ledger.value) ledger.value.scrollTop = 0;
});
watch(vaultGroups, groups => {
  if (selectedVaultGroup.value !== 'all' && !groups.some(group => group.id === selectedVaultGroup.value)) selectVaultGroup('all');
});
watch(() => vaultList.value.map(item => item.id), async (ids, previous) => {
  if (ids.length !== previous.length + 1) return;
  const added = vaultList.value.find(item => !previous.includes(item.id));
  if (!added) return;
  query.value = '';
  if (selectedVaultGroup.value !== 'all') selectVaultGroup(resolveVaultGameId(added.platform));
  await nextTick();
  const addedAccount = document.getElementById('vault-account-' + added.id);
  addedAccount?.scrollIntoView({ block: 'nearest' });
  addedAccount?.focus({ preventScroll: true });
});

function closeMenu(event: Event) {
  const menu = (event.currentTarget as HTMLElement).closest('details');
  if (!menu) return;
  menu.open = false;
  if (event instanceof KeyboardEvent) menu.querySelector('summary')?.focus();
}

function openBackup() {
  settingsSection.value = 'backup';
  currentTab.value = 'settings';
}

function addAccount() {
  openAddVaultModal(selectedVaultGroup.value === 'all' ? '' : selectedVaultGroupLabel.value);
}
</script>

<template>
  <section class="vault-workspace">
    <header class="vault-heading">
      <div><p class="vault-eyebrow">你的游戏身份</p><h1>资产金库<span>.</span></h1><p>账号、角色与珍藏，各有归处。</p></div>
      <div class="vault-heading-actions">
        <span class="vault-total"><b>{{ vaultList.length }}</b> 账号<span>/</span><b>{{ rosterCount }}</b> 角色</span>
        <button type="button" class="vault-button is-primary" @click="addAccount"><Plus />登记账号</button>
        <details class="vault-menu" @keydown.esc.prevent.stop="closeMenu($event)">
          <summary class="vault-icon-button" aria-label="更多金库操作"><Ellipsis /></summary>
          <div class="vault-menu-panel" @click="closeMenu($event)">
            <button type="button" @click="openBackup">加密备份与恢复<ArrowUpRight /></button>
            <button type="button" :disabled="isProjectingObsidian" @click="handleProjectToObsidian('vault')">{{ isProjectingObsidian ? '投影中…' : '投影到 Obsidian' }}</button>
            <hr />
            <button type="button" @click="handleImportVaultPlain">导入金库 JSON</button>
            <button type="button" @click="handleExportVaultPlain">导出明文金库…</button>
          </div>
        </details>
      </div>
    </header>
    <SteamConnection v-if="selectedVaultGroup === 'steam' || (selectedVaultGroup === 'all' && vaultList.some(v => isSteamVaultPlatform(v.platform)))" />
    <nav class="vault-groups" aria-label="金库平台分组">
      <button type="button" :aria-current="selectedVaultGroup === 'all' ? 'page' : undefined" @click="selectVaultGroup('all')"><Shield />全部<span>{{ vaultList.length }}</span></button>
      <button v-for="group in vaultGroups" :key="group.id" type="button" :style="{ '--account-color': vaultPlatformColor(group.id) }"
        :aria-current="selectedVaultGroup === group.id ? 'page' : undefined" @click="selectVaultGroup(group.id)">
        <GameStampIcon :game="group.stamp" class="w-3.5 h-3.5" />{{ group.name }}<span>{{ group.count }}</span>
      </button>
    </nav>
    <div class="vault-browse-bar">
      <label class="vault-search"><Search /><input v-model="query" type="search" aria-label="搜索账号、角色、区服或资产" placeholder="搜索账号、角色、区服或资产" /><button v-if="query" type="button" aria-label="清空搜索" @click="query = ''"><X /></button></label>
      <p>{{ query ? '搜索结果' : selectedVaultGroupLabel }}<span>{{ searchResults.length }} 个账号 · {{ visibleRosterCount }} 位角色</span></p>
      <span class="vault-local-note"><LockKeyhole />本机加密保存</span>
    </div>
    <div ref="ledger" class="vault-ledger" aria-label="账号与角色资产">
      <article v-for="{ account, characters, games } in entries" :id="'vault-account-' + account.id" :key="account.id" class="vault-account" tabindex="-1"
        :aria-label="account.account_name || '未命名账号'" :style="{ '--account-color': vaultPlatformColor(account.platform) }">
        <header class="vault-account-heading">
          <span class="vault-platform-mark"><GameStampIcon :game="vaultStampOf(account.platform)" class="w-5 h-5" /></span>
          <div class="vault-account-identity"><div class="vault-account-title"><h2>{{ account.account_name || '未命名账号' }}</h2><button type="button" class="vault-icon-button" :aria-label="'复制账号' + account.account_name" title="复制账号" @click="copyToClipboard(account.account_name, '账号')"><Copy /></button></div><p>{{ account.platform || '未分类' }}<span>·</span>{{ account.locked ? '本机暂时不可读' : characters.length ? `${characters.length} 位角色 · ${characters.filter(ch => ch.starred).length} 位重要角色` : isSteamVaultPlatform(account.platform) ? `${games.length} 款已同步游戏` : (account.server || '独立登录账号') }}</p></div>
          <div class="vault-inline-actions"><button type="button" class="vault-button" :disabled="!!account.locked" :aria-label="'编辑资料：' + account.account_name" @click="handleOpenEditVault(account)">编辑资料</button>
            <details class="vault-menu" @keydown.esc.prevent.stop="closeMenu($event)">
              <summary class="vault-icon-button" :aria-label="'更多账号操作：' + account.account_name"><Ellipsis /></summary>
              <div class="vault-menu-panel" @click="closeMenu($event)"><button type="button" class="is-danger" @click="handleDeleteVaultItem(account.id)">删除此账号…</button></div>
            </details>
          </div>
        </header>
        <div v-if="account.locked" class="vault-locked" role="status"><LockKeyhole /><div><h3>请从原电脑迁移</h3><p>{{ account.locked }}</p><p>在原电脑导出完整加密备份，再到本机恢复。</p><button type="button" class="vault-text-button" @click="openBackup">前往备份与恢复<ArrowUpRight /></button></div></div>
        <template v-else>
          <section v-if="characters.length" class="vault-roster" aria-label="角色与资产">
            <div class="vault-roster-caption"><span>角色 / 区服</span><span>角色资产</span><span>角色管理</span></div>
            <div v-for="ch in characters" :key="ch.id" class="vault-character" :class="{ 'is-important': ch.starred }">
              <div class="vault-character-heading">
                <CharAvatar :stamp="vaultStampOf(account.platform)" :name="ch.name" :sect="ch.sect" />
                <div class="vault-character-copy"><h3>{{ ch.name }}<span v-if="ch.starred" class="vault-important-label">重要</span></h3><p>{{ [ch.server, ch.sect].filter(Boolean).join(' · ') || '尚未填写区服' }}</p></div>
              </div>
              <div class="vault-character-assets">
                <dl v-if="ch.assets.length" class="vault-assets"><div v-for="asset in ch.assets" :key="asset.id"><dt>{{ asset.name }}<small v-if="asset.note">{{ asset.note }}</small></dt><dd>{{ asset.qty || '已登记' }}</dd></div></dl>
                <p v-else class="vault-empty-copy">尚未登记角色资产</p>
                <div v-if="ch.query_id" class="vault-query"><span>查询 ID</span><code>{{ ch.query_id }}</code><button type="button" class="vault-icon-button" :aria-label="'复制' + ch.name + '查询ID'" @click="copyToClipboard(ch.query_id, '查询 ID')"><Copy /></button></div>
              </div>
              <div class="vault-character-actions">
                <button type="button" class="vault-star" :class="{ 'is-starred': ch.starred }" :aria-label="(ch.starred ? '取消重要角色：' : '设为重要角色：') + ch.name" :aria-pressed="!!ch.starred" @click="handleToggleVaultStar(account.id, ch.id)"><Star :fill="ch.starred ? 'currentColor' : 'none'" />{{ ch.starred ? '重要角色' : '标为重要' }}</button>
                <button type="button" class="vault-track" :class="{ 'is-tracked': ch.track_daily }" :aria-label="ch.name + (ch.track_daily ? '移出日课' : '加入日课')" :aria-pressed="ch.track_daily" @click="handleToggleVaultTrackDaily(account.id, ch.id)"><Check v-if="ch.track_daily" /><Plus v-else />{{ ch.track_daily ? '参与日课' : '加入日课' }}</button>
              </div>
            </div>
          </section>
          <section v-if="isSteamVaultPlatform(account.platform)" class="vault-detail-section">
            <div class="vault-section-heading"><h3>Steam 游戏库<span>{{ games.length }}</span></h3><button class="vault-text-button" @click="steamImportAccountId = account.id; currentTab = 'galgame'; openSteamImport()">查看、筛选与同步</button></div>
            <p v-if="!account.steamid64" class="vault-empty-copy">编辑资料，填入 SteamID64 后即可同步游戏与游玩时长。</p>
            <p v-else-if="!games.length" class="vault-empty-copy">尚未同步游戏。请确保 Steam 资料页的「游戏详情」公开。</p>
            <dl v-else class="vault-steam-games"><div v-for="row in games.slice(0, 6)" :key="row.game.appid"><dt>{{ row.title }}<small v-if="row.original">{{ row.original }}</small></dt><dd>{{ formatSteamPlaytime(row.game.playtime_forever) }}</dd></div></dl>
            <p v-if="games.length > 6" class="vault-empty-copy">优先展示书库已有作品，其余 {{ games.length - 6 }} 款可在游戏库中查看。</p>
            <p v-if="account.steamid64" class="vault-steamid">SteamID64 <code>{{ account.steamid64 }}</code></p>
          </section>
          <section v-if="account.assets.length || !characters.length" class="vault-detail-section">
            <div class="vault-section-heading"><h3>账号资产<span>{{ account.assets.length }}</span></h3></div>
            <dl v-if="account.assets.length" class="vault-assets"><div v-for="asset in account.assets" :key="asset.id"><dt>{{ asset.name }}<small v-if="asset.note">{{ asset.note }}</small></dt><dd>{{ asset.qty || '已登记' }}</dd></div></dl>
            <p v-else class="vault-empty-copy">还没有账号资产。绑定信息、密保等可以在「编辑资料」中登记。</p>
          </section>
          <section v-if="account.notes" class="vault-detail-section"><div class="vault-section-heading"><h3>备注</h3></div><p class="vault-notes">{{ account.notes }}</p></section>
        </template>
      </article>
      <div v-if="!entries.length" class="vault-detail-placeholder"><Shield /><h2>{{ query ? '换个关键词试试' : '每一段旅程，从一个身份开始' }}</h2><p>{{ query ? '可以搜索账号、角色名、区服、门派或资产名称。' : '把登录账号和角色收在一起，日课与资产就有了归属。' }}</p><button v-if="!query" type="button" class="vault-button is-primary" @click="addAccount"><Plus />登记账号</button></div>
    </div>
  </section>
</template>

<style scoped>
.vault-workspace { width: 100%; height: 100%; min-height: 0; max-width: 1440px; margin-inline: auto; display: flex; flex-direction: column; container-type: inline-size; }
.vault-heading { display: flex; justify-content: space-between; align-items: center; gap: 20px; padding-bottom: 24px; flex-shrink: 0; }
.vault-eyebrow { font-size: 10px; color: var(--color-faint); letter-spacing: .12em; }
.vault-heading h1 { font-size: 32px; line-height: 1.25; letter-spacing: -.04em; font-weight: 600; margin: 4px 0 8px; }
.vault-heading h1 span { color: var(--color-beacon); }
.vault-heading h1 + p { font-size: 12px; color: var(--color-muted); }
.vault-heading-actions, .vault-inline-actions { display: flex; align-items: center; gap: 10px; }
.vault-total { font-size: 11px; color: var(--color-faint); margin-right: 10px; white-space: nowrap; }
.vault-total b { font-family: var(--font-display); color: var(--color-ink); font-size: 20px; font-weight: 400; margin-right: 4px; }
.vault-total > span { margin: 0 12px; color: var(--color-hairline); }
.vault-button, .vault-text-button, .vault-icon-button { display: inline-flex; align-items: center; justify-content: center; gap: 7px; font-size: 12px; cursor: pointer; color: var(--color-muted); }
.vault-button { min-height: 34px; border: 1px solid var(--color-hairline); border-radius: 5px; padding: 6px 12px; white-space: nowrap; }
.vault-button:hover, .vault-icon-button:hover { background: var(--color-paper); color: var(--color-ink); }
.vault-button.is-primary { border-color: color-mix(in srgb, var(--color-beacon) 35%, transparent); color: var(--color-beacon); background: color-mix(in srgb, var(--color-beacon) 8%, transparent); }
.vault-icon-button { width: 32px; height: 32px; border-radius: 4px; flex-shrink: 0; }
.vault-text-button { color: var(--color-beacon); min-height: 32px; }
.vault-text-button:hover { color: var(--color-ink); }
.vault-button svg, .vault-text-button svg, .vault-icon-button svg { width: 15px; height: 15px; }
.vault-menu { position: relative; }
.vault-menu > summary { list-style: none; }
.vault-menu > summary::-webkit-details-marker { display: none; }
.vault-menu-panel { position: absolute; right: 0; top: calc(100% + 6px); z-index: 15; width: 205px; padding: 6px; background: var(--color-paper); border: 1px solid var(--color-hairline); border-radius: 6px; box-shadow: 0 12px 30px #0005; }
.vault-menu-panel button { display: flex; align-items: center; justify-content: space-between; width: 100%; padding: 10px; border-radius: 3px; text-align: left; font-size: 12px; color: var(--color-muted); cursor: pointer; }
.vault-menu-panel button:hover { background: var(--color-raised); color: var(--color-ink); }
.vault-menu-panel svg { width: 13px; height: 13px; }
.vault-menu-panel hr { border-color: var(--color-hairline); margin: 5px; }
.vault-menu-panel .is-danger { color: var(--color-alert); }
.vault-groups { display: flex; gap: 10px; border-bottom: 1px solid var(--color-hairline); flex-shrink: 0; overflow-x: auto; scrollbar-width: thin; scrollbar-color: var(--color-hairline) transparent; }
.vault-groups button { display: inline-flex; align-items: center; gap: 7px; padding: 11px 9px; white-space: nowrap; border-bottom: 2px solid transparent; color: var(--color-faint); font-size: 12px; cursor: pointer; }
.vault-groups button:hover { color: var(--color-ink); }
.vault-groups button[aria-current] { color: var(--account-color, var(--color-beacon)); border-bottom-color: currentColor; }
.vault-groups svg { width: 14px; height: 14px; }
.vault-groups button > span:last-child { font-family: var(--font-mono); font-size: 10px; opacity: .8; margin-left: 3px; }
.vault-browse-bar { display: flex; align-items: center; gap: 22px; padding-block: 18px; flex-shrink: 0; }
.vault-search { display: flex; align-items: center; gap: 9px; padding: 9px 10px; border: 1px solid var(--color-hairline); border-radius: 5px; background: var(--color-chrome); width: 310px; max-width: 100%; }
.vault-search:focus-within { border-color: var(--color-beacon); }
.vault-search > svg, .vault-search button svg { width: 14px; height: 14px; color: var(--color-faint); flex-shrink: 0; }
.vault-search input { font-size: 12px; width: 100%; min-width: 0; outline: none; background: transparent; }
.vault-search input::-webkit-search-cancel-button { display: none; }
.vault-search button { cursor: pointer; }
.vault-browse-bar > p { font-size: 11px; color: var(--color-muted); }
.vault-browse-bar > p span { margin-left: 12px; color: var(--color-faint); }
.vault-local-note { display: inline-flex; align-items: center; gap: 7px; margin-left: auto; font-size: 10px; color: var(--color-faint); white-space: nowrap; }
.vault-local-note svg { width: 12px; height: 12px; }
.vault-ledger { flex: 1; min-height: 0; overflow-y: auto; scrollbar-gutter: stable; padding-right: 10px; padding-bottom: 30px; }
.vault-account { min-width: 0; margin-bottom: 28px; outline-offset: -2px; }
.vault-account-heading { display: flex; align-items: center; gap: 12px; padding: 14px 10px; border-top: 1px solid var(--color-hairline); border-bottom: 1px solid var(--color-hairline); background: var(--color-chrome); }
.vault-platform-mark { color: var(--account-color); flex-shrink: 0; }
.vault-account-identity { flex: 1; min-width: 0; }
.vault-account-title { display: flex; align-items: center; gap: 8px; }
.vault-account-title h2 { font-size: 16px; line-height: 1.35; font-weight: 500; overflow-wrap: anywhere; user-select: text; }
.vault-account-title .vault-icon-button { width: 26px; height: 26px; color: var(--color-faint); }
.vault-account-title svg { width: 12px; height: 12px; }
.vault-account-identity > p { color: var(--color-faint); font-size: 10px; margin-top: 3px; }
.vault-account-identity > p > span { margin-inline: 7px; }
.vault-detail-section { padding: 18px 14px; border-bottom: 1px solid var(--color-hairline); }
.vault-section-heading { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 12px; }
.vault-section-heading h3 { font-size: 12px; font-weight: 500; color: var(--color-muted); }
.vault-section-heading h3 span { font-family: var(--font-mono); font-size: 10px; color: var(--color-faint); margin-left: 9px; }
.vault-character, .vault-roster-caption { display: grid; grid-template-columns: minmax(170px, .9fr) minmax(0, 2fr) 94px; gap: 22px; align-items: center; }
.vault-roster-caption { font-size: 10px; color: var(--color-faint); padding: 13px 14px 9px; }
.vault-roster-caption > :last-child { text-align: center; }
.vault-character { padding: 18px 14px; border-bottom: 1px solid color-mix(in srgb, var(--color-hairline) 60%, transparent); }
.vault-character.is-important { background: color-mix(in srgb, var(--color-beacon) 7%, transparent); box-shadow: inset 2px 0 0 var(--color-beacon); }
.vault-character-heading { display: flex; align-items: center; gap: 12px; min-width: 0; }
.vault-character-copy { min-width: 0; }
.vault-character-copy h3 { display: flex; align-items: center; flex-wrap: wrap; gap: 7px; font-size: 15px; font-weight: 500; overflow-wrap: anywhere; }
.is-important .vault-character-copy h3 { color: var(--color-beacon); }
.vault-important-label { font-size: 9px; font-weight: 400; border: 1px solid color-mix(in srgb, var(--color-beacon) 35%, transparent); padding: 1px 4px; border-radius: 2px; white-space: nowrap; }
.vault-character-copy p { font-size: 11px; color: var(--color-faint); margin-top: 4px; overflow-wrap: anywhere; }
.vault-character-actions { display: grid; gap: 7px; }
.vault-star, .vault-track { display: inline-flex; align-items: center; justify-content: center; gap: 6px; min-height: 28px; font-size: 11px; color: var(--color-faint); cursor: pointer; border-radius: 4px; white-space: nowrap; }
.vault-star:hover { color: var(--color-beacon); background: var(--color-paper); }
.vault-star.is-starred { color: var(--color-beacon); }
.vault-star svg { width: 13px; height: 13px; }
.vault-track { border: 1px solid var(--color-hairline); padding: 4px 7px; }
.vault-track.is-tracked { color: var(--color-done); border-color: color-mix(in srgb, var(--color-done) 30%, transparent); }
.vault-track svg { width: 12px; height: 12px; }
.vault-character-assets { min-width: 0; }
.vault-assets { display: grid; grid-template-columns: repeat(auto-fit, minmax(110px, 1fr)); gap: 15px 24px; font-size: 12px; }
.vault-assets dt { color: var(--color-faint); font-size: 11px; overflow-wrap: anywhere; }
.vault-assets small { display: block; font-size: 10px; margin-top: 2px; }
.vault-assets dd { color: var(--color-ink); font-family: var(--font-mono); font-size: 15px; margin-top: 3px; overflow-wrap: anywhere; user-select: text; }
.vault-query { display: flex; align-items: center; gap: 9px; margin-top: 9px; color: var(--color-faint); font-size: 10px; }
.vault-query > span { flex-shrink: 0; }
.vault-query code { overflow-wrap: anywhere; min-width: 0; user-select: text; }
.vault-query .vault-icon-button { width: 24px; height: 24px; }
.vault-query svg { width: 12px; height: 12px; }
.vault-notes { color: var(--color-muted); font-size: 13px; line-height: 1.9; white-space: pre-wrap; overflow-wrap: anywhere; user-select: text; }
.vault-steam-games { font-size: 12px; }
.vault-steam-games > div { display: flex; justify-content: space-between; align-items: baseline; gap: 18px; padding-block: 10px; border-bottom: 1px solid color-mix(in srgb, var(--color-hairline) 60%, transparent); }
.vault-steam-games dt { overflow-wrap: anywhere; }
.vault-steam-games dt small { display: block; margin-top: 3px; color: var(--color-faint); font-size: 11px; }
.vault-steam-games dd { color: var(--color-faint); white-space: nowrap; font-size: 11px; font-family: var(--font-mono); }
.vault-steamid { color: var(--color-faint); font-size: 10px; margin-top: 15px; overflow-wrap: anywhere; }
.vault-steamid code { user-select: text; margin-left: 7px; }
.vault-empty-copy { font-size: 12px; color: var(--color-faint); line-height: 1.9; }
.vault-detail-placeholder { text-align: center; padding: 35px; color: var(--color-faint); }
.vault-detail-placeholder > svg { width: 48px; height: 48px; stroke-width: .8; color: var(--color-beacon); margin: 0 auto 24px; }
.vault-detail-placeholder h2 { font-family: var(--font-display); font-size: 22px; color: var(--color-muted); }
.vault-detail-placeholder p { font-size: 12px; line-height: 1.8; margin: 12px 0 24px; }
.vault-locked { display: flex; gap: 15px; padding-block: 25px; color: var(--color-alert); }
.vault-locked > svg { width: 22px; height: 22px; flex-shrink: 0; }
.vault-locked h3 { font-size: 14px; }
.vault-locked p { margin-top: 8px; font-size: 12px; color: var(--color-muted); line-height: 1.8; }
button:disabled { opacity: .45; cursor: default; }
:where(button, summary, input):focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 3px; }
.vault-search input:focus-visible { outline: none; }
@container (max-width: 850px) {
  .vault-total, .vault-local-note { display: none; }
  .vault-browse-bar { justify-content: space-between; gap: 12px; }
  .vault-browse-bar > p { text-align: right; }
  .vault-browse-bar > p span { display: block; margin-left: 0; }
  .vault-character, .vault-roster-caption { grid-template-columns: minmax(145px, .9fr) minmax(0, 1.5fr) 90px; gap: 16px; }
}
@container (max-width: 600px) {
  .vault-heading { gap: 12px; padding-bottom: 18px; }
  .vault-heading h1 { font-size: 27px; }
  .vault-heading h1 + p { display: none; }
  .vault-browse-bar { flex-wrap: wrap; padding-block: 12px; }
  .vault-search { width: 100%; }
  .vault-browse-bar > p { display: flex; gap: 12px; }
  .vault-ledger { padding-right: 3px; }
  .vault-account-heading { gap: 9px; padding-inline: 8px; }
  .vault-inline-actions { gap: 3px; }
  .vault-account-title h2 { font-size: 14px; }
  .vault-roster-caption { display: none; }
  .vault-character { grid-template-columns: minmax(0, 1fr) 90px; gap: 12px; padding: 16px 12px; }
  .vault-character-heading { grid-column: 1; grid-row: 1; }
  .vault-character-actions { grid-column: 2; grid-row: 1; }
  .vault-character-assets { grid-column: 1 / -1; border-top: 1px solid color-mix(in srgb, var(--color-hairline) 60%, transparent); padding-top: 12px; }
  .vault-groups { gap: 2px; }
  .vault-groups button { padding-inline: 8px; }
}
@media (max-height: 600px) {
  .vault-heading { padding-bottom: 12px; }
  .vault-heading .vault-eyebrow, .vault-heading h1 + p { display: none; }
  .vault-heading h1 { font-size: 24px; margin: 0; }
  .vault-groups button { padding-block: 8px; }
}
</style>
