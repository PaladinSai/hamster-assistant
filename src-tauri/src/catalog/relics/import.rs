//! Relics 落库。

use std::collections::HashMap;

use rusqlite::{params, Connection, Transaction};

use crate::catalog::weapons::VARIANT_PRIME as WEAPON_VARIANT_PRIME;
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
    let weapon_part_lookup = load_weapon_prime_part_lookup(tx)?;
    let i18n = load_name_i18n_map(tx)?;
    let mut stats = RelicImportStats::default();
    let mut drop_sort = 0i32;
    let mut weapon_drop_sort = 0i32;
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

        for part in &relic.weapon_prime_parts {
            let Some(&part_id) =
                weapon_part_lookup.get(&(part.family_key.clone(), part.part_key.to_string()))
            else {
                continue;
            };
            let drop_id = snowflake.next_id();
            let inserted = tx
                .execute(
                    "INSERT OR IGNORE INTO weapon_part_relic (id, part_id, relic_id, rarity, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![drop_id, part_id, id, part.rarity, weapon_drop_sort],
                )
                .map_err(|e| format!("插入武器掉落失败: {e}"))?;
            if inserted > 0 {
                stats.weapon_drops += 1;
                weapon_drop_sort += 1;
            }
        }
    }
    Ok(stats)
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

pub(crate) fn load_weapon_prime_part_lookup(
    tx: &Transaction<'_>,
) -> Result<HashMap<(String, String), i64>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT f.family_key, p.part_key, p.id
             FROM weapon_part p
             JOIN weapon w ON w.id = p.weapon_id
             JOIN weapon_family f ON f.id = w.family_id
             WHERE w.variant = ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([WEAPON_VARIANT_PRIME], |row| {
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



