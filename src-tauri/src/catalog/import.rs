//! 机体（战甲 / Archwing / 机甲 / 同伴）catalog 入库事务。
//!
//! - 上游 fetch / 轻量 parse / suit 家族规则：[`crate::catalog::wfcd`]
//! - 武器入库：[`crate::catalog::weapons`]（本模块不处理武器）
//! - 自制同伴精通部件（moa/hound）概念见 [`crate::catalog::modular`]；武器侧对称见 `import_modular_weapons_json`

use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{params, Connection, Transaction};

use crate::catalog::i18n::ZhLookup;
use crate::catalog::modular;
use crate::catalog::wfcd::{
    companion_type_for, family_and_variant, modular_companion_type, modular_companion_zh_fallback,
    parse_archwings, parse_modular_companions, parse_pets, parse_sentinels, parse_warframes,
    part_meta, suit_kind_for, vault_status_code, WfcdWarframe, SUIT_KIND_COMPANION, VARIANT_BASE,
    VARIANT_PRIME,
};
use crate::id::Snowflake;

#[derive(Debug, Default, Clone)]
pub struct ImportStats {
    pub families: usize,
    pub warframes: usize,
    pub parts: usize,
}

fn with_catalog_tx<T>(
    conn: &mut Connection,
    open_err: &str,
    commit_err: &str,
    f: impl FnOnce(&Transaction<'_>) -> Result<T, String>,
) -> Result<T, String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("{open_err}: {e}"))?;
    let out = f(&tx)?;
    tx.commit().map_err(|e| format!("{commit_err}: {e}"))?;
    Ok(out)
}

/// 将 WFCD Warframes.json 字节导入 catalog 连接（事务）。
pub fn import_warframes_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let items = parse_warframes(json)?;
    with_catalog_tx(conn, "开启事务失败", "提交事务失败", |tx| {
        import_in_tx(tx, &items, snowflake)
    })
}

/// 将 WFCD Archwing.json 导入（与战甲共用表，suit_kind=archwing）。
pub fn import_archwing_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let items = parse_archwings(json)?;
    with_catalog_tx(conn, "开启事务失败", "提交事务失败", |tx| {
        import_in_tx(tx, &items, snowflake)
    })
}

/// 将 WFCD Sentinels.json 导入（suit_kind=companion, companion_type=sentinel）。
pub fn import_sentinels_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let items = parse_sentinels(json)?;
    with_catalog_tx(conn, "开启事务失败", "提交事务失败", |tx| {
        import_in_tx(tx, &items, snowflake)
    })
}

/// 将 WFCD Pets.json 导入（库娃 / 库珀 / 狐獴 / 扑猎达赛）。
pub fn import_pets_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let items = parse_pets(json)?;
    with_catalog_tx(conn, "开启事务失败", "提交事务失败", |tx| {
        import_in_tx(tx, &items, snowflake)
    })
}

/// Pets.json → 自制恐鸟 / 猎犬型号（精通部件，`companion_type`=moa\|hound）。
/// 与武器侧 [`crate::catalog::weapons::import_modular_weapons_json`] 对称，见 [`crate::catalog::modular`]。
pub fn import_modular_companions_json(
    conn: &mut Connection,
    json: &[u8],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let items = parse_modular_companions(json)?;
    with_catalog_tx(conn, "自制同伴事务失败", "自制同伴提交失败", |tx| {
        import_modular_companions_in_tx(tx, &items, snowflake)
    })
}

fn resolve_modular_companion_zh(zh: &ZhLookup, name_en: &str) -> Option<String> {
    // 同伴 userdict 键形与英文展示名不完全一致，先走别名再共用 resolve_name_zh
    zh.get(name_en)
        .or_else(|| {
            // userdict：拜拉猎犬等键为 「Bhaira Model」
            name_en
                .strip_suffix(" Hound")
                .and_then(|base| zh.get(&format!("{base} Model")))
        })
        .or_else(|| {
            // userdict：恐鸟型号常只有 「Lambeo」
            name_en
                .strip_suffix(" Moa")
                .and_then(|base| zh.get(base).map(|z| format!("{z}恐鸟")))
        })
        .or_else(|| modular::resolve_name_zh(zh, name_en, modular_companion_zh_fallback))
}

fn import_modular_companions_in_tx(
    tx: &Transaction<'_>,
    items: &[WfcdWarframe],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let _ = tx.execute_batch("PRAGMA foreign_keys = ON;");
    let mut family_ids: HashMap<String, i64> = load_family_map(tx)?;
    let mut warframe_ids: HashMap<String, i64> = load_warframe_map(tx)?;
    let now = Utc::now().to_rfc3339();
    let mut stats = ImportStats::default();
    let zh = ZhLookup::load(tx)?;

    for wf in items {
        let Some(companion_ty) = modular_companion_type(&wf.unique_name) else {
            continue;
        };
        let family_key = wf.name.clone();
        let name_zh = resolve_modular_companion_zh(&zh, &wf.name);

        let family_id = if let Some(&id) = family_ids.get(&family_key) {
            tx.execute(
                "UPDATE warframe_family
                 SET name_en = ?2, name_zh = COALESCE(?3, name_zh),
                     suit_kind = ?4, companion_type = ?5, updated_at = ?6
                 WHERE id = ?1",
                params![
                    id,
                    wf.name,
                    name_zh,
                    SUIT_KIND_COMPANION,
                    companion_ty,
                    now
                ],
            )
            .map_err(|e| format!("更新自制同伴家族 {} 失败: {e}", wf.name))?;
            id
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO warframe_family
                 (id, family_key, name_en, name_zh, nickname, vault_status, suit_kind, companion_type, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?6, ?7, ?7)",
                params![
                    id,
                    family_key,
                    wf.name,
                    name_zh,
                    SUIT_KIND_COMPANION,
                    companion_ty,
                    now
                ],
            )
            .map_err(|e| format!("插入自制同伴家族 {} 失败: {e}", wf.name))?;
            family_ids.insert(family_key, id);
            stats.families += 1;
            id
        };

        if let Some(&wid) = warframe_ids.get(&wf.unique_name) {
            tx.execute(
                "UPDATE warframe SET family_id=?2, variant=?3, name_en=?4, name_zh=COALESCE(?5, name_zh),
                 unique_name=?6, vault_status=0, sort_order=0 WHERE id=?1",
                params![wid, family_id, VARIANT_BASE, wf.name, name_zh, wf.unique_name],
            )
            .map_err(|e| format!("更新自制同伴 {} 失败: {e}", wf.name))?;
        } else {
            let wid = snowflake.next_id();
            tx.execute(
                "INSERT INTO warframe
                 (id, family_id, variant, name_en, name_zh, unique_name, vault_status, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)",
                params![
                    wid,
                    family_id,
                    VARIANT_BASE,
                    wf.name,
                    name_zh,
                    wf.unique_name
                ],
            )
            .map_err(|e| format!("插入自制同伴 {} 失败: {e}", wf.name))?;
            warframe_ids.insert(wf.unique_name.clone(), wid);
            stats.warframes += 1;
        }
    }
    Ok(stats)
}

struct SuitImportRow<'a> {
    wf: &'a WfcdWarframe,
    family_key: String,
    variant: i32,
    kind: String,
    companion_ty: Option<String>,
}

fn import_in_tx(
    tx: &Transaction<'_>,
    items: &[WfcdWarframe],
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    let _ = tx.execute_batch("PRAGMA foreign_keys = ON;");

    let mut family_ids: HashMap<String, i64> = load_family_map(tx)?;
    let mut warframe_ids: HashMap<String, i64> = load_warframe_map(tx)?;
    let mut part_ids: HashMap<String, i64> = load_part_map(tx)?;

    let now = Utc::now().to_rfc3339();
    let mut stats = ImportStats::default();
    let zh = ZhLookup::load(tx)?;

    // 预计算家族键 / 变体 / suit_kind，避免三遍扫描重复解析
    let rows: Vec<SuitImportRow<'_>> = items
        .iter()
        .map(|wf| {
            let (family_key, variant) = family_and_variant(&wf.name);
            let kind = suit_kind_for(&wf.unique_name, wf.item_type.as_deref()).to_string();
            let companion_ty = companion_type_for(
                &wf.unique_name,
                Some(&wf.name),
                wf.item_type.as_deref().unwrap_or(""),
            )
            .map(|s| s.to_string());
            SuitImportRow {
                wf,
                family_key,
                variant,
                kind,
                companion_ty,
            }
        })
        .collect();

    for row in &rows {
        if family_ids.contains_key(&row.family_key) {
            continue;
        }
        let id = snowflake.next_id();
        let name_zh = zh.get(&row.family_key);
        tx.execute(
            "INSERT INTO warframe_family (id, family_key, name_en, name_zh, nickname, vault_status, suit_kind, companion_type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, 0, ?5, ?6, ?7, ?7)",
            params![
                id,
                row.family_key,
                row.family_key,
                name_zh,
                row.kind,
                row.companion_ty,
                now
            ],
        )
        .map_err(|e| format!("插入家族 {} 失败: {e}", row.family_key))?;
        family_ids.insert(row.family_key.clone(), id);
        stats.families += 1;
    }

    // 更新已有家族 updated_at / name_en / name_zh / suit_kind / companion_type
    for row in &rows {
        let Some(id) = family_ids.get(&row.family_key) else {
            continue;
        };
        let name_zh = zh.get(&row.family_key);
        tx.execute(
            "UPDATE warframe_family SET name_en = ?2, name_zh = COALESCE(?3, name_zh), suit_kind = ?4,
             companion_type = COALESCE(?5, companion_type), updated_at = ?6 WHERE id = ?1",
            params![
                id,
                row.family_key,
                name_zh,
                row.kind,
                row.companion_ty,
                now
            ],
        )
        .map_err(|e| format!("更新家族 {} 失败: {e}", row.family_key))?;
    }

    for row in &rows {
        let family_id = *family_ids
            .get(&row.family_key)
            .ok_or_else(|| format!("缺少家族 {}", row.family_key))?;

        let is_prime = row.variant == VARIANT_PRIME;
        let vault = vault_status_code(is_prime, row.wf.vaulted);

        let wf_zh = zh
            .get(&row.wf.name)
            .or_else(|| zh.get(&row.family_key));
        let warframe_id = if let Some(id) = warframe_ids.get(&row.wf.unique_name) {
            tx.execute(
                "UPDATE warframe SET family_id=?2, variant=?3, name_en=?4, name_zh=COALESCE(?5, name_zh), unique_name=?6, vault_status=?7, sort_order=?3
                 WHERE id=?1",
                params![
                    id,
                    family_id,
                    row.variant,
                    row.wf.name,
                    wf_zh,
                    row.wf.unique_name,
                    vault
                ],
            )
            .map_err(|e| format!("更新变体 {} 失败: {e}", row.wf.name))?;
            *id
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO warframe (id, family_id, variant, name_en, name_zh, unique_name, vault_status, sort_order)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?3)",
                params![
                    id,
                    family_id,
                    row.variant,
                    row.wf.name,
                    wf_zh,
                    row.wf.unique_name,
                    vault
                ],
            )
            .map_err(|e| format!("插入变体 {} 失败: {e}", row.wf.name))?;
            warframe_ids.insert(row.wf.unique_name.clone(), id);
            stats.warframes += 1;
            id
        };

        for comp in &row.wf.components {
            let Some(meta) = part_meta(&comp.name) else {
                continue;
            };
            let pkey = meta.key;
            let name_zh = meta.name_zh;
            let sort = meta.sort;
            // 对账键：战甲 uniqueName + part_key
            let part_map_key = format!("{}#{}", row.wf.unique_name, pkey);

            if let Some(pid) = part_ids.get(&part_map_key) {
                tx.execute(
                    "UPDATE warframe_part SET warframe_id=?2, part_key=?3, name_zh=?4, sort_order=?5 WHERE id=?1",
                    params![pid, warframe_id, pkey, name_zh, sort],
                )
                .map_err(|e| format!("更新部件 {part_map_key} 失败: {e}"))?;
            } else {
                let pid = snowflake.next_id();
                tx.execute(
                    "INSERT INTO warframe_part (id, warframe_id, part_key, name_zh, sort_order)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![pid, warframe_id, pkey, name_zh, sort],
                )
                .map_err(|e| format!("插入部件 {part_map_key} 失败: {e}"))?;
                part_ids.insert(part_map_key, pid);
                stats.parts += 1;
            }
        }
    }

    purge_excluded_and_orphan_families(tx)?;
    sync_family_vault_status(tx)?;
    Ok(stats)
}

fn sync_family_vault_status(tx: &Transaction<'_>) -> Result<(), String> {
    tx.execute(
        "UPDATE warframe_family
         SET vault_status = COALESCE(
           (SELECT w.vault_status FROM warframe w
            WHERE w.family_id = warframe_family.id AND w.variant = 1),
           0
         )",
        [],
    )
    .map_err(|e| format!("回写家族入库状态失败: {e}"))?;
    Ok(())
}

/// 清掉 Helminth / 空壳 Orion；机甲与 Archwing 保留。
fn purge_excluded_and_orphan_families(tx: &Transaction<'_>) -> Result<(), String> {
    // 避免「DELETE … WHERE id IN (SELECT id FROM 同表)」在 SQLite 上踩坑
    tx.execute_batch(
        "
        DELETE FROM warframe_part_relic WHERE part_id IN (
          SELECT p.id FROM warframe_part p
          JOIN warframe w ON w.id = p.warframe_id
          WHERE w.unique_name LIKE '%/PowersuitAbilities/Helminth%'
             OR IFNULL(w.name_en, '') = 'Helminth'
        );
        DELETE FROM warframe_part WHERE warframe_id IN (
          SELECT id FROM warframe
          WHERE unique_name LIKE '%/PowersuitAbilities/Helminth%'
             OR IFNULL(name_en, '') = 'Helminth'
        );
        DELETE FROM warframe
          WHERE unique_name LIKE '%/PowersuitAbilities/Helminth%'
             OR IFNULL(name_en, '') = 'Helminth';

        DELETE FROM warframe_part_relic WHERE part_id IN (
          SELECT p.id FROM warframe_part p
          JOIN warframe w ON w.id = p.warframe_id
          JOIN warframe_family f ON f.id = w.family_id
          WHERE f.family_key IN ('Helminth', 'Orion & Sirius')
        );
        DELETE FROM warframe_part WHERE warframe_id IN (
          SELECT w.id FROM warframe w
          JOIN warframe_family f ON f.id = w.family_id
          WHERE f.family_key IN ('Helminth', 'Orion & Sirius')
        );
        DELETE FROM warframe WHERE family_id IN (
          SELECT id FROM warframe_family
          WHERE family_key IN ('Helminth', 'Orion & Sirius')
        );
        DELETE FROM warframe_family
          WHERE family_key IN ('Helminth', 'Orion & Sirius');
        ",
    )
    .map_err(|e| format!("清理非战甲失败: {e}"))?;
    Ok(())
}

fn load_family_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare("SELECT family_key, id FROM warframe_family")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

fn load_warframe_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare("SELECT unique_name, id FROM warframe WHERE unique_name IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

fn load_part_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    // 部件表没有 unique_name 列；用 warframe.unique_name + part_key 合成对账需要额外存储。
    // P1：通过 join 重建「合成键」仅用于同一次导入的 upsert；跨次导入靠 warframe_id+part_key UNIQUE。
    let mut stmt = tx
        .prepare(
            "SELECT w.unique_name, p.part_key, p.id
             FROM warframe_part p
             JOIN warframe w ON w.id = p.warframe_id
             WHERE w.unique_name IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let wu: String = row.get(0)?;
            let pk: String = row.get(1)?;
            let id: i64 = row.get(2)?;
            Ok((format!("{wu}#{pk}"), id))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (k, v) = r.map_err(|e| e.to_string())?;
        map.insert(k, v);
    }
    Ok(map)
}

/// 统计 catalog 内战甲家族数。
pub fn count_families(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM warframe_family", [], |row| row.get(0))
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_import_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn import_sets_vault_and_keeps_nickname() {
        let dir = temp_dir("hint");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let json = br#"[
          {
            "name": "Ash",
            "uniqueName": "/Lotus/Powersuits/Ninja/Ninja",
            "category": "Warframes",
            "type": "Warframe",
            "isPrime": false,
            "components": [
              {
                "name": "Blueprint",
                "drops": [
                  { "location": "Mercury/Apollodorus (Survival)" },
                  { "location": "Market" }
                ]
              },
              { "name": "Neuroptics", "drops": [] }
            ]
          },
          {
            "name": "Ash Prime",
            "uniqueName": "/Lotus/Powersuits/Ninja/AshPrime",
            "category": "Warframes",
            "type": "Warframe",
            "isPrime": true,
            "components": [
              {
                "name": "Blueprint",
                "drops": [{ "location": "Axi A1 Relic" }]
              }
            ]
          }
        ]"#;

        import_warframes_json(&mut catalog, json, &snow).unwrap();

        let base_vault: i32 = catalog
            .query_row(
                "SELECT vault_status FROM warframe WHERE variant = 0",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(base_vault, 0);

        let prime_vault: i32 = catalog
            .query_row(
                "SELECT vault_status FROM warframe WHERE variant = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(prime_vault, 1);

        let family_vault: i32 = catalog
            .query_row(
                "SELECT vault_status FROM warframe_family WHERE family_key = 'Ash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(family_vault, 1);

        catalog
            .execute(
                "UPDATE warframe_family SET nickname = '阿屎' WHERE family_key = 'Ash'",
                [],
            )
            .unwrap();
        import_warframes_json(&mut catalog, json, &snow).unwrap();
        let nick: String = catalog
            .query_row(
                "SELECT nickname FROM warframe_family WHERE family_key = 'Ash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(nick, "阿屎");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn gemini_one_body_and_equinox_aspects() {
        let dir = temp_dir("gemini_eq");
        db::init_databases_ex(&dir, false).unwrap();
        let mut catalog = db::open_catalog(&db::catalog_path(&dir)).unwrap();
        let snow = Snowflake::new(1);
        let json = br#"[
          {
            "name": "Orion & Sirius",
            "uniqueName": "/Lotus/Powersuits/SiriusOrion/OrionSuit",
            "category": "Warframes",
            "type": "Warframe",
            "components": []
          },
          {
            "name": "Sirius & Orion",
            "uniqueName": "/Lotus/Powersuits/SiriusOrion/SiriusSuit",
            "category": "Warframes",
            "type": "Warframe",
            "components": [
              { "name": "Blueprint" },
              { "name": "Chassis" },
              { "name": "Systems" },
              { "name": "Neuroptics" }
            ]
          },
          {
            "name": "Equinox",
            "uniqueName": "/Lotus/Powersuits/YinYang/YinYang",
            "category": "Warframes",
            "type": "Warframe",
            "components": [
              { "name": "Blueprint" },
              { "name": "Day Aspect Blueprint" },
              { "name": "Day Aspect Systems" },
              { "name": "Day Aspect Chassis" },
              { "name": "Day Aspect Neuroptics" },
              { "name": "Night Aspect Blueprint" },
              { "name": "Night Aspect Systems" },
              { "name": "Night Aspect Chassis" },
              { "name": "Night Aspect Neuroptics" }
            ]
          },
          {
            "name": "Equinox Prime",
            "uniqueName": "/Lotus/Powersuits/YinYang/EquinoxPrime",
            "category": "Warframes",
            "type": "Warframe",
            "isPrime": true,
            "components": [
              { "name": "Blueprint" },
              { "name": "Neuroptics" },
              { "name": "Chassis" },
              { "name": "Systems" }
            ]
          }
        ]"#;

        import_warframes_json(&mut catalog, json, &snow).unwrap();

        let gemini: (String, i32, i64) = catalog
            .query_row(
                "SELECT f.family_key, w.variant, COUNT(p.id)
                 FROM warframe w
                 JOIN warframe_family f ON f.id = w.family_id
                 LEFT JOIN warframe_part p ON p.warframe_id = w.id
                 WHERE f.family_key LIKE '%Orion%' OR f.family_key LIKE '%Sirius%'
                 GROUP BY f.family_key, w.variant",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(gemini.0, "Sirius & Orion");
        assert_eq!(gemini.1, 0);
        assert_eq!(gemini.2, 4);

        let names: Vec<String> = catalog
            .prepare(
                "SELECT p.name_zh FROM warframe_part p
                 JOIN warframe w ON w.id = p.warframe_id
                 JOIN warframe_family f ON f.id = w.family_id
                 WHERE f.family_key = 'Equinox' AND w.variant = 0
                 ORDER BY p.sort_order",
            )
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            names,
            vec![
                "总蓝图",
                "白昼面蓝图",
                "白昼面系统",
                "白昼面机体",
                "白昼面头",
                "黑夜面蓝图",
                "黑夜面系统",
                "黑夜面机体",
                "黑夜面头",
            ]
        );

        let prime_n: i64 = catalog
            .query_row(
                "SELECT COUNT(*) FROM warframe_part p
                 JOIN warframe w ON w.id = p.warframe_id
                 JOIN warframe_family f ON f.id = w.family_id
                 WHERE f.family_key = 'Equinox' AND w.variant = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(prime_n, 4);

        let _ = fs::remove_dir_all(&dir);
    }
}
