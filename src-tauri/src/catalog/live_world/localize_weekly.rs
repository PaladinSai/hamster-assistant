//! 周常码表：执政官 / 科研 / 回廊 / 日历。

use serde::Deserialize;

use super::localize::{humanize_camel_pub, mission_type_en, node_name};
use super::types::LabeledTerm;

/// 执政官猎杀首领 → userdict 英文键。
pub(crate) fn archon_boss_en(code: &str) -> String {
    match code {
        "SORTIE_BOSS_AMAR" => "Archon Amar".into(),
        "SORTIE_BOSS_BOREAL" => "Archon Boreal".into(),
        "SORTIE_BOSS_NIRA" => "Archon Nira".into(),
        _ => humanize_camel_pub(code.trim_start_matches("SORTIE_BOSS_")),
    }
}

pub(crate) fn archon_boss_fallback_zh(en: &str) -> String {
    match en {
        "Archon Amar" => "执刑官欺谋狼主".into(),
        "Archon Boreal" => "执刑官诡文枭主".into(),
        "Archon Nira" => "执刑官混沌蛇主".into(),
        _ => en.to_string(),
    }
}

/// 本周源力石 → userdict 英文键（由首领固定）。
pub(crate) fn archon_shard_en(boss_code: &str) -> String {
    match boss_code {
        "SORTIE_BOSS_AMAR" => "Crimson Archon Shard".into(),
        "SORTIE_BOSS_BOREAL" => "Azure Archon Shard".into(),
        "SORTIE_BOSS_NIRA" => "Amber Archon Shard".into(),
        _ => "Archon Shard".into(),
    }
}

pub(crate) fn archon_shard_fallback_zh(en: &str) -> String {
    match en {
        "Crimson Archon Shard" => "深红执刑官源力石".into(),
        "Azure Archon Shard" => "蔚蓝执刑官源力石".into(),
        "Amber Archon Shard" => "琥珀执刑官源力石".into(),
        "Archon Shard" => "执刑官源力石".into(),
        _ => en.to_string(),
    }
}

/// 兼容旧调用：无词典时回退到与 userdict 一致的中文。
pub(crate) fn archon_boss(code: &str) -> String {
    let en = archon_boss_en(code);
    archon_boss_fallback_zh(&en)
}

pub(crate) fn conquest_faction(code: &str) -> String {
    match code {
        // 游戏内 / 社区常用「低语者」；MITW = Murmur in the Walls
        "FC_MITW" => "低语者".into(),
        "FC_INFESTATION" => "Infested".into(),
        "FC_SCALDRA" => "炽蛇军".into(),
        "FC_TECHROT" => "科腐者".into(),
        "FC_GRINEER" => "Grineer".into(),
        "FC_CORPUS" => "Corpus".into(),
        "FC_OROKIN" => "Orokin".into(),
        _ => code.to_string(),
    }
}

/// 科研词条：browse.wf Language 查表；缺中文则英文原文。
pub(crate) fn conquest_deviation(
    kind: &str,
    code: &str,
    lang: Option<&super::conquest_i18n::ConquestLang>,
) -> LabeledTerm {
    match lang {
        Some(l) => l.deviation(kind, code),
        None => super::conquest_i18n::fallback_term(code),
    }
}

pub(crate) fn conquest_risk(
    code: &str,
    lang: Option<&super::conquest_i18n::ConquestLang>,
) -> LabeledTerm {
    match lang {
        Some(l) => l.risk(code),
        None => super::conquest_i18n::fallback_term(code),
    }
}

pub(crate) fn conquest_personal_mod(
    code: &str,
    lang: Option<&super::conquest_i18n::ConquestLang>,
) -> LabeledTerm {
    match lang {
        Some(l) => l.personal_mod(code),
        None => super::conquest_i18n::fallback_term(code),
    }
}

pub(crate) fn archimedea_kind(code: &str) -> String {
    match code {
        "CT_LAB" => "深层科研".into(),
        "CT_HEX" => "时光科研".into(),
        _ => code.to_string(),
    }
}

/// 科研任务类型：同一 MT_ 码在深层/时光下对应不同玩法名。
/// 深层 `MT_DEFENSE` = 镜像防御；时光 `MT_DEFENSE` = 舞台防御。
/// 时光生存玩法即地狱净化，社区卡片常仍标「生存」，此处用 Hell-Scrub 英文键，中文回退见 fallback。
pub(crate) fn archimedea_mission_type_en(kind: &str, mission_type_code: &str) -> String {
    match (kind, mission_type_code) {
        ("CT_LAB", "MT_DEFENSE") => "Mirror Defense".into(),
        ("CT_HEX", "MT_DEFENSE") => "Stage Defense".into(),
        // 时光生存 = 地狱净化；与沃沃等社区源一致时用「生存」展示名见 fallback「生存」键
        ("CT_HEX", "MT_SURVIVAL") => "Survival".into(),
        ("CT_HEX", "MT_ENDLESS_CAPTURE") => "Legacyte Harvest".into(),
        _ => mission_type_en(mission_type_code),
    }
}

pub(crate) fn mission_and_node(mission_type_code: &str, node_code: &str) -> (String, String) {
    (mission_type_en(mission_type_code), node_name(node_code))
}

/// 钢铁 / 无尽回廊轮换表（本地 JSON，与公开赛季表一致）。
#[derive(Debug, Deserialize)]
struct CircuitRotationsFile {
    steel: Vec<Vec<String>>,
    normal: Vec<Vec<String>>,
}

fn circuit_rotations() -> &'static CircuitRotationsFile {
    use std::sync::OnceLock;
    static ROT: OnceLock<CircuitRotationsFile> = OnceLock::new();
    ROT.get_or_init(|| {
        serde_json::from_str(include_str!("circuit_rotations.json"))
            .expect("circuit_rotations.json")
    })
}

pub(crate) fn steel_circuit_rotation() -> &'static [Vec<String>] {
    &circuit_rotations().steel
}

pub(crate) fn normal_circuit_rotation() -> &'static [Vec<String>] {
    &circuit_rotations().normal
}

pub(crate) fn display_circuit_choice(raw: &str) -> String {
    let pretty = match raw {
        "CeramicDagger" => "Ceramic Dagger",
        "DualToxocyst" => "Dual Toxocyst",
        "DualIchor" => "Dual Ichor",
        "AckAndBrunt" => "Ack & Brunt",
        "NamiSolo" => "Nami Solo",
        _ => raw,
    };
    pretty.to_string()
}

pub(crate) fn display_rotation_week(week: &[String]) -> Vec<String> {
    week.iter().map(|s| display_circuit_choice(s)).collect()
}

/// 匹配本周在轮换表中的下标；失败返回 None。
pub(crate) fn match_circuit_week_index(
    this_week: &[String],
    rotation: &[Vec<String>],
) -> Option<usize> {
    let norm: Vec<String> = this_week.iter().map(|s| normalize_choice(s)).collect();
    rotation.iter().position(|week| {
        let refs: Vec<&str> = week.iter().map(|s| s.as_str()).collect();
        week_matches(&norm, &refs)
    })
}

pub(crate) fn next_circuit_week(
    this_week: &[String],
    rotation: &[Vec<String>],
) -> Vec<String> {
    let Some(i) = match_circuit_week_index(this_week, rotation) else {
        return Vec::new();
    };
    let next = &rotation[(i + 1) % rotation.len()];
    display_rotation_week(next)
}

/// 根据本周 Choices 推断下一组钢铁灵化；匹配失败返回空。
pub(crate) fn next_steel_circuit_week(this_week: &[String]) -> Vec<String> {
    next_circuit_week(this_week, steel_circuit_rotation())
}

pub(crate) fn next_normal_circuit_week(this_week: &[String]) -> Vec<String> {
    next_circuit_week(this_week, normal_circuit_rotation())
}

fn week_matches(this_week: &[String], scheduled: &[&str]) -> bool {
    if this_week.len() != scheduled.len() {
        return false;
    }
    let mut a: Vec<_> = this_week.to_vec();
    let mut b: Vec<_> = scheduled.iter().map(|s| normalize_choice(s)).collect();
    a.sort();
    b.sort();
    a == b
}

fn normalize_choice(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

pub(crate) fn calendar_event_kind(code: &str) -> String {
    match code {
        "CET_CHALLENGE" => "challenge".into(),
        "CET_REWARD" => "reward".into(),
        "CET_UPGRADE" => "upgrade".into(),
        _ => code.to_string(),
    }
}

pub(crate) fn calendar_kind_label(kind: &str) -> String {
    match kind {
        "challenge" | "CET_CHALLENGE" => "挑战".into(),
        "reward" | "CET_REWARD" => "奖励".into(),
        "upgrade" | "CET_UPGRADE" => "增益".into(),
        _ => kind.to_string(),
    }
}

/// 路径 → (标题, 说明)。
/// 挑战/礼包优先官方词典；Hex 增益只出英文名（前端汇总为「增益」悬停提示）；
/// 已知物品奖励直接出区分清楚的中文名；其余英文供 apply 阶段 ZhLookup。
pub(crate) fn calendar_path_title(
    path: &str,
    challenge_i18n: Option<&super::challenge_i18n::ChallengeLocalizer>,
) -> (String, String) {
    if let Some(loc) = challenge_i18n {
        if let Some(pair) = loc.calendar_zh(path) {
            return pair;
        }
    }
    let base = path.rsplit('/').next().unwrap_or(path);
    // Hex Override：无公开中文 Language，标题用英文 humanize，无说明
    if path.contains("/Upgrades/Calendar/") {
        return (humanize_camel_pub(base), String::new());
    }
    if let Some((t, d)) = pack_reward_fallback(base) {
        return (t, d);
    }
    // 连接器等易混物品：标题直接写清战甲/武器、成品/蓝图
    if let Some((zh, _)) = item_reward_zh(base) {
        return (zh, String::new());
    }
    let en = reward_en_key(base);
    (en, String::new())
}

/// 已是中文的挑战/增益原样返回；英文物品名给静态中文兜底。
pub(crate) fn calendar_title_fallback_zh(title: &str) -> String {
    if title.chars().any(|c| c >= '\u{4e00}') {
        return title.to_string();
    }
    // 反查 reward_en_key 映射
    let basenames = [
        "ArchonCrystalBoreal",
        "ArchonCrystalAmar",
        "ArchonCrystalNira",
        "ArchonCrystalOrange",
        "ArchonCrystalGreen",
        "ArchonCrystalViolet",
        "OrokinReactorBlueprint",
        "OrokinCatalystBlueprint",
        "WeaponPrimaryArcaneUnlocker",
        "WeaponSecondaryArcaneUnlocker",
        "WeaponMeleeArcaneUnlocker",
        "WeaponUtilityUnlocker",
        "WeaponUtilityUnlockerBlueprint",
        "UtilityUnlocker",
        "UtilityUnlockerBlueprint",
        "ModDropChanceBooster3DayStoreItem",
        "ResourceDropChance3DayStoreItem",
        "Forma",
    ];
    for b in basenames {
        if reward_en_key(b) == title {
            if let Some((zh, _)) = item_reward_zh(b) {
                return zh;
            }
        }
    }
    if let Some((zh, _)) = item_reward_zh(title) {
        return zh;
    }
    title.to_string()
}

fn reward_en_key(base: &str) -> String {
    match base {
        "Forma" => "Forma".into(),
        "ArchonCrystalBoreal" => "Azure Archon Shard".into(),
        "ArchonCrystalAmar" => "Crimson Archon Shard".into(),
        "ArchonCrystalNira" => "Amber Archon Shard".into(),
        "ArchonCrystalOrange" => "Topaz Archon Shard".into(),
        "ArchonCrystalGreen" => "Emerald Archon Shard".into(),
        "ArchonCrystalViolet" => "Violet Archon Shard".into(),
        "OrokinReactorBlueprint" => "Orokin Reactor Blueprint".into(),
        "OrokinCatalystBlueprint" => "Orokin Catalyst Blueprint".into(),
        "WeaponPrimaryArcaneUnlocker" => "Primary Arcane Adapter".into(),
        "WeaponSecondaryArcaneUnlocker" => "Secondary Arcane Adapter".into(),
        "WeaponMeleeArcaneUnlocker" => "Melee Arcane Adapter".into(),
        "UtilityUnlocker" => "Warframe Exilus Adapter".into(),
        "UtilityUnlockerBlueprint" => "Warframe Exilus Adapter Blueprint".into(),
        "WeaponUtilityUnlocker" => "Weapon Exilus Adapter".into(),
        "WeaponUtilityUnlockerBlueprint" => "Weapon Exilus Adapter Blueprint".into(),
        "ModDropChanceBooster3DayStoreItem" => "Mod Drop Chance Booster".into(),
        "ResourceDropChance3DayStoreItem" => "Resource Drop Chance Booster".into(),
        other => humanize_camel_pub(other),
    }
}

/// dict.zh 未命中时的礼包中文回退（不含 Hex Override）。
fn pack_reward_fallback(base: &str) -> Option<(String, String)> {
    let table: &[(&str, &str, &str)] = &[
        ("CalendarArtifactPack", "赋能助力", "获得随机赋能"),
        (
            "CalendarMajorArtifactPack",
            "赋能助力：双重组合",
            "获得两枚随机赋能",
        ),
        ("CalendarRivenPack", "裂罅", "获得随机裂罅 MOD"),
        ("CalendarVosforPack", "沃斯福礼包", "获得沃斯福"),
        (
            "CircuitSilverSteelPathFusionBundle",
            "钢铁回廊银核融合包",
            "钢铁之路回廊银核奖励包",
        ),
    ];
    table
        .iter()
        .find(|(k, _, _)| *k == base)
        .map(|(_, t, d)| ((*t).to_string(), (*d).to_string()))
}

fn item_reward_zh(base: &str) -> Option<(String, String)> {
    let table: &[(&str, &str, &str)] = &[
        ("Forma", "Forma", "用于极化装备模组槽"),
        ("ArchonCrystalBoreal", "蔚蓝执刑官源力石", ""),
        ("ArchonCrystalAmar", "深红执刑官源力石", ""),
        ("ArchonCrystalNira", "琥珀执刑官源力石", ""),
        ("ArchonCrystalOrange", "黄玉执刑官源力石", ""),
        ("ArchonCrystalGreen", "翡翠执刑官源力石", ""),
        ("ArchonCrystalViolet", "紫晶执刑官源力石", ""),
        ("OrokinReactorBlueprint", "Orokin 反应堆蓝图", ""),
        ("OrokinCatalystBlueprint", "Orokin 催化剂蓝图", ""),
        (
            "WeaponPrimaryArcaneUnlocker",
            "主要武器赋能槽连接器",
            "为主要武器解锁赋能槽",
        ),
        (
            "WeaponSecondaryArcaneUnlocker",
            "次要武器赋能槽连接器",
            "为次要武器解锁赋能槽",
        ),
        (
            "WeaponMeleeArcaneUnlocker",
            "近战武器赋能槽连接器",
            "为近战武器解锁赋能槽",
        ),
        (
            "UtilityUnlocker",
            "战甲特殊功能槽连接器",
            "",
        ),
        (
            "UtilityUnlockerBlueprint",
            "战甲特殊功能槽连接器蓝图",
            "",
        ),
        (
            "WeaponUtilityUnlocker",
            "武器特殊功能槽连接器",
            "",
        ),
        (
            "WeaponUtilityUnlockerBlueprint",
            "武器特殊功能槽连接器蓝图",
            "",
        ),
        (
            "ModDropChanceBooster3DayStoreItem",
            "3 天 Mod 掉落几率加成",
            "领取后立即生效",
        ),
        (
            "ResourceDropChance3DayStoreItem",
            "3 天资源掉落几率加成",
            "领取后立即生效",
        ),
        (
            "AffinityBooster3DayStoreItem",
            "3 天经验值加成",
            "领取后立即生效",
        ),
    ];
    table
        .iter()
        .find(|(k, _, _)| *k == base)
        .map(|(_, t, d)| ((*t).to_string(), (*d).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_week_from_group_d() {
        let this = ["Lex", "Magistar", "Boltor", "Bronco", "CeramicDagger"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let next = next_steel_circuit_week(&this);
        assert!(next.iter().any(|s| s.contains("Torid")), "{next:?}");
    }

    #[test]
    fn normal_circuit_next_from_week2() {
        let this = ["Loki", "Mag", "Rhino"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let next = next_normal_circuit_week(&this);
        assert!(next.iter().any(|s| s == "Ash"), "{next:?}");
        assert_eq!(normal_circuit_rotation().len(), 11);
        assert_eq!(steel_circuit_rotation().len(), 9);
    }

    #[test]
    fn archon_names_and_shards_match_userdict_keys() {
        assert_eq!(archon_boss_en("SORTIE_BOSS_AMAR"), "Archon Amar");
        assert_eq!(archon_boss("SORTIE_BOSS_AMAR"), "执刑官欺谋狼主");
        assert_eq!(
            archon_shard_en("SORTIE_BOSS_NIRA"),
            "Amber Archon Shard"
        );
        assert_eq!(
            archon_shard_fallback_zh("Amber Archon Shard"),
            "琥珀执刑官源力石"
        );
    }

    #[test]
    fn archimedea_mission_types_are_contextual() {
        assert_eq!(
            archimedea_mission_type_en("CT_LAB", "MT_DEFENSE"),
            "Mirror Defense"
        );
        assert_eq!(
            archimedea_mission_type_en("CT_HEX", "MT_DEFENSE"),
            "Stage Defense"
        );
        assert_eq!(
            archimedea_mission_type_en("CT_HEX", "MT_SURVIVAL"),
            "Survival"
        );
        assert_eq!(
            archimedea_mission_type_en("CT_HEX", "MT_ENDLESS_CAPTURE"),
            "Legacyte Harvest"
        );
        assert_eq!(
            archimedea_mission_type_en("CT_LAB", "MT_ARTIFACT"),
            "Disruption"
        );
    }

    #[test]
    fn archimedea_terms_without_lang_are_english_fallback() {
        let d = conquest_deviation("CT_LAB", "NecramechActivation", None);
        assert!(d.title.contains("Necramech") || d.title.contains("Activation"), "{d:?}");
        assert!(d.description.is_empty());

        let unknown = conquest_risk("/Lotus/Foo/UnknownRiskThing", None);
        assert!(unknown.title.contains("Unknown") || !unknown.title.is_empty());
        assert!(unknown.description.is_empty());
    }

    #[test]
    fn archimedea_terms_with_lang_prefer_zh() {
        let lang = super::super::conquest_i18n::ConquestLang::from_bytes(
            r#"{
              "/Lotus/Language/Conquest/MissionVariant_LabConquest_NecramechActivation": "殁世机甲激增",
              "/Lotus/Language/Conquest/MissionVariant_LabConquest_NecramechActivation_Desc": "殁世机甲更频繁。",
              "/Lotus/Language/Conquest/Condition_PointBlank": "短视弹药",
              "/Lotus/Language/Conquest/Condition_PointBlank_Desc": "15 米。",
              "/Lotus/Language/Conquest/PersonalMod_Starvation": "弹药亏空",
              "/Lotus/Language/Conquest/PersonalMod_Starvation_Desc": "少弹药。"
            }"#
            .as_bytes(),
            None,
        )
        .unwrap();
        let d = conquest_deviation("CT_LAB", "NecramechActivation", Some(&lang));
        assert!(d.title.contains("殁世"), "{d:?}");
        assert!(!d.description.is_empty());
        assert!(conquest_risk("PointBlank", Some(&lang)).title.contains("短视"));
        assert!(conquest_personal_mod("Starvation", Some(&lang)).title.contains("弹药"));
    }

    #[test]
    fn calendar_challenge_quota_and_reward_zh() {
        let ch = r#"{
          "/Lotus/Types/Challenges/Calendar1999/CalendarKillTechrotEnemiesHard": {
            "name": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesTechrot_Name",
            "description": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesTechrot_Description",
            "requiredCount": 750
          },
          "/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy": {
            "name": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name",
            "description": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description",
            "requiredCount": 10
          },
          "/Lotus/Types/Challenges/Calendar1999/CalendarKillScaldraEnemiesWithAbilitiesMedium": {
            "name": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesWithAbilitiesScaldra_Name",
            "description": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesWithAbilitiesScaldra_Description",
            "requiredCount": 150
          }
        }"#;
        let dict = r#"{
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesTechrot_Name": "净化感染",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesTechrot_Description": "击杀 |COUNT| 名科腐者",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name": "逝去卓越者",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description": "击杀 |COUNT| 名卓越者",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesWithAbilitiesScaldra_Name": "敲山震虎",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEnemiesWithAbilitiesScaldra_Description": "使用技能击杀 |COUNT| 名炽蛇军敌人",
          "/Lotus/Language/1999/CalendarArtifactPackName": "赋能助力",
          "/Lotus/Language/1999/CalendarArtifactPackDesc": "获得一枚随机赋能。"
        }"#;
        let loc = super::super::challenge_i18n::ChallengeLocalizer::from_bytes(
            ch.as_bytes(),
            dict.as_bytes(),
        )
        .unwrap();

        let (t, d) = calendar_path_title(
            "/Lotus/Types/Challenges/Calendar1999/CalendarKillTechrotEnemiesHard",
            Some(&loc),
        );
        assert_eq!(t, "净化感染");
        assert!(d.contains("750"), "{d}");
        assert!(d.contains("科腐者"), "{d}");
        let (easy, ed) = calendar_path_title(
            "/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy",
            Some(&loc),
        );
        assert_eq!(easy, "逝去卓越者");
        assert!(ed.contains("10"), "{ed}");
        assert!(ed.contains("卓越者"), "{ed}");
        let (abil, ad) = calendar_path_title(
            "/Lotus/Types/Challenges/Calendar1999/CalendarKillScaldraEnemiesWithAbilitiesMedium",
            Some(&loc),
        );
        assert_eq!(abil, "敲山震虎");
        assert!(ad.contains("150"), "{ad}");
        let (pack, pd) = calendar_path_title(
            "/Lotus/Types/Items/MiscItems/CalendarArtifactPack",
            Some(&loc),
        );
        assert_eq!(pack, "赋能助力");
        assert!(pd.contains("赋能"), "{pd}");

        let (rt, _) = calendar_path_title(
            "/Lotus/StoreItems/Types/Gameplay/NarmerSorties/ArchonCrystalBoreal",
            None,
        );
        assert!(rt.contains("蔚蓝"), "{rt}");
        assert!(calendar_title_fallback_zh("Azure Archon Shard").contains("蔚蓝"));
        let (ft, _) = calendar_path_title("/Lotus/StoreItems/Types/Items/Forma", None);
        assert_eq!(ft, "Forma");
        let (up, ud) = calendar_path_title(
            "/Lotus/Upgrades/Calendar/ElectricStatusDamageAndChance",
            None,
        );
        assert!(up.contains("Electric") || up.contains("Status"), "{up}");
        assert!(ud.is_empty(), "{ud}");
        let (exilus, _) = calendar_path_title(
            "/Lotus/StoreItems/Types/Items/MiscItems/UtilityUnlocker",
            None,
        );
        assert_eq!(exilus, "战甲特殊功能槽连接器");
        let (wex, _) = calendar_path_title(
            "/Lotus/StoreItems/Types/Items/MiscItems/WeaponUtilityUnlockerBlueprint",
            None,
        );
        assert_eq!(wex, "武器特殊功能槽连接器蓝图");
        assert_eq!(calendar_kind_label("challenge"), "挑战");
        assert_eq!(calendar_kind_label("reward"), "奖励");
        assert_eq!(calendar_kind_label("upgrade"), "增益");
    }
}
