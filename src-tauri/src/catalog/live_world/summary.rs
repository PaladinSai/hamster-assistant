//! 读库组装前端 brief。

use std::path::Path;

use chrono::Utc;
use rusqlite::OptionalExtension;

use crate::db::{self, open_catalog};
use crate::id::Snowflake;

use super::store_bounty;
use super::store_weekly;
use super::types::{
    ArchimedeaApi, ArchonHuntApi, BountyBriefApi, CircuitApi, FissureBriefDto, HexCalendarApi,
    LiveArchimedeaDto, LiveArchimedeaMissionDto, LiveArchonHuntDto, LiveArchonMissionDto,
    LiveCircuitDto, LiveDailyBrief, LiveFissureDto, LiveHexCalendarDayDto, LiveHexCalendarDto,
    LiveHexCalendarGroupDto, LiveNightwaveChallengeDto, LiveNightwaveSeasonDto, LiveSortieDto,
    LiveSortieMissionDto, LiveSteelIncursionDto, LiveSteelIncursionMissionDto, WeeklyBundle,
    FISSURE_KIND_NORMAL, FISSURE_KIND_STEEL, FISSURE_KIND_STORM, SETTING_NIGHTWAVE_SEASON,
    SORTIE_ROW_ID, STEEL_ROW_ID,
};

pub fn get_live_daily_brief(
    data_dir: &Path,
    _snowflake: &Snowflake,
) -> Result<LiveDailyBrief, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    read_brief(&catalog)
}

pub fn get_bounty_brief(
    data_dir: &Path,
    _snowflake: &Snowflake,
) -> Result<BountyBriefApi, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    read_bounty_brief(&catalog)
}

pub fn get_fissure_brief(
    data_dir: &Path,
    _snowflake: &Snowflake,
) -> Result<FissureBriefDto, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    read_fissures(&catalog)
}

fn read_fissures(catalog: &rusqlite::Connection) -> Result<FissureBriefDto, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT kind, tier, tier_zh, mission_type, node, activation, expiry
             FROM live_fissure
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(LiveFissureDto {
                kind: r.get(0)?,
                tier: r.get(1)?,
                tier_zh: r.get(2)?,
                mission_type: r.get(3)?,
                node: r.get(4)?,
                activation: r.get(5)?,
                expiry: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut brief = FissureBriefDto::default();
    let now = Utc::now().to_rfc3339();
    for row in rows {
        let mut f = row.map_err(|e| e.to_string())?;
        // 跳过已过期（仍保留未到点的）
        if let Some(ref exp) = f.expiry {
            if exp.as_str() < now.as_str() {
                continue;
            }
        }
        match f.kind.as_str() {
            FISSURE_KIND_STEEL => brief.steel.push(f),
            FISSURE_KIND_STORM => brief.storm.push(f),
            _ => {
                f.kind = FISSURE_KIND_NORMAL.into();
                brief.normal.push(f);
            }
        }
    }
    Ok(brief)
}

pub(crate) fn read_bounty_brief(
    catalog: &rusqlite::Connection,
) -> Result<BountyBriefApi, String> {
    store_bounty::read_bounties(catalog)
}

pub(crate) fn read_brief(catalog: &rusqlite::Connection) -> Result<LiveDailyBrief, String> {
    let sortie = read_sortie(catalog)?;
    let (daily, weekly_nw) = read_nightwave(catalog)?;
    let steel = read_steel(catalog)?;
    let weekly = read_weekly(catalog)?;
    Ok(LiveDailyBrief {
        sortie,
        nightwave_daily: daily,
        nightwave_weekly: weekly_nw,
        nightwave_season: read_nightwave_season(catalog)?,
        steel_incursion: steel,
        archon_hunt: weekly.archon_hunt.map(map_archon),
        deep_archimedea: weekly.deep_archimedea.map(map_archimedea),
        temporal_archimedea: weekly.temporal_archimedea.map(map_archimedea),
        steel_circuit: weekly.steel_circuit.map(map_circuit),
        normal_circuit: weekly.normal_circuit.map(map_circuit),
        hex_calendar: weekly.hex_calendar.map(map_hex),
    })
}

fn read_nightwave_season(
    catalog: &rusqlite::Connection,
) -> Result<Option<LiveNightwaveSeasonDto>, String> {
    let Some(json) = db::get_setting(catalog, SETTING_NIGHTWAVE_SEASON)? else {
        return Ok(None);
    };
    #[derive(serde::Deserialize)]
    struct Raw {
        activation: Option<String>,
        expiry: Option<String>,
    }
    let raw: Raw = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    if raw.activation.is_none() && raw.expiry.is_none() {
        return Ok(None);
    }
    Ok(Some(LiveNightwaveSeasonDto {
        activation: raw.activation,
        expiry: raw.expiry,
    }))
}

fn read_weekly(catalog: &rusqlite::Connection) -> Result<WeeklyBundle, String> {
    store_weekly::read_weekly(catalog)
}

fn map_archon(a: ArchonHuntApi) -> LiveArchonHuntDto {
    LiveArchonHuntDto {
        activation: a.activation,
        expiry: a.expiry,
        boss: a.boss,
        shard: a.shard,
        missions: a
            .missions
            .into_iter()
            .map(|m| LiveArchonMissionDto {
                mission_type: m.mission_type,
                node: m.node,
            })
            .collect(),
    }
}

fn map_archimedea(a: ArchimedeaApi) -> LiveArchimedeaDto {
    LiveArchimedeaDto {
        activation: a.activation,
        expiry: a.expiry,
        kind_label: a.kind_label,
        missions: a
            .missions
            .into_iter()
            .map(|m| LiveArchimedeaMissionDto {
                mission_type: m.mission_type,
                faction: m.faction,
                deviation: m.deviation,
                risks_normal: m.risks_normal,
                risks_elite: m.risks_elite,
            })
            .collect(),
        personal_modifiers: a.personal_modifiers,
    }
}

fn map_circuit(c: CircuitApi) -> LiveCircuitDto {
    LiveCircuitDto {
        activation: c.activation,
        expiry: c.expiry,
        this_week: c.this_week,
        next_week: c.next_week,
        rotation_weeks: c.rotation_weeks,
        current_week_index: c.current_week_index,
    }
}

fn map_hex(h: HexCalendarApi) -> LiveHexCalendarDto {
    LiveHexCalendarDto {
        activation: h.activation,
        expiry: h.expiry,
        days: h
            .days
            .into_iter()
            .map(|d| LiveHexCalendarDayDto {
                day: d.day,
                groups: d
                    .groups
                    .into_iter()
                    .map(|g| LiveHexCalendarGroupDto {
                        kind: g.kind,
                        choices: g.choices,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn read_sortie(catalog: &rusqlite::Connection) -> Result<Option<LiveSortieDto>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry, boss, faction FROM live_sortie WHERE id = ?1",
            [SORTIE_ROW_ID],
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
    let Some((activation, expiry, boss, faction)) = row else {
        return Ok(None);
    };
    let mut stmt = catalog
        .prepare(
            "SELECT sort_order, mission_type, modifier, node
             FROM live_sortie_mission WHERE sortie_id = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let missions = stmt
        .query_map([SORTIE_ROW_ID], |r| {
            Ok(LiveSortieMissionDto {
                sort_order: r.get(0)?,
                mission_type: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                modifier: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                node: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(Some(LiveSortieDto {
        activation,
        expiry,
        boss,
        faction,
        missions,
    }))
}

fn read_nightwave(
    catalog: &rusqlite::Connection,
) -> Result<(Vec<LiveNightwaveChallengeDto>, Vec<LiveNightwaveChallengeDto>), String> {
    let now = Utc::now();
    let mut stmt = catalog
        .prepare(
            "SELECT title, description, is_daily, is_elite, activation, expiry, reputation
             FROM live_nightwave_challenge
             ORDER BY is_daily DESC, is_elite ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, i32>(2)?,
                r.get::<_, i32>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, Option<i32>>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut daily = Vec::new();
    let mut weekly = Vec::new();
    for r in rows {
        let (title, description, is_daily, is_elite, activation, expiry, reputation) =
            r.map_err(|e| e.to_string())?;
        // 过滤已过期
        if let Some(ref exp) = expiry {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(exp) {
                if now >= dt.with_timezone(&Utc) {
                    continue;
                }
            }
        }
        let dto = LiveNightwaveChallengeDto {
            title,
            description,
            is_elite: is_elite != 0,
            activation,
            expiry,
            reputation,
        };
        if is_daily != 0 {
            daily.push(dto);
        } else {
            weekly.push(dto);
        }
    }
    Ok((daily, weekly))
}

fn read_steel(catalog: &rusqlite::Connection) -> Result<Option<LiveSteelIncursionDto>, String> {
    let row = catalog
        .query_row(
            "SELECT activation, expiry,
                    COALESCE(today_json, '[]'), COALESCE(tomorrow_json, '[]')
             FROM live_steel_incursion WHERE id = ?1",
            [STEEL_ROW_ID],
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
    Ok(row.map(|(activation, expiry, today_json, tomorrow_json)| {
        let today: Vec<LiveSteelIncursionMissionDto> =
            serde_json::from_str(&today_json).unwrap_or_default();
        let tomorrow: Vec<LiveSteelIncursionMissionDto> =
            serde_json::from_str(&tomorrow_json).unwrap_or_default();
        LiveSteelIncursionDto {
            activation,
            expiry,
            today,
            tomorrow,
        }
    }))
}
