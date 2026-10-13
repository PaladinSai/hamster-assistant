import { ref } from "vue";
import {
  listArcanes,
  listMods,
  listModTags,
  listWarframes,
  listWeaponFamilies,
  type ArcaneListItem,
  type ModListItem,
  type ModTagDto,
  type WarframeListItem,
  type WeaponFamilyListItem,
} from "../lib/api";
import type { View } from "./useAppNav";

/** 本地列表种类（非通用 data layer，只服务壳层预热 / ensureLoaded）。 */
type CatalogKind =
  | "warframes"
  | "archwing"
  | "necramechs"
  | "companions"
  | "weapons"
  | "archwingWeapons"
  | "mods"
  | "arcanes";

/** 进入某 view 时需要保证已加载的种类。 */
const VIEW_KINDS: Partial<Record<View, CatalogKind[]>> = {
  home: ["warframes", "weapons", "mods", "arcanes"],
  feed: ["warframes", "weapons"],
  warframes: ["warframes"],
  archwing: ["archwing"],
  necramechs: ["necramechs"],
  companions: ["companions"],
  weapons: ["weapons"],
  archwingWeapons: ["archwingWeapons"],
  mods: ["mods"],
  arcanes: ["arcanes"],
};

function runWhenIdle(fn: () => void, timeoutMs = 1200) {
  const ric = (
    globalThis as unknown as {
      requestIdleCallback?: (cb: () => void, opts?: { timeout: number }) => number;
    }
  ).requestIdleCallback;
  if (typeof ric === "function") {
    ric(fn, { timeout: timeoutMs });
    return;
  }
  setTimeout(fn, 0);
}

export function useCatalogData() {
  const families = ref<WarframeListItem[]>([]);
  const archwingFamilies = ref<WarframeListItem[]>([]);
  const necramechFamilies = ref<WarframeListItem[]>([]);
  const companionFamilies = ref<WarframeListItem[]>([]);
  const weaponFamilies = ref<WeaponFamilyListItem[]>([]);
  const archwingWeaponFamilies = ref<WeaponFamilyListItem[]>([]);
  const mods = ref<ModListItem[]>([]);
  const modTags = ref<ModTagDto[]>([]);
  const arcanes = ref<ArcaneListItem[]>([]);
  const error = ref("");
  const warming = ref(false);
  const inflight = new Map<CatalogKind, Promise<void>>();

  function isEmpty(kind: CatalogKind): boolean {
    switch (kind) {
      case "warframes":
        return families.value.length === 0;
      case "archwing":
        return archwingFamilies.value.length === 0;
      case "necramechs":
        return necramechFamilies.value.length === 0;
      case "companions":
        return companionFamilies.value.length === 0;
      case "weapons":
        return weaponFamilies.value.length === 0;
      case "archwingWeapons":
        return archwingWeaponFamilies.value.length === 0;
      case "mods":
        return mods.value.length === 0;
      case "arcanes":
        return arcanes.value.length === 0;
    }
  }

  async function loadByKind(kind: CatalogKind) {
    const existing = inflight.get(kind);
    if (existing) return existing;

    const p = (async () => {
      error.value = "";
      try {
        switch (kind) {
          case "warframes":
            families.value = await listWarframes("warframe");
            break;
          case "archwing":
            archwingFamilies.value = await listWarframes("archwing");
            break;
          case "necramechs":
            necramechFamilies.value = await listWarframes("necramech");
            break;
          case "companions":
            companionFamilies.value = await listWarframes("companion");
            break;
          case "weapons":
            weaponFamilies.value = await listWeaponFamilies("weapon");
            break;
          case "archwingWeapons":
            archwingWeaponFamilies.value = await listWeaponFamilies("archwing");
            break;
          case "mods": {
            const [list, tags] = await Promise.all([listMods(), listModTags()]);
            mods.value = list;
            modTags.value = tags;
            break;
          }
          case "arcanes":
            arcanes.value = await listArcanes();
            break;
        }
      } catch (e) {
        error.value = String(e);
      } finally {
        inflight.delete(kind);
      }
    })();
    inflight.set(kind, p);
    return p;
  }

  const loadFamilies = () => loadByKind("warframes");
  const loadArchwingFamilies = () => loadByKind("archwing");
  const loadNecramechFamilies = () => loadByKind("necramechs");
  const loadCompanionFamilies = () => loadByKind("companions");
  const loadWeaponFamilies = () => loadByKind("weapons");
  const loadArchwingWeaponFamilies = () => loadByKind("archwingWeapons");
  const loadMods = () => loadByKind("mods");
  const loadArcanes = () => loadByKind("arcanes");

  function ensureLoaded(next: View) {
    for (const kind of VIEW_KINDS[next] ?? []) {
      if (isEmpty(kind)) void loadByKind(kind);
    }
  }

  /**
   * 启动预热：先拉战甲/武器（壳子可交互），再空闲时后台补 Mod/赋能（心愿单）。
   * 全部 fire-and-forget，不阻塞首屏。
   */
  function preloadAll() {
    if (warming.value) return;
    warming.value = true;
    void (async () => {
      try {
        await Promise.all([
          isEmpty("warframes") ? loadByKind("warframes") : Promise.resolve(),
          isEmpty("weapons") ? loadByKind("weapons") : Promise.resolve(),
        ]);
      } finally {
        runWhenIdle(() => {
          void (async () => {
            try {
              await Promise.all([
                isEmpty("mods") ? loadByKind("mods") : Promise.resolve(),
                isEmpty("arcanes") ? loadByKind("arcanes") : Promise.resolve(),
              ]);
            } finally {
              warming.value = false;
            }
          })();
        });
      }
    })();
  }

  return {
    families,
    archwingFamilies,
    necramechFamilies,
    companionFamilies,
    weaponFamilies,
    archwingWeaponFamilies,
    mods,
    modTags,
    arcanes,
    error,
    loadFamilies,
    loadArchwingFamilies,
    loadNecramechFamilies,
    loadCompanionFamilies,
    loadWeaponFamilies,
    loadArchwingWeaponFamilies,
    loadMods,
    loadArcanes,
    ensureLoaded,
    preloadAll,
  };
}
