//! 周常正规表读写。

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::id::Snowflake;

use super::types::{
    ArchimedeaApi, ArchimedeaMissionApi, ArchonHuntApi, ArchonMissionApi, CircuitApi,
    HexCalendarApi, HexCalendarDayApi, HexCalendarGroupApi, LabeledTerm, WeeklyBundle,
    LEGACY_WEEKLY_JSON_KEY,
};

const ARCHON_ID: i64 = 1;
const HEX_ID: i64 = 1;
const KIND_DEEP: &str = "deep";
const KIND_TEMPORAL: &str = "temporal";
const KIND_STEEL: &str = "steel";
const KIND_NORMAL: &str = "normal";

pub(crate) fn replace_weekly(
    tx: &Connection,
    snowflake: &Snowflake,
    weekly: &WeeklyBundle,
) -> Result<(), String> {
    clear_weekly(tx)?;
    let now = Utc::now().to_rfc3339();

    if let Some(archon) = &weekly.archon_hunt {
        write_archon(tx, snowflake, archon, &now)?;
    }
    if let Some(a) = &weekly.deep_archimedea {
        write_archimedea(tx, snowflake, KIND_DEEP, a, &now)?;
    }
    if let Some(a) = &weekly.temporal_archimedea {
        write_archimedea(tx, snowflake, KIND_TEMPORAL, a, &now)?;
    }
    if let Some(c) = &weekly.steel_circuit {
        write_circuit(tx, KIND_STEEL, c, &now)?;
    }
    if let Some(c) = &weekly.normal_circuit {
        write_circuit(tx, KIND_NORMAL, c, &now)?;
    }
    if let Some(h) = &weekly.hex_calendar {
        write_hex(tx, snowflake, h, &now)?;
    }

    tx.execute("DELETE FROM setting WHERE key = ?1", [LEGACY_WEEKLY_JSON_KEY])
        .ok();
    Ok(())
}

fn clear_weekly(tx: &Connection) -> Result<(), String> {
    tx.execute("DELETE FROM live_hex_calendar_group", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_hex_calendar_day", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_hex_calendar", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_archimedea_mission", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_archimedea", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_circuit", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_archon_mission", [])
        .map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM live_archon_hunt", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_archon(
    tx: &Connection,
    snowflake: &Snowflake,
    a: &ArchonHuntApi,
    now: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO live_archon_hunt (id, activation, expiry, boss, shard, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![ARCHON_ID, a.activation, a.expiry, a.boss, a.shard, now],
    )
    .map_err(|e| format!("写 live_archon_hunt 失败: {e}"))?;
    for (i, m) in a.missions.iter().enumerate() {
        tx.execute(
            "INSERT INTO live_archon_mission (id, hunt_id, sort_order, mission_type, node)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                snowflake.next_id(),
                ARCHON_ID,
                i as i32,
                m.mission_type,
                m.node
            ],
        )
        .map_err(|e| format!("写 live_archon_mission 失败: {e}"))?;
    }
    Ok(())
}

fn write_archimedea(
    tx: &Connection,
    snowflake: &Snowflake,
    kind: &str,
    a: &ArchimedeaApi,
    now: &str,
) -> Result<(), String> {
    let mods = serde_json::to_string(&a.personal_modifiers).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO live_archimedea
         (kind, activation, expiry, kind_label, personal_modifiers_json, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![kind, a.activation, a.expiry, a.kind_label, mods, now],
    )
    .map_err(|e| format!("写 live_archimedea 失败: {e}"))?;
    for (i, m) in a.missions.iter().enumerate() {
        let risks_n = serde_json::to_string(&m.risks_normal).map_err(|e| e.to_string())?;
        let risks_e = serde_json::to_string(&m.risks_elite).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO live_archimedea_mission
             (id, archimedea_kind, sort_order, mission_type, faction,
              deviation_title, deviation_description, risks_normal_json, risks_elite_json)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                snowflake.next_id(),
                kind,
                i as i32,
                m.mission_type,
                m.faction,
                m.deviation.title,
                m.deviation.description,
                risks_n,
                risks_e,
            ],
        )
        .map_err(|e| format!("写 live_archimedea_mission 失败: {e}"))?;
    }
    Ok(())
}

fn write_circuit(
    tx: &Connection,
    kind: &str,
    c: &CircuitApi,
    now: &str,
) -> Result<(), String> {
    let this_week = serde_json::to_string(&c.this_week).map_err(|e| e.to_string())?;
    let next_week = serde_json::to_string(&c.next_week).map_err(|e| e.to_string())?;
    let rotation = serde_json::to_string(&c.rotation_weeks).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO live_circuit
         (kind, activation, expiry, this_week_json, next_week_json,
          rotation_weeks_json, current_week_index, fetched_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            kind,
            c.activation,
            c.expiry,
            this_week,
            next_week,
            rotation,
            c.current_week_index,
            now
        ],
    )
    .map_err(|e| format!("写 live_circuit 失败: {e}"))?;
    Ok(())
}

fn write_hex(
    tx: &Connection,
    snowflake: &Snowflake,
    h: &HexCalendarApi,
    now: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO live_hex_calendar (id, activation, expiry, fetched_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![HEX_ID, h.activation, h.expiry, now],
    )
    .map_err(|e| format!("写 live_hex_calendar 失败: {e}"))?;
    for day in &h.days {
        let day_id = snowflake.next_id();
        tx.execute(
            "INSERT INTO live_hex_calendar_day (id, calendar_id, day) VALUES (?1, ?2, ?3)",
            params![day_id, HEX_ID, day.day],
        )
        .map_err(|e| format!("写 live_hex_calendar_day 失败: {e}"))?;
        for (gi, g) in day.groups.iter().enumerate() {
            let choices = serde_json::to_string(&g.choices).map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO live_hex_calendar_group
                 (id, day_id, sort_order, kind, choices_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![snowflake.next_id(), day_id, gi as i32, g.kind, choices],
            )
            .map_err(|e| format!("写 live_hex_calendar_group 失败: {e}"))?;
        }
    }
    Ok(())
}

pub(crate) fn read_weekly(catalog: &Connection) -> Result<WeeklyBundle, String> {
    Ok(WeeklyBundle {
        archon_hunt: read_archon(catalog)?,
        deep_archimedea: read_archimedea(catalog, KIND_DEEP)?,
        temporal_archimedea: read_archimedea(catalog, KIND_TEMPORAL)?,
        steel_circuit: read_circuit(catalog, KIND_STEEL)?,
        normal_circuit: read_circuit(catalog, KIND_NORMAL)?,
        hex_calendar: read_hex(catalog)?,
    })
}

fn read_archon(catalog: &Connection) -> Result<Option<ArchonHuntApi>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry, boss, shard FROM live_archon_hunt WHERE id = ?1",
            [ARCHON_ID],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((activation, expiry, boss, shard)) = row else {
        return Ok(None);
    };
    let mut stmt = catalog
        .prepare(
            "SELECT mission_type, node FROM live_archon_mission
             WHERE hunt_id = ?1 ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let missions = stmt
        .query_map([ARCHON_ID], |r| {
            Ok(ArchonMissionApi {
                mission_type: r.get(0)?,
                node: r.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(Some(ArchonHuntApi {
        activation,
        expiry,
        boss,
        shard,
        missions,
    }))
}

fn read_archimedea(catalog: &Connection, kind: &str) -> Result<Option<ArchimedeaApi>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry, kind_label, personal_modifiers_json
             FROM live_archimedea WHERE kind = ?1",
            [kind],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((activation, expiry, kind_label, mods_json)) = row else {
        return Ok(None);
    };
    let personal_modifiers: Vec<LabeledTerm> =
        serde_json::from_str(&mods_json).unwrap_or_default();
    let mut stmt = catalog
        .prepare(
            "SELECT mission_type, faction, deviation_title, deviation_description,
                    risks_normal_json, risks_elite_json
             FROM live_archimedea_mission
             WHERE archimedea_kind = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let missions = stmt
        .query_map([kind], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(
            |(mission_type, faction, d_title, d_desc, risks_n, risks_e)| ArchimedeaMissionApi {
                mission_type,
                faction,
                deviation: LabeledTerm {
                    title: d_title,
                    description: d_desc,
                },
                risks_normal: serde_json::from_str(&risks_n).unwrap_or_default(),
                risks_elite: serde_json::from_str(&risks_e).unwrap_or_default(),
            },
        )
        .collect();
    Ok(Some(ArchimedeaApi {
        activation,
        expiry,
        kind_label,
        missions,
        personal_modifiers,
    }))
}

fn read_circuit(catalog: &Connection, kind: &str) -> Result<Option<CircuitApi>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry, this_week_json, next_week_json,
                    rotation_weeks_json, current_week_index
             FROM live_circuit WHERE kind = ?1",
            [kind],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<i32>>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((activation, expiry, this_w, next_w, rot, idx)) = row else {
        return Ok(None);
    };
    Ok(Some(CircuitApi {
        activation,
        expiry,
        this_week: serde_json::from_str(&this_w).unwrap_or_default(),
        next_week: serde_json::from_str(&next_w).unwrap_or_default(),
        rotation_weeks: serde_json::from_str(&rot).unwrap_or_default(),
        current_week_index: idx,
    }))
}

fn read_hex(catalog: &Connection) -> Result<Option<HexCalendarApi>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry FROM live_hex_calendar WHERE id = ?1",
            [HEX_ID],
            |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((activation, expiry)) = row else {
        return Ok(None);
    };
    let mut day_stmt = catalog
        .prepare(
            "SELECT id, day FROM live_hex_calendar_day
             WHERE calendar_id = ?1 ORDER BY day ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let day_rows = day_stmt
        .query_map([HEX_ID], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i32>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut days = Vec::new();
    for (day_id, day) in day_rows {
        let mut g_stmt = catalog
            .prepare(
                "SELECT kind, choices_json FROM live_hex_calendar_group
                 WHERE day_id = ?1 ORDER BY sort_order ASC, id ASC",
            )
            .map_err(|e| e.to_string())?;
        let groups = g_stmt
            .query_map([day_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(kind, choices_json)| HexCalendarGroupApi {
                kind,
                choices: serde_json::from_str(&choices_json).unwrap_or_default(),
            })
            .collect();
        days.push(HexCalendarDayApi { day, groups });
    }
    Ok(Some(HexCalendarApi {
        activation,
        expiry,
        days,
    }))
}
