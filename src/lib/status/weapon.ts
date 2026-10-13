/** 武器变体码 → 短标签（列表 pill；对齐灰机常见译名） */
export function weaponVariantLabel(code: number): string {
  switch (code) {
    case 1:
      return "Prime";
    case 2:
      return "赤毒";
    case 3:
      return "信条";
    case 4:
      return "终幕";
    case 5:
      return "MK1";
    case 6:
      return "Dex";
    case 7:
      return "破坏者"; // Vandal
    case 8:
      return "亡魂"; // Wraith
    case 9:
      return "棱晶"; // Prisma
    default:
      return "本体";
  }
}

/** 抽屉 Tab / 标题用：优先简中全称 */
export function weaponDisplayName(
  nameZh: string | null | undefined,
  nameEn: string,
): string {
  const zh = nameZh?.trim();
  return zh || nameEn;
}

export function isSpecialWeaponVariant(code: number): boolean {
  return code === 2 || code === 3 || code === 4;
}

/** 武器整枪进度：0 未获取 / 2 蓝图已齐 / 1 已获取 */
export function weaponStatusLabel(code: number): string {
  switch (code) {
    case 1:
      return "已获取";
    case 2:
      return "蓝图已齐";
    default:
      return "未获取";
  }
}

export function weaponMemberStatusLabel(_variant: number, status: number): string {
  return weaponStatusLabel(status);
}

/** 灵化进度（同码）：0 未获取 / 1 已灵化 / 2 已获取灵化之源 */
export function incarnonLabel(canIncarnon: number, incarnon: number): string {
  if (!canIncarnon) return "无";
  switch (incarnon) {
    case 1:
      return "已灵化";
    case 2:
      return "已获取灵化之源";
    default:
      return "未获取";
  }
}

export function weaponVaultStatusLabel(code: number | null | undefined): string {
  switch (code) {
    case 1:
      return "未入库";
    case 2:
      return "已入库";
    case 3:
      return "重生中";
    default:
      return "—";
  }
}

export type WeaponKindFilter =
  | "all"
  | "kuva"
  | "tenet"
  | "coda"
  | "prime"
  | "incarnon"
  | "kitgun"
  | "zaw"
  | "amp";

/** 自制精通部件 pill */
export function modularWeaponKindLabel(
  kind: string | null | undefined,
): string {
  switch (kind) {
    case "kitgun":
      return "组合枪";
    case "zaw":
      return "Zaw";
    case "amp":
      return "增幅器";
    default:
      return "自制";
  }
}

/** 第三层：进度维（整枪 / 全部分类共用） */
export type WeaponProgressFilter = "all" | "missing" | "progress" | "owned";

/** 第三层：Prime 入库维（catalog） */
export type WeaponVaultFilter = "all" | "unvaulted" | "vaulted" | "resurgence";

/** 第三层：灵化维 */
export type WeaponIncarnonFilter = "all" | "missing" | "ready" | "done";

/** 可铸造线：未获取 / 蓝图已齐 / 已获取 */
export const WEAPON_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 2, label: "蓝图已齐" },
  { value: 1, label: "已获取" },
] as const;

/** 赤毒/信条/终幕：仅未获取 / 已获取 */
export const WEAPON_SPECIAL_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 1, label: "已获取" },
] as const;

/** @deprecated 使用 WEAPON_STATUS_OPTIONS */
export const WEAPON_CRAFT_STATUS_OPTIONS = WEAPON_STATUS_OPTIONS;

export const WEAPON_PART_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 3, label: "已获取" },
] as const;

export const INCARNON_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 2, label: "已获取灵化之源" },
  { value: 1, label: "已灵化" },
] as const;
