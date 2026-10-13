//! 用户声望等级（手动记录，存 user.sqlite）。

use std::path::Path;

use crate::db::{self, open_user};

use super::localize_bounty;
use super::types::{
    standing_rank_max_for, StandingRanksPrefs, SETTING_STANDING_RANKS,
};

pub fn get_standing_ranks(data_dir: &Path) -> Result<StandingRanksPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    Ok(sanitize(read_ranks(&user)?))
}

pub fn set_standing_ranks(
    data_dir: &Path,
    prefs: StandingRanksPrefs,
) -> Result<StandingRanksPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let clean = sanitize(prefs);
    write_ranks(&user, &clean)?;
    Ok(clean)
}

fn sanitize(mut prefs: StandingRanksPrefs) -> StandingRanksPrefs {
    prefs.ranks.retain(|tag, rank| {
        if !localize_bounty::is_tracked_syndicate(tag) {
            return false;
        }
        *rank = (*rank).clamp(0, standing_rank_max_for(tag));
        true
    });
    prefs
}

fn read_ranks(user: &rusqlite::Connection) -> Result<StandingRanksPrefs, String> {
    let Some(json) = db::get_setting(user, SETTING_STANDING_RANKS)? else {
        return Ok(StandingRanksPrefs::default());
    };
    serde_json::from_str(&json).map_err(|e| format!("解析声望等级失败: {e}"))
}

fn write_ranks(user: &rusqlite::Connection, prefs: &StandingRanksPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db::set_setting(user, SETTING_STANDING_RANKS, &json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_clamps_and_filters() {
        let mut prefs = StandingRanksPrefs::default();
        prefs.ranks.insert("CetusSyndicate".into(), 9);
        prefs.ranks.insert("NecraloidSyndicate".into(), 9);
        prefs.ranks.insert("FakeSyndicate".into(), 2);
        prefs.ranks.insert("HexSyndicate".into(), -1);
        let clean = sanitize(prefs);
        assert_eq!(clean.ranks.get("CetusSyndicate"), Some(&5));
        assert_eq!(clean.ranks.get("NecraloidSyndicate"), Some(&3));
        assert_eq!(clean.ranks.get("HexSyndicate"), Some(&0));
        assert!(!clean.ranks.contains_key("FakeSyndicate"));
    }
}
