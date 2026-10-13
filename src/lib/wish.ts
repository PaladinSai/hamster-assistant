import type {
  ArcaneListItem,
  ModListItem,
  WarframeListItem,
  WeaponFamilyListItem,
} from "./api";
import {
  arcaneStatusLabel,
  baseStatusLabel,
  modSourceLabel,
  modStatusLabel,
  warframeVariantLabel,
  weaponDisplayName,
  weaponStatusLabel,
  weaponVariantLabel,
} from "./status";

export function nextWish(f: WarframeListItem): number {
  return f.wish ? 0 : 1;
}

export function wishTitle(f: WarframeListItem): string {
  return f.wish ? "取消心愿" : "加入心愿";
}

export type WishRow =
  | {
      key: string;
      kind: "warframe";
      tag: string;
      nameEn: string;
      nick: string;
      meta: string;
      warframeId: string;
    }
  | {
      key: string;
      kind: "weapon";
      tag: string;
      nameEn: string;
      nick: string;
      meta: string;
      familyId: string;
      weaponId: string;
    }
  | {
      key: string;
      kind: "mod";
      tag: string;
      nameEn: string;
      nick: string;
      meta: string;
      modId: string;
    }
  | {
      key: string;
      kind: "arcane";
      tag: string;
      nameEn: string;
      nick: string;
      meta: string;
      arcaneId: string;
    };

function nickOf(f: { nickname: string | null; nameZh: string | null }) {
  return f.nickname || f.nameZh || "";
}

function slotTag(slot: number) {
  switch (slot) {
    case 0:
      return "主武";
    case 1:
      return "副武";
    case 2:
      return "近战";
    default:
      return "武器";
  }
}

export function buildWishRows(input: {
  warframes: WarframeListItem[];
  weaponFamilies: WeaponFamilyListItem[];
  mods: ModListItem[];
  arcanes: ArcaneListItem[];
}): WishRow[] {
  const out: WishRow[] = [];

  for (const f of input.warframes) {
    if (f.wish !== 1) continue;
    out.push({
      key: `wf-${f.warframeId}`,
      kind: "warframe",
      tag: warframeVariantLabel(f.variant),
      nameEn: f.nameEn,
      nick: nickOf(f),
      meta: f.note || baseStatusLabel(f.status),
      warframeId: f.warframeId,
    });
  }

  for (const f of input.weaponFamilies) {
    for (const m of f.members) {
      if (m.wish !== 1) continue;
      out.push({
        key: `wp-${m.weaponId}`,
        kind: "weapon",
        tag: slotTag(f.slot),
        nameEn: weaponDisplayName(m.nameZh, m.nameEn || f.nameEn),
        nick: "",
        meta:
          m.note || weaponVariantLabel(m.variant) || weaponStatusLabel(m.status),
        familyId: f.familyId,
        weaponId: m.weaponId,
      });
    }
  }

  for (const m of input.mods) {
    if (m.wish !== 1) continue;
    out.push({
      key: `mod-${m.modId}`,
      kind: "mod",
      tag: "Mod",
      nameEn: m.nameZh?.trim() || m.nameEn,
      nick: "",
      meta:
        m.note || m.tagName || modSourceLabel(m.sourceKind) || modStatusLabel(m.status),
      modId: m.modId,
    });
  }

  for (const a of input.arcanes) {
    if (a.wish !== 1) continue;
    out.push({
      key: `arc-${a.arcaneId}`,
      kind: "arcane",
      tag: "赋能",
      nameEn: a.nameZh?.trim() || a.nameEn,
      nick: "",
      meta: a.note || arcaneStatusLabel(a.status),
      arcaneId: a.arcaneId,
    });
  }

  return out;
}
