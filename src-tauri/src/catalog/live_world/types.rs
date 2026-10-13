//! 类型与常量。

use serde::{Deserialize, Serialize};

pub(crate) const SETTING_CHECKED_AT: &str = "live_world_checked_at";
pub(crate) const SETTING_SOURCE_URL: &str = "live_world_source_url";
pub(crate) const SETTING_NIGHTWAVE_SEASON: &str = "live_nightwave_season_json";
/// 当前仲裁档到期时间（rfc3339）；后台据此在换档后触发刷新。
pub(crate) const SETTING_ARBITRATION_EXPIRY: &str = "live_arbitration_expiry";
/// 平原循环中最近一档到期时间。
pub(crate) const SETTING_PLAINS_EXPIRY: &str = "live_plains_expiry";
#[allow(dead_code)]
pub(crate) const SETTING_ETAG: &str = "live_world_etag";

/// 旧版 setting blob 键：写入正规表时顺手清掉。
pub(crate) const LEGACY_BOUNTY_JSON_KEY: &str = "live_bounty_brief_json";
pub(crate) const LEGACY_WEEKLY_JSON_KEY: &str = "live_weekly_brief_json";

/// user.sqlite：声望关注偏好
pub const SETTING_BOUNTY_WATCH: &str = "bounty_watch_json";
/// user.sqlite：各势力声望等级（手动记录）
pub const SETTING_STANDING_RANKS: &str = "standing_ranks_json";
/// user.sqlite：声望日常/周常完成勾选（UTC 日切 / 周一重置）
pub const SETTING_STANDING_DAILY: &str = "standing_daily_json";

/// 开放世界声望等级默认上限（含 0 中立）。
/// 前端镜像：`src/lib/api/bounty.ts` 的 `STANDING_RANK_MAX` / `standingRankMax`（改规则请同 PR）。
pub const STANDING_RANK_MAX: i32 = 5;

/// 各势力声望等级上限（未列出的用 [`STANDING_RANK_MAX`]）。
pub fn standing_rank_max_for(tag: &str) -> i32 {
    match tag {
        "NecraloidSyndicate" => 3, // 殁世械灵满级 3
        _ => STANDING_RANK_MAX,
    }
}

/// 周常个人进度（卡尔驻军）。
pub const STANDING_WEEKLY_TAG: &str = "KahlSyndicate";

pub(crate) const SORTIE_ROW_ID: i64 = 1;
pub(crate) const STEEL_ROW_ID: i64 = 1;

/// 与重生一致：warframestat 常 403，官方源用浏览器 UA。
pub(crate) const USER_AGENT: &str = "Mozilla/5.0";

/// browse.wf oracle：坚守者 / 科维兽 / 六人组实时赏金轮换（worldState Jobs 常为空）。
pub(crate) const BOUNTY_CYCLE_URL: &str = "https://oracle.browse.wf/bounty-cycle";

#[derive(Debug, Clone)]
pub enum LiveWorldOutcome {
    SkippedFresh,
    KeptCache,
    Updated,
}

/// 入库用中间结构（已本地化展示文案）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SortieApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub boss: Option<String>,
    pub faction: Option<String>,
    #[serde(default)]
    pub variants: Vec<SortieVariantApi>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SortieVariantApi {
    pub mission_type: Option<String>,
    pub modifier: Option<String>,
    pub node: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NightwaveApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    #[serde(default)]
    pub active_challenges: Vec<NightwaveChallengeApi>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NightwaveChallengeApi {
    pub id: String,
    pub activation: Option<String>,
    pub expiry: Option<String>,
    #[serde(default)]
    pub is_daily: bool,
    #[serde(default)]
    pub is_elite: bool,
    pub title: Option<String>,
    pub desc: Option<String>,
    pub reputation: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SteelPathApi {
    pub incursions: Option<SteelIncursionApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SteelIncursionApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    #[serde(default)]
    pub today: Vec<SteelIncursionMissionApi>,
    #[serde(default)]
    pub tomorrow: Vec<SteelIncursionMissionApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SteelIncursionMissionApi {
    pub mission_type: String,
    pub faction: String,
    pub min_level: i32,
    pub max_level: i32,
    pub node: String,
}

/// 周常动态（解析后、已本地化）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeeklyBundle {
    pub archon_hunt: Option<ArchonHuntApi>,
    pub deep_archimedea: Option<ArchimedeaApi>,
    pub temporal_archimedea: Option<ArchimedeaApi>,
    pub steel_circuit: Option<CircuitApi>,
    pub normal_circuit: Option<CircuitApi>,
    pub hex_calendar: Option<HexCalendarApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchonHuntApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub boss: String,
    #[serde(default)]
    pub shard: String,
    pub missions: Vec<ArchonMissionApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchonMissionApi {
    pub mission_type: String,
    pub node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabeledTerm {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchimedeaApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub kind_label: String,
    pub missions: Vec<ArchimedeaMissionApi>,
    pub personal_modifiers: Vec<LabeledTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchimedeaMissionApi {
    pub mission_type: String,
    pub faction: String,
    pub deviation: LabeledTerm,
    pub risks_normal: Vec<LabeledTerm>,
    pub risks_elite: Vec<LabeledTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CircuitApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub this_week: Vec<String>,
    #[serde(default)]
    pub next_week: Vec<String>,
    /// 完整轮换表（本地内嵌，展示用）。
    #[serde(default)]
    pub rotation_weeks: Vec<Vec<String>>,
    /// 本周在轮换表中的 0-based 下标；匹配失败为 None。
    #[serde(default)]
    pub current_week_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HexCalendarApi {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub days: Vec<HexCalendarDayApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HexCalendarDayApi {
    pub day: i32,
    pub groups: Vec<HexCalendarGroupApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HexCalendarGroupApi {
    /// challenge | reward | upgrade
    pub kind: String,
    pub choices: Vec<LabeledTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveHexCalendarDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub days: Vec<LiveHexCalendarDayDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveHexCalendarDayDto {
    pub day: i32,
    pub groups: Vec<LiveHexCalendarGroupDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveHexCalendarGroupDto {
    pub kind: String,
    pub choices: Vec<LabeledTerm>,
}

/// 裂隙分类：普通 / 钢铁 / 九重天风暴。
pub(crate) const FISSURE_KIND_NORMAL: &str = "normal";
pub(crate) const FISSURE_KIND_STEEL: &str = "steel";
pub(crate) const FISSURE_KIND_STORM: &str = "storm";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FissureApi {
    pub kind: String,
    pub tier: String,
    pub tier_zh: String,
    pub mission_type: String,
    pub node: String,
    /// 官方节点码（如 CrewBattleNode518）；九重天无 MissionType，靠此反查。
    #[serde(default, skip_serializing)]
    pub node_code: String,
    pub activation: Option<String>,
    pub expiry: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveFissureDto {
    pub kind: String,
    pub tier: String,
    pub tier_zh: String,
    pub mission_type: String,
    pub node: String,
    pub activation: Option<String>,
    pub expiry: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FissureBriefDto {
    pub normal: Vec<LiveFissureDto>,
    pub steel: Vec<LiveFissureDto>,
    pub storm: Vec<LiveFissureDto>,
}

#[derive(Debug, Clone)]
pub(crate) struct LiveWorldFetch {
    pub sortie: SortieApi,
    pub nightwave: NightwaveApi,
    pub steel: SteelPathApi,
    pub weekly: WeeklyBundle,
    pub bounties: BountyBriefApi,
    pub fissures: Vec<FissureApi>,
    pub source_url: String,
}

/// 赏金板快照（存 `live_bounty_*` 正规表；前端只读 IPC DTO）。
/// IPC 出口；前端对应 `BountyBriefDto`（`src/lib/api/bounty.ts`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyBriefApi {
    #[serde(default)]
    pub boards: Vec<BountyBoardApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyBoardApi {
    pub tag: String,
    pub name: String,
    pub activation: Option<String>,
    pub expiry: Option<String>,
    #[serde(default)]
    pub jobs: Vec<BountyJobApi>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyJobApi {
    pub job_type: String,
    pub kind: String,
    pub kind_zh: String,
    pub title: String,
    pub min_enemy_level: i32,
    pub max_enemy_level: i32,
    pub standing_total: i32,
    pub is_advanced: bool,
    pub is_steel_path: bool,
    /// 挑战/附加目标中文（cycle 板）；平原赏金可为空。
    #[serde(default)]
    pub challenge_zh: String,
    /// 奖励物品展示名（已尽量中文化）。
    #[serde(default)]
    pub rewards: Vec<String>,
    /// worldState `Jobs[].rewards` 掉落表路径（仅用于解析当前 A/B/C 轮换；不发给前端）。
    #[serde(default, skip_serializing)]
    pub reward_table: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyWatchPrefs {
    #[serde(default)]
    pub followed_syndicates: Vec<String>,
    #[serde(default)]
    pub followed_job_kinds: Vec<String>,
    #[serde(default)]
    pub last_notified: Vec<String>,
}

/// 赏金关注命中后的通知条目（系统通知 + 程序内弹窗共用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BountyAlertDto {
    pub body: String,
    /// 赏金板过期时间（RFC3339）；弹窗到期自动关闭。
    pub expiry: Option<String>,
    /// 势力 tag（如 ZarimanSyndicate），前端点通知跳转声望用。
    pub tag: String,
}

/// 用户手动记录的各势力声望等级：`tag → rank`（0..=STANDING_RANK_MAX）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingRanksPrefs {
    #[serde(default)]
    pub ranks: std::collections::HashMap<String, i32>,
}

/// 声望「今日/本周已完成」勾选。
///
/// - `day_key`：UTC `YYYY-MM-DD`，与今日不同则日常勾选视为未完成
/// - `done`：有日常的势力 tag → 是否已完成
/// - `week_key`：UTC 当周周一日期；卡尔驻军按周重置
/// - `kahl_done`：本周卡尔是否已勾选完成
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingDailyPrefs {
    #[serde(default)]
    pub day_key: String,
    #[serde(default)]
    pub done: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub week_key: String,
    #[serde(default)]
    pub kahl_done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSortieMissionDto {
    pub sort_order: i32,
    pub mission_type: String,
    pub modifier: String,
    pub node: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSortieDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub boss: String,
    pub faction: String,
    pub missions: Vec<LiveSortieMissionDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveNightwaveChallengeDto {
    pub title: String,
    pub description: String,
    pub is_elite: bool,
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub reputation: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSteelIncursionMissionDto {
    pub mission_type: String,
    pub faction: String,
    pub min_level: i32,
    pub max_level: i32,
    pub node: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveSteelIncursionDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    #[serde(default)]
    pub today: Vec<LiveSteelIncursionMissionDto>,
    #[serde(default)]
    pub tomorrow: Vec<LiveSteelIncursionMissionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveArchonHuntDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub boss: String,
    #[serde(default)]
    pub shard: String,
    pub missions: Vec<LiveArchonMissionDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveArchonMissionDto {
    pub mission_type: String,
    pub node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveArchimedeaDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub kind_label: String,
    pub missions: Vec<LiveArchimedeaMissionDto>,
    pub personal_modifiers: Vec<LabeledTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveArchimedeaMissionDto {
    pub mission_type: String,
    pub faction: String,
    pub deviation: LabeledTerm,
    pub risks_normal: Vec<LabeledTerm>,
    pub risks_elite: Vec<LabeledTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveCircuitDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub this_week: Vec<String>,
    pub next_week: Vec<String>,
    #[serde(default)]
    pub rotation_weeks: Vec<Vec<String>>,
    #[serde(default)]
    pub current_week_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveNightwaveSeasonDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveDailyBrief {
    pub sortie: Option<LiveSortieDto>,
    pub nightwave_daily: Vec<LiveNightwaveChallengeDto>,
    pub nightwave_weekly: Vec<LiveNightwaveChallengeDto>,
    pub nightwave_season: Option<LiveNightwaveSeasonDto>,
    pub steel_incursion: Option<LiveSteelIncursionDto>,
    pub archon_hunt: Option<LiveArchonHuntDto>,
    pub deep_archimedea: Option<LiveArchimedeaDto>,
    pub temporal_archimedea: Option<LiveArchimedeaDto>,
    pub steel_circuit: Option<LiveCircuitDto>,
    pub normal_circuit: Option<LiveCircuitDto>,
    pub hex_calendar: Option<LiveHexCalendarDto>,
}
