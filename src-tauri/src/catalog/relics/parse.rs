//! Relics.json 解析。

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use crate::catalog::weapons::weapon_part_meta;

#[derive(Debug, Deserialize)]
struct WfcdRelic {
    name: String,
    #[serde(default, rename = "uniqueName")]
    unique_name: Option<String>,
    #[serde(default)]
    vaulted: Option<bool>,
    #[serde(default)]
    rewards: Vec<WfcdRelicReward>,
}

#[derive(Debug, Deserialize)]
struct WfcdRelicReward {
    rarity: Option<String>,
    #[serde(default)]
    chance: Option<f64>,
    #[serde(default)]
    item: Option<WfcdRelicItem>,
}

#[derive(Debug, Deserialize)]
struct WfcdRelicItem {
    name: Option<String>,
    #[serde(rename = "uniqueName")]
    unique_name: Option<String>,
}

pub fn parse_intact_relics(bytes: &[u8]) -> Result<Vec<ParsedRelic>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Relics JSON 解析失败: {e}"))?;
    let Value::Array(arr) = value else {
        return Err("Relics.json 根节点不是数组".to_string());
    };

    let mut by_key: HashMap<String, ParsedRelic> = HashMap::new();
    for item in arr {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if !name.ends_with(" Intact") {
            continue;
        }
        let relic: WfcdRelic = serde_json::from_value(item)
            .map_err(|e| format!("单条遗物解析失败: {e}"))?;
        let Some((era, code, relic_key)) = parse_relic_identity(&relic.name) else {
            continue;
        };
        let mut all_rewards = Vec::new();
        let mut prime_parts = Vec::new();
        let mut weapon_prime_parts = Vec::new();
        for r in &relic.rewards {
            let Some(item) = r.item.as_ref() else {
                continue;
            };
            let uniq = item.unique_name.as_deref().unwrap_or("");
            let iname = item.name.as_deref().unwrap_or("").trim();
            if iname.is_empty() {
                continue;
            }
            let rarity = rarity_code(r.rarity.as_deref(), r.chance);
            all_rewards.push(ParsedFullReward {
                item_name_en: iname.to_string(),
                unique_name: if uniq.is_empty() {
                    None
                } else {
                    Some(uniq.to_string())
                },
                rarity,
            });
            if uniq.contains("WarframeRecipes") && iname.contains("Prime") {
                if let Some((family_key, part_key)) = map_prime_reward(iname, uniq) {
                    prime_parts.push(ParsedPrimePart {
                        family_key,
                        part_key,
                        rarity,
                    });
                }
            } else if uniq.contains("/Recipes/Weapons/") && iname.contains("Prime") {
                if let Some((family_key, part_key)) = map_weapon_prime_reward(iname) {
                    weapon_prime_parts.push(ParsedPrimePart {
                        family_key,
                        part_key,
                        rarity,
                    });
                }
            }
        }
        let relic_unique = relic
            .unique_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let entry = by_key.entry(relic_key.clone()).or_insert_with(|| ParsedRelic {
            relic_key,
            era,
            code: code.clone(),
            name_zh: format!("{} {}", era_name_zh(era), code),
            vaulted: relic.vaulted.unwrap_or(false),
            unique_name: relic_unique.clone(),
            all_rewards: Vec::new(),
            prime_parts: Vec::new(),
            weapon_prime_parts: Vec::new(),
        });
        if entry.unique_name.is_none() {
            entry.unique_name = relic_unique;
        }
        if relic.vaulted.unwrap_or(false) {
            entry.vaulted = true;
        }
        entry.all_rewards.extend(all_rewards);
        entry.prime_parts.extend(prime_parts);
        entry.weapon_prime_parts.extend(weapon_prime_parts);
    }
    Ok(by_key.into_values().collect())
}

#[derive(Debug, Clone)]
pub struct ParsedRelic {
    pub relic_key: String,
    pub era: i32,
    pub code: String,
    pub name_zh: String,
    pub vaulted: bool,
    /// WFCD Intact 的 uniqueName，用来对上货架投影路径。
    pub unique_name: Option<String>,
    pub all_rewards: Vec<ParsedFullReward>,
    pub prime_parts: Vec<ParsedPrimePart>,
    pub weapon_prime_parts: Vec<ParsedPrimePart>,
}

#[derive(Debug, Clone)]
pub struct ParsedFullReward {
    pub item_name_en: String,
    pub unique_name: Option<String>,
    pub rarity: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ParsedPrimePart {
    pub family_key: String,
    pub part_key: &'static str,
    pub rarity: Option<i32>,
}

pub(crate) fn parse_relic_identity(name: &str) -> Option<(i32, String, String)> {
    let base = name.strip_suffix(" Intact")?;
    let mut parts = base.split_whitespace();
    let tier = parts.next()?;
    let code = parts.next()?.to_string();
    if parts.next().is_some() {
        return None;
    }
    let era = match tier {
        "Lith" => 0,
        "Meso" => 1,
        "Neo" => 2,
        "Axi" => 3,
        _ => return None,
    };
    Some((era, code.clone(), format!("{tier} {code}")))
}

pub(crate) fn era_name_zh(era: i32) -> &'static str {
    match era {
        0 => "古纪",
        1 => "前纪",
        2 => "中纪",
        3 => "后纪",
        _ => "遗物",
    }
}

/// WFCD Intact：铜/银都标成 `Uncommon`，用 chance 拆开（铜≈25%、银≈11%、金=Rare≈2%）。
pub fn rarity_code(raw: Option<&str>, chance: Option<f64>) -> Option<i32> {
    match raw? {
        "Rare" => Some(2),
        "Common" => Some(0),
        "Uncommon" => {
            // Intact：铜档 chance≥20；银档约 11。无 chance 时退回银，避免误标。
            if chance.is_some_and(|c| c >= 20.0) {
                Some(0)
            } else {
                Some(1)
            }
        }
        _ => None,
    }
}

/// 从奖励名/路径得到战甲家族键与 part_key。
pub fn map_prime_reward(item_name: &str, unique_name: &str) -> Option<(String, &'static str)> {
    let part_key = if unique_name.contains("Helmet") || item_name.contains("Neuroptics") {
        "neuroptics"
    } else if unique_name.contains("Chassis") || item_name.contains("Chassis") {
        "chassis"
    } else if unique_name.contains("Systems") || item_name.contains("Systems") {
        "systems"
    } else if item_name.contains("Blueprint") || unique_name.contains("Blueprint") {
        "blueprint"
    } else {
        return None;
    };

    let family = item_name
        .split(" Prime")
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some((family.to_string(), part_key))
}

/// 武器 Prime 部件奖励 → (family_key, part_key)。
pub fn map_weapon_prime_reward(item_name: &str) -> Option<(String, &'static str)> {
    // 「Xxx Prime Barrel」/「Xxx Prime Blueprint」
    let (family_prime, rest) = item_name.split_once(" Prime")?;
    let family = family_prime.trim();
    if family.is_empty() {
        return None;
    }
    let part_name = rest.trim();
    if part_name.is_empty() {
        // 整枪奖励，不当部件
        return None;
    }
    let meta = weapon_part_meta(part_name)?;
    Some((family.to_string(), meta.key))
}

