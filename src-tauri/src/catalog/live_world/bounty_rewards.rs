//! 从 WFCD drop-all 赏金表为任务条挂奖励名（中文优先）。

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

use crate::catalog::i18n::ZhLookup;
use crate::catalog::upstream_cache;
use crate::db;

use super::types::{BountyBriefApi, BountyJobApi};

#[derive(Debug, Deserialize)]
struct DropBountyTables {
    #[serde(rename = "cetusBountyRewards", default)]
    cetus: Vec<DropBountyTier>,
    #[serde(rename = "solarisBountyRewards", default)]
    solaris: Vec<DropBountyTier>,
    #[serde(rename = "deimosRewards", default)]
    deimos: Vec<DropBountyTier>,
    #[serde(rename = "zarimanRewards", default)]
    zariman: Vec<DropBountyTier>,
    #[serde(rename = "entratiLabRewards", default)]
    entrati_lab: Vec<DropBountyTier>,
    #[serde(rename = "hexRewards", default)]
    hex: Vec<DropBountyTier>,
}

#[derive(Debug, Deserialize)]
struct DropBountyTier {
    #[serde(rename = "bountyLevel", default)]
    bounty_level: String,
    #[serde(default)]
    rewards: HashMap<String, Vec<DropRewardItem>>,
}

#[derive(Debug, Deserialize)]
struct DropRewardItem {
    #[serde(rename = "itemName", default)]
    item_name: String,
}

/// 打包 userdict `Text` 映射（EN→ZH），作 name_i18n 回退。
fn bundled_userdict() -> &'static HashMap<String, String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        let path = db::bundled_resource_path("userdict.json");
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return HashMap::new();
        };
        let Ok(root) = serde_json::from_str::<Value>(&raw) else {
            return HashMap::new();
        };
        let Some(text) = root.get("Text").and_then(|v| v.as_object()) else {
            return HashMap::new();
        };
        let mut map = HashMap::with_capacity(text.len());
        for (en, zh_val) in text {
            let Some(zh) = zh_val.as_str() else {
                continue;
            };
            let en = en.trim();
            let zh = zh.trim();
            if en.is_empty() || zh.is_empty() {
                continue;
            }
            map.insert(en.to_string(), zh.to_string());
        }
        map
    })
}

/// 词典缺键或拼写不一致时的顽固别名（少量）。
const REWARD_ALIASES: &[(&str, &str)] = &[
    ("Byteryte", "字节卫士"), // drop-all 拼写；userdict 为 ByteRyte
    ("ByteRyte", "字节卫士"),
];

/// 为赏金板任务填充 `rewards`（英文名 → 尽量中文化）。
pub(crate) fn attach_drop_rewards(
    data_dir: &Path,
    brief: &mut BountyBriefApi,
    zh: Option<&ZhLookup>,
) {
    let Some(tables) = cached_drop_bounty_tables(data_dir) else {
        return;
    };
    let userdict = bundled_userdict();

    for board in &mut brief.boards {
        let tiers = tiers_for_tag(tables, &board.tag);
        if tiers.is_empty() {
            continue;
        }
        // 同板任务共用一轮换；个别 job 缺路径时用板内已解析字母兜底
        let board_rotation = board
            .jobs
            .iter()
            .find_map(|j| parse_reward_rotation(&j.reward_table));
        for job in &mut board.jobs {
            if !job.rewards.is_empty() {
                continue;
            }
            let rotation =
                parse_reward_rotation(&job.reward_table).or(board_rotation);
            let items = match_tier_items(&board.tag, tiers, job, rotation);
            if items.is_empty() {
                continue;
            }
            job.rewards = items
                .into_iter()
                .map(|en| localize_reward_name(&en, zh, userdict))
                .collect();
        }
    }
}

/// 对齐 WFCD：`TierXTableYRewards` / `NarmerTableYRewards` / `GhoulBountyTableYRewards` → 轮换 Y。
fn parse_reward_rotation(reward_table: &str) -> Option<char> {
    let last = reward_table.rsplit('/').next().unwrap_or(reward_table);
    let lower = last.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("narmertable") {
        return rotation_letter(rest);
    }
    if let Some(rest) = lower.strip_prefix("ghoulbountytable") {
        return rotation_letter(rest);
    }
    // VenusTierATableBRewards / TierATableBRewards
    let tier_at = lower.rfind("tier")?;
    let after_tier = &lower[tier_at + "tier".len()..];
    let mut chars = after_tier.chars();
    let tier_letter = chars.next()?;
    if !matches!(tier_letter, 'a' | 'b' | 'c' | 'd' | 'e') {
        return None;
    }
    let rest: String = chars.collect();
    let after_table = rest.strip_prefix("table")?;
    rotation_letter(after_table)
}

fn rotation_letter(after_table: &str) -> Option<char> {
    let ch = after_table.chars().next()?;
    if matches!(ch, 'a' | 'b' | 'c') && after_table[ch.len_utf8()..].starts_with("rewards") {
        Some(ch.to_ascii_uppercase())
    } else {
        None
    }
}

fn cached_drop_bounty_tables(data_dir: &Path) -> Option<&'static DropBountyTables> {
    static CACHE: OnceLock<Option<DropBountyTables>> = OnceLock::new();
    // OnceLock 只初始化一次：进程内复用解析结果，避免每次刷新都解析 ~6MB JSON
    let cached = CACHE.get_or_init(|| {
        let raw = load_drop_all_bytes(data_dir)?;
        match serde_json::from_slice::<DropBountyTables>(&raw) {
            Ok(tables) => Some(tables),
            Err(_) => {
                eprintln!("[仓鼠] drop-all 赏金奖励表解析失败");
                None
            }
        }
    });
    cached.as_ref()
}

fn load_drop_all_bytes(data_dir: &Path) -> Option<Vec<u8>> {
    if let Ok(Some(b)) = upstream_cache::read(data_dir, upstream_cache::DROP_ALL) {
        return Some(b);
    }
    let path = db::bundled_resource_path("drop-all.json");
    std::fs::read(path).ok()
}

fn tiers_for_tag<'a>(tables: &'a DropBountyTables, tag: &str) -> &'a [DropBountyTier] {
    match tag {
        "CetusSyndicate" => tables.cetus.as_slice(),
        "SolarisSyndicate" => tables.solaris.as_slice(),
        "EntratiSyndicate" => tables.deimos.as_slice(),
        "ZarimanSyndicate" => tables.zariman.as_slice(),
        "EntratiLabSyndicate" => tables.entrati_lab.as_slice(),
        "HexSyndicate" => tables.hex.as_slice(),
        _ => &[],
    }
}

/// 六人组：游戏内等级比官方 drop-all 标签高约 10，匹配时用降档后的等级。
fn drop_match_levels(tag: &str, job: &BountyJobApi) -> (i32, i32) {
    let (min, max) = (job.min_enemy_level, job.max_enemy_level);
    if tag == "HexSyndicate" {
        (min - 10, max - 10)
    } else {
        (min, max)
    }
}

fn match_tier_items(
    tag: &str,
    tiers: &[DropBountyTier],
    job: &BountyJobApi,
    rotation: Option<char>,
) -> Vec<String> {
    let (job_lo, job_hi) = drop_match_levels(tag, job);
    let candidates: Vec<&DropBountyTier> = tiers
        .iter()
        .filter(|t| tier_eligible_for_job(t, job))
        .collect();
    if let Some(t) = candidates
        .iter()
        .find(|t| levels_match(&t.bounty_level, job_lo, job_hi))
    {
        return unique_items(t, rotation);
    }
    let mut best: Option<&DropBountyTier> = None;
    let mut best_dist = i32::MAX;
    for t in candidates {
        let Some((lo, hi)) = parse_level_range(&t.bounty_level) else {
            continue;
        };
        let dist = (lo - job_lo).abs() + (hi - job_hi).abs();
        if dist < best_dist {
            best_dist = dist;
            best = Some(t);
        }
    }
    best.map(|t| unique_items(t, rotation)).unwrap_or_default()
}

/// 英择谛 drop-all 混有隔离密室 / Arcana；野外赏金只匹配 Cambion Drift Bounty。
fn tier_eligible_for_job(tier: &DropBountyTier, job: &BountyJobApi) -> bool {
    let level = tier.bounty_level.to_ascii_lowercase();
    let is_vaultish = level.contains("isolation")
        || level.contains("arcana")
        || level.contains("vault");
    if !is_vaultish {
        return true;
    }
    // vault 任务本身目前在 parse 阶段已跳过；此处再挡一层避免等级邻近误匹配
    let jt = job.job_type.to_ascii_lowercase();
    jt.contains("vault") || jt.contains("isolation") || jt.contains("arcana")
}

fn levels_match(bounty_level: &str, job_lo: i32, job_hi: i32) -> bool {
    let Some((lo, hi)) = parse_level_range(bounty_level) else {
        return false;
    };
    lo == job_lo && hi == job_hi
}

fn parse_level_range(s: &str) -> Option<(i32, i32)> {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            if let Ok(n) = cur.parse::<i32>() {
                nums.push(n);
            }
            cur.clear();
            if nums.len() >= 2 {
                break;
            }
        }
    }
    if !cur.is_empty() {
        if let Ok(n) = cur.parse::<i32>() {
            nums.push(n);
        }
    }
    if nums.len() >= 2 {
        Some((nums[0], nums[1]))
    } else {
        None
    }
}

fn unique_items(tier: &DropBountyTier, rotation: Option<char>) -> Vec<String> {
    let mut out = Vec::new();
    for k in reward_pool_keys(tier, rotation) {
        let Some(list) = tier.rewards.get(&k) else {
            continue;
        };
        for it in list {
            let name = it.item_name.trim();
            if name.is_empty() {
                continue;
            }
            if !out.iter().any(|x| x == name) {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// 只取当前轮换池；无字母时若 A/B/C 仅一个非空则用它（扎里曼等），绝不合并三池。
fn reward_pool_keys(tier: &DropBountyTier, rotation: Option<char>) -> Vec<String> {
    if let Some(r) = rotation {
        let k = r.to_ascii_uppercase().to_string();
        if tier
            .rewards
            .get(&k)
            .map(|l| !l.is_empty())
            .unwrap_or(false)
        {
            return vec![k];
        }
    }
    let nonempty_abc: Vec<String> = ["A", "B", "C"]
        .iter()
        .filter(|k| {
            tier.rewards
                .get(**k)
                .map(|l| !l.is_empty())
                .unwrap_or(false)
        })
        .map(|s| (*s).to_string())
        .collect();
    if nonempty_abc.len() == 1 {
        return nonempty_abc;
    }
    if nonempty_abc.is_empty() {
        return tier
            .rewards
            .iter()
            .filter(|(k, l)| {
                !matches!(k.as_str(), "A" | "B" | "C") && !l.is_empty()
            })
            .map(|(k, _)| k.clone())
            .collect();
    }
    // 有多池但缺路径：宁可只展示一池，也不再拼 A+B+C
    vec![nonempty_abc[0].clone()]
}

fn lookup_en(en: &str, zh: Option<&ZhLookup>, userdict: &HashMap<String, String>) -> Option<String> {
    let t = en.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(z) = zh.and_then(|z| z.get(t)) {
        return Some(z);
    }
    if let Some(z) = userdict.get(t) {
        return Some(z.clone());
    }
    // 大小写不敏感（userdict）
    let lower = t.to_ascii_lowercase();
    for (k, v) in userdict {
        if k.to_ascii_lowercase() == lower {
            return Some(v.clone());
        }
    }
    for (k, v) in REWARD_ALIASES {
        if k.eq_ignore_ascii_case(t) {
            return Some((*v).to_string());
        }
    }
    None
}

/// 中文优先；词典与规则都盖不住时才保留英文。
fn localize_reward_name(
    en: &str,
    zh: Option<&ZhLookup>,
    userdict: &HashMap<String, String>,
) -> String {
    let en = en.trim();
    if en.is_empty() {
        return String::new();
    }
    if let Some(z) = lookup_en(en, zh, userdict) {
        return z;
    }

    // "3X 1,500 Credits Cache" / "20X Efervon Sample"：先剥份数，再本地化剩余部分
    if let Some((count, rest)) = split_count_prefix(en) {
        let rest_zh = localize_reward_name(rest, zh, userdict);
        if !rest_zh.is_empty() {
            return format!("{count}×{rest_zh}");
        }
    }

    // "1,500 Credits Cache" / "15,000 Credits"
    if let Some((n, rest)) = split_leading_number(en) {
        let rest = rest.trim();
        if rest.eq_ignore_ascii_case("Credits Cache") {
            return format!("{n} 现金匣");
        }
        if rest.eq_ignore_ascii_case("Credits") {
            return format!("{n} 现金");
        }
        if let Some(z) = lookup_en(rest, zh, userdict) {
            return format!("{n} {z}");
        }
        if let Some(composed) = compose_complex(rest, zh, userdict) {
            return format!("{n} {composed}");
        }
    }

    if let Some(composed) = compose_complex(en, zh, userdict) {
        return composed;
    }

    en.to_string()
}

fn compose_complex(
    en: &str,
    zh: Option<&ZhLookup>,
    userdict: &HashMap<String, String>,
) -> Option<String> {
    let en = en.trim();

    // Relic (Radiant)
    if let Some(core) = en.strip_suffix(" Relic (Radiant)") {
        return Some(format!("{}（光辉）", format_relic(core)?));
    }
    if let Some(core) = en.strip_suffix(" Relic") {
        return format_relic(core);
    }

    // Blueprint parts
    if let Some(core) = en.strip_suffix(" Blueprint") {
        return compose_blueprint(core, zh, userdict);
    }

    // Gauntlet / barrel etc. — try stripping last word if whole miss
    None
}

fn format_relic(core: &str) -> Option<String> {
    // "Axi A21" / "Lith L8" / "Meso Y2" / "Neo C7"
    let mut parts = core.split_whitespace();
    let era = parts.next()?;
    let code = parts.collect::<Vec<_>>().join(" ");
    if code.is_empty() {
        return None;
    }
    let era_zh = match era {
        "Lith" => "古纪",
        "Meso" => "中纪",
        "Neo" => "新纪",
        "Axi" => "安魂",
        _ => return None,
    };
    Some(format!("{era_zh} {code} 遗物"))
}

fn compose_blueprint(
    core: &str,
    zh: Option<&ZhLookup>,
    userdict: &HashMap<String, String>,
) -> Option<String> {
    const PARTS: &[(&str, &str)] = &[
        (" Chassis", "机体蓝图"),
        (" Neuroptics", "神经光元蓝图"),
        (" Systems", "系统蓝图"),
    ];
    for (suffix, part_zh) in PARTS {
        if let Some(name) = core.strip_suffix(suffix) {
            let name_zh = lookup_en(name, zh, userdict).unwrap_or_else(|| name.to_string());
            return Some(format!("{name_zh} {part_zh}"));
        }
    }
    // 整名蓝图
    if let Some(z) = lookup_en(core, zh, userdict) {
        return Some(format!("{z} 蓝图"));
    }
    // Furax Wraith Left Gauntlet — 无蓝图后缀时不在此处理
    Some(format!("{core} 蓝图"))
}

fn split_count_prefix(en: &str) -> Option<(String, &str)> {
    let t = en.trim();
    let (num, rest) = t.split_once('X').or_else(|| t.split_once('x'))?;
    let num = num.trim();
    let rest = rest.trim();
    if num.is_empty() || rest.is_empty() || !num.chars().all(|c| c.is_ascii_digit() || c == ',') {
        return None;
    }
    Some((num.to_string(), rest))
}

fn split_leading_number(en: &str) -> Option<(String, &str)> {
    let t = en.trim();
    let mut i = 0;
    for ch in t.chars() {
        if ch.is_ascii_digit() || ch == ',' {
            i += ch.len_utf8();
        } else {
            break;
        }
    }
    if i == 0 {
        return None;
    }
    let num = t[..i].trim();
    let rest = t[i..].trim();
    if num.is_empty() || rest.is_empty() {
        return None;
    }
    Some((num.to_string(), rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_dict() -> HashMap<String, String> {
        HashMap::new()
    }

    fn sample_dict() -> HashMap<String, String> {
        let mut m = HashMap::new();
        m.insert("Efervon Sample".into(), "艾弗旺样本".into());
        m.insert("Endo".into(), "内融核心".into());
        m.insert("On-Lyne CD".into(), "线上时代Cd".into());
        m.insert("Cyte-09".into(), "塞特-09".into());
        m.insert("Gara".into(), "迦楼罗".into());
        m
    }

    fn sample_job(min: i32, max: i32) -> BountyJobApi {
        BountyJobApi {
            job_type: "test".into(),
            kind: String::new(),
            kind_zh: String::new(),
            title: String::new(),
            min_enemy_level: min,
            max_enemy_level: max,
            standing_total: 0,
            is_advanced: false,
            is_steel_path: false,
            challenge_zh: String::new(),
            rewards: vec![],
            reward_table: String::new(),
        }
    }

    fn hex_drop_tiers() -> Vec<DropBountyTier> {
        [
            ("Level  55 - 60 WF1999 Bounty", "Tier55"),
            ("Level  65 - 70 WF1999 Bounty", "Tier65"),
            ("Level  75 - 80 WF1999 Bounty", "Tier75"),
            ("Level  85 - 90 WF1999 Bounty", "Tier85"),
            ("Level  95 - 100 WF1999 Bounty", "Tier95"),
            ("Level  105 - 110 WF1999 Bounty", "Tier105"),
            ("Level  115 - 120 WF1999 Bounty", "Tier115"),
        ]
        .into_iter()
        .map(|(level, item)| {
            let mut rewards = HashMap::new();
            rewards.insert(
                "B".into(),
                vec![DropRewardItem {
                    item_name: item.into(),
                }],
            );
            DropBountyTier {
                bounty_level: level.into(),
                rewards,
            }
        })
        .collect()
    }

    #[test]
    fn parse_level_range_ok() {
        assert_eq!(
            parse_level_range("Level  65 - 70 WF1999 Bounty"),
            Some((65, 70))
        );
        assert_eq!(
            parse_level_range("Level 5 - 15 Cetus Bounty"),
            Some((5, 15))
        );
    }

    #[test]
    fn hex_levels_match_drop_all_with_minus_ten() {
        let tiers = hex_drop_tiers();
        let cases = [
            (65, 70, "Tier55"),
            (75, 80, "Tier65"),
            (125, 130, "Tier115"),
        ];
        for (min, max, expect) in cases {
            let job = sample_job(min, max);
            let items = match_tier_items("HexSyndicate", &tiers, &job, Some('B'));
            assert_eq!(items, vec![expect.to_string()], "Lv {min}-{max}");
        }
        // 非 Hex 不偏移：65–70 应对 drop 的 65–70
        let job = sample_job(65, 70);
        let items = match_tier_items("EntratiLabSyndicate", &tiers, &job, Some('B'));
        assert_eq!(items, vec!["Tier65".to_string()]);
    }

    #[test]
    fn cycle_rot_path_selects_pool_b() {
        assert_eq!(
            parse_reward_rotation("TierATableBRewards"),
            Some('B')
        );
    }

    #[test]
    fn parse_reward_rotation_from_worldstate_path() {
        assert_eq!(
            parse_reward_rotation(
                "/Lotus/Types/Game/MissionDecks/DeimosMissionRewards/TierATableBRewards"
            ),
            Some('B')
        );
        assert_eq!(
            parse_reward_rotation(
                "/Lotus/Types/Game/MissionDecks/VenusJobMissionRewards/VenusTierCTableARewards"
            ),
            Some('A')
        );
        assert_eq!(
            parse_reward_rotation(
                "/Lotus/Types/Game/MissionDecks/EidolonJobMissionRewards/NarmerTableCRewards"
            ),
            Some('C')
        );
        assert_eq!(
            parse_reward_rotation(
                "/Lotus/Types/Game/MissionDecks/EidolonJobMissionRewards/GhoulBountyTableARewards"
            ),
            Some('A')
        );
        assert_eq!(parse_reward_rotation(""), None);
    }

    #[test]
    fn unique_items_only_current_rotation() {
        let mut rewards = HashMap::new();
        rewards.insert(
            "A".into(),
            vec![DropRewardItem {
                item_name: "Only A".into(),
            }],
        );
        rewards.insert(
            "B".into(),
            vec![DropRewardItem {
                item_name: "Only B".into(),
            }],
        );
        rewards.insert(
            "C".into(),
            vec![DropRewardItem {
                item_name: "Only C".into(),
            }],
        );
        let tier = DropBountyTier {
            bounty_level: "Level 5 - 15 Cetus Bounty".into(),
            rewards,
        };
        assert_eq!(unique_items(&tier, Some('B')), vec!["Only B".to_string()]);
        assert_eq!(unique_items(&tier, Some('A')), vec!["Only A".to_string()]);
        // 无轮换且多池时不合并
        assert_eq!(unique_items(&tier, None).len(), 1);
    }

    #[test]
    fn localize_count_prefix() {
        let d = sample_dict();
        assert_eq!(
            localize_reward_name("20X Efervon Sample", None, &d),
            "20×艾弗旺样本"
        );
    }

    #[test]
    fn localize_endo_and_credits_cache() {
        let d = sample_dict();
        assert_eq!(
            localize_reward_name("600 Endo", None, &d),
            "600 内融核心"
        );
        assert_eq!(
            localize_reward_name("1,500 Credits Cache", None, &empty_dict()),
            "1,500 现金匣"
        );
        assert_eq!(
            localize_reward_name("15,000 Credits", None, &empty_dict()),
            "15,000 现金"
        );
        // drop-all 常见：份数 × 带金额的现金匣
        assert_eq!(
            localize_reward_name("3X 1,500 Credits Cache", None, &empty_dict()),
            "3×1,500 现金匣"
        );
        assert_eq!(
            localize_reward_name("2X 3,000 Credits Cache", None, &empty_dict()),
            "2×3,000 现金匣"
        );
    }

    #[test]
    fn localize_case_insensitive_userdict() {
        let d = sample_dict();
        assert_eq!(
            localize_reward_name("On-Lyne Cd", None, &d),
            "线上时代Cd"
        );
    }

    #[test]
    fn localize_relic_and_blueprint() {
        let d = sample_dict();
        assert_eq!(
            localize_reward_name("Lith A13 Relic", None, &empty_dict()),
            "古纪 A13 遗物"
        );
        assert_eq!(
            localize_reward_name("Axi S21 Relic (Radiant)", None, &empty_dict()),
            "安魂 S21 遗物（光辉）"
        );
        assert_eq!(
            localize_reward_name("Cyte-09 Chassis Blueprint", None, &d),
            "塞特-09 机体蓝图"
        );
        assert_eq!(
            localize_reward_name("Gara Neuroptics Blueprint", None, &d),
            "迦楼罗 神经光元蓝图"
        );
    }

    #[test]
    fn localize_alias_and_miss_keeps_english() {
        assert_eq!(
            localize_reward_name("Byteryte", None, &empty_dict()),
            "字节卫士"
        );
        assert_eq!(
            localize_reward_name("Totally Unknown Widget", None, &empty_dict()),
            "Totally Unknown Widget"
        );
    }

    #[test]
    fn localize_with_zh_lookup_preferred() {
        let mut zh = ZhLookup::default();
        zh.exact
            .insert("Efervon Sample".into(), "炽蛇军艾弗旺样本".into());
        let d = sample_dict();
        assert_eq!(
            localize_reward_name("20X Efervon Sample", Some(&zh), &d),
            "20×炽蛇军艾弗旺样本"
        );
    }
}
