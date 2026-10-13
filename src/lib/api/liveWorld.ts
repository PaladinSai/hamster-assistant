import { invoke } from "@tauri-apps/api/core";

/** 前端只读本地库 DTO；网络更新由 Rust 后台完成后再发事件。 */

export interface LiveSortieMissionDto {
  sortOrder: number;
  missionType: string;
  modifier: string;
  node: string;
}

export interface LiveSortieDto {
  activation: string | null;
  expiry: string | null;
  boss: string;
  faction: string;
  missions: LiveSortieMissionDto[];
}

export interface LiveNightwaveChallengeDto {
  title: string;
  description: string;
  isElite: boolean;
  activation: string | null;
  expiry: string | null;
  reputation: number | null;
}

export interface LiveSteelIncursionMissionDto {
  missionType: string;
  faction: string;
  minLevel: number;
  maxLevel: number;
  node: string;
}

export interface LiveSteelIncursionDto {
  activation: string | null;
  expiry: string | null;
  today: LiveSteelIncursionMissionDto[];
  tomorrow: LiveSteelIncursionMissionDto[];
}

export interface LiveArchonMissionDto {
  missionType: string;
  node: string;
}

export interface LiveArchonHuntDto {
  activation: string | null;
  expiry: string | null;
  boss: string;
  shard: string;
  missions: LiveArchonMissionDto[];
}

export interface LiveLabeledTerm {
  title: string;
  description: string;
}

export interface LiveArchimedeaMissionDto {
  missionType: string;
  faction: string;
  deviation: LiveLabeledTerm;
  risksNormal: LiveLabeledTerm[];
  risksElite: LiveLabeledTerm[];
}

export interface LiveArchimedeaDto {
  activation: string | null;
  expiry: string | null;
  kindLabel: string;
  missions: LiveArchimedeaMissionDto[];
  personalModifiers: LiveLabeledTerm[];
}

export interface LiveCircuitDto {
  activation: string | null;
  expiry: string | null;
  thisWeek: string[];
  nextWeek: string[];
  rotationWeeks: string[][];
  currentWeekIndex: number | null;
}

export interface LiveHexCalendarGroupDto {
  kind: string;
  choices: LiveLabeledTerm[];
}

export interface LiveHexCalendarDayDto {
  day: number;
  groups: LiveHexCalendarGroupDto[];
}

export interface LiveHexCalendarDto {
  activation: string | null;
  expiry: string | null;
  days: LiveHexCalendarDayDto[];
}

export interface LiveNightwaveSeasonDto {
  activation: string | null;
  expiry: string | null;
}

export interface LiveDailyBrief {
  sortie: LiveSortieDto | null;
  nightwaveDaily: LiveNightwaveChallengeDto[];
  nightwaveWeekly: LiveNightwaveChallengeDto[];
  nightwaveSeason: LiveNightwaveSeasonDto | null;
  steelIncursion: LiveSteelIncursionDto | null;
  archonHunt: LiveArchonHuntDto | null;
  deepArchimedea: LiveArchimedeaDto | null;
  temporalArchimedea: LiveArchimedeaDto | null;
  steelCircuit: LiveCircuitDto | null;
  normalCircuit: LiveCircuitDto | null;
  hexCalendar: LiveHexCalendarDto | null;
}

/** 读本地 catalog 组装的日常/周常 DTO（不触网）。 */
export function getLiveDailyBrief() {
  return invoke<LiveDailyBrief>("get_live_daily_brief_cmd");
}

/** 用户点「更新数据」：踢 Rust 后台强制刷新，立即返回。 */
export function kickUpdateLiveWorld() {
  return invoke<void>("kick_update_live_world");
}

/** 动态页：踢后台强制刷新电波+重生，立即返回。 */
export function kickUpdateFeed() {
  return invoke<void>("kick_update_feed");
}

export interface ShopOfferDto {
  nameEn: string;
  nameZh: string;
  cost: number;
  costZh: string;
}

export interface WeeklyShopBrief {
  /** live | fallback */
  source: string;
  teshinActivation: string | null;
  teshinExpiry: string | null;
  teshinCurrent: ShopOfferDto;
  teshinRotation: ShopOfferDto[];
  bird3Current: ShopOfferDto;
  bird3Rotation: ShopOfferDto[];
}

/** 周限商店：钢铁之路阁下 live + 鸟三周序；失败回退本地推算。 */
export function getWeeklyShopBrief() {
  return invoke<WeeklyShopBrief>("get_weekly_shop_brief_cmd");
}

export interface LiveFissureDto {
  kind: string;
  tier: string;
  tierZh: string;
  missionType: string;
  node: string;
  activation: string | null;
  expiry: string | null;
}

export interface FissureBriefDto {
  normal: LiveFissureDto[];
  steel: LiveFissureDto[];
  storm: LiveFissureDto[];
}

/** 虚空裂隙：普通 / 钢铁 / 九重天（只读本地库）。 */
export function getFissureBrief() {
  return invoke<FissureBriefDto>("get_fissure_brief_cmd");
}

export interface ArbitrationSlotDto {
  activation: string;
  expiry: string;
  node: string;
  missionType: string;
  faction: string;
  nodeCode: string;
}

export interface ArbitrationBriefDto {
  source: string;
  current: ArbitrationSlotDto | null;
  upcoming: ArbitrationSlotDto[];
}

/** 仲裁：browse.wf 日程 · 当前 + 后续预测。 */
export function getArbitrationBrief() {
  return invoke<ArbitrationBriefDto>("get_arbitration_brief_cmd");
}

export interface PlainsCycleDto {
  id: string;
  nameZh: string;
  state: string;
  stateZh: string;
  nextStateZh: string;
  activation: string;
  expiry: string;
  noteZh: string | null;
}

export interface PlainsBriefDto {
  cycles: PlainsCycleDto[];
}

/** 开放世界循环：夜灵 / 福尔图娜 / 魔胎 / 双衍 / 地球。 */
export function getPlainsBrief() {
  return invoke<PlainsBriefDto>("get_plains_brief_cmd");
}
