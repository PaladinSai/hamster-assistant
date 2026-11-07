//! Relics 落库。

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, Transaction};

use crate::catalog::weapons::weapon_part_meta;
use crate::catalog::wfcd::VARIANT_PRIME;
use crate::id::Snowflake;

#[derive(Debug, Default, Clone)]
pub struct RelicImportStats {
    pub relics: usize,
    pub drops: usize,
    pub weapon_drops: usize,
    pub rewards: usize,
}

use super::parse::{parse_intact_relics, ParsedRelic};

pub fn import_relics_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<RelicImportStats, String> {
    let relics = parse_intact_relics(json)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("遗物事务失败: {e}"))?;
    let stats = import_relics_in_tx(&tx, &relics, snowflake)?;
    tx.commit().map_err(|e| format!("遗物提交失败: {e}"))?;
    Ok(stats)
}

pub(crate) fn import_relics_in_tx(
    tx: &Transaction<'_>,
    relics: &[ParsedRelic],
    snowflake: &Snowflake,
) -> Result<RelicImportStats, String> {
    let _ = tx.execute_batch("PRAGMA foreign_keys = ON;");
    tx.execute_batch(
        "DELETE FROM relic_location;
         DELETE FROM relic_reward;
         DELETE FROM warframe_part_relic;
         DELETE FROM weapon_part_relic;
         DELETE FROM void_relic;",
    )
    .map_err(|e| format!("清空遗物表失败: {e}"))?;

    let part_lookup = load_prime_part_lookup(tx)?;
    let i18n = load_name_i18n_map(tx)?;
    let mut stats = RelicImportStats::default();
    let mut drop_sort = 0i32;
    let mut reward_sort = 0i32;

    for relic in relics {
        let id = snowflake.next_id();
        tx.execute(
            "INSERT INTO void_relic (id, relic_key, era, code, name_zh, vaulted, unique_name)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                relic.relic_key,
                relic.era,
                relic.code,
                relic.name_zh,
                if relic.vaulted { 1 } else { 0 },
                relic.unique_name,
            ],
        )
        .map_err(|e| format!("插入遗物 {} 失败: {e}", relic.relic_key))?;
        stats.relics += 1;

        for reward in &relic.all_rewards {
            let rid = snowflake.next_id();
            let item_zh = resolve_reward_zh(&reward.item_name_en, &i18n);
            tx.execute(
                "INSERT INTO relic_reward
                 (id, relic_id, rarity, item_name_en, item_name_zh, unique_name, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    rid,
                    id,
                    reward.rarity,
                    reward.item_name_en,
                    item_zh,
                    reward.unique_name,
                    reward_sort,
                ],
            )
            .map_err(|e| format!("插入奖励失败: {e}"))?;
            stats.rewards += 1;
            reward_sort += 1;
        }

        for part in &relic.prime_parts {
            let Some(&part_id) = part_lookup.get(&(part.family_key.clone(), part.part_key.to_string()))
            else {
                continue;
            };
            let drop_id = snowflake.next_id();
            let inserted = tx
                .execute(
                    "INSERT OR IGNORE INTO warframe_part_relic (id, part_id, relic_id, rarity, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![drop_id, part_id, id, part.rarity, drop_sort],
                )
                .map_err(|e| format!("插入掉落失败: {e}"))?;
            if inserted > 0 {
                stats.drops += 1;
                drop_sort += 1;
            }
        }

    }
    reconcile_weapon_part_labels(tx)?;
    stats.weapon_drops = link_weapon_relics(tx, snowflake)?;
    link_suit_parts_by_unique(tx, snowflake)?;
    Ok(stats)
}

/// 已有遗物奖励时，按路径回写部件中文并重挂 `weapon_part_relic`。
/// 不重导遗物，因此不碰 `relic_location`。
pub fn relink_weapon_parts_by_unique(
    conn: &mut Connection,
    snowflake: &Snowflake,
) -> Result<usize, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("武器部件重链事务失败: {e}"))?;
    tx.execute("DELETE FROM weapon_part_relic", [])
        .map_err(|e| format!("清空武器遗物关联失败: {e}"))?;
    reconcile_weapon_part_labels(&tx)?;
    let n = link_weapon_relics(&tx, snowflake)?;
    tx.commit()
        .map_err(|e| format!("武器部件重链提交失败: {e}"))?;
    Ok(n)
}

struct RewardLabel {
    item_en: String,
    item_zh: Option<String>,
}

struct StoredWeaponPart {
    id: i64,
    weapon_id: i64,
    part_key: String,
    name_zh: String,
    unique_name: Option<String>,
    weapon_zh: Option<String>,
}

/// 遗物奖励和配方部件的 `uniqueName` 相同。用奖励英文名改 `part_key`，用中文去掉枪名做部件名。
fn reconcile_weapon_part_labels(tx: &Transaction<'_>) -> Result<(), String> {
    let rewards = load_reward_labels(tx)?;
    let parts = load_stored_weapon_parts(tx)?;
    let mut occupied: HashMap<(i64, String), i64> = HashMap::new();
    for p in &parts {
        occupied.insert((p.weapon_id, p.part_key.clone()), p.id);
    }
    for p in &parts {
        let Some(unique) = p.unique_name.as_deref() else {
            continue;
        };
        let Some(reward) = rewards.get(unique) else {
            continue;
        };
        let meta = reward
            .item_en
            .split_once(" Prime")
            .map(|(_, rest)| rest.trim())
            .and_then(weapon_part_meta);
        let stripped = match (&reward.item_zh, &p.weapon_zh) {
            (Some(item_zh), Some(weapon_zh)) => strip_weapon_zh(item_zh, weapon_zh),
            _ => None,
        };
        let name_zh = stripped
            .or_else(|| meta.map(|m| m.name_zh.to_string()))
            .unwrap_or_else(|| p.name_zh.clone());
        let mut part_key = p.part_key.clone();
        let mut sort = None;
        if let Some(meta) = meta {
            let target = meta.key.to_string();
            let blocker = occupied.get(&(p.weapon_id, target.clone())).copied();
            if blocker.is_none() || blocker == Some(p.id) {
                if target != p.part_key {
                    occupied.remove(&(p.weapon_id, p.part_key.clone()));
                    occupied.insert((p.weapon_id, target.clone()), p.id);
                }
                part_key = target;
                sort = Some(meta.sort);
            }
        }
        tx.execute(
            "UPDATE weapon_part
             SET part_key=?2, name_zh=?3, sort_order=COALESCE(?4, sort_order)
             WHERE id=?1",
            params![p.id, part_key, name_zh, sort],
        )
        .map_err(|e| format!("回写武器部件译名失败: {e}"))?;
    }
    Ok(())
}

fn load_reward_labels(tx: &Transaction<'_>) -> Result<HashMap<String, RewardLabel>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT unique_name, item_name_en, item_name_zh
             FROM relic_reward
             WHERE unique_name IS NOT NULL AND TRIM(unique_name) != ''",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (unique, item_en, item_zh) = r.map_err(|e| e.to_string())?;
        map.entry(unique).or_insert(RewardLabel { item_en, item_zh });
    }
    Ok(map)
}

fn load_stored_weapon_parts(tx: &Transaction<'_>) -> Result<Vec<StoredWeaponPart>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT p.id, p.weapon_id, p.part_key, p.name_zh, p.unique_name, w.name_zh
             FROM weapon_part p
             JOIN weapon w ON w.id = p.weapon_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(StoredWeaponPart {
                id: r.get(0)?,
                weapon_id: r.get(1)?,
                part_key: r.get(2)?,
                name_zh: r.get(3)?,
                unique_name: r.get(4)?,
                weapon_zh: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        let mut part = r.map_err(|e| e.to_string())?;
        if part
            .unique_name
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            part.unique_name = None;
        }
        out.push(part);
    }
    Ok(out)
}

/// 「旋刃飞刀 Prime 镖袋」去掉枪名「旋刃飞刀 Prime」得到「镖袋」。
/// 枪名没带 Prime 时，剩余部分开头的 Prime 也去掉。
pub(crate) fn strip_weapon_zh(item_zh: &str, weapon_zh: &str) -> Option<String> {
    let item_norm: String = item_zh.split_whitespace().collect::<Vec<_>>().join("");
    let weapon_norm: String = weapon_zh.split_whitespace().collect::<Vec<_>>().join("");
    if weapon_norm.is_empty() {
        return None;
    }
    let mut rest = item_norm.strip_prefix(&weapon_norm)?.to_string();
    if let Some(stripped) = rest.strip_prefix("Prime") {
        rest = stripped.to_string();
    }
    let rest = rest.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

fn link_weapon_relics(tx: &Transaction<'_>, snowflake: &Snowflake) -> Result<usize, String> {
    let pairs: Vec<(i64, Option<i32>, i64)> = {
        let mut stmt = tx
            .prepare(
                "SELECT rr.relic_id, rr.rarity, p.id
                 FROM relic_reward rr
                 JOIN weapon_part p ON p.unique_name = rr.unique_name
                 WHERE IFNULL(rr.unique_name, '') != ''
                   AND IFNULL(p.unique_name, '') != ''
                 ORDER BY rr.relic_id, rr.sort_order, p.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    let mut seen = HashSet::<(i64, i64)>::new();
    let mut n = 0usize;
    let mut sort = 0i32;
    for (relic_id, rarity, part_id) in pairs {
        if !seen.insert((relic_id, part_id)) {
            continue;
        }
        let id = snowflake.next_id();
        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO weapon_part_relic (id, part_id, relic_id, rarity, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, part_id, relic_id, rarity, sort],
            )
            .map_err(|e| format!("插入武器掉落失败: {e}"))?;
        if inserted > 0 {
            n += 1;
            sort += 1;
        }
    }
    Ok(n)
}

/// 配方叶子和遗物叶子常常只差 `Component` / `Blueprint`。
/// `PrimeArchwingChassisComponent` 与 `PrimeArchwingChassisBlueprint` 是同一件。
fn recipe_leaf_stem(unique: &str) -> String {
    let leaf = unique.rsplit('/').next().unwrap_or(unique);
    for suf in ["Component", "Blueprint", "Item"] {
        if let Some(stem) = leaf.strip_suffix(suf) {
            if !stem.is_empty() {
                return stem.to_string();
            }
        }
    }
    leaf.to_string()
}

struct SuitPartRef {
    id: i64,
    suit_zh: Option<String>,
    suit_kind: String,
}

/// Archwing / 同伴 / 机甲的遗物不在 `WarframeRecipes` 下，按配方路径挂上。
fn link_suit_parts_by_unique(tx: &Transaction<'_>, snowflake: &Snowflake) -> Result<usize, String> {
    let parts: Vec<(String, SuitPartRef)> = {
        let mut stmt = tx
            .prepare(
                "SELECT p.id, p.unique_name, COALESCE(w.name_zh, f.name_zh), f.suit_kind
                 FROM warframe_part p
                 JOIN warframe w ON w.id = p.warframe_id
                 JOIN warframe_family f ON f.id = w.family_id
                 WHERE p.unique_name IS NOT NULL AND TRIM(p.unique_name) != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(1)?,
                    SuitPartRef {
                        id: r.get(0)?,
                        suit_zh: r.get(2)?,
                        suit_kind: r.get(3)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        out
    };
    if parts.is_empty() {
        return Ok(0);
    }
    let mut by_exact: HashMap<String, SuitPartRef> = HashMap::new();
    let mut by_stem: HashMap<String, Option<i64>> = HashMap::new();
    let mut part_by_id: HashMap<i64, SuitPartRef> = HashMap::new();
    for (unique, part) in parts {
        let stem = recipe_leaf_stem(&unique);
        let clash = matches!(by_stem.get(&stem), Some(Some(id)) if *id != part.id);
        if clash {
            by_stem.insert(stem, None);
        } else if !by_stem.contains_key(&stem) {
            by_stem.insert(stem, Some(part.id));
        }
        part_by_id.insert(
            part.id,
            SuitPartRef {
                id: part.id,
                suit_zh: part.suit_zh.clone(),
                suit_kind: part.suit_kind.clone(),
            },
        );
        by_exact.insert(unique, part);
    }

    let rewards: Vec<(i64, Option<i32>, String, Option<String>)> = {
        let mut stmt = tx
            .prepare(
                "SELECT relic_id, rarity, unique_name, item_name_zh
                 FROM relic_reward
                 WHERE unique_name IS NOT NULL AND TRIM(unique_name) != ''",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };

    let mut seen = HashSet::<(i64, i64)>::new();
    let mut named = HashSet::<i64>::new();
    let mut n = 0usize;
    let mut sort = 0i32;
    for (relic_id, rarity, unique, item_zh) in rewards {
        let part_id = if let Some(part) = by_exact.get(&unique) {
            part.id
        } else {
            let stem = recipe_leaf_stem(&unique);
            match by_stem.get(&stem) {
                Some(Some(id)) => *id,
                _ => continue,
            }
        };
        if let (Some(item_zh), Some(part)) = (item_zh.as_deref(), part_by_id.get(&part_id)) {
            let rename = matches!(part.suit_kind.as_str(), "archwing" | "companion" | "necramech");
            if rename && named.insert(part_id) {
                if let Some(suit_zh) = part.suit_zh.as_deref() {
                    if let Some(short) = strip_weapon_zh(item_zh, suit_zh) {
                        tx.execute(
                            "UPDATE warframe_part SET name_zh=?2 WHERE id=?1",
                            params![part_id, short],
                        )
                        .map_err(|e| format!("回写机体部件译名失败: {e}"))?;
                    }
                }
            }
        }
        if !seen.insert((relic_id, part_id)) {
            continue;
        }
        let id = snowflake.next_id();
        let inserted = tx
            .execute(
                "INSERT OR IGNORE INTO warframe_part_relic
                 (id, part_id, relic_id, rarity, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, part_id, relic_id, rarity, sort],
            )
            .map_err(|e| format!("插入机体遗物失败: {e}"))?;
        if inserted > 0 {
            n += 1;
            sort += 1;
        }
    }
    Ok(n)
}

/// 奖励已在库里时补挂机体部件，不清遗物来源。
pub fn link_suit_relics_from_rewards(
    conn: &mut Connection,
    snowflake: &Snowflake,
) -> Result<usize, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("机体部件重链事务失败: {e}"))?;
    let n = link_suit_parts_by_unique(&tx, snowflake)?;
    tx.commit()
        .map_err(|e| format!("机体部件重链提交失败: {e}"))?;
    Ok(n)
}

pub(crate) fn load_name_i18n_map(tx: &Transaction<'_>) -> Result<HashMap<String, String>, String> {
    let sql = format!(
        "SELECT name_en, {} AS name_zh FROM name_i18n WHERE {} IS NOT NULL",
        crate::catalog::i18n::DISPLAY_ZH_SQL,
        crate::catalog::i18n::DISPLAY_ZH_SQL
    );
    let mut stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (en, zh) = r.map_err(|e| e.to_string())?;
        map.insert(en, zh);
    }
    Ok(map)
}

/// 先拆已知零件后缀，再查整装简中；零件译名优先 `name_i18n`。
pub fn resolve_reward_zh(item_en: &str, i18n: &HashMap<String, String>) -> Option<String> {
    if let Some((base_en, part_zh)) = split_item_and_part(item_en, i18n) {
        let base_zh = lookup_base_zh(base_en, i18n)
            .unwrap_or_else(|| format_item_zh_spacing(base_en));
        return Some(format!("{base_zh} {part_zh}"));
    }
    // 非零件奖励：词典精确 / 最长前缀
    if let Some(zh) = i18n.get(item_en) {
        return Some(format_item_zh_spacing(zh));
    }
    let mut best_key: Option<&str> = None;
    for key in i18n.keys() {
        if !item_en.starts_with(key.as_str()) {
            continue;
        }
        if item_en.len() != key.len() && item_en.as_bytes().get(key.len()) != Some(&b' ') {
            continue;
        }
        if best_key.map(|b| key.len() > b.len()).unwrap_or(true) {
            best_key = Some(key.as_str());
        }
    }
    best_key
        .and_then(|k| i18n.get(k))
        .map(|zh| format_item_zh_spacing(zh))
}

pub(crate) fn split_item_and_part<'a>(
    item_en: &'a str,
    i18n: &HashMap<String, String>,
) -> Option<(&'a str, String)> {
    for &suffix in PART_SUFFIX_KEYS {
        if let Some(base) = item_en.strip_suffix(suffix) {
            let base = base.trim_end();
            if base.is_empty() {
                continue;
            }
            let part_zh = lookup_part_zh(suffix, i18n)?;
            return Some((base, part_zh));
        }
    }
    None
}

/// 零件后缀简中：先查对照表（含 `Prime Receiver` 等），再回退内置表。
pub(crate) fn lookup_part_zh(suffix: &str, i18n: &HashMap<String, String>) -> Option<String> {
    let prime_key = format!("Prime {suffix}");
    for key in [suffix, prime_key.as_str()] {
        if let Some(zh) = i18n.get(key) {
            let zh = zh.trim();
            if !zh.is_empty() {
                return Some(zh.to_string());
            }
        }
    }
    PART_SUFFIX_FALLBACK
        .iter()
        .find(|&&(suf, _)| suf == suffix)
        .map(|&(_, zh)| zh.to_string())
}

pub(crate) fn is_part_compound_key(key: &str) -> bool {
    key.contains(' ')
        && PART_SUFFIX_KEYS
            .iter()
            .any(|suf| key.ends_with(suf) && key.len() > suf.len())
}

pub(crate) fn lookup_base_zh(base_en: &str, i18n: &HashMap<String, String>) -> Option<String> {
    if let Some(zh) = i18n.get(base_en) {
        return Some(format_item_zh_spacing(zh));
    }
    let mut best_key: Option<&str> = None;
    for key in i18n.keys() {
        if !base_en.starts_with(key.as_str()) {
            continue;
        }
        if base_en.len() != key.len() && base_en.as_bytes().get(key.len()) != Some(&b' ') {
            continue;
        }
        if is_part_compound_key(key) {
            continue;
        }
        if best_key.map(|b| key.len() > b.len()).unwrap_or(true) {
            best_key = Some(key.as_str());
        }
    }
    best_key
        .and_then(|k| i18n.get(k))
        .map(|zh| format_item_zh_spacing(zh))
}

/// `伯斯顿Prime` → `伯斯顿 Prime`（国际服常见空格）。
pub(crate) fn format_item_zh_spacing(zh: &str) -> String {
    let mut s = zh.to_string();
    for token in ["Prime", "Umbra"] {
        let glued = token.to_string();
        let spaced = format!(" {token}");
        if s.contains(token) && !s.contains(&spaced) {
            s = s.replace(&glued, &spaced);
        }
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 可从奖励英文名右侧拆出的零件后缀（长的在前）。
const PART_SUFFIX_KEYS: &[&str] = &[
    "Neuroptics Blueprint",
    "Chassis Blueprint",
    "Systems Blueprint",
    "Harness Blueprint",
    "Wings Blueprint",
    "Kubrow Collar Blueprint",
    "Lower Limb",
    "Upper Limb",
    "Blueprint",
    "Barrel",
    "Receiver",
    "Stock",
    "Blade",
    "Blades",
    "Handle",
    "Grip",
    "Link",
    "Gauntlet",
    "String",
    "Guard",
    "Hilt",
    "Head",
    "Chain",
    "Disc",
    "Ornament",
    "Pouch",
    "Stars",
    "Band",
    "Boot",
    "Buckle",
    "Carapace",
    "Cerebrum",
    "Systems",
    "Neuroptics",
    "Chassis",
];

/// 对照表缺失时的回退（与常见译法对齐：Receiver→枪机，Link→链接）。
const PART_SUFFIX_FALLBACK: &[(&str, &str)] = &[
    ("Neuroptics Blueprint", "头部神经光元蓝图"),
    ("Chassis Blueprint", "机体蓝图"),
    ("Systems Blueprint", "系统蓝图"),
    ("Harness Blueprint", "外甲蓝图"),
    ("Wings Blueprint", "机翼蓝图"),
    ("Kubrow Collar Blueprint", "库狛项圈蓝图"),
    ("Lower Limb", "下臂"),
    ("Upper Limb", "上臂"),
    ("Blueprint", "蓝图"),
    ("Barrel", "枪管"),
    ("Receiver", "枪机"),
    ("Stock", "枪托"),
    ("Blade", "刀刃"),
    ("Blades", "刀刃"),
    ("Handle", "握柄"),
    ("Grip", "握把"),
    ("Link", "链接"),
    ("Gauntlet", "护手"),
    ("String", "弓弦"),
    ("Guard", "护手"),
    ("Hilt", "柄"),
    ("Head", "头部"),
    ("Chain", "链条"),
    ("Disc", "圆盘"),
    ("Ornament", "饰物"),
    ("Pouch", "弹药包"),
    ("Stars", "飞星"),
    ("Band", "束带"),
    ("Boot", "靴部"),
    ("Buckle", "扣环"),
    ("Carapace", "外壳"),
    ("Cerebrum", "大脑"),
    ("Systems", "系统"),
    ("Neuroptics", "头部神经光元"),
    ("Chassis", "机体"),
];

pub(crate) fn load_prime_part_lookup(
    tx: &Transaction<'_>,
) -> Result<HashMap<(String, String), i64>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT f.family_key, p.part_key, p.id
             FROM warframe_part p
             JOIN warframe w ON w.id = p.warframe_id
             JOIN warframe_family f ON f.id = w.family_id
             WHERE w.variant = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([VARIANT_PRIME], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut map = HashMap::new();
    for r in rows {
        let (family, pkey, id) = r.map_err(|e| e.to_string())?;
        map.insert((family, pkey), id);
    }
    Ok(map)
}



