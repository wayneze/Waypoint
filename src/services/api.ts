import { invoke } from "@tauri-apps/api/core";
import { normalizeVaultItem } from "./vault";
import { resolveVaultGameId } from "./games";
import type { TaskItem, VaultItem, GameRecord, CustomMemo, SteamLibraryGame } from "../types";
import { kvFlush, kvGet, kvSet, kvSetConfirmed } from "./kv";

export const isTauri = () => {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
};

function parseJsonArray<T>(raw: string | null): T[] | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed as T[] : null;
  } catch {
    return null;
  }
}

// 获取所有任务 (全账号跨游戏)
export async function getAllTasks(): Promise<TaskItem[]> {
  if (isTauri()) return invoke<TaskItem[]>("get_tasks", { characterId: "all" });

  const items = await getVaultItems();
  const result: TaskItem[] = [];
  for (const item of items) {
    for (const ch of item.characters || []) {
      result.push(...await getTasks(ch.id));
    }
  }
  return result;
}

// 获取角色任务
export async function getTasks(characterId: string): Promise<TaskItem[]> {
  if (isTauri()) return invoke<TaskItem[]>("get_tasks", { characterId });
  return parseJsonArray<TaskItem>(kvGet(`waypoint_tasks_${characterId}`)) || [];
}

// 切换任务状态
export async function toggleTask(taskId: string, done: boolean, characterId?: string): Promise<void> {
  if (isTauri()) {
    await invoke("toggle_task", { taskId, done });
    // 托盘上的剩余计数跟着刷新
    void invoke("refresh_cycle_now");
    return;
  }

  if (characterId) {
    const tasks = await getTasks(characterId);
    const target = tasks.find(t => t.id === taskId);
    if (target) {
      target.done = done;
      kvSet(`waypoint_tasks_${characterId}`, JSON.stringify(tasks));
    }
  }
}

// 添加任务
export async function addTask(task: TaskItem): Promise<void> {
  if (isTauri()) {
    await invoke("add_task", { task });
    return;
  }

  const tasks = await getTasks(task.character_id);
  tasks.push(task);
  kvSet(`waypoint_tasks_${task.character_id}`, JSON.stringify(tasks));
}

// 删除任务
export async function deleteTask(taskId: string, characterId?: string): Promise<void> {
  if (isTauri()) {
    await invoke("delete_task", { taskId });
    return;
  }

  if (characterId) {
    let tasks = await getTasks(characterId);
    tasks = tasks.filter(t => t.id !== taskId);
    kvSet(`waypoint_tasks_${characterId}`, JSON.stringify(tasks));
  }
}

function sanitizeVaultItem(raw: VaultItem): VaultItem {
  return normalizeVaultItem(raw, resolveVaultGameId);
}

function persistVaultItems(items: VaultItem[]) {
  kvSet("waypoint_vault_items", JSON.stringify(items.map(sanitizeVaultItem)));
}

// 获取账号金库
export async function getVaultItems(): Promise<VaultItem[]> {
  if (isTauri()) {
    const items = await invoke<VaultItem[]>("get_vault_items");
    return (items || []).map(sanitizeVaultItem);
  }
  return (parseJsonArray<VaultItem>(kvGet("waypoint_vault_items")) || []).map(sanitizeVaultItem);
}

// 添加金库项
export async function addVaultItem(item: VaultItem): Promise<void> {
  if (isTauri()) {
    await invoke("add_vault_item", { item });
    return;
  }

  const items = await getVaultItems();
  items.push(sanitizeVaultItem(item));
  persistVaultItems(items);
}

// 删除金库项
export async function deleteVaultItem(itemId: string): Promise<void> {
  if (isTauri()) {
    await invoke("delete_vault_item", { itemId });
    return;
  }

  let items = await getVaultItems();
  items = items.filter(i => i.id !== itemId);
  persistVaultItems(items);
  persistSteamLibrary(readLocalSteamLibrary().filter(g => g.vault_item_id !== itemId));
}

// 获取单机与Galgame档案
export async function getGameRecords(): Promise<GameRecord[]> {
  if (isTauri()) return invoke<GameRecord[]>("get_game_records");
  return parseJsonArray<GameRecord>(kvGet("waypoint_game_records")) || [];
}

// 新增或编辑游戏档案
let gameRecordWrite: Promise<unknown> = Promise.resolve();
function queueGameWrite<T>(write: () => Promise<T>): Promise<T> {
  const result = gameRecordWrite.catch(() => {}).then(write);
  gameRecordWrite = result;
  return result;
}
export function saveGameRecord(record: GameRecord, creating: boolean): Promise<void> {
  return queueGameWrite(() => writeGameRecord(record, creating));
}
export function updateGameRecord(id: string, update: (current: GameRecord) => GameRecord): Promise<GameRecord> {
  return queueGameWrite(async () => {
    const current = (await getGameRecords()).find(r => r.id === id);
    if (!current) throw new Error('游戏档案不存在');
    const record = update(current);
    await writeGameRecord(record, false);
    return record;
  });
}
export interface LibraryRemovalTarget { record_id?: string; guide_id: string; guide_path: string }
export interface LibraryRemovalFile { path: string; kind: 'game' | 'guide'; blocked: string | null }
export const previewLibraryRemoval = (target: LibraryRemovalTarget) =>
  invoke<{ title: string; files: LibraryRemovalFile[] }>('preview_library_removal', { target });
export function removeLibraryGame(target: LibraryRemovalTarget, selected: LibraryRemovalFile[]) {
  return queueGameWrite(() => invoke<void>('remove_library_game', { target, selected }));
}
async function writeGameRecord(record: GameRecord, creating: boolean): Promise<void> {
  if (isTauri()) {
    await invoke("save_game_record", { record, creating });
    return;
  }

  const records = await getGameRecords();
  const index = records.findIndex(item => item.id === record.id);
  if (creating) {
    if (index !== -1) throw new Error("游戏档案已存在");
    records.push(record);
  } else {
    if (index === -1) throw new Error("游戏档案已不存在，请刷新书库");
    records[index] = record;
  }
  await kvSetConfirmed("waypoint_game_records", JSON.stringify(records));
}

// 更新账号金库项 (全字段)
export async function updateVaultItem(item: VaultItem): Promise<void> {
  if (isTauri()) {
    await invoke("update_vault_item", { item });
    return;
  }

  const items = await getVaultItems();
  const idx = items.findIndex(v => v.id === item.id);
  if (idx !== -1) {
    items[idx] = sanitizeVaultItem(item);
    persistVaultItems(items);
  }
}

export async function updateTaskTitle(taskId: string, title: string, characterId?: string): Promise<void> {
  if (isTauri()) {
    await invoke("update_task_title", { taskId, title });
    return;
  }
  if (!characterId) return;
  const tasks = await getTasks(characterId);
  const target = tasks.find(t => t.id === taskId);
  if (!target) return;
  target.title = title;
  kvSet(`waypoint_tasks_${characterId}`, JSON.stringify(tasks));
}

export async function updateTaskScheduleTime(taskId: string, scheduleTime: string | null, characterId?: string): Promise<void> {
  if (isTauri()) {
    await invoke("update_task_schedule_time", { taskId, scheduleTime: scheduleTime || null });
    return;
  }
  if (!characterId) return;
  const tasks = await getTasks(characterId);
  const target = tasks.find(t => t.id === taskId);
  if (!target) return;
  target.schedule_time = scheduleTime || undefined;
  kvSet(`waypoint_tasks_${characterId}`, JSON.stringify(tasks));
}

export async function updateTaskCategory(taskId: string, category: 'daily' | 'weekly', characterId?: string): Promise<void> {
  if (isTauri()) {
    await invoke("update_task_category", { taskId, category });
    return;
  }
  if (!characterId) return;
  const tasks = await getTasks(characterId);
  const target = tasks.find(t => t.id === taskId);
  if (!target) return;
  target.category = category;
  kvSet(`waypoint_tasks_${characterId}`, JSON.stringify(tasks));
}

/** 复位钟改了以后让后端按新钟重记各任务周期（换日由后端执行）。 */
export async function rebaseTaskCycles(): Promise<void> {
  if (!isTauri()) return;
  await kvFlush();
  await invoke("rebase_task_cycles");
}

// 获取持久化配置
export async function getSetting(key: string): Promise<string | null> {
  const cached = kvGet(key);
  if (cached != null) return cached;
  if (isTauri()) {
    try {
      return await invoke<string | null>("get_setting", { key });
    } catch (e) {
      console.warn("Tauri get_setting error:", e);
    }
  }
  return null;
}

// 保存持久化配置
export async function saveSetting(key: string, value: string): Promise<void> {
  kvSet(key, value);
}

function persistSteamLibrary(games: SteamLibraryGame[]) {
  kvSet("waypoint_steam_library", JSON.stringify(games));
}

function readLocalSteamLibrary(vaultItemId?: string): SteamLibraryGame[] {
  const raw = kvGet("waypoint_steam_library");
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw) as SteamLibraryGame[];
    if (!Array.isArray(parsed)) return [];
    if (!vaultItemId) return parsed;
    return parsed.filter(g => g.vault_item_id === vaultItemId);
  } catch {
    return [];
  }
}

export async function getSteamLibrary(vaultItemId?: string): Promise<SteamLibraryGame[]> {
  if (isTauri()) return invoke<SteamLibraryGame[]>("get_steam_library", {
        vaultItemId: vaultItemId ?? null
      });
  return readLocalSteamLibrary(vaultItemId);
}

export async function refreshSteamLibrary(vaultItemId: string): Promise<SteamLibraryGame[]> {
  if (isTauri()) {
    const games = await invoke<SteamLibraryGame[]>("refresh_steam_library", { vaultItemId });
    return games;
  }
  throw new Error("拉取 Steam 库存只在桌面端可用");
}

// 获取万象记事列表
export async function getCustomMemos(): Promise<CustomMemo[]> {
  if (isTauri()) return invoke<CustomMemo[]>("get_custom_memos");
  return parseJsonArray<CustomMemo>(kvGet("waypoint_custom_memos")) || [];
}

// 新增万象记事
export async function addCustomMemo(memo: CustomMemo): Promise<CustomMemo> {
  if (isTauri()) return invoke<CustomMemo>("add_custom_memo", { memo });

  const list = await getCustomMemos();
  list.unshift(memo);
  kvSet("waypoint_custom_memos", JSON.stringify(list));
  return memo;
}

// 更新万象记事
export async function updateCustomMemo(memo: CustomMemo): Promise<CustomMemo> {
  if (isTauri()) return invoke<CustomMemo>("update_custom_memo", { memo });

  const list = await getCustomMemos();
  const idx = list.findIndex(m => m.id === memo.id);
  if (idx !== -1) {
    list[idx] = memo;
    kvSet("waypoint_custom_memos", JSON.stringify(list));
  }
  return memo;
}

// 删除万象记事
export async function deleteCustomMemo(id: string): Promise<boolean> {
  if (isTauri()) return invoke<boolean>("delete_custom_memo", { id });

  const list = await getCustomMemos();
  const filtered = list.filter(m => m.id !== id);
  kvSet("waypoint_custom_memos", JSON.stringify(filtered));
  return true;
}

// 切换完成状态
export async function toggleCustomMemo(id: string): Promise<boolean> {
  if (isTauri()) return invoke<boolean>("toggle_custom_memo", { id });

  const list = await getCustomMemos();
  const item = list.find(m => m.id === id);
  if (item) {
    item.done = !item.done;
    item.last_completed_at = item.done ? new Date().toISOString() : undefined;
    kvSet("waypoint_custom_memos", JSON.stringify(list));
    return item.done;
  }
  return false;
}
