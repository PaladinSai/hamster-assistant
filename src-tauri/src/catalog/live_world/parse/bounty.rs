//! 赏金板 parse。

use serde::Deserialize;

use super::de::{de_date_to_rfc3339, millis_to_rfc3339, DeSyndicateMission, DeWorldState};
use crate::catalog::live_world::localize_bounty;
use crate::catalog::live_world::types::{BountyBoardApi, BountyBriefApi, BountyJobApi};

#[derive(Debug, Deserialize)]
pub(crate) struct DeBountyCycle {
    pub(crate) expiry: Option<i64>,
    /// 当前 A/B/C 轮换（如 `"B"`）；cycle 补齐任务写入 `reward_table` 供掉落匹配。
    #[serde(default)]
    pub(crate) rot: Option<String>,
    #[serde(default)]
    pub(crate) bounties: std::collections::HashMap<String, Vec<DeCycleJob>>,
}

/// 把 bounty-cycle `rot` 编成可被 `parse_reward_rotation` 识别的伪路径。
fn cycle_reward_table(rot: Option<&str>) -> String {
    let Some(raw) = rot.map(str::trim).filter(|s| !s.is_empty()) else {
        return String::new();
    };
    let ch = raw.chars().next().unwrap_or('\0').to_ascii_uppercase();
    if matches!(ch, 'A' | 'B' | 'C') {
        format!("TierATable{ch}Rewards")
    } else {
        String::new()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeCycleJob {
    pub(crate) node: Option<String>,
    pub(crate) challenge: Option<String>,
    #[allow(dead_code)]
    pub(crate) ally: Option<String>,
}

pub(super) fn parse_bounties(
    state: &DeWorldState,
    cycle: Option<&DeBountyCycle>,
    challenge_i18n: Option<&crate::catalog::live_world::challenge_i18n::ChallengeLocalizer>,
) -> BountyBriefApi {
    let mut by_tag: std::collections::HashMap<String, &DeSyndicateMission> =
        std::collections::HashMap::new();
    for m in &state.syndicate_missions {
        let Some(tag) = m.tag.as_deref() else {
            continue;
        };
        if !localize_bounty::is_tracked_syndicate(tag) {
            continue;
        }
        by_tag.insert(tag.to_string(), m);
    }

    let cycle_expiry = cycle.and_then(|c| c.expiry).and_then(millis_to_rfc3339);

    let mut boards = Vec::with_capacity(localize_bounty::BOUNTY_SYNDICATES.len());
    for (tag, _) in localize_bounty::BOUNTY_SYNDICATES {
        let name = localize_bounty::syndicate_display_name(tag);
        let raw = by_tag.get(*tag);

        let mut jobs: Vec<BountyJobApi> = Vec::new();
        if let Some(raw) = raw {
            for j in &raw.jobs {
                if j.is_vault == Some(true) {
                    continue;
                }
                let job_type = j.job_type.as_deref().unwrap_or("").to_string();
                if job_type.is_empty() {
                    continue;
                }
                let min_lv = j.min_enemy_level.unwrap_or(0);
                let max_lv = j.max_enemy_level.unwrap_or(0);
                let standing_total: i32 = j.xp_amounts.iter().sum();
                let kind = localize_bounty::job_kind_key(&job_type);
                let kind_zh = localize_bounty::job_kind_zh(&kind);
                let title = localize_bounty::job_title_zh(&job_type, &kind, min_lv, max_lv);
                jobs.push(BountyJobApi {
                    job_type,
                    kind,
                    kind_zh,
                    title,
                    min_enemy_level: min_lv,
                    max_enemy_level: max_lv,
                    standing_total,
                    is_advanced: false,
                    is_steel_path: min_lv >= 100 && max_lv >= 100,
                    challenge_zh: String::new(),
                    rewards: vec![],
                    reward_table: j.rewards.clone().unwrap_or_default(),
                });
            }
        }

        // worldState Jobs 空时，用 bounty-cycle 补齐三家
        let mut filled_from_cycle = false;
        if jobs.is_empty() && localize_bounty::is_cycle_filled_syndicate(tag) {
            if let Some(cycle) = cycle {
                if let Some(entries) = cycle.bounties.get(*tag) {
                    let reward_table = cycle_reward_table(cycle.rot.as_deref());
                    for (idx, entry) in entries.iter().enumerate() {
                        let challenge = entry.challenge.as_deref().unwrap_or("").to_string();
                        let node = entry.node.as_deref().unwrap_or("");
                        if challenge.is_empty() {
                            continue;
                        }
                        let (min_lv, max_lv, standing) =
                            localize_bounty::cycle_tier_stats(tag, idx).unwrap_or((0, 0, 0));
                        let (kind, kind_zh) = localize_bounty::cycle_node_mission_kind(node);
                        let title = localize_bounty::cycle_node_title_zh(node);
                        let challenge_zh =
                            localize_bounty::cycle_challenge_zh(&challenge, challenge_i18n);
                        jobs.push(BountyJobApi {
                            job_type: challenge,
                            kind,
                            kind_zh,
                            title,
                            min_enemy_level: min_lv,
                            max_enemy_level: max_lv,
                            standing_total: standing,
                            is_advanced: false,
                            is_steel_path: false,
                            challenge_zh,
                            rewards: vec![],
                            reward_table: reward_table.clone(),
                        });
                    }
                    filled_from_cycle = !jobs.is_empty();
                }
            }
        }

        if let Some(max_standing) = jobs.iter().map(|j| j.standing_total).max() {
            for j in &mut jobs {
                if j.standing_total == max_standing && max_standing > 0 {
                    j.is_advanced = true;
                }
            }
        }
        jobs.sort_by(|a, b| {
            b.standing_total
                .cmp(&a.standing_total)
                .then(a.min_enemy_level.cmp(&b.min_enemy_level))
        });

        let activation = raw.and_then(|r| r.activation.as_ref().and_then(de_date_to_rfc3339));
        let expiry = if filled_from_cycle {
            cycle_expiry.clone()
        } else {
            raw.and_then(|r| r.expiry.as_ref().and_then(de_date_to_rfc3339))
        };

        boards.push(BountyBoardApi {
            tag: (*tag).to_string(),
            name,
            activation,
            expiry,
            jobs,
        });
    }

    BountyBriefApi { boards }
}

