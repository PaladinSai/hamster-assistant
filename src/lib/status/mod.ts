/** Mod 进度：0 未获取 / 1 已获取 */
export function modStatusLabel(code: number): string {
  return code === 1 ? "已获取" : "未获取";
}

export function modKindLabel(code: number): string {
  switch (code) {
    case 0:
      return "战甲";
    case 1:
      return "主武器";
    case 2:
      return "副武器";
    case 3:
      return "近战";
    case 4:
      return "光环";
    case 5:
      return "姿态";
    case 6:
      return "同伴";
    default:
      return "其它";
  }
}

/** catalog source_kind → 英文原文（列表改用标签后，仅作后备展示） */
export function modSourceLabel(code: number): string {
  switch (code) {
    case 1:
      return "Enemy";
    case 2:
      return "Mission";
    case 3:
      return "Cetus";
    case 4:
      return "Fortuna";
    case 5:
      return "Deimos";
    case 6:
      return "Zariman";
    case 7:
      return "Hex";
    case 8:
      return "Derelict Vault";
    case 9:
      return "Sortie";
    case 10:
      return "Syndicate";
    case 11:
      return "Transient";
    case 12:
      return "Nightmare Mode";
    default:
      return "Other";
  }
}

/** 全部 | 无标签 | 具体 tagId */
export type ModTagFilter = "all" | "none" | string;
export type ModKindFilter = "all" | number;
export type ModProgressFilter = "all" | "missing" | "owned";

export const MOD_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 1, label: "已获取" },
] as const;

export const MOD_KIND_FILTER_OPTS: { key: ModKindFilter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: 0, label: "战甲" },
  { key: 1, label: "主武器" },
  { key: 2, label: "副武器" },
  { key: 3, label: "近战" },
  { key: 4, label: "光环" },
  { key: 5, label: "姿态" },
  { key: 6, label: "同伴" },
  { key: 7, label: "其它" },
];

export const MOD_PROGRESS_FILTER_OPTS: {
  key: ModProgressFilter;
  label: string;
}[] = [
  { key: "all", label: "全部" },
  { key: "missing", label: "未获取" },
  { key: "owned", label: "已获取" },
];
