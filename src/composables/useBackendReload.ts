import { onMounted, onUnmounted, type Ref } from "vue";
import { pushBountyAlerts } from "./useAppToasts";

/**
 * 订阅 Rust 后台 `backend://*` 事件 → bump token / 重载本地列表。
 * 网络拉取在后端；这里只做前端 reload，不轮询。
 */
export function useBackendReload(opts: {
  /** 日常/周常/电波等时效 brief 共用；live-world / nightwave 事件都 bump 它 */
  liveRefreshToken: Ref<number>;
  homeRefreshToken: Ref<number>;
  relicRefreshToken: Ref<number>;
  loadFamilies: () => void | Promise<void>;
  loadArchwingFamilies: () => void | Promise<void>;
  loadNecramechFamilies: () => void | Promise<void>;
  loadCompanionFamilies: () => void | Promise<void>;
  loadWeaponFamilies: () => void | Promise<void>;
  loadArchwingWeaponFamilies: () => void | Promise<void>;
  loadMods: () => void | Promise<void>;
  loadArcanes: () => void | Promise<void>;
  onBackendUpdateFinished: (msg: string) => void;
}) {
  const unsubs: Array<() => void> = [];

  onMounted(() => {
    void (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unsubs.push(
          await listen<string>("backend://live-world-updated", () => {
            opts.liveRefreshToken.value += 1;
          }),
        );
        unsubs.push(
          await listen<string>("backend://nightwave-updated", () => {
            opts.liveRefreshToken.value += 1;
          }),
        );
        unsubs.push(
          await listen<string>("backend://resurgence-updated", () => {
            opts.homeRefreshToken.value += 1;
            void opts.loadFamilies();
            void opts.loadWeaponFamilies();
          }),
        );
        unsubs.push(
          await listen<string>("backend://warframes-updated", () => {
            void opts.loadFamilies();
            void opts.loadArchwingFamilies();
            void opts.loadNecramechFamilies();
            void opts.loadCompanionFamilies();
          }),
        );
        unsubs.push(
          await listen<string>("backend://weapons-updated", () => {
            void opts.loadWeaponFamilies();
            void opts.loadArchwingWeaponFamilies();
          }),
        );
        unsubs.push(
          await listen<string>("backend://mods-updated", () => {
            void opts.loadMods();
          }),
        );
        unsubs.push(
          await listen<string>("backend://arcanes-updated", () => {
            void opts.loadArcanes();
          }),
        );
        unsubs.push(
          await listen<string>("backend://relics-updated", () => {
            opts.relicRefreshToken.value += 1;
          }),
        );
        unsubs.push(
          await listen<string>("backend://update-finished", (ev) => {
            opts.onBackendUpdateFinished(ev.payload ?? "");
          }),
        );
        unsubs.push(
          await listen<
            Array<{ body: string; expiry: string | null; tag?: string | null }>
          >("backend://bounty-alert", (ev) => {
            const list = ev.payload;
            if (Array.isArray(list) && list.length > 0) {
              opts.liveRefreshToken.value += 1;
              pushBountyAlerts(list);
            }
          }),
        );
      } catch {
        /* 无事件 API 时忽略（纯浏览器预览等） */
      }
    })();
  });

  onUnmounted(() => {
    for (const off of unsubs) off();
    unsubs.length = 0;
  });
}
