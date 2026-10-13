//! 赏金板正规表读写。

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::id::Snowflake;

use super::localize_bounty;
use super::types::{BountyBoardApi, BountyBriefApi, BountyJobApi, LEGACY_BOUNTY_JSON_KEY};

pub(crate) fn replace_bounties(
    tx: &Connection,
    snowflake: &Snowflake,
    brief: &BountyBriefApi,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    tx.execute("DELETE FROM live_bounty_job_reward", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_bounty_job", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_bounty_board", [])
        .map_err(|e| e.to_string())?;

    for board in &brief.boards {
        tx.execute(
            "INSERT INTO live_bounty_board (tag, name, activation, expiry, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                board.tag,
                board.name,
                board.activation,
                board.expiry,
                now
            ],
        )
        .map_err(|e| format!("写 live_bounty_board 失败: {e}"))?;

        for (i, job) in board.jobs.iter().enumerate() {
            let job_id = snowflake.next_id();
            tx.execute(
                "INSERT INTO live_bounty_job
                 (id, board_tag, sort_order, job_type, kind, kind_zh, title,
                  min_enemy_level, max_enemy_level, standing_total,
                  is_advanced, is_steel_path, challenge_zh)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    job_id,
                    board.tag,
                    i as i32,
                    job.job_type,
                    job.kind,
                    job.kind_zh,
                    job.title,
                    job.min_enemy_level,
                    job.max_enemy_level,
                    job.standing_total,
                    if job.is_advanced { 1 } else { 0 },
                    if job.is_steel_path { 1 } else { 0 },
                    job.challenge_zh,
                ],
            )
            .map_err(|e| format!("写 live_bounty_job 失败: {e}"))?;

            for (ri, name) in job.rewards.iter().enumerate() {
                tx.execute(
                    "INSERT INTO live_bounty_job_reward (job_id, sort_order, name)
                     VALUES (?1, ?2, ?3)",
                    params![job_id, ri as i32, name],
                )
                .map_err(|e| format!("写 live_bounty_job_reward 失败: {e}"))?;
            }
        }
    }

    tx.execute("DELETE FROM setting WHERE key = ?1", [LEGACY_BOUNTY_JSON_KEY])
        .ok();
    Ok(())
}

pub(crate) fn read_bounties(catalog: &Connection) -> Result<BountyBriefApi, String> {
    let mut boards = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT tag, name, activation, expiry FROM live_bounty_board
             ORDER BY tag ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (tag, name, activation, expiry) = row.map_err(|e| e.to_string())?;
        let jobs = read_jobs(catalog, &tag)?;
        boards.push(BountyBoardApi {
            tag,
            name,
            activation,
            expiry,
            jobs,
        });
    }

    if boards.is_empty() {
        return Ok(empty_bounty_brief());
    }

    // 保证白名单势力都有板（jobs 可空）
    let mut by_tag: std::collections::HashMap<String, BountyBoardApi> =
        boards.into_iter().map(|b| (b.tag.clone(), b)).collect();
    let mut ordered = Vec::new();
    for (tag, name) in localize_bounty::BOUNTY_SYNDICATES {
        if let Some(b) = by_tag.remove(*tag) {
            ordered.push(b);
        } else {
            ordered.push(BountyBoardApi {
                tag: (*tag).into(),
                name: (*name).into(),
                activation: None,
                expiry: None,
                jobs: vec![],
            });
        }
    }
    for (_, b) in by_tag {
        ordered.push(b);
    }
    Ok(BountyBriefApi { boards: ordered })
}

fn read_jobs(catalog: &Connection, board_tag: &str) -> Result<Vec<BountyJobApi>, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT id, job_type, kind, kind_zh, title, min_enemy_level, max_enemy_level,
                    standing_total, is_advanced, is_steel_path, challenge_zh
             FROM live_bounty_job
             WHERE board_tag = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([board_tag], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i32>(5)?,
                r.get::<_, i32>(6)?,
                r.get::<_, i32>(7)?,
                r.get::<_, i32>(8)?,
                r.get::<_, i32>(9)?,
                r.get::<_, String>(10)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut jobs = Vec::new();
    for row in rows {
        let (
            id,
            job_type,
            kind,
            kind_zh,
            title,
            min_lv,
            max_lv,
            standing,
            advanced,
            steel,
            challenge_zh,
        ) = row.map_err(|e| e.to_string())?;
        let rewards = read_rewards(catalog, id)?;
        jobs.push(BountyJobApi {
            job_type,
            kind,
            kind_zh,
            title,
            min_enemy_level: min_lv,
            max_enemy_level: max_lv,
            standing_total: standing,
            is_advanced: advanced != 0,
            is_steel_path: steel != 0,
            challenge_zh,
            rewards,
            reward_table: String::new(),
        });
    }
    Ok(jobs)
}

fn read_rewards(catalog: &Connection, job_id: i64) -> Result<Vec<String>, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT name FROM live_bounty_job_reward
             WHERE job_id = ?1 ORDER BY sort_order ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([job_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn empty_bounty_brief() -> BountyBriefApi {
    BountyBriefApi {
        boards: localize_bounty::BOUNTY_SYNDICATES
            .iter()
            .map(|(tag, name)| BountyBoardApi {
                tag: (*tag).to_string(),
                name: (*name).to_string(),
                activation: None,
                expiry: None,
                jobs: vec![],
            })
            .collect(),
    }
}

/// 任一赏金板已过期。
pub(crate) fn any_board_expired(catalog: &Connection) -> Result<bool, String> {
    let now = Utc::now().to_rfc3339();
    let hit: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM live_bounty_board
             WHERE expiry IS NOT NULL AND expiry != '' AND expiry <= ?1",
            [&now],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(hit > 0)
}
