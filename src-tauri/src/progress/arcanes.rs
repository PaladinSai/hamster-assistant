//! 赋能进度。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::common::{id_to_str, open_pair, parse_id};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcaneListItem {
    pub arcane_id: String,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub arcane_kind: i32,
    pub rarity: i32,
    pub source_kind: i32,
    pub max_rank: i32,
    pub status: i32,
    pub rank: i32,
    pub copies_to_max: i32,
    pub wish: i32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertArcaneProgress {
    pub arcane_id: String,
    pub status: Option<i32>,
    pub rank: Option<i32>,
    pub wish: Option<i32>,
    pub note: Option<String>,
}

#[derive(Clone, Default)]
struct ArcaneProgressRow {
    status: i32,
    rank: i32,
    wish: i32,
    note: Option<String>,
}

pub fn list_arcanes(data_dir: &Path) -> Result<Vec<ArcaneListItem>, String> {
    let (catalog, user) = open_pair(data_dir)?;

    let has_table: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='arcane'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has_table == 0 {
        return Ok(Vec::new());
    }

    let mut progress: HashMap<i64, ArcaneProgressRow> = HashMap::new();
    {
        let mut stmt = user
            .prepare("SELECT arcane_id, status, rank, wish, note FROM arcane_progress")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ArcaneProgressRow {
                        status: row.get(1)?,
                        rank: row.get(2)?,
                        wish: row.get(3)?,
                        note: row.get(4)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (id, row) = r.map_err(|e| e.to_string())?;
            progress.insert(id, row);
        }
    }

    let mut stmt = catalog
        .prepare(
            "SELECT id, name_en, name_zh, arcane_kind, rarity, source_kind, max_rank, sort_order
             FROM arcane
             ORDER BY sort_order, name_en",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows {
        let (id, name_en, name_zh, arcane_kind, rarity, source_kind, max_rank) =
            r.map_err(|e| e.to_string())?;
        let pf = progress.get(&id).cloned().unwrap_or_default();
        let status = pf.status.clamp(0, 2);
        let rank = if status == 0 {
            0
        } else {
            pf.rank.clamp(0, max_rank.max(0))
        };
        out.push(ArcaneListItem {
            arcane_id: id_to_str(id),
            name_en,
            name_zh,
            arcane_kind,
            rarity,
            source_kind,
            max_rank,
            status,
            rank,
            copies_to_max: crate::catalog::copies_to_max(status, rank, max_rank),
            wish: pf.wish,
            note: pf.note,
        });
    }
    Ok(out)
}

pub fn upsert_arcane_progress(data_dir: &Path, patch: UpsertArcaneProgress) -> Result<(), String> {
    let arcane_id = parse_id(&patch.arcane_id, "赋能 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let max_rank: i32 = catalog
        .query_row(
            "SELECT max_rank FROM arcane WHERE id = ?1",
            [arcane_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("赋能 id={arcane_id} 不存在"))?;

    let existing = user
        .query_row(
            "SELECT status, rank, wish, note FROM arcane_progress WHERE arcane_id = ?1",
            [arcane_id],
            |row| {
                Ok(ArcaneProgressRow {
                    status: row.get(0)?,
                    rank: row.get(1)?,
                    wish: row.get(2)?,
                    note: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let wish = patch
        .wish
        .map(|w| if w != 0 { 1 } else { 0 })
        .unwrap_or(existing.wish);
    let note = patch.note.clone().or(existing.note);

    let (status, rank) = match (patch.status, patch.rank) {
        (Some(0), _) => (0, 0),
        (_, Some(r)) => {
            let rank = r.clamp(0, max_rank.max(0));
            let status = crate::catalog::status_from_rank(true, rank, max_rank);
            (status, if status == 0 { 0 } else { rank })
        }
        (Some(2), _) => (2, max_rank.max(0)),
        (Some(1), _) => {
            let r = existing.rank.clamp(0, max_rank.max(0));
            let rank = if max_rank > 0 && r >= max_rank {
                max_rank - 1
            } else {
                r
            };
            (1, rank)
        }
        _ => (
            existing.status.clamp(0, 2),
            existing.rank.clamp(0, max_rank.max(0)),
        ),
    };

    user.execute(
        "INSERT INTO arcane_progress (arcane_id, status, rank, wish, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(arcane_id) DO UPDATE SET
            status=excluded.status,
            rank=excluded.rank,
            wish=excluded.wish,
            note=excluded.note,
            updated_at=excluded.updated_at",
        params![arcane_id, status, rank, wish, note, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

