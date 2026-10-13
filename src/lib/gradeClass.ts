import { rarityLabel } from "./status";

/** 稀有度 → CSS class（金/银/铜） */
export function gradeClass(rarity: number | null | undefined): string {
  const label = rarityLabel(rarity);
  if (label === "金") return "gold";
  if (label === "银") return "silver";
  if (label === "铜") return "bronze";
  return "";
}
