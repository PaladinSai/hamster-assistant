import { ref, watch, type Ref } from "vue";
import {
  getWeaponFamilyDetail,
  upsertWeaponPartProgress,
  upsertWeaponProgress,
  type WeaponFamilyDetail,
  type WeaponFamilyListItem,
} from "../lib/api";
import type {
  WeaponIncarnonFilter,
  WeaponKindFilter,
  WeaponProgressFilter,
  WeaponVaultFilter,
} from "../lib/status";
import type { View } from "./useAppNav";

export function useWeaponProgress(opts: {
  weaponFamilies: Ref<WeaponFamilyListItem[]>;
  archwingWeaponFamilies: Ref<WeaponFamilyListItem[]>;
  error: Ref<string>;
  loadWeaponFamilies: () => Promise<void>;
  loadArchwingWeaponFamilies: () => Promise<void>;
  view: Ref<View>;
}) {
  const weaponSlot = ref(0);
  const archwingWeaponSlot = ref(-1);
  const weaponKindFilter = ref<WeaponKindFilter>("all");
  const weaponProgressFilter = ref<WeaponProgressFilter>("all");
  const weaponVaultFilter = ref<WeaponVaultFilter>("all");
  const weaponIncarnonFilter = ref<WeaponIncarnonFilter>("all");
  const selectedWeaponFamilyId = ref<string | null>(null);
  const selectedWeaponId = ref<string | null>(null);
  const weaponDetail = ref<WeaponFamilyDetail | null>(null);
  const weaponDetailLoading = ref(false);

  function familyLists(): Ref<WeaponFamilyListItem[]>[] {
    return [opts.weaponFamilies, opts.archwingWeaponFamilies];
  }

  function closeWeaponDrawer() {
    selectedWeaponFamilyId.value = null;
    selectedWeaponId.value = null;
    weaponDetail.value = null;
  }

  function patchLocalWeaponMember(
    familyId: string,
    weaponId: string,
    patch: Partial<WeaponFamilyListItem["members"][number]>,
  ) {
    for (const list of familyLists()) {
      const fi = list.value.findIndex((f) => f.familyId === familyId);
      if (fi < 0) continue;
      const fam = list.value[fi];
      const members = fam.members.map((m) =>
        m.weaponId === weaponId ? { ...m, ...patch } : m,
      );
      list.value[fi] = { ...fam, members };
      break;
    }
    if (weaponDetail.value?.family.familyId === familyId) {
      const weapons = weaponDetail.value.weapons.map((w) =>
        w.weaponId === weaponId ? { ...w, ...patch } : w,
      );
      const members = weaponDetail.value.family.members.map((m) =>
        m.weaponId === weaponId ? { ...m, ...patch } : m,
      );
      weaponDetail.value = {
        ...weaponDetail.value,
        family: { ...weaponDetail.value.family, members },
        weapons,
      };
    }
  }

  function replaceLocalFamily(family: WeaponFamilyListItem) {
    for (const list of familyLists()) {
      const fi = list.value.findIndex((f) => f.familyId === family.familyId);
      if (fi >= 0) {
        list.value[fi] = family;
        return;
      }
    }
  }

  function familyIdForWeapon(weaponId: string): string | null {
    if (
      selectedWeaponFamilyId.value &&
      weaponDetail.value?.weapons.some((w) => w.weaponId === weaponId)
    ) {
      return selectedWeaponFamilyId.value;
    }
    for (const list of familyLists()) {
      const fam = list.value.find((f) =>
        f.members.some((m) => m.weaponId === weaponId),
      );
      if (fam) return fam.familyId;
    }
    return null;
  }

  function isArchwingFamily(familyId: string): boolean {
    return opts.archwingWeaponFamilies.value.some((f) => f.familyId === familyId);
  }

  async function openWeapon(familyId: string, weaponId: string) {
    selectedWeaponFamilyId.value = familyId;
    selectedWeaponId.value = weaponId;
    opts.view.value = isArchwingFamily(familyId) ? "archwingWeapons" : "weapons";
    weaponDetailLoading.value = true;
    try {
      weaponDetail.value = await getWeaponFamilyDetail(familyId);
      if (weaponDetail.value.family.weaponKind === "archwing") {
        opts.view.value = "archwingWeapons";
      }
    } catch (e) {
      opts.error.value = String(e);
      weaponDetail.value = null;
    } finally {
      weaponDetailLoading.value = false;
    }
  }

  async function openWeaponFromHome(familyId: string, weaponId: string) {
    if (opts.weaponFamilies.value.length === 0) {
      await opts.loadWeaponFamilies();
    }
    if (
      !opts.weaponFamilies.value.some((f) => f.familyId === familyId) &&
      opts.archwingWeaponFamilies.value.length === 0
    ) {
      await opts.loadArchwingWeaponFamilies();
    }
    await openWeapon(familyId, weaponId);
  }

  async function updateWeaponStatus(weaponId: string, status: number) {
    const familyId = familyIdForWeapon(weaponId);
    if (!familyId) return;

    let canIncarnon = 0;
    let prevIncarnon = 0;
    let variant = 0;
    for (const list of familyLists()) {
      for (const f of list.value) {
        const m = f.members.find((x) => x.weaponId === weaponId);
        if (m) {
          canIncarnon = m.canIncarnon;
          prevIncarnon = m.incarnon;
          variant = m.variant;
          break;
        }
      }
    }

    await upsertWeaponProgress({ weaponId, status });

    const patch: Partial<WeaponFamilyListItem["members"][number]> = { status };
    // 与后端一致：自带灵化在「已获取」时自动进「已有灵化之源」（创世改装边缘情况打开抽屉再对齐）
    if (canIncarnon && status === 1 && prevIncarnon === 0) {
      patch.incarnon = 2;
    }
    patchLocalWeaponMember(familyId, weaponId, patch);

    if (weaponDetail.value?.family.familyId === familyId) {
      const special = variant === 2 || variant === 3 || variant === 4;
      const partStatus = status === 1 || status === 2 ? 3 : 0;
      weaponDetail.value = {
        ...weaponDetail.value,
        weapons: weaponDetail.value.weapons.map((w) =>
          w.weaponId !== weaponId
            ? w
            : {
                ...w,
                status: patch.status ?? w.status,
                incarnon: patch.incarnon ?? w.incarnon,
                parts: special
                  ? w.parts
                  : w.parts.map((p) => ({ ...p, status: partStatus })),
              },
        ),
      };
    }
  }

  async function updateWeaponIncarnon(weaponId: string, incarnon: number) {
    const familyId = familyIdForWeapon(weaponId);
    if (!familyId) return;
    await upsertWeaponProgress({ weaponId, incarnon });
    patchLocalWeaponMember(familyId, weaponId, { incarnon });
  }

  async function updateWeaponNote(weaponId: string, note: string) {
    const familyId = familyIdForWeapon(weaponId);
    if (!familyId) return;
    await upsertWeaponProgress({ weaponId, note });
    patchLocalWeaponMember(familyId, weaponId, { note });
  }

  async function updateWeaponPartStatus(partId: string, status: number) {
    const familyId = selectedWeaponFamilyId.value;
    if (!familyId) return;
    await upsertWeaponPartProgress({ partId, status });
    const next = await getWeaponFamilyDetail(familyId);
    weaponDetail.value = next;
    replaceLocalFamily(next.family);
  }

  async function updateWeaponPartHaveQty(partId: string, haveQty: number) {
    const familyId = selectedWeaponFamilyId.value;
    if (!familyId) return;
    await upsertWeaponPartProgress({ partId, haveQty });
    const next = await getWeaponFamilyDetail(familyId);
    weaponDetail.value = next;
    replaceLocalFamily(next.family);
  }

  async function toggleWeaponWish(weaponId: string, wish: number) {
    const familyId =
      selectedWeaponFamilyId.value ?? familyIdForWeapon(weaponId);
    if (!familyId) return;
    await upsertWeaponProgress({ weaponId, wish });
    patchLocalWeaponMember(familyId, weaponId, { wish });
  }

  async function toggleWeaponMastered(weaponId: string, mastered: number) {
    const familyId = familyIdForWeapon(weaponId);
    if (!familyId) return;
    await upsertWeaponProgress({ weaponId, mastered });
    patchLocalWeaponMember(familyId, weaponId, { mastered });
  }

  async function updateWeaponPartNote(partId: string, note: string) {
    if (!weaponDetail.value) return;
    await upsertWeaponPartProgress({ partId, note });
    weaponDetail.value = {
      ...weaponDetail.value,
      weapons: weaponDetail.value.weapons.map((w) => ({
        ...w,
        parts: w.parts.map((p) =>
          p.partId === partId ? { ...p, note } : p,
        ),
      })),
    };
  }

  watch(selectedWeaponFamilyId, (id) => {
    if (id == null) {
      weaponDetail.value = null;
      selectedWeaponId.value = null;
    }
  });

  const markingAll = ref(false);

  async function markAllOwned(items: { weaponId: string; status: number }[]) {
    if (markingAll.value) return;
    markingAll.value = true;
    try {
      for (const it of items) {
        if (it.status === 1) continue;
        await updateWeaponStatus(it.weaponId, 1);
      }
    } catch (e) {
      opts.error.value = String(e);
    } finally {
      markingAll.value = false;
    }
  }

  return {
    weaponSlot,
    archwingWeaponSlot,
    weaponKindFilter,
    weaponProgressFilter,
    weaponVaultFilter,
    weaponIncarnonFilter,
    selectedWeaponFamilyId,
    selectedWeaponId,
    weaponDetail,
    weaponDetailLoading,
    markingAll,
    closeWeaponDrawer,
    openWeapon,
    openWeaponFromHome,
    updateWeaponStatus,
    updateWeaponIncarnon,
    updateWeaponNote,
    updateWeaponPartStatus,
    updateWeaponPartHaveQty,
    toggleWeaponWish,
    toggleWeaponMastered,
    updateWeaponPartNote,
    markAllOwned,
  };
}
