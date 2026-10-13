import { ref, watch, type Ref } from "vue";
import {
  getWarframeDetail,
  upsertProgressPart,
  upsertProgressWarframe,
  type WarframeDetail,
  type WarframeListItem,
} from "../lib/api";
import type { FilterKey, WarframeStatusFilter } from "../lib/status";
import { nextWish } from "../lib/wish";
import type { CompanionType } from "../lib/api/warframes";
import type { View } from "./useAppNav";

export type CompanionTypeFilter = "all" | CompanionType;

export function useWarframeProgress(opts: {
  families: Ref<WarframeListItem[]>;
  error: Ref<string>;
  view: Ref<View>;
}) {
  const filter = ref<FilterKey>("all");
  const statusFilter = ref<WarframeStatusFilter>("all");
  const companionTypeFilter = ref<CompanionTypeFilter>("all");
  const selectedId = ref<string | null>(null);
  const detail = ref<WarframeDetail | null>(null);
  const detailLoading = ref(false);

  function closeDrawer() {
    selectedId.value = null;
    detail.value = null;
  }

  function patchLocal(warframeId: string, patch: Partial<WarframeListItem>) {
    const i = opts.families.value.findIndex((f) => f.warframeId === warframeId);
    if (i >= 0) {
      opts.families.value[i] = { ...opts.families.value[i], ...patch };
    }
    if (detail.value?.warframe.warframeId === warframeId) {
      detail.value = {
        ...detail.value,
        warframe: { ...detail.value.warframe, ...patch },
      };
    }
  }

  async function openWarframe(warframeId: string) {
    selectedId.value = warframeId;
    detailLoading.value = true;
    try {
      detail.value = await getWarframeDetail(warframeId);
      const kind = detail.value?.warframe.suitKind;
      if (kind === "archwing") opts.view.value = "archwing";
      else if (kind === "necramech") opts.view.value = "necramechs";
      else if (kind === "companion" || kind === "sentinel")
        opts.view.value = "companions";
      else if (
        opts.view.value !== "archwing" &&
        opts.view.value !== "necramechs" &&
        opts.view.value !== "companions" &&
        opts.view.value !== "warframes"
      ) {
        opts.view.value = "warframes";
      }
    } catch (e) {
      opts.error.value = String(e);
      detail.value = null;
    } finally {
      detailLoading.value = false;
    }
  }

  async function toggleHelminth(f: WarframeListItem) {
    if (f.variant !== 0) return;
    const next = f.helminth ? 0 : 1;
    patchLocal(f.warframeId, { helminth: next });
    await upsertProgressWarframe({ warframeId: f.warframeId, helminth: next });
  }

  async function toggleForma(f: WarframeListItem) {
    const next = f.formaDone ? 0 : 1;
    patchLocal(f.warframeId, { formaDone: next });
    await upsertProgressWarframe({ warframeId: f.warframeId, formaDone: next });
  }

  async function toggleMastered(f: WarframeListItem) {
    const next = f.mastered ? 0 : 1;
    patchLocal(f.warframeId, { mastered: next });
    await upsertProgressWarframe({ warframeId: f.warframeId, mastered: next });
  }

  async function updateStatus(warframeId: string, status: number) {
    await upsertProgressWarframe({ warframeId, status });
    const display =
      status === 1
        ? "蓝图已齐"
        : status === 2
          ? "铸造中"
          : status === 3
            ? "已获取"
            : "未获取";
    patchLocal(warframeId, { status, statusDisplay: display });
    if (detail.value?.warframe.warframeId === warframeId) {
      detail.value = {
        ...detail.value,
        warframe: { ...detail.value.warframe, status, statusDisplay: display },
        parts: detail.value.parts.map((p) => ({ ...p, status })),
      };
    }
  }

  async function updateStatusFromDrawer(status: number) {
    if (!detail.value) return;
    await updateStatus(detail.value.warframe.warframeId, status);
  }

  async function updatePartStatus(partId: string, status: number) {
    if (!detail.value) return;
    const warframeId = detail.value.warframe.warframeId;
    await upsertProgressPart({ partId, status });
    const next = await getWarframeDetail(warframeId);
    detail.value = next;
    patchLocal(warframeId, {
      status: next.warframe.status,
      statusDisplay: next.warframe.statusDisplay,
    });
  }

  async function updatePartNote(partId: string, note: string) {
    if (!detail.value) return;
    detail.value = {
      ...detail.value,
      parts: detail.value.parts.map((p) =>
        p.partId === partId ? { ...p, note } : p,
      ),
    };
    await upsertProgressPart({ partId, note });
  }

  async function updateNote(note: string) {
    if (!detail.value) return;
    const id = detail.value.warframe.warframeId;
    patchLocal(id, { note });
    await upsertProgressWarframe({ warframeId: id, note });
  }

  async function toggleWish(f: WarframeListItem) {
    const wish = nextWish(f);
    patchLocal(f.warframeId, { wish });
    await upsertProgressWarframe({ warframeId: f.warframeId, wish });
  }

  async function toggleWishFromDrawer() {
    if (!detail.value) return;
    await toggleWish(detail.value.warframe);
  }

  async function toggleHelminthFromDrawer() {
    if (!detail.value) return;
    await toggleHelminth(detail.value.warframe);
  }

  async function toggleFormaFromDrawer() {
    if (!detail.value) return;
    await toggleForma(detail.value.warframe);
  }

  async function toggleMasteredFromDrawer() {
    if (!detail.value) return;
    await toggleMastered(detail.value.warframe);
  }

  const markingAll = ref(false);

  async function markAllOwned(items: { warframeId: string; status: number }[]) {
    if (markingAll.value) return;
    markingAll.value = true;
    try {
      for (const f of items) {
        if (f.status !== 3) {
          await updateStatus(f.warframeId, 3);
        }
      }
    } catch (e) {
      opts.error.value = String(e);
    } finally {
      markingAll.value = false;
    }
  }

  watch(selectedId, (id) => {
    if (id == null) detail.value = null;
  });

  return {
    filter,
    statusFilter,
    companionTypeFilter,
    selectedId,
    detail,
    detailLoading,
    markingAll,
    closeDrawer,
    openWarframe,
    toggleHelminth,
    toggleForma,
    toggleMastered,
    updateStatus,
    updateStatusFromDrawer,
    updatePartStatus,
    updatePartNote,
    updateNote,
    toggleWish,
    toggleWishFromDrawer,
    toggleHelminthFromDrawer,
    toggleFormaFromDrawer,
    toggleMasteredFromDrawer,
    markAllOwned,
  };
}
