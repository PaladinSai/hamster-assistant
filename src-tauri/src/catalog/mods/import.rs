//! Mods.json 落库。

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, Transaction};
use serde_json::Value;

use crate::catalog::i18n::ZhLookup;
use crate::id::Snowflake;

use super::filter::{
    is_primed_mod, map_mod_kind, should_skip_wfcd_mod,
};
use super::source::{build_source_map, source_from_mod_drops};
use super::types::*;

pub fn import_mods_json(
    conn: &mut Connection,
    mods_json: &[u8],
    drops_json: &[u8],
    snowflake: &Snowflake,
) -> Result<ModImportStats, String> {
    let arr: Vec<Value> =
        serde_json::from_slice(mods_json).map_err(|e| format!("Mods.json 解析失败: {e}"))?;
    let sources = build_source_map(drops_json)?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("Mod 事务失败: {e}"))?;
    let stats = import_in_tx(&tx, arr, &sources, snowflake)?;
    tx.commit().map_err(|e| format!("Mod 提交失败: {e}"))?;
    Ok(stats)
}

fn load_mod_id_map(tx: &Transaction<'_>) -> Result<HashMap<String, i64>, String> {
    let mut stmt = tx
        .prepare("SELECT unique_name, id FROM mod")
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

fn import_in_tx(
    tx: &Transaction<'_>,
    arr: Vec<Value>,
    sources: &HashMap<String, i32>,
    snowflake: &Snowflake,
) -> Result<ModImportStats, String> {
    let mut id_map = load_mod_id_map(tx)?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut stats = ModImportStats::default();
    let mut sort: i32 = 0;
    let zh = ZhLookup::load(tx)?;

    for item in arr {
        // 先读 drops / name（仍持有 Value），再 move 反序列化，避免 item.clone()
        let from_map = item
            .get("name")
            .and_then(|v| v.as_str())
            .map(|n| sources.get(&name_key(n)).copied().unwrap_or(SRC_OTHER))
            .unwrap_or(SRC_OTHER);
        let source_kind = source_from_mod_drops(&item, from_map);
        let m: WfcdMod = match serde_json::from_value(item) {
            Ok(m) => m,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        let item_type = m.item_type.as_deref().unwrap_or("");
        if should_skip_wfcd_mod(&m) {
            stats.skipped += 1;
            continue;
        }
        let mod_kind = map_mod_kind(item_type, m.compat_name.as_deref());
        let is_prime = if m.is_prime.unwrap_or(false) || is_primed_mod(&m.name, &m.unique_name) {
            1
        } else {
            0
        };
        let is_augment = if m.is_augment.unwrap_or(false) { 1 } else { 0 };
        let name_zh = zh.get(&m.name);

        if let Some(&id) = id_map.get(&m.unique_name) {
            tx.execute(
                "UPDATE mod SET
                    name_en=?2, name_zh=?3, mod_kind=?4, source_kind=?5,
                    is_prime=?6, is_augment=?7, polarity=?8, sort_order=?9
                 WHERE id=?1",
                params![
                    id,
                    m.name,
                    name_zh,
                    mod_kind,
                    source_kind,
                    is_prime,
                    is_augment,
                    m.polarity,
                    sort
                ],
            )
            .map_err(|e| format!("更新 Mod {} 失败: {e}", m.name))?;
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO mod (
                    id, unique_name, name_en, name_zh, mod_kind, source_kind,
                    is_prime, is_augment, polarity, sort_order
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    m.unique_name,
                    m.name,
                    name_zh,
                    mod_kind,
                    source_kind,
                    is_prime,
                    is_augment,
                    m.polarity,
                    sort
                ],
            )
            .map_err(|e| format!("插入 Mod {} 失败: {e}", m.name))?;
            id_map.insert(m.unique_name.clone(), id);
        }

        seen.insert(m.unique_name.clone());
        sort += 1;
        stats.mods += 1;
    }

    let orphans: Vec<i64> = id_map
        .iter()
        .filter(|(uname, _)| !seen.contains(*uname))
        .map(|(_, id)| *id)
        .collect();
    for id in orphans {
        tx.execute("DELETE FROM mod WHERE id = ?1", [id])
            .map_err(|e| format!("删除 orphan Mod id={id} 失败: {e}"))?;
    }

    Ok(stats)
}
