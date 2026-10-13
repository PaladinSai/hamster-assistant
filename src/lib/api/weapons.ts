import { invoke } from "@tauri-apps/api/core";
import type { PartDropDto } from "./warframes";

export interface WeaponMemberDto {
  weaponId: string;
  variant: number;
  nameEn: string;
  nameZh: string | null;
  canIncarnon: number;
  vaultStatus: number;
  status: number;
  incarnon: number;
  wish: number;
  mastered: number;
  note: string | null;
  /** catalog 部件数；>1 才可点开抽屉 */
  partCount: number;
}

export type WeaponKind = "weapon" | "archwing";

export type ModularWeaponKind = "kitgun" | "zaw" | "amp";

export interface WeaponFamilyListItem {
  familyId: string;
  familyKey: string;
  nameEn: string;
  nameZh: string | null;
  nickname: string | null;
  slot: number;
  /** weapon | archwing */
  weaponKind: WeaponKind;
  /** kitgun | zaw | amp；自制精通部件 */
  modularKind: ModularWeaponKind | null;
  members: WeaponMemberDto[];
}

export interface WeaponPartDto {
  partId: string;
  partKey: string;
  nameZh: string;
  sortOrder: number;
  needQty: number;
  haveQty: number;
  status: number;
  note: string | null;
  drops: PartDropDto[];
}

export interface WeaponDetailDto {
  weaponId: string;
  variant: number;
  nameEn: string;
  nameZh: string | null;
  uniqueName: string | null;
  canIncarnon: number;
  vaultStatus: number;
  status: number;
  incarnon: number;
  wish: number;
  mastered: number;
  note: string | null;
  parts: WeaponPartDto[];
}

export interface WeaponFamilyDetail {
  family: WeaponFamilyListItem;
  weapons: WeaponDetailDto[];
}

export interface UpsertWeaponProgress {
  weaponId: string;
  status?: number;
  note?: string | null;
  incarnon?: number;
  wish?: number;
  mastered?: number;
}

export interface UpsertWeaponPartProgress {
  partId: string;
  status?: number;
  haveQty?: number;
  note?: string | null;
}

export function listWeaponFamilies(kind: WeaponKind = "weapon") {
  return invoke<WeaponFamilyListItem[]>("list_weapon_families", { kind });
}

export function getWeaponFamilyDetail(familyId: string) {
  return invoke<WeaponFamilyDetail>("get_weapon_family_detail", { familyId });
}

export function upsertWeaponProgress(patch: UpsertWeaponProgress) {
  return invoke<void>("upsert_weapon_progress", { patch });
}

export function upsertWeaponPartProgress(patch: UpsertWeaponPartProgress) {
  return invoke<void>("upsert_weapon_part_progress", { patch });
}
