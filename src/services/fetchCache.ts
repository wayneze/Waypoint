import { reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// 网络数据缓存：桌面端存在独立的 cache.db（不进用户库和备份），浏览器调试用 localStorage。
// 启动时整表读进内存，读取同步；写入先改内存再异步落盘。
// cachedFetch 的策略是"先给缓存，后台刷新"：有缓存就立刻返回，过期则后台再拉，拉到后通过 onUpdate 推送。
// 缓存里存的多是解析后的结果，换了新版程序解析规则可能已变：上一版写的条目一律当作不新鲜，照常先显示再后台重拉。

type Entry = { body: string; fetched_at: number };

const mem = new Map<string, Entry>();
const LS_PREFIX = "waypoint_cache:";
const BUILD_KEY = "__build";
/** 上一版程序写入、本版还没重写过的键 */
const prevBuild = new Set<string>();

function markBuild() {
  const built = cacheGet<string>(BUILD_KEY)?.data === __BUILD_ID__;
  prevBuild.clear();
  if (!built) for (const k of mem.keys()) if (k !== BUILD_KEY) prevBuild.add(k);
  // 每次启动都重写，免得同一版用满 30 天后标记被 cache.db 的过期清理删掉
  cacheSet(BUILD_KEY, __BUILD_ID__);
}

/** 这条缓存是不是当前这版程序写的 */
export function cacheFromThisBuild(key: string): boolean {
  return mem.has(key) && !prevBuild.has(key);
}

function isTauri() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function hydrateCache(): Promise<void> {
  mem.clear();
  if (isTauri()) {
    // 缓存读不到不影响使用，只是首屏要等网络
    const all = await invoke<Record<string, Entry>>("cache_get_all").catch(e => {
      console.warn("cache_get_all failed", e);
      return {};
    });
    for (const [k, v] of Object.entries(all)) mem.set(k, v);
    markBuild();
    // 后台线程（比如托盘时重拉的光遇档期）写进 cache.db 后推过来，内存跟着换
    void listen<{ key: string; label: string; warning: string } & Entry>("cache-updated", ({ payload }) => {
      mem.set(payload.key, { body: payload.body, fetched_at: payload.fetched_at });
      prevBuild.delete(payload.key);
      mark(payload.key, payload.label, {
        state: payload.warning ? "partial" : "ok",
        fetchedAt: payload.fetched_at,
        error: payload.warning,
      });
    });
    return;
  }
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (!k?.startsWith(LS_PREFIX)) continue;
      const raw = localStorage.getItem(k);
      if (raw) mem.set(k.slice(LS_PREFIX.length), JSON.parse(raw) as Entry);
    }
  } catch {
    /* 调试缓存坏了就当没有 */
  }
  markBuild();
}

export function cacheGet<T>(key: string): { data: T; fetchedAt: number } | null {
  const entry = mem.get(key);
  if (!entry) return null;
  try {
    return { data: JSON.parse(entry.body) as T, fetchedAt: entry.fetched_at };
  } catch {
    return null;
  }
}

export function cacheSet(key: string, data: unknown): void {
  const entry: Entry = { body: JSON.stringify(data), fetched_at: Date.now() };
  mem.set(key, entry);
  prevBuild.delete(key);
  if (isTauri()) {
    invoke("cache_put", { key, body: entry.body, fetchedAt: entry.fetched_at })
      .catch(e => console.warn("cache_put failed", key, e));
    return;
  }
  try {
    localStorage.setItem(LS_PREFIX + key, JSON.stringify(entry));
  } catch {
    /* 配额满时只是少一份缓存 */
  }
}

/** partial：主数据拉到了，但有子数据源失败（error 里是原因） */
export type SourceState = "loading" | "ok" | "partial" | "stale" | "refreshing" | "error";

export interface SourceStatus {
  label: string;
  state: SourceState;
  /** 当前显示数据的拉取时间（毫秒），没有数据为 0 */
  fetchedAt: number;
  error: string;
}

/** 各数据源的最近状态，给页面上的数据源状态条用。key 与缓存键一致。 */
export const sourceStatus = reactive<Record<string, SourceStatus>>({});

function mark(key: string, label: string, patch: Partial<SourceStatus>) {
  const cur = sourceStatus[key] ?? { label, state: "loading", fetchedAt: 0, error: "" };
  sourceStatus[key] = { ...cur, label, ...patch };
}

/** 不走 cachedFetch 的数据源（比如自己拼装的光遇今日任务、剑三接口）也在状态条上报告。 */
export function reportSource(key: string, label: string, state: SourceState, error = "", fetchedAt?: number) {
  const prev = sourceStatus[key]?.fetchedAt ?? 0;
  mark(key, label, { state, error, fetchedAt: fetchedAt ?? (state === "ok" ? Date.now() : prev) });
}

export interface CachedFetchOptions<T> {
  key: string;
  /** 状态条上显示的名字 */
  label: string;
  fetch: () => Promise<T | null>;
  /** 缓存是否还新鲜；不新鲜也会先返回，再后台刷新 */
  fresh: (data: T, fetchedAt: number) => boolean;
  /** 拉到的数据是否可用（空列表之类视为失败，不覆盖缓存） */
  valid?: (data: T) => boolean;
  /** 数据可用但不完整时返回原因，状态条显示为 partial */
  warn?: (data: T) => string;
  /** 后台刷新拿到新数据时回调 */
  onUpdate?: (data: T) => void;
  force?: boolean;
}

const inflight = new Map<string, Promise<unknown>>();

function errText(e: unknown): string {
  if (e instanceof Error) return e.message;
  return String(e || "拉取失败");
}

async function fetchAndStore<T>(opts: CachedFetchOptions<T>): Promise<T | null> {
  const running = inflight.get(opts.key) as Promise<T | null> | undefined;
  if (running) return running;
  const job = (async () => {
    try {
      const data = await opts.fetch();
      if (data == null || (opts.valid && !opts.valid(data))) {
        throw new Error("数据源没有返回可用内容");
      }
      cacheSet(opts.key, data);
      const warning = opts.warn?.(data) || "";
      mark(opts.key, opts.label, { state: warning ? "partial" : "ok", fetchedAt: Date.now(), error: warning });
      return data;
    } catch (e) {
      const has = (sourceStatus[opts.key]?.fetchedAt ?? 0) > 0;
      mark(opts.key, opts.label, { state: has ? "stale" : "error", error: errText(e) });
      return null;
    } finally {
      inflight.delete(opts.key);
    }
  })();
  inflight.set(opts.key, job);
  return job;
}

export async function cachedFetch<T>(opts: CachedFetchOptions<T>): Promise<T | null> {
  const cached = cacheGet<T>(opts.key);
  const usable = cached && (!opts.valid || opts.valid(cached.data)) ? cached : null;
  if (usable && !opts.force) {
    const fresh = cacheFromThisBuild(opts.key) && opts.fresh(usable.data, usable.fetchedAt);
    const warning = fresh ? opts.warn?.(usable.data) || "" : "";
    mark(opts.key, opts.label, {
      state: !fresh ? "refreshing" : warning ? "partial" : "ok",
      fetchedAt: usable.fetchedAt,
      error: warning,
    });
    if (!fresh) {
      void fetchAndStore(opts).then(next => {
        if (next != null) opts.onUpdate?.(next);
      });
    }
    return usable.data;
  }
  mark(opts.key, opts.label, {
    state: usable ? "refreshing" : "loading",
    fetchedAt: usable?.fetchedAt ?? 0,
  });
  const next = await fetchAndStore(opts);
  return next ?? usable?.data ?? null;
}

/** 本地日历日期 YYYY-MM-DD */
export function localDay(ts = Date.now()): string {
  const d = new Date(ts);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}
