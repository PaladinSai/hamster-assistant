use super::parse::{
    is_amp_prism, is_kitgun_chamber, is_zaw_strike, modular_zh_fallback, parse_modular_parts,
    parse_weapons,
};
use super::rules::{
    can_incarnon, is_space_product_category, weapon_family_and_variant, weapon_part_meta,
    WfcdWeapon, SLOT_PRIMARY, VARIANT_BASE, VARIANT_CODA, VARIANT_DEX, VARIANT_KUVA,
    VARIANT_MK1, VARIANT_PRIME, VARIANT_VANDAL,
};
use crate::catalog::wfcd::{MODULAR_KIND_AMP, MODULAR_KIND_KITGUN, MODULAR_KIND_ZAW};

    #[test]
    fn family_variant_prefixes() {
        assert_eq!(
            weapon_family_and_variant("Kuva Braton"),
            ("Braton".into(), VARIANT_KUVA)
        );
        assert_eq!(
            weapon_family_and_variant("Braton Prime"),
            ("Braton".into(), VARIANT_PRIME)
        );
        assert_eq!(
            weapon_family_and_variant("Mk1-Braton"),
            ("Braton".into(), VARIANT_MK1)
        );
        assert_eq!(
            weapon_family_and_variant("Braton Vandal"),
            ("Braton".into(), VARIANT_VANDAL)
        );
        assert_eq!(
            weapon_family_and_variant("Coda Hema"),
            ("Hema".into(), VARIANT_CODA)
        );
        assert_eq!(
            weapon_family_and_variant("Dex Sybaris"),
            ("Sybaris".into(), VARIANT_DEX)
        );
    }

    #[test]
    fn part_meta_filters_materials() {
        assert!(weapon_part_meta("Blueprint").is_some());
        assert!(weapon_part_meta("Barrel").is_some());
        assert!(weapon_part_meta("Orokin Cell").is_none());
        assert!(weapon_part_meta("Alloy Plate").is_none());
    }

    #[test]
    fn modular_part_classifiers() {
        assert!(is_kitgun_chamber(
            "/Lotus/Weapons/SolarisUnited/Secondary/SUModularSecondarySet1/Barrel/SUModularSecondaryBarrelAPart",
            "Kitgun Component",
        ));
        assert!(is_kitgun_chamber(
            "/Lotus/Weapons/Infested/Pistols/InfKitGun/Barrels/InfBarrelEgg/InfModularBarrelEggPart",
            "Pistol",
        ));
        assert!(!is_kitgun_chamber(
            "/Lotus/Weapons/SolarisUnited/Secondary/SUModularSecondarySet1/Grip/Foo",
            "Kitgun Component",
        ));
        assert!(is_zaw_strike(
            "/Lotus/Weapons/Ostron/Melee/ModularMelee01/Tip/TipOne",
            "Zaw Component",
            &["Zaw".into()],
        ));
        assert!(is_zaw_strike(
            "/Lotus/Weapons/Ostron/Melee/ModularMeleeInfested/Tips/InfestedTipOne",
            "Zaw Component",
            &["Infested".into(), "Zaw".into()],
        ));
        assert!(!is_zaw_strike(
            "/Lotus/Weapons/Ostron/Melee/ModularMelee01/Grip/GripOne",
            "Zaw Component",
            &["Zaw".into()],
        ));
        assert!(is_amp_prism(
            "Raplak Prism",
            "/Lotus/Weapons/Sentients/OperatorAmplifiers/Set1/Barrel/SentAmpSet1BarrelPartA",
        ));
        assert_eq!(modular_zh_fallback("Mote Prism"), Some("微尘棱镜"));
        assert_eq!(modular_zh_fallback("Catchmoon"), Some("捕月"));
    }

    #[test]
    fn parse_modular_parts_from_sample() {
        let misc = r#"[
          {"name":"Catchmoon","uniqueName":"/Lotus/Weapons/SolarisUnited/Secondary/SUModularSecondarySet1/Barrel/SUModularSecondaryBarrelAPart","type":"Kitgun Component"},
          {"name":"Gaze Grip","uniqueName":"/Lotus/Weapons/SolarisUnited/Secondary/SUModularSecondarySet1/Grip/Foo","type":"Kitgun Component"},
          {"name":"Raplak Prism","uniqueName":"/Lotus/Weapons/Sentients/OperatorAmplifiers/Set1/Barrel/SentAmpSet1BarrelPartA","type":"Amp"},
          {"name":"Sporelacer","uniqueName":"/Lotus/Weapons/Infested/Pistols/InfKitGun/Barrels/InfBarrelEgg/InfModularBarrelEggPart","type":"Pistol"}
        ]"#;
        let melee = r#"[
          {"name":"Balla","uniqueName":"/Lotus/Weapons/Ostron/Melee/ModularMelee01/Tip/TipOne","type":"Zaw Component","tags":["Zaw"]},
          {"name":"Balla","uniqueName":"/Lotus/Weapons/Ostron/Melee/ModularMelee01/Tip/PvPVariantTipOne","type":"Zaw Component","tags":["Zaw"]}
        ]"#;
        let a = parse_modular_parts(misc.as_bytes()).unwrap();
        let b = parse_modular_parts(melee.as_bytes()).unwrap();
        assert_eq!(a.len(), 3); // Catchmoon, Raplak, Sporelacer
        assert_eq!(b.len(), 1); // Balla only (PvP skipped)
        assert!(a.iter().any(|p| p.name_en == "Catchmoon" && p.modular_kind == MODULAR_KIND_KITGUN));
        assert!(a.iter().any(|p| p.name_en == "Raplak Prism" && p.modular_kind == MODULAR_KIND_AMP));
        assert!(b.iter().any(|p| p.name_en == "Balla" && p.modular_kind == MODULAR_KIND_ZAW));
    }

    #[test]
    fn space_product_category_filters_misfiled_corufell() {
        let space = WfcdWeapon {
            name: "Veritux".into(),
            unique_name: "/Lotus/Weapons/Tenno/Archwing/Melee/Archsword/ArchSwordWeapon"
                .into(),
            tags: vec![],
            vaulted: None,
            category: Some("Arch-Melee".into()),
            item_type: Some("Arch-Melee".into()),
            product_category: Some("SpaceMelee".into()),
            components: vec![],
        };
        let misfiled = WfcdWeapon {
            name: "Corufell Prime".into(),
            unique_name:
                "/Lotus/Weapons/Tenno/Archwing/Melee/PrimeCorufell/PrimeCorufellScytheWeapon"
                    .into(),
            tags: vec![],
            vaulted: None,
            category: Some("Arch-Melee".into()),
            item_type: Some("Arch-Melee".into()),
            product_category: Some("Melee".into()),
            components: vec![],
        };
        assert!(is_space_product_category(&space, "SpaceMelee"));
        assert!(!is_space_product_category(&misfiled, "SpaceMelee"));
        assert!(!is_space_product_category(&misfiled, "SpaceGuns"));
    }

    #[test]
    fn refs_only_components_normalize_names() {
        let json = r#"[
          {"name":"Braton Prime","uniqueName":"/Lotus/Weapons/Tenno/Rifle/BratonPrime","components":[
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeBarrel","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/BratonPrimeBlueprint","itemCount":1},
            {"uniqueName":"/Lotus/Types/Items/MiscItems/OrokinCell","itemCount":10},
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeReceiver","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/Weapons/WeaponParts/BratonPrimeStock","itemCount":1}
          ]}
        ]"#;
        let items = parse_weapons(json.as_bytes(), SLOT_PRIMARY).unwrap();
        assert_eq!(items.len(), 1);
        let names: Vec<&str> = items[0].0.components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Barrel"));
        assert!(names.contains(&"Blueprint"));
        assert!(names.contains(&"Receiver"));
        assert!(names.contains(&"Stock"));
        assert!(weapon_part_meta("Barrel").is_some());
        // 材料 uniqueName 无法映射，name 仍为空，导入时跳过
        let cell = items[0]
            .0
            .components
            .iter()
            .find(|c| {
                c.unique_name
                    .as_deref()
                    .unwrap_or("")
                    .contains("OrokinCell")
            })
            .unwrap();
        assert!(cell.name.is_empty());
    }

    #[test]
    fn incarnon_genesis_and_native_tags() {
        let empty: Vec<String> = Vec::new();
        let native = vec!["Incarnon".to_string()];

        assert_eq!(can_incarnon("Burston", VARIANT_BASE, &empty), 1);
        assert_eq!(can_incarnon("Burston", VARIANT_PRIME, &empty), 1);
        assert_eq!(can_incarnon("Lex", VARIANT_BASE, &empty), 1);
        assert_eq!(can_incarnon("Braton", VARIANT_VANDAL, &empty), 1);
        assert_eq!(can_incarnon("Braton", VARIANT_MK1, &empty), 1);
        assert_eq!(can_incarnon("Synoid Gammacor", VARIANT_BASE, &empty), 1);
        assert_eq!(can_incarnon("Sancti Magistar", VARIANT_BASE, &empty), 1);
        assert_eq!(can_incarnon("Telos Boltor", VARIANT_BASE, &empty), 1);

        // 赤毒等特殊线不可装灵化之源
        assert_eq!(can_incarnon("Braton", VARIANT_KUVA, &empty), 0);
        // 无名单、无 tag
        assert_eq!(can_incarnon("Hema", VARIANT_BASE, &empty), 0);
        // 自带灵化 tag
        assert_eq!(can_incarnon("Felarx", VARIANT_BASE, &native), 1);
    }
