import { invoke } from "@tauri-apps/api/core";

export interface ArcaneListItem {
  arcaneId: string;
  nameEn: string;
  nameZh: string | null;
  arcaneKind: number;
  rarity: number;
  sourceKind: number;
  maxRank: number;
  status: number;
  rank: number;
  copiesToMax: number;
  wish: number;
  note: string | null;
}

export interface UpsertArcaneProgress {
  arcaneId: string;
  status?: number;
  rank?: number;
  wish?: number;
  note?: string | null;
}

export function listArcanes() {
  return invoke<ArcaneListItem[]>("list_arcanes");
}

export function upsertArcaneProgress(patch: UpsertArcaneProgress) {
  return invoke<void>("upsert_arcane_progress", { patch });
}
