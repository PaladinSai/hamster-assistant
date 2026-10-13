//! 武器进度。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::catalog::is_incarnon_genesis_family;
use super::common::{id_to_str, load_part_relic_drops, open_pair, parse_id, PartDropDto};

const WEAPON_VARIANT_KUVA: i32 = 2;
const WEAPON_VARIANT_TENET: i32 = 3;
const WEAPON_VARIANT_CODA: i32 = 4;

fn is_special_weapon_variant(variant: i32) -> bool {
    matches!(
        variant,
        WEAPON_VARIANT_KUVA | WEAPON_VARIANT_TENET | WEAPON_VARIANT_CODA
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponMemberDto {
    pub weapon_id: String,
    pub variant: i32,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub can_incarnon: i32,
    pub vault_status: i32,
    pub status: i32,
    pub incarnon: i32,
    pub wish: i32,
    pub mastered: i32,
    pub note: Option<String>,
    /// catalog `weapon_part` 行数；>1 才允许点开抽屉追部件
    pub part_count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponFamilyListItem {
    pub family_id: String,
    pub family_key: String,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub nickname: Option<String>,
    pub slot: i32,
    /// weapon | archwing
    pub weapon_kind: String,
    /// kitgun | zaw | amp；自制精通部件
    pub modular_kind: Option<String>,
    pub members: Vec<WeaponMemberDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponPartDto {
    pub part_id: String,
    pub part_key: String,
    pub name_zh: String,
    pub sort_order: i32,
    pub need_qty: i32,
    pub have_qty: i32,
    pub status: i32,
    pub note: Option<String>,
    pub drops: Vec<PartDropDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponDetailDto {
    pub weapon_id: String,
    pub variant: i32,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub unique_name: Option<String>,
    pub can_incarnon: i32,
    pub vault_status: i32,
    pub status: i32,
    pub incarnon: i32,
    pub wish: i32,
    pub mastered: i32,
    pub note: Option<String>,
    pub parts: Vec<WeaponPartDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponFamilyDetail {
    pub family: WeaponFamilyListItem,
    pub weapons: Vec<WeaponDetailDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWeaponProgress {
    pub weapon_id: String,
    pub status: Option<i32>,
    pub note: Option<String>,
    pub incarnon: Option<i32>,
    pub wish: Option<i32>,
    pub mastered: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWeaponPartProgress {
    pub part_id: String,
    pub status: Option<i32>,
    pub have_qty: Option<i32>,
    pub note: Option<String>,
}

#[derive(Clone, Default)]
struct WeaponProgressRow {
    status: i32,
    note: Option<String>,
    incarnon: i32,
    wish: i32,
    mastered: i32,
}

#[derive(Clone, Default)]
struct WeaponPartProgressRow {
    status: i32,
    have_qty: i32,
    note: Option<String>,
}

pub fn list_weapon_families(
    data_dir: &Path,
    weapon_kind: Option<&str>,
) -> Result<Vec<WeaponFamilyListItem>, String> {
    let kind = weapon_kind.unwrap_or("weapon");
    let (catalog, user) = open_pair(data_dir)?;
    let progress = load_all_weapon_progress(&user)?;

    // 一次拉齐部件数，避免 per-weapon COUNT
    let mut part_counts: HashMap<i64, i32> = HashMap::new();
    {
        let mut stmt = catalog
            .prepare("SELECT weapon_id, COUNT(*) FROM weapon_part GROUP BY weapon_id")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i32>(1)?)))
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (wid, n) = r.map_err(|e| e.to_string())?;
            part_counts.insert(wid, n);
        }
    }

    let mut stmt = catalog
        .prepare(
            "SELECT f.id, f.family_key, f.name_en, f.name_zh, f.nickname, f.slot,
                    IFNULL(f.weapon_kind, 'weapon'), f.modular_kind,
                    w.id, w.variant, w.name_en, w.name_zh, w.can_incarnon, w.vault_status
             FROM weapon_family f
             LEFT JOIN weapon w ON w.family_id = f.id
             WHERE IFNULL(f.weapon_kind, 'weapon') = ?1
             ORDER BY f.slot, f.name_en, w.variant, w.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![kind], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<i64>>(8)?,
                row.get::<_, Option<i32>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<i32>>(12)?,
                row.get::<_, Option<i32>>(13)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out: Vec<WeaponFamilyListItem> = Vec::new();
    let mut last_fid: Option<i64> = None;
    for r in rows {
        let (
            fid,
            key,
            fam_en,
            fam_zh,
            nickname,
            slot,
            weapon_kind,
            modular_kind,
            wid_opt,
            variant_opt,
            w_en,
            w_zh,
            can_inc_opt,
            vault_opt,
        ) = r.map_err(|e| e.to_string())?;

        if last_fid != Some(fid) {
            out.push(WeaponFamilyListItem {
                family_id: id_to_str(fid),
                family_key: key,
                name_en: fam_en,
                name_zh: fam_zh,
                nickname,
                slot,
                weapon_kind,
                modular_kind,
                members: Vec::new(),
            });
            last_fid = Some(fid);
        }

        let Some(wid) = wid_opt else {
            continue;
        };
        let variant = variant_opt.unwrap_or(0);
        let can_incarnon = can_inc_opt.unwrap_or(0);
        let vault_status = vault_opt.unwrap_or(0);
        let name_en = w_en.unwrap_or_default();
        let pf = progress.get(&wid).cloned().unwrap_or_default();
        let incarnon = if can_incarnon == 0 { 0 } else { pf.incarnon };
        let part_count = part_counts.get(&wid).copied().unwrap_or(0);
        if let Some(fam) = out.last_mut() {
            fam.members.push(WeaponMemberDto {
                weapon_id: id_to_str(wid),
                variant,
                name_en,
                name_zh: w_zh,
                can_incarnon,
                vault_status,
                status: pf.status,
                incarnon,
                wish: pf.wish,
                mastered: pf.mastered,
                note: pf.note,
                part_count,
            });
        }
    }
    Ok(out)
}

pub fn get_weapon_family_detail(
    data_dir: &Path,
    family_id_raw: &str,
) -> Result<WeaponFamilyDetail, String> {
    let family_id = parse_id(family_id_raw, "武器家族 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let progress = load_all_weapon_progress(&user)?;
    let part_progress = load_all_weapon_part_progress(&user)?;

    let (key, name_en, name_zh, nickname, slot, weapon_kind, modular_kind): (
        String,
        String,
        Option<String>,
        Option<String>,
        i32,
        String,
        Option<String>,
    ) = catalog
        .query_row(
            "SELECT f.family_key, f.name_en, f.name_zh, f.nickname, f.slot,
                    IFNULL(f.weapon_kind, 'weapon'), f.modular_kind
             FROM weapon_family f
             WHERE f.id = ?1",
            [family_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|e| format!("未找到武器家族 id={family_id_raw}: {e}"))?;

    let members = load_weapon_members(&catalog, family_id, &progress)?;
    let family = WeaponFamilyListItem {
        family_id: id_to_str(family_id),
        family_key: key,
        name_en,
        name_zh,
        nickname,
        slot,
        weapon_kind,
        modular_kind,
        members: members.clone(),
    };

    let resurgence_relics = crate::catalog::load_resurgence_relic_keys(&catalog).unwrap_or_default();
    let mut weapons = Vec::new();
    for m in &members {
        let wid = parse_id(&m.weapon_id, "武器 id")?;
        let unique_name: Option<String> = catalog
            .query_row(
                "SELECT unique_name FROM weapon WHERE id = ?1",
                [wid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();
        let parts = if is_special_weapon_variant(m.variant) {
            Vec::new()
        } else {
            load_weapon_parts(&catalog, wid, &part_progress, &resurgence_relics)?
        };
        weapons.push(WeaponDetailDto {
            weapon_id: m.weapon_id.clone(),
            variant: m.variant,
            name_en: m.name_en.clone(),
            name_zh: m.name_zh.clone(),
            unique_name,
            can_incarnon: m.can_incarnon,
            vault_status: m.vault_status,
            status: m.status,
            incarnon: m.incarnon,
            wish: m.wish,
            mastered: m.mastered,
            note: m.note.clone(),
            parts,
        });
    }

    Ok(WeaponFamilyDetail { family, weapons })
}

pub fn upsert_weapon_progress(data_dir: &Path, patch: UpsertWeaponProgress) -> Result<(), String> {
    let weapon_id = parse_id(&patch.weapon_id, "武器 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let (variant, can_incarnon, family_key): (i32, i32, String) = catalog
        .query_row(
            "SELECT w.variant, w.can_incarnon, f.family_key
             FROM weapon w
             JOIN weapon_family f ON f.id = w.family_id
             WHERE w.id = ?1",
            [weapon_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| format!("武器 id={weapon_id} 不存在: {e}"))?;

    let existing = user
        .query_row(
            "SELECT status, note, incarnon, wish, mastered FROM weapon_progress WHERE weapon_id = ?1",
            [weapon_id],
            |row| {
                Ok(WeaponProgressRow {
                    status: row.get(0)?,
                    note: row.get(1)?,
                    incarnon: row.get(2)?,
                    wish: row.get(3)?,
                    mastered: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let mut status = patch.status.unwrap_or(existing.status);
    // 赤毒/信条/终幕：无「蓝图已齐」
    if is_special_weapon_variant(variant) && status == 2 {
        status = 0;
    }
    let mut incarnon = patch.incarnon.unwrap_or(existing.incarnon);
    if can_incarnon == 0 {
        incarnon = 0;
    } else if patch.incarnon.is_none()
        && status == 1
        && incarnon == 0
        && !is_incarnon_genesis_family(&family_key)
    {
        // 自带灵化：已获取 → 自动进入「已有灵化之源」
        incarnon = 2;
    }
    let wish = patch
        .wish
        .map(|w| if w != 0 { 1 } else { 0 })
        .unwrap_or(existing.wish);
    let mastered = patch
        .mastered
        .map(|m| if m != 0 { 1 } else { 0 })
        .unwrap_or(existing.mastered);
    let row = WeaponProgressRow {
        status,
        note: patch.note.clone().or(existing.note),
        incarnon,
        wish,
        mastered,
    };

    user.execute(
        "INSERT INTO weapon_progress (weapon_id, status, note, incarnon, wish, mastered, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(weapon_id) DO UPDATE SET
            status=excluded.status,
            note=excluded.note,
            incarnon=excluded.incarnon,
            wish=excluded.wish,
            mastered=excluded.mastered,
            updated_at=excluded.updated_at",
        params![
            weapon_id,
            row.status,
            row.note,
            row.incarnon,
            row.wish,
            row.mastered,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    if patch.status.is_some() && !is_special_weapon_variant(variant) {
        cascade_weapon_parts_status(
            &user,
            &catalog,
            weapon_id,
            weapon_status_to_part_status(row.status),
            &now,
        )?;
    }

    Ok(())
}

/// 整枪进度 → 部件进度（部件仅 0 没有 / 3 已有）
fn weapon_status_to_part_status(status: i32) -> i32 {
    match status {
        1 | 2 => 3, // 已获取 / 蓝图已齐 → 已有
        _ => 0,
    }
}

pub fn upsert_weapon_part_progress(
    data_dir: &Path,
    patch: UpsertWeaponPartProgress,
) -> Result<(), String> {
    let part_id = parse_id(&patch.part_id, "武器部件 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let need_qty: i32 = catalog
        .query_row(
            "SELECT need_qty FROM weapon_part WHERE id = ?1",
            [part_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("武器部件 id={part_id} 不存在: {e}"))?;

    let existing = user
        .query_row(
            "SELECT status, have_qty, note FROM weapon_part_progress WHERE part_id = ?1",
            [part_id],
            |row| {
                Ok(WeaponPartProgressRow {
                    status: row.get(0)?,
                    have_qty: row.get(1)?,
                    note: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let have_qty = patch.have_qty.unwrap_or(existing.have_qty).max(0);
    let mut status = patch.status.unwrap_or(existing.status);
    // 数量达标自动标已有
    if have_qty >= need_qty && need_qty > 0 {
        status = 3;
    }
    // 部件不再使用铸造中(2)
    if status == 2 {
        status = 0;
    }
    let row = WeaponPartProgressRow {
        status,
        have_qty,
        note: patch.note.clone().or(existing.note),
    };

    user.execute(
        "INSERT INTO weapon_part_progress (part_id, status, have_qty, note, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(part_id) DO UPDATE SET
            status=excluded.status,
            have_qty=excluded.have_qty,
            note=excluded.note,
            updated_at=excluded.updated_at",
        params![part_id, row.status, row.have_qty, row.note, now],
    )
    .map_err(|e| e.to_string())?;

    if patch.status.is_some() || patch.have_qty.is_some() {
        recompute_weapon_from_part(&user, &catalog, part_id, &now)?;
    }

    Ok(())
}

fn load_all_weapon_progress(user: &Connection) -> Result<HashMap<i64, WeaponProgressRow>, String> {
    let mut stmt = user
        .prepare("SELECT weapon_id, status, note, incarnon, wish, mastered FROM weapon_progress")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                WeaponProgressRow {
                    status: row.get(1)?,
                    note: row.get(2)?,
                    incarnon: row.get(3)?,
                    wish: row.get(4)?,
                    mastered: row.get(5)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (id, row) = r.map_err(|e| e.to_string())?;
        map.insert(id, row);
    }
    Ok(map)
}

fn load_all_weapon_part_progress(
    user: &Connection,
) -> Result<HashMap<i64, WeaponPartProgressRow>, String> {
    let mut stmt = user
        .prepare("SELECT part_id, status, have_qty, note FROM weapon_part_progress")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                WeaponPartProgressRow {
                    status: row.get(1)?,
                    have_qty: row.get(2)?,
                    note: row.get(3)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (id, row) = r.map_err(|e| e.to_string())?;
        map.insert(id, row);
    }
    Ok(map)
}

fn load_weapon_members(
    catalog: &Connection,
    family_id: i64,
    progress: &HashMap<i64, WeaponProgressRow>,
) -> Result<Vec<WeaponMemberDto>, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT w.id, w.variant, w.name_en, w.name_zh, w.can_incarnon, w.vault_status,
                    (SELECT COUNT(*) FROM weapon_part p WHERE p.weapon_id = w.id)
             FROM weapon w
             WHERE w.family_id = ?1
             ORDER BY w.variant, w.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([family_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut members = Vec::new();
    for r in rows {
        let (wid, variant, name_en_opt, name_zh, can_incarnon, vault_status, part_count) =
            r.map_err(|e| e.to_string())?;
        let name_en = name_en_opt.unwrap_or_default();
        let pf = progress.get(&wid).cloned().unwrap_or_default();
        let incarnon = if can_incarnon == 0 { 0 } else { pf.incarnon };
        members.push(WeaponMemberDto {
            weapon_id: id_to_str(wid),
            variant,
            name_en,
            name_zh,
            can_incarnon,
            vault_status,
            status: pf.status,
            incarnon,
            wish: pf.wish,
            mastered: pf.mastered,
            note: pf.note,
            part_count,
        });
    }
    Ok(members)
}

fn load_weapon_parts(
    catalog: &Connection,
    weapon_id: i64,
    part_progress: &HashMap<i64, WeaponPartProgressRow>,
    resurgence_relics: &std::collections::HashSet<String>,
) -> Result<Vec<WeaponPartDto>, String> {
    let mut stmt = catalog
        .prepare(
            "SELECT id, part_key, name_zh, need_qty, sort_order
             FROM weapon_part WHERE weapon_id = ?1 ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([weapon_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut parts = Vec::new();
    for r in rows {
        let (pid, pkey, name_zh, need_qty, sort) = r.map_err(|e| e.to_string())?;
        let pf = part_progress
            .get(&pid)
            .cloned()
            .unwrap_or_default();
        let drops = load_part_relic_drops(catalog, pid, "weapon_part_relic", resurgence_relics)?;
        parts.push(WeaponPartDto {
            part_id: id_to_str(pid),
            part_key: pkey,
            name_zh,
            sort_order: sort,
            need_qty,
            have_qty: pf.have_qty,
            status: pf.status,
            note: pf.note,
            drops,
        });
    }
    Ok(parts)
}


fn list_weapon_part_ids(catalog: &Connection, weapon_id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = catalog
        .prepare("SELECT id FROM weapon_part WHERE weapon_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([weapon_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for r in rows {
        ids.push(r.map_err(|e| e.to_string())?);
    }
    Ok(ids)
}

fn cascade_weapon_parts_status(
    user: &Connection,
    catalog: &Connection,
    weapon_id: i64,
    status: i32,
    now: &str,
) -> Result<(), String> {
    // catalog / user 分库，无法 INSERT…SELECT；复用预编译语句减少逐行 prepare
    let ids = list_weapon_part_ids(catalog, weapon_id)?;
    if ids.is_empty() {
        return Ok(());
    }
    let mut stmt = user
        .prepare(
            "INSERT INTO weapon_part_progress (part_id, status, have_qty, note, updated_at)
             VALUES (?1, ?2, 0, NULL, ?3)
             ON CONFLICT(part_id) DO UPDATE SET
                status=excluded.status,
                updated_at=excluded.updated_at",
        )
        .map_err(|e| e.to_string())?;
    for part_id in ids {
        stmt.execute(params![part_id, status, now])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn recompute_weapon_from_part(
    user: &Connection,
    catalog: &Connection,
    part_id: i64,
    now: &str,
) -> Result<(), String> {
    let (weapon_id, variant): (i64, i32) = catalog
        .query_row(
            "SELECT w.id, w.variant
             FROM weapon_part p
             JOIN weapon w ON w.id = p.weapon_id
             WHERE p.id = ?1",
            [part_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| format!("部件所属武器查询失败: {e}"))?;

    if is_special_weapon_variant(variant) {
        return Ok(());
    }

    let part_ids = list_weapon_part_ids(catalog, weapon_id)?;
    if part_ids.is_empty() {
        return Ok(());
    }

    // 全齐 = 每个部件都有 status=3 的进度行
    let placeholders = std::iter::repeat_n("?", part_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT COUNT(*) FROM weapon_part_progress WHERE part_id IN ({placeholders}) AND status = 3"
    );
    let owned_cnt: i64 = user
        .query_row(&sql, params_from_iter(part_ids.iter().copied()), |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    let all_owned = owned_cnt as usize == part_ids.len();

    let existing = user
        .query_row(
            "SELECT status, note, incarnon, wish, mastered FROM weapon_progress WHERE weapon_id = ?1",
            [weapon_id],
            |row| {
                Ok(WeaponProgressRow {
                    status: row.get(0)?,
                    note: row.get(1)?,
                    incarnon: row.get(2)?,
                    wish: row.get(3)?,
                    mastered: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    // 已获取不降级；全齐 → 蓝图已齐；未全齐 → 未获取
    let new_status = if existing.status == 1 {
        1
    } else if all_owned {
        2
    } else {
        0
    };

    if new_status == existing.status {
        return Ok(());
    }

    user.execute(
        "INSERT INTO weapon_progress (weapon_id, status, note, incarnon, wish, mastered, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(weapon_id) DO UPDATE SET
            status=excluded.status,
            updated_at=excluded.updated_at",
        params![
            weapon_id,
            new_status,
            existing.note,
            existing.incarnon,
            existing.wish,
            existing.mastered,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

