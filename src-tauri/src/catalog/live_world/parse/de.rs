//! worldState JSON 反序列化骨架。

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct DeWorldState {
    #[serde(rename = "Sorties", default)]
    pub(crate) sorties: Vec<DeSortie>,
    #[serde(rename = "SeasonInfo")]
    pub(crate) season_info: Option<DeSeasonInfo>,
    #[serde(rename = "LiteSorties", default)]
    pub(crate) lite_sorties: Vec<DeLiteSortie>,
    #[serde(rename = "Conquests", default)]
    pub(crate) conquests: Vec<DeConquest>,
    #[serde(rename = "EndlessXpSchedule", default)]
    pub(crate) endless_xp: Vec<DeEndlessXp>,
    #[serde(rename = "KnownCalendarSeasons", default)]
    pub(crate) calendars: Vec<DeCalendarSeason>,
    #[serde(rename = "SyndicateMissions", default)]
    pub(crate) syndicate_missions: Vec<DeSyndicateMission>,
    #[serde(rename = "ActiveMissions", default)]
    pub(crate) active_missions: Vec<DeActiveMission>,
    #[serde(rename = "VoidStorms", default)]
    pub(crate) void_storms: Vec<DeVoidStorm>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeActiveMission {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Node")]
    pub(crate) node: Option<String>,
    #[serde(rename = "MissionType")]
    pub(crate) mission_type: Option<String>,
    #[serde(rename = "Modifier")]
    pub(crate) modifier: Option<String>,
    #[serde(rename = "Hard", default)]
    pub(crate) hard: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeVoidStorm {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Node")]
    pub(crate) node: Option<String>,
    #[serde(rename = "ActiveMissionTier")]
    pub(crate) active_mission_tier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeSortie {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Boss")]
    pub(crate) boss: Option<String>,
    #[serde(rename = "Variants", default)]
    pub(crate) variants: Vec<DeVariant>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeVariant {
    #[serde(rename = "missionType")]
    pub(crate) mission_type: Option<String>,
    #[serde(rename = "modifierType")]
    pub(crate) modifier_type: Option<String>,
    #[serde(rename = "node")]
    pub(crate) node: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeSeasonInfo {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "ActiveChallenges", default)]
    pub(crate) active_challenges: Vec<DeChallenge>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeChallenge {
    #[serde(rename = "_id")]
    pub(crate) id: Option<DeOid>,
    #[serde(rename = "Daily")]
    pub(crate) daily: Option<bool>,
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Challenge")]
    pub(crate) challenge: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeLiteSortie {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Boss")]
    pub(crate) boss: Option<String>,
    #[serde(rename = "Missions", default)]
    pub(crate) missions: Vec<DeLiteMission>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeLiteMission {
    #[serde(rename = "missionType")]
    pub(crate) mission_type: Option<String>,
    #[serde(rename = "node")]
    pub(crate) node: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeConquest {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Type")]
    pub(crate) kind: Option<String>,
    #[serde(rename = "Missions", default)]
    pub(crate) missions: Vec<DeConquestMission>,
    #[serde(rename = "Variables", default)]
    pub(crate) variables: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeConquestMission {
    #[serde(rename = "faction")]
    pub(crate) faction: Option<String>,
    #[serde(rename = "missionType")]
    pub(crate) mission_type: Option<String>,
    #[serde(rename = "difficulties", default)]
    pub(crate) difficulties: Vec<DeDifficulty>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeDifficulty {
    #[serde(rename = "type")]
    pub(crate) kind: Option<String>,
    #[serde(rename = "deviation")]
    pub(crate) deviation: Option<String>,
    #[serde(rename = "risks", default)]
    pub(crate) risks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeEndlessXp {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "CategoryChoices", default)]
    pub(crate) categories: Vec<DeCategoryChoice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeCategoryChoice {
    #[serde(rename = "Category")]
    pub(crate) category: Option<String>,
    #[serde(rename = "Choices", default)]
    pub(crate) choices: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeCalendarSeason {
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Days", default)]
    pub(crate) days: Vec<DeCalendarDay>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeCalendarDay {
    pub(crate) day: Option<i32>,
    #[serde(default)]
    pub(crate) events: Vec<DeCalendarEvent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeCalendarEvent {
    #[serde(rename = "type")]
    pub(crate) kind: Option<String>,
    pub(crate) challenge: Option<String>,
    pub(crate) reward: Option<String>,
    pub(crate) upgrade: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeSyndicateMission {
    #[serde(rename = "Tag")]
    pub(crate) tag: Option<String>,
    #[serde(rename = "Activation")]
    pub(crate) activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    pub(crate) expiry: Option<DeDate>,
    #[serde(rename = "Jobs", default)]
    pub(crate) jobs: Vec<DeSyndicateJob>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeSyndicateJob {
    #[serde(rename = "jobType")]
    pub(crate) job_type: Option<String>,
    #[serde(rename = "minEnemyLevel")]
    pub(crate) min_enemy_level: Option<i32>,
    #[serde(rename = "maxEnemyLevel")]
    pub(crate) max_enemy_level: Option<i32>,
    #[serde(rename = "xpAmounts", default)]
    pub(crate) xp_amounts: Vec<i32>,
    #[serde(rename = "isVault")]
    pub(crate) is_vault: Option<bool>,
    /// 掉落表路径，如 `.../TierATableBRewards`（第二字母为当前轮换）。
    #[serde(rename = "rewards")]
    pub(crate) rewards: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeOid {
    #[serde(rename = "$oid")]
    pub(crate) oid: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeDate {
    #[serde(rename = "$date")]
    pub(crate) date: DeDateInner,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeDateInner {
    #[serde(rename = "$numberLong")]
    pub(crate) number_long: String,
}


pub(crate) fn de_date_to_rfc3339(date: &DeDate) -> Option<String> {
    use chrono::{TimeZone, Utc};
    let ms: i64 = date.date.number_long.parse().ok()?;
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.to_rfc3339())
}

pub(crate) fn millis_to_rfc3339(ms: i64) -> Option<String> {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.to_rfc3339())
}
