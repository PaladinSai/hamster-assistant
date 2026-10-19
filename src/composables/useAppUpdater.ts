import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { ref } from "vue";
import { dismissToast, pushToast } from "./useAppToasts";

const checking = ref(false);
const installing = ref(false);
const currentVersion = ref("");
const dismissedVersion = ref<string | null>(null);
let pendingUpdate: Update | null = null;
let autoChecked = false;
let updateToastId: string | null = null;

async function ensureCurrentVersion() {
  if (currentVersion.value) return currentVersion.value;
  try {
    currentVersion.value = await getVersion();
  } catch {
    currentVersion.value = "";
  }
  return currentVersion.value;
}

async function installPending() {
  if (!pendingUpdate || installing.value) return;
  installing.value = true;
  if (updateToastId) dismissToast(updateToastId);
  updateToastId = pushToast({
    title: "正在更新",
    body: `下载并安装 v${pendingUpdate.version}…`,
  });
  try {
    await pendingUpdate.downloadAndInstall();
    await relaunch();
  } catch (e) {
    if (updateToastId) dismissToast(updateToastId);
    pushToast({
      title: "更新失败",
      body: String(e),
    });
  } finally {
    installing.value = false;
  }
}

function offerUpdate(update: Update, manual: boolean) {
  pendingUpdate = update;
  if (!manual && dismissedVersion.value === update.version) return;
  if (updateToastId) dismissToast(updateToastId);
  updateToastId = pushToast({
    title: "发现新版本",
    body: `当前 ${currentVersion.value || "—"} → v${update.version}`,
    actions: [
      {
        label: "立即更新",
        kind: "primary",
        run: () => {
          void installPending();
        },
      },
      {
        label: "稍后",
        kind: "ghost",
        run: () => {
          dismissedVersion.value = update.version;
        },
      },
    ],
  });
}

/**
 * 检查 GitHub Releases（latest.json）。
 * @param manual 手动检查：无更新也提示；自动检查失败静默。
 */
export async function checkAppUpdate(manual = false) {
  if (checking.value || installing.value) return;
  checking.value = true;
  try {
    await ensureCurrentVersion();
    const update = await check();
    if (!update) {
      if (manual) {
        pushToast({
          title: "已是最新",
          body: currentVersion.value
            ? `当前版本 v${currentVersion.value}`
            : "没有可用更新",
        });
      }
      return;
    }
    offerUpdate(update, manual);
  } catch (e) {
    if (manual) {
      pushToast({
        title: "检查更新失败",
        body: String(e),
      });
    }
  } finally {
    checking.value = false;
  }
}

/** 启动后空闲检查一次（失败静默）。 */
export function scheduleIdleUpdateCheck() {
  if (autoChecked) return;
  autoChecked = true;
  const run = () => {
    void checkAppUpdate(false);
  };
  const ric = (
    window as Window & {
      requestIdleCallback?: (
        cb: () => void,
        opts?: { timeout: number },
      ) => number;
    }
  ).requestIdleCallback;
  if (typeof ric === "function") {
    ric(run, { timeout: 12_000 });
  } else {
    window.setTimeout(run, 8_000);
  }
}

export function useAppUpdater() {
  return {
    checking,
    installing,
    currentVersion,
    checkAppUpdate,
    scheduleIdleUpdateCheck,
    ensureCurrentVersion,
  };
}
