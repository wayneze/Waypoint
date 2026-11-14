// 账号金库页：登录号、角色挂板、Steam 库。
import { optimistic } from '../services/optimistic';
import { ref, computed } from 'vue';
import { askConfirmation } from './confirmation';
import {
  type SteamLibraryGame,
  type VaultAsset,
  type VaultCharacter,
  type VaultItem,
} from '../types';
import {
  addVaultItem,
  deleteVaultItem,
  getSteamLibrary,
  refreshSteamLibrary,
  updateVaultItem,
} from '../services/api';
import { exportVaultPlain, importVaultPlain, pickOpenFile, pickSaveFile, saveSteamWebApiKey } from '../services/systemApi';
import { getVaultItems } from '../services/api';
import { GAMES, type GameStamp, gameHasRoster, gameMetaOf, resolveVaultGameId } from '../services/games';
import { kvGet, kvRemove } from '../services/kv';
import { isSteamVaultPlatform, newVaultAsset, newVaultCharacter, pruneVaultItem } from '../services/vault';
import {
  activeCharacter,
  allAccountTasks,
  charQuickAddCategory,
  showToast,
  state,
  vaultIdentityName,
  vaultList,
} from './core';


export const steamLibrary = ref<SteamLibraryGame[]>([]);
export const steamLibraryRefreshingId = ref<string | null>(null);
/** 密钥本身存在 Windows 凭据管理器，前端只知道配没配。 */
export const steamWebApiKeyConfigured = ref(false);
export const steamWebApiKeyDraft = ref('');
export const steamWebApiKeySaved = ref(false);

export function steamGamesOf(vaultItemId: string): SteamLibraryGame[] {
  return steamLibrary.value.filter(g => g.vault_item_id === vaultItemId);
}

export async function loadSteamLibrary() {
  try {
    steamLibrary.value = await getSteamLibrary();
  } catch (err) {
    console.warn('Failed to load steam library:', err);
  }
}

export async function syncSteamAccount(vaultItemId: string) {
  if (steamLibraryRefreshingId.value) throw new Error('已有 Steam 同步正在进行，请稍候');
  steamLibraryRefreshingId.value = vaultItemId;
  try {
    const games = await refreshSteamLibrary(vaultItemId);
    steamLibrary.value = [...steamLibrary.value.filter(g => g.vault_item_id !== vaultItemId), ...games];
    return games;
  } finally { steamLibraryRefreshingId.value = null; }
}

export async function handleClearSteamWebApiKey() {
  steamWebApiKeyDraft.value = '';
  await handleSaveSteamWebApiKey();
}

export async function handleSaveSteamWebApiKey() {
  const value = steamWebApiKeyDraft.value.trim();
  await saveSteamWebApiKey(value);
  steamWebApiKeyConfigured.value = !!value;
  steamWebApiKeyDraft.value = '';
  steamWebApiKeySaved.value = true;
  showToast(value ? '已保存 Steam Web API key' : '已清除 Steam Web API key', '存在 Windows 凭据管理器，不进数据库和备份');
}

export const selectedVaultGroup = ref<string>('all');

export const vaultGroups = computed(() => {
  const counts: Record<string, number> = {};
  for (const v of vaultList.value) {
    const id = resolveVaultGameId(v.platform);
    counts[id] = (counts[id] || 0) + 1;
  }

  return Object.entries(counts).map(([id, count]) => {
    const known = GAMES.some(g => g.id === id);
    return {
      id,
      name: known ? gameMetaOf(id).label : isSteamVaultPlatform(id) ? 'Steam' : vaultList.value.find(item => resolveVaultGameId(item.platform) === id)?.platform || id,
      stamp: known ? gameMetaOf(id).stamp : 'gal' as GameStamp,
      count
    };
  });
});

const hasStar = (v: VaultItem) => v.characters.some(c => c.starred);

export const filteredVaultList = computed(() => {
  const list = selectedVaultGroup.value === 'all'
    ? vaultList.value
    : vaultList.value.filter(v => resolveVaultGameId(v.platform) === selectedVaultGroup.value);
  return [...list].sort((a, b) => Number(hasStar(b)) - Number(hasStar(a)));
});

export const selectedVaultGroupLabel = computed(() => {
  if (selectedVaultGroup.value === 'all') return '全部';
  return vaultGroups.value.find(g => g.id === selectedVaultGroup.value)?.name || selectedVaultGroup.value;
});

export function selectVaultGroup(groupId: string) {
  selectedVaultGroup.value = groupId;
}

export function isSkyVaultPlatform(platform: string): boolean {
  return resolveVaultGameId(platform) === 'sky';
}

// 探索角色包括未挂日常板的角色；不借用另一个角色的查询 ID。
export const skyCharacters = computed(() => vaultList.value
  .filter(v => isSkyVaultPlatform(v.platform) && !v.locked)
  .flatMap(v => v.characters.map(ch => ({ ...ch, accountName: vaultIdentityName(v) }))));
export const activeSkyCharacter = computed(() =>
  skyCharacters.value.find(ch => ch.id === activeCharacter.value));

/** 把探索页旧 KV 里的查询ID迁到唯一一个空的光遇角色上。迁不走就先留着。 */
export async function migrateSkyLegacyQueryId() {
  const leftover = (kvGet('waypoint_sky_game_id') || '').trim();
  if (!leftover) return;
  const skyChars = vaultList.value.flatMap(v =>
    isSkyVaultPlatform(v.platform) ? v.characters.map(ch => ({ v, ch })) : []
  );
  if (skyChars.some(x => (x.ch.query_id || '').trim() === leftover)) {
    kvRemove('waypoint_sky_game_id');
    return;
  }
  const empty = skyChars.filter(x => !(x.ch.query_id || '').trim());
  if (empty.length !== 1) return;
  empty[0].ch.query_id = leftover;
  const saved = pruneVaultItem(empty[0].v);
  await updateVaultItem(saved);
  const idx = vaultList.value.findIndex(v => v.id === saved.id);
  if (idx !== -1) vaultList.value[idx] = saved;
  kvRemove('waypoint_sky_game_id');
}

export function openAddVaultModal(platform = '') {
  const gameId = resolveVaultGameId(platform);
  const characters = gameHasRoster(gameId) ? [newVaultCharacter()] : [];
  state.newVault = {
    platform,
    account_name: '',
    server: '',
    notes: '',
    steamid64: '',
    assets: [],
    characters
  };
  state.showAddVaultModal = true;
}

// 账号金库管理
export async function handleAddVaultItem() {
  if (!state.newVault.account_name.trim()) return;
  const item = pruneVaultItem({
    id: `v_${Date.now()}`,
    platform: state.newVault.platform,
    account_name: state.newVault.account_name,
    server: state.newVault.server || undefined,
    notes: state.newVault.notes || undefined,
    steamid64: state.newVault.steamid64 || undefined,
    assets: state.newVault.assets,
    characters: state.newVault.characters
  });
  await addVaultItem(item);
  vaultList.value.push(item);
  for (const ch of item.characters) {
    if (!allAccountTasks.value[ch.id]) allAccountTasks.value[ch.id] = [];
    if (!charQuickAddCategory[ch.id]) charQuickAddCategory[ch.id] = 'daily';
  }
  if (item.characters[0]) activeCharacter.value = item.characters[0].id;
  state.newVault = { platform: '', account_name: '', server: '', notes: '', steamid64: '', assets: [], characters: [] };
  state.showAddVaultModal = false;
  showToast('金库已存入', `【${vaultIdentityName(item)}】已登记`);
}

async function toggleVaultCharFlag(vaultId: string, charId: string, flag: 'track_daily' | 'starred') {
  const item = vaultList.value.find(v => v.id === vaultId);
  const ch = item?.characters.find(c => c.id === charId);
  if (!item || !ch) return null;
  const next = !ch[flag];
  let saved = item;
  await optimistic(
    () => { ch[flag] = next; saved = pruneVaultItem(item); },
    () => { ch[flag] = !next; },
    () => updateVaultItem(saved)
  );
  const idx = vaultList.value.findIndex(v => v.id === saved.id);
  if (idx !== -1) vaultList.value[idx] = saved;
  const draft = editingVault.value?.id === saved.id
    ? editingVault.value.characters.find(c => c.id === charId)
    : undefined;
  if (draft) draft[flag] = next;
  return ch;
}

export async function handleToggleVaultStar(vaultId: string, charId: string) {
  await toggleVaultCharFlag(vaultId, charId, 'starred');
}

export async function handleToggleVaultTrackDaily(vaultId: string, charId: string) {
  const ch = await toggleVaultCharFlag(vaultId, charId, 'track_daily');
  if (!ch) return;
  showToast(
    ch.track_daily ? '已挂到日常板' : '已移出日常板',
    ch.track_daily ? `【${ch.name}】会出现在日常 · 周常` : `【${ch.name}】仍留在金库`
  );
}

export async function handleDeleteVaultItem(id: string) {
  const item = vaultList.value.find(v => v.id === id);
  if (!item || !await askConfirmation({
    title: '删除这个账号？',
    message: '将移除账号、角色与账号资产，并清除该账号的 Steam 游戏库缓存。角色会退出日课目录；此操作不能直接撤销。',
    confirmLabel: '删除账号',
    details: [{ label: '账号', value: vaultIdentityName(item) }, { label: '平台', value: item.platform }],
  })) return;
  await deleteVaultItem(id);
  vaultList.value = vaultList.value.filter(v => v.id !== id);
  steamLibrary.value = steamLibrary.value.filter(g => g.vault_item_id !== id);
  await (await import('./gameActivity')).loadGameActivity();
  if (item) {
    const next = { ...allAccountTasks.value };
    for (const ch of item.characters || []) delete next[ch.id];
    allAccountTasks.value = next;
  }
  showToast('已移除', '账号已从金库注销');
}

// 编辑账号金库项全字段
export const editingVault = ref<VaultItem | null>(null);
export const showEditVaultModal = ref(false);

export function handleOpenEditVault(item: VaultItem) {
  if (item.locked) return;
  editingVault.value = {
    ...item,
    steamid64: item.steamid64 || '',
    assets: (item.assets || []).map(a => ({ ...a })),
    characters: (item.characters || []).map(ch => ({
      ...ch,
      assets: (ch.assets || []).map(a => ({ ...a }))
    }))
  };
  showEditVaultModal.value = true;
}

export async function handleSaveEditVault() {
  if (!editingVault.value) return;
  const prev = vaultList.value.find(v => v.id === editingVault.value!.id);
  const saved = pruneVaultItem(editingVault.value);
  await updateVaultItem(saved);
  if (isSteamVaultPlatform(saved.platform) || (prev && isSteamVaultPlatform(prev.platform))) {
    await loadSteamLibrary();
    await (await import('./gameActivity')).loadGameActivity();
  }
  const idx = vaultList.value.findIndex(v => v.id === saved.id);
  if (idx !== -1) {
    vaultList.value[idx] = saved;
  }
  const keep = new Set(saved.characters.map(ch => ch.id));
  const next = { ...allAccountTasks.value };
  for (const ch of prev?.characters || []) {
    if (!keep.has(ch.id)) delete next[ch.id];
  }
  for (const ch of saved.characters) {
    if (!next[ch.id]) next[ch.id] = [];
    if (!charQuickAddCategory[ch.id]) charQuickAddCategory[ch.id] = 'daily';
  }
  allAccountTasks.value = next;
  editingVault.value = saved;
  showEditVaultModal.value = false;
  showToast('保存成功', `已更新【${vaultIdentityName(saved)}】`);
}

export function addDraftVaultCharacter(target: { id?: string; platform?: string; characters: VaultCharacter[]; server?: string }) {
  const ch = newVaultCharacter();
  const last = target.characters[target.characters.length - 1];
  ch.server = last?.server || target.server || '';
  target.characters.push(ch);
}

export function syncDraftVaultRoster(target: { id?: string; platform: string; characters: VaultCharacter[] }) {
  if (!gameHasRoster(resolveVaultGameId(target.platform))) {
    target.characters = [];
    return;
  }
  if (target.characters.length === 0) target.characters.push(newVaultCharacter());
}

export function removeDraftVaultCharacter(target: { characters: VaultCharacter[] }, index: number) {
  target.characters.splice(index, 1);
}

export function addDraftVaultAsset(target: { assets: VaultAsset[] }) {
  target.assets.push(newVaultAsset());
}

export function removeDraftVaultAsset(target: { assets: VaultAsset[] }, index: number) {
  target.assets.splice(index, 1);
}

// 保留 JSON 导入/导出供单独交换金库使用；换机的主入口是设置页完整加密备份。
export async function handleExportVaultPlain() {
  if (!await askConfirmation({
    title: '导出明文金库？',
    message: '文件包含账号和备注里的密码，任何能读取文件的人都能查看。导入新电脑后请删除这份明文文件。',
    confirmLabel: '导出明文文件',
  })) return;
  const dest = await pickSaveFile('导出金库（明文）', 'JSON (*.json)|*.json', 'waypoint-vault.json');
  if (!dest) return;
  const n = await exportVaultPlain(dest);
  showToast('金库已导出', `${n} 个账号 · ${dest}`);
}

export async function handleImportVaultPlain() {
  const src = await pickOpenFile('导入金库', 'JSON (*.json)|*.json');
  if (!src) return;
  const n = await importVaultPlain(src);
  vaultList.value = await getVaultItems();
  showToast('金库已导入', `${n} 个账号，已用本机账户重新加密`);
}
