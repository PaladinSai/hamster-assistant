//! 写入 catalog 日常动态表。

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::catalog::i18n::ZhLookup;
use crate::id::Snowflake;

use super::localize;
use super::store_bounty;
use super::store_weekly;
use super::types::{LiveWorldFetch, SETTING_NIGHTWAVE_SEASON, SORTIE_ROW_ID, STEEL_ROW_ID};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ApplyScope {
    /// 突击 / 侵袭 / 周常包（不含电波）
    Routines,
    /// 午夜电波挑战与赛季
    Nightwave,
    /// 全量
    All,
}

pub(crate) fn apply_live_fetch(
    catalog: &Connection,
    snowflake: &Snowflake,
    fetch: &LiveWorldFetch,
) -> Result<(), String> {
    apply_live_fetch_scoped(catalog, snowflake, fetch, ApplyScope::All)
}

pub(crate) fn apply_live_fetch_scoped(
    catalog: &Connection,
    snowflake: &Snowflake,
    fetch: &LiveWorldFetch,
    scope: ApplyScope,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339();
    let tx = catalog
        .unchecked_transaction()
        .map_err(|e| format!("开事务失败: {e}"))?;

    let do_routines = matches!(scope, ApplyScope::Routines | ApplyScope::All);
    let do_nightwave = matches!(scope, ApplyScope::Nightwave | ApplyScope::All);

    if do_routines {
        tx.execute("DELETE FROM live_sortie_mission", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM live_sortie", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM live_steel_incursion", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM live_fissure", [])
            .map_err(|e| e.to_string())?;

        let s = &fetch.sortie;
        tx.execute(
            "INSERT INTO live_sortie (id, activation, expiry, boss, faction, fetched_at, source_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                SORTIE_ROW_ID,
                s.activation,
                s.expiry,
                s.boss.as_deref().unwrap_or(""),
                s.faction.as_deref().unwrap_or(""),
                now,
                fetch.source_url,
            ],
        )
        .map_err(|e| format!("写 live_sortie 失败: {e}"))?;

        let zh = ZhLookup::load(&tx).ok();
        for (i, v) in s.variants.iter().enumerate() {
            let mid = snowflake.next_id();
            let mt_raw = v.mission_type.as_deref().unwrap_or("");
            let mission_type = localize::resolve_mission_type(zh.as_ref(), mt_raw);
            let node = localize::apply_place_zh(v.node.as_deref().unwrap_or(""), zh.as_ref());
            tx.execute(
                "INSERT INTO live_sortie_mission
                 (id, sortie_id, sort_order, mission_type, modifier, node)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    mid,
                    SORTIE_ROW_ID,
                    i as i32,
                    mission_type,
                    v.modifier.as_deref().unwrap_or(""),
                    node,
                ],
            )
            .map_err(|e| format!("写 live_sortie_mission 失败: {e}"))?;
        }

        let inc = fetch.steel.incursions.as_ref();
        let today_json = serde_json::to_string(inc.map(|i| i.today.as_slice()).unwrap_or(&[]))
            .unwrap_or_else(|_| "[]".into());
        let tomorrow_json =
            serde_json::to_string(inc.map(|i| i.tomorrow.as_slice()).unwrap_or(&[]))
                .unwrap_or_else(|_| "[]".into());
        let steel_source = if inc.map(|i| !i.today.is_empty()).unwrap_or(false) {
            super::steel_incursion::SP_INCURSIONS_URL
        } else {
            fetch.source_url.as_str()
        };
        tx.execute(
            "INSERT INTO live_steel_incursion
             (id, activation, expiry, fetched_at, source_url, today_json, tomorrow_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                STEEL_ROW_ID,
                inc.and_then(|i| i.activation.clone()),
                inc.and_then(|i| i.expiry.clone()),
                now,
                steel_source,
                today_json,
                tomorrow_json,
            ],
        )
        .map_err(|e| format!("写 live_steel_incursion 失败: {e}"))?;

        for (i, f) in fetch.fissures.iter().enumerate() {
            let mid = snowflake.next_id();
            tx.execute(
                "INSERT INTO live_fissure
                 (id, kind, tier, tier_zh, mission_type, node, activation, expiry, sort_order, fetched_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    mid,
                    f.kind,
                    f.tier,
                    f.tier_zh,
                    f.mission_type,
                    f.node,
                    f.activation,
                    f.expiry,
                    i as i32,
                    now,
                ],
            )
            .map_err(|e| format!("写 live_fissure 失败: {e}"))?;
        }

        let mut weekly = fetch.weekly.clone();
        localize_weekly_bundle(&mut weekly, zh.as_ref());
        store_weekly::replace_weekly(&tx, snowflake, &weekly)?;
        store_bounty::replace_bounties(&tx, snowflake, &fetch.bounties)?;
    }

    if do_nightwave {
        tx.execute("DELETE FROM live_nightwave_challenge", [])
            .map_err(|e| e.to_string())?;
        for c in &fetch.nightwave.active_challenges {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO live_nightwave_challenge
                 (id, challenge_key, activation, expiry, is_daily, is_elite, title, description, reputation, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    c.id,
                    c.activation,
                    c.expiry,
                    if c.is_daily { 1 } else { 0 },
                    if c.is_elite { 1 } else { 0 },
                    c.title.as_deref().unwrap_or(""),
                    c.desc.as_deref().unwrap_or(""),
                    c.reputation,
                    now,
                ],
            )
            .map_err(|e| format!("写 live_nightwave_challenge 失败: {e}"))?;
        }

        let season_json = serde_json::json!({
            "activation": fetch.nightwave.activation,
            "expiry": fetch.nightwave.expiry,
        })
        .to_string();
        crate::db::set_setting(&tx, SETTING_NIGHTWAVE_SEASON, &season_json)?;
    }

    tx.commit().map_err(|e| format!("提交日常动态失败: {e}"))?;
    Ok(())
}

fn localize_weekly_bundle(weekly: &mut super::types::WeeklyBundle, zh: Option<&ZhLookup>) {
    use super::localize_weekly;

    if let Some(archon) = weekly.archon_hunt.as_mut() {
        let boss_fb = localize_weekly::archon_boss_fallback_zh(&archon.boss);
        archon.boss = localize::resolve_zh(zh, &archon.boss, &boss_fb);
        let shard_fb = localize_weekly::archon_shard_fallback_zh(&archon.shard);
        archon.shard = localize::resolve_zh(zh, &archon.shard, &shard_fb);
        for m in &mut archon.missions {
            m.mission_type = localize::resolve_mission_type(zh, &m.mission_type);
            m.node = localize::apply_place_zh(&m.node, zh);
        }
    }
    for arch in [
        weekly.deep_archimedea.as_mut(),
        weekly.temporal_archimedea.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        for m in &mut arch.missions {
            m.mission_type = localize::resolve_mission_type(zh, &m.mission_type);
        }
    }
    for circuit in [
        weekly.steel_circuit.as_mut(),
        weekly.normal_circuit.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        for name in circuit
            .this_week
            .iter_mut()
            .chain(circuit.next_week.iter_mut())
            .chain(circuit.rotation_weeks.iter_mut().flatten())
        {
            if let Some(z) = zh.and_then(|lookup| lookup.get(name)) {
                *name = z;
            }
        }
    }
    if let Some(cal) = weekly.hex_calendar.as_mut() {
        for day in &mut cal.days {
            for g in &mut day.groups {
                // 增益标题保留英文 humanize，供前端悬停；勿走 ZhLookup 以免被翻成笼统中文
                if g.kind == "upgrade" {
                    continue;
                }
                for c in &mut g.choices {
                    let fb = localize_weekly::calendar_title_fallback_zh(&c.title);
                    c.title = localize::resolve_zh(zh, &c.title, &fb);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::live_world::types::{
        NightwaveApi, NightwaveChallengeApi, SortieApi, SortieVariantApi, SteelIncursionApi,
        SteelPathApi, WeeklyBundle,
    };

    #[test]
    fn apply_writes_rows() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../../sql/catalog_v1.sql"))
            .unwrap();
        let snow = Snowflake::new(1);
        let fetch = LiveWorldFetch {
            sortie: SortieApi {
                activation: Some("2026-09-29T16:00:00.000Z".into()),
                expiry: Some("2026-09-30T16:00:00.000Z".into()),
                boss: Some("Captain Vor".into()),
                faction: Some("Grineer".into()),
                variants: vec![SortieVariantApi {
                    mission_type: Some("Defense".into()),
                    modifier: Some("Heat".into()),
                    node: Some("Casta · 谷神星".into()),
                }],
            },
            nightwave: NightwaveApi {
                activation: None,
                expiry: None,
                active_challenges: vec![NightwaveChallengeApi {
                    id: "daily1".into(),
                    activation: None,
                    expiry: None,
                    is_daily: true,
                    is_elite: false,
                    title: Some("Researcher".into()),
                    desc: Some("Scan 25".into()),
                    reputation: Some(1000),
                }],
            },
            steel: SteelPathApi {
                incursions: Some(SteelIncursionApi {
                    activation: Some("2026-09-30T00:00:00.000Z".into()),
                    expiry: Some("2026-09-30T23:59:59.000Z".into()),
                    today: Vec::new(),
                    tomorrow: Vec::new(),
                }),
            },
            weekly: WeeklyBundle::default(),
            bounties: Default::default(),
            fissures: vec![],
            source_url: "test".into(),
        };
        apply_live_fetch(&conn, &snow, &fetch).unwrap();
        let missions: i32 = conn
            .query_row("SELECT COUNT(*) FROM live_sortie_mission", [], |r| r.get(0))
            .unwrap();
        assert_eq!(missions, 1);
        let nw: i32 = conn
            .query_row("SELECT COUNT(*) FROM live_nightwave_challenge", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(nw, 1);
        let steel: i32 = conn
            .query_row("SELECT COUNT(*) FROM live_steel_incursion", [], |r| r.get(0))
            .unwrap();
        assert_eq!(steel, 1);
        let boards: i32 = conn
            .query_row("SELECT COUNT(*) FROM live_bounty_board", [], |r| r.get(0))
            .unwrap();
        // Default bounties empty → no boards written
        assert_eq!(boards, 0);
    }

    #[test]
    fn apply_resolves_archon_via_name_i18n() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../../sql/catalog_v1.sql"))
            .unwrap();
        conn.execute(
            "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES
             ('Archon Boreal', '执刑官诡文枭主'),
             ('Azure Archon Shard', '蔚蓝执刑官源力石'),
             ('Disruption', '中断'),
             ('Sabotage', '破坏')",
            [],
        )
        .unwrap();
        let snow = Snowflake::new(1);
        let mut weekly = WeeklyBundle::default();
        weekly.archon_hunt = Some(crate::catalog::live_world::types::ArchonHuntApi {
            activation: None,
            expiry: None,
            boss: "Archon Boreal".into(),
            shard: "Azure Archon Shard".into(),
            missions: vec![
                crate::catalog::live_world::types::ArchonMissionApi {
                    mission_type: "Sabotage".into(),
                    node: "Node".into(),
                },
                crate::catalog::live_world::types::ArchonMissionApi {
                    mission_type: "Disruption".into(),
                    node: "Node2".into(),
                },
            ],
        });
        let fetch = LiveWorldFetch {
            sortie: SortieApi {
                activation: None,
                expiry: None,
                boss: Some("Vor".into()),
                faction: Some("Grineer".into()),
                variants: vec![SortieVariantApi {
                    mission_type: Some("Disruption".into()),
                    modifier: Some("Heat".into()),
                    node: Some("Casta · 谷神星".into()),
                }],
            },
            nightwave: NightwaveApi {
                activation: None,
                expiry: None,
                active_challenges: vec![],
            },
            steel: SteelPathApi { incursions: None },
            weekly,
            bounties: Default::default(),
            fissures: vec![],
            source_url: "test".into(),
        };
        apply_live_fetch(&conn, &snow, &fetch).unwrap();
        let mt: String = conn
            .query_row(
                "SELECT mission_type FROM live_sortie_mission LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mt, "中断");
        let boss: String = conn
            .query_row(
                "SELECT boss FROM live_archon_hunt WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(boss, "执刑官诡文枭主");
        let shard: String = conn
            .query_row(
                "SELECT shard FROM live_archon_hunt WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(shard, "蔚蓝执刑官源力石");
        let archon_mt: String = conn
            .query_row(
                "SELECT mission_type FROM live_archon_mission
                 WHERE hunt_id = 1 ORDER BY sort_order DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(archon_mt, "中断");
    }
}
