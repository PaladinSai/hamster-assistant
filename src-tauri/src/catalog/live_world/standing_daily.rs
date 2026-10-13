//! 声望日常 / 卡尔周常完成勾选（存 user.sqlite，UTC 重置）。

use std::path::Path;

use chrono::{Datelike, Utc, Weekday};

use crate::db::{self, open_user};

use super::localize_bounty;
use super::types::{StandingDailyPrefs, SETTING_STANDING_DAILY, STANDING_WEEKLY_TAG};

pub fn get_standing_daily(data_dir: &Path) -> Result<StandingDailyPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    Ok(effective(read_prefs(&user)?))
}

pub fn set_standing_daily(
    data_dir: &Path,
    prefs: StandingDailyPrefs,
) -> Result<StandingDailyPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let clean = sanitize(prefs);
    write_prefs(&user, &clean)?;
    Ok(effective(clean))
}

fn utc_day_key() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

fn utc_week_monday_key() -> String {
    let today = Utc::now().date_naive();
    let monday = today - chrono::Duration::days(today.weekday().num_days_from_monday() as i64);
    debug_assert_eq!(monday.weekday(), Weekday::Mon);
    monday.format("%Y-%m-%d").to_string()
}

/// 按当前 UTC 日/周折叠过期勾选（读时生效，不强制回写）。
fn effective(mut prefs: StandingDailyPrefs) -> StandingDailyPrefs {
    let today = utc_day_key();
    let week = utc_week_monday_key();
    if prefs.day_key != today {
        prefs.day_key = today;
        prefs.done.clear();
    }
    if prefs.week_key != week {
        prefs.week_key = week;
        prefs.kahl_done = false;
    }
    prefs
}

fn sanitize(mut prefs: StandingDailyPrefs) -> StandingDailyPrefs {
    prefs.day_key = utc_day_key();
    prefs.week_key = utc_week_monday_key();
    prefs.done.retain(|tag, done| {
        if !*done {
            return false;
        }
        // 卡尔走周常字段；无日常势力也可记「今日是否已交」
        if tag == STANDING_WEEKLY_TAG {
            return false;
        }
        localize_bounty::is_tracked_syndicate(tag)
    });
    prefs
}

fn read_prefs(user: &rusqlite::Connection) -> Result<StandingDailyPrefs, String> {
    let Some(json) = db::get_setting(user, SETTING_STANDING_DAILY)? else {
        return Ok(StandingDailyPrefs::default());
    };
    serde_json::from_str(&json).map_err(|e| format!("解析声望完成勾选失败: {e}"))
}

fn write_prefs(user: &rusqlite::Connection, prefs: &StandingDailyPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db::set_setting(user, SETTING_STANDING_DAILY, &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_no_daily_drops_false_and_unknown() {
        let mut prefs = StandingDailyPrefs::default();
        prefs
            .done
            .insert("CetusSyndicate".into(), true);
        prefs
            .done
            .insert("QuillsSyndicate".into(), true);
        prefs.done.insert("HexSyndicate".into(), false);
        prefs.done.insert("FakeSyndicate".into(), true);
        prefs.kahl_done = true;
        let clean = sanitize(prefs);
        assert_eq!(clean.done.get("CetusSyndicate"), Some(&true));
        assert_eq!(clean.done.get("QuillsSyndicate"), Some(&true));
        assert!(!clean.done.contains_key("HexSyndicate"));
        assert!(!clean.done.contains_key("FakeSyndicate"));
        assert!(clean.kahl_done);
        assert_eq!(clean.day_key, utc_day_key());
        assert_eq!(clean.week_key, utc_week_monday_key());
    }

    #[test]
    fn effective_clears_stale_day() {
        let mut prefs = StandingDailyPrefs {
            day_key: "2000-01-01".into(),
            week_key: utc_week_monday_key(),
            kahl_done: true,
            ..Default::default()
        };
        prefs.done.insert("CetusSyndicate".into(), true);
        let eff = effective(prefs);
        assert!(eff.done.is_empty());
        assert_eq!(eff.day_key, utc_day_key());
        assert!(eff.kahl_done);
    }
}
