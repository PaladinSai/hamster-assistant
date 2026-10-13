//! 遗物来源搜索。

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rusqlite::{params_from_iter, Connection};

use crate::db::{self, open_catalog};

use super::locations::load_locations_for_relics;
use super::types::*;

const MAX_RELICS: usize = 120;

pub fn search_relic_sources(
    data_dir: &Path,
    q: RelicSourceQuery,
) -> Result<RelicSourceSearchResult, String> {
    db::init_databases(data_dir)?;
    let catalog = open_catalog(&db::catalog_path(data_dir))?;

    let mut relic_ids: Option<HashSet<i64>> = None;
    let mut matched_by_relic: HashMap<i64, Vec<RelicMatchedItemDto>> = HashMap::new();
    let mut text_search = false;

    if let Some(key) = q
        .relic_key
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        let id: Option<i64> = catalog
            .query_row(
                "SELECT id FROM void_relic WHERE relic_key = ?1 COLLATE NOCASE",
                [&key],
                |r| r.get(0),
            )
            .ok();
        let mut set = HashSet::new();
        if let Some(id) = id {
            set.insert(id);
        }
        relic_ids = Some(set);
    } else if let Some(raw) = q.query.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        text_search = true;
        let (ids, matched) = resolve_query_relics(&catalog, raw)?;
        matched_by_relic = matched;
        relic_ids = Some(ids);
        if matched_by_relic.is_empty() && relic_ids.as_ref().map(|s| s.is_empty()) == Some(true) {
            return Ok(RelicSourceSearchResult {
                items: Vec::new(),
                truncated: false,
                filter_options: RelicSourceFilterOptions::default(),
            });
        }
    }

    let eras = q.eras.unwrap_or_default();
    let relic_keys = q.relic_keys.unwrap_or_default();
    let game_modes = q.game_modes.unwrap_or_default();
    let rotations = q.rotations.unwrap_or_default();
    let planets = q.planets.unwrap_or_default();
    let want_bounty = game_modes.iter().any(|m| m == "bounty");
    let modes: Vec<String> = game_modes.into_iter().filter(|m| m != "bounty").collect();
    let blank_rot = rotations.iter().any(|r| r.is_empty());
    let named_rots: Vec<String> = rotations.into_iter().filter(|r| !r.is_empty()).collect();

    let searching = relic_ids.is_some();

    let mut sql = String::from(
        "SELECT DISTINCT v.id, v.relic_key, v.name_zh, v.era, v.vaulted
         FROM void_relic v",
    );
    // 没有搜索词时，筛选对不上的核桃不出现。有搜索词时列表只由搜索决定。
    if !searching {
        sql.push_str(" JOIN relic_location l ON l.relic_id = v.id");
    }
    sql.push_str(" WHERE 1=1");

    let mut binds: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref ids) = relic_ids {
        if ids.is_empty() {
            return Ok(RelicSourceSearchResult {
                items: Vec::new(),
                truncated: false,
                filter_options: RelicSourceFilterOptions::default(),
            });
        }
        let placeholders: Vec<&str> = ids.iter().map(|_| "?").collect();
        sql.push_str(" AND v.id IN (");
        sql.push_str(&placeholders.join(","));
        sql.push(')');
        for id in ids {
            binds.push(Box::new(*id));
        }
    }

    if !searching && (!eras.is_empty() || !relic_keys.is_empty()) {
        sql.push_str(" AND (");
        let mut parts = Vec::new();
        if !eras.is_empty() {
            let ph: Vec<&str> = eras.iter().map(|_| "?").collect();
            parts.push(format!("v.era IN ({})", ph.join(",")));
            for e in &eras {
                binds.push(Box::new(*e));
            }
        }
        if !relic_keys.is_empty() {
            let ph: Vec<&str> = relic_keys.iter().map(|_| "?").collect();
            parts.push(format!("v.relic_key IN ({})", ph.join(",")));
            for k in &relic_keys {
                binds.push(Box::new(k.clone()));
            }
        }
        sql.push_str(&parts.join(" OR "));
        sql.push(')');
    }

    if !searching && (!modes.is_empty() || want_bounty) {
        sql.push_str(" AND (");
        let mut parts = Vec::new();
        if !modes.is_empty() {
            let ph: Vec<&str> = modes.iter().map(|_| "?").collect();
            parts.push(format!("l.game_mode IN ({})", ph.join(",")));
            for m in &modes {
                binds.push(Box::new(m.clone()));
            }
        }
        if want_bounty {
            parts.push("l.source_kind = 'bounty'".into());
        }
        sql.push_str(&parts.join(" OR "));
        sql.push(')');
    }

    if !searching && (!named_rots.is_empty() || blank_rot) {
        sql.push_str(" AND (");
        let mut parts = Vec::new();
        if !named_rots.is_empty() {
            let ph: Vec<&str> = named_rots.iter().map(|_| "?").collect();
            parts.push(format!("l.rotation IN ({})", ph.join(",")));
            for r in &named_rots {
                binds.push(Box::new(r.clone()));
            }
        }
        if blank_rot {
            parts.push("(l.rotation IS NULL OR l.rotation = '')".into());
        }
        sql.push_str(&parts.join(" OR "));
        sql.push(')');
    }

    if !searching && !planets.is_empty() {
        let ph: Vec<&str> = planets.iter().map(|_| "?").collect();
        sql.push_str(" AND l.planet_or_hub IN (");
        sql.push_str(&ph.join(","));
        sql.push(')');
        for p in &planets {
            binds.push(Box::new(p.clone()));
        }
    }

    sql.push_str(" ORDER BY v.era, v.code LIMIT ?");
    binds.push(Box::new((MAX_RELICS + 1) as i64));

    let mut stmt = catalog.prepare(&sql).map_err(|e| e.to_string())?;
    let bind_refs: Vec<&dyn rusqlite::types::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(params_from_iter(bind_refs.iter().map(|b| *b)), |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i32>(3)?,
                r.get::<_, i32>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut relics = Vec::new();
    for row in rows {
        relics.push(row.map_err(|e| e.to_string())?);
    }
    let truncated = relics.len() > MAX_RELICS;
    if truncated {
        relics.truncate(MAX_RELICS);
    }

    let loc_ids: Vec<i64> = relics
        .iter()
        .filter(|(_, key, _, era, vaulted)| {
            if text_search && *vaulted != 0 {
                return false;
            }
            passes_relic_pick(era, key, &eras, &relic_keys)
        })
        .map(|(id, _, _, _, _)| *id)
        .collect();
    let mut locations_by_id =
        load_locations_for_relics(&catalog, &loc_ids, &modes, want_bounty, &named_rots, blank_rot, &planets)?;

    let mut items = Vec::new();
    for (id, relic_key, name_zh, era, vaulted) in relics {
        if text_search && vaulted != 0 {
            continue;
        }
        let locations = locations_by_id.remove(&id).unwrap_or_default();
        let matched = matched_by_relic.remove(&id).unwrap_or_default();
        items.push(RelicSourceItemDto {
            relic_key,
            name_zh,
            era,
            vaulted,
            matched_items: matched,
            locations,
        });
    }

    Ok(RelicSourceSearchResult {
        items,
        truncated,
        // 筛选项由前端单独拉一次，避免每次搜索重建大芯片列表
        filter_options: RelicSourceFilterOptions::default(),
    })
}

pub(crate) fn passes_relic_pick(era: &i32, relic_key: &str, eras: &[i32], relic_keys: &[String]) -> bool {
    if eras.is_empty() && relic_keys.is_empty() {
        return true;
    }
    eras.contains(era) || relic_keys.iter().any(|k| k == relic_key)
}

pub(crate) fn resolve_query_relics(
    catalog: &Connection,
    q: &str,
) -> Result<(HashSet<i64>, HashMap<i64, Vec<RelicMatchedItemDto>>), String> {
    let like = format!("%{q}%");
    let mut ids = HashSet::new();
    let mut matched: HashMap<i64, Vec<RelicMatchedItemDto>> = HashMap::new();

    // 直接匹配核桃键 / 中文名
    {
        let mut stmt = catalog
            .prepare(
                "SELECT id FROM void_relic
                 WHERE relic_key LIKE ?1 COLLATE NOCASE OR name_zh LIKE ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&like], |r| r.get::<_, i64>(0))
            .map_err(|e| e.to_string())?;
        for r in rows {
            ids.insert(r.map_err(|e| e.to_string())?);
        }
    }

    // 奖励英文 / 落库简中（导入时由 name_i18n 前缀解析写入）
    {
        let mut stmt = catalog
            .prepare(
                "SELECT DISTINCT relic_id, item_name_en, rarity, item_name_zh
                 FROM relic_reward
                 WHERE item_name_en LIKE ?1 COLLATE NOCASE
                    OR IFNULL(item_name_zh, '') LIKE ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([&like], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<i32>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (relic_id, en, rarity, zh) = r.map_err(|e| e.to_string())?;
            ids.insert(relic_id);
            let list = matched.entry(relic_id).or_default();
            if list.len() < 8
                && !list.iter().any(|m| m.item_name_en == en)
            {
                list.push(RelicMatchedItemDto {
                    item_name_en: en,
                    item_name_zh: zh,
                    rarity,
                });
            }
        }
    }

    Ok((ids, matched))
}
