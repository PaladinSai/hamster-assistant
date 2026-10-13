//! 遗物掉落点与筛选项加载。

use std::collections::HashMap;

use rusqlite::{params_from_iter, Connection};

use super::types::*;

#[allow(dead_code)]
pub(crate) fn load_locations(
    catalog: &Connection,
    relic_id: i64,
    modes: &[String],
    want_bounty: bool,
    named_rots: &[String],
    blank_rot: bool,
    planets: &[String],
) -> Result<Vec<RelicLocationDto>, String> {
    let mut map = load_locations_for_relics(
        catalog,
        &[relic_id],
        modes,
        want_bounty,
        named_rots,
        blank_rot,
        planets,
    )?;
    Ok(map.remove(&relic_id).unwrap_or_default())
}

/// 一次查出多个遗物的掉落点，避免搜索时 N+1。
pub(crate) fn load_locations_for_relics(
    catalog: &Connection,
    relic_ids: &[i64],
    modes: &[String],
    want_bounty: bool,
    named_rots: &[String],
    blank_rot: bool,
    planets: &[String],
) -> Result<HashMap<i64, Vec<RelicLocationDto>>, String> {
    let mut out: HashMap<i64, Vec<RelicLocationDto>> = HashMap::new();
    if relic_ids.is_empty() {
        return Ok(out);
    }

    let mut sql = String::from(
        "SELECT relic_id, source_kind, planet_or_hub, planet_or_hub_zh, node_or_bounty, node_or_bounty_zh,
                game_mode, game_mode_zh, rotation, chance, stage, is_event
         FROM relic_location WHERE relic_id IN (",
    );
    let ph: Vec<&str> = relic_ids.iter().map(|_| "?").collect();
    sql.push_str(&ph.join(","));
    sql.push(')');

    let mut binds: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    for id in relic_ids {
        binds.push(Box::new(*id));
    }

    if !modes.is_empty() || want_bounty {
        sql.push_str(" AND (");
        let mut parts = Vec::new();
        if !modes.is_empty() {
            let mph: Vec<&str> = modes.iter().map(|_| "?").collect();
            parts.push(format!("game_mode IN ({})", mph.join(",")));
            for m in modes {
                binds.push(Box::new(m.clone()));
            }
        }
        if want_bounty {
            parts.push("source_kind = 'bounty'".into());
        }
        sql.push_str(&parts.join(" OR "));
        sql.push(')');
    }
    if !named_rots.is_empty() || blank_rot {
        sql.push_str(" AND (");
        let mut parts = Vec::new();
        if !named_rots.is_empty() {
            let rph: Vec<&str> = named_rots.iter().map(|_| "?").collect();
            parts.push(format!("rotation IN ({})", rph.join(",")));
            for r in named_rots {
                binds.push(Box::new(r.clone()));
            }
        }
        if blank_rot {
            parts.push("(rotation IS NULL OR rotation = '')".into());
        }
        sql.push_str(&parts.join(" OR "));
        sql.push(')');
    }
    if !planets.is_empty() {
        let pph: Vec<&str> = planets.iter().map(|_| "?").collect();
        sql.push_str(" AND planet_or_hub IN (");
        sql.push_str(&pph.join(","));
        sql.push(')');
        for p in planets {
            binds.push(Box::new(p.clone()));
        }
    }
    sql.push_str(" ORDER BY relic_id, (chance IS NULL), chance DESC, planet_or_hub, node_or_bounty");

    let mut stmt = catalog.prepare(&sql).map_err(|e| e.to_string())?;
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(params_from_iter(bind_refs.iter().map(|b| *b)), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<f64>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, i32>(11)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in rows {
        let (relic_id, kind, hub, hub_zh, node, node_zh, mode, mode_zh, rot, chance, stage, is_event) =
            row.map_err(|e| e.to_string())?;
        out.entry(relic_id).or_default().push(RelicLocationDto {
            source_kind: kind,
            planet_or_hub: hub.clone(),
            planet_or_hub_zh: Some(hub_zh.unwrap_or(hub)),
            node_or_bounty: node.clone(),
            node_or_bounty_zh: Some(node_zh.unwrap_or(node)),
            game_mode: mode.clone(),
            game_mode_zh: mode_zh.or(mode),
            rotation: rot,
            chance,
            stage,
            is_event,
        });
    }
    Ok(out)
}

#[allow(dead_code)]
pub(crate) fn sync_vaulted_from_locations(catalog: &Connection) -> Result<(), String> {
    let n: i64 = catalog
        .query_row("SELECT COUNT(*) FROM relic_location", [], |r| r.get(0))
        .unwrap_or(0);
    if n == 0 {
        return Ok(());
    }
    catalog
        .execute_batch(
            "UPDATE void_relic SET vaulted = 0
               WHERE id IN (SELECT DISTINCT relic_id FROM relic_location);
             UPDATE void_relic SET vaulted = 1
               WHERE id NOT IN (SELECT DISTINCT relic_id FROM relic_location);",
        )
        .map_err(|e| e.to_string())
}
