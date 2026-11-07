//! 声望关注：按势力勾任务、掉落、挑战，任意一项命中就提醒。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Deserialize;

use crate::db::{self, open_catalog, open_user};

use super::bounty_rewards;
use super::challenge_i18n::ChallengeLocalizer;
use super::localize_bounty::{self, cycle_challenge_zh};
use super::summary::read_bounty_brief;
use super::types::{
    BountyAlertDto, BountyBriefApi, BountyJobApi, BountyWatchPrefs, BountyWatchView,
    SyndicateWatch, SyndicateWatchCatalog, WatchChoice, SETTING_BOUNTY_WATCH,
};

pub fn get_bounty_watch(data_dir: &Path) -> Result<BountyWatchView, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let prefs = read_watch(&user)?;
    let catalog_conn = open_catalog(&db::catalog_path(data_dir))?;
    let marks = super::reward_inventory::reward_mark_index(&catalog_conn, &user)?;
    let catalog = build_catalog(data_dir, &marks);
    Ok(BountyWatchView { prefs, catalog })
}

pub fn set_bounty_watch(data_dir: &Path, prefs: BountyWatchPrefs) -> Result<BountyWatchPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let clean = sanitize(prefs);
    write_watch(&user, &clean)?;
    Ok(clean)
}

fn read_watch(user: &rusqlite::Connection) -> Result<BountyWatchPrefs, String> {
    let Some(json) = db::get_setting(user, SETTING_BOUNTY_WATCH)? else {
        return Ok(BountyWatchPrefs::default());
    };
    let (prefs, migrated) = parse_watch_json(&json)?;
    if migrated {
        write_watch(user, &prefs)?;
    }
    Ok(prefs)
}

fn write_watch(user: &rusqlite::Connection, prefs: &BountyWatchPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db::set_setting(user, SETTING_BOUNTY_WATCH, &json)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WatchFile {
    watches: Option<Vec<SyndicateWatch>>,
    #[serde(default)]
    followed_syndicates: Vec<String>,
    #[serde(default)]
    followed_job_kinds: Vec<String>,
    #[serde(default)]
    last_notified: Vec<String>,
}

/// 新格式直接用；旧的「全局种类」只把轮换三家里对得上的种类挂成任务。
fn parse_watch_json(json: &str) -> Result<(BountyWatchPrefs, bool), String> {
    let file: WatchFile = serde_json::from_str(json).map_err(|e| format!("解析赏金关注失败: {e}"))?;
    if let Some(watches) = file.watches {
        return Ok((
            sanitize(BountyWatchPrefs {
                watches,
                last_notified: file.last_notified,
            }),
            false,
        ));
    }
    Ok((
        migrate_legacy(
            &file.followed_syndicates,
            &file.followed_job_kinds,
            file.last_notified,
        ),
        true,
    ))
}

fn migrate_legacy(
    syndicates: &[String],
    kinds: &[String],
    last_notified: Vec<String>,
) -> BountyWatchPrefs {
    let mut watches = Vec::new();
    for tag in syndicates {
        if !localize_bounty::is_cycle_watch_syndicate(tag) {
            continue;
        }
        let mut jobs = Vec::new();
        for kind in kinds {
            for node in localize_bounty::cycle_watch_nodes(tag) {
                if localize_bounty::cycle_node_mission_kind(node).0 == *kind {
                    jobs.push((*node).to_string());
                }
            }
        }
        jobs.sort();
        jobs.dedup();
        if jobs.is_empty() {
            continue;
        }
        watches.push(SyndicateWatch {
            tag: tag.clone(),
            jobs,
            rewards: Vec::new(),
            challenges: Vec::new(),
        });
    }
    sanitize(BountyWatchPrefs {
        watches,
        last_notified,
    })
}

fn sanitize(mut prefs: BountyWatchPrefs) -> BountyWatchPrefs {
    let mut merged: Vec<SyndicateWatch> = Vec::new();
    for watch in prefs.watches.drain(..) {
        if !localize_bounty::is_bounty_watch_syndicate(&watch.tag) {
            continue;
        }
        let mut jobs = tidy(watch.jobs);
        let rewards = tidy(watch.rewards);
        let mut challenges = tidy(watch.challenges);
        if !localize_bounty::is_cycle_watch_syndicate(&watch.tag) {
            challenges.clear();
        }
        if jobs.is_empty() && rewards.is_empty() && challenges.is_empty() {
            continue;
        }
        if let Some(existing) = merged.iter_mut().find(|w| w.tag == watch.tag) {
            existing.jobs.append(&mut jobs);
            existing.rewards.extend(rewards);
            existing.challenges.append(&mut challenges);
            existing.jobs = tidy(std::mem::take(&mut existing.jobs));
            existing.rewards = tidy(std::mem::take(&mut existing.rewards));
            existing.challenges = tidy(std::mem::take(&mut existing.challenges));
        } else {
            merged.push(SyndicateWatch {
                tag: watch.tag,
                jobs,
                rewards,
                challenges,
            });
        }
    }
    merged.sort_by(|a, b| syndicate_ord(&a.tag).cmp(&syndicate_ord(&b.tag)));
    prefs.watches = merged;
    prefs.last_notified = tidy(prefs.last_notified);
    prefs
}

fn tidy(mut items: Vec<String>) -> Vec<String> {
    for item in &mut items {
        *item = item.trim().to_string();
    }
    items.retain(|s| !s.is_empty());
    items.sort();
    items.dedup();
    items
}

fn syndicate_ord(tag: &str) -> usize {
    localize_bounty::WATCH_SYNDICATE_TAGS
        .iter()
        .position(|t| *t == tag)
        .unwrap_or(usize::MAX)
}

fn build_catalog(
    data_dir: &Path,
    marks: &HashMap<String, String>,
) -> Vec<SyndicateWatchCatalog> {
    let zh = open_catalog(&db::catalog_path(data_dir))
        .ok()
        .and_then(|conn| crate::catalog::i18n::ZhLookup::load(&conn).ok());
    let localizer = ChallengeLocalizer::load_from_cache(data_dir)
        .ok()
        .flatten();
    let brief = open_catalog(&db::catalog_path(data_dir))
        .ok()
        .and_then(|conn| read_bounty_brief(&conn).ok());

    localize_bounty::WATCH_SYNDICATE_TAGS
        .iter()
        .map(|tag| SyndicateWatchCatalog {
            tag: (*tag).to_string(),
            name: localize_bounty::syndicate_display_name(tag),
            jobs: job_choices(tag),
            rewards: reward_choices(data_dir, tag, zh.as_ref(), marks),
            challenges: challenge_choices(tag, localizer.as_ref(), brief.as_ref()),
        })
        .collect()
}

fn job_choices(tag: &str) -> Vec<WatchChoice> {
    let mut choices = Vec::new();
    if localize_bounty::is_cycle_watch_syndicate(tag) {
        for node in localize_bounty::cycle_watch_nodes(tag) {
            choices.push(WatchChoice {
                id: (*node).to_string(),
                label: localize_bounty::cycle_node_title_zh(node),
                mark: None,
            });
        }
    } else {
        for base in localize_bounty::plains_job_pool(tag) {
            let kind = localize_bounty::job_kind_key(base);
            choices.push(WatchChoice {
                id: (*base).to_string(),
                label: localize_bounty::job_title_zh(base, &kind, 0, 0),
                mark: None,
            });
        }
    }
    choices.sort_by(|a, b| a.label.cmp(&b.label));
    choices
}

fn reward_choices(
    data_dir: &Path,
    tag: &str,
    zh: Option<&crate::catalog::i18n::ZhLookup>,
    marks: &HashMap<String, String>,
) -> Vec<WatchChoice> {
    bounty_rewards::list_watch_reward_labels(data_dir, tag, zh)
        .into_iter()
        .map(|label| {
            let mark = marks.get(&label).cloned();
            WatchChoice {
                id: label.clone(),
                label,
                mark,
            }
        })
        .collect()
}

fn challenge_choices(
    tag: &str,
    localizer: Option<&ChallengeLocalizer>,
    brief: Option<&BountyBriefApi>,
) -> Vec<WatchChoice> {
    let Some(prefix) = localize_bounty::cycle_challenge_prefix(tag) else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut choices = Vec::new();
    if let Some(loc) = localizer {
        for path in loc.paths_with_prefix(prefix) {
            let label = cycle_challenge_zh(&path, Some(loc));
            push_challenge(&mut choices, &mut seen, label);
        }
    }
    if choices.is_empty() {
        if let Some(brief) = brief {
            if let Some(board) = brief.boards.iter().find(|b| b.tag == tag) {
                for job in &board.jobs {
                    push_challenge(&mut choices, &mut seen, job.challenge_zh.clone());
                }
            }
        }
    }
    choices.sort_by(|a, b| a.label.cmp(&b.label));
    choices
}

fn push_challenge(choices: &mut Vec<WatchChoice>, seen: &mut HashSet<String>, label: String) {
    let label = label.trim().to_string();
    if label.is_empty() || !seen.insert(label.clone()) {
        return;
    }
    choices.push(WatchChoice {
        id: label.clone(),
        label,
        mark: None,
    });
}

/// 匹配应通知的赏金；同一势力同一轮换只记一条。更新 last_notified。
pub fn match_and_record_notifications(data_dir: &Path) -> Result<Vec<BountyAlertDto>, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    let user = open_user(&db::user_path(data_dir))?;
    let brief = read_bounty_brief(&catalog)?;
    let mut prefs = read_watch(&user)?;
    let alerts = collect_new_alerts(&brief, &mut prefs);
    if !alerts.is_empty() {
        write_watch(&user, &prefs)?;
    }
    Ok(alerts)
}

pub(crate) fn collect_new_alerts(
    brief: &BountyBriefApi,
    prefs: &mut BountyWatchPrefs,
) -> Vec<BountyAlertDto> {
    let mut alerts = Vec::new();
    let mut new_fps = Vec::new();

    for watch in &prefs.watches {
        let Some(board) = brief.boards.iter().find(|b| b.tag == watch.tag) else {
            continue;
        };
        let mut job_lines = Vec::new();
        let mut reward_lines = Vec::new();
        let mut challenge_lines = Vec::new();
        let mut seen_jobs = HashSet::new();
        let mut seen_rewards = HashSet::new();
        let mut seen_challenges = HashSet::new();

        for job in &board.jobs {
            if job.is_advanced {
                for id in &watch.jobs {
                    if seen_jobs.contains(id) {
                        continue;
                    }
                    if !localize_bounty::watch_job_matches(
                        &board.tag,
                        &job.job_type,
                        &job.title,
                        id,
                    ) {
                        continue;
                    }
                    seen_jobs.insert(id.clone());
                    job_lines.push(job_line(job));
                }
            }
            for reward in &watch.rewards {
                if !job.rewards.iter().any(|name| name == reward) {
                    continue;
                }
                let key = format!("{reward}|{}", job.title);
                if !seen_rewards.insert(key) {
                    continue;
                }
                reward_lines.push(format!("{reward} · {}", job.title));
            }
            for challenge in &watch.challenges {
                if job.challenge_zh.trim() != challenge {
                    continue;
                }
                if seen_challenges.insert(challenge.clone()) {
                    challenge_lines.push(challenge.clone());
                }
            }
        }

        if job_lines.is_empty() && reward_lines.is_empty() && challenge_lines.is_empty() {
            continue;
        }
        let fp = format!("{}|{}", board.tag, board.expiry.as_deref().unwrap_or(""));
        if prefs.last_notified.iter().any(|x| x == &fp) {
            continue;
        }
        new_fps.push(fp);
        alerts.push(BountyAlertDto {
            body: alert_body(&board.name, &job_lines, &reward_lines, &challenge_lines),
            expiry: board.expiry.clone(),
            tag: board.tag.clone(),
        });
    }

    for fp in new_fps {
        prefs.last_notified.push(fp);
    }
    if prefs.last_notified.len() > 64 {
        let drain = prefs.last_notified.len() - 64;
        prefs.last_notified.drain(0..drain);
    }
    alerts
}

fn job_line(job: &BountyJobApi) -> String {
    let challenge = job.challenge_zh.trim();
    let title = if challenge.is_empty() {
        job.title.clone()
    } else {
        format!("{} · {challenge}", job.title)
    };
    if job.standing_total > 0 {
        format!("{title}（声望 {}）", job.standing_total)
    } else {
        title
    }
}

fn alert_body(name: &str, jobs: &[String], rewards: &[String], challenges: &[String]) -> String {
    let mut body = format!("{name} 赏金命中");
    if !jobs.is_empty() {
        body.push_str("\n任务：");
        body.push_str(&jobs.join("、"));
    }
    if !rewards.is_empty() {
        body.push_str("\n掉落：");
        body.push_str(&rewards.join("、"));
    }
    if !challenges.is_empty() {
        body.push_str("\n挑战：");
        body.push_str(&challenges.join("、"));
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::live_world::types::{BountyBoardApi, BountyJobApi};

    fn job(
        job_type: &str,
        title: &str,
        standing: i32,
        advanced: bool,
        challenge: &str,
        rewards: &[&str],
    ) -> BountyJobApi {
        BountyJobApi {
            job_type: job_type.into(),
            kind: String::new(),
            kind_zh: String::new(),
            title: title.into(),
            min_enemy_level: 0,
            max_enemy_level: 0,
            standing_total: standing,
            is_advanced: advanced,
            is_steel_path: false,
            challenge_zh: challenge.into(),
            rewards: rewards.iter().map(|s| (*s).to_string()).collect(),
            reward_table: String::new(),
        }
    }

    fn cetus_brief() -> BountyBriefApi {
        BountyBriefApi {
            boards: vec![BountyBoardApi {
                tag: "CetusSyndicate".into(),
                name: "Ostron".into(),
                activation: None,
                expiry: Some("2026-10-01T00:00:00Z".into()),
                jobs: vec![
                    job(
                        "/Lotus/Types/Jobs/AttritionBountyExt",
                        "宰杀敌人",
                        5020,
                        true,
                        "",
                        &["Gara 蓝图"],
                    ),
                    job(
                        "/Lotus/Types/Jobs/CaptureBountyCapOne",
                        "捕获 Grineer 指挥官",
                        1290,
                        false,
                        "",
                        &["内融核心"],
                    ),
                ],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn job_alerts_only_on_top_tier() {
        let brief = cetus_brief();
        let mut low = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "CetusSyndicate".into(),
                jobs: vec!["CaptureBountyCapOne".into()],
                rewards: vec![],
                challenges: vec![],
            }],
            last_notified: vec![],
        };
        assert!(collect_new_alerts(&brief, &mut low).is_empty());

        let mut top = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "CetusSyndicate".into(),
                jobs: vec!["AttritionBountyExt".into()],
                rewards: vec![],
                challenges: vec![],
            }],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut top);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("宰杀敌人"));
        assert!(collect_new_alerts(&brief, &mut top).is_empty());
    }

    #[test]
    fn low_tier_reward_still_alerts() {
        let brief = cetus_brief();
        let mut prefs = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "CetusSyndicate".into(),
                jobs: vec![],
                rewards: vec!["内融核心".into()],
                challenges: vec![],
            }],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("掉落：内融核心"));
        assert!(!msgs[0].body.contains("任务："));
    }

    #[test]
    fn low_tier_challenge_still_alerts() {
        let mut brief = cetus_brief();
        brief.boards[0].jobs[1].challenge_zh = "找到一块耀金奖章并交给梅利卡".into();
        let mut prefs = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "CetusSyndicate".into(),
                jobs: vec![],
                rewards: vec![],
                challenges: vec!["找到一块耀金奖章并交给梅利卡".into()],
            }],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("挑战：找到一块耀金奖章并交给梅利卡"));
    }

    #[test]
    fn reward_and_job_share_one_alert() {
        let brief = cetus_brief();
        let mut prefs = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "CetusSyndicate".into(),
                jobs: vec!["AttritionBountyExt".into()],
                rewards: vec!["Gara 蓝图".into()],
                challenges: vec![],
            }],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("任务："));
        assert!(msgs[0].body.contains("掉落：Gara 蓝图"));
    }

    #[test]
    fn challenge_hit_on_cycle_board() {
        let brief = BountyBriefApi {
            boards: vec![BountyBoardApi {
                tag: "ZarimanSyndicate".into(),
                name: "坚守者".into(),
                activation: None,
                expiry: Some("2026-10-02T00:00:00Z".into()),
                jobs: vec![job(
                    "/Lotus/Types/Challenges/Zariman/ZarimanFindMelicaCacheChallenge",
                    "歼灭 · 哈拉科防线",
                    5000,
                    true,
                    "找到一块耀金奖章并交给梅利卡",
                    &[],
                )],
            }],
            ..Default::default()
        };
        let mut prefs = BountyWatchPrefs {
            watches: vec![SyndicateWatch {
                tag: "ZarimanSyndicate".into(),
                jobs: vec![],
                rewards: vec![],
                challenges: vec!["找到一块耀金奖章并交给梅利卡".into()],
            }],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("挑战：找到一块耀金奖章并交给梅利卡"));
        assert!(!msgs[0].body.contains("任务："));
    }

    #[test]
    fn legacy_kind_maps_only_onto_cycle_nodes() {
        let json = r#"{
            "followedSyndicates": ["CetusSyndicate", "ZarimanSyndicate"],
            "followedJobKinds": ["Exterminate", "Capture"],
            "lastNotified": ["keep"]
        }"#;
        let (prefs, migrated) = parse_watch_json(json).unwrap();
        assert!(migrated);
        assert_eq!(prefs.watches.len(), 1);
        assert_eq!(prefs.watches[0].tag, "ZarimanSyndicate");
        assert_eq!(prefs.watches[0].jobs, vec!["SolNode231".to_string()]);
        assert_eq!(prefs.last_notified, vec!["keep".to_string()]);
    }

    #[test]
    fn empty_follow_does_not_alert() {
        let brief = cetus_brief();
        let mut prefs = BountyWatchPrefs::default();
        assert!(collect_new_alerts(&brief, &mut prefs).is_empty());
    }
}
