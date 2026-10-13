import { invoke } from "@tauri-apps/api/core";

/** 列表/抽屉：一个变体一行 */
export type SuitKind = "warframe" | "archwing" | "necramech" | "companion";

export type CompanionType =
  | "sentinel"
  | "kubrow"
  | "kavat"
  | "vulpaphyla"
  | "predasite"
  | "moa"
  | "hound";

export interface WarframeListItem {
  warframeId: string;
  familyId: string;
  familyKey: string;
  /** 0 本体 / 1 Prime / 2 Umbra */
  variant: number;
  nameEn: string;
  nameZh: string | null;
  nickname: string | null;
  status: number;
  statusDisplay: string;
  helminth: number;
  formaDone: number;
  wish: number;
  note: string | null;
  /** catalog 只读；仅 Prime */
  vaultStatus: number | null;
  mastered: number;
  /** warframe | archwing | necramech | companion */
  suitKind?: SuitKind | string;
  /** 同伴细分 */
  companionType?: CompanionType | string | null;
}

/** @deprecated 使用 WarframeListItem */
export type FamilyListItem = WarframeListItem;

export interface PartDropDto {
  relicName: string;
  relicKey: string;
  rarity: number | null;
  vaulted: number;
  inResurgence: boolean;
}

export interface PartDto {
  partId: string;
  partKey: string;
  nameZh: string;
  sortOrder: number;
  status: number;
  note: string | null;
  drops: PartDropDto[];
}

export interface WarframeDetail {
  warframe: WarframeListItem;
  parts: PartDto[];
}

export interface UpsertWarframeProgress {
  warframeId: string;
  status?: number;
  helminth?: number;
  formaDone?: number;
  wish?: number;
  note?: string | null;
  mastered?: number;
}

export interface UpsertPartProgress {
  partId: string;
  status?: number;
  note?: string | null;
}

export function listWarframes(kind: SuitKind = "warframe") {
  return invoke<WarframeListItem[]>("list_warframes", { kind });
}

/** @deprecated */
export function listWarframeFamilies() {
  return listWarframes();
}

export function getWarframeDetail(warframeId: string) {
  return invoke<WarframeDetail>("get_warframe_detail", { warframeId });
}

export function upsertProgressWarframe(patch: UpsertWarframeProgress) {
  return invoke<void>("upsert_progress_warframe", { patch });
}

export function upsertProgressPart(patch: UpsertPartProgress) {
  return invoke<void>("upsert_progress_part", { patch });
}
