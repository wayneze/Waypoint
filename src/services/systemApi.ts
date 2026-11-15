import { Channel, invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { BackupProgress, RestoreProgress, ObsidianFile, DbInfo, ObsidianGameGuide, WaypointProjectionSnapshot, WaypointProjectionResult, WebdavConfig, WebdavStatus, WebdavSyncResult } from "../types";
import { isTauri } from "./api";

// 触发原生通知
export async function sendNotification(title: string, body: string) {
  if (isTauri()) {
    try {
      await invoke("trigger_notification", { title, body });
      return;
    } catch (e) {
      console.warn("Tauri notification error:", e);
    }
  }
  console.log(`[Notification Fallback] ${title}: ${body}`);
}

// 窗口置顶控制
export async function toggleAlwaysOnTop(enabled: boolean): Promise<boolean> {
  if (isTauri()) {
    try {
      return await invoke<boolean>("toggle_always_on_top", { enabled });
    } catch (e) {
      console.warn("Tauri toggle_always_on_top error:", e);
    }
  }
  return enabled;
}

// 检查窗口置顶状态
export async function isAlwaysOnTop(): Promise<boolean> {
  if (isTauri()) {
    try {
      return await invoke<boolean>("is_always_on_top");
    } catch (e) {
      console.warn("Tauri is_always_on_top error:", e);
    }
  }
  return false;
}

// 在资源管理器中打开指定路径
export async function openInExplorer(path: string): Promise<void> {
  if (isTauri()) {
    try {
      await invoke("open_in_explorer", { path });
      return;
    } catch (e) {
      console.warn("Tauri open_in_explorer error:", e);
      throw e;
    }
  }
  console.log("Mock open_in_explorer:", path);
}

// 打开 Windows 原生系统文件夹选择器
export async function pickFolder(description?: string): Promise<string | null> {
  if (isTauri()) {
    try {
      return await invoke<string | null>("pick_folder", { description: description ?? null });
    } catch (e) {
      console.warn("Tauri pick_folder error:", e);
      throw e;
    }
  }
  return null;
}

// 扫描 Obsidian 真实仓库下的 Markdown 笔记
export async function scanObsidianVault(vaultPath: string): Promise<ObsidianFile[]> {
  if (isTauri()) {
    try {
      return await invoke<ObsidianFile[]>("scan_obsidian_vault", { vaultPath });
    } catch (e) {
      console.warn("Tauri scan_obsidian_vault error:", e);
      throw e;
    }
  }
  return [];
}

// 读取指定 Markdown 笔记内容
export async function readObsidianFile(filePath: string): Promise<string> {
  if (isTauri()) {
    try {
      return await invoke<string>("read_obsidian_file", { filePath });
    } catch (e) {
      console.warn("Tauri read_obsidian_file error:", e);
      throw e;
    }
  }

  throw new Error("仅在桌面端可用");
}

// 智能扫描并结构化解析 Obsidian 游戏与 Galgame 攻略库
export async function scanObsidianGameGuides(customVaultPath?: string): Promise<ObsidianGameGuide[]> {
  if (isTauri()) {
    try {
      return await invoke<ObsidianGameGuide[]>("scan_obsidian_game_guides", { customVaultPath });
    } catch (e) {
      console.warn("Tauri scan_obsidian_game_guides error:", e);
      throw e;
    }
  }

  return [];
}

export interface ExploreHit {
  title: string;
  url: string;
  detail: string;
}

export interface ResourceArticle { title: string; url: string; markdown: string }
export async function readResourceArticle(url: string): Promise<ResourceArticle> {
  if (!isTauri()) throw new Error('正文读取需要桌面客户端。');
  return invoke('read_resource_article', { url });
}
export async function writeResourceMarkdown(destPath: string, article: ResourceArticle, gameTitle: string): Promise<string> {
  if (!isTauri()) throw new Error('保存 Markdown 文件需要桌面客户端。');
  return invoke('write_resource_markdown', { destPath, article, gameTitle });
}
export async function openResourceWeb(url: string, title = '游戏资料', gameId = '', sourceUrl = url): Promise<void> {
  const parsed = new URL(url);
  if (!['http:', 'https:'].includes(parsed.protocol) || parsed.username || parsed.password) throw new Error('只支持不含登录凭据的网页链接');
  if (!isTauri()) throw new Error('应用内网页需要桌面客户端。');
  return invoke('open_resource_web', { url, context: { title, game_id: gameId, source_url: sourceUrl } });
}
export async function currentResourceWeb(): Promise<string> {
  return invoke('resource_web_current');
}

export async function searchExploreSite(
  kind: NonNullable<import('./exploreSites').ExploreSite['result']>,
  query: string,
  home: string,
): Promise<ExploreHit[]> {
  if (!isTauri()) return [];
  return invoke<ExploreHit[]>("search_explore_site", { kind, query, home });
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!/^https?:\/\//i.test(url)) throw new Error("只打开 http 链接");
  if (isTauri()) {
    await openUrl(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
}

// 用系统关联程序 (Obsidian) 打开攻略笔记文件
export async function openInObsidian(filePath: string): Promise<void> {
  if (isTauri()) {
    try {
      await invoke("open_in_obsidian", { filePath });
      return;
    } catch (e) {
      console.warn("Tauri open_in_obsidian error:", e);
      throw e;
    }
  }
}

// 同步更新 Obsidian 攻略笔记的 Frontmatter 元数据 (如状态 status, 已通关路线 routes.cleared) 以及正文表格路线完成度
export async function updateGuideFrontmatter(
  filePath: string,
  status?: string,
  routesCleared?: number,
  routeTitle?: string,
  routeCompleted?: boolean
): Promise<void> {
  if (isTauri()) {
    try {
      await invoke("update_guide_frontmatter", {
        filePath,
        status,
        routesCleared,
        routeTitle,
        routeCompleted
      });
    } catch (e) {
      console.warn("Tauri update_guide_frontmatter error:", e);
    }
  }
}

export async function exportWaypointProjection(snapshot: WaypointProjectionSnapshot): Promise<WaypointProjectionResult> {
  if (isTauri()) {
    try {
      return await invoke<WaypointProjectionResult>("export_waypoint_projection", { snapshot });
    } catch (e) {
      console.warn("Tauri export_waypoint_projection error:", e);
      throw e;
    }
  }
  throw new Error("仅在桌面端可用");
}

// 获取 SQLite 数据库物理存储信息
export async function getDbInfo(): Promise<DbInfo | null> {
  if (isTauri()) {
    try {
      return await invoke<DbInfo>("get_db_info");
    } catch (e) {
      console.warn("Tauri get_db_info error:", e);
    }
  }
  return null;
}

export interface HotkeyStatus {
  panel: string;
  capture: string;
  panel_error?: string | null;
  capture_error?: string | null;
}

export async function applyHotkeys(): Promise<HotkeyStatus | null> {
  if (!isTauri()) return null;
  return invoke<HotkeyStatus>("apply_hotkeys_command");
}

export async function getHotkeyStatus(): Promise<HotkeyStatus | null> {
  if (!isTauri()) return null;
  return invoke<HotkeyStatus>("hotkey_status");
}

export async function pickSaveFile(title: string, filter: string, defaultName: string): Promise<string | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<string | null>("pick_save_file", { title, filter, defaultName });
  } catch (e) {
    console.warn("Tauri pick_save_file error:", e);
    throw e;
  }
}

export async function pickOpenFile(title: string, filter: string): Promise<string | null> {
  if (!isTauri()) return null;
  try {
    return await invoke<string | null>("pick_open_file", { title, filter });
  } catch (e) {
    console.warn("Tauri pick_open_file error:", e);
    return null;
  }
}

export async function exportDatabase(destPath: string, password: string): Promise<string> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  return await invoke<string>("export_database", { destPath, password });
}

function restoreChannel(onProgress?: (progress: RestoreProgress) => void): Channel<RestoreProgress> {
  const channel = new Channel<RestoreProgress>();
  channel.onmessage = progress => onProgress?.(progress);
  return channel;
}

export async function importDatabase(srcPath: string, password = '', onProgress?: (progress: RestoreProgress) => void): Promise<void> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  await invoke("import_database", { srcPath, password, onProgress: restoreChannel(onProgress) });
}

export interface ActivityStats {
  since: string;
  /** [日期, scope, 条数] */
  checks: [string, string, number][];
  progress: [string, string, number][];
  daily_tasks: number;
}

export async function getActivityStats(days: number): Promise<ActivityStats | null> {
  if (!isTauri()) return null;
  return await invoke<ActivityStats>("get_activity_stats", { days });
}

export async function exportVaultPlain(destPath: string): Promise<number> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  return await invoke<number>("export_vault_plain", { destPath });
}

export async function importVaultPlain(srcPath: string): Promise<number> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  return await invoke<number>("import_vault_plain", { srcPath });
}

export async function saveSteamWebApiKey(key: string): Promise<void> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  await invoke("save_steam_web_api_key", { key });
}

export async function steamWebApiKeyIsConfigured(): Promise<boolean> {
  if (!isTauri()) return false;
  return await invoke<boolean>("steam_web_api_key_configured");
}

export async function undoLastRestore(onProgress?: (progress: RestoreProgress) => void): Promise<void> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  await invoke("undo_last_restore", { onProgress: restoreChannel(onProgress) });
}

export async function saveWebdavConfig(config: WebdavConfig): Promise<void> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  await invoke("save_webdav_config", { config });
}

export async function testWebdavConnection(
  url: string,
  username: string,
  password?: string
): Promise<{ ok: boolean; message: string }> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  return await invoke<{ ok: boolean; message: string }>("test_webdav_connection", {
    url,
    username,
    password: password || null,
  });
}

export async function getWebdavStatus(): Promise<WebdavStatus> {
  if (!isTauri()) {
    return {
      configured: false,
      url: "",
      username: "",
      has_password: false,
      remote_exists: false,
    };
  }
  return await invoke<WebdavStatus>("get_webdav_status");
}

export async function uploadWebdavBackup(password: string, force = false, onProgress?: (progress: BackupProgress) => void): Promise<WebdavSyncResult> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  const channel = new Channel<BackupProgress>();
  channel.onmessage = progress => onProgress?.(progress);
  return await invoke<WebdavSyncResult>("upload_webdav_backup", { password, force, onProgress: channel });
}

export async function cancelWebdavBackup(): Promise<void> {
  await invoke("cancel_webdav_backup");
}

export async function downloadWebdavBackup(password = '', onProgress?: (progress: RestoreProgress) => void): Promise<WebdavSyncResult> {
  if (!isTauri()) throw new Error("仅在桌面端可用");
  return await invoke<WebdavSyncResult>("download_webdav_backup", { password, onProgress: restoreChannel(onProgress) });
}
