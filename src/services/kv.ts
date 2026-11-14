import { invoke } from "@tauri-apps/api/core";

// 桌面端以 SQLite app_settings 为唯一权威，内存只是读缓存；
// localStorage 只在纯浏览器调试（没有 SQLite）时使用。
const mem = new Map<string, string>();
let hydrated = false;
let hydrateJob: Promise<void> | null = null;
// 同一个键的写入按调用顺序落库，避免快速连点时旧值后到覆盖新值。
const pending = new Map<string, Promise<unknown>>();

function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function lsGet(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function lsSet(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* quota / private mode */
  }
}

function lsRemove(key: string) {
  try {
    localStorage.removeItem(key);
  } catch {
    /* ignore */
  }
}

function enqueue(key: string, run: () => Promise<unknown>): Promise<unknown> {
  const prev = pending.get(key) ?? Promise.resolve();
  const next = prev.catch(() => {}).then(run);
  pending.set(key, next);
  const cleanup = () => {
    if (pending.get(key) === next) pending.delete(key);
  };
  void next.then(cleanup, cleanup);
  return next;
}

export function kvGet(key: string): string | null {
  if (mem.has(key)) return mem.get(key)!;
  return isTauri() ? null : lsGet(key);
}

export function kvSet(key: string, value: string): void {
  mem.set(key, value);
  if (!isTauri()) {
    lsSet(key, value);
    return;
  }
  // 无等待调用方的写入仍交给 main.ts 的全局保存失败提示。
  void enqueue(key, () => invoke("save_setting", { key, value })).catch(error => { throw error; });
}

/** 用户明确保存的内容：落库成功后才发布缓存，失败保留旧值并交给调用方处理。 */
export async function kvSetConfirmed(key: string, value: string): Promise<void> {
  if (!isTauri()) {
    localStorage.setItem(key, value);
    mem.set(key, value);
    return;
  }
  await enqueue(key, async () => {
    await invoke("save_setting", { key, value });
    mem.set(key, value);
  });
}

export function kvRemove(key: string): void {
  mem.delete(key);
  if (!isTauri()) {
    lsRemove(key);
    return;
  }
  void enqueue(key, () => invoke("delete_setting", { key })).catch(error => { throw error; });
}

export function kvGetJson<T>(key: string, fallback: T): T {
  const raw = kvGet(key);
  if (raw == null || raw === "") return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function kvSetJson(key: string, value: unknown): void {
  kvSet(key, JSON.stringify(value));
}

/** 备份或替换数据库前等待写入，失败时阻止继续。 */
export async function kvFlush(): Promise<void> {
  await Promise.all([...pending.values()]);
}

export async function hydrateKv(): Promise<void> {
  if (hydrated || !isTauri()) return;
  if (hydrateJob) return hydrateJob;
  hydrateJob = (async () => {
    const raw = await invoke<Record<string, string>>("get_all_settings");
    mem.clear();
    for (const [k, v] of Object.entries(raw ?? {})) {
      if (typeof v === "string") mem.set(k, v);
    }
    hydrated = true;
  })();
  try {
    await hydrateJob;
  } finally {
    hydrateJob = null;
  }
}
