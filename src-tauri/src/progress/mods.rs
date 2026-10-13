//! Mod 进度与标签。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::catalog::{SRC_KEY, SRC_NIGHTMARE, SRC_SYNDICATE};
use crate::db::{self, open_user};
use crate::user_migrate::{
    MOD_TAG_CORRUPTED, MOD_TAG_NIGHTMARE, MOD_TAG_PRIME, MOD_TAG_SYNDICATE,
};
use super::common::{id_to_str, open_pair, parse_id};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModTagDto {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModListItem {
    pub mod_id: String,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub mod_kind: i32,
    pub source_kind: i32,
    pub is_prime: i32,
    pub is_augment: i32,
    pub polarity: Option<String>,
    pub status: i32,
    pub wish: i32,
    pub note: Option<String>,
    pub tag_id: Option<String>,
    pub tag_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertModProgress {
    pub mod_id: String,
    pub status: Option<i32>,
    pub wish: Option<i32>,
    pub note: Option<String>,
    /// 缺省=不改；`Some(None)`=清空；`Some(Some(id))`=设置（均记 tag_manual=1）
    #[serde(default)]
    pub tag_id: Option<Option<String>>,
}

#[derive(Clone, Default)]
struct ModProgressRow {
    status: i32,
    wish: i32,
    note: Option<String>,
    tag_id: Option<i64>,
    tag_manual: i32,
}

fn suggested_mod_tag(source_kind: i32, is_prime: i32) -> Option<i64> {
    if source_kind == SRC_KEY {
        return Some(MOD_TAG_CORRUPTED);
    }
    if source_kind == SRC_NIGHTMARE {
        return Some(MOD_TAG_NIGHTMARE);
    }
    if source_kind == SRC_SYNDICATE {
        return Some(MOD_TAG_SYNDICATE);
    }
    if is_prime != 0 {
        return Some(MOD_TAG_PRIME);
    }
    None
}

/// 对 `tag_manual=0` 的 Mod 按 catalog 规则自动挂签（不覆盖用户手改）。
pub fn apply_auto_mod_tags(data_dir: &Path) -> Result<usize, String> {
    let (catalog, user) = open_pair(data_dir)?;
    apply_auto_mod_tags_conn(&catalog, &user)
}

fn apply_auto_mod_tags_conn(catalog: &Connection, user: &Connection) -> Result<usize, String> {
    let has_mod: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mod'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if has_mod == 0 {
        return Ok(0);
    }

    let mut progress: HashMap<i64, (Option<i64>, i32)> = HashMap::new();
    {
        let mut stmt = user
            .prepare("SELECT mod_id, tag_id, tag_manual FROM mod_progress")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (id, tag_id, tag_manual) = r.map_err(|e| e.to_string())?;
            progress.insert(id, (tag_id, tag_manual));
        }
    }

    let mut stmt = catalog
        .prepare("SELECT id, source_kind, is_prime FROM mod")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    let mut updated = 0usize;
    for r in rows {
        let (mod_id, source_kind, is_prime) = r.map_err(|e| e.to_string())?;
        let (cur_tag, tag_manual) = progress.get(&mod_id).copied().unwrap_or((None, 0));
        if tag_manual != 0 {
            continue;
        }
        let suggested = suggested_mod_tag(source_kind, is_prime);
        if cur_tag == suggested {
            continue;
        }
        user.execute(
            "INSERT INTO mod_progress (mod_id, status, wish, note, tag_id, tag_manual, updated_at)
             VALUES (?1, 0, 0, NULL, ?2, 0, ?3)
             ON CONFLICT(mod_id) DO UPDATE SET
                tag_id=excluded.tag_id,
                updated_at=excluded.updated_at
             WHERE mod_progress.tag_manual = 0",
            params![mod_id, suggested, now],
        )
        .map_err(|e| e.to_string())?;
        updated += 1;
    }
    Ok(updated)
}

pub fn list_mod_tags(data_dir: &Path) -> Result<Vec<ModTagDto>, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let mut stmt = user
        .prepare(
            "SELECT id, name, sort_order, is_builtin FROM mod_tag
             ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ModTagDto {
                id: id_to_str(row.get::<_, i64>(0)?),
                name: row.get(1)?,
                sort_order: row.get(2)?,
                is_builtin: row.get::<_, i32>(3)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn create_mod_tag(data_dir: &Path, name: &str) -> Result<ModTagDto, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("标签名不能为空".into());
    }
    if name.len() > 32 {
        return Err("标签名过长".into());
    }
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let next_order: i32 = user
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM mod_tag",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    user.execute(
        "INSERT INTO mod_tag (name, sort_order, is_builtin) VALUES (?1, ?2, 0)",
        params![name, next_order],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            format!("标签「{name}」已存在")
        } else {
            e.to_string()
        }
    })?;
    let id: i64 = user
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(ModTagDto {
        id: id_to_str(id),
        name: name.to_string(),
        sort_order: next_order,
        is_builtin: false,
    })
}

pub fn delete_mod_tag(data_dir: &Path, id: &str) -> Result<(), String> {
    let tag_id = parse_id(id, "标签 id")?;
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;

    let is_builtin: Option<i32> = user
        .query_row(
            "SELECT is_builtin FROM mod_tag WHERE id = ?1",
            [tag_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;

    match is_builtin {
        None => return Err("标签不存在".into()),
        Some(1) => return Err("内置标签不可删除".into()),
        Some(_) => {}
    }

    user.execute(
        "UPDATE mod_progress SET tag_id = NULL WHERE tag_id = ?1",
        [tag_id],
    )
    .map_err(|e| e.to_string())?;

    let n = user
        .execute(
            "DELETE FROM mod_tag WHERE id = ?1 AND is_builtin = 0",
            [tag_id],
        )
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("标签不存在或不可删除".into());
    }
    Ok(())
}

pub fn list_mods(data_dir: &Path) -> Result<Vec<ModListItem>, String> {
    let (catalog, user) = open_pair(data_dir)?;
    // 自动打标只在 kick_update_mods 后执行，列表热路径只读。

    let mut tag_names: HashMap<i64, String> = HashMap::new();
    {
        let mut stmt = user
            .prepare("SELECT id, name FROM mod_tag")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (id, name) = r.map_err(|e| e.to_string())?;
            tag_names.insert(id, name);
        }
    }

    let mut progress: HashMap<i64, ModProgressRow> = HashMap::new();
    {
        let mut stmt = user
            .prepare("SELECT mod_id, status, wish, note, tag_id, tag_manual FROM mod_progress")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ModProgressRow {
                        status: row.get(1)?,
                        wish: row.get(2)?,
                        note: row.get(3)?,
                        tag_id: row.get(4)?,
                        tag_manual: row.get(5)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        for r in rows {
            let (id, row) = r.map_err(|e| e.to_string())?;
            progress.insert(id, row);
        }
    }

    let mut stmt = catalog
        .prepare(
            "SELECT m.id, m.name_en, m.name_zh, m.mod_kind, m.source_kind,
                    m.is_prime, m.is_augment, m.polarity, m.sort_order
             FROM mod m
             ORDER BY m.sort_order, m.name_en",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, Option<String>>(7)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows {
        let (id, name_en, name_zh, mod_kind, source_kind, is_prime, is_augment, polarity) =
            r.map_err(|e| e.to_string())?;
        let pf = progress.get(&id).cloned().unwrap_or_default();
        let tag_name = pf.tag_id.and_then(|tid| tag_names.get(&tid).cloned());
        out.push(ModListItem {
            mod_id: id_to_str(id),
            name_en,
            name_zh,
            mod_kind,
            source_kind,
            is_prime,
            is_augment,
            polarity,
            status: pf.status,
            wish: pf.wish,
            note: pf.note,
            tag_id: pf.tag_id.map(id_to_str),
            tag_name,
        });
    }
    Ok(out)
}

pub fn upsert_mod_progress(data_dir: &Path, patch: UpsertModProgress) -> Result<(), String> {
    let mod_id = parse_id(&patch.mod_id, "Mod id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let exists: i64 = catalog
        .query_row("SELECT 1 FROM mod WHERE id = ?1", [mod_id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    if exists == 0 {
        return Err(format!("Mod id={mod_id} 不存在"));
    }

    let existing = user
        .query_row(
            "SELECT status, wish, note, tag_id, tag_manual FROM mod_progress WHERE mod_id = ?1",
            [mod_id],
            |row| {
                Ok(ModProgressRow {
                    status: row.get(0)?,
                    wish: row.get(1)?,
                    note: row.get(2)?,
                    tag_id: row.get(3)?,
                    tag_manual: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();

    let status = patch.status.unwrap_or(existing.status);
    let status = if status != 0 { 1 } else { 0 };
    let wish = patch
        .wish
        .map(|w| if w != 0 { 1 } else { 0 })
        .unwrap_or(existing.wish);
    let note = patch.note.clone().or(existing.note);

    let (tag_id, tag_manual) = if let Some(tag_patch) = &patch.tag_id {
        let tid = match tag_patch {
            None => None,
            Some(s) => {
                let id = parse_id(s, "标签 id")?;
                let ok: i64 = user
                    .query_row("SELECT 1 FROM mod_tag WHERE id = ?1", [id], |r| r.get(0))
                    .optional()
                    .map_err(|e| e.to_string())?
                    .unwrap_or(0);
                if ok == 0 {
                    return Err(format!("标签 id={id} 不存在"));
                }
                Some(id)
            }
        };
        (tid, 1)
    } else {
        (existing.tag_id, existing.tag_manual)
    };

    user.execute(
        "INSERT INTO mod_progress (mod_id, status, wish, note, tag_id, tag_manual, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(mod_id) DO UPDATE SET
            status=excluded.status,
            wish=excluded.wish,
            note=excluded.note,
            tag_id=excluded.tag_id,
            tag_manual=excluded.tag_manual,
            updated_at=excluded.updated_at",
        params![mod_id, status, wish, note, tag_id, tag_manual, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

