import { invoke } from "@tauri-apps/api/core";

export interface DbStatus {
  paths: { dataDir: string; catalog: string; user: string };
  catalogSchemaVersion: number;
  userSchemaVersion: number;
  catalogTables: string[];
  userTables: string[];
}

export interface ResurgenceWarframeDto {
  familyId: string | null;
  familyKey: string;
  nameEn: string;
  nameZh: string | null;
  nickname: string | null;
}

export interface ResurgenceWeaponDto {
  weaponId: string | null;
  familyId: string | null;
  nameEn: string;
  nameZh: string | null;
  uniqueName: string | null;
}

export interface ResurgenceSummary {
  activation: string | null;
  expiry: string | null;
  warframes: ResurgenceWarframeDto[];
  weapons: ResurgenceWeaponDto[];
  relicCount: number;
  fresh: boolean;
  next: ResurgenceNextDto | null;
}

export interface ResurgenceNextDto {
  activation: string | null;
  expiry: string | null;
  warframes: ResurgenceWarframeDto[];
}

export interface RelicSourceQuery {
  query?: string;
  relicKey?: string;
  relicKeys?: string[];
  eras?: number[];
  gameModes?: string[];
  rotations?: string[];
  planets?: string[];
}

export interface RelicLocationDto {
  sourceKind: string;
  planetOrHub: string;
  planetOrHubZh: string | null;
  nodeOrBounty: string;
  nodeOrBountyZh: string | null;
  gameMode: string | null;
  gameModeZh: string | null;
  rotation: string | null;
  chance: number | null;
  stage: string | null;
  isEvent: number;
}

export interface RelicMatchedItemDto {
  itemNameEn: string;
  itemNameZh: string | null;
  rarity: number | null;
}

export interface RelicSourceItemDto {
  relicKey: string;
  nameZh: string;
  era: number;
  vaulted: number;
  matchedItems: RelicMatchedItemDto[];
  locations: RelicLocationDto[];
}

export interface FilterChip {
  key: string;
  labelZh: string;
  era: number | null;
}

export interface RelicSourceFilterOptions {
  relics: FilterChip[];
  modes: FilterChip[];
  planets: FilterChip[];
  rotations: string[];
  hasBlankRotation: boolean;
  hasBounty: boolean;
}

export interface RelicSourceSearchResult {
  items: RelicSourceItemDto[];
  truncated: boolean;
  filterOptions: RelicSourceFilterOptions;
}

export function getDbStatus() {
  return invoke<DbStatus>("get_db_status");
}

export function getResurgence() {
  return invoke<ResurgenceSummary>("get_resurgence");
}

/** 踢后台更新战甲；完成后听 backend://warframes-updated / update-finished */
export function kickUpdateWarframes() {
  return invoke<void>("kick_update_warframes");
}

/** 踢后台更新武器；完成后听 backend://weapons-updated / update-finished */
export function kickUpdateWeapons() {
  return invoke<void>("kick_update_weapons");
}

/** 踢后台更新模组；完成后听 backend://mods-updated / update-finished */
export function kickUpdateMods() {
  return invoke<void>("kick_update_mods");
}

/** 踢后台更新赋能；完成后听 backend://arcanes-updated / update-finished */
export function kickUpdateArcanes() {
  return invoke<void>("kick_update_arcanes");
}

/** 踢后台更新遗物；完成后听 backend://relics-updated / update-finished */
export function kickUpdateRelics() {
  return invoke<void>("kick_update_relics");
}

export function searchRelicSources(query: RelicSourceQuery) {
  return invoke<RelicSourceSearchResult>("search_relic_sources_cmd", { query });
}

export function getRelicSourceFilters() {
  return invoke<RelicSourceFilterOptions>("get_relic_source_filters");
}
