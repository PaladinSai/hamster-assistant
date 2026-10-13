//! 声望关注偏好与高级档通知匹配。

use std::path::Path;

use crate::db::{self, open_catalog, open_user};

use super::localize_bounty;
use super::summary::read_bounty_brief;
use super::types::{BountyAlertDto, BountyBriefApi, BountyWatchPrefs, SETTING_BOUNTY_WATCH};

pub fn get_bounty_watch(data_dir: &Path) -> Result<BountyWatchPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    read_watch(&user)
}

pub fn set_bounty_watch(data_dir: &Path, prefs: BountyWatchPrefs) -> Result<BountyWatchPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let mut clean = prefs;
    clean.followed_syndicates.retain(|t| localize_bounty::is_tracked_syndicate(t));
    clean.followed_job_kinds.retain(|k| !k.trim().is_empty());
    clean.followed_syndicates.sort();
    clean.followed_syndicates.dedup();
    clean.followed_job_kinds.sort();
    clean.followed_job_kinds.dedup();
    write_watch(&user, &clean)?;
    Ok(clean)
}

fn read_watch(user: &rusqlite::Connection) -> Result<BountyWatchPrefs, String> {
    let Some(json) = db::get_setting(user, SETTING_BOUNTY_WATCH)? else {
        return Ok(BountyWatchPrefs::default());
    };
    serde_json::from_str(&json).map_err(|e| format!("解析赏金关注失败: {e}"))
}

fn write_watch(user: &rusqlite::Connection, prefs: &BountyWatchPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db::set_setting(user, SETTING_BOUNTY_WATCH, &json)
}

/// 匹配应通知的高级档；更新 last_notified；返回通知条目列表。
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
    if prefs.followed_syndicates.is_empty() || prefs.followed_job_kinds.is_empty() {
        return Vec::new();
    }
    let followed_tags: std::collections::HashSet<&str> =
        prefs.followed_syndicates.iter().map(|s| s.as_str()).collect();
    let followed_kinds: std::collections::HashSet<&str> =
        prefs.followed_job_kinds.iter().map(|s| s.as_str()).collect();

    let mut alerts = Vec::new();
    let mut new_fps = Vec::new();

    for board in &brief.boards {
        if !followed_tags.contains(board.tag.as_str()) {
            continue;
        }
        for job in board.jobs.iter().filter(|j| j.is_advanced) {
            if !followed_kinds.contains(job.kind.as_str()) {
                continue;
            }
            let fp = fingerprint(
                &board.tag,
                board.expiry.as_deref(),
                &job.job_type,
                job.standing_total,
            );
            if prefs.last_notified.iter().any(|x| x == &fp) {
                continue;
            }
            new_fps.push(fp);
            let mut body = format!(
                "{} 高级赏金：{}（声望 {}）",
                board.name, job.title, job.standing_total
            );
            let challenge = job.challenge_zh.trim();
            if !challenge.is_empty() {
                body.push_str("\n挑战：");
                body.push_str(challenge);
            }
            alerts.push(BountyAlertDto {
                body,
                expiry: board.expiry.clone(),
                tag: board.tag.clone(),
            });
        }
    }

    for fp in new_fps {
        prefs.last_notified.push(fp);
    }
    // 控制体积：只保留最近 64 条指纹
    if prefs.last_notified.len() > 64 {
        let drain = prefs.last_notified.len() - 64;
        prefs.last_notified.drain(0..drain);
    }
    alerts
}

fn fingerprint(tag: &str, expiry: Option<&str>, job_type: &str, standing: i32) -> String {
    format!(
        "{}|{}|{}|{}",
        tag,
        expiry.unwrap_or(""),
        job_type,
        standing
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::live_world::types::BountyJobApi;

    fn sample_brief() -> BountyBriefApi {
        BountyBriefApi {
            boards: vec![crate::catalog::live_world::types::BountyBoardApi {
                tag: "CetusSyndicate".into(),
                name: "Ostron".into(),
                activation: None,
                expiry: Some("2026-10-01T00:00:00Z".into()),
                jobs: vec![
                    BountyJobApi {
                        job_type: "/Lotus/Types/Jobs/AttritionBountyExt".into(),
                        kind: "Attrition".into(),
                        kind_zh: "消耗".into(),
                        title: "钢路·消耗".into(),
                        min_enemy_level: 100,
                        max_enemy_level: 100,
                        standing_total: 5020,
                        is_advanced: true,
                        is_steel_path: true,
                        challenge_zh: "在 6 分钟内完成任务".into(),
                        rewards: vec![],
                        reward_table: String::new(),
                    },
                    BountyJobApi {
                        job_type: "/Lotus/Types/Jobs/CaptureBountyCapOne".into(),
                        kind: "Capture".into(),
                        kind_zh: "捕获".into(),
                        title: "捕获".into(),
                        min_enemy_level: 5,
                        max_enemy_level: 15,
                        standing_total: 1290,
                        is_advanced: false,
                        is_steel_path: false,
                        challenge_zh: String::new(),
                        rewards: vec![],
                        reward_table: String::new(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn alerts_when_followed_advanced_kind() {
        let brief = sample_brief();
        let mut prefs = BountyWatchPrefs {
            followed_syndicates: vec!["CetusSyndicate".into()],
            followed_job_kinds: vec!["Attrition".into()],
            last_notified: vec![],
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].body.contains("Ostron"));
        assert!(msgs[0].body.contains("挑战：在 6 分钟内完成任务"));
        assert_eq!(msgs[0].expiry.as_deref(), Some("2026-10-01T00:00:00Z"));
        assert_eq!(prefs.last_notified.len(), 1);
        // 同指纹不重复
        let msgs2 = collect_new_alerts(&brief, &mut prefs);
        assert!(msgs2.is_empty());
    }

    #[test]
    fn no_alert_when_kind_not_followed() {
        let brief = sample_brief();
        let mut prefs = BountyWatchPrefs {
            followed_syndicates: vec!["CetusSyndicate".into()],
            followed_job_kinds: vec!["Capture".into()],
            last_notified: vec![],
        };
        assert!(collect_new_alerts(&brief, &mut prefs).is_empty());
    }
}
