import { nextTick, ref, watch, type Ref } from "vue";
import {
  upsertArcaneProgress,
  type ArcaneListItem,
} from "../lib/api";
import type {
  ArcaneKindFilter,
  ArcaneProgressFilter,
  ArcaneRarityFilter,
  ArcaneSourceFilter,
} from "../lib/status";
import type { View } from "./useAppNav";

export function useArcaneProgress(opts: {
  arcanes: Ref<ArcaneListItem[]>;
  error: Ref<string>;
  loadArcanes: () => Promise<void>;
  setView: (next: View) => void;
  view: Ref<View>;
  query: Ref<string>;
}) {
  const arcaneSourceFilter = ref<ArcaneSourceFilter>("all");
  const arcaneKindFilter = ref<ArcaneKindFilter>("all");
  const arcaneRarityFilter = ref<ArcaneRarityFilter>("all");
  const arcaneProgressFilter = ref<ArcaneProgressFilter>("all");
  const selectedArcaneId = ref<string | null>(null);
  /** 心愿跳转：仅展示这一项，直到用户改筛选/搜索或离开赋能页。 */
  const focusArcaneId = ref<string | null>(null);

  let suppressFocusClear = false;

  function clearArcaneFocus() {
    if (suppressFocusClear) return;
    focusArcaneId.value = null;
  }

  function patchLocalArcane(arcaneId: string, patch: Partial<ArcaneListItem>) {
    opts.arcanes.value = opts.arcanes.value.map((a) =>
      a.arcaneId === arcaneId ? { ...a, ...patch } : a,
    );
  }

  async function toggleArcaneWish(arcaneId: string, wish: number) {
    await upsertArcaneProgress({ arcaneId, wish });
    patchLocalArcane(arcaneId, { wish });
  }

  async function updateArcaneStatus(arcaneId: string, status: number) {
    await upsertArcaneProgress({ arcaneId, status });
    await opts.loadArcanes();
  }

  async function updateArcaneRank(arcaneId: string, rank: number) {
    await upsertArcaneProgress({ arcaneId, rank });
    await opts.loadArcanes();
  }

  async function updateArcaneNote(arcaneId: string, note: string) {
    await upsertArcaneProgress({ arcaneId, note });
    patchLocalArcane(arcaneId, { note });
  }

  async function openArcaneFromHome(arcaneId: string) {
    if (opts.arcanes.value.length === 0) {
      await opts.loadArcanes();
    }
    suppressFocusClear = true;
    try {
      arcaneSourceFilter.value = "all";
      arcaneKindFilter.value = "all";
      arcaneRarityFilter.value = "all";
      arcaneProgressFilter.value = "all";
      opts.query.value = "";
      selectedArcaneId.value = arcaneId;
      focusArcaneId.value = arcaneId;
      opts.setView("arcanes");
      await nextTick();
    } finally {
      suppressFocusClear = false;
    }
  }

  watch(opts.query, () => {
    if (focusArcaneId.value) clearArcaneFocus();
  });

  watch(
    [arcaneSourceFilter, arcaneKindFilter, arcaneRarityFilter, arcaneProgressFilter],
    () => {
      if (focusArcaneId.value) clearArcaneFocus();
    },
  );

  watch(opts.view, (next) => {
    if (next !== "arcanes") clearArcaneFocus();
  });

  const markingAll = ref(false);

  async function markAllOwned(items: { arcaneId: string; status: number }[]) {
    if (markingAll.value) return;
    markingAll.value = true;
    try {
      let did = false;
      for (const it of items) {
        if (it.status === 2) continue;
        await upsertArcaneProgress({ arcaneId: it.arcaneId, status: 2 });
        did = true;
      }
      if (did) {
        await opts.loadArcanes();
      }
    } catch (e) {
      opts.error.value = String(e);
    } finally {
      markingAll.value = false;
    }
  }

  return {
    arcaneSourceFilter,
    arcaneKindFilter,
    arcaneRarityFilter,
    arcaneProgressFilter,
    selectedArcaneId,
    focusArcaneId,
    clearArcaneFocus,
    markingAll,
    toggleArcaneWish,
    updateArcaneStatus,
    updateArcaneRank,
    updateArcaneNote,
    openArcaneFromHome,
    markAllOwned,
  };
}
