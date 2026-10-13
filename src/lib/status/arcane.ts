/** 赋能三态 */
export function arcaneStatusLabel(code: number): string {
  switch (code) {
    case 1:
      return "未满级";
    case 2:
      return "已获取";
    default:
      return "未获取";
  }
}

export function arcaneKindLabel(code: number): string {
  switch (code) {
    case 0:
      return "战甲赋能";
    case 1:
      return "主要武器赋能";
    case 2:
      return "次要武器赋能";
    case 3:
      return "近战武器赋能";
    case 4:
      return "指挥官赋能";
    case 5:
      return "增幅器赋能";
    case 6:
      return "组合枪赋能";
    case 7:
      return "Zaw 赋能";
    default:
      return "其它";
  }
}

export function arcaneRarityLabel(code: number): string {
  switch (code) {
    case 0:
      return "常见";
    case 1:
      return "罕见";
    case 2:
      return "稀有";
    case 3:
      return "传说";
    default:
      return "—";
  }
}

export function arcaneSourceLabel(code: number): string {
  switch (code) {
    case 1:
      return "夜灵";
    case 2:
      return "隔离库";
    case 3:
      return "扎里曼";
    case 4:
      return "双衍王境";
    case 5:
      return "航道星舰";
    case 6:
      return "1999·六人组";
    case 7:
      return "解剖圣所";
    case 8:
      return "集团商人";
    case 9:
      return "侍从";
    case 10:
      return "扬升";
    case 11:
      return "仲裁";
    case 12:
      return "联结生存";
    default:
      return "其它";
  }
}

export type ArcaneKindFilter = "all" | number;
export type ArcaneRarityFilter = "all" | number;
export type ArcaneSourceFilter = "all" | number;
export type ArcaneProgressFilter = "all" | "missing" | "partial" | "maxed";

export const ARCANE_STATUS_OPTIONS = [
  { value: 0, label: "未获取" },
  { value: 1, label: "未满级" },
  { value: 2, label: "已获取" },
] as const;

export const ARCANE_KIND_FILTER_OPTS: { key: ArcaneKindFilter; label: string }[] =
  [
    { key: "all", label: "全部品类" },
    { key: 0, label: "战甲赋能" },
    { key: 1, label: "主要武器赋能" },
    { key: 2, label: "次要武器赋能" },
    { key: 3, label: "近战武器赋能" },
    { key: 4, label: "指挥官赋能" },
    { key: 5, label: "增幅器赋能" },
    { key: 6, label: "组合枪赋能" },
    { key: 7, label: "Zaw 赋能" },
    { key: 8, label: "其它" },
  ];

export const ARCANE_RARITY_FILTER_OPTS: {
  key: ArcaneRarityFilter;
  label: string;
}[] = [
  { key: "all", label: "全部品级" },
  { key: 0, label: "常见" },
  { key: 1, label: "罕见" },
  { key: 2, label: "稀有" },
  { key: 3, label: "传说" },
];

export const ARCANE_SOURCE_FILTER_OPTS: {
  key: ArcaneSourceFilter;
  label: string;
}[] = [
  { key: "all", label: "全部来源" },
  { key: 1, label: "夜灵" },
  { key: 2, label: "隔离库" },
  { key: 3, label: "扎里曼" },
  { key: 4, label: "双衍王境" },
  { key: 5, label: "航道星舰" },
  { key: 6, label: "1999·六人组" },
  { key: 7, label: "解剖圣所" },
  { key: 8, label: "集团商人" },
  { key: 9, label: "侍从" },
  { key: 10, label: "扬升" },
  { key: 11, label: "仲裁" },
  { key: 12, label: "联结生存" },
  { key: 0, label: "其它" },
];

export const ARCANE_PROGRESS_FILTER_OPTS: {
  key: ArcaneProgressFilter;
  label: string;
}[] = [
  { key: "all", label: "全部" },
  { key: "missing", label: "未获取" },
  { key: "partial", label: "未满级" },
  { key: "maxed", label: "已获取" },
];
