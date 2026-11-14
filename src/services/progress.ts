import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

export const progressRevision = ref(0);

// 收集/图鉴进度（progress_items）和每日打卡（daily_checks）的前端入口。
// 启动时整表读进内存，读取同步；保存时只把和内存不同的行写回 SQLite。
// 浏览器调试没有 SQLite，用 localStorage 存同样的内存结构。

type Ops =
  | { op: "put"; scope: string; character_id: string; item_id: string; state: unknown }
  | { op: "del"; scope: string; character_id: string; item_id: string }
  | { op: "check" | "uncheck"; scope: string; day: string; item_id: string };
type ItemOp = Extract<Ops, { op: "put" | "del" }>;

/** scope → characterId → itemId → 序列化后的 state（用于比较）与原值 */
const items = new Map<string, Map<string, Map<string, { raw: string; value: unknown }>>>();
/** scope|day → itemId 集合 */
const checks = new Map<string, Set<string>>();
let queue: Promise<unknown> = Promise.resolve();
const LS_KEY = "waypoint_progress_debug";

function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function bucket(scope: string, characterId: string) {
  let byChar = items.get(scope);
  if (!byChar) items.set(scope, (byChar = new Map()));
  let rows = byChar.get(characterId);
  if (!rows) byChar.set(characterId, (rows = new Map()));
  return rows;
}

function checkKey(scope: string, day: string) {
  return `${scope}|${day}`;
}

function persistDebug() {
  try {
    const out = {
      items: [...items].flatMap(([scope, byChar]) =>
        [...byChar].flatMap(([ch, rows]) => [...rows].map(([id, r]) => [scope, ch, id, r.value]))),
      checks: [...checks].flatMap(([k, set]) => {
        const [scope, day] = k.split("|");
        return [...set].map(id => [scope, day, id]);
      }),
    };
    localStorage.setItem(LS_KEY, JSON.stringify(out));
  } catch {
    /* 私密模式 */
  }
}

function send(ops: Ops[]) {
  if (!ops.length) return;
  if (!isTauri()) {
    persistDebug();
    return;
  }
  // 串行写，保证落库顺序和调用顺序一致；失败交给全局 unhandledrejection 提示。
  queue = queue.catch(() => {}).then(() => invoke("apply_progress_ops", { ops }));
  void queue;
}

type Snapshot = { items: [string, string, string, unknown][]; checks: [string, string, string][] };

function load(snapshot: Snapshot) {
  items.clear();
  checks.clear();
  for (const [scope, ch, id, value] of snapshot.items ?? []) {
    bucket(scope, ch).set(id, { raw: JSON.stringify(value), value });
  }
  for (const [scope, day, id] of snapshot.checks ?? []) {
    const k = checkKey(scope, day);
    let set = checks.get(k);
    if (!set) checks.set(k, (set = new Set()));
    set.add(id);
  }
  progressRevision.value++;
}

export async function hydrateProgress(): Promise<void> {
  if (isTauri()) {
    load(await invoke<Snapshot>("get_progress_snapshot"));
    return;
  }
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (raw) load(JSON.parse(raw));
  } catch {
    /* 调试数据坏了就从空开始 */
  }
}

/** 某个 scope 下一个角色的全部条目，按写入顺序。全局进度用 characterId = ''。 */
export function progressEntries<T>(scope: string, characterId = ""): [string, T][] {
  void progressRevision.value;
  const rows = items.get(scope)?.get(characterId);
  if (!rows) return [];
  return [...rows].map(([id, r]) => [id, r.value as T]);
}

export function progressMap<T>(scope: string, characterId = ""): Record<string, T> {
  return Object.fromEntries(progressEntries<T>(scope, characterId));
}

export function progressGet<T>(scope: string, itemId: string, characterId = ""): T | undefined {
  void progressRevision.value;
  return items.get(scope)?.get(characterId)?.get(itemId)?.value as T | undefined;
}

/** 有数据的角色 id 列表 */
export function progressCharacters(scope: string): string[] {
  return [...(items.get(scope)?.keys() ?? [])].filter(ch => (items.get(scope)?.get(ch)?.size ?? 0) > 0);
}

function replacementOps(scope: string, entries: Iterable<[string, unknown]>, characterId: string): ItemOp[] {
  const rows = bucket(scope, characterId);
  const seen = new Set<string>();
  const ops: ItemOp[] = [];
  for (const [id, value] of entries) {
    seen.add(id);
    const raw = JSON.stringify(value);
    if (rows.get(id)?.raw === raw) continue;
    ops.push({ op: "put", scope, character_id: characterId, item_id: id, state: JSON.parse(raw) });
  }
  for (const id of [...rows.keys()]) {
    if (seen.has(id)) continue;
    ops.push({ op: "del", scope, character_id: characterId, item_id: id });
  }
  return ops;
}

function applyItemOps(ops: ItemOp[]) {
  for (const op of ops) {
    const rows = bucket(op.scope, op.character_id);
    if (op.op === "put") {
      const raw = JSON.stringify(op.state);
      rows.set(op.item_id, { raw, value: JSON.parse(raw) });
    } else {
      rows.delete(op.item_id);
    }
  }
  if (ops.length) progressRevision.value++;
}

/** 用完整的新列表替换某个角色在该 scope 下的全部条目：只写变化的行，缺席的行删除。 */
export function progressReplace(scope: string, entries: Iterable<[string, unknown]>, characterId = ""): void {
  const ops = replacementOps(scope, entries, characterId);
  applyItemOps(ops);
  send(ops);
}

/** 整批落库成功才发布内存状态，失败后的重试仍会生成完整差异。 */
async function commitItems(build: () => Ops[]): Promise<void> {
  const job = queue.catch(() => {}).then(async () => {
    const ops = build();
    if (!ops.length) return;
    if (isTauri()) await invoke("apply_progress_ops", { ops });
    applyItemOps(ops.filter((op): op is ItemOp => op.op === "put" || op.op === "del"));
    for (const op of ops) {
      if (op.op !== "check" && op.op !== "uncheck") continue;
      const key = checkKey(op.scope, op.day);
      let set = checks.get(key);
      if (!set) checks.set(key, (set = new Set()));
      if (op.op === "check") set.add(op.item_id);
      else set.delete(op.item_id);
    }
    if (!isTauri()) persistDebug();
  });
  queue = job;
  await job;
}

/** 在一个事务中替换角色的多个进度类别。 */
export function replaceProgressScopes(scopes: Record<string, [string, unknown][]>, characterId: string): Promise<void> {
  if (!characterId) throw new Error("请先选择记录所属角色");
  const snapshot = JSON.parse(JSON.stringify(scopes)) as typeof scopes;
  return commitItems(() => Object.entries(snapshot).flatMap(([scope, entries]) =>
    replacementOps(scope, entries, characterId)));
}

/** 原子更新指定条目，不覆盖同类别中的其他记录。 */
export function setProgressEntries(scope: string, entries: [string, unknown][], characterId: string): Promise<void> {
  if (!characterId) throw new Error("请先选择记录所属角色");
  const snapshot = JSON.parse(JSON.stringify(entries)) as [string, unknown][];
  return commitItems(() => snapshot.map(([id, state]) => ({
    op: "put" as const, scope, character_id: characterId, item_id: id, state,
  })));
}

/** 写单个条目；value 为 undefined 时删除。 */
export function progressSet(scope: string, itemId: string, value: unknown, characterId = ""): void {
  const rows = bucket(scope, characterId);
  if (value === undefined) {
    if (!rows.delete(itemId)) return;
    progressRevision.value++;
    send([{ op: "del", scope, character_id: characterId, item_id: itemId }]);
    return;
  }
  const raw = JSON.stringify(value);
  if (rows.get(itemId)?.raw === raw) return;
  rows.set(itemId, { raw, value: JSON.parse(raw) });
  progressRevision.value++;
  send([{ op: "put", scope, character_id: characterId, item_id: itemId, state: value }]);
}

/** 将旧全局记录归入角色，保留角色已有值；整批落库成功后才更新内存。 */
export async function claimGlobalProgress(scopes: string[], characterId: string, merge?: (legacy: unknown, current: unknown) => unknown): Promise<void> {
  if (!characterId) throw new Error("请先选择记录所属角色");
  await commitItems(() => {
    const ops: ItemOp[] = [];
    for (const scope of scopes) {
      const target = bucket(scope, characterId);
      for (const [id, row] of bucket(scope, "")) {
        if (!target.has(id)) ops.push({ op: "put", scope, character_id: characterId, item_id: id, state: row.value });
        else if (merge) ops.push({ op: "put", scope, character_id: characterId, item_id: id, state: merge(row.value, target.get(id)!.value) });
        ops.push({ op: "del", scope, character_id: "", item_id: id });
      }
    }
    return ops;
  });
}

export function dailyChecked(scope: string, day: string, characterId: string): Set<string> {
  const prefix = `${day}/`;
  return new Set(characterId ? progressEntries(scope, characterId)
    .filter(([id, value]) => id.startsWith(prefix) && value === true)
    .map(([id]) => id.slice(prefix.length)) : []);
}

/** 角色日常复用 progress_items；一次事务提交多个项目，不改变其他日期或角色。 */
export async function setCharacterDailyChecks(
  characterId: string, day: string, groups: Record<string, string[]>, checked: boolean,
): Promise<void> {
  if (!characterId) throw new Error("请先选择光遇角色");
  const entries = Object.entries(groups).map(([scope, ids]) => [scope, [...new Set(ids)]] as const);
  await commitItems(() => entries.flatMap(([scope, ids]) => ids.flatMap(id => {
    const itemId = `${day}/${id}`;
    const exists = progressGet(scope, itemId, characterId) === true;
    if (exists === checked) return [];
    return [checked
      ? { op: "put" as const, scope, character_id: characterId, item_id: itemId, state: true }
      : { op: "del" as const, scope, character_id: characterId, item_id: itemId }];
  })));
}

export function legacyDailyCheckCount(scopes: string[]): number {
  return [...checks].filter(([key]) => scopes.some(scope => key.startsWith(`${scope}|`)))
    .reduce((sum, [, ids]) => sum + ids.size, 0);
}

/** 用户选定归属后，整批迁移旧共享打卡；同一事务移除旧记录，失败仍可重试。 */
export async function claimDailyChecks(scopes: string[], characterId: string): Promise<void> {
  if (!characterId) throw new Error("请先选择光遇角色");
  await commitItems(() => {
    const ops: Ops[] = [];
    for (const [key, ids] of checks) {
      const [scope, day] = key.split("|");
      if (!scope || !day || !scopes.includes(scope)) continue;
      for (const id of ids) {
        ops.push({ op: "put", scope, character_id: characterId, item_id: `${day}/${id}`, state: true });
        ops.push({ op: "uncheck", scope, day, item_id: id });
      }
    }
    return ops;
  });
}

/** 备份或替换数据库前等待写入，失败时阻止继续。 */
export async function progressFlush(): Promise<void> {
  await queue;
}
