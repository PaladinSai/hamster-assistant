import { ref, type Ref } from "vue";
import {
  kickUpdateArcanes,
  kickUpdateFeed,
  kickUpdateLiveWorld,
  kickUpdateMods,
  kickUpdateRelics,
  kickUpdateWarframes,
  kickUpdateWeapons,
} from "../lib/api";
import type { View } from "./useAppNav";

export function useCatalogUpdates(opts: { error: Ref<string> }) {
  const updating = ref(false);
  const updateMsg = ref("");
  const relicRefreshToken = ref(0);
  const homeRefreshToken = ref(0);
  const liveRefreshToken = ref(0);
  let finishTimer: ReturnType<typeof setTimeout> | null = null;

  function markUpdating() {
    updating.value = true;
    updateMsg.value = "";
    opts.error.value = "";
    if (finishTimer) clearTimeout(finishTimer);
    // 防止事件丢失导致按钮一直转
    finishTimer = setTimeout(() => {
      updating.value = false;
    }, 90_000);
  }

  function finishUpdate(msg?: string) {
    if (finishTimer) {
      clearTimeout(finishTimer);
      finishTimer = null;
    }
    updating.value = false;
    if (msg) updateMsg.value = msg;
  }

  /** 供 AppShell 监听 backend://update-finished */
  function onBackendUpdateFinished(msg: string) {
    finishUpdate(msg);
  }

  async function kick(
    fn: () => Promise<void>,
  ) {
    if (updating.value) return;
    markUpdating();
    try {
      await fn();
    } catch (e) {
      finishUpdate();
      opts.error.value = String(e);
    }
  }

  async function runCatalogUpdate(
    kind:
      | "warframes"
      | "weapons"
      | "mods"
      | "arcanes"
      | "relics"
      | "resurgence"
      | "liveWorld"
      | "feed",
  ) {
    switch (kind) {
      case "liveWorld":
        return kick(kickUpdateLiveWorld);
      case "feed":
      case "resurgence":
        return kick(kickUpdateFeed);
      case "warframes":
        return kick(kickUpdateWarframes);
      case "weapons":
        return kick(kickUpdateWeapons);
      case "mods":
        return kick(kickUpdateMods);
      case "arcanes":
        return kick(kickUpdateArcanes);
      case "relics":
        return kick(kickUpdateRelics);
    }
  }

  function topbarUpdateData(view: View) {
    if (view === "home" || view === "standing") void runCatalogUpdate("liveWorld");
    else if (view === "feed") void runCatalogUpdate("feed");
    else if (
      view === "warframes" ||
      view === "archwing" ||
      view === "necramechs" ||
      view === "companions"
    )
      void runCatalogUpdate("warframes");
    else if (view === "weapons" || view === "archwingWeapons")
      void runCatalogUpdate("weapons");
    else if (view === "mods") void runCatalogUpdate("mods");
    else if (view === "arcanes") void runCatalogUpdate("arcanes");
    else if (view === "relics") void runCatalogUpdate("relics");
  }

  return {
    updating,
    updateMsg,
    relicRefreshToken,
    homeRefreshToken,
    liveRefreshToken,
    runCatalogUpdate,
    topbarUpdateData,
    onBackendUpdateFinished,
  };
}
