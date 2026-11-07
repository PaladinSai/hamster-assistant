//! WFCD warframe-drop-data → relic_location（星图任务 + 赏金）

use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection};
use serde_json::Value;

use crate::catalog::ensure_mission_type_i18n;
use crate::catalog::i18n::ZhLookup;
use crate::catalog::wfcd::{fetch_json_url, FetchResult};
use crate::db::{self, open_catalog, open_user};
use crate::id::Snowflake;

const SETTING_ETAG: &str = "catalog_drop_locations_etag";
const SETTING_CHECKED_AT: &str = "catalog_drop_locations_checked_at";
/// 与目录月检一致：有本地来源且未满间隔则不访问网络。
const CHECK_INTERVAL: Duration = Duration::days(30);

pub const DEFAULT_DROP_ALL_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-drop-data/gh-pages/data/all.json";

const BOUNTY_KEYS: &[(&str, &str)] = &[
    ("cetusBountyRewards", "Cetus"),
    ("solarisBountyRewards", "Fortuna"),
    ("deimosRewards", "Deimos"),
    ("zarimanRewards", "Zariman"),
    ("hexRewards", "Hex"),
    ("entratiLabRewards", "EntratiLab"),
];

#[derive(Debug, Default, Clone)]
pub struct DropLocationStats {
    pub locations: usize,
    pub mission: usize,
    pub bounty: usize,
}

#[derive(Debug, Clone)]
struct ParsedLocation {
    relic_key: String,
    source_kind: &'static str,
    planet_or_hub: String,
    node_or_bounty: String,
    game_mode: Option<String>,
    rotation: Option<String>,
    chance: Option<f64>,
    stage: Option<String>,
    is_event: bool,
}

pub fn fetch_drop_all(url: &str, if_none_match: Option<&str>) -> Result<FetchResult, String> {
    fetch_json_url(url, if_none_match, 120)
}

/// 「Lith Q3 Relic」→「Lith Q3」
pub fn relic_key_from_drop_item(item_name: &str) -> Option<String> {
    let name = item_name.trim();
    let base = name.strip_suffix(" Relic").unwrap_or(name);
    let mut parts = base.split_whitespace();
    let tier = parts.next()?;
    let code = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    match tier {
        "Lith" | "Meso" | "Neo" | "Axi" => Some(format!("{tier} {code}")),
        _ => None,
    }
}

fn parse_drop_locations(bytes: &[u8]) -> Result<Vec<ParsedLocation>, String> {
    let root: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("drop-data JSON 解析失败: {e}"))?;
    let mut out = Vec::new();

    let missions = root
        .get("missionRewards")
        .cloned()
        .or_else(|| {
            if root.as_object().is_some_and(|o| {
                o.keys().any(|k| {
                    matches!(
                        k.as_str(),
                        "Earth" | "Mercury" | "Venus" | "Void" | "Lua" | "Eris"
                    )
                })
            }) {
                Some(root.clone())
            } else {
                None
            }
        });

    if let Some(missions) = missions {
        parse_mission_rewards(&missions, &mut out);
    }

    for &(key, hub) in BOUNTY_KEYS {
        if let Some(arr) = root.get(key).and_then(|v| v.as_array()) {
            parse_bounty_array(arr, hub, &mut out);
        }
    }

    Ok(out)
}

fn parse_mission_rewards(missions: &Value, out: &mut Vec<ParsedLocation>) {
    let Some(planets) = missions.as_object() else {
        return;
    };
    for (planet, nodes) in planets {
        let Some(nodes) = nodes.as_object() else {
            continue;
        };
        for (node, payload) in nodes {
            let game_mode = payload
                .get("gameMode")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let is_event = payload
                .get("isEvent")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let rewards = payload.get("rewards").unwrap_or(payload);
            collect_reward_rotations(
                rewards,
                "mission",
                planet,
                node,
                game_mode.as_deref(),
                is_event,
                out,
            );
        }
    }
}

fn parse_bounty_array(arr: &[Value], hub: &str, out: &mut Vec<ParsedLocation>) {
    for entry in arr {
        let bounty = entry
            .get("bountyLevel")
            .and_then(|v| v.as_str())
            .unwrap_or(hub);
        let Some(rewards) = entry.get("rewards") else {
            continue;
        };
        collect_reward_rotations(rewards, "bounty", hub, bounty, Some("Bounty"), false, out);
    }
}

fn collect_reward_rotations(
    rewards: &Value,
    source_kind: &'static str,
    planet_or_hub: &str,
    node_or_bounty: &str,
    game_mode: Option<&str>,
    is_event: bool,
    out: &mut Vec<ParsedLocation>,
) {
    if let Some(obj) = rewards.as_object() {
        // 轮换 A/B/C 或扁平？若键都是单字母轮换
        let looks_like_rotation = obj.keys().all(|k| {
            let t = k.trim();
            t.len() == 1 && t.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        });
        if looks_like_rotation {
            for (rot, items) in obj {
                push_items(
                    items,
                    source_kind,
                    planet_or_hub,
                    node_or_bounty,
                    game_mode,
                    Some(rot.as_str()),
                    is_event,
                    out,
                );
            }
            return;
        }
    }
    push_items(
        rewards,
        source_kind,
        planet_or_hub,
        node_or_bounty,
        game_mode,
        None,
        is_event,
        out,
    );
}

fn push_items(
    items: &Value,
    source_kind: &'static str,
    planet_or_hub: &str,
    node_or_bounty: &str,
    game_mode: Option<&str>,
    rotation: Option<&str>,
    is_event: bool,
    out: &mut Vec<ParsedLocation>,
) {
    let list = match items {
        Value::Array(a) => a.as_slice(),
        _ => return,
    };
    for it in list {
        let name = it.get("itemName").and_then(|v| v.as_str()).unwrap_or("");
        let Some(relic_key) = relic_key_from_drop_item(name) else {
            continue;
        };
        let chance = it.get("chance").and_then(|v| v.as_f64());
        let stage = it
            .get("stage")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        out.push(ParsedLocation {
            relic_key,
            source_kind,
            planet_or_hub: planet_or_hub.to_string(),
            node_or_bounty: node_or_bounty.to_string(),
            game_mode: game_mode.map(|s| s.to_string()),
            rotation: rotation.map(|s| s.to_string()),
            chance,
            stage,
            is_event,
        });
    }
}

pub fn import_drop_locations_json(
    catalog: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<DropLocationStats, String> {
    let rows = parse_drop_locations(json)?;
    let key_to_id = load_relic_key_map(catalog)?;
    let tx = catalog
        .transaction()
        .map_err(|e| format!("来源事务失败: {e}"))?;
    tx.execute_batch("DELETE FROM relic_location;")
        .map_err(|e| format!("清空来源表失败: {e}"))?;
    ensure_mission_type_i18n(&tx)?;
    let zh = ZhLookup::load(&tx)?;

    let mut stats = DropLocationStats::default();
    let mut sort = 0i32;
    let mut skipped = 0usize;

    for row in rows {
        let Some(&relic_id) = key_to_id.get(&row.relic_key) else {
            skipped += 1;
            continue;
        };
        let id = snowflake.next_id();
        let hub_zh = place_zh(&zh, &row.planet_or_hub);
        let node_zh = place_zh(&zh, &row.node_or_bounty);
        let mode_zh = if row.source_kind == "bounty" {
            zh.get("Bounty").or_else(|| Some("赏金".into()))
        } else {
            row.game_mode.as_deref().and_then(|m| place_zh(&zh, m))
        };
        tx.execute(
            "INSERT INTO relic_location
             (id, relic_id, source_kind, planet_or_hub, planet_or_hub_zh,
              node_or_bounty, node_or_bounty_zh, game_mode, game_mode_zh,
              rotation, chance, stage, is_event, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id,
                relic_id,
                row.source_kind,
                row.planet_or_hub,
                hub_zh,
                row.node_or_bounty,
                node_zh,
                row.game_mode,
                mode_zh,
                row.rotation,
                row.chance,
                row.stage,
                if row.is_event { 1 } else { 0 },
                sort,
            ],
        )
        .map_err(|e| format!("插入来源失败: {e}"))?;
        stats.locations += 1;
        sort += 1;
        if row.source_kind == "mission" {
            stats.mission += 1;
        } else {
            stats.bounty += 1;
        }
    }
    tx.execute_batch(
        "UPDATE void_relic SET vaulted = 0
           WHERE id IN (SELECT DISTINCT relic_id FROM relic_location);
         UPDATE void_relic SET vaulted = 1
           WHERE id NOT IN (SELECT DISTINCT relic_id FROM relic_location);",
    )
    .map_err(|e| format!("回写入库状态失败: {e}"))?;
    let _ = crate::catalog::ensure_mission_type_i18n(&tx);
    tx.commit().map_err(|e| format!("来源提交失败: {e}"))?;
    if skipped > 0 {
        eprintln!("[仓鼠] 来源导入跳过未匹配核桃≈{skipped}");
    }
    Ok(stats)
}

fn load_relic_key_map(catalog: &Connection) -> Result<HashMap<String, i64>, String> {
    let mut stmt = catalog
        .prepare("SELECT relic_key, id FROM void_relic")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, id) = r.map_err(|e| e.to_string())?;
        map.insert(k, id);
    }
    Ok(map)
}

fn place_zh(zh: &ZhLookup, en: &str) -> Option<String> {
    if let Some(z) = zh.get(en) {
        return Some(z);
    }
    match en {
        "Cetus" => Some("希图斯".into()),
        "Fortuna" => Some("福尔图娜".into()),
        "Deimos" => Some("火卫二".into()),
        "Zariman" => Some("扎里曼".into()),
        "Hex" => Some("Hex".into()),
        "EntratiLab" => Some("英择谛实验室".into()),
        _ => None,
    }
}

/// 拉取并写入；`force` 忽略 ETag。空库或无来源行时也会强制。
pub fn refresh_drop_locations(
    data_dir: &Path,
    snowflake: &Snowflake,
    force: bool,
) -> Result<DropLocationStats, String> {
    db::init_databases(data_dir)?;
    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);
    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;
    refresh_drop_locations_conn(data_dir, &mut catalog, &user, snowflake, force)
}

/// 若已有遗物但 `relic_location` 为空，联网拉一次 drop-all 再写入。
/// 拉取失败则返回错误，不改用本地缓存，留下现有来源表。
pub fn heal_empty_relic_locations(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<Option<DropLocationStats>, String> {
    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);
    if !catalog_path.is_file() || !user_path.is_file() {
        return Ok(None);
    }
    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;
    let relics: i64 = catalog
        .query_row("SELECT COUNT(*) FROM void_relic", [], |r| r.get(0))
        .unwrap_or(0);
    let locs: i64 = catalog
        .query_row("SELECT COUNT(*) FROM relic_location", [], |r| r.get(0))
        .unwrap_or(0);
    if relics == 0 || locs > 0 {
        return Ok(None);
    }
    let stats = refresh_drop_locations_conn(data_dir, &mut catalog, &user, snowflake, true)?;
    Ok(Some(stats))
}

pub fn refresh_drop_locations_conn(
    data_dir: &Path,
    catalog: &mut Connection,
    _user: &Connection,
    snowflake: &Snowflake,
    force: bool,
) -> Result<DropLocationStats, String> {
    let loc_count: i64 = catalog
        .query_row("SELECT COUNT(*) FROM relic_location", [], |r| r.get(0))
        .unwrap_or(0);
    let empty = loc_count == 0;
    let force = force || empty;

    // 已有来源且未满 30 天：完全跳过联网（含 ETag 探测）
    if !force && !due_for_drop_check(catalog)? {
        return Ok(DropLocationStats::default());
    }

    let prev = if force {
        None
    } else {
        db::get_setting(catalog, SETTING_ETAG)?
    };
    // 失败直接返回，不用本地 drop-all 补写，留下现有 relic_location。
    let fetched = fetch_drop_all(DEFAULT_DROP_ALL_URL, prev.as_deref())?;
    let now = Utc::now().to_rfc3339();
    let _ = db::set_setting(catalog, SETTING_CHECKED_AT, &now);

    if fetched.not_modified {
        return Ok(DropLocationStats::default());
    }

    crate::catalog::upstream_cache::save(
        data_dir,
        crate::catalog::upstream_cache::DROP_ALL,
        &fetched.body,
    )?;
    let stats = import_drop_locations_json(catalog, &fetched.body, snowflake)?;
    if let Some(ref etag) = fetched.etag {
        db::set_setting(catalog, SETTING_ETAG, etag)?;
    } else {
        let mark = format!("len:{}", fetched.body.len());
        db::set_setting(catalog, SETTING_ETAG, &mark)?;
    }
    Ok(stats)
}

fn due_for_drop_check(catalog: &Connection) -> Result<bool, String> {
    let raw = db::get_setting(catalog, SETTING_CHECKED_AT)?;
    let Some(raw) = raw else {
        return Ok(true);
    };
    let Ok(ts) = DateTime::parse_from_rfc3339(&raw) else {
        return Ok(true);
    };
    let age = Utc::now().signed_duration_since(ts.with_timezone(&Utc));
    Ok(age >= CHECK_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::import::import_warframes_json;
    use crate::catalog::relics::import_relics_json;
    use crate::db;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_droploc_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn parse_relic_key_from_item() {
        assert_eq!(
            relic_key_from_drop_item("Lith Q3 Relic").as_deref(),
            Some("Lith Q3")
        );
        assert_eq!(relic_key_from_drop_item("Redirection"), None);
    }

    #[test]
    fn import_mission_and_bounty() {
        let dir = temp_dir("mb");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        let wf = br#"[
          {"name":"Saryn","uniqueName":"/Lotus/Powersuits/Saryn/Saryn","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Saryn Prime","uniqueName":"/Lotus/Powersuits/Saryn/SarynPrime","category":"Warframes","type":"Warframe","isPrime":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();
        let relics = br#"[
          {"name":"Lith A1 Intact","vaulted":false,"rewards":[
            {"rarity":"Uncommon","item":{"name":"Saryn Prime Neuroptics Blueprint","uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeHelmetBlueprint"}}
          ]}
        ]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();

        let drops = br#"{
          "missionRewards": {
            "Mercury": {
              "Apollodorus": {
                "gameMode": "Survival",
                "isEvent": false,
                "rewards": {
                  "A": [{"itemName":"Lith A1 Relic","chance":7.69,"rarity":"Rare"}]
                }
              }
            }
          },
          "cetusBountyRewards": [
            {
              "bountyLevel": "Level 5 - 15 Cetus Bounty",
              "rewards": {
                "A": [{"itemName":"Lith A1 Relic","chance":10.0,"stage":"Final stage"}]
              }
            }
          ]
        }"#;
        let stats = import_drop_locations_json(&mut catalog, drops, &snow).unwrap();
        assert_eq!(stats.locations, 2);
        assert_eq!(stats.mission, 1);
        assert_eq!(stats.bounty, 1);
        let vaulted: i32 = catalog
            .query_row("SELECT vaulted FROM void_relic", [], |r| r.get(0))
            .unwrap();
        assert_eq!(vaulted, 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skip_network_when_fresh_and_has_locations() {
        let dir = temp_dir("fresh");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let user = db::open_user(&db::user_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        let wf = br#"[
          {"name":"Saryn","uniqueName":"/Lotus/Powersuits/Saryn/Saryn","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Saryn Prime","uniqueName":"/Lotus/Powersuits/Saryn/SarynPrime","category":"Warframes","type":"Warframe","isPrime":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();
        let relics = br#"[
          {"name":"Lith A1 Intact","vaulted":false,"rewards":[
            {"rarity":"Uncommon","item":{"name":"Saryn Prime Neuroptics Blueprint","uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeHelmetBlueprint"}}
          ]}
        ]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();
        let drops = br#"{
          "missionRewards":{"Mercury":{"Apollodorus":{"gameMode":"Survival","rewards":{"A":[{"itemName":"Lith A1 Relic","chance":7.69}]}}}},
          "cetusBountyRewards":[]
        }"#;
        import_drop_locations_json(&mut catalog, drops, &snow).unwrap();
        db::set_setting(&catalog, SETTING_CHECKED_AT, &Utc::now().to_rfc3339()).unwrap();

        // 未满 30 天且已有 location：不应联网；返回空统计
        let stats = refresh_drop_locations_conn(&dir, &mut catalog, &user, &snow, false).unwrap();
        assert_eq!(stats.locations, 0);
        assert_eq!(stats.mission, 0);
        let n: i64 = catalog
            .query_row("SELECT COUNT(*) FROM relic_location", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
