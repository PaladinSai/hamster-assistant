//! 战甲进度：按变体（warframe_id）一行；catalog + user 合并。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::progress::common::{id_to_str, load_part_relic_drops, open_pair, parse_id, PartDropDto};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarframeListItem {
    pub warframe_id: String,
    pub family_id: String,
    pub family_key: String,
    /// 0 本体 / 1 Prime / 2 Umbra
    pub variant: i32,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub nickname: Option<String>,
    pub status: i32,
    pub status_display: String,
    pub helminth: i32,
    pub forma_done: i32,
    pub wish: i32,
    pub note: Option<String>,
    /// catalog 只读；仅 Prime 有意义
    pub vault_status: Option<i32>,
    pub mastered: i32,
    /// warframe | archwing | necramech | companion
    pub suit_kind: String,
    /// 同伴细分：sentinel | kubrow | kavat | vulpaphyla | predasite
    pub companion_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartDto {
    pub part_id: String,
    pub part_key: String,
    pub name_zh: String,
    pub sort_order: i32,
    pub status: i32,
    pub note: Option<String>,
    pub drops: Vec<PartDropDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WarframeDetail {
    pub warframe: WarframeListItem,
    pub parts: Vec<PartDto>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertWarframeProgress {
    pub warframe_id: String,
    pub status: Option<i32>,
    pub helminth: Option<i32>,
    pub forma_done: Option<i32>,
    pub wish: Option<i32>,
    pub note: Option<String>,
    pub mastered: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertPartProgress {
    pub part_id: String,
    pub status: Option<i32>,
    pub note: Option<String>,
}

#[derive(Clone, Default)]
struct ProgressRow {
    status: i32,
    forma_done: i32,
    note: Option<String>,
    wish: i32,
    mastered: i32,
    helminth: i32,
}

pub fn list_warframes(
    data_dir: &Path,
    suit_kind: Option<&str>,
) -> Result<Vec<WarframeListItem>, String> {
    let (catalog, user) = open_pair(data_dir)?;
    let progress = load_all_progress(&user)?;
    // 旧前端可能传 sentinel；统一为 companion
    let kind = match suit_kind.unwrap_or("warframe") {
        "sentinel" => "companion",
        other => other,
    };

    let mut stmt = catalog
        .prepare(
            "SELECT w.id, w.family_id, f.family_key, w.variant, w.name_en,
                    COALESCE(w.name_zh, f.name_zh), f.nickname, w.vault_status,
                    IFNULL(f.suit_kind, 'warframe'), f.companion_type
             FROM warframe w
             JOIN warframe_family f ON f.id = w.family_id
             WHERE IFNULL(f.suit_kind, 'warframe') = ?1
             ORDER BY f.companion_type, f.name_en COLLATE NOCASE, w.variant, w.id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([kind], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, i32>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for r in rows {
        let (wid, fid, key, variant, name_en, name_zh, nickname, vault, suit_kind, companion_type) =
            r.map_err(|e| e.to_string())?;
        let pf = progress.get(&wid).cloned().unwrap_or_default();
        out.push(merge_item(
            wid,
            fid,
            key,
            variant,
            name_en,
            name_zh,
            nickname,
            vault,
            suit_kind,
            companion_type,
            &pf,
        ));
    }
    Ok(out)
}

/// 兼容旧命令名
pub fn list_warframe_families(data_dir: &Path) -> Result<Vec<WarframeListItem>, String> {
    list_warframes(data_dir, Some("warframe"))
}

pub fn get_warframe_detail(
    data_dir: &Path,
    warframe_id_raw: &str,
) -> Result<WarframeDetail, String> {
    let warframe_id = parse_id(warframe_id_raw, "战甲 id")?;
    let (catalog, user) = open_pair(data_dir)?;

    let (fid, key, variant, name_en, name_zh, nickname, vault, suit_kind, companion_type): (
        i64,
        String,
        i32,
        String,
        Option<String>,
        Option<String>,
        i32,
        String,
        Option<String>,
    ) = catalog
        .query_row(
            "SELECT w.family_id, f.family_key, w.variant, w.name_en,
                    COALESCE(w.name_zh, f.name_zh), f.nickname, w.vault_status,
                    IFNULL(f.suit_kind, 'warframe'), f.companion_type
             FROM warframe w
             JOIN warframe_family f ON f.id = w.family_id
             WHERE w.id = ?1",
            [warframe_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("未找到战甲 id={warframe_id_raw}"))?;

    let pf = load_one_progress(&user, warframe_id)?.unwrap_or_default();
    let warframe = merge_item(
        warframe_id,
        fid,
        key,
        variant,
        name_en,
        name_zh,
        nickname,
        vault,
        suit_kind,
        companion_type,
        &pf,
    );

    let part_progress = load_part_progress_for_warframe(&catalog, &user, warframe_id)?;
    let parts = load_parts(&catalog, warframe_id, &part_progress)?;

    Ok(WarframeDetail { warframe, parts })
}

pub fn upsert_progress_warframe(
    data_dir: &Path,
    patch: UpsertWarframeProgress,
) -> Result<(), String> {
    let warframe_id = parse_id(&patch.warframe_id, "战甲 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let variant: i32 = catalog
        .query_row(
            "SELECT variant FROM warframe WHERE id = ?1",
            [warframe_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("未找到战甲 id={}", patch.warframe_id))?;

    let existing = load_one_progress(&user, warframe_id)?.unwrap_or_default();
    let mut row = ProgressRow {
        status: patch.status.unwrap_or(existing.status),
        forma_done: patch.forma_done.unwrap_or(existing.forma_done),
        note: patch.note.clone().or(existing.note),
        wish: patch.wish.unwrap_or(existing.wish),
        mastered: patch.mastered.unwrap_or(existing.mastered),
        helminth: patch.helminth.unwrap_or(existing.helminth),
    };
    // Helminth 仅本体有意义
    if variant != 0 {
        row.helminth = 0;
        if patch.helminth.is_some() {
            // 忽略非本体的 helminth 写入意图
            row.helminth = 0;
        }
    }

    upsert_row(&user, warframe_id, &row, &now)?;

    if patch.status.is_some() {
        cascade_parts_status(&user, &catalog, warframe_id, row.status, &now)?;
    }

    Ok(())
}

pub fn upsert_progress_part(data_dir: &Path, patch: UpsertPartProgress) -> Result<(), String> {
    let part_id = parse_id(&patch.part_id, "部件 id")?;
    let (catalog, user) = open_pair(data_dir)?;
    let now = Utc::now().to_rfc3339();

    let (cur_status, cur_note): (i32, Option<String>) = user
        .query_row(
            "SELECT status, note FROM warframe_part_progress WHERE part_id = ?1",
            [part_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or((0, None));

    let status = patch.status.unwrap_or(cur_status);
    let note = patch.note.or(cur_note);

    user.execute(
        "INSERT INTO warframe_part_progress (part_id, status, note, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(part_id) DO UPDATE SET
            status=excluded.status,
            note=excluded.note,
            updated_at=excluded.updated_at",
        params![part_id, status, note, now],
    )
    .map_err(|e| e.to_string())?;

    if patch.status.is_some() {
        recompute_warframe_from_part(&user, &catalog, part_id, &now)?;
    }

    Ok(())
}

fn upsert_row(
    user: &Connection,
    warframe_id: i64,
    row: &ProgressRow,
    now: &str,
) -> Result<(), String> {
    user.execute(
        "INSERT INTO warframe_progress (
            warframe_id, status, forma_done, note, wish, mastered, helminth, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(warframe_id) DO UPDATE SET
            status=excluded.status,
            forma_done=excluded.forma_done,
            note=excluded.note,
            wish=excluded.wish,
            mastered=excluded.mastered,
            helminth=excluded.helminth,
            updated_at=excluded.updated_at",
        params![
            warframe_id,
            row.status,
            row.forma_done,
            row.note,
            row.wish,
            row.mastered,
            row.helminth,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn list_part_ids(catalog: &Connection, warframe_id: i64) -> Result<Vec<i64>, String> {
    let mut stmt = catalog
        .prepare("SELECT id FROM warframe_part WHERE warframe_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([warframe_id], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for r in rows {
        ids.push(r.map_err(|e| e.to_string())?);
    }
    Ok(ids)
}

fn cascade_parts_status(
    user: &Connection,
    catalog: &Connection,
    warframe_id: i64,
    status: i32,
    now: &str,
) -> Result<(), String> {
    // catalog / user 分库，无法 INSERT…SELECT；复用预编译语句减少逐行 prepare
    let ids = list_part_ids(catalog, warframe_id)?;
    if ids.is_empty() {
        return Ok(());
    }
    let mut stmt = user
        .prepare(
            "INSERT INTO warframe_part_progress (part_id, status, note, updated_at)
             VALUES (?1, ?2, NULL, ?3)
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

fn recompute_warframe_from_part(
    user: &Connection,
    catalog: &Connection,
    part_id: i64,
    now: &str,
) -> Result<(), String> {
    let warframe_id: i64 = catalog
        .query_row(
            "SELECT warframe_id FROM warframe_part WHERE id = ?1",
            [part_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("部件所属战甲查询失败: {e}"))?;

    let part_ids = list_part_ids(catalog, warframe_id)?;
    if part_ids.is_empty() {
        return Ok(());
    }

    // 缺行视为 0：COUNT < 部件数 → min=0；否则取 MIN(status)
    let placeholders = std::iter::repeat_n("?", part_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT COUNT(*), MIN(status) FROM warframe_part_progress WHERE part_id IN ({placeholders})"
    );
    let (cnt, min_found): (i64, Option<i32>) = user
        .query_row(&sql, params_from_iter(part_ids.iter().copied()), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .map_err(|e| e.to_string())?;
    let min_status = if (cnt as usize) < part_ids.len() {
        0
    } else {
        min_found.unwrap_or(0)
    };

    let existing = load_one_progress(user, warframe_id)?.unwrap_or_default();
    upsert_row(
        user,
        warframe_id,
        &ProgressRow {
            status: min_status,
            forma_done: existing.forma_done,
            note: existing.note,
            wish: existing.wish,
            mastered: existing.mastered,
            helminth: existing.helminth,
        },
        now,
    )?;
    Ok(())
}

fn merge_item(
    warframe_id: i64,
    family_id: i64,
    family_key: String,
    variant: i32,
    name_en: String,
    name_zh: Option<String>,
    nickname: Option<String>,
    vault: i32,
    suit_kind: String,
    companion_type: Option<String>,
    pf: &ProgressRow,
) -> WarframeListItem {
    let status_display = match pf.status {
        1 => "蓝图已齐".into(),
        2 => "铸造中".into(),
        3 => "已获取".into(),
        _ => "未获取".into(),
    };
    let is_warframe = suit_kind == "warframe";
    WarframeListItem {
        warframe_id: id_to_str(warframe_id),
        family_id: id_to_str(family_id),
        family_key,
        variant,
        name_en,
        name_zh,
        nickname,
        status: pf.status,
        status_display,
        helminth: if is_warframe && variant == 0 {
            pf.helminth
        } else {
            0
        },
        forma_done: pf.forma_done,
        wish: pf.wish,
        note: pf.note.clone(),
        vault_status: if variant == 1 { Some(vault) } else { None },
        mastered: pf.mastered,
        suit_kind,
        companion_type,
    }
}

fn load_one_progress(user: &Connection, warframe_id: i64) -> Result<Option<ProgressRow>, String> {
    user.query_row(
        "SELECT status, forma_done, note, wish, mastered, helminth
         FROM warframe_progress WHERE warframe_id = ?1",
        [warframe_id],
        |row| {
            Ok(ProgressRow {
                status: row.get(0)?,
                forma_done: row.get(1)?,
                note: row.get(2)?,
                wish: row.get(3)?,
                mastered: row.get(4)?,
                helminth: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn load_all_progress(user: &Connection) -> Result<HashMap<i64, ProgressRow>, String> {
    let mut stmt = user
        .prepare(
            "SELECT warframe_id, status, forma_done, note, wish, mastered, helminth
             FROM warframe_progress",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                ProgressRow {
                    status: row.get(1)?,
                    forma_done: row.get(2)?,
                    note: row.get(3)?,
                    wish: row.get(4)?,
                    mastered: row.get(5)?,
                    helminth: row.get(6)?,
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

/// 详情热路径：只拉该 warframe 部件的进度（catalog / user 分库，先取 id 再 IN）。
fn load_part_progress_for_warframe(
    catalog: &Connection,
    user: &Connection,
    warframe_id: i64,
) -> Result<HashMap<i64, (i32, Option<String>)>, String> {
    let mut stmt = catalog
        .prepare("SELECT id FROM warframe_part WHERE warframe_id = ?1")
        .map_err(|e| e.to_string())?;
    let ids: Vec<i64> = stmt
        .query_map([warframe_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = std::iter::repeat_n("?", ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT part_id, status, note FROM warframe_part_progress WHERE part_id IN ({placeholders})"
    );
    let mut stmt = user.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params_from_iter(ids.iter().copied()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for r in rows {
        let (id, status, note) = r.map_err(|e| e.to_string())?;
        map.insert(id, (status, note));
    }
    Ok(map)
}

fn load_parts(
    catalog: &Connection,
    warframe_id: i64,
    part_progress: &HashMap<i64, (i32, Option<String>)>,
) -> Result<Vec<PartDto>, String> {
    let resurgence_relics = crate::catalog::load_resurgence_relic_keys(catalog).unwrap_or_default();
    let mut stmt = catalog
        .prepare(
            "SELECT id, part_key, name_zh, sort_order
             FROM warframe_part WHERE warframe_id = ?1 ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([warframe_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut parts = Vec::new();
    for r in rows {
        let (pid, pkey, name_zh, sort) = r.map_err(|e| e.to_string())?;
        let (status, note) = part_progress.get(&pid).cloned().unwrap_or((0, None));
        let drops = load_part_relic_drops(catalog, pid, "warframe_part_relic", &resurgence_relics)?;
        parts.push(PartDto {
            part_id: id_to_str(pid),
            part_key: pkey,
            name_zh,
            sort_order: sort,
            status,
            note,
            drops,
        });
    }
    Ok(parts)
}

#[cfg(test)]
mod tests;
