//! 战甲俗称：打包 `warframe_nicknames.json` → `warframe_family.nickname`

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::Deserialize;

use crate::db;

#[derive(Debug, Deserialize)]
struct NicknameJson {
    family_key: String,
    #[allow(dead_code)]
    name_zh: String,
    nickname: String,
}

/// 从打包资源读取俗称并写回 catalog（按 family_key）。
pub fn apply_bundled_warframe_nicknames(catalog: &Connection) -> Result<usize, String> {
    let path = db::bundled_resource_path("warframe_nicknames.json");
    if !path.is_file() {
        eprintln!("[仓鼠] 未找到俗称 JSON: {}", path.display());
        return Ok(0);
    }
    let map = read_nicknames_json(&path);
    apply_nicknames_map(catalog, &map)
}

pub fn read_nicknames_json(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return map;
    };
    let Ok(rows) = serde_json::from_str::<Vec<NicknameJson>>(&raw) else {
        eprintln!("[仓鼠] 俗称 JSON 解析失败: {}", path.display());
        return map;
    };
    for row in rows {
        if !row.nickname.is_empty() {
            map.insert(row.family_key, row.nickname);
        }
    }
    map
}

pub fn apply_nicknames_map(
    catalog: &Connection,
    nicknames: &HashMap<String, String>,
) -> Result<usize, String> {
    if nicknames.is_empty() {
        return Ok(0);
    }
    let mut n = 0usize;
    for (key, nick) in nicknames {
        let updated = catalog
            .execute(
                "UPDATE warframe_family SET nickname = ?2 WHERE family_key = ?1",
                params![key, nick],
            )
            .map_err(|e| format!("写俗称 {key} 失败: {e}"))?;
        n += updated;
    }
    Ok(n)
}
