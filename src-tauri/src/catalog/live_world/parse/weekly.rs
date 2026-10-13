//! 执政官 / 科研 / 回廊 / 六人组等周常包 parse。

use super::de::*;
use crate::catalog::live_world::localize_weekly;
use crate::catalog::live_world::types::{
    ArchimedeaApi, ArchimedeaMissionApi, ArchonHuntApi, ArchonMissionApi, CircuitApi,
    HexCalendarApi, HexCalendarDayApi, HexCalendarGroupApi, WeeklyBundle,
};

pub(super) fn parse_weekly(
    state: &DeWorldState,
    challenge_i18n: Option<&crate::catalog::live_world::challenge_i18n::ChallengeLocalizer>,
    conquest_lang: Option<&crate::catalog::live_world::conquest_i18n::ConquestLang>,
) -> WeeklyBundle {
    WeeklyBundle {
        archon_hunt: parse_archon(state.lite_sorties.first()),
        deep_archimedea: state
            .conquests
            .iter()
            .find(|c| c.kind.as_deref() == Some("CT_LAB"))
            .and_then(|c| parse_archimedea(c, conquest_lang)),
        temporal_archimedea: state
            .conquests
            .iter()
            .find(|c| c.kind.as_deref() == Some("CT_HEX"))
            .and_then(|c| parse_archimedea(c, conquest_lang)),
        steel_circuit: parse_circuit(state.endless_xp.first(), "EXC_HARD"),
        normal_circuit: parse_circuit(state.endless_xp.first(), "EXC_NORMAL"),
        hex_calendar: parse_calendar(state.calendars.first(), challenge_i18n),
    }
}

pub(super) fn parse_archon(raw: Option<&DeLiteSortie>) -> Option<ArchonHuntApi> {
    let raw = raw?;
    let boss_code = raw.boss.as_deref().unwrap_or("");
    let missions = raw
        .missions
        .iter()
        .map(|m| {
            let mt = m.mission_type.as_deref().unwrap_or("");
            let node = m.node.as_deref().unwrap_or("");
            let (mission_type, node) = localize_weekly::mission_and_node(mt, node);
            ArchonMissionApi { mission_type, node }
        })
        .collect();
    Some(ArchonHuntApi {
        activation: raw.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: raw.expiry.as_ref().and_then(de_date_to_rfc3339),
        boss: localize_weekly::archon_boss_en(boss_code),
        shard: localize_weekly::archon_shard_en(boss_code),
        missions,
    })
}

pub(super) fn parse_archimedea(
    raw: &DeConquest,
    conquest_lang: Option<&crate::catalog::live_world::conquest_i18n::ConquestLang>,
) -> Option<ArchimedeaApi> {
    let kind = raw.kind.as_deref().unwrap_or("");
    let missions = raw
        .missions
        .iter()
        .map(|m| {
            let normal = m
                .difficulties
                .iter()
                .find(|d| d.kind.as_deref() == Some("CD_NORMAL"));
            let hard = m
                .difficulties
                .iter()
                .find(|d| d.kind.as_deref() == Some("CD_HARD"));
            let deviation = hard
                .or(normal)
                .and_then(|d| d.deviation.as_deref())
                .unwrap_or("");
            let risks_normal: Vec<_> = normal
                .map(|d| {
                    d.risks
                        .iter()
                        .map(|r| localize_weekly::conquest_risk(r, conquest_lang))
                        .collect()
                })
                .unwrap_or_default();
            let risks_elite: Vec<_> = hard
                .map(|d| {
                    let all: Vec<_> = d
                        .risks
                        .iter()
                        .map(|r| localize_weekly::conquest_risk(r, conquest_lang))
                        .collect();
                    all.into_iter()
                        .filter(|r| !risks_normal.iter().any(|n| n.title == r.title))
                        .collect()
                })
                .unwrap_or_default();
            ArchimedeaMissionApi {
                mission_type: localize_weekly::archimedea_mission_type_en(
                    kind,
                    m.mission_type.as_deref().unwrap_or(""),
                ),
                faction: localize_weekly::conquest_faction(m.faction.as_deref().unwrap_or("")),
                deviation: localize_weekly::conquest_deviation(kind, deviation, conquest_lang),
                risks_normal,
                risks_elite,
            }
        })
        .collect();
    Some(ArchimedeaApi {
        activation: raw.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: raw.expiry.as_ref().and_then(de_date_to_rfc3339),
        kind_label: localize_weekly::archimedea_kind(kind),
        missions,
        personal_modifiers: raw
            .variables
            .iter()
            .map(|v| localize_weekly::conquest_personal_mod(v, conquest_lang))
            .collect(),
    })
}

pub(super) fn parse_circuit(raw: Option<&DeEndlessXp>, category: &str) -> Option<CircuitApi> {
    let raw = raw?;
    let choices = raw
        .categories
        .iter()
        .find(|c| c.category.as_deref() == Some(category))?
        .choices
        .clone();
    if choices.is_empty() {
        return None;
    }
    let this_week: Vec<String> = choices
        .iter()
        .map(|c| localize_weekly::display_circuit_choice(c))
        .collect();
    let rotation = match category {
        "EXC_HARD" => localize_weekly::steel_circuit_rotation(),
        "EXC_NORMAL" => localize_weekly::normal_circuit_rotation(),
        _ => &[][..],
    };
    let current_week_index = localize_weekly::match_circuit_week_index(&choices, rotation)
        .map(|i| i as i32);
    let next_week = localize_weekly::next_circuit_week(&choices, rotation);
    let rotation_weeks: Vec<Vec<String>> = rotation
        .iter()
        .map(|w| localize_weekly::display_rotation_week(w))
        .collect();
    Some(CircuitApi {
        activation: raw.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: raw.expiry.as_ref().and_then(de_date_to_rfc3339),
        this_week,
        next_week,
        rotation_weeks,
        current_week_index,
    })
}

pub(super) fn parse_calendar(
    raw: Option<&DeCalendarSeason>,
    challenge_i18n: Option<&crate::catalog::live_world::challenge_i18n::ChallengeLocalizer>,
) -> Option<HexCalendarApi> {
    let raw = raw?;
    let mut days_out = Vec::new();
    for day in &raw.days {
        let day_n = day.day.unwrap_or(0);
        let mut groups: Vec<HexCalendarGroupApi> = Vec::new();
        for ev in &day.events {
            let kind_code = ev.kind.as_deref().unwrap_or("");
            let path = ev
                .challenge
                .as_deref()
                .or(ev.reward.as_deref())
                .or(ev.upgrade.as_deref())
                .unwrap_or("");
            if path.is_empty() {
                continue;
            }
            let kind = localize_weekly::calendar_event_kind(kind_code);
            let (title, description) =
                localize_weekly::calendar_path_title(path, challenge_i18n);
            let choice = crate::catalog::live_world::types::LabeledTerm { title, description };
            if let Some(last) = groups.last_mut() {
                if last.kind == kind {
                    last.choices.push(choice);
                    continue;
                }
            }
            groups.push(HexCalendarGroupApi {
                kind,
                choices: vec![choice],
            });
        }
        if !groups.is_empty() {
            days_out.push(HexCalendarDayApi {
                day: day_n,
                groups,
            });
        }
    }
    if days_out.is_empty() {
        return None;
    }
    Some(HexCalendarApi {
        activation: raw.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: raw.expiry.as_ref().and_then(de_date_to_rfc3339),
        days: days_out,
    })
}

