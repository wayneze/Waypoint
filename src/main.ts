import { createApp } from "vue";
import "./style.css";
import { hydrateKv } from "./services/kv";
import { hydrateProgress } from "./services/progress";
import { hydrateCache } from "./services/fetchCache";
import { describeError } from "./services/errors";

Promise.all([hydrateKv(), hydrateProgress(), hydrateCache()])
  .then(async () => {
    const [{ default: App }, { showToast }] = await Promise.all([
      import("./App.vue"),
      import("./workspace/core"),
    ]);
    const app = createApp(App);
    // 桌面端读写失败直接抛出，这里统一提示，避免静默丢写入。
    app.config.errorHandler = (err) => {
      console.error(err);
      showToast("操作失败", describeError(err).message, { persistent: true });
    };
    // 未处理的异步错误可能来自加载、刷新或保存，不统一误报成保存失败。
    window.addEventListener("unhandledrejection", (ev) => {
      console.error(ev.reason);
      showToast("后台操作失败", describeError(ev.reason).message, { persistent: true });
    });
    app.mount("#app");
  })
  .catch((err) => {
    // 读不到设置库时不挂载界面，避免在空状态上写入覆盖真实数据。
    console.error("hydrateKv failed:", err);
    document.body.textContent = `无法读取本地数据库：${err instanceof Error ? err.message : String(err)}`;
  });
