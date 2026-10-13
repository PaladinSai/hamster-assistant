//! 日常 / 周常清单（懒重置）。
//!
//! - 一般 daily：UTC 日历日切（0:00 UTC；钢铁侵袭 / 电波日常 / 声望上限等）
//! - 内置「突击」：加拿大东部正午轮换（wiki；夏令 16:00 UTC / 冬令 17:00 UTC），period_key 记 UTC
//! - weekly：UTC 周一（电波周常 / 执政官 / 卡尔粉碎纳拉摩等）

use std::path::Path;

use chrono::{Datelike, Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::America::Toronto;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::{self, open_user};
use super::common::{id_to_str, parse_id};

const BUILTIN_SORTIE_TITLE: &str = "突击";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineItemDto {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub is_builtin: bool,
    pub ignored: bool,
    pub done: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoutine {
    pub kind: String,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRoutine {
    pub id: String,
    pub title: Option<String>,
    pub done: Option<bool>,
    pub ignored: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderRoutines {
    pub kind: String,
    pub ids: Vec<String>,
}

pub fn period_key_for(kind: &str, now: chrono::DateTime<Utc>) -> Result<String, String> {
    match kind {
        "daily" => Ok(now.format("%Y-%m-%d").to_string()),
        "weekly" => {
            let date = now.date_naive();
            let days = date.weekday().num_days_from_monday() as i64;
            let monday = date - Duration::days(days);
            Ok(monday.format("%Y-%m-%d").to_string())
        }
        other => Err(format!("无效的 kind={other}，仅支持 daily / weekly")),
    }
}

/// 当前突击周期起点（UTC）：最近一次加拿大东部（America/Toronto）正午。
fn sortie_cycle_start_utc(now: chrono::DateTime<Utc>) -> chrono::DateTime<Utc> {
    let local = now.with_timezone(&Toronto);
    let noon = NaiveTime::from_hms_opt(12, 0, 0).expect("noon");
    let day = if local.time() < noon {
        local.date_naive() - Duration::days(1)
    } else {
        local.date_naive()
    };
    let local_noon = day.and_time(noon);
    Toronto
        .from_local_datetime(&local_noon)
        .single()
        .expect("Toronto noon is unambiguous")
        .with_timezone(&Utc)
}

/// 突击 period_key：周期起点的 UTC 时刻（与其它键一样用 UTC 字符串）。
fn sortie_period_key(now: chrono::DateTime<Utc>) -> String {
    sortie_cycle_start_utc(now)
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

/// 为某条例程生成当前周期键（勾选写入 / 懒重置比对用）。
fn period_key_for_item(
    kind: &str,
    title: &str,
    is_builtin: bool,
    now: chrono::DateTime<Utc>,
) -> Result<String, String> {
    if is_builtin && title == BUILTIN_SORTIE_TITLE {
        return Ok(sortie_period_key(now));
    }
    period_key_for(kind, now)
}

fn apply_lazy_reset(
    user: &rusqlite::Connection,
    id: i64,
    kind: &str,
    title: &str,
    is_builtin: bool,
    done: i32,
    period_key: Option<String>,
) -> Result<(i32, Option<String>), String> {
    if done == 0 {
        return Ok((0, None));
    }
    let current = period_key_for_item(kind, title, is_builtin, Utc::now())?;
    if period_key.as_deref() == Some(current.as_str()) {
        return Ok((1, period_key));
    }
    user.execute(
        "UPDATE routine_item SET done=0, period_key=NULL, updated_at=?1 WHERE id=?2",
        params![Utc::now().to_rfc3339(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok((0, None))
}

fn row_to_dto(
    id: i64,
    kind: String,
    title: String,
    is_builtin: i32,
    ignored: i32,
    done: i32,
    sort_order: i32,
) -> RoutineItemDto {
    RoutineItemDto {
        id: id_to_str(id),
        kind,
        title,
        is_builtin: is_builtin != 0,
        ignored: ignored != 0,
        done: done != 0,
        sort_order,
    }
}

pub fn list_routines(data_dir: &Path) -> Result<Vec<RoutineItemDto>, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let mut stmt = user
        .prepare(
            "SELECT id, kind, title, is_builtin, ignored, done, period_key, sort_order
             FROM routine_item
             ORDER BY kind ASC, sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i32>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows {
        let (id, kind, title, is_builtin, ignored, done, period_key, sort_order) =
            r.map_err(|e| e.to_string())?;
        let (done, _) = apply_lazy_reset(
            &user,
            id,
            &kind,
            &title,
            is_builtin != 0,
            done,
            period_key,
        )?;
        out.push(row_to_dto(
            id, kind, title, is_builtin, ignored, done, sort_order,
        ));
    }
    Ok(out)
}

pub fn create_routine(data_dir: &Path, input: CreateRoutine) -> Result<RoutineItemDto, String> {
    let kind = input.kind.trim();
    if kind != "daily" && kind != "weekly" {
        return Err("kind 仅支持 daily / weekly".into());
    }
    let title = input.title.trim();
    if title.is_empty() {
        return Err("标题不能为空".into());
    }
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();
    let next_order: i32 = user
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM routine_item WHERE kind = ?1",
            [kind],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    user.execute(
        "INSERT INTO routine_item (kind, title, is_builtin, ignored, done, period_key, sort_order, updated_at)
         VALUES (?1, ?2, 0, 0, 0, NULL, ?3, ?4)",
        params![kind, title, next_order, now],
    )
    .map_err(|e| e.to_string())?;
    let id: i64 = user
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(row_to_dto(
        id,
        kind.to_string(),
        title.to_string(),
        0,
        0,
        0,
        next_order,
    ))
}

pub fn update_routine(data_dir: &Path, patch: UpdateRoutine) -> Result<RoutineItemDto, String> {
    let id = parse_id(&patch.id, "日常/周常 id")?;
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();

    let existing = user
        .query_row(
            "SELECT kind, title, is_builtin, ignored, done, period_key, sort_order
             FROM routine_item WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, i32>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i32>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("日常/周常 id={id} 不存在"))?;

    let (kind, old_title, is_builtin, old_ignored, old_done, old_period, sort_order) = existing;
    let (old_done, _) = apply_lazy_reset(
        &user,
        id,
        &kind,
        &old_title,
        is_builtin != 0,
        old_done,
        old_period,
    )?;

    if patch.title.is_some() && is_builtin != 0 {
        return Err("内置项不能改名".into());
    }
    let title = patch
        .title
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(old_title);

    let ignored = patch
        .ignored
        .map(|v| if v { 1 } else { 0 })
        .unwrap_or(old_ignored);

    let utc_now = Utc::now();
    let (done, period_key): (i32, Option<String>) = match patch.done {
        Some(true) => (
            1,
            Some(period_key_for_item(
                &kind,
                &title,
                is_builtin != 0,
                utc_now,
            )?),
        ),
        Some(false) => (0, None),
        None => (
            old_done,
            if old_done != 0 {
                Some(period_key_for_item(
                    &kind,
                    &title,
                    is_builtin != 0,
                    utc_now,
                )?)
            } else {
                None
            },
        ),
    };

    user.execute(
        "UPDATE routine_item
         SET title=?1, ignored=?2, done=?3, period_key=?4, updated_at=?5
         WHERE id=?6",
        params![title, ignored, done, period_key, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(row_to_dto(
        id, kind, title, is_builtin, ignored, done, sort_order,
    ))
}

pub fn delete_routine(data_dir: &Path, id: &str) -> Result<(), String> {
    let id = parse_id(id, "日常/周常 id")?;
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let is_builtin: i32 = user
        .query_row(
            "SELECT is_builtin FROM routine_item WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("日常/周常 id={id} 不存在"))?;
    if is_builtin != 0 {
        return Err("内置项不能删除，可改为忽略".into());
    }
    let n = user
        .execute("DELETE FROM routine_item WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("日常/周常 id={id} 不存在"));
    }
    Ok(())
}

pub fn reorder_routines(data_dir: &Path, input: ReorderRoutines) -> Result<(), String> {
    let kind = input.kind.trim();
    if kind != "daily" && kind != "weekly" {
        return Err("kind 仅支持 daily / weekly".into());
    }
    if input.ids.is_empty() {
        return Err("排序列表不能为空".into());
    }
    db::init_databases(data_dir)?;
    let mut user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();

    let mut seen = std::collections::HashSet::new();
    let mut parsed: Vec<i64> = Vec::with_capacity(input.ids.len());
    for raw in &input.ids {
        let id = parse_id(raw, "日常/周常 id")?;
        if !seen.insert(id) {
            return Err(format!("排序列表含重复 id={id}"));
        }
        let row_kind: String = user
            .query_row(
                "SELECT kind FROM routine_item WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("日常/周常 id={id} 不存在"))?;
        if row_kind != kind {
            return Err(format!("id={id} 不属于 {kind}"));
        }
        parsed.push(id);
    }

    let tx = user.transaction().map_err(|e| e.to_string())?;
    for (i, id) in parsed.iter().enumerate() {
        tx.execute(
            "UPDATE routine_item SET sort_order=?1, updated_at=?2 WHERE id=?3",
            params![i as i32, now, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    use crate::user_migrate::migrate_user_conn;

    #[test]
    fn period_keys_daily_and_weekly() {
        let wed = Utc.with_ymd_and_hms(2026, 9, 30, 12, 0, 0).unwrap();
        assert_eq!(period_key_for("daily", wed).unwrap(), "2026-09-30");
        // 2026-09-30 is Wednesday → week starts 2026-09-28
        assert_eq!(period_key_for("weekly", wed).unwrap(), "2026-09-28");
    }

    #[test]
    fn sortie_follows_toronto_noon_recorded_as_utc() {
        // 2026-10 仍为夏令：东部正午 = 16:00 UTC
        let after_noon_edt = Utc.with_ymd_and_hms(2026, 10, 7, 20, 0, 0).unwrap();
        assert_eq!(sortie_period_key(after_noon_edt), "2026-10-07T16:00:00Z");
        let before_noon_edt = Utc.with_ymd_and_hms(2026, 10, 7, 15, 0, 0).unwrap();
        assert_eq!(sortie_period_key(before_noon_edt), "2026-10-06T16:00:00Z");
        // 2026-01 冬令：东部正午 = 17:00 UTC
        let after_noon_est = Utc.with_ymd_and_hms(2026, 1, 15, 20, 0, 0).unwrap();
        assert_eq!(sortie_period_key(after_noon_est), "2026-01-15T17:00:00Z");
        // 普通日常仍用 UTC 日历日
        assert_eq!(
            period_key_for_item("daily", "钢铁之路侵袭", true, after_noon_edt).unwrap(),
            "2026-10-07"
        );
    }

    #[test]
    fn lazy_reset_clears_stale_done() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_conn(&conn).unwrap();
        conn.execute(
            "UPDATE routine_item SET done=1, period_key='2000-01-01'
             WHERE title='突击'",
            [],
        )
        .unwrap();
        let (id, kind, done, pk): (i64, String, i32, Option<String>) = conn
            .query_row(
                "SELECT id, kind, done, period_key FROM routine_item WHERE title='突击'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        let (done2, pk2) =
            apply_lazy_reset(&conn, id, &kind, "突击", true, done, pk).unwrap();
        assert_eq!(done2, 0);
        assert!(pk2.is_none());
    }

    #[test]
    fn refuse_delete_builtin_flag() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_conn(&conn).unwrap();
        let builtin: i32 = conn
            .query_row(
                "SELECT is_builtin FROM routine_item WHERE title='突击'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(builtin, 1);
    }

    #[test]
    fn weekly_builtin_default_order() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_conn(&conn).unwrap();
        let titles: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT title FROM routine_item
                     WHERE kind='weekly' AND title != '午夜电波·周常'
                     ORDER BY sort_order ASC, id ASC",
                )
                .unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(
            titles,
            crate::user_migrate::WEEKLY_BUILTIN_ORDER
                .iter()
                .map(|s| (*s).to_string())
                .collect::<Vec<_>>()
        );
    }
}
