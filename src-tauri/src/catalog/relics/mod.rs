//! WFCD Relics.json → void_relic + warframe_part_relic + weapon_part_relic + relic_reward

mod fetch;
mod parse;
mod import;

pub use fetch::fetch_relics;
pub use import::{import_relics_json, RelicImportStats};

pub const DEFAULT_RELICS_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Relics.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Relics.json",
];

#[cfg(test)]
mod tests {
    use super::*;
    use super::import::resolve_reward_zh;
    use super::parse::{map_prime_reward, map_weapon_prime_reward, rarity_code};
    use crate::catalog::import::import_warframes_json;
    use crate::db;
    use crate::id::Snowflake;
    use std::collections::HashMap;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_relic_{tag}_{}_{}",
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
    fn map_prime_reward_parses() {
        let (f, p) = map_prime_reward(
            "Saryn Prime Neuroptics Blueprint",
            "/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeHelmetBlueprint",
        )
        .unwrap();
        assert_eq!(f, "Saryn");
        assert_eq!(p, "neuroptics");
    }

    #[test]
    fn resolve_reward_zh_longest_prefix() {
        let mut i18n = HashMap::new();
        i18n.insert("Burston".into(), "伯斯顿".into());
        i18n.insert("Burston Prime".into(), "伯斯顿Prime".into());
        // 词典半英文零件行不应盖过自拼简中
        i18n.insert("Burston Prime Barrel".into(), "伯斯顿Prime Barrel".into());
        assert_eq!(
            resolve_reward_zh("Burston Prime Barrel", &i18n).as_deref(),
            Some("伯斯顿 Prime 枪管")
        );
        assert_eq!(
            resolve_reward_zh("Burston Prime Blueprint", &i18n).as_deref(),
            Some("伯斯顿 Prime 蓝图")
        );
        assert_eq!(
            resolve_reward_zh("Burston Prime", &i18n).as_deref(),
            Some("伯斯顿 Prime")
        );
        i18n.insert("Saryn Prime".into(), "SarynPrime".into());
        assert_eq!(
            resolve_reward_zh("Saryn Prime Neuroptics Blueprint", &i18n).as_deref(),
            Some("Saryn Prime 头部神经光元蓝图")
        );
        // 无战甲整装词典时仍能拼出简中零件
        let empty = HashMap::new();
        assert_eq!(
            resolve_reward_zh("Valkyr Prime Neuroptics Blueprint", &empty).as_deref(),
            Some("Valkyr Prime 头部神经光元蓝图")
        );
        assert_eq!(
            resolve_reward_zh("Trinity Prime Chassis Blueprint", &empty).as_deref(),
            Some("Trinity Prime 机体蓝图")
        );
        // 对照表：Receiver → 枪机（不是「接收器」）
        let mut gun = HashMap::new();
        gun.insert("Burston Prime".into(), "伯斯顿Prime".into());
        gun.insert("Receiver".into(), "枪机".into());
        gun.insert("Prime Receiver".into(), "枪机".into());
        assert_eq!(
            resolve_reward_zh("Burston Prime Receiver", &gun).as_deref(),
            Some("伯斯顿 Prime 枪机")
        );
    }

    #[test]
    fn map_weapon_prime_reward_parses() {
        let (f, p) = map_weapon_prime_reward("Burston Prime Barrel").unwrap();
        assert_eq!(f, "Burston");
        assert_eq!(p, "barrel");
        let (f, p) = map_weapon_prime_reward("Braton Prime Blueprint").unwrap();
        assert_eq!(f, "Braton");
        assert_eq!(p, "blueprint");
        assert!(map_weapon_prime_reward("Burston Prime").is_none());
    }

    #[test]
    fn rarity_code_splits_wfcd_uncommon_by_chance() {
        assert_eq!(rarity_code(Some("Rare"), Some(2.0)), Some(2));
        assert_eq!(rarity_code(Some("Common"), Some(25.33)), Some(0));
        assert_eq!(rarity_code(Some("Uncommon"), Some(25.33)), Some(0));
        assert_eq!(rarity_code(Some("Uncommon"), Some(11.0)), Some(1));
        assert_eq!(rarity_code(Some("Uncommon"), None), Some(1));
        assert_eq!(rarity_code(None, Some(25.0)), None);
    }

    #[test]
    fn import_links_relic_to_prime_part() {
        let dir = temp_dir("link");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        let wf = br#"[
          {
            "name": "Saryn",
            "uniqueName": "/Lotus/Powersuits/Saryn/Saryn",
            "category": "Warframes",
            "type": "Warframe",
            "components": [{ "name": "Blueprint" }]
          },
          {
            "name": "Saryn Prime",
            "uniqueName": "/Lotus/Powersuits/Saryn/SarynPrime",
            "category": "Warframes",
            "type": "Warframe",
            "isPrime": true,
            "components": [
              { "name": "Blueprint" },
              { "name": "Neuroptics" },
              { "name": "Chassis" },
              { "name": "Systems" }
            ]
          }
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();

        let relics = br#"[
          {
            "name": "Lith A1 Intact",
            "uniqueName": "/Lotus/Types/Game/Projections/T1VoidProjectionLithA1",
            "vaulted": true,
            "rewards": [
              {
                "rarity": "Uncommon",
                "chance": 11,
                "item": {
                  "name": "Saryn Prime Neuroptics Blueprint",
                  "uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeHelmetBlueprint"
                }
              },
              {
                "rarity": "Uncommon",
                "chance": 25.33,
                "item": {
                  "name": "Saryn Prime Chassis Blueprint",
                  "uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeChassisBlueprint"
                }
              },
              {
                "rarity": "Rare",
                "chance": 2,
                "item": {
                  "name": "Saryn Prime Systems Blueprint",
                  "uniqueName": "/Lotus/Types/Recipes/WarframeRecipes/SarynPrimeSystemsBlueprint"
                }
              },
              {
                "rarity": "Common",
                "chance": 25.33,
                "item": {
                  "name": "Some Weapon Blueprint",
                  "uniqueName": "/Lotus/Types/Recipes/Weapons/Foo"
                }
              }
            ]
          },
          {
            "name": "Lith A1 Exceptional",
            "rewards": []
          }
        ]"#;
        let stats = import_relics_json(&mut catalog, relics, &snow).unwrap();
        assert_eq!(stats.relics, 1);
        assert_eq!(stats.drops, 3);
        assert_eq!(stats.rewards, 4);

        let neuro: (String, i32, i32) = catalog
            .query_row(
                "SELECT v.name_zh, d.rarity, v.vaulted
                 FROM warframe_part_relic d
                 JOIN void_relic v ON v.id = d.relic_id
                 JOIN warframe_part p ON p.id = d.part_id
                 WHERE p.part_key = 'neuroptics'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(neuro.0, "古纪 A1");
        assert_eq!(neuro.1, 1); // 银
        assert_eq!(neuro.2, 1);

        let chassis: i32 = catalog
            .query_row(
                "SELECT d.rarity FROM warframe_part_relic d
                 JOIN warframe_part p ON p.id = d.part_id
                 WHERE p.part_key = 'chassis'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(chassis, 0); // 铜：Uncommon + chance≥20

        let systems: i32 = catalog
            .query_row(
                "SELECT d.rarity FROM warframe_part_relic d
                 JOIN warframe_part p ON p.id = d.part_id
                 WHERE p.part_key = 'systems'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(systems, 2); // 金

        let stored_unique: String = catalog
            .query_row(
                "SELECT unique_name FROM void_relic WHERE relic_key = 'Lith A1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(stored_unique.ends_with("T1VoidProjectionLithA1"));

        let n_weapon: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM relic_reward WHERE item_name_en LIKE '%Weapon%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n_weapon, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
