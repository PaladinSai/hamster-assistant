//! 当期 Prime Resurgence（Varzia / vaultTrader）

mod types;
mod fetch;
mod apply;
mod summary;

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::catalog::upstream_cache;
use crate::db::{self, open_catalog, open_user};
use crate::id::Snowflake;

pub use types::{ResurgenceOutcome, ResurgenceSummary};
pub use summary::{get_resurgence_summary, load_resurgence_relic_keys};
pub(crate) use types::WORLDSTATE_URL;

use apply::apply_vault_fetch;
use fetch::fetch_vault_trader;
use types::{SETTING_CHECKED_AT, SETTING_ETAG, SETTING_SOURCE_URL};

pub fn refresh_resurgence_if_needed(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<ResurgenceOutcome, String> {
    db::init_databases(data_dir)?;
    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);
    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;

    if local_rotation_fresh(&catalog)? {
        let now = Utc::now().to_rfc3339();
        let _ = db::set_setting(&catalog, SETTING_CHECKED_AT, &now);
        return Ok(ResurgenceOutcome::SkippedFresh);
    }

    pull_and_apply_resurgence(&data_dir, &mut catalog, &user, snowflake)
}

/// 手动「更新数据」：忽略本地轮次是否未过期，强制拉网。
pub fn force_refresh_resurgence(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<ResurgenceOutcome, String> {
    db::init_databases(data_dir)?;
    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);
    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;
    pull_and_apply_resurgence(data_dir, &mut catalog, &user, snowflake)
}

fn pull_and_apply_resurgence(
    data_dir: &Path,
    catalog: &mut Connection,
    _user: &Connection,
    snowflake: &Snowflake,
) -> Result<ResurgenceOutcome, String> {
    let prev_etag = db::get_setting(catalog, SETTING_ETAG)?;
    let prev_source = db::get_setting(catalog, SETTING_SOURCE_URL)?;
    let fetch = fetch_vault_trader(prev_etag.as_deref(), prev_source.as_deref())?;
    let now = Utc::now().to_rfc3339();
    db::set_setting(catalog, SETTING_CHECKED_AT, &now)?;
    if fetch.not_modified {
        return Ok(ResurgenceOutcome::NotModified);
    }
    upstream_cache::save(data_dir, fetch.cache_name, &fetch.raw_body)?;
    if let Some(ref etag) = fetch.etag {
        db::set_setting(catalog, SETTING_ETAG, etag)?;
    }
    db::set_setting(catalog, SETTING_SOURCE_URL, &fetch.source_url)?;
    let stats = apply_vault_fetch(catalog, snowflake, &fetch.vault, &fetch.source_url)?;
    Ok(ResurgenceOutcome::Updated {
        warframes: stats.0,
        weapons: stats.1,
        relics: stats.2,
    })
}

fn local_rotation_fresh(catalog: &Connection) -> Result<bool, String> {
    let now = Utc::now().to_rfc3339();
    let future_id: Option<i64> = catalog
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
    let Some(id) = future_id else {
        return Ok(false);
    };
    // 旧推导把历史核桃的 unique_name 留空。这种当期要重拉，改成只标货架投影。
    let stale: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM resurgence_item
             WHERE rotation_id = ?1 AND kind = 'relic' AND unique_name IS NULL",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if stale > 0 {
        return Ok(false);
    }
    // 有当期但还没有下期行时再拉一次（写入下期预告）
    let current_expiry: Option<String> = catalog
        .query_row(
            "SELECT expiry FROM resurgence_rotation WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();
    if let Some(exp) = current_expiry {
        let has_next: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM resurgence_rotation
                 WHERE id != ?1 AND (
                   activation = ?2
                   OR (expiry IS NOT NULL AND expiry > ?2)
                 )",
                params![id, exp],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if has_next == 0 {
            return Ok(false);
        }
    }
    Ok(true)
}


#[cfg(test)]
mod tests {
    use super::apply::{
        apply_vault_fetch, families_from_featured, humanize_path_tail,
        import_vault_trader_json, parse_inventory,
    };
    use super::fetch::trader_from_worldstate;
    use super::types::*;
    use super::*;
    use crate::catalog::import::import_warframes_json;
    use crate::id::Snowflake;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_resurgence_{tag}_{}_{}",
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
    fn official_worldstate_becomes_vault_trader() {
        let json = br#"{
          "PrimeVaultTraders":[{
            "Activation":{"$date":{"$numberLong":"1788458400000"}},
            "Expiry":{"$date":{"$numberLong":"1790877600000"}},
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime"},
              {"ItemType":"/Lotus/StoreItems/Weapons/Tenno/Melee/PrimeKogake/KogakePrimeKnuckles"}
            ]
          }]
        }"#;
        let fetched = trader_from_worldstate(json).unwrap();
        assert!(fetched.current.activation.unwrap().starts_with("2026-"));
        assert_eq!(fetched.current.inventory.len(), 2);
        assert!(fetched.next.is_none());
        let items = parse_inventory(&fetched.current.inventory);
        assert_eq!(items.iter().filter(|i| i.kind == "warframe").count(), 1);
        assert_eq!(items[0].family_key.as_deref(), Some("Banshee"));
        assert_eq!(items[0].name_en, "Banshee Prime");
        let wp = items.iter().find(|i| i.kind == "weapon").expect("weapon");
        assert_eq!(wp.name_en, "Kogake Prime");
        assert_eq!(wp.family_key.as_deref(), Some("Kogake"));
    }

    #[test]
    fn humanize_path_tail_splits_camel_case() {
        assert_eq!(humanize_path_tail("BansheePrime"), "Banshee Prime");
        assert_eq!(
            humanize_path_tail("KogakePrimeKnuckles"),
            "Kogake Prime Knuckles"
        );
    }

    #[test]
    fn worldstate_no_item_label_resolves_zh_from_entity() {
        let dir = temp_dir("ws-zh");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        let wf = br#"[
          {"name":"Banshee","uniqueName":"/Lotus/Powersuits/Banshee/Banshee","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Banshee Prime","uniqueName":"/Lotus/Powersuits/Banshee/BansheePrime","category":"Warframes","type":"Warframe","isPrime":true,"vaulted":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();
        catalog
            .execute(
                "UPDATE warframe SET name_zh = 'Banshee Prime' WHERE variant = 1",
                [],
            )
            .unwrap();
        catalog
            .execute(
                "UPDATE warframe_family SET name_zh = 'Banshee' WHERE family_key = 'Banshee'",
                [],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO weapon_family (id, family_key, name_en, name_zh, slot)
                 VALUES (1, 'Kogake', 'Kogake', '科加基', 3)",
                [],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO weapon (id, family_id, variant, name_en, name_zh, unique_name, can_incarnon, vault_status, sort_order)
                 VALUES (10, 1, 1, 'Kogake Prime', '科加基 Prime', '/Lotus/Weapons/Tenno/Melee/PrimeKogake/KogakePrime', 0, 2, 0)",
                [],
            )
            .unwrap();

        // worldstate 风格：仅 uniqueName / ItemType，无 item 展示名
        let body = br#"{
          "PrimeVaultTraders":[{
            "Activation":{"$date":{"$numberLong":"1788458400000"}},
            "Expiry":{"$date":{"$numberLong":"1893456000000"}},
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime"},
              {"ItemType":"/Lotus/StoreItems/Weapons/Tenno/Melee/PrimeKogake/KogakePrimeKnuckles"}
            ]
          }]
        }"#;
        let fetched = trader_from_worldstate(body).unwrap();
        apply_vault_fetch(&mut catalog, &snow, &fetched, "test://ws-zh").unwrap();

        let wf_zh: String = catalog
            .query_row(
                "SELECT name_zh FROM resurgence_item WHERE kind = 'warframe'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(wf_zh, "Banshee Prime");

        let (wp_zh, fk): (String, String) = catalog
            .query_row(
                "SELECT name_zh, family_key FROM resurgence_item WHERE kind = 'weapon'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wp_zh, "科加基 Prime");
        assert_eq!(fk, "Kogake");

        let joined: i64 = catalog
            .query_row(
                "SELECT w.id FROM resurgence_item i
                 JOIN weapon_family wf ON wf.family_key = i.family_key
                 JOIN weapon w ON w.family_id = wf.id AND w.variant = 1
                 WHERE i.kind = 'weapon'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(joined, 10);

        let summary = get_resurgence_summary(&dir).unwrap();
        let weapon = summary.weapons.first().expect("summary weapon");
        assert_eq!(weapon.name_zh.as_deref(), Some("科加基 Prime"));
        let warframe = summary.warframes.first().expect("summary warframe");
        assert!(
            warframe.name_zh.as_deref().is_some_and(|z| !z.is_empty()),
            "warframe name_zh empty"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn schedule_info_picks_next_featured() {
        // 当期 expiry 2026-10-01；下期 Featured 2026-10-29；更远一条无 Featured
        let json = br#"{
          "PrimeVaultTraders":[{
            "Activation":{"$date":{"$numberLong":"1756706400000"}},
            "Expiry":{"$date":{"$numberLong":"1759312800000"}},
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime"}
            ],
            "ScheduleInfo":[
              {
                "Expiry":{"$date":{"$numberLong":"1759312800000"}},
                "FeaturedItem":"/Lotus/StoreItems/Packages/MPVBansheeNezhaPrimeDualPack"
              },
              {
                "Expiry":{"$date":{"$numberLong":"1761732000000"}},
                "FeaturedItem":"/Lotus/StoreItems/Packages/MPVProteaIvaraPrimeDualPack"
              },
              {
                "Expiry":{"$date":{"$numberLong":"1764151200000"}}
              }
            ]
          }]
        }"#;
        let fetched = trader_from_worldstate(json).unwrap();
        let next = fetched.next.expect("next");
        assert!(next.featured.contains("ProteaIvara"));
        assert_eq!(next.activation, fetched.current.expiry);
        assert!(next.expiry.as_ref().unwrap().starts_with("2025-"));
    }

    #[test]
    fn featured_pack_splits_family_keys() {
        let known = vec![
            "Protea".into(),
            "Ivara".into(),
            "Banshee".into(),
            "Nezha".into(),
        ];
        let keys = families_from_featured(
            "/Lotus/StoreItems/Packages/MPVProteaIvaraPrimeDualPack",
            &known,
        );
        assert_eq!(keys, vec!["Protea".to_string(), "Ivara".to_string()]);
        let single = families_from_featured(
            "/Lotus/StoreItems/Packages/MPVBansheePrimeSinglePack",
            &known,
        );
        assert_eq!(single, vec!["Banshee".to_string()]);
    }

    #[test]
    fn shelf_projection_marks_only_matching_relic() {
        let dir = temp_dir("shelf");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        catalog
            .execute_batch(
                "INSERT INTO void_relic (id, relic_key, era, code, name_zh, vaulted, unique_name) VALUES
                 (1, 'Lith K5', 0, 'K5', '古纪 K5', 1, '/Lotus/Types/Game/Projections/T1VoidProjectionBansheeMirageVaultABronze'),
                 (2, 'Axi B1', 3, 'B1', '后纪 B1', 1, '/Lotus/Types/Game/Projections/T4VoidProjectionOther');",
            )
            .unwrap();
        let snow = Snowflake::new(1);
        let trader = br#"{
          "activation": "2026-09-03T18:00:00.000Z",
          "expiry": "2099-10-01T18:00:00.000Z",
          "inventory": [
            {"uniqueName":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime","item":"Banshee Prime"},
            {"uniqueName":"/Lotus/StoreItems/Types/Game/Projections/T1VoidProjectionBansheeMirageVaultABronze","item":"Lith K5 Relic"}
          ]
        }"#;
        let (_, _, n_relic) = import_vault_trader_json(&mut catalog, &snow, trader).unwrap();
        assert_eq!(n_relic, 1);
        let key: String = catalog
            .query_row(
                "SELECT relic_key FROM resurgence_item WHERE kind = 'relic'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(key, "Lith K5");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn null_relic_path_forces_refresh() {
        let dir = temp_dir("stale-relic");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let trader = br#"{
          "activation": "2026-09-03T18:00:00.000Z",
          "expiry": "2099-12-01T18:00:00.000Z",
          "inventory": []
        }"#;
        import_vault_trader_json(&mut catalog, &snow, trader).unwrap();
        assert!(local_rotation_fresh(&catalog).unwrap());
        let rotation_id: i64 = catalog
            .query_row("SELECT id FROM resurgence_rotation", [], |r| r.get(0))
            .unwrap();
        catalog
            .execute(
                "INSERT INTO resurgence_item
                 (id, rotation_id, kind, name_en, unique_name, relic_key, sort_order)
                 VALUES (99, ?1, 'relic', 'Axi B1', NULL, 'Axi B1', 0)",
                [rotation_id],
            )
            .unwrap();
        assert!(!local_rotation_fresh(&catalog).unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn classify_warframe_and_weapon() {
        let items = parse_inventory(&[
            VaultItem {
                unique_name: Some(
                    "/Lotus/StoreItems/Powersuits/Banshee/BansheePrime".into(),
                ),
                item: Some("Banshee Prime".into()),
            },
            VaultItem {
                unique_name: Some(
                    "/Lotus/StoreItems/Weapons/Tenno/Pistols/PrimeAkbolto/PrimeAkBoltoWeapon"
                        .into(),
                ),
                item: Some("Prime Ak Bolto Weapon".into()),
            },
            VaultItem {
                unique_name: Some(
                    "/Lotus/Types/StoreItems/Packages/MegaPrimeVault/MPVBansheePrimeSinglePack"
                        .into(),
                ),
                item: Some("pack".into()),
            },
        ]);
        assert_eq!(items.iter().filter(|i| i.kind == "warframe").count(), 1);
        assert_eq!(items.iter().filter(|i| i.kind == "weapon").count(), 1);
        assert_eq!(items[0].family_key.as_deref(), Some("Banshee"));
    }

    #[test]
    fn import_sets_vault_status_three() {
        let dir = temp_dir("vault3");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let wf = br#"[
          {"name":"Banshee","uniqueName":"/Lotus/Powersuits/Banshee/Banshee","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Banshee Prime","uniqueName":"/Lotus/Powersuits/Banshee/BansheePrime","category":"Warframes","type":"Warframe","isPrime":true,"vaulted":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();

        let trader = br#"{
          "activation": "2026-09-03T18:00:00.000Z",
          "expiry": "2099-10-01T18:00:00.000Z",
          "inventory": [
            {"uniqueName":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime","item":"Banshee Prime"},
            {"uniqueName":"/Lotus/StoreItems/Weapons/Tenno/Melee/PrimeKogake/KogakePrimeKnuckles","item":"Kogake Prime"}
          ]
        }"#;
        let (nw, nwp, _) = import_vault_trader_json(&mut catalog, &snow, trader).unwrap();
        assert_eq!(nw, 1);
        assert_eq!(nwp, 1);

        let vs: i32 = catalog
            .query_row(
                "SELECT vault_status FROM warframe WHERE variant = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vs, 3);

        assert!(local_rotation_fresh(&catalog).unwrap());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn summary_weapon_join_by_family_key_not_storeitems_path() {
        let dir = temp_dir("wepjoin");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        // WFCD 路径（无 StoreItems），与重生 Manifest 不一致
        catalog
            .execute(
                "INSERT INTO weapon_family (id, family_key, name_en, slot) VALUES (1, 'Kogake', 'Kogake', 3)",
                [],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO weapon (id, family_id, variant, name_en, unique_name, can_incarnon, vault_status, sort_order)
                 VALUES (10, 1, 1, 'Kogake Prime', '/Lotus/Weapons/Tenno/Melee/PrimeKogake/KogakePrime', 0, 2, 0)",
                [],
            )
            .unwrap();

        let trader = br#"{
          "activation": "2026-09-03T18:00:00.000Z",
          "expiry": "2099-10-01T18:00:00.000Z",
          "inventory": [
            {"uniqueName":"/Lotus/StoreItems/Weapons/Tenno/Melee/PrimeKogake/KogakePrimeKnuckles","item":"Kogake Prime"}
          ]
        }"#;
        let (_, nwp, _) = import_vault_trader_json(&mut catalog, &snow, trader).unwrap();
        assert_eq!(nwp, 1);

        // 确认 unique_name 精确相等会失败（StoreItems vs Weapons）
        let by_path: Option<i64> = catalog
            .query_row(
                "SELECT w.id FROM resurgence_item i
                 JOIN weapon w ON w.unique_name = i.unique_name
                 WHERE i.kind = 'weapon'",
                [],
                |r| r.get(0),
            )
            .optional()
            .unwrap();
        assert!(by_path.is_none());

        // 与 get_resurgence_summary 相同的 family_key JOIN
        let (wid, fid): (i64, i64) = catalog
            .query_row(
                "SELECT w.id, wf.id
                 FROM resurgence_item i
                 LEFT JOIN weapon_family wf ON wf.family_key = i.family_key
                 LEFT JOIN weapon w ON w.family_id = wf.id AND w.variant = 1
                 WHERE i.kind = 'weapon'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wid, 10);
        assert_eq!(fid, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn next_featured_written_without_vault_status() {
        let dir = temp_dir("next");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let wf = br#"[
          {"name":"Banshee","uniqueName":"/Lotus/Powersuits/Banshee/Banshee","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Banshee Prime","uniqueName":"/Lotus/Powersuits/Banshee/BansheePrime","category":"Warframes","type":"Warframe","isPrime":true,"vaulted":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]},
          {"name":"Protea","uniqueName":"/Lotus/Powersuits/Protea/Protea","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Protea Prime","uniqueName":"/Lotus/Powersuits/Protea/ProteaPrime","category":"Warframes","type":"Warframe","isPrime":true,"vaulted":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]},
          {"name":"Ivara","uniqueName":"/Lotus/Powersuits/Ivara/Ivara","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Ivara Prime","uniqueName":"/Lotus/Powersuits/Ivara/IvaraPrime","category":"Warframes","type":"Warframe","isPrime":true,"vaulted":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();

        // 时间戳须晚于「现在」，否则 current_rotation_id 会把下期当成当期
        let body = br#"{
          "PrimeVaultTraders":[{
            "Activation":{"$date":{"$numberLong":"1893456000000"}},
            "Expiry":{"$date":{"$numberLong":"1896048000000"}},
            "Manifest":[
              {"ItemType":"/Lotus/StoreItems/Powersuits/Banshee/BansheePrime"}
            ],
            "ScheduleInfo":[
              {
                "Expiry":{"$date":{"$numberLong":"1898640000000"}},
                "FeaturedItem":"/Lotus/StoreItems/Packages/MPVProteaIvaraPrimeDualPack"
              }
            ]
          }]
        }"#;
        let fetched = trader_from_worldstate(body).unwrap();
        apply_vault_fetch(&mut catalog, &snow, &fetched, "test://ws").unwrap();

        let next_wf: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM resurgence_item i
                 JOIN resurgence_rotation r ON r.id = i.rotation_id
                 WHERE i.kind = 'warframe' AND r.activation = ?1",
                [fetched.current.expiry.as_deref().unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(next_wf, 2);

        let protea_vs: i32 = catalog
            .query_row(
                "SELECT vault_status FROM warframe_family WHERE family_key = 'Protea'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(protea_vs, 3);

        let summary = get_resurgence_summary(&dir).unwrap();
        let next = summary.next.expect("summary.next");
        assert_eq!(next.warframes.len(), 2);
        let keys: Vec<_> = next.warframes.iter().map(|w| w.family_key.as_str()).collect();
        assert!(keys.contains(&"Protea"));
        assert!(keys.contains(&"Ivara"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fresh_rotation_skips_network_path() {
        let dir = temp_dir("fresh");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let trader = br#"{
          "activation": "2026-09-03T18:00:00.000Z",
          "expiry": "2099-12-01T18:00:00.000Z",
          "inventory": []
        }"#;
        import_vault_trader_json(&mut catalog, &snow, trader).unwrap();
        let updated_before: String = catalog
            .query_row(
                "SELECT updated_at FROM resurgence_rotation ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(local_rotation_fresh(&catalog).unwrap());
        // 未过期时 refresh 应走 SkippedFresh，不改 rotation 行
        let user = open_user(&db::user_path(&dir)).unwrap();
        let now = Utc::now().to_rfc3339();
        let _ = db::set_setting(&user, SETTING_CHECKED_AT, &now);
        let updated_after: String = catalog
            .query_row(
                "SELECT updated_at FROM resurgence_rotation ORDER BY id DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(updated_before, updated_after);

        let _ = fs::remove_dir_all(&dir);
    }
}

