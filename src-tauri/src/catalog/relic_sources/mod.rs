//! 遗物来源表：搜索 / 筛选 IPC

mod types;
mod locations;
mod filters;
mod search;

pub use filters::get_relic_source_filter_options;
pub use search::search_relic_sources;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::drop_locations::import_drop_locations_json;
    use crate::catalog::import::import_warframes_json;
    use crate::catalog::relics::import_relics_json;
    use crate::db;
    use crate::id::Snowflake;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_relicsrc_{tag}_{}_{}",
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
    fn search_by_zh_finds_relic() {
        let dir = temp_dir("zh");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        catalog
            .execute(
                "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES ('Saryn Prime Neuroptics Blueprint', 'Saryn Prime头部神经光元蓝图')
                 ON CONFLICT(name_en) DO UPDATE SET name_zh_userdict = excluded.name_zh_userdict",
                [],
            )
            .unwrap();

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
        drop(catalog);

        let res = search_relic_sources(
            &dir,
            RelicSourceQuery {
                query: Some("神经光元".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].relic_key, "Lith A1");
        assert!(!res.items[0].locations.is_empty());
        assert_eq!(
            res.items[0].locations[0].game_mode_zh.as_deref(),
            Some("生存")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_by_weapon_zh_prefix_finds_part() {
        let dir = temp_dir("burston-zh");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);

        // 占位战甲，避免 search 时 init_databases 因空库拷贝种子冲掉本测数据
        let wf = br#"[
          {"name":"Excalibur","uniqueName":"/Lotus/Powersuits/Excalibur/Excalibur","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();

        // 仅整装简中，无零件级 i18n（与线上一致）
        catalog
            .execute(
                "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES ('Burston Prime', '伯斯顿Prime')
                 ON CONFLICT(name_en) DO UPDATE SET name_zh_userdict = excluded.name_zh_userdict",
                [],
            )
            .unwrap();

        let relics = br#"[
          {"name":"Lith B1 Intact","vaulted":false,"rewards":[
            {"rarity":"Uncommon","item":{"name":"Burston Prime Barrel","uniqueName":"/Lotus/Types/Recipes/WeaponParts/BurstonPrimeBarrel"}}
          ]}
        ]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();
        let drops = br#"{
          "missionRewards":{"Mercury":{"Apollodorus":{"gameMode":"Survival","rewards":{"A":[{"itemName":"Lith B1 Relic","chance":7.69}]}}}},
          "cetusBountyRewards":[]
        }"#;
        import_drop_locations_json(&mut catalog, drops, &snow).unwrap();
        drop(catalog);

        let res = search_relic_sources(
            &dir,
            RelicSourceQuery {
                query: Some("伯斯顿".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.items.len(), 1);
        assert_eq!(res.items[0].relic_key, "Lith B1");
        let matched = &res.items[0].matched_items;
        assert!(
            matched.iter().any(|m| m.item_name_en == "Burston Prime Barrel"),
            "expected Burston part match, got {matched:?}"
        );
        assert_eq!(
            matched
                .iter()
                .find(|m| m.item_name_en == "Burston Prime Barrel")
                .and_then(|m| m.item_name_zh.as_deref()),
            Some("伯斯顿 Prime 枪管")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_keeps_relic_when_mode_filter_misses() {
        let dir = temp_dir("keep");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        catalog
            .execute(
                "INSERT INTO name_i18n (name_en, name_zh_userdict) VALUES ('Caliban Prime Neuroptics Blueprint', 'Caliban Prime头部神经光元蓝图')
                 ON CONFLICT(name_en) DO UPDATE SET name_zh_userdict = excluded.name_zh_userdict",
                [],
            )
            .unwrap();
        let wf = br#"[
          {"name":"Caliban","uniqueName":"/Lotus/Powersuits/Caliban/Caliban","category":"Warframes","type":"Warframe","components":[{"name":"Blueprint"}]},
          {"name":"Caliban Prime","uniqueName":"/Lotus/Powersuits/Caliban/CalibanPrime","category":"Warframes","type":"Warframe","isPrime":true,"components":[{"name":"Blueprint"},{"name":"Neuroptics"},{"name":"Chassis"},{"name":"Systems"}]}
        ]"#;
        import_warframes_json(&mut catalog, wf, &snow).unwrap();
        let relics = br#"[
          {"name":"Lith C1 Intact","vaulted":false,"rewards":[
            {"rarity":"Uncommon","item":{"name":"Caliban Prime Neuroptics Blueprint","uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/CalibanPrimeHelmetBlueprint"}}
          ]},
          {"name":"Axi C1 Intact","vaulted":false,"rewards":[
            {"rarity":"Rare","item":{"name":"Caliban Prime Neuroptics Blueprint","uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/CalibanPrimeHelmetBlueprint"}}
          ]},
          {"name":"Neo C1 Intact","vaulted":true,"rewards":[
            {"rarity":"Common","item":{"name":"Caliban Prime Neuroptics Blueprint","uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/CalibanPrimeHelmetBlueprint"}}
          ]}
        ]"#;
        import_relics_json(&mut catalog, relics, &snow).unwrap();
        let drops = br#"{
          "missionRewards":{
            "Earth":{"Mariana":{"gameMode":"Capture","rewards":{"A":[{"itemName":"Lith C1 Relic","chance":10}]}}},
            "Pluto":{"Hades":{"gameMode":"Survival","rewards":{"A":[{"itemName":"Axi C1 Relic","chance":8}]}}}
          },
          "cetusBountyRewards":[]
        }"#;
        import_drop_locations_json(&mut catalog, drops, &snow).unwrap();
        drop(catalog);

        let res = search_relic_sources(
            &dir,
            RelicSourceQuery {
                query: Some("Caliban".into()),
                game_modes: Some(vec!["Capture".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.items.len(), 2);
        assert!(res.items.iter().all(|it| it.relic_key != "Neo C1"));
        let axi = res.items.iter().find(|it| it.relic_key == "Axi C1").unwrap();
        assert!(axi.locations.is_empty());
        let lith = res.items.iter().find(|it| it.relic_key == "Lith C1").unwrap();
        assert_eq!(lith.locations.len(), 1);

        let hidden = search_relic_sources(
            &dir,
            RelicSourceQuery {
                game_modes: Some(vec!["Capture".into()]),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(hidden.items.iter().all(|it| it.relic_key != "Axi C1"));
        assert!(hidden.items.iter().any(|it| it.relic_key == "Lith C1"));

        let opened = search_relic_sources(
            &dir,
            RelicSourceQuery {
                relic_key: Some("Neo C1".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(opened.items.len(), 1);
        assert_eq!(opened.items[0].vaulted, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
