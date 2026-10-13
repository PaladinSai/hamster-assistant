//! 进度查询共用：ID、双库打开、遗物 drop 加载。

use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::db::{self, open_catalog, open_user};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartDropDto {
    pub relic_name: String,
    pub relic_key: String,
    pub rarity: Option<i32>,
    pub vaulted: i32,
    pub in_resurgence: bool,
}

pub(crate) fn id_to_str(id: i64) -> String {
    id.to_string()
}

pub(crate) fn parse_id(raw: &str, label: &str) -> Result<i64, String> {
    raw.parse::<i64>()
        .map_err(|_| format!("无效{label}: {raw}"))
}

pub(crate) fn open_pair(data_dir: &Path) -> Result<(Connection, Connection), String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;
    let user = open_user(&db::user_path(data_dir))?;
    Ok((catalog, user))
}

/// 未入库(0) → 重生(1) → 已入库(2)
fn relic_drop_rank(d: &PartDropDto) -> u8 {
    if d.in_resurgence {
        1
    } else if d.vaulted != 0 {
        2
    } else {
        0
    }
}

pub(crate) fn cmp_relic_drops(a: &PartDropDto, b: &PartDropDto) -> std::cmp::Ordering {
    relic_drop_rank(a)
        .cmp(&relic_drop_rank(b))
        .then_with(|| a.relic_name.cmp(&b.relic_name))
}

/// `join_table`: `warframe_part_relic` 或 `weapon_part_relic`
pub(crate) fn load_part_relic_drops(
    catalog: &Connection,
    part_id: i64,
    join_table: &str,
    resurgence_relics: &std::collections::HashSet<String>,
) -> Result<Vec<PartDropDto>, String> {
    let sql = format!(
        "SELECT v.name_zh, d.rarity, v.vaulted, v.relic_key
         FROM {join_table} d
         JOIN void_relic v ON v.id = d.relic_id
         WHERE d.part_id = ?1
         ORDER BY d.sort_order, d.id"
    );
    let mut stmt = catalog.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([part_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<i32>>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        let (name, rarity, vaulted, key) = r.map_err(|e| e.to_string())?;
        out.push(PartDropDto {
            relic_name: name,
            relic_key: key.clone(),
            rarity,
            vaulted,
            in_resurgence: resurgence_relics.contains(&key),
        });
    }
    out.sort_by(cmp_relic_drops);
    Ok(out)
}
