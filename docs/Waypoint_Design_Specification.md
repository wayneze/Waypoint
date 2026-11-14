# Waypoint 架构与数据边界

运行、构建、备份迁移和清理方式见 [README](../README.md)。本文描述模块职责；具体表结构、命令签名和目录内容以源码为准。

## 调用链

`pages / components → workspace → services → Tauri IPC → Rust 领域模块 → SQLite / 文件系统 / 外部服务`

- 页面直接导入所属 `workspace` 模块，不经统一总出口。
- `src/main.ts` 先读取设置、进度和缓存，再挂载界面；`workspace/bootstrap.ts` 组织本机数据初始化和后台刷新。
- 光遇模块、剑三完整成就目录及较重的弹窗按需加载。WebDAV 状态、外部目录扫描与 Steam 活动刷新独立于本机初始化。
- `src-tauri/src/lib.rs` 注册命令与应用状态，在数据库准备好后创建窗口。

## 数据归属

| 数据 | 权威位置 | 入口 |
| --- | --- | --- |
| 账号、角色、资产 | `vault_items`；角色位于 `payload.characters` | `workspace/vault.ts`、`vault.rs` |
| 角色任务 | `tasks` | `workspace/core.ts`、`workspace/daily.ts`、`tasks.rs` |
| 周期记事 | `custom_memos` | `workspace/memos.ts`、`memos.rs` |
| 游戏档案与 Steam 库 | `game_records`、`steam_library` | `workspace/guides`、`workspace/vault.ts`、`records.rs`、`steam.rs` |
| 收藏、图鉴与打卡 | `progress_items`、`daily_checks` | `services/progress.ts`、`progress.rs` |
| 设置与单块记录 | `app_settings` | `services/kv.ts`、`settings.rs` |
| 网络缓存 | 独立 `cache.db` 中的 `fetch_cache` | `services/fetchCache.ts`、`cache.rs` |

角色只从金库展开，不从任务或演示数据补造。角色进度使用稳定 ID 隔离；旧共享记录由用户明确选择归属，迁移与删除旧记录在同一事务中完成。空库保持为空。

桌面端 SQLite 是唯一持久化来源，前端内存只作缓存；localStorage 仅用于独立浏览器预览。同一设置键的写入按调用顺序落库，备份前等待待写入操作。网络目录、缓存和用户进度分别保存，刷新目录不推断用户完成状态。

表版本定义和升级在 `lib.rs`，进度迁移在 `progress.rs`，凭据迁移在 `secrets.rs`。升级与恢复前保留数据库快照；恢复失败时保留或回滚当前数据。文档不复制建表 SQL。

## 领域模块

| 功能 | 前端状态 | Rust 实现 |
| --- | --- | --- |
| 剑三日历与收集 | `workspace/jx3/calendar.ts`、`collect.ts` | `jx3.rs` |
| 剑三声望、奇遇、成就 | `workspace/jx3/progress.ts` | `jx3.rs`、`mingyi.rs` |
| 剑三外观 | `workspace/jx3/exterior.ts` | `jx3_exterior.rs` |
| 光遇任务、档期、收藏与探索 | `workspace/sky` | `sky` |
| 攻略与资料 | `workspace/guides` | `guides.rs`、`explore.rs`、`resources.rs`、`juij.rs` |
| 作品发现与元数据 | `workspace/discovery.ts`、`services/gameLibrary.ts` | `discovery.rs`、`game_library.rs`、`game_trailers.rs` |
| 本地启动与时长 | `workspace/gameActivity.ts` | `game_operations.rs`、`playtime.rs` |
| 存档与版本 | `workspace/gameSaves.ts` | `game_saves.rs`、`save_backups.rs`、`game_versions.rs` |
| 本地作品移除 | 攻略与书库工作流 | `library_removal.rs` |
| 画面缩放 | `services/gameScaling.ts` | `scaling.rs`、`native/scaling` |
| Obsidian 投影 | `workspace/bootstrap.ts` | `projection.rs` |
| 设置与完整备份 | `workspace/settings.ts` | `db.rs`、`backup.rs`、`webdav.rs`、`dialogs.rs` |

剑三收藏基础目录在 `services/jx3CollectData.ts`；完整成就目录由进度模块加载。门派图标经 `services/jx3School.ts` 和 `jx3Artwork.ts` 解析，本地静态路径不经过网络图片代理。

光遇 wiki 与 Fandom 的协议和解析分别位于 `sky/wiki.rs` 和 `sky/fandom`。公共资源可缓存，个人拥有、打卡与探索状态独立保存；数据缺失或匹配不明确时不补成成功记录。

## 安全与备份

- 金库 `notes` 与 `payload` 使用 Windows DPAPI，加密错误保留锁定状态，不以空数据覆盖。
- 服务密码与令牌使用 Windows 凭据管理器，设置表仅记录配置标记。
- `backup.rs` 使用 SQLite 在线快照，打包数据库及图片缓存，以密码派生密钥进行 AES-256-GCM 认证加密。密码不持久化，服务凭据不进入完整备份。
- 恢复前验证格式、密码、数据库及金库可读性，随后在目标机器重新加密金库。图片恢复只接受规范的缓存文件名，不解包任意路径。
- 外部攻略、游戏程序与存档不进入完整备份；游戏存档版本由独立存档备份功能管理。

## 桌面生命周期

`cycle.rs` 负责游戏换日、提醒和托盘计数。游戏与复位时间的定义在 `src/content/games.json`，用户可在设置覆盖。前端接收换日事件后重载相关状态。

`hotkeys.rs` 负责快捷键，`lib.rs` 负责窗口、托盘和窗口状态持久化。关闭窗口隐藏到托盘，托盘退出结束应用。独立「记一笔」webview 挂载前读取自己的设置缓存。

缩放核心通过独立宿主进程运行，源码与许可证在 `native/scaling`。构建产物位于 `target/scaling`，运行时从嵌入载荷释放；源码分发需保留原生组件源码和对应版权声明。
