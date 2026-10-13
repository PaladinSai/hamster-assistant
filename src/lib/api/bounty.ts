import { invoke } from "@tauri-apps/api/core";

/** 与 Rust `BountyJobApi` 同形（IPC camelCase）。 */
export interface BountyJobDto {
  jobType: string;
  kind: string;
  kindZh: string;
  title: string;
  minEnemyLevel: number;
  maxEnemyLevel: number;
  standingTotal: number;
  isAdvanced: boolean;
  isSteelPath: boolean;
  /** 挑战文案；无挑战时为空串（与 Rust `challenge_zh: String` 对齐）。 */
  challengeZh: string;
  rewards?: string[];
}

/** 与 Rust `BountyBoardApi` 同形。 */
export interface BountyBoardDto {
  tag: string;
  name: string;
  activation: string | null;
  expiry: string | null;
  jobs: BountyJobDto[];
}

/** 与 Rust `BountyBriefApi` 同形。 */
export interface BountyBriefDto {
  boards: BountyBoardDto[];
}

/** @deprecated 旧名；请用 {@link BountyBriefDto}（对应 Rust `BountyBriefApi`）。 */
export type BountyBriefApi = BountyBriefDto;

export interface BountyWatchPrefs {
  followedSyndicates: string[];
  followedJobKinds: string[];
  lastNotified: string[];
}

/** 各势力声望等级（0–5），存 user 库。 */
export interface StandingRanksPrefs {
  ranks: Record<string, number>;
}

/** 声望今日/本周完成勾选（UTC 日切；卡尔按周一重置）。 */
export interface StandingDailyPrefs {
  dayKey: string;
  done: Record<string, boolean>;
  weekKey: string;
  kahlDone: boolean;
}

/** 与 Rust `STANDING_RANK_MAX` / `standing_rank_max_for` 保持同步（改规则请同 PR 改两侧）。 */
export const STANDING_RANK_MAX = 5;

/** 各势力声望等级上限（殁世械灵满级 3）。见 Rust `standing_rank_max_for`。 */
export function standingRankMax(tag: string): number {
  if (tag === "NecraloidSyndicate") return 3;
  return STANDING_RANK_MAX;
}

export function getBountyBrief() {
  return invoke<BountyBriefDto>("get_bounty_brief_cmd");
}

export function getBountyWatch() {
  return invoke<BountyWatchPrefs>("get_bounty_watch_cmd");
}

export function setBountyWatch(prefs: BountyWatchPrefs) {
  return invoke<BountyWatchPrefs>("set_bounty_watch_cmd", { prefs });
}

export function getStandingRanks() {
  return invoke<StandingRanksPrefs>("get_standing_ranks_cmd");
}

export function setStandingRanks(prefs: StandingRanksPrefs) {
  return invoke<StandingRanksPrefs>("set_standing_ranks_cmd", { prefs });
}

export function getStandingDaily() {
  return invoke<StandingDailyPrefs>("get_standing_daily_cmd");
}

export function setStandingDaily(prefs: StandingDailyPrefs) {
  return invoke<StandingDailyPrefs>("set_standing_daily_cmd", { prefs });
}
