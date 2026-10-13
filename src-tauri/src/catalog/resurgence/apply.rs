//! 重生落库。

use std::collections::HashSet;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::catalog::i18n::ZhLookup;
use crate::catalog::weapon_family_and_variant;
use crate::id::Snowflake;

use super::fetch::relic_key_for_projection;
use super::types::*;

#[cfg(test)]
pub(crate) fn import_vault_trader_json(
    catalog: &mut Connection,
    snowflake: &Snowflake,
    json: &[u8],
) -> Result<(usize, usize, usize), String> {
    use super::fetch::next_from_warframestat_schedule;
    let trader: VaultTrader =
        serde_json::from_slice(json).map_err(|e| format!("vaultTrader JSON 解析失败: {e}"))?;
    let next = next_from_warframestat_schedule(&trader);
    apply_vault_fetch(
        catalog,
        snowflake,
        &VaultFetch {
            current: trader,
            next,
        },
        "test://vaultTrader",
    )
}

pub(crate) fn apply_vault_fetch(
    catalog: &mut Connection,
    snowflake: &Snowflake,
    fetched: &VaultFetch,
    source_url: &str,
) -> Result<(usize, usize, usize), String> {
    let trader = &fetched.current;
    let items = parse_inventory(&trader.inventory);

    let tx = catalog
        .transaction()
        .map_err(|e| format!("重生事务失败: {e}"))?;
    let zh = ZhLookup::load(&tx)?;

    let rotation_id = upsert_rotation(&tx, snowflake, trader, source_url)?;

    let mut sort = 0i32;
    let mut n_wf = 0usize;
    let mut n_wp = 0usize;
    let mut family_keys = Vec::new();
    let mut weapon_family_keys = Vec::new();
    for item in &items {
        let id = snowflake.next_id();
        let mut name_en = item.name_en.clone();
        let mut family_key = item.family_key.clone();
        let mut name_zh = None;
        if let Some((en, zh_name, fk)) = lookup_entity_names(&tx, item.kind, &item.unique_name)? {
            name_en = en;
            name_zh = zh_name;
            if let Some(f) = fk {
                family_key = Some(f);
            }
        }
        if name_zh.is_none() {
            name_zh = zh.get(&name_en).or_else(|| {
                family_key.as_deref().and_then(|k| zh.get(k))
            });
        }
        match item.kind {
            "warframe" => {
                if let Some(ref fk) = family_key {
                    family_keys.push(fk.clone());
                }
                n_wf += 1;
            }
            "weapon" => {
                if let Some(ref fk) = family_key {
                    weapon_family_keys.push(fk.clone());
                }
                n_wp += 1;
            }
            _ => {}
        }
        tx.execute(
            "INSERT INTO resurgence_item
             (id, rotation_id, kind, name_en, name_zh, unique_name, family_key, relic_key, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8)",
            params![
                id,
                rotation_id,
                item.kind,
                name_en,
                name_zh,
                item.unique_name,
                family_key,
                sort
            ],
        )
        .map_err(|e| format!("插入重生条目失败: {e}"))?;
        sort += 1;
    }

    // 本期核桃：只认货架上的虚空遗物投影，用路径末段对上 void_relic.unique_name。
    let mut relic_keys: HashSet<String> = HashSet::new();
    let projections: Vec<(String, String)> = items
        .iter()
        .filter(|i| i.kind == "relic_store")
        .map(|i| (i.unique_name.clone(), i.name_en.clone()))
        .collect();
    let mut n_relic = 0usize;
    for (unique_name, name_en) in projections {
        let Some(key) = relic_key_for_projection(&tx, &unique_name)? else {
            continue;
        };
        if !relic_keys.insert(key.clone()) {
            continue;
        }
        let id = snowflake.next_id();
        let relic_zh: Option<String> = tx
            .query_row(
                "SELECT name_zh FROM void_relic WHERE relic_key = ?1",
                [&key],
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten()
            .or_else(|| zh.get(&name_en));
        tx.execute(
            "INSERT INTO resurgence_item
             (id, rotation_id, kind, name_en, name_zh, unique_name, family_key, relic_key, sort_order)
             VALUES (?1, ?2, 'relic', ?3, ?4, ?5, NULL, ?6, ?7)",
            params![id, rotation_id, name_en, relic_zh, unique_name, key, sort],
        )
        .map_err(|e| format!("插入重生遗物失败: {e}"))?;
        sort += 1;
        n_relic += 1;
    }

    let known_families = load_family_keys(&tx)?;
    let next_id = match &fetched.next {
        Some(next) => Some(apply_next_preview(
            &tx,
            snowflake,
            next,
            &known_families,
            source_url,
            &zh,
        )?),
        None => {
            // 占位：activation=当期 expiry，expiry 置空，避免和当期抢「最早未过期」
            trader
                .expiry
                .as_ref()
                .map(|exp| {
                    upsert_rotation_times(
                        &tx,
                        snowflake,
                        Some(exp.as_str()),
                        None,
                        source_url,
                    )
                })
                .transpose()?
        }
    };

    retain_rotations(&tx, rotation_id, next_id)?;

    // vault_status：先清掉旧重生，再标本期（变体和家族一起）；下期不改 vault_status
    tx.execute(
        "UPDATE warframe SET vault_status = 2 WHERE variant = 1 AND vault_status = 3",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE warframe_family SET vault_status = 2 WHERE vault_status = 3",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE weapon SET vault_status = 2 WHERE variant = 1 AND vault_status = 3",
        [],
    )
    .map_err(|e| e.to_string())?;
    for fk in &family_keys {
        tx.execute(
            "UPDATE warframe SET vault_status = 3
             WHERE variant = 1 AND family_id = (
               SELECT id FROM warframe_family WHERE family_key = ?1
             )",
            params![fk],
        )
        .map_err(|e| format!("标记重生失败 {fk}: {e}"))?;
        tx.execute(
            "UPDATE warframe_family SET vault_status = 3 WHERE family_key = ?1",
            params![fk],
        )
        .map_err(|e| format!("标记家族重生失败 {fk}: {e}"))?;
    }
    for fk in &weapon_family_keys {
        tx.execute(
            "UPDATE weapon SET vault_status = 3
             WHERE variant = 1 AND family_id = (
               SELECT id FROM weapon_family WHERE family_key = ?1
             )",
            params![fk],
        )
        .map_err(|e| format!("标记武器重生失败 {fk}: {e}"))?;
    }

    tx.commit().map_err(|e| format!("重生提交失败: {e}"))?;
    Ok((n_wf, n_wp, n_relic))
}

pub(crate) fn load_family_keys(tx: &rusqlite::Transaction<'_>) -> Result<Vec<String>, String> {
    let mut stmt = tx
        .prepare("SELECT family_key FROM warframe_family")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut keys = Vec::new();
    for r in rows {
        keys.push(r.map_err(|e| e.to_string())?);
    }
    Ok(keys)
}

pub(crate) fn apply_next_preview(
    tx: &rusqlite::Transaction<'_>,
    snowflake: &Snowflake,
    next: &NextPreview,
    known_families: &[String],
    source_url: &str,
    zh: &ZhLookup,
) -> Result<i64, String> {
    let rotation_id = upsert_rotation_times(
        tx,
        snowflake,
        next.activation.as_deref(),
        next.expiry.as_deref(),
        source_url,
    )?;
    let families = families_from_featured(&next.featured, known_families);
    let mut sort = 0i32;
    for fk in families {
        let name_en = format!("{fk} Prime");
        let unique = format!("/Lotus/Powersuits/{fk}/{fk}Prime");
        let id = snowflake.next_id();
        let name_zh = zh.get(&name_en).or_else(|| zh.get(&fk));
        tx.execute(
            "INSERT INTO resurgence_item
             (id, rotation_id, kind, name_en, name_zh, unique_name, family_key, relic_key, sort_order)
             VALUES (?1, ?2, 'warframe', ?3, ?4, ?5, ?6, NULL, ?7)",
            params![id, rotation_id, name_en, name_zh, unique, fk, sort],
        )
        .map_err(|e| format!("插入下期战甲失败: {e}"))?;
        sort += 1;
    }
    Ok(rotation_id)
}

/// 从 FeaturedItem 包名拆战甲家族：去掉 MPV / PrimeDualPack / PrimeSinglePack，再最长前缀匹配。
pub(crate) fn families_from_featured(featured: &str, known: &[String]) -> Vec<String> {
    let tail = featured.rsplit('/').next().unwrap_or(featured);
    let mut stem = tail.strip_prefix("MPV").unwrap_or(tail);
    for suffix in [
        "PrimeDualPack",
        "PrimeSinglePack",
        "DualPack",
        "SinglePack",
    ] {
        if let Some(s) = stem.strip_suffix(suffix) {
            stem = s;
            break;
        }
    }
    if stem.is_empty() {
        // warframestat 可能直接给 "Protea Prime"
        let key = featured
            .trim()
            .strip_suffix(" Prime")
            .unwrap_or(featured.trim());
        if known.iter().any(|k| k == key) {
            return vec![key.to_string()];
        }
        return Vec::new();
    }
    let mut sorted: Vec<&String> = known.iter().collect();
    sorted.sort_by_key(|k| std::cmp::Reverse(k.len()));
    let mut rest = stem;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let mut hit = false;
        for k in &sorted {
            if rest.starts_with(k.as_str()) {
                out.push((*k).clone());
                rest = &rest[k.len()..];
                hit = true;
                break;
            }
        }
        if !hit {
            break;
        }
    }
    out
}

pub(crate) fn upsert_rotation(
    tx: &rusqlite::Transaction<'_>,
    snowflake: &Snowflake,
    trader: &VaultTrader,
    source_url: &str,
) -> Result<i64, String> {
    upsert_rotation_times(
        tx,
        snowflake,
        trader.activation.as_deref(),
        trader.expiry.as_deref(),
        source_url,
    )
}

pub(crate) fn upsert_rotation_times(
    tx: &rusqlite::Transaction<'_>,
    snowflake: &Snowflake,
    activation: Option<&str>,
    expiry: Option<&str>,
    source_url: &str,
) -> Result<i64, String> {
    let now = Utc::now().to_rfc3339();
    let existing: Option<i64> = match activation {
        Some(act) if !act.is_empty() => tx
            .query_row(
                "SELECT id FROM resurgence_rotation WHERE activation = ?1",
                [act],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?,
        _ => None,
    };
    if let Some(id) = existing {
        tx.execute(
            "DELETE FROM resurgence_item WHERE rotation_id = ?1",
            [id],
        )
        .map_err(|e| format!("清空重生条目失败: {e}"))?;
        tx.execute(
            "UPDATE resurgence_rotation
             SET expiry = ?2, updated_at = ?3, source_url = ?4
             WHERE id = ?1",
            params![id, expiry, now, source_url],
        )
        .map_err(|e| format!("更新重生轮次失败: {e}"))?;
        return Ok(id);
    }
    let id = snowflake.next_id();
    tx.execute(
        "INSERT INTO resurgence_rotation (id, activation, expiry, updated_at, source_url)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, activation, expiry, now, source_url],
    )
    .map_err(|e| format!("插入重生轮次失败: {e}"))?;
    Ok(id)
}

/// 保留：当期 + 下期 + 最近一条上期（最多三轮）。
pub(crate) fn retain_rotations(
    tx: &rusqlite::Transaction<'_>,
    current_id: i64,
    next_id: Option<i64>,
) -> Result<(), String> {
    let mut keep = vec![current_id];
    if let Some(n) = next_id {
        if n != current_id {
            keep.push(n);
        }
    }
    let placeholders = keep
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT id FROM resurgence_rotation
         WHERE id NOT IN ({placeholders})
         ORDER BY COALESCE(expiry, activation, '') DESC
         LIMIT 1"
    );
    let prev: Option<i64> = {
        let mut stmt = tx.prepare(&sql).map_err(|e| e.to_string())?;
        let params_vec: Vec<&dyn rusqlite::ToSql> =
            keep.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        stmt.query_row(params_vec.as_slice(), |r| r.get(0))
            .optional()
            .map_err(|e| e.to_string())?
    };
    if let Some(p) = prev {
        keep.push(p);
    }
    let del_ph = keep
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(", ");
    let del_sql = format!("DELETE FROM resurgence_rotation WHERE id NOT IN ({del_ph})");
    let params_vec: Vec<&dyn rusqlite::ToSql> =
        keep.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
    tx.execute(&del_sql, params_vec.as_slice())
        .map_err(|e| format!("清理更早重生轮次失败: {e}"))?;
    Ok(())
}

pub(crate) fn parse_inventory(inventory: &[VaultItem]) -> Vec<ParsedItem> {
    let mut out = Vec::new();
    let mut seen_wf = HashSet::new();
    let mut seen_wp = HashSet::new();
    for raw in inventory {
        let uniq = raw.unique_name.as_deref().unwrap_or("").to_string();
        if uniq.is_empty() {
            continue;
        }
        if let Some(item) = classify_item(&uniq, raw.item.as_deref()) {
            match item.kind {
                "warframe" => {
                    let key = item.family_key.clone().unwrap_or_else(|| item.name_en.clone());
                    if !seen_wf.insert(key) {
                        continue;
                    }
                }
                "weapon" => {
                    if !seen_wp.insert(uniq.clone()) {
                        continue;
                    }
                }
                _ => {}
            }
            out.push(item);
        }
    }
    out
}

pub(crate) fn classify_item(unique_name: &str, item_label: Option<&str>) -> Option<ParsedItem> {
    let u = unique_name;
    if u.contains("VoidProjection") || u.contains("/Projections/") {
        return Some(ParsedItem {
            kind: "relic_store",
            name_en: display_name(item_label, u),
            unique_name: u.to_string(),
            family_key: None,
        });
    }
    if u.contains("/Powersuits/")
        && u.contains("Prime")
        && !u.contains("Package")
        && !u.contains("Bobble")
        && !u.contains("Sentinel")
        && !u.contains("MegaPrimeVault")
    {
        let name = display_name(item_label, u);
        let family = name
            .strip_suffix(" Prime")
            .or_else(|| name.strip_suffix("Prime"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| family_from_powersuit_path(u));
        return Some(ParsedItem {
            kind: "warframe",
            name_en: if name.contains("Prime") {
                name
            } else {
                format!("{name} Prime")
            },
            unique_name: u.to_string(),
            family_key: family,
        });
    }
    if u.contains("/Weapons/") {
        let name = clean_weapon_name(item_label, u);
        let (family_key, _) = weapon_family_and_variant(&name);
        return Some(ParsedItem {
            kind: "weapon",
            name_en: name,
            unique_name: u.to_string(),
            family_key: Some(family_key),
        });
    }
    None
}

pub(crate) fn family_from_powersuit_path(unique_name: &str) -> Option<String> {
    // /Lotus/StoreItems/Powersuits/Banshee/BansheePrime
    let tail = unique_name.rsplit('/').next()?;
    let base = tail.strip_suffix("Prime")?;
    if base.is_empty() {
        return None;
    }
    Some(base.to_string())
}

pub(crate) fn display_name(item_label: Option<&str>, unique_name: &str) -> String {
    if let Some(s) = item_label.map(str::trim).filter(|s| !s.is_empty()) {
        // 跳过明显乱码空格名，改从 path 取
        if s.chars().filter(|c| c.is_ascii_alphabetic()).count() >= 3 {
            return s.to_string();
        }
    }
    let tail = unique_name
        .rsplit('/')
        .next()
        .unwrap_or(unique_name);
    humanize_path_tail(tail)
}

/// 路径尾段 CamelCase → 空格形式：`BansheePrime` → `Banshee Prime`。
pub(crate) fn humanize_path_tail(tail: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = tail.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && c.is_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).map(|n| n.is_lowercase()).unwrap_or(false);
            // 插入空格：小写/数字后接大写，或连续大写后接「大写+小写」（…SGPrime → …SG Prime）
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower)
            {
                out.push(' ');
            }
        }
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn clean_weapon_name(item_label: Option<&str>, unique_name: &str) -> String {
    let raw = display_name(item_label, unique_name);
    // "Prime Ak Bolto Weapon" / "Kogake Prime Knuckles" / path CamelCase
    let mut s = raw
        .replace(" Weapon", "")
        .replace(" Knuckles", "")
        .replace(" Gun", "")
        .replace("Prime ", "")
        .trim()
        .to_string();
    if let Some(rest) = unique_name.rsplit('/').next() {
        if raw.chars().filter(|c| c.is_ascii_alphabetic()).count() < 4 {
            s = humanize_path_tail(rest)
                .replace(" Weapon", "")
                .replace(" Knuckles", "")
                .replace(" Gun", "")
                .replace("Prime ", "")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
        }
    }
    // 常见映射：AllNew1hSG → Euphona Prime（路径里没有好名字时保留清洗结果）
    if unique_name.contains("AllNew1hSG") {
        return "Euphona Prime".into();
    }
    if unique_name.contains("PrimeAkbolto") || unique_name.contains("AkBolto") {
        return "Akbolto Prime".into();
    }
    if unique_name.contains("PrimeKogake") || unique_name.contains("KogakePrime") {
        return "Kogake Prime".into();
    }
    if !s.contains("Prime") && unique_name.contains("Prime") {
        if s.is_empty() {
            return s;
        }
        return format!("{s} Prime");
    }
    s
}

/// `/Lotus/StoreItems/...` → `/Lotus/...`，对齐 WFCD 实体 unique_name。
pub(crate) fn normalize_store_unique(unique_name: &str) -> String {
    unique_name.replace("/StoreItems/", "/")
}

/// 武器货架路径尾段常带 Weapon/Knuckles，实体表没有这些后缀。
fn weapon_unique_candidates(unique_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let push = |list: &mut Vec<String>, s: String| {
        if !s.is_empty() && !list.iter().any(|x| x == &s) {
            list.push(s);
        }
    };
    push(&mut out, unique_name.to_string());
    let norm = normalize_store_unique(unique_name);
    push(&mut out, norm.clone());
    if let Some((parent, leaf)) = norm.rsplit_once('/') {
        for suffix in ["Weapon", "Knuckles", "Gun"] {
            if let Some(base) = leaf.strip_suffix(suffix) {
                if !base.is_empty() {
                    push(&mut out, format!("{parent}/{base}"));
                }
            }
        }
    }
    out
}

/// 用 unique_name 对齐实体表，取展示名与家族键。
pub(crate) fn lookup_entity_names(
    tx: &rusqlite::Transaction<'_>,
    kind: &str,
    unique_name: &str,
) -> Result<Option<(String, Option<String>, Option<String>)>, String> {
    match kind {
        "warframe" => {
            for cand in [unique_name.to_string(), normalize_store_unique(unique_name)] {
                let row: Option<(String, Option<String>, Option<String>)> = tx
                    .query_row(
                        "SELECT w.name_en, w.name_zh, f.family_key
                         FROM warframe w
                         LEFT JOIN warframe_family f ON f.id = w.family_id
                         WHERE w.unique_name = ?1",
                        [&cand],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                if row.is_some() {
                    return Ok(row);
                }
            }
            Ok(None)
        }
        "weapon" => {
            for cand in weapon_unique_candidates(unique_name) {
                let row: Option<(String, Option<String>, Option<String>)> = tx
                    .query_row(
                        "SELECT w.name_en, w.name_zh, wf.family_key
                         FROM weapon w
                         LEFT JOIN weapon_family wf ON wf.id = w.family_id
                         WHERE w.unique_name = ?1",
                        [&cand],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                if row.is_some() {
                    return Ok(row);
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}


