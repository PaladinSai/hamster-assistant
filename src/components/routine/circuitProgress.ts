import type { WarframeListItem, WeaponFamilyListItem } from "../../lib/api";
import { baseStatusLabel } from "../../lib/status";

const SYNDICATE_PREFIXES = ["Synoid ", "Sancti ", "Telos "] as const;

/** 英文名归一（去空格标点）；中文勿走此函数。 */
function normEn(s: string): string {
  return s.replace(/[^a-zA-Z0-9]/g, "").toLowerCase();
}

function genesisBaseKey(familyKey: string): string {
  for (const p of SYNDICATE_PREFIXES) {
    if (familyKey.startsWith(p)) {
      const rest = familyKey.slice(p.length).trim();
      if (rest) return rest;
    }
  }
  return familyKey;
}

/** 回廊展示名（可能已是中文）↔ 图鉴英/中文名。 */
function nameMatches(
  circuitName: string,
  en: string | null | undefined,
  zh: string | null | undefined,
): boolean {
  const raw = circuitName.trim();
  if (!raw) return false;
  if (zh && zh.trim() === raw) return true;
  if (en) {
    if (en === raw) return true;
    if (normEn(en) === normEn(raw)) return true;
    if (normEn(genesisBaseKey(en)) === normEn(raw)) return true;
  }
  return false;
}

function familyMatchesCircuit(
  f: WeaponFamilyListItem,
  circuitName: string,
): boolean {
  if (nameMatches(circuitName, f.familyKey, f.nameZh)) return true;
  if (nameMatches(circuitName, f.nameEn, f.nameZh)) return true;
  if (nameMatches(circuitName, genesisBaseKey(f.familyKey), null)) return true;
  return f.members.some((m) => nameMatches(circuitName, m.nameEn, m.nameZh));
}

/**
 * 钢铁回廊灵化进度：未获取 / 已获取 / 已灵化。
 * 回廊名常被译成中文，需同时对 nameZh。
 */
export function steelCircuitStatus(
  name: string,
  weaponFamilies: WeaponFamilyListItem[],
): string | null {
  const members = weaponFamilies
    .filter((f) => familyMatchesCircuit(f, name))
    .flatMap((f) => f.members.filter((m) => m.canIncarnon === 1));
  if (!members.length) return null;
  if (members.some((m) => m.incarnon === 1)) return "已灵化";
  if (members.some((m) => m.incarnon === 2)) return "已获取";
  return "未获取";
}

/** 无尽回廊：普通甲（variant=0）获取状态。 */
export function normalCircuitStatus(
  name: string,
  families: WarframeListItem[],
): string | null {
  const base =
    families.find(
      (f) => f.variant === 0 && nameMatches(name, f.familyKey, f.nameZh),
    ) ??
    families.find(
      (f) => f.variant === 0 && nameMatches(name, f.nameEn, f.nameZh),
    );
  if (!base) return null;
  return baseStatusLabel(base.status);
}

export function circuitItemStatus(
  label: string,
  name: string,
  opts: {
    weaponFamilies: WeaponFamilyListItem[];
    families: WarframeListItem[];
  },
): string | null {
  if (label === "钢铁回廊") {
    return steelCircuitStatus(name, opts.weaponFamilies);
  }
  if (label === "无尽回廊") {
    return normalCircuitStatus(name, opts.families);
  }
  return null;
}
