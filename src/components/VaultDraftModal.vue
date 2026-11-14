<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { Check, Plus, Shield, X } from '@lucide/vue';
import { describeError } from '../services/errors';
import {
  editingVault,
  handleAddVaultItem,
  handleSaveEditVault,
  showEditVaultModal,
  addDraftVaultAsset,
  addDraftVaultCharacter,
  isSkyVaultPlatform,
  removeDraftVaultAsset,
  removeDraftVaultCharacter,
  syncDraftVaultRoster,
  vaultGroups,
} from '../workspace/vault';
import { state } from '../workspace/core';
import { GAMES, gameHasRoster, resolveVaultGameId } from '../services/games';
import { isSteamVaultPlatform } from '../services/vault';
import type { VaultAsset, VaultCharacter } from '../types';

type VaultDraft = {
  id?: string;
  platform: string;
  account_name: string;
  server?: string;
  notes?: string;
  steamid64?: string;
  assets: VaultAsset[];
  characters: VaultCharacter[];
};

const vaultDraft = computed((): VaultDraft | null => {
  if (showEditVaultModal.value && editingVault.value) return editingVault.value;
  if (state.showAddVaultModal) return state.newVault;
  return null;
});

const vaultDraftMode = computed(() => (showEditVaultModal.value && editingVault.value ? 'edit' : 'add'));

const vaultHasRoster = computed(() => !!vaultDraft.value && gameHasRoster(resolveVaultGameId(vaultDraft.value.platform)));

const vaultIsSteam = computed(() => !!vaultDraft.value && isSteamVaultPlatform(vaultDraft.value.platform));

const vaultIsSky = computed(() => !!vaultDraft.value && isSkyVaultPlatform(vaultDraft.value.platform));

const vaultPlatformChips = computed(() => {
  const known = GAMES.map(game => game.label);
  const extra = vaultGroups.value
    .filter(group => !GAMES.some(game => game.id === group.id))
    .map(group => group.name);
  return [...known, ...extra.filter(name => !known.includes(name))];
});

const dialog = ref<HTMLDialogElement | null>(null);
const saving = ref(false);
const saveError = ref('');
watch(() => !!vaultDraft.value, async open => {
  saveError.value = '';
  await nextTick();
  if (open) dialog.value?.showModal();
  else dialog.value?.close();
}, { immediate: true });

function closeVaultDraft() {
  if (saving.value) return;
  if (vaultDraftMode.value === 'edit') showEditVaultModal.value = false;
  else state.showAddVaultModal = false;
}

function pickVaultPlatform(name: string) {
  const draft = vaultDraft.value;
  if (!draft) return;
  draft.platform = name;
  syncDraftVaultRoster(draft);
}

async function submitVaultDraft() {
  if (saving.value || !vaultDraft.value?.account_name.trim()) return;
  saving.value = true;
  saveError.value = '';
  try {
    if (vaultDraftMode.value === 'edit') await handleSaveEditVault();
    else await handleAddVaultItem();
  } catch (error) {
    saveError.value = describeError(error).message;
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <dialog ref="dialog" class="vault-editor" aria-labelledby="vault-editor-title" @cancel.prevent="closeVaultDraft">
    <form v-if="vaultDraft" class="vault-editor-form" @submit.prevent="submitVaultDraft">
      <header class="vault-editor-heading">
        <div class="vault-editor-emblem"><Shield /></div>
        <div><p>账号档案</p><h2 id="vault-editor-title">{{ vaultDraftMode === 'edit' ? '编辑资料' : '登记账号' }}</h2></div>
        <button type="button" class="editor-icon" aria-label="关闭账号编辑" :disabled="saving" @click="closeVaultDraft"><X /></button>
      </header>
      <div class="vault-editor-scroll">
        <fieldset :disabled="saving">
          <section class="editor-section">
            <div class="editor-section-title"><span>01</span><div><h3>登录身份</h3><p>账号归属的平台与登录信息。</p></div></div>
            <div class="editor-section-fields">
              <label class="editor-field"><span>游戏 / 平台</span><input v-model="vaultDraft.platform" type="text" required placeholder="选择下方平台，或填写其他平台" @change="syncDraftVaultRoster(vaultDraft)" /></label>
              <div class="editor-platforms"><button v-for="name in vaultPlatformChips" :key="name" type="button" :aria-pressed="vaultDraft.platform === name" @click="pickVaultPlatform(name)">{{ name }}</button><button v-if="!vaultPlatformChips.includes('Steam')" type="button" :aria-pressed="vaultDraft.platform === 'Steam'" @click="pickVaultPlatform('Steam')">Steam</button></div>
              <div class="editor-field-pair">
                <label class="editor-field"><span>登录账号</span><input v-model="vaultDraft.account_name" type="text" required autocomplete="off" placeholder="邮箱 / 手机号 / 用户名" /></label>
                <label v-if="vaultIsSteam" class="editor-field"><span>SteamID64</span><input v-model="vaultDraft.steamid64" type="text" placeholder="17 位数字或资料页链接" /></label>
                <label v-else-if="!vaultHasRoster" class="editor-field"><span>区服</span><input v-model="vaultDraft.server" type="text" placeholder="选填" /></label>
              </div>
              <p v-if="vaultIsSteam" class="editor-help">Steam Web API Key 在「设置 → 外部连接」中填写。</p>
            </div>
          </section>
          <section v-if="vaultHasRoster" class="editor-section">
            <div class="editor-section-title"><span>02</span><div><h3>角色名册</h3><p>角色各自拥有区服、日课和资产。</p><button type="button" class="editor-text" @click="addDraftVaultCharacter(vaultDraft)"><Plus />添加角色</button></div></div>
            <div class="editor-section-fields">
              <div v-for="(ch, i) in vaultDraft.characters" :key="ch.id" class="editor-character">
                <div class="editor-character-bar"><span>角色 {{ String(i + 1).padStart(2, '0') }}</span><label class="editor-check" :class="{ 'is-important': ch.starred }"><input v-model="ch.starred" type="checkbox" :aria-label="'角色' + (i + 1) + '设为重要角色'" />重要角色</label><label class="editor-check"><input v-model="ch.track_daily" type="checkbox" :aria-label="'角色' + (i + 1) + '参与日课'" />参与日课</label><button v-if="vaultDraftMode === 'add' || vaultDraft.characters.length > 1" type="button" class="editor-icon" :aria-label="'移除角色' + (i + 1)" @click="removeDraftVaultCharacter(vaultDraft, i)"><X /></button></div>
                <div class="editor-character-fields">
                  <label class="editor-field"><span>角色名</span><input v-model="ch.name" type="text" :aria-label="'角色' + (i + 1) + '名称'" placeholder="角色名" /></label>
                  <label class="editor-field"><span>区服</span><input v-model="ch.server" type="text" :aria-label="'角色' + (i + 1) + '区服'" placeholder="区服" /></label>
                  <label class="editor-field"><span>门派</span><input v-model="ch.sect" type="text" :aria-label="'角色' + (i + 1) + '门派'" placeholder="选填" /></label>
                </div>
                <label v-if="vaultIsSky" class="editor-field editor-query"><span>光遇查询 ID</span><input v-model="ch.query_id" type="text" :aria-label="'角色' + (i + 1) + '查询ID'" placeholder="光遇 → 设置 → 精灵 → 工具指南" spellcheck="false" autocomplete="off" /></label>
                <div v-for="(asset, ai) in ch.assets" :key="asset.id" class="editor-asset-row">
                  <input v-model="asset.name" type="text" :aria-label="'角色' + (i + 1) + '资产' + (ai + 1) + '名称'" placeholder="资产名称，如金、外观" />
                  <input v-model="asset.qty" type="text" :aria-label="'角色' + (i + 1) + '资产' + (ai + 1) + '数量'" placeholder="数量 / 内容" />
                  <button type="button" class="editor-icon" :aria-label="'移除角色' + (i + 1) + '资产' + (ai + 1)" @click="removeDraftVaultAsset(ch, ai)"><X /></button>
                </div>
                <button type="button" class="editor-text" :aria-label="'给角色' + (i + 1) + '添加资产'" @click="addDraftVaultAsset(ch)"><Plus />角色资产</button>
              </div>
              <p v-if="!vaultDraft.characters.length" class="editor-help">可以先登记账号，稍后再添加角色。</p>
            </div>
          </section>
          <section class="editor-section">
            <div class="editor-section-title"><span>{{ vaultHasRoster ? '03' : '02' }}</span><div><h3>账号资产</h3><p>绑定信息、密保等随账号保存。</p><button type="button" class="editor-text" @click="addDraftVaultAsset(vaultDraft)"><Plus />添加账号资产</button></div></div>
            <div class="editor-section-fields">
              <p v-if="!vaultDraft.assets.length" class="editor-help">尚未登记账号资产，可按需添加。</p>
              <div v-for="(asset, i) in vaultDraft.assets" :key="asset.id" class="editor-asset-row">
                <input v-model="asset.name" type="text" :aria-label="'账号资产' + (i + 1) + '名称'" placeholder="资产名称" />
                <input v-model="asset.qty" type="text" :aria-label="'账号资产' + (i + 1) + '内容'" placeholder="数量 / 内容" />
                <button type="button" class="editor-icon" :aria-label="'移除账号资产' + (i + 1)" @click="removeDraftVaultAsset(vaultDraft, i)"><X /></button>
              </div>
              <label class="editor-field"><span>备注</span><textarea v-model="vaultDraft.notes" rows="3" placeholder="其他需要记住的信息…"></textarea></label>
            </div>
          </section>
        </fieldset>
      </div>
      <footer class="vault-editor-footer"><p v-if="saveError" role="alert">{{ saveError }}</p><span v-else>账号与角色统一保存</span><button type="button" class="editor-cancel" :disabled="saving" @click="closeVaultDraft">取消</button><button type="submit" class="editor-save" :disabled="saving || !vaultDraft.account_name.trim() || !vaultDraft.platform.trim()"><Check />{{ saving ? '保存中…' : vaultDraftMode === 'edit' ? '保存修改' : '登记入库' }}</button></footer>
    </form>
  </dialog>
</template>

<style scoped>
.vault-editor { position: fixed; inset: 0; margin: auto; width: 880px; height: 800px; max-width: calc(100% - 32px); max-height: calc(100% - 32px); overflow: hidden; border: 1px solid var(--color-hairline); border-radius: 8px; background: var(--color-canvas); color: var(--color-ink); box-shadow: 0 24px 80px #0008; container-type: inline-size; }
.vault-editor[open] { display: flex; flex-direction: column; }
.vault-editor::backdrop { background: #0e100ed4; }
.vault-editor-form { display: flex; flex-direction: column; min-height: 0; flex: 1; }
.vault-editor-heading { display: flex; align-items: center; gap: 13px; padding: 22px 28px; border-bottom: 1px solid var(--color-hairline); flex-shrink: 0; }
.vault-editor-emblem { color: var(--color-beacon); }
.vault-editor-emblem svg { width: 28px; height: 28px; stroke-width: 1; }
.vault-editor-heading h2 { font-size: 21px; font-weight: 500; margin-top: 3px; }
.vault-editor-heading p { color: var(--color-faint); font-size: 10px; letter-spacing: .1em; }
.vault-editor-heading > button { margin-left: auto; }
.vault-editor-scroll { min-height: 0; overflow-y: auto; padding-inline: 28px; scrollbar-gutter: stable; }
.editor-section { display: grid; grid-template-columns: 160px minmax(0, 1fr); gap: 28px; padding-block: 25px; border-bottom: 1px solid var(--color-hairline); }
.editor-section:last-child { border-bottom: 0; }
.editor-section-title { display: flex; align-items: baseline; gap: 10px; }
.editor-section-title > span { font-family: var(--font-display); font-size: 11px; color: var(--color-beacon); }
.editor-section-title h3 { font-size: 14px; font-weight: 500; }
.editor-section-title p { color: var(--color-faint); font-size: 11px; line-height: 1.8; margin-top: 7px; }
.editor-section-fields { min-width: 0; display: grid; gap: 14px; }
.editor-field { display: flex; flex-direction: column; gap: 7px; min-width: 0; font-size: 11px; color: var(--color-muted); }
input:not([type='checkbox']), textarea { min-width: 0; width: 100%; border: 1px solid var(--color-hairline); background: var(--color-chrome); border-radius: 4px; min-height: 36px; padding: 8px 10px; color: var(--color-ink); font-size: 12px; }
input::placeholder, textarea::placeholder { color: var(--color-faint); }
textarea { resize: vertical; }
.editor-platforms { display: flex; flex-wrap: wrap; gap: 7px 14px; margin-top: -5px; }
.editor-platforms button { font-size: 11px; color: var(--color-faint); padding-block: 4px; cursor: pointer; border-bottom: 1px solid transparent; }
.editor-platforms button[aria-pressed='true'] { color: var(--color-beacon); border-color: currentColor; }
.editor-field-pair { display: flex; gap: 12px; }
.editor-field-pair > * { flex: 1; }
.editor-help { font-size: 11px; color: var(--color-faint); line-height: 1.8; }
.editor-text { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; color: var(--color-beacon); cursor: pointer; min-height: 32px; justify-self: start; }
.editor-text svg { width: 12px; height: 12px; }
.editor-character { padding: 0 0 20px; border-bottom: 1px solid var(--color-hairline); }
.editor-character:last-child { padding-bottom: 0; border-bottom: 0; }
.editor-character-bar { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; margin-bottom: 10px; }
.editor-character-bar > span { font-size: 10px; font-family: var(--font-mono); color: var(--color-faint); margin-right: auto; }
.editor-check { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--color-muted); cursor: pointer; }
.editor-check.is-important { color: var(--color-beacon); }
.editor-character-fields { display: grid; grid-template-columns: 1.3fr 1fr 1fr; gap: 9px; }
.editor-query { margin-top: 12px; }
.editor-asset-row { display: grid; grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr) 28px; gap: 8px; align-items: center; }
.editor-character .editor-asset-row { margin-top: 9px; }
.editor-icon { display: grid; place-items: center; width: 28px; height: 28px; color: var(--color-faint); cursor: pointer; border-radius: 4px; }
.editor-icon:hover { color: var(--color-ink); background: var(--color-raised); }
.editor-icon svg { width: 15px; height: 15px; }
.vault-editor-footer { display: flex; align-items: center; gap: 12px; padding: 17px 28px; background: var(--color-chrome); border-top: 1px solid var(--color-hairline); flex-shrink: 0; }
.vault-editor-footer > span, .vault-editor-footer > p { margin-right: auto; font-size: 11px; color: var(--color-faint); }
.vault-editor-footer > p { color: var(--color-alert); max-width: 55%; overflow-wrap: anywhere; }
.editor-cancel, .editor-save { padding: 7px 14px; font-size: 12px; cursor: pointer; flex-shrink: 0; border-radius: 4px; }
.editor-cancel { color: var(--color-muted); }
.editor-save { display: inline-flex; align-items: center; gap: 7px; color: var(--color-beacon); border: 1px solid color-mix(in srgb, var(--color-beacon) 35%, transparent); background: color-mix(in srgb, var(--color-beacon) 8%, transparent); }
.editor-save svg { width: 14px; height: 14px; }
button:disabled { opacity: .4; cursor: default; }
:where(button, input, textarea):focus-visible { outline: 2px solid var(--color-beacon); outline-offset: 3px; }
@container (max-width: 700px) {
  .vault-editor-heading { padding: 15px 18px; }
  .vault-editor-scroll { padding-inline: 18px; }
  .editor-section { grid-template-columns: 1fr; gap: 15px; padding-block: 20px; }
  .editor-section-title > div { flex: 1; display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; }
  .editor-section-title p { display: none; }
  .editor-section-title .editor-text { margin-left: auto; }
  .vault-editor-footer { padding: 12px 18px; }
}
@media (max-height: 600px) { .vault-editor-heading { padding-block: 12px; } .vault-editor-heading p { display: none; } .vault-editor-heading h2 { font-size: 18px; } }
</style>
