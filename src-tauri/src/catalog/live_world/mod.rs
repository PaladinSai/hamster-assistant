//! 世界状态日常：突击 / 午夜电波 / 钢铁侵袭计时 / 赏金。

mod apply;
mod arbitration;
mod bounty_rewards;
mod bounty_watch;
mod challenge_i18n;
mod conquest_i18n;
mod fetch;
mod localize;
mod localize_bounty;
mod localize_weekly;
mod parse;
mod plains;
mod shop;
mod standing_daily;
mod standing_ranks;
mod steel_incursion;
mod store_bounty;
mod store_weekly;
mod summary;
mod types;

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension};

use crate::db::{self, open_catalog};
use crate::id::Snowflake;

pub use arbitration::{get_arbitration_brief, ArbitrationBriefDto};
pub use bounty_watch::{get_bounty_watch, match_and_record_notifications, set_bounty_watch};
pub use plains::{get_plains_brief, PlainsBriefDto};
pub use shop::{get_weekly_shop_brief, WeeklyShopBrief};
pub use standing_daily::{get_standing_daily, set_standing_daily};
pub use standing_ranks::{get_standing_ranks, set_standing_ranks};
pub use summary::{get_bounty_brief, get_fissure_brief, get_live_daily_brief};
pub use types::{
    BountyAlertDto, BountyBriefApi, BountyWatchPrefs, FissureBriefDto, LiveDailyBrief,
    LiveWorldOutcome, StandingDailyPrefs, StandingRanksPrefs,
};

use apply::{apply_live_fetch_scoped, ApplyScope};
use fetch::fetch_live_world;
use types::{
    SETTING_ARBITRATION_EXPIRY, SETTING_CHECKED_AT, SETTING_PLAINS_EXPIRY, SETTING_SOURCE_URL,
    SORTIE_ROW_ID, STEEL_ROW_ID,
};

pub fn refresh_live_world_if_needed(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<LiveWorldOutcome, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    if local_live_fresh(&catalog)? {
        return Ok(LiveWorldOutcome::SkippedFresh);
    }
    pull_and_apply(data_dir, &mut catalog, snowflake, ApplyScope::All)
}

/// 手动「更新数据」：无视到期判断，强制拉官方 worldState。
pub fn force_refresh_live_world(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<LiveWorldOutcome, String> {
    force_refresh_live_world_scoped(data_dir, snowflake, ApplyScope::All)
}

/// 首页：只写日常/周常（突击、侵袭、周常包）。
pub fn force_refresh_live_routines(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<LiveWorldOutcome, String> {
    force_refresh_live_world_scoped(data_dir, snowflake, ApplyScope::Routines)
}

/// 动态：只写午夜电波。
pub fn force_refresh_live_nightwave(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<LiveWorldOutcome, String> {
    force_refresh_live_world_scoped(data_dir, snowflake, ApplyScope::Nightwave)
}

fn force_refresh_live_world_scoped(
    data_dir: &Path,
    snowflake: &Snowflake,
    scope: ApplyScope,
) -> Result<LiveWorldOutcome, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let out = pull_and_apply(data_dir, &mut catalog, snowflake, scope)?;
    // 仲裁日程独立于 worldState；仅手动「更新数据」时强制重拉
    if matches!(out, LiveWorldOutcome::Updated | LiveWorldOutcome::KeptCache) {
        if let Err(e) = arbitration::force_refresh_arbitration_schedule(data_dir) {
            eprintln!("[仓鼠] 仲裁日程刷新失败（可稍后重试）: {e}");
        }
        if let Err(e) = arbitration::sync_expiry_from_cache(data_dir, &catalog) {
            eprintln!("[仓鼠] 仲裁到期点同步失败: {e}");
        }
    }
    Ok(out)
}

fn pull_and_apply(
    data_dir: &Path,
    catalog: &mut Connection,
    snowflake: &Snowflake,
    scope: ApplyScope,
) -> Result<LiveWorldOutcome, String> {
    let fetch = match fetch_live_world(data_dir) {
        Ok(f) => f,
        Err(e) => {
            if has_any_live_row(catalog)? {
                eprintln!("[仓鼠] 日常动态拉取失败，保留缓存: {e}");
                return Ok(LiveWorldOutcome::KeptCache);
            }
            return Err(e);
        }
    };
    let mut fetch = fetch;
    {
        let zh = crate::catalog::i18n::ZhLookup::load(catalog).ok();
        bounty_rewards::attach_drop_rewards(data_dir, &mut fetch.bounties, zh.as_ref());
    }
    let now = Utc::now().to_rfc3339();
    db::set_setting(catalog, SETTING_CHECKED_AT, &now)?;
    db::set_setting(catalog, SETTING_SOURCE_URL, &fetch.source_url)?;
    apply_live_fetch_scoped(catalog, snowflake, &fetch, scope)?;
    if let Err(e) = arbitration::sync_expiry_from_cache(data_dir, catalog) {
        eprintln!("[仓鼠] 仲裁到期点同步失败: {e}");
    }
    if let Err(e) = plains::sync_expiry_to_catalog(data_dir, catalog) {
        eprintln!("[仓鼠] 平原到期点同步失败: {e}");
    }
    Ok(LiveWorldOutcome::Updated)
}

fn has_any_live_row(catalog: &Connection) -> Result<bool, String> {
    let sortie: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM live_sortie WHERE id = ?1",
            [SORTIE_ROW_ID],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(sortie > 0)
}

/// 仅按各条目到期时间判断；不做「每隔 N 分钟强制拉」——不对时靠用户手动更新。
fn local_live_fresh(catalog: &Connection) -> Result<bool, String> {
    let Some(checked) = db::get_setting(catalog, SETTING_CHECKED_AT)? else {
        return Ok(false);
    };
    if DateTime::parse_from_rfc3339(&checked).is_err() {
        return Ok(false);
    }
    // 突击或侵袭已过期 → 需要刷新
    if expiry_passed(catalog, "SELECT expiry FROM live_sortie WHERE id = ?1", SORTIE_ROW_ID)? {
        return Ok(false);
    }
    if expiry_passed(
        catalog,
        "SELECT expiry FROM live_steel_incursion WHERE id = ?1",
        STEEL_ROW_ID,
    )? {
        return Ok(false);
    }
    // 任一赏金板已过期 → 需要刷新（声望页久留时依赖此判断）
    if bounty_board_expired(catalog)? {
        return Ok(false);
    }
    // 任一裂隙到期 → 需要刷新
    if any_fissure_expired(catalog)? {
        return Ok(false);
    }
    // 当前仲裁档到期 → 需要刷新（前端换档）
    if setting_time_passed(catalog, SETTING_ARBITRATION_EXPIRY)? {
        return Ok(false);
    }
    // 平原循环换档 → 需要刷新
    if setting_time_passed(catalog, SETTING_PLAINS_EXPIRY)? {
        return Ok(false);
    }
    Ok(true)
}

fn setting_time_passed(catalog: &Connection, key: &str) -> Result<bool, String> {
    let Some(raw) = db::get_setting(catalog, key)? else {
        return Ok(false);
    };
    let Ok(t) = DateTime::parse_from_rfc3339(&raw) else {
        return Ok(false);
    };
    Ok(Utc::now() >= t.with_timezone(&Utc))
}

fn any_fissure_expired(catalog: &Connection) -> Result<bool, String> {
    let now = Utc::now().to_rfc3339();
    let n: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM live_fissure
             WHERE expiry IS NOT NULL AND expiry != '' AND expiry <= ?1",
            [&now],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn bounty_board_expired(catalog: &Connection) -> Result<bool, String> {
    store_bounty::any_board_expired(catalog)
}

fn expiry_passed(catalog: &Connection, sql: &str, id: i64) -> Result<bool, String> {
    let expiry: Option<String> = catalog
        .query_row(sql, [id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    let Some(expiry) = expiry else {
        return Ok(false);
    };
    let Ok(exp) = DateTime::parse_from_rfc3339(&expiry) else {
        return Ok(false);
    };
    Ok(Utc::now() >= exp.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn catalog_with_schema() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        let sql = include_str!("../../../sql/catalog_v1.sql");
        conn.execute_batch(sql).unwrap();
        conn
    }

    #[test]
    fn fresh_when_checked_recent_and_not_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        let exp = (now + chrono::Duration::hours(12)).to_rfc3339();
        let checked = now.to_rfc3339();
        db::set_setting(&conn, SETTING_CHECKED_AT, &checked).unwrap();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&exp],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_steel_incursion (id, expiry) VALUES (1, ?1)",
            [&exp],
        )
        .unwrap();
        assert!(local_live_fresh(&conn).unwrap());
    }

    #[test]
    fn not_fresh_when_sortie_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        db::set_setting(&conn, SETTING_CHECKED_AT, &now.to_rfc3339()).unwrap();
        let past = (now - chrono::Duration::hours(1)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&past],
        )
        .unwrap();
        assert!(!local_live_fresh(&conn).unwrap());
    }

    #[test]
    fn not_fresh_when_bounty_board_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        db::set_setting(&conn, SETTING_CHECKED_AT, &now.to_rfc3339()).unwrap();
        let future = (now + chrono::Duration::hours(12)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_steel_incursion (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        let past = (now - chrono::Duration::minutes(1)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_bounty_board (tag, name, activation, expiry, fetched_at)
             VALUES ('CetusSyndicate', 'Ostron', NULL, ?1, ?2)",
            rusqlite::params![past, now.to_rfc3339()],
        )
        .unwrap();
        assert!(!local_live_fresh(&conn).unwrap());
    }

    #[test]
    fn not_fresh_when_fissure_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        db::set_setting(&conn, SETTING_CHECKED_AT, &now.to_rfc3339()).unwrap();
        let future = (now + chrono::Duration::hours(12)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_steel_incursion (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        let past = (now - chrono::Duration::minutes(1)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_fissure
             (id, kind, tier, tier_zh, mission_type, node, expiry, sort_order, fetched_at)
             VALUES (1, 'storm', 'VoidT2', '前纪', '爆发', 'Ogal', ?1, 0, ?2)",
            rusqlite::params![past, now.to_rfc3339()],
        )
        .unwrap();
        assert!(!local_live_fresh(&conn).unwrap());
    }

    #[test]
    fn not_fresh_when_arbitration_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        db::set_setting(&conn, SETTING_CHECKED_AT, &now.to_rfc3339()).unwrap();
        let future = (now + chrono::Duration::hours(12)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_steel_incursion (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        let past = (now - chrono::Duration::minutes(1)).to_rfc3339();
        db::set_setting(&conn, SETTING_ARBITRATION_EXPIRY, &past).unwrap();
        assert!(!local_live_fresh(&conn).unwrap());
    }

    #[test]
    fn still_fresh_when_old_check_but_nothing_expired() {
        let conn = catalog_with_schema();
        let now = Utc::now();
        // 很久以前拉过，但突击/裂隙等都未到期 → 不主动上网
        let checked = (now - chrono::Duration::hours(6)).to_rfc3339();
        db::set_setting(&conn, SETTING_CHECKED_AT, &checked).unwrap();
        let future = (now + chrono::Duration::hours(12)).to_rfc3339();
        conn.execute(
            "INSERT INTO live_sortie (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_steel_incursion (id, expiry) VALUES (1, ?1)",
            [&future],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO live_fissure
             (id, kind, tier, tier_zh, mission_type, node, expiry, sort_order, fetched_at)
             VALUES (1, 'normal', 'VoidT1', '古纪', '生存', 'Tharsis', ?1, 0, ?2)",
            rusqlite::params![future, now.to_rfc3339()],
        )
        .unwrap();
        assert!(local_live_fresh(&conn).unwrap());
    }
}
