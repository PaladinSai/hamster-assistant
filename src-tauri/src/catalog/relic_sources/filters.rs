//! 遗物来源筛选项。

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use crate::db::{self, open_catalog};

use super::types::*;

pub fn get_relic_source_filter_options(data_dir: &Path) -> Result<RelicSourceFilterOptions, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    load_filter_options(&catalog)
}

pub(crate) fn load_filter_options(catalog: &Connection) -> Result<RelicSourceFilterOptions, String> {
    let mut relics = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT DISTINCT v.relic_key, v.name_zh, v.era
             FROM void_relic v
             JOIN relic_location l ON l.relic_id = v.id
             ORDER BY v.era, v.code",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i32>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for r in rows {
        let (key, name_zh, era) = r.map_err(|e| e.to_string())?;
        relics.push(FilterChip {
            key,
            label_zh: name_zh,
            era: Some(era),
        });
    }

    let mut modes = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT DISTINCT game_mode, game_mode_zh FROM relic_location
             WHERE source_kind = 'mission' AND game_mode IS NOT NULL AND game_mode != ''
             ORDER BY game_mode",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| e.to_string())?;
    for r in rows {
        let (key, zh) = r.map_err(|e| e.to_string())?;
        modes.push(FilterChip {
            label_zh: zh.unwrap_or_else(|| key.clone()),
            key,
            era: None,
        });
    }
    let has_bounty: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM relic_location WHERE source_kind = 'bounty'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has_bounty > 0 {
        let bounty_zh: Option<String> = catalog
            .query_row(
                "SELECT game_mode_zh FROM relic_location
                 WHERE source_kind = 'bounty' AND game_mode_zh IS NOT NULL LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        modes.push(FilterChip {
            key: "bounty".into(),
            label_zh: bounty_zh.unwrap_or_else(|| "赏金".into()),
            era: None,
        });
    }

    let mut planets = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT DISTINCT planet_or_hub, planet_or_hub_zh FROM relic_location ORDER BY planet_or_hub",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| e.to_string())?;
    for r in rows {
        let (key, zh) = r.map_err(|e| e.to_string())?;
        planets.push(FilterChip {
            label_zh: zh.unwrap_or_else(|| key.clone()),
            key,
            era: None,
        });
    }

    let mut rotations = Vec::new();
    let mut stmt = catalog
        .prepare(
            "SELECT DISTINCT rotation FROM relic_location
             WHERE rotation IS NOT NULL AND rotation != ''
             ORDER BY rotation",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    for r in rows {
        rotations.push(r.map_err(|e| e.to_string())?);
    }
    let blank: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM relic_location WHERE rotation IS NULL OR rotation = ''",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    Ok(RelicSourceFilterOptions {
        relics,
        modes,
        planets,
        rotations,
        has_blank_rotation: blank > 0,
        has_bounty: has_bounty > 0,
    })
}
