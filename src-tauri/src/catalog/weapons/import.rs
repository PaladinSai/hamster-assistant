//! 武器 catalog 入库事务。

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::catalog::i18n::ZhLookup;
use crate::catalog::modular;
use crate::catalog::wfcd::{
    vault_status_code, WEAPON_KIND_ARCHWING, WEAPON_KIND_WEAPON,
};
use crate::id::Snowflake;

use super::parse::{modular_zh_fallback, parse_modular_parts, parse_weapons, ModularPart};
use super::rules::{
    can_incarnon, is_material_component, is_space_product_category, skip_craft_parts,
    weapon_family_and_variant, weapon_part_meta, WeaponImportStats, WfcdWeapon,
    SLOT_MELEE, SLOT_MODULAR, SLOT_PRIMARY, SLOT_SECONDARY, VARIANT_BASE, VARIANT_PRIME,
};

/// 合并三槽 JSON 导入（地面武器）。
pub fn import_weapons_json(
    conn: &mut Connection,
    primary: &[u8],
    secondary: &[u8],
    melee: &[u8],
    snowflake: &Snowflake,
) -> Result<WeaponImportStats, String> {
    let mut items = Vec::new();
    items.extend(parse_weapons(primary, SLOT_PRIMARY)?);
    items.extend(parse_weapons(secondary, SLOT_SECONDARY)?);
    items.extend(parse_weapons(melee, SLOT_MELEE)?);

    let tx = conn
        .transaction()
        .map_err(|e| format!("武器事务失败: {e}"))?;
    let stats = import_in_tx(&tx, &items, snowflake, WEAPON_KIND_WEAPON)?;
    tx.commit().map_err(|e| format!("武器提交失败: {e}"))?;
    Ok(stats)
}

/// Arch-Gun + Arch-Melee → 空战武器（slot：枪械=0 / 近战=2）。
/// 仅入库 `productCategory` 为 SpaceGuns / SpaceMelee 的条目；
/// JSON 里误挂的地面武器（如闪劫 Prime）改按地面近战入库。
pub fn import_archwing_weapons_json(
    conn: &mut Connection,
    arch_gun: &[u8],
    arch_melee: &[u8],
    snowflake: &Snowflake,
) -> Result<WeaponImportStats, String> {
    let guns = parse_weapons(arch_gun, SLOT_PRIMARY)?;
    let melees = parse_weapons(arch_melee, SLOT_MELEE)?;

    let mut arch_items = Vec::new();
    let mut stray_ground = Vec::new();
    for (w, slot) in guns {
        if is_space_product_category(&w, "SpaceGuns") {
            arch_items.push((w, slot));
        } else {
            eprintln!(
                "[仓鼠] Arch-Gun 非 SpaceGuns，改按地面武器: {} ({:?})",
                w.name, w.product_category
            );
            stray_ground.push((w, slot));
        }
    }
    for (w, slot) in melees {
        if is_space_product_category(&w, "SpaceMelee") {
            arch_items.push((w, slot));
        } else {
            eprintln!(
                "[仓鼠] Arch-Melee 非 SpaceMelee，改按地面近战: {} ({:?})",
                w.name, w.product_category
            );
            // 误挂条目按近战槽入库（如闪劫 Prime）
            stray_ground.push((w, SLOT_MELEE));
        }
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("空战武器事务失败: {e}"))?;
    let mut stats = import_in_tx(&tx, &arch_items, snowflake, WEAPON_KIND_ARCHWING)?;
    if !stray_ground.is_empty() {
        let stray = import_in_tx(&tx, &stray_ground, snowflake, WEAPON_KIND_WEAPON)?;
        stats.families += stray.families;
        stats.weapons += stray.weapons;
        stats.parts += stray.parts;
    }
    tx.commit().map_err(|e| format!("空战武器提交失败: {e}"))?;
    Ok(stats)
}

/// Misc + Melee → 自制精通部件（slot=3，`modular_kind`=kitgun\|zaw\|amp）。
/// 与同伴侧 [`crate::catalog::import::import_modular_companions_json`] 对称，见 [`crate::catalog::modular`]。
pub fn import_modular_weapons_json(
    conn: &mut Connection,
    misc: &[u8],
    melee: &[u8],
    snowflake: &Snowflake,
) -> Result<WeaponImportStats, String> {
    let mut items = Vec::new();
    items.extend(parse_modular_parts(misc)?);
    items.extend(parse_modular_parts(melee)?);

    let tx = conn
        .transaction()
        .map_err(|e| format!("自制武器事务失败: {e}"))?;
    let stats = import_modular_in_tx(&tx, &items, snowflake)?;
    tx.commit().map_err(|e| format!("自制武器提交失败: {e}"))?;
    Ok(stats)
}

fn import_modular_in_tx(
    tx: &Transaction<'_>,
    items: &[ModularPart],
    snowflake: &Snowflake,
) -> Result<WeaponImportStats, String> {
    let _ = tx.execute_batch("PRAGMA foreign_keys = ON;");
    let mut family_ids: HashMap<String, i64> = load_family_map(tx)?;
    let mut weapon_ids: HashMap<String, i64> = load_weapon_map(tx)?;
    let now = Utc::now().to_rfc3339();
    let mut stats = WeaponImportStats::default();
    let zh = ZhLookup::load(tx)?;

    for p in items {
        let family_key = p.name_en.clone();
        let name_zh = modular::resolve_name_zh(&zh, &p.name_en, modular_zh_fallback);

        let family_id = if let Some(&id) = family_ids.get(&family_key) {
            tx.execute(
                "UPDATE weapon_family
                 SET name_en = ?2, name_zh = COALESCE(?3, name_zh),
                     slot = ?4, weapon_kind = ?5, modular_kind = ?6, updated_at = ?7
                 WHERE id = ?1",
                params![
                    id,
                    p.name_en,
                    name_zh,
                    SLOT_MODULAR,
                    WEAPON_KIND_WEAPON,
                    p.modular_kind,
                    now
                ],
            )
            .map_err(|e| format!("更新自制家族 {} 失败: {e}", p.name_en))?;
            id
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO weapon_family
                 (id, family_key, name_en, name_zh, nickname, slot, weapon_kind, modular_kind, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?8, ?8)",
                params![
                    id,
                    family_key,
                    p.name_en,
                    name_zh,
                    SLOT_MODULAR,
                    WEAPON_KIND_WEAPON,
                    p.modular_kind,
                    now
                ],
            )
            .map_err(|e| format!("插入自制家族 {} 失败: {e}", p.name_en))?;
            family_ids.insert(family_key, id);
            stats.families += 1;
            id
        };

        if let Some(&wid) = weapon_ids.get(&p.unique_name) {
            tx.execute(
                "UPDATE weapon SET family_id=?2, variant=?3, name_en=?4, name_zh=COALESCE(?5, name_zh),
                 unique_name=?6, can_incarnon=0, vault_status=0, sort_order=0 WHERE id=?1",
                params![wid, family_id, VARIANT_BASE, p.name_en, name_zh, p.unique_name],
            )
            .map_err(|e| format!("更新自制部件 {} 失败: {e}", p.name_en))?;
        } else {
            let wid = snowflake.next_id();
            tx.execute(
                "INSERT INTO weapon
                 (id, family_id, variant, name_en, name_zh, unique_name, can_incarnon, vault_status, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0)",
                params![
                    wid,
                    family_id,
                    VARIANT_BASE,
                    p.name_en,
                    name_zh,
                    p.unique_name
                ],
            )
            .map_err(|e| format!("插入自制部件 {} 失败: {e}", p.name_en))?;
            weapon_ids.insert(p.unique_name.clone(), wid);
            stats.weapons += 1;
        }
    }
    Ok(stats)
}

fn import_in_tx(
    tx: &Transaction<'_>,
    items: &[(WfcdWeapon, i32)],
    snowflake: &Snowflake,
    weapon_kind: &str,
) -> Result<WeaponImportStats, String> {
    let _ = tx.execute_batch("PRAGMA foreign_keys = ON;");

    let mut family_ids: HashMap<String, i64> = load_family_map(tx)?;
    let mut weapon_ids: HashMap<String, i64> = load_weapon_map(tx)?;
    let mut part_ids: HashMap<String, i64> = load_part_map(tx)?;
    // (family_id, variant) → weapon id，用于冲突时跳过重复名
    let mut variant_owner: HashMap<(i64, i32), i64> = load_variant_owner(tx)?;

    let now = Utc::now().to_rfc3339();
    let mut stats = WeaponImportStats::default();
    let zh = ZhLookup::load(tx)?;

    for (w, slot) in items {
        let (family_key, _) = weapon_family_and_variant(&w.name);
        if family_ids.contains_key(&family_key) {
            continue;
        }
        let id = snowflake.next_id();
        let name_zh = zh.get(&family_key);
        tx.execute(
            "INSERT INTO weapon_family (id, family_key, name_en, name_zh, nickname, slot, weapon_kind, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7, ?7)",
            params![id, family_key, family_key, name_zh, slot, weapon_kind, now],
        )
        .map_err(|e| format!("插入武器家族 {family_key} 失败: {e}"))?;
        family_ids.insert(family_key, id);
        stats.families += 1;
    }

    // 仅回写本批次家族译名；不改写已有 weapon_kind（避免误挂条目把地面家族标成空战）
    let mut touched: HashMap<String, i64> = HashMap::new();
    for (w, _) in items {
        let (family_key, _) = weapon_family_and_variant(&w.name);
        if let Some(&id) = family_ids.get(&family_key) {
            touched.insert(family_key, id);
        }
    }
    for (key, id) in &touched {
        let name_zh = zh.get(key);
        tx.execute(
            "UPDATE weapon_family SET name_en = ?2, name_zh = COALESCE(?3, name_zh), updated_at = ?4 WHERE id = ?1",
            params![id, key, name_zh, now],
        )
        .map_err(|e| e.to_string())?;
    }

    for (w, _slot) in items {
        let (family_key, variant) = weapon_family_and_variant(&w.name);
        let family_id = *family_ids
            .get(&family_key)
            .ok_or_else(|| format!("缺少武器家族 {family_key}"))?;
        let incarnon = can_incarnon(&family_key, variant, &w.tags);
        let vault = vault_status_code(variant == VARIANT_PRIME, w.vaulted);

        if let Some(&existing_id) = variant_owner.get(&(family_id, variant)) {
            if weapon_ids.get(&w.unique_name).copied() != Some(existing_id) {
                // 同家族同变体已有另一把枪（如双 Grimoire）→ 跳过
                continue;
            }
        }

        let wep_zh = zh.get(&w.name).or_else(|| zh.get(&family_key));
        let weapon_id = if let Some(id) = weapon_ids.get(&w.unique_name).copied() {
            tx.execute(
                "UPDATE weapon SET family_id=?2, variant=?3, name_en=?4, name_zh=COALESCE(?5, name_zh), unique_name=?6,
                 can_incarnon=?7, vault_status=?8, sort_order=?3 WHERE id=?1",
                params![id, family_id, variant, w.name, wep_zh, w.unique_name, incarnon, vault],
            )
            .map_err(|e| format!("更新武器 {} 失败: {e}", w.name))?;
            id
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO weapon (id, family_id, variant, name_en, name_zh, unique_name, can_incarnon, vault_status, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?3)",
                params![id, family_id, variant, w.name, wep_zh, w.unique_name, incarnon, vault],
            )
            .map_err(|e| format!("插入武器 {} 失败: {e}", w.name))?;
            weapon_ids.insert(w.unique_name.clone(), id);
            variant_owner.insert((family_id, variant), id);
            stats.weapons += 1;
            id
        };
        variant_owner.insert((family_id, variant), weapon_id);

        if skip_craft_parts(variant) {
            continue;
        }

        for comp in &w.components {
            if is_material_component(comp) {
                continue;
            }
            let Some(meta) = weapon_part_meta(&comp.name) else {
                continue;
            };
            let need_qty = comp.item_count.unwrap_or(1).max(1);
            let part_map_key = format!("{}#{}", w.unique_name, meta.key);

            if let Some(pid) = part_ids.get(&part_map_key).copied() {
                tx.execute(
                    "UPDATE weapon_part SET weapon_id=?2, part_key=?3, name_zh=?4, need_qty=?5, sort_order=?6
                     WHERE id=?1",
                    params![pid, weapon_id, meta.key, meta.name_zh, need_qty, meta.sort],
                )
                .map_err(|e| format!("更新武器部件 {part_map_key} 失败: {e}"))?;
            } else {
                let pid = snowflake.next_id();
                tx.execute(
                    "INSERT INTO weapon_part (id, weapon_id, part_key, name_zh, need_qty, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![pid, weapon_id, meta.key, meta.name_zh, need_qty, meta.sort],
                )
                .map_err(|e| format!("插入武器部件 {part_map_key} 失败: {e}"))?;
                part_ids.insert(part_map_key, pid);
                stats.parts += 1;
            }
        }
    }

    Ok(stats)
}

fn load_family_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare("SELECT family_key, id FROM weapon_family")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, id) = r.map_err(|e| e.to_string())?;
        map.insert(k, id);
    }
    Ok(map)
}

fn load_weapon_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare("SELECT unique_name, id FROM weapon WHERE unique_name IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, id) = r.map_err(|e| e.to_string())?;
        map.insert(k, id);
    }
    Ok(map)
}

fn load_part_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare(
            "SELECT w.unique_name, p.part_key, p.id
             FROM weapon_part p
             JOIN weapon w ON w.id = p.weapon_id
             WHERE w.unique_name IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (uniq, pkey, id) = r.map_err(|e| e.to_string())?;
        map.insert(format!("{uniq}#{pkey}"), id);
    }
    Ok(map)
}

fn load_variant_owner(tx: &Transaction<'_>) -> Result<HashMap<(i64, i32), i64>, String> {
    let mut stmt = tx
        .prepare("SELECT family_id, variant, id FROM weapon")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i32>(1)?, r.get::<_, i64>(2)?))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (fid, v, id) = r.map_err(|e| e.to_string())?;
        map.insert((fid, v), id);
    }
    Ok(map)
}

#[allow(dead_code)]
pub fn count_weapon_families(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM weapon_family", [], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())
        .map(|o| o.unwrap_or(0))
}
