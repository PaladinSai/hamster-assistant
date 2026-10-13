//! WFCD Mods.json + drop-data all.json → catalog.mod

mod types;
mod filter;
mod source;
mod import;

pub use import::import_mods_json;
pub use types::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use super::filter::{
        is_incomplete_primed_stub, is_primed_mod, is_unreleased_mod, map_mod_kind, should_skip_mod,
        should_skip_wfcd_mod,
    };
    use super::source::{build_source_map, offer_source, source_from_drop_location};
    use super::types::WfcdMod;

    #[test]
    fn kind_aura_stance_warframe() {
        assert_eq!(map_mod_kind("Warframe Mod", Some("AURA")), MOD_KIND_AURA);
        assert_eq!(map_mod_kind("Stance Mod", None), MOD_KIND_STANCE);
        assert_eq!(map_mod_kind("Warframe Mod", Some("WARFRAME")), MOD_KIND_WARFRAME);
        assert_eq!(map_mod_kind("Shotgun Mod", None), MOD_KIND_PRIMARY);
        assert_eq!(map_mod_kind("Companion Mod", None), MOD_KIND_COMPANION);
        assert_eq!(map_mod_kind("Archwing Mod", None), MOD_KIND_OTHER);
    }

    #[test]
    fn skip_riven_focus_flawed() {
        assert!(should_skip_mod("Something", "/x", "Rifle Riven Mod"));
        assert!(should_skip_mod("Focus Way", "/x", "Focus Way"));
        assert!(should_skip_mod("Ammo Drum", "/Lotus/Flawed/Ammo", "Primary Mod"));
        assert!(!should_skip_mod("Serration", "/Lotus/Upgrades/Mods/Rifle/x", "Primary Mod"));
    }

    #[test]
    fn skip_beginner_expert_intermediate_unfused() {
        assert!(should_skip_mod(
            "Ammo Drum",
            "/Lotus/Upgrades/Mods/Rifle/Beginner/WeaponAmmoMaxModBeginner",
            "Primary Mod"
        ));
        assert!(should_skip_mod(
            "Serration",
            "/Lotus/Upgrades/Mods/Rifle/Expert/WeaponDamageAmountModExpert",
            "Primary Mod"
        ));
        assert!(should_skip_mod(
            "Ammo Drum",
            "/Lotus/Upgrades/Mods/Rifle/Intermediate/x",
            "Primary Mod"
        ));
        assert!(should_skip_mod(
            "Unfused Artifact",
            "/Lotus/Upgrades/Mods/Unfused/Artifact",
            "Warframe Mod"
        ));
        assert!(!should_skip_mod(
            "Ammo Drum",
            "/Lotus/Upgrades/Mods/Rifle/WeaponAmmoMaxMod",
            "Primary Mod"
        ));
        // Primed 也常走 /Expert/，不能跳过
        assert!(!should_skip_mod(
            "Primed Continuity",
            "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationPowerMod",
            "Warframe Mod"
        ));
    }

    #[test]
    fn primed_mod_detection() {
        assert!(is_primed_mod(
            "Primed Continuity",
            "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationPowerMod"
        ));
        assert!(is_primed_mod(
            "Something",
            "/Lotus/Upgrades/Mods/PrimedFoo"
        ));
        assert!(!is_primed_mod(
            "Serration",
            "/Lotus/Upgrades/Mods/Rifle/Expert/WeaponDamageAmountModExpert"
        ));
        assert!(is_primed_mod("primed flow", "/x"));
    }

    #[test]
    fn skip_unreleased_and_incomplete_primed() {
        assert!(is_unreleased_mod(
            Some("0000-00-00"),
            Some("TBA"),
            Some("0000-00-00")
        ));
        assert!(is_unreleased_mod(Some("0000-00-00"), None, None));
        assert!(is_unreleased_mod(None, Some("tba"), None));
        assert!(is_unreleased_mod(None, None, Some("0000-00-00")));
        assert!(!is_unreleased_mod(
            Some("2014-12-12"),
            Some("Specters of the Rail"),
            Some("2016-07-08")
        ));

        // Primed Streamline 形态
        let streamline: WfcdMod = serde_json::from_value(serde_json::json!({
            "name": "Primed Streamline",
            "uniqueName": "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityEfficiencyModExpert",
            "type": "Warframe Mod",
            "isPrime": true,
            "releaseDate": "0000-00-00",
            "introduced": { "name": "TBA", "date": "0000-00-00" }
        }))
        .unwrap();
        assert!(should_skip_wfcd_mod(&streamline));

        // 正常 Primed Continuity
        let continuity: WfcdMod = serde_json::from_value(serde_json::json!({
            "name": "Primed Continuity",
            "uniqueName": "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationPowerMod",
            "type": "Warframe Mod",
            "isPrime": true,
            "releaseDate": "2014-12-12",
            "wikiaUrl": "https://wiki.warframe.com/w/Primed_Continuity",
            "introduced": { "name": "Update 15.13", "date": "2014-12-12" }
        }))
        .unwrap();
        assert!(!should_skip_wfcd_mod(&continuity));

        // 无发布日、无 wiki 的 Primed 残缺桩
        assert!(is_incomplete_primed_stub(
            "Primed Blunderbuss",
            "/Lotus/Upgrades/Mods/Shotgun/Expert/x",
            None,
            None
        ));
        assert!(!is_incomplete_primed_stub(
            "Primed Continuity",
            "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationPowerMod",
            Some("2014-12-12"),
            Some("https://wiki.warframe.com/w/Primed_Continuity")
        ));
    }

    #[test]
    fn import_upsert_keeps_id_and_drops_orphans() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
CREATE TABLE name_i18n (name_en TEXT NOT NULL PRIMARY KEY, name_zh_userdict TEXT, name_zh_wfcd TEXT, updated_at TEXT);
CREATE TABLE mod (
  id INTEGER NOT NULL PRIMARY KEY,
  unique_name TEXT NOT NULL UNIQUE,
  name_en TEXT NOT NULL,
  name_zh TEXT,
  mod_kind INTEGER NOT NULL DEFAULT 7,
  source_kind INTEGER NOT NULL DEFAULT 0,
  is_prime INTEGER NOT NULL DEFAULT 0,
  is_augment INTEGER NOT NULL DEFAULT 0,
  polarity TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0
);
"#,
        )
        .unwrap();

        let snowflake = crate::id::Snowflake::new(1);
        let drops = br"{}";
        let uniq = "/Lotus/Upgrades/Mods/Warframe/Expert/AvatarAbilityDurationPowerMod";

        let round1 = format!(
            r#"[{{
            "name": "Primed Continuity",
            "uniqueName": "{uniq}",
            "type": "Warframe Mod",
            "isPrime": true,
            "releaseDate": "2014-12-12",
            "wikiaUrl": "https://wiki.warframe.com/w/Primed_Continuity",
            "polarity": "Madurai"
          }},
          {{
            "name": "Serration",
            "uniqueName": "/Lotus/Upgrades/Mods/Rifle/WeaponDamageAmountMod",
            "type": "Primary Mod",
            "releaseDate": "2012-10-25",
            "wikiaUrl": "https://wiki.warframe.com/w/Serration"
          }}]"#
        );
        let stats1 = import_mods_json(&mut conn, round1.as_bytes(), drops, &snowflake).unwrap();
        assert_eq!(stats1.mods, 2);

        let id1: i64 = conn
            .query_row(
                "SELECT id FROM mod WHERE unique_name = ?1",
                [uniq],
                |r| r.get(0),
            )
            .unwrap();

        let round2 = format!(
            r#"[{{
            "name": "Primed Continuity Renamed",
            "uniqueName": "{uniq}",
            "type": "Warframe Mod",
            "isPrime": true,
            "releaseDate": "2014-12-12",
            "wikiaUrl": "https://wiki.warframe.com/w/Primed_Continuity",
            "polarity": "Naramon"
          }}]"#
        );
        let stats2 = import_mods_json(&mut conn, round2.as_bytes(), drops, &snowflake).unwrap();
        assert_eq!(stats2.mods, 1);

        let (id2, name_en, polarity): (i64, String, Option<String>) = conn
            .query_row(
                "SELECT id, name_en, polarity FROM mod WHERE unique_name = ?1",
                [uniq],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(id2, id1, "unique_name 相同则 id 必须稳定");
        assert_eq!(name_en, "Primed Continuity Renamed");
        assert_eq!(polarity.as_deref(), Some("Naramon"));

        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM mod", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1, "本轮未出现的 Serration 应被删掉");
    }

    #[test]
    fn source_priority_key_beats_enemy() {
        let mut map = HashMap::new();
        offer_source(&mut map, "Vitality", SRC_ENEMY);
        offer_source(&mut map, "Vitality", SRC_KEY);
        assert_eq!(map.get("vitality"), Some(&SRC_KEY));
        offer_source(&mut map, "Vitality", SRC_CETUS);
        assert_eq!(map.get("vitality"), Some(&SRC_KEY));
    }

    #[test]
    fn source_map_casefold() {
        let mut map = HashMap::new();
        offer_source(&mut map, "Bane of Corpus", SRC_ENEMY);
        assert_eq!(map.get("bane of corpus"), Some(&SRC_ENEMY));
        assert_eq!(map.get(&name_key("Bane Of Corpus")), Some(&SRC_ENEMY));
    }

    #[test]
    fn build_source_map_from_minimal_json() {
        let json = br#"{
          "modLocations": [{"modName": "EnemyOnly"}],
          "keyRewards": [{"keyName": "Recover The Orokin Archive", "rewards": {"A": [{"itemName": "Vitality", "chance": 1}]}}],
          "transientRewards": [
            {"objectiveName": "Arbitrations", "rewards": [{"itemName": "Adaptation", "chance": 1}]},
            {"objectiveName": "Derelict Vault", "rewards": [
              {"itemName": "Blind Rage", "chance": 4.17},
              {"itemName": "Transient Fortitude", "chance": 4.17}
            ]},
            {"objectiveName": "Nightmare Mode Rewards", "rewards": [
              {"itemName": "Hammer Shot", "chance": 1.01}
            ]}
          ],
          "cetusBountyRewards": [{"rewards": {"A": [{"itemName": "Redirection", "chance": 1}]}}],
          "syndicates": {
            "Steel Meridian": [{"item": "Abating Link", "chance": 100}]
          }
        }"#;
        let map = build_source_map(json).unwrap();
        assert_eq!(map.get("enemyonly"), Some(&SRC_ENEMY));
        // 普通钥匙任务不再进龙钥桶
        assert_ne!(map.get("vitality"), Some(&SRC_KEY));
        assert_eq!(map.get("adaptation"), Some(&SRC_TRANSIENT));
        assert_eq!(map.get("blind rage"), Some(&SRC_KEY));
        assert_eq!(map.get("transient fortitude"), Some(&SRC_KEY));
        assert_eq!(map.get("hammer shot"), Some(&SRC_NIGHTMARE));
        assert_eq!(map.get("redirection"), Some(&SRC_CETUS));
        assert_eq!(map.get("abating link"), Some(&SRC_SYNDICATE));
    }

    #[test]
    fn drop_location_heuristics() {
        assert_eq!(
            source_from_drop_location("Steel Meridian, General"),
            Some(SRC_SYNDICATE)
        );
        assert_eq!(
            source_from_drop_location("Earth/Cetus (Level 5 - 15 Cetus Bounty), Rotation A"),
            Some(SRC_CETUS)
        );
        assert_eq!(
            source_from_drop_location("Deimos/Cambion Drift (Level 30 - 40 Arcana Isolation Vault), Rotation A"),
            Some(SRC_DEIMOS)
        );
        assert_eq!(
            source_from_drop_location("Derelict Vault"),
            Some(SRC_KEY)
        );
        assert_eq!(
            source_from_drop_location("Orokin Vault"),
            Some(SRC_KEY)
        );
        assert_eq!(
            source_from_drop_location("Nightmare Mode Rewards"),
            Some(SRC_NIGHTMARE)
        );
        assert_eq!(source_from_drop_location("Scaldra Eradicator"), None);
    }
}
