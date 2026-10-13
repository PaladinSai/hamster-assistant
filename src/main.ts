import { getCurrentWindow } from "@tauri-apps/api/window";
import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";

createApp(App).mount("#app");

/** 配置里窗口默认隐藏，挂载后再显示，避免闪白/黑屏；失败时不挡页面。 */
function revealWindow() {
  const win = getCurrentWindow();
  void win
    .show()
    .then(() => win.setFocus())
    .catch((e) => console.warn("[仓鼠] 显示主窗口失败", e));
}

requestAnimationFrame(() => {
  requestAnimationFrame(revealWindow);
});
