//! 开放世界循环：夜灵 / 福尔图娜 / 魔胎 / 双衍 / 地球。
//! 算法对齐 WFCD warframe-worldstate-parser（Cetus 依赖 Ostron 赏金到期）。

use std::path::Path;

use chrono::{DateTime, Timelike, TimeZone, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, open_catalog};

use super::types::SETTING_PLAINS_EXPIRY;

const NIGHT_SECS: i64 = 3_000; // Cetus 夜晚 50 分钟
const EARTH_CYCLE_SECS: i64 = 28_800; // 8 小时
const EARTH_DAY_SECS: i64 = 14_400; // 4 小时
const VALLIS_LOOP_MS: i64 = 1_600_000; // 26m40s
const VALLIS_COLD_MS: i64 = 1_200_000; // 20m
/// WFCD VallisCycle 校准点
const VALLIS_EPOCH_MS: i64 = 1_770_230_808_000; // 2026-02-04T19:46:48Z
const DUVIRI_CYCLE_SECS: i64 = 36_000; // 10 小时
const DUVIRI_STATE_SECS: i64 = 7_200; // 2 小时
const DUVIRI_STATES: &[&str] = &["sorrow", "fear", "joy", "anger", "envy"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlainsCycleDto {
    pub id: String,
    pub name_zh: String,
    pub state: String,
    pub state_zh: String,
    pub next_state_zh: String,
    pub activation: String,
    pub expiry: String,
    /// 双衍：是否有 Kullervo 岛
    pub note_zh: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlainsBriefDto {
    pub cycles: Vec<PlainsCycleDto>,
}

pub fn get_plains_brief(data_dir: &Path) -> Result<PlainsBriefDto, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    let brief = build_brief(&catalog, Utc::now())?;
    let _ = write_expiry_setting(&catalog, &brief);
    Ok(brief)
}

pub(crate) fn sync_expiry_to_catalog(
    _data_dir: &Path,
    catalog: &Connection,
) -> Result<(), String> {
    let brief = build_brief(catalog, Utc::now())?;
    write_expiry_setting(catalog, &brief)
}

fn write_expiry_setting(catalog: &Connection, brief: &PlainsBriefDto) -> Result<(), String> {
    let mut earliest: Option<&str> = None;
    for c in &brief.cycles {
        if c.expiry.is_empty() {
            continue;
        }
        match earliest {
            None => earliest = Some(c.expiry.as_str()),
            Some(prev) if c.expiry.as_str() < prev => earliest = Some(c.expiry.as_str()),
            _ => {}
        }
    }
    match earliest {
        Some(ts) => db::set_setting(catalog, SETTING_PLAINS_EXPIRY, ts),
        None => Ok(()),
    }
}

fn build_brief(catalog: &Connection, now: DateTime<Utc>) -> Result<PlainsBriefDto, String> {
    let cetus = cetus_cycle(catalog, now);
    let cambion = cetus.as_ref().map(|c| PlainsCycleDto {
        id: "cambion".into(),
        name_zh: "魔胎之境".into(),
        state: if c.state == "day" {
            "fass".into()
        } else {
            "vome".into()
        },
        state_zh: if c.state == "day" {
            "混乱 Fass".into()
        } else {
            "Vome".into()
        },
        next_state_zh: if c.state == "day" {
            "Vome".into()
        } else {
            "混乱 Fass".into()
        },
        activation: c.activation.clone(),
        expiry: c.expiry.clone(),
        note_zh: None,
    });

    let mut cycles = Vec::with_capacity(5);
    if let Some(c) = cetus {
        cycles.push(c);
    }
    cycles.push(vallis_cycle(now));
    if let Some(c) = cambion {
        cycles.push(c);
    }
    cycles.push(duviri_cycle(now));
    cycles.push(earth_cycle(now));
    Ok(PlainsBriefDto { cycles })
}

fn cetus_cycle(catalog: &Connection, now: DateTime<Utc>) -> Option<PlainsCycleDto> {
    let expiry_s: Option<String> = catalog
        .query_row(
            "SELECT expiry FROM live_bounty_board WHERE tag = 'CetusSyndicate'",
            [],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let expiry_s = expiry_s.filter(|s| !s.is_empty())?;
    let bounty_end = DateTime::parse_from_rfc3339(&expiry_s)
        .ok()?
        .with_timezone(&Utc);
    // 对齐整分（与 WFCD 一致）
    let bounty_end = bounty_end
        .with_second(0)
        .and_then(|t| Timelike::with_nanosecond(&t, 0))
        .unwrap_or(bounty_end);

    let millis_left = (bounty_end - now).num_milliseconds();
    if millis_left <= 0 {
        return None;
    }
    let seconds_to_night_end = millis_left / 1000;
    let is_day = seconds_to_night_end > NIGHT_SECS;
    let phase_ms = if is_day {
        (seconds_to_night_end - NIGHT_SECS) * 1000
    } else {
        seconds_to_night_end * 1000
    };
    let expiry = now + chrono::Duration::milliseconds(phase_ms);
    let state_ms = if is_day {
        6_000_000i64
    } else {
        3_000_000i64
    };
    let activation = expiry - chrono::Duration::milliseconds(state_ms);

    Some(PlainsCycleDto {
        id: "cetus".into(),
        name_zh: "夜灵平原".into(),
        state: if is_day { "day" } else { "night" }.into(),
        state_zh: if is_day { "白昼" } else { "夜晚" }.into(),
        next_state_zh: if is_day { "夜晚" } else { "白昼" }.into(),
        activation: activation.to_rfc3339(),
        expiry: expiry.to_rfc3339(),
        note_zh: None,
    })
}

fn earth_cycle(now: DateTime<Utc>) -> PlainsCycleDto {
    let cycle_seconds = now.timestamp() % EARTH_CYCLE_SECS;
    let is_day = cycle_seconds < EARTH_DAY_SECS;
    let seconds_left = EARTH_DAY_SECS - (cycle_seconds % EARTH_DAY_SECS);
    let expiry = now + chrono::Duration::seconds(seconds_left);
    let activation = expiry - chrono::Duration::seconds(EARTH_DAY_SECS);
    PlainsCycleDto {
        id: "earth".into(),
        name_zh: "地球".into(),
        state: if is_day { "day" } else { "night" }.into(),
        state_zh: if is_day { "白昼" } else { "夜晚" }.into(),
        next_state_zh: if is_day { "夜晚" } else { "白昼" }.into(),
        activation: activation.to_rfc3339(),
        expiry: expiry.to_rfc3339(),
        note_zh: None,
    }
}

fn vallis_cycle(now: DateTime<Utc>) -> PlainsCycleDto {
    let since_last = (now.timestamp_millis() - VALLIS_EPOCH_MS).rem_euclid(VALLIS_LOOP_MS);
    let to_next_full = VALLIS_LOOP_MS - since_last;
    let is_warm = to_next_full > VALLIS_COLD_MS;
    let to_next = if to_next_full < VALLIS_COLD_MS {
        to_next_full
    } else {
        to_next_full - VALLIS_COLD_MS
    };
    let expiry_ms = now.timestamp_millis() + to_next;
    let prev_ms = now.timestamp_millis() + to_next_full
        - if is_warm {
            VALLIS_LOOP_MS
        } else {
            VALLIS_COLD_MS
        };
    let expiry = Utc
        .timestamp_millis_opt(expiry_ms)
        .single()
        .unwrap_or(now);
    let activation = Utc
        .timestamp_millis_opt(prev_ms)
        .single()
        .unwrap_or(now);
    PlainsCycleDto {
        id: "vallis".into(),
        name_zh: "福尔图娜".into(),
        state: if is_warm { "warm" } else { "cold" }.into(),
        state_zh: if is_warm { "温暖" } else { "寒冷" }.into(),
        next_state_zh: if is_warm { "寒冷" } else { "温暖" }.into(),
        activation: activation.to_rfc3339(),
        expiry: expiry.to_rfc3339(),
        note_zh: None,
    }
}

fn duviri_cycle(now: DateTime<Utc>) -> PlainsCycleDto {
    let cycle_delta = (now.timestamp() - 52).rem_euclid(DUVIRI_CYCLE_SECS);
    let state_ind = (cycle_delta / DUVIRI_STATE_SECS) as usize;
    let state_delta = cycle_delta % DUVIRI_STATE_SECS;
    let until_next = DUVIRI_STATE_SECS - state_delta;
    let state = DUVIRI_STATES[state_ind.min(DUVIRI_STATES.len() - 1)];
    let next = DUVIRI_STATES[(state_ind + 1) % DUVIRI_STATES.len()];
    let expiry = now + chrono::Duration::seconds(until_next);
    let activation = expiry - chrono::Duration::seconds(DUVIRI_STATE_SECS);
    let kullervo = matches!(state, "anger" | "sorrow" | "fear");
    PlainsCycleDto {
        id: "duviri".into(),
        name_zh: "双衍王境".into(),
        state: state.into(),
        state_zh: duviri_state_zh(state).into(),
        next_state_zh: duviri_state_zh(next).into(),
        activation: activation.to_rfc3339(),
        expiry: expiry.to_rfc3339(),
        note_zh: Some(if kullervo {
            "在 Kullervo".into()
        } else {
            "不在 Kullervo".into()
        }),
    }
}

fn duviri_state_zh(state: &str) -> String {
    match state {
        "joy" => "喜悦".into(),
        "anger" => "愤怒".into(),
        "envy" => "嫉妒".into(),
        "sorrow" => "悲伤".into(),
        "fear" => "恐惧".into(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earth_day_night_alternates() {
        // 对齐到周期起点附近
        let t0 = Utc.timestamp_opt(0, 0).single().unwrap();
        let day = earth_cycle(t0);
        assert_eq!(day.state, "day");
        assert_eq!(day.state_zh, "白昼");
        let night_at = t0 + chrono::Duration::seconds(EARTH_DAY_SECS + 10);
        let night = earth_cycle(night_at);
        assert_eq!(night.state, "night");
    }

    #[test]
    fn duviri_kullervo_on_anger_sorrow_fear() {
        // 找一个 anger 档：cycle_delta 落在 3 * 7200 .. 4 * 7200
        // (ts - 52) % 36000 ∈ [21600, 28800)
        let ts = 52 + 21_600 + 60;
        let now = Utc.timestamp_opt(ts, 0).single().unwrap();
        let d = duviri_cycle(now);
        assert_eq!(d.state, "anger");
        assert_eq!(d.note_zh.as_deref(), Some("在 Kullervo"));
    }

    #[test]
    fn vallis_epoch_produces_state() {
        let now = Utc
            .timestamp_millis_opt(VALLIS_EPOCH_MS + 100_000)
            .single()
            .unwrap();
        let v = vallis_cycle(now);
        assert!(v.state == "warm" || v.state == "cold");
        assert!(!v.state_zh.is_empty());
    }
}
