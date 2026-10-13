//! 官方内部码 → 中文展示。

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::catalog::i18n::ZhLookup;

/// 突击首领。
pub(crate) fn sortie_boss(code: &str) -> (String, String) {
    let zh = lookup(
        code,
        &[
            ("SORTIE_BOSS_VOR", "沃尔队长"),
            ("SORTIE_BOSS_KRIL", "利桓中尉"),
            ("SORTIE_BOSS_TYLOR", "泰尔中尉"),
            ("SORTIE_BOSS_ROUTA", "督军"),
            ("SORTIE_BOSS_ALAD", "阿拉德 V"),
            ("SORTIE_BOSS_AMBULAS", "阿姆帕拉斯"),
            ("SORTIE_BOSS_RAPTOR", "猛禽"),
            ("SORTIE_BOSS_HYENA", "鬣狗群"),
            ("SORTIE_BOSS_NEF", "奈弗·安悠"),
            ("SORTIE_BOSS_LECH_KRIL", "利桓中尉"),
            ("SORTIE_BOSS_KELA", "凯菈·德·赛姆"),
            ("SORTIE_BOSS_HEK", "维·赫克议员"),
            ("SORTIE_BOSS_PHORID", "弗瑞德"),
            ("SORTIE_BOSS_LEPHANTIS", "利凡提斯"),
            ("SORTIE_BOSS_JORDAS", "卓达斯魔尸体"),
            ("SORTIE_BOSS_CORRUPTED_VOR", "堕落的沃尔"),
        ],
    );
    let faction = boss_faction(code);
    (zh.unwrap_or_else(|| humanize_code(code)), faction)
}

fn boss_faction(code: &str) -> String {
    if code.contains("ALAD")
        || code.contains("AMBULAS")
        || code.contains("RAPTOR")
        || code.contains("HYENA")
        || code.contains("NEF")
    {
        return "Corpus".into();
    }
    if code.contains("PHORID") || code.contains("LEPHANTIS") || code.contains("JORDAS") {
        return "Infested".into();
    }
    if code.contains("CORRUPTED") {
        return "Orokin".into();
    }
    "Grineer".into()
}

pub(crate) fn mission_type(code: &str) -> String {
    // 兼容旧调用：无 ZhLookup 时用英文键对应的中文回退
    let en = mission_type_en(code);
    mission_type_fallback_zh(&en)
}

/// 内部码 → userdict / name_i18n 用的英文键。
pub(crate) fn mission_type_en(code: &str) -> String {
    match code {
        "MT_EXTERMINATION" => "Exterminate".into(),
        "MT_SURVIVAL" => "Survival".into(),
        "MT_RESCUE" => "Rescue".into(),
        "MT_SABOTAGE" => "Sabotage".into(),
        "MT_INTEL" | "MT_SPY" => "Spy".into(),
        "MT_DEFENSE" => "Defense".into(),
        "MT_MOBILE_DEFENSE" => "Mobile Defense".into(),
        "MT_TERRITORY" => "Interception".into(),
        "MT_RETRIEVAL" => "Hijack".into(),
        "MT_ASSAULT" => "Assault".into(),
        "MT_EXCAVATE" => "Excavation".into(),
        "MT_HIVE" => "Hive".into(),
        "MT_CAPTURE" => "Capture".into(),
        "MT_ASSASSINATION" => "Assassination".into(),
        "MT_PURSUIT" => "Pursuit".into(),
        "MT_RACE" => "Rush".into(),
        "MT_SALVAGE" => "Infested Salvage".into(),
        "MT_EVACUATION" => "Defection".into(),
        "MT_ARENA" => "Arena".into(),
        "MT_LANDSCAPE" => "Free Roam".into(),
        "MT_ALCHEMY" => "Alchemy".into(),
        "MT_ENDLESS_CAPTURE" => "Legacyte Harvest".into(),
        "MT_CORRUPTION" => "Void Flood".into(),
        "MT_VOID_CASCADE" => "Void Cascade".into(),
        "MT_ARMAGEDDON" => "Void Armageddon".into(),
        // Artifact 在词典是「神器MOD」，中断必须用 Disruption
        "MT_ARTIFACT" | "MT_DISRUPTION" => "Disruption".into(),
        _ => humanize_code(code),
    }
}

pub(crate) fn mission_type_fallback_zh(en: &str) -> String {
    match en {
        "Exterminate" => "歼灭".into(),
        "Survival" => "生存".into(),
        "Rescue" => "救援".into(),
        "Sabotage" => "破坏".into(),
        "Spy" => "间谍".into(),
        "Defense" => "防御".into(),
        "Mirror Defense" => "镜像防御".into(),
        "Stage Defense" => "舞台防御".into(),
        "Mobile Defense" => "移动防御".into(),
        "Interception" => "拦截".into(),
        "Hijack" => "劫持".into(),
        "Assault" => "突击（奥罗金）".into(),
        "Excavation" => "挖掘".into(),
        "Hive" => "巢穴".into(),
        "Capture" => "捕获".into(),
        "Assassination" => "刺杀".into(),
        "Pursuit" => "追击".into(),
        "Rush" => "竞速".into(),
        "Infested Salvage" => "打捞".into(),
        "Defection" => "撤离".into(),
        "Arena" => "竞技场".into(),
        "Free Roam" => "开放世界".into(),
        "Alchemy" => "炼金".into(),
        "Legacyte Harvest" => "传承种收割".into(),
        "Hell-Scrub" => "地狱净化".into(),
        "Void Flood" => "虚空洪流".into(),
        "Void Cascade" => "虚空惊涛".into(),
        "Void Armageddon" => "虚空大灾变".into(),
        "Disruption" => "中断".into(),
        _ => en.to_string(),
    }
}

/// 有词典用词典，否则用 fallback。
pub(crate) fn resolve_zh(zh: Option<&ZhLookup>, en: &str, fallback: &str) -> String {
    zh.and_then(|z| z.get(en))
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn resolve_mission_type(zh: Option<&ZhLookup>, code_or_en: &str) -> String {
    let en = if code_or_en.starts_with("MT_") {
        mission_type_en(code_or_en)
    } else {
        code_or_en.to_string()
    };
    let fb = mission_type_fallback_zh(&en);
    resolve_zh(zh, &en, &fb)
}

pub(crate) fn sortie_modifier(code: &str) -> String {
    lookup(
        code,
        &[
            ("SORTIE_MODIFIER_HAZARD_RADIATION", "环境灾害：辐射"),
            ("SORTIE_MODIFIER_HAZARD_FIRE", "环境灾害：火焰"),
            ("SORTIE_MODIFIER_HAZARD_ICE", "环境灾害：冰冻"),
            ("SORTIE_MODIFIER_HAZARD_COLD", "环境灾害：寒冷"),
            ("SORTIE_MODIFIER_HAZARD_FOG", "环境灾害：浓雾"),
            ("SORTIE_MODIFIER_HAZARD_MAGNETIC", "环境灾害：磁力"),
            ("SORTIE_MODIFIER_HAZARD_CASINGS", "环境灾害：弹药效能"),
            ("SORTIE_MODIFIER_FIRE", "敌人元素强化：火焰"),
            ("SORTIE_MODIFIER_ICE", "敌人元素强化：冰冻"),
            ("SORTIE_MODIFIER_ELECTRICITY", "敌人元素强化：电击"),
            ("SORTIE_MODIFIER_POISON", "敌人元素强化：毒素"),
            ("SORTIE_MODIFIER_RADIATION", "敌人元素强化：辐射"),
            ("SORTIE_MODIFIER_MAGNETIC", "敌人元素强化：磁力"),
            ("SORTIE_MODIFIER_VIRAL", "敌人元素强化：病毒"),
            ("SORTIE_MODIFIER_CORROSIVE", "敌人元素强化：腐蚀"),
            ("SORTIE_MODIFIER_BLAST", "敌人元素强化：爆炸"),
            ("SORTIE_MODIFIER_GAS", "敌人元素强化：毒气"),
            ("SORTIE_MODIFIER_LOW_ENERGY", "能量削减"),
            ("SORTIE_MODIFIER_EXIMUS", "卓越者丰收"),
            ("SORTIE_MODIFIER_ARMOR", "敌人护甲强化"),
            ("SORTIE_MODIFIER_SHIELDS", "敌人护盾强化"),
            ("SORTIE_MODIFIER_SECONDARY_ONLY", "仅限副武器"),
            ("SORTIE_MODIFIER_MELEE_ONLY", "仅限近战"),
            ("SORTIE_MODIFIER_BOW_ONLY", "仅限弓"),
            ("SORTIE_MODIFIER_SNIPER_ONLY", "仅限狙击枪"),
            ("SORTIE_MODIFIER_RIFLE_ONLY", "仅限步枪"),
            ("SORTIE_MODIFIER_SHOTGUN_ONLY", "仅限霰弹枪"),
        ],
    )
    .unwrap_or_else(|| humanize_code(code))
}

#[derive(Debug, Deserialize)]
struct SolNodeEntry {
    value: Option<String>,
    #[serde(rename = "systemName")]
    system_name: Option<String>,
}

fn sol_nodes() -> &'static HashMap<String, SolNodeEntry> {
    static MAP: OnceLock<HashMap<String, SolNodeEntry>> = OnceLock::new();
    MAP.get_or_init(|| {
        serde_json::from_str(include_str!("sol_nodes.json")).unwrap_or_default()
    })
}

fn planet_zh(en: &str) -> String {
    lookup(
        en,
        &[
            ("Mercury", "水星"),
            ("Venus", "金星"),
            ("Earth", "地球"),
            ("Lua", "月球"),
            ("Mars", "火星"),
            ("Phobos", "火卫一"),
            ("Deimos", "火卫二"),
            ("Ceres", "谷神星"),
            ("Jupiter", "木星"),
            ("Europa", "木卫二"),
            ("Saturn", "土星"),
            ("Uranus", "天王星"),
            ("Neptune", "海王星"),
            ("Pluto", "冥王星"),
            ("Eris", "阋神星"),
            ("Sedna", "赛德娜"),
            ("Void", "虚空"),
            ("Kuva Fortress", "赤毒要塞"),
            ("Zariman", "扎里曼"),
            ("Duviri", "都维利"),
            ("Veil", "面纱"),
            ("Hollvania", "霍瓦尼亚"),
        ],
    )
    .unwrap_or_else(|| en.to_string())
}

const NODE_SEP: &str = " · ";

/// 星图节点英文地名；缺表 / 无地名则 None。
pub(crate) fn sol_node_place_name(code: &str) -> Option<String> {
    let entry = sol_nodes().get(code)?;
    let loc = entry.value.as_deref()?.trim();
    if loc.is_empty() {
        None
    } else {
        Some(loc.to_string())
    }
}

/// 节点：`地点 · 星球`；缺表则保留原码。
pub(crate) fn node_name(code: &str) -> String {
    let Some(entry) = sol_nodes().get(code) else {
        return code.to_string();
    };
    let loc = entry.value.as_deref().unwrap_or("").trim();
    let planet_en = entry.system_name.as_deref().unwrap_or("").trim();
    if loc.is_empty() && planet_en.is_empty() {
        return code.to_string();
    }
    if planet_en.is_empty() {
        return loc.to_string();
    }
    if loc.is_empty() {
        return planet_zh(planet_en);
    }
    format!("{loc}{NODE_SEP}{}", planet_zh(planet_en))
}

/// 星图地点名：与客户端星图一致，保持 sol_nodes 英文地名。
/// 不查 name_i18n：词典里 `War`/`Hate`/`Despair` 等是武器，会误伤节点。
pub(crate) fn apply_place_zh(display: &str, _zh: Option<&ZhLookup>) -> String {
    display.to_string()
}

/// 电波挑战 → (标题, 描述)。
///
/// 优先 browse.wf `ExportChallenges` + `dict.zh`；未命中再回退手写表 / CamelCase。
pub(crate) fn nightwave_challenge(
    path: &str,
    dict: Option<&super::challenge_i18n::ChallengeLocalizer>,
) -> (String, String) {
    if let Some(loc) = dict {
        if let Some((title, desc)) = loc.nightwave_zh(path) {
            if desc.is_empty() {
                let (_, fallback_desc) = nightwave_challenge_builtin(path);
                return (title, fallback_desc);
            }
            return (title, desc);
        }
    }
    nightwave_challenge_builtin(path)
}

/// 手写回退表（词典未就绪时）。
fn nightwave_challenge_builtin(path: &str) -> (String, String) {
    let base = path.rsplit('/').next().unwrap_or(path);
    if let Some((title, desc)) = lookup_pair(
        base,
        &[
            (
                "SeasonDailyDonateLeverian",
                "赞助人",
                "向 Leverian 博物馆捐献物品",
            ),
            (
                "SeasonDailyKillEnemiesWithRadiation",
                "反应堆",
                "用辐射伤害击杀 150 名敌人",
            ),
            (
                "SeasonDailyCodexScan",
                "研究者",
                "扫描 15 次物件或敌人",
            ),
            (
                "SeasonDailyKillEnemies",
                "清剿",
                "击杀一定数量的敌人",
            ),
            (
                "SeasonDailyCompleteMission",
                "任务达人",
                "完成任意任务",
            ),
            (
                "SeasonDailyPlayAMission",
                "出击",
                "进行一次任务",
            ),
            (
                "SeasonWeeklyPermanentCompleteMissions8",
                "任务完成 VIII",
                "完成任意 15 个任务",
            ),
            (
                "SeasonWeeklyPermanentKillEximus8",
                "卓越者终结者 VIII",
                "击杀 30 名卓越者",
            ),
            (
                "SeasonWeeklyPermanentKillEnemies8",
                "不是警告射击 VIII",
                "击杀 500 名敌人",
            ),
            (
                "SeasonWeeklyOpenLockers",
                "盗窃狂",
                "打开 30 个储物柜",
            ),
            (
                "SeasonWeeklyCatchRarePlainsFish",
                "地球渔夫",
                "在夜灵平野捕获 6 条稀有鱼",
            ),
            (
                "SeasonWeeklyHardTerminated",
                "已终止",
                "摧毁 3 个尸鬼金库守卫",
            ),
            (
                "SeasonWeeklyHardCompleteConquest",
                "自愿样本",
                "完成一轮深层科研或时光科研",
            ),
        ],
    ) {
        return (title, desc);
    }
    (humanize_camel(base), String::new())
}

pub(crate) fn is_elite_challenge(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.contains("Hard") || base.contains("Elite")
}

fn lookup(key: &str, table: &[(&str, &str)]) -> Option<String> {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| (*v).to_string())
}

fn lookup_pair(key: &str, table: &[(&str, &str, &str)]) -> Option<(String, String)> {
    table
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, t, d)| ((*t).to_string(), (*d).to_string()))
}

fn humanize_code(code: &str) -> String {
    code.trim_start_matches("SORTIE_BOSS_")
        .trim_start_matches("SORTIE_MODIFIER_")
        .trim_start_matches("MT_")
        .replace('_', " ")
}

fn humanize_camel(s: &str) -> String {
    humanize_camel_pub(s)
}

pub(crate) fn humanize_camel_pub(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out.replace("Season Daily ", "")
        .replace("Season Weekly Permanent ", "")
        .replace("Season Weekly Hard ", "")
        .replace("Season Weekly ", "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_name_maps_place_and_planet() {
        let n = node_name("SolNode149");
        assert!(n.contains("Casta"), "{n}");
        assert!(n.contains("谷神星"), "{n}");
        assert!(!n.starts_with("SolNode"), "{n}");
    }

    #[test]
    fn node_name_unknown_keeps_code() {
        assert_eq!(node_name("SolNode99999"), "SolNode99999");
    }

    #[test]
    fn apply_place_zh_without_dict_keeps() {
        let s = "Casta · 谷神星";
        assert_eq!(apply_place_zh(s, None), s);
    }

    #[test]
    fn disruption_codes_map_to_en_and_zh_fallback() {
        assert_eq!(mission_type_en("MT_ARTIFACT"), "Disruption");
        assert_eq!(mission_type_en("MT_DISRUPTION"), "Disruption");
        assert_eq!(mission_type("MT_ARTIFACT"), "中断");
        assert_eq!(resolve_mission_type(None, "Disruption"), "中断");
    }

    #[test]
    fn apply_place_zh_keeps_star_chart_war() {
        // 地点不走 item 词典（War 武器≠星图节点 War）
        let s = "War · 火星";
        assert_eq!(apply_place_zh(s, None), s);
        assert_eq!(node_name("SolNode99"), "War · 火星");
    }
}
