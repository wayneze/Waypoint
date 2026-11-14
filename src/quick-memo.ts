import { createApp } from "vue";
import "./style.css";
import QuickMemo from "./pages/QuickMemo.vue";
import { hydrateKv } from "./services/kv";

document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";
const app = document.getElementById("app");
if (app) app.style.background = "transparent";

// This is a separate webview, so its in-memory settings must be loaded too.
hydrateKv().then(() => createApp(QuickMemo).mount("#app")).catch(error => {
  console.error("Quick memo settings could not be loaded:", error);
  if (app) app.textContent = "无法读取本地设置，请重新打开记一笔。";
});
