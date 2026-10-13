import { ref, type Ref } from "vue";
import {
  createModTag,
  deleteModTag,
  upsertModProgress,
  type ModListItem,
  type ModTagDto,
} from "../lib/api";
import type {
  ModKindFilter,
  ModProgressFilter,
  ModTagFilter,
} from "../lib/status";
import type { View } from "./useAppNav";

export function useModProgress(opts: {
  mods: Ref<ModListItem[]>;
  modTags: Ref<ModTagDto[]>;
  error: Ref<string>;
  loadMods: () => Promise<void>;
  setView: (next: View) => void;
}) {
  const modTagFilter = ref<ModTagFilter>("all");
  const modKindFilter = ref<ModKindFilter>("all");
  const modProgressFilter = ref<ModProgressFilter>("all");
  const selectedModId = ref<string | null>(null);

  function patchLocalMod(modId: string, patch: Partial<ModListItem>) {
    opts.mods.value = opts.mods.value.map((m) =>
      m.modId === modId ? { ...m, ...patch } : m,
    );
  }

  async function toggleModWish(modId: string, wish: number) {
    await upsertModProgress({ modId, wish });
    patchLocalMod(modId, { wish });
  }

  async function updateModStatus(modId: string, status: number) {
    await upsertModProgress({ modId, status });
    patchLocalMod(modId, { status });
  }

  async function updateModNote(modId: string, note: string) {
    await upsertModProgress({ modId, note });
    patchLocalMod(modId, { note });
  }

  async function updateModTag(modId: string, tagId: string | null) {
    await upsertModProgress({ modId, tagId });
    const tagName = tagId
      ? (opts.modTags.value.find((t) => t.id === tagId)?.name ?? null)
      : null;
    patchLocalMod(modId, { tagId, tagName });
  }

  async function onCreateModTag(name: string) {
    try {
      const tag = await createModTag(name);
      opts.modTags.value = [...opts.modTags.value, tag].sort(
        (a, b) => a.sortOrder - b.sortOrder || a.id.localeCompare(b.id),
      );
      modTagFilter.value = tag.id;
    } catch (e) {
      opts.error.value = String(e);
    }
  }

  async function onDeleteModTag(id: string) {
    try {
      await deleteModTag(id);
      opts.modTags.value = opts.modTags.value.filter((t) => t.id !== id);
      opts.mods.value = opts.mods.value.map((m) =>
        m.tagId === id ? { ...m, tagId: null, tagName: null } : m,
      );
      if (modTagFilter.value === id) {
        modTagFilter.value = "all";
      }
    } catch (e) {
      opts.error.value = String(e);
    }
  }

  async function openModFromHome(modId: string) {
    if (opts.mods.value.length === 0) {
      await opts.loadMods();
    }
    selectedModId.value = modId;
    opts.setView("mods");
  }

  const markingAll = ref(false);

  async function markAllOwned(items: { modId: string; status: number }[]) {
    if (markingAll.value) return;
    markingAll.value = true;
    try {
      for (const it of items) {
        if (it.status === 1) continue;
        await upsertModProgress({ modId: it.modId, status: 1 });
        patchLocalMod(it.modId, { status: 1 });
      }
    } catch (e) {
      opts.error.value = String(e);
    } finally {
      markingAll.value = false;
    }
  }

  return {
    modTagFilter,
    modKindFilter,
    modProgressFilter,
    selectedModId,
    markingAll,
    toggleModWish,
    updateModStatus,
    updateModNote,
    updateModTag,
    onCreateModTag,
    onDeleteModTag,
    openModFromHome,
    markAllOwned,
  };
}
