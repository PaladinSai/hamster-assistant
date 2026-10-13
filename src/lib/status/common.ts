export function relicEraLabel(code: number | null | undefined): string {
  if (code == null) return "";
  switch (code) {
    case 0:
      return "古纪";
    case 1:
      return "前纪";
    case 2:
      return "中纪";
    case 3:
      return "后纪";
    default:
      return "";
  }
}

export function rarityLabel(code: number | null | undefined): string {
  if (code == null) return "";
  switch (code) {
    case 0:
      return "铜";
    case 1:
      return "银";
    case 2:
      return "金";
    default:
      return "";
  }
}

/** 核桃展示：优先 relic_label，否则纪元+code */
export function relicDisplay(
  label: string | null | undefined,
  era: number | null | undefined,
  code: string | null | undefined,
): string {
  if (label) return label;
  if (!code) return "";
  const eraZh = relicEraLabel(era);
  return eraZh ? `${eraZh} ${code}` : code;
}

export type PillKind = "ok" | "warn" | "bad" | "mute" | "info";

export function statusPillKind(label: string): PillKind {
  const map: Record<string, PillKind> = {
    未满级: "warn",
    已有: "ok", // 兼容旧文案
    已获取: "ok",
    已灵化: "ok",
    铸造中: "warn",
    蓝图已齐: "warn",
    有蓝图: "warn",
    进行中: "warn",
    已有灵化之源: "warn", // 兼容旧文案
    已获取灵化之源: "warn",
    没有: "bad", // 兼容旧文案
    未获取: "bad",
    未获取灵化之源: "bad",
    尚无: "mute",
    可刷: "info",
    未入库: "info",
    重生: "warn",
    重生中: "warn",
    已入库: "mute",
    可灵化: "info",
    无: "mute",
  };
  return map[label] ?? "mute";
}
