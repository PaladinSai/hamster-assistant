import type {
  LiveArchimedeaMissionDto,
  LiveArchonMissionDto,
  LiveHexCalendarDayDto,
  LiveLabeledTerm,
  LiveNightwaveChallengeDto,
  LiveSortieMissionDto,
  LiveSteelIncursionMissionDto,
} from "../../lib/api";

/** 日常/周常条目展开后的时效详情（由 brief 组装，不触网）。 */
export type LiveDetail =
  | {
      kind: "sortie";
      summary: string;
      boss: string;
      faction: string;
      missions: LiveSortieMissionDto[];
      expiryLabel: string;
      expiry: string | null;
    }
  | {
      kind: "nightwave";
      summary: string;
      challenges: LiveNightwaveChallengeDto[];
    }
  | {
      kind: "steel";
      summary: string;
      expiry: string | null;
      today: LiveSteelIncursionMissionDto[];
      tomorrow: LiveSteelIncursionMissionDto[];
      expiryLabel: string;
    }
  | {
      kind: "archon";
      summary: string;
      boss: string;
      shard: string;
      missions: LiveArchonMissionDto[];
      expiry: string | null;
    }
  | {
      kind: "archimedea";
      summary: string;
      kindLabel: string;
      missions: LiveArchimedeaMissionDto[];
      personalModifiers: LiveLabeledTerm[];
      expiry: string | null;
    }
  | {
      kind: "circuit";
      summary: string;
      label: string;
      thisWeek: string[];
      nextWeek: string[];
      rotationWeeks: string[][];
      currentWeekIndex: number | null;
      expiry: string | null;
    }
  | {
      kind: "hex";
      summary: string;
      days: LiveHexCalendarDayDto[];
      expiry: string | null;
    }
  | { kind: "empty"; summary: string };

export type SortieLiveDetail = Extract<LiveDetail, { kind: "sortie" }>;
export type NightwaveLiveDetail = Extract<LiveDetail, { kind: "nightwave" }>;
export type SteelLiveDetail = Extract<LiveDetail, { kind: "steel" }>;
export type ArchonLiveDetail = Extract<LiveDetail, { kind: "archon" }>;
export type ArchimedeaLiveDetail = Extract<LiveDetail, { kind: "archimedea" }>;
export type CircuitLiveDetail = Extract<LiveDetail, { kind: "circuit" }>;
export type HexLiveDetail = Extract<LiveDetail, { kind: "hex" }>;
