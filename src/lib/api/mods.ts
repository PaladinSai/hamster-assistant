import { invoke } from "@tauri-apps/api/core";

export interface ModTagDto {
  id: string;
  name: string;
  sortOrder: number;
  isBuiltin: boolean;
}

export interface ModListItem {
  modId: string;
  nameEn: string;
  nameZh: string | null;
  modKind: number;
  sourceKind: number;
  isPrime: number;
  isAugment: number;
  polarity: string | null;
  status: number;
  wish: number;
  note: string | null;
  tagId: string | null;
  tagName: string | null;
}

export interface UpsertModProgress {
  modId: string;
  status?: number;
  wish?: number;
  note?: string | null;
  /** undefined=不改；null=清空；string=设置 */
  tagId?: string | null;
}

export function listMods() {
  return invoke<ModListItem[]>("list_mods");
}

export function listModTags() {
  return invoke<ModTagDto[]>("list_mod_tags");
}

export function createModTag(name: string) {
  return invoke<ModTagDto>("create_mod_tag", { name });
}

export function deleteModTag(id: string) {
  return invoke<void>("delete_mod_tag", { id });
}

export function upsertModProgress(patch: UpsertModProgress) {
  return invoke<void>("upsert_mod_progress", { patch });
}
