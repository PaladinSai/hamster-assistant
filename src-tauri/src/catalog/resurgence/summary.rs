//! 重生读侧摘要。

use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{self, open_catalog};

use super::types::*;

pub fn get_resurgence_summary(data_dir: &Path) -> Result<ResurgenceSummary, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;

    let current_id = current_rotation_id(&catalog)?;
    let (activation, expiry) = if let Some(id) = current_id {
        catalog
            .query_row(
                "SELECT activation, expiry FROM resurgence_rotation WHERE id = ?1",
                [id],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?
    } else {
        (None, None)
    };
    let fresh = expiry
        .as_ref()
        .and_then(|e| DateTime::parse_from_rfc3339(e).ok())
        .map(|ts| Utc::now() < ts.with_timezone(&Utc))
        .unwrap_or(false);

    let mut warframes = Vec::new();
    {
        let mut stmt = catalog
            .prepare(
                "SELECT i.family_key, i.name_en, f.id, COALESCE(i.name_zh, f.name_zh), f.nickname
                 FROM resurgence_item i
                 LEFT JOIN warframe_family f ON f.family_key = i.family_key
                 WHERE i.kind = 'warframe' AND i.rotation_id = ?1
                 ORDER BY i.sort_order, i.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![current_id.unwrap_or(-1)], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (fk, name_en, fid, name_zh, nickname) = row.map_err(|e| e.to_string())?;
            let family_key = fk.unwrap_or_else(|| {
                name_en
                    .strip_suffix(" Prime")
                    .unwrap_or(&name_en)
                    .to_string()
            });
            warframes.push(ResurgenceWarframeDto {
                family_id: fid.map(|id| id.to_string()),
                family_key,
                name_en,
                name_zh,
                nickname,
            });
        }
    }

    let mut weapons = Vec::new();
    {
        let mut stmt = catalog
            .prepare(
                "SELECT i.name_en, i.unique_name, COALESCE(i.name_zh, w.name_zh, wf.name_zh), w.id, wf.id
                 FROM resurgence_item i
                 LEFT JOIN weapon_family wf ON wf.family_key = i.family_key
                 LEFT JOIN weapon w ON w.family_id = wf.id AND w.variant = 1
                 WHERE i.kind = 'weapon' AND i.rotation_id = ?1
                 ORDER BY i.sort_order, i.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![current_id.unwrap_or(-1)], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (name_en, unique_name, name_zh, wid, fid) =
                row.map_err(|e| e.to_string())?;
            weapons.push(ResurgenceWeaponDto {
                weapon_id: wid.map(|id| id.to_string()),
                family_id: fid.map(|id| id.to_string()),
                name_en,
                name_zh,
                unique_name,
            });
        }
    }

    let relic_count: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM resurgence_item WHERE kind = 'relic' AND rotation_id = ?1",
            [current_id.unwrap_or(-1)],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let next = load_next_summary(&catalog, current_id, expiry.as_deref())?;

    Ok(ResurgenceSummary {
        activation,
        expiry,
        warframes,
        weapons,
        relic_count,
        fresh,
        next,
    })
}

pub(crate) fn load_warframes_for_rotation(
    catalog: &Connection,
    rotation_id: i64,
) -> Result<Vec<ResurgenceWarframeDto>, String> {
    let mut warframes = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT i.family_key, i.name_en, f.id, COALESCE(i.name_zh, f.name_zh), f.nickname
             FROM resurgence_item i
             LEFT JOIN warframe_family f ON f.family_key = i.family_key
             WHERE i.kind = 'warframe' AND i.rotation_id = ?1
             ORDER BY i.sort_order, i.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![rotation_id], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (fk, name_en, fid, name_zh, nickname) = row.map_err(|e| e.to_string())?;
        let family_key = fk.unwrap_or_else(|| {
            name_en
                .strip_suffix(" Prime")
                .unwrap_or(&name_en)
                .to_string()
        });
        warframes.push(ResurgenceWarframeDto {
            family_id: fid.map(|id| id.to_string()),
            family_key,
            name_en,
            name_zh,
            nickname,
        });
    }
    Ok(warframes)
}

pub(crate) fn load_next_summary(
    catalog: &Connection,
    current_id: Option<i64>,
    current_expiry: Option<&str>,
) -> Result<Option<ResurgenceNextDto>, String> {
    let Some(cur_id) = current_id else {
        return Ok(None);
    };
    let Some(exp) = current_expiry else {
        return Ok(None);
    };
    let next_id: Option<i64> = catalog
        .query_row(
            "SELECT id FROM resurgence_rotation
             WHERE id != ?1 AND (
               activation = ?2
               OR (expiry IS NOT NULL AND expiry > ?2)
             )
             ORDER BY COALESCE(expiry, activation, '') ASC
             LIMIT 1",
            params![cur_id, exp],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some(nid) = next_id else {
        return Ok(None);
    };
    let (activation, expiry): (Option<String>, Option<String>) = catalog
        .query_row(
            "SELECT activation, expiry FROM resurgence_rotation WHERE id = ?1",
            [nid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let warframes = load_warframes_for_rotation(catalog, nid)?;
    if warframes.is_empty() {
        return Ok(None);
    }
    Ok(Some(ResurgenceNextDto {
        activation,
        expiry,
        warframes,
    }))
}

pub(crate) fn current_rotation_id(catalog: &Connection) -> Result<Option<i64>, String> {
    let now = Utc::now().to_rfc3339();
    let future: Option<i64> = catalog
        .query_row(
            "SELECT id FROM resurgence_rotation
             WHERE expiry IS NOT NULL AND expiry > ?1
             ORDER BY expiry ASC
             LIMIT 1",
            [&now],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if future.is_some() {
        return Ok(future);
    }
    catalog
        .query_row(
            "SELECT id FROM resurgence_rotation
             ORDER BY COALESCE(expiry, activation, '') DESC
             LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
}

pub fn load_resurgence_relic_keys(catalog: &Connection) -> Result<HashSet<String>, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT relic_key FROM resurgence_item
             WHERE kind = 'relic' AND relic_key IS NOT NULL AND rotation_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let current = current_rotation_id(catalog)?.unwrap_or(-1);
    let rows = stmt
        .query_map([current], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut set = HashSet::new();
    for r in rows {
        set.insert(r.map_err(|e| e.to_string())?);
    }
    Ok(set)
}


