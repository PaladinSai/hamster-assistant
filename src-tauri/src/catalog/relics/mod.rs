//! WFCD Relics.json → void_relic + warframe_part_relic + weapon_part_relic + relic_reward

mod fetch;
mod parse;
mod import;

pub use fetch::fetch_relics;
pub use import::{
    import_relics_json, link_suit_relics_from_rewards, relink_weapon_parts_by_unique,
    RelicImportStats,
};

pub const DEFAULT_RELICS_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Relics.json";

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
    fn empty_intact_rewards_are_rejected() {
        let empty = br#"[{"name":"Lith A1 Intact","rewards":[]}]"#;
        let err = super::fetch::reject_empty_intact_rewards(empty).unwrap_err();
        assert!(err.contains("没有 Intact 奖励"));

        let with_reward = br#"[{
          "name":"Lith A1 Intact",
          "rewards":[{"rarity":"Rare","chance":2,"item":{"name":"Forma Blueprint","uniqueName":"/Lotus/Types/Items/MiscItems/FormaBlueprint"}}]
        }]"#;
        super::fetch::reject_empty_intact_rewards(with_reward).unwrap();
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

    #[test]
    fn strip_weapon_zh_drops_weapon_title() {
        use super::import::strip_weapon_zh;
        assert_eq!(
            strip_weapon_zh("旋刃飞刀 Prime 镖袋", "旋刃飞刀 Prime").as_deref(),
            Some("镖袋")
        );
        assert_eq!(
            strip_weapon_zh("诸葛连弩 Prime 弓身", "诸葛连弩Prime").as_deref(),
            Some("弓身")
        );
        assert_eq!(
            strip_weapon_zh("凯旋之爪 Prime 爪刃", "凯旋之爪").as_deref(),
            Some("爪刃")
        );
    }

    #[test]
    fn weapon_parts_align_by_unique_name() {
        let dir = temp_dir("uid");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        catalog
            .execute_batch(
                "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES
                 ('Spira Prime', '旋刃飞刀Prime'),
                 ('Zhuge Prime', '诸葛连弩Prime'),
                 ('Venka Prime', '凯旋之爪Prime'),
                 ('Pouch', '镖袋'),
                 ('Grip', '弓身'),
                 ('Blades', '爪刃')
                 ON CONFLICT(name_en) DO UPDATE SET
                   name_zh_userdict = excluded.name_zh_userdict,
                   name_zh_wfcd = NULL;",
            )
            .unwrap();

        let primary = b"[]";
        let secondary = br#"[
          {"name":"Spira Prime","uniqueName":"/Lotus/Weapons/Tenno/ThrowingWeapons/PrimeLiDagger/PrimeLiDagger","components":[
            {"itemCount":2,"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeLiDaggerBlade"},
            {"itemCount":1,"uniqueName":"/Lotus/Types/Recipes/Weapons/PrimeLiDaggerBlueprint"},
            {"itemCount":2,"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeLiDaggerHolster"},
            {"itemCount":8,"uniqueName":"/Lotus/Types/Items/MiscItems/OrokinCell"}
          ]},
          {"name":"Akbronco Prime","uniqueName":"/Lotus/Weapons/Tenno/Pistol/AkbroncoPrime","components":[
            {"itemCount":1,"uniqueName":"/Lotus/Types/Recipes/Weapons/AkbroncoPrimeBlueprint"},
            {"itemCount":2,"uniqueName":"/Lotus/Weapons/Tenno/Pistol/BroncoPrime"},
            {"itemCount":1,"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/AkbroncoPrimeLink"}
          ]}
        ]"#;
        let melee = br#"[
          {"name":"Zhuge Prime","uniqueName":"/Lotus/Weapons/Tenno/Bows/PrimeZhuge/PrimeZhugeWeapon","components":[
            {"itemCount":1,"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/ZhugePrimeStock"},
            {"itemCount":1,"uniqueName":"/Lotus/Types/Recipes/Weapons/ZhugePrimeBlueprint"}
          ]},
          {"name":"Venka Prime","uniqueName":"/Lotus/Weapons/Tenno/Melee/PrimeVenkaClaws/PrimeVenkaClaws","components":[
            {"itemCount":2,"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeVenkaClawsBlade"}
          ]}
        ]"#;
        crate::catalog::import_weapons_json(&mut catalog, primary, secondary, melee, &snow).unwrap();

        let blade_id: i64 = catalog
            .query_row(
                "SELECT p.id FROM weapon_part p
                 JOIN weapon w ON w.id = p.weapon_id
                 WHERE w.name_en = 'Spira Prime' AND p.part_key = 'blade'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        catalog
            .execute(
                "UPDATE weapon_part SET unique_name = NULL WHERE id = ?1",
                [blade_id],
            )
            .unwrap();
        crate::catalog::import_weapons_json(&mut catalog, primary, secondary, melee, &snow).unwrap();
        let blade_again: (i64, String) = catalog
            .query_row(
                "SELECT p.id, p.unique_name FROM weapon_part p
                 JOIN weapon w ON w.id = p.weapon_id
                 WHERE w.name_en = 'Spira Prime' AND p.unique_name LIKE '%PrimeLiDaggerBlade'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(blade_again.0, blade_id);

        let relics = br#"[{
          "name":"Lith Z1 Intact",
          "uniqueName":"/Lotus/Types/Game/Projections/T1VoidProjectionLithZ1",
          "rewards":[
            {"rarity":"Uncommon","chance":11,"item":{"name":"Spira Prime Pouch","uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeLiDaggerHolster"}},
            {"rarity":"Common","chance":25.33,"item":{"name":"Zhuge Prime Grip","uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/ZhugePrimeStock"}},
            {"rarity":"Rare","chance":2,"item":{"name":"Venka Prime Blades","uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeVenkaClawsBlade"}}
          ]
        }]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();

        let pouch: (String, String, i32, i64) = catalog
            .query_row(
                "SELECT p.part_key, p.name_zh, p.need_qty,
                        (SELECT COUNT(*) FROM weapon_part_relic r WHERE r.part_id = p.id)
                 FROM weapon_part p
                 JOIN weapon w ON w.id = p.weapon_id
                 WHERE p.unique_name LIKE '%PrimeLiDaggerHolster'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(pouch.0, "pouch");
        assert_eq!(pouch.1, "镖袋");
        assert_eq!(pouch.2, 2);
        assert_eq!(pouch.3, 1);

        let grip: (String, String, i64) = catalog
            .query_row(
                "SELECT p.part_key, p.name_zh,
                        (SELECT COUNT(*) FROM weapon_part_relic r WHERE r.part_id = p.id)
                 FROM weapon_part p
                 WHERE p.unique_name LIKE '%ZhugePrimeStock'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(grip.0, "grip");
        assert_eq!(grip.1, "弓身");
        assert_eq!(grip.2, 1);

        let blades: (String, String, i64) = catalog
            .query_row(
                "SELECT p.part_key, p.name_zh,
                        (SELECT COUNT(*) FROM weapon_part_relic r WHERE r.part_id = p.id)
                 FROM weapon_part p
                 WHERE p.unique_name LIKE '%PrimeVenkaClawsBlade'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(blades.0, "blades");
        assert_eq!(blades.1, "爪刃");
        assert_eq!(blades.2, 1);

        let body: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM weapon_part
                 WHERE unique_name = '/Lotus/Weapons/Tenno/Pistol/BroncoPrime'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(body, 0);
        let ak_parts: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM weapon_part p
                 JOIN weapon w ON w.id = p.weapon_id
                 WHERE w.name_en = 'Akbronco Prime'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ak_parts, 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn suit_parts_link_archwing_and_companion_relics() {
        let dir = temp_dir("suit");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        catalog
            .execute_batch(
                "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES
                 ('Odonata Prime', '陨蜓Prime'),
                 ('Carrier Prime', '搬运者Prime'),
                 ('Harness Blueprint', '外甲蓝图'),
                 ('Wings Blueprint', '机翼蓝图'),
                 ('Carapace', '外壳'),
                 ('Cerebrum', '头部')
                 ON CONFLICT(name_en) DO UPDATE SET
                   name_zh_userdict = excluded.name_zh_userdict,
                   name_zh_wfcd = NULL;",
            )
            .unwrap();

        let arch = br#"[{
          "name":"Odonata Prime",
          "uniqueName":"/Lotus/Powersuits/Archwing/PrimeJetPack/PrimeJetPack",
          "category":"Archwing",
          "type":"Archwing",
          "components":[
            {"uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/PrimeArchwing/PrimeArchwingBlueprint"},
            {"uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/PrimeArchwing/PrimeArchwingChassisComponent"},
            {"uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/PrimeArchwing/PrimeArchwingWingsComponent"}
          ]
        }]"#;
        let sentinels = br#"[{
          "name":"Carrier Prime",
          "uniqueName":"/Lotus/Types/Sentinels/SentinelPowersuits/PrimeCarrierPowerSuit",
          "category":"Sentinels",
          "type":"Sentinel",
          "components":[
            {"uniqueName":"/Lotus/Types/Recipes/SentinelRecipes/PrimeCarrierSentinelBlueprint"},
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeCarrierCarapace"},
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeCarrierCerebrum"}
          ]
        }]"#;
        let mechs = br#"[{
          "name":"Bonewidow",
          "uniqueName":"/Lotus/Powersuits/EntratiMech/ThanoTech",
          "category":"Warframes",
          "type":"Necramech",
          "components":[
            {"uniqueName":"/Lotus/Types/Gameplay/InfestedMicroplanet/Resources/Mechs/ThanomechPartEngineItem"},
            {"uniqueName":"/Lotus/Types/Gameplay/InfestedMicroplanet/Resources/Mechs/ThanomechPartWeaponPodItem"}
          ]
        }]"#;
        crate::catalog::import_archwing_json(&mut catalog, arch, &snow).unwrap();
        crate::catalog::import_sentinels_json(&mut catalog, sentinels, &snow).unwrap();
        crate::catalog::import_warframes_json(&mut catalog, mechs, &snow).unwrap();

        let relics = br#"[{
          "name":"Lith Z2 Intact",
          "uniqueName":"/Lotus/Types/Game/Projections/T1VoidProjectionLithZ2",
          "rewards":[
            {"rarity":"Rare","chance":2,"item":{"name":"Odonata Prime Harness Blueprint","uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/PrimeArchwing/PrimeArchwingChassisBlueprint"}},
            {"rarity":"Uncommon","chance":11,"item":{"name":"Odonata Prime Wings Blueprint","uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/PrimeArchwing/PrimeArchwingWingsBlueprint"}},
            {"rarity":"Common","chance":25,"item":{"name":"Carrier Prime Carapace","uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeCarrierCarapace"}},
            {"rarity":"Uncommon","chance":11,"item":{"name":"Carrier Prime Cerebrum","uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/PrimeCarrierCerebrum"}}
          ]
        }]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();

        let harness: (String, i64) = catalog
            .query_row(
                "SELECT p.name_zh, (SELECT COUNT(*) FROM warframe_part_relic r WHERE r.part_id=p.id)
                 FROM warframe_part p
                 WHERE p.unique_name LIKE '%PrimeArchwingChassisComponent'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(harness.0, "外甲蓝图");
        assert_eq!(harness.1, 1);

        let wings: (String, i64) = catalog
            .query_row(
                "SELECT p.name_zh, (SELECT COUNT(*) FROM warframe_part_relic r WHERE r.part_id=p.id)
                 FROM warframe_part p
                 WHERE p.unique_name LIKE '%PrimeArchwingWingsComponent'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wings.0, "机翼蓝图");
        assert_eq!(wings.1, 1);

        let cerebrum: (String, i64) = catalog
            .query_row(
                "SELECT p.name_zh, (SELECT COUNT(*) FROM warframe_part_relic r WHERE r.part_id=p.id)
                 FROM warframe_part p
                 WHERE p.unique_name LIKE '%PrimeCarrierCerebrum'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(cerebrum.0, "头部");
        assert_eq!(cerebrum.1, 1);

        let engine: String = catalog
            .query_row(
                "SELECT name_zh FROM warframe_part WHERE unique_name LIKE '%PartEngineItem'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(engine, "引擎");
        let pod: String = catalog
            .query_row(
                "SELECT name_zh FROM warframe_part WHERE unique_name LIKE '%PartWeaponPodItem'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pod, "武器舱");

        let _ = fs::remove_dir_all(&dir);
    }
}
