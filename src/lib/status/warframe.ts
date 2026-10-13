/** 与设计文档 11 状态码表一致 */

export type FilterKey =
  | "all"
  | "helminth"
  | "rebirth"
  | "farmable"
  | "noPrime"
  | "noBase"
  | "unforma"
  | "unmastered";

/** 进度状态筛选，与场景 FilterKey 独立、同时 AND */
export type WarframeStatusFilter = "all" | 0 | 1 | 2 | 3;

export function baseStatusLabel(code: number): string {
  switch (code) {
    case 1:
      return "蓝图已齐";
    case 2:
      return "铸造中";
    case 3:
      return "已获取";
    default:
      return "未获取";
  }
}

export function primeStatusLabel(code: number, hasPrime: boolean): string {
  if (!hasPrime) return "尚无";
  switch (code) {
    case 1:
      return "蓝图已齐";
    case 2:
      return "铸造中";
    case 3:
      return "已获取";
    default:
      return "未获取";
  }
}

export function vaultStatusLabel(code: number | null | undefined, hasPrime: boolean): string {
  if (!hasPrime || code == null || code === 0) return "—";
  switch (code) {
    case 1:
      return "可刷";
    case 2:
      return "已入库";
    case 3:
      return "重生";
    default:
      return "—";
  }
}

export function partStatusLabel(code: number): string {
  switch (code) {
    case 1:
      return "有蓝图";
    case 2:
      return "铸造中";
    case 3:
      return "已获取";
    default:
      return "未获取";
  }
}

export function warframeVariantLabel(variant: number): string {
  switch (variant) {
    case 1:
      return "Prime";
    case 2:
      return "Umbra";
    default:
      return "本体";
  }
}

export const BASE_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 1, label: "蓝图已齐" },
  { value: 2, label: "铸造中" },
  { value: 3, label: "已获取" },
] as const;

export const WARFRAME_STATUS_FILTER_OPTS: {
  key: WarframeStatusFilter;
  label: string;
}[] = [
  { key: "all", label: "全部" },
  ...BASE_STATUS_OPTIONS.map((o) => ({ key: o.value, label: o.label })),
];

export const PRIME_STATUS_OPTIONS = BASE_STATUS_OPTIONS;

export const PART_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 1, label: "有蓝图" },
  { value: 2, label: "铸造中" },
  { value: 3, label: "已获取" },
] as const;
