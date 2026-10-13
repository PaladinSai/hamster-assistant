//! WFCD Arcanes.json → catalog.`arcane`

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::Value;

use crate::catalog::i18n::ZhLookup;
use crate::id::Snowflake;

/// 品类（与 11-表结构草案一致）
pub const ARCANE_KIND_WARFRAME: i32 = 0;
pub const ARCANE_KIND_PRIMARY: i32 = 1;
pub const ARCANE_KIND_SECONDARY: i32 = 2;
pub const ARCANE_KIND_MELEE: i32 = 3;
pub const ARCANE_KIND_OPERATOR: i32 = 4;
pub const ARCANE_KIND_AMP: i32 = 5;
pub const ARCANE_KIND_KITGUN: i32 = 6;
pub const ARCANE_KIND_ZAW: i32 = 7;
pub const ARCANE_KIND_OTHER: i32 = 8;

/// 品级
pub const RARITY_COMMON: i32 = 0;
pub const RARITY_UNCOMMON: i32 = 1;
pub const RARITY_RARE: i32 = 2;
pub const RARITY_LEGENDARY: i32 = 3;

/// 来源桶
pub const SRC_OTHER: i32 = 0;
pub const SRC_EIDOLON: i32 = 1;
pub const SRC_ISOLATION: i32 = 2;
pub const SRC_ZARIMAN: i32 = 3;
pub const SRC_DUVIRI: i32 = 4;
pub const SRC_RAILJACK: i32 = 5;
pub const SRC_HEX: i32 = 6;
pub const SRC_SANCTUM: i32 = 7;
pub const SRC_VENDOR: i32 = 8;
pub const SRC_ACOLYTE: i32 = 9;
pub const SRC_ASCENSION: i32 = 10;
pub const SRC_ARBITRATION: i32 = 11;
pub const SRC_LUA: i32 = 12;

#[derive(Debug, Default, Clone)]
pub struct ArcaneImportStats {
    pub arcanes: usize,
    pub skipped: usize,
}

#[derive(Debug, Deserialize)]
struct WfcdDrop {
    #[serde(default)]
    location: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WfcdLevelStat {
    #[serde(default)]
    #[allow(dead_code)]
    stats: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WfcdArcane {
    name: String,
    #[serde(rename = "uniqueName")]
    unique_name: String,
    #[serde(default)]
    #[serde(rename = "type")]
    item_type: Option<String>,
    #[serde(default)]
    rarity: Option<String>,
    #[serde(default)]
    #[serde(rename = "excludeFromCodex")]
    exclude_from_codex: Option<bool>,
    #[serde(default)]
    #[serde(rename = "levelStats")]
    level_stats: Vec<WfcdLevelStat>,
    #[serde(default)]
    drops: Vec<WfcdDrop>,
}

fn map_arcane_kind(item_type: &str) -> i32 {
    let t = item_type.trim();
    if t.eq_ignore_ascii_case("Warframe Arcane") {
        ARCANE_KIND_WARFRAME
    } else if t.eq_ignore_ascii_case("Primary Arcane") {
        ARCANE_KIND_PRIMARY
    } else if t.eq_ignore_ascii_case("Secondary Arcane") {
        ARCANE_KIND_SECONDARY
    } else if t.eq_ignore_ascii_case("Melee Arcane") {
        ARCANE_KIND_MELEE
    } else if t.eq_ignore_ascii_case("Operator Arcane") {
        ARCANE_KIND_OPERATOR
    } else if t.eq_ignore_ascii_case("Amp Arcane") {
        ARCANE_KIND_AMP
    } else if t.eq_ignore_ascii_case("Kitgun Arcane") {
        ARCANE_KIND_KITGUN
    } else if t.eq_ignore_ascii_case("Zaw Arcane") {
        ARCANE_KIND_ZAW
    } else {
        ARCANE_KIND_OTHER
    }
}

fn map_rarity(rarity: Option<&str>) -> i32 {
    match rarity.map(|s| s.trim()) {
        Some("Common") => RARITY_COMMON,
        Some("Uncommon") => RARITY_UNCOMMON,
        Some("Rare") => RARITY_RARE,
        Some("Legendary") => RARITY_LEGENDARY,
        _ => RARITY_COMMON,
    }
}

fn source_rank(kind: i32) -> i32 {
    match kind {
        SRC_EIDOLON => 0,
        SRC_ISOLATION => 1,
        SRC_ZARIMAN => 2,
        SRC_DUVIRI => 3,
        SRC_HEX => 4,
        SRC_SANCTUM => 5,
        SRC_RAILJACK => 6,
        SRC_VENDOR => 7,
        SRC_ACOLYTE => 8,
        SRC_ASCENSION => 9,
        SRC_ARBITRATION => 10,
        SRC_LUA => 11,
        _ => 99,
    }
}

fn prefer_source(current: i32, candidate: i32) -> i32 {
    if source_rank(candidate) < source_rank(current) {
        candidate
    } else {
        current
    }
}

pub fn source_from_drop_location(location: &str) -> i32 {
    let loc = location.to_ascii_lowercase();
    if loc.contains("eidolon")
        || loc.contains("teralyst")
        || loc.contains("gantulyst")
        || loc.contains("hydrolyst")
    {
        return SRC_EIDOLON;
    }
    if loc.contains("isolation vault") || loc.contains("arcana isolation") {
        return SRC_ISOLATION;
    }
    if loc.contains("void angel")
        || loc.contains("thrax")
        || loc.contains("holdfast")
        || loc.contains("cavalero")
        || loc.contains("zariman")
    {
        return SRC_ZARIMAN;
    }
    if loc.contains("duviri") {
        return SRC_DUVIRI;
    }
    if loc.contains("proxima") || loc.contains("skirmish") || loc.contains("railjack") {
        return SRC_RAILJACK;
    }
    if loc.contains("descendia")
        || loc.contains("höllvania")
        || loc.contains("hollvania")
        || loc.contains("h-09")
        || loc.contains("nokko")
        || loc.contains("scaldra")
        || loc.contains("techrot")
        || loc.contains("the hex")
    {
        return SRC_HEX;
    }
    if loc.contains("archimedea")
        || loc.contains("netracell")
        || loc.contains("sanctum")
        || loc.contains("tyana")
        || loc.contains("whisper")
        || loc.contains("fragmented")
    {
        return SRC_SANCTUM;
    }
    if loc.contains("quill")
        || loc.contains("vox solaris")
        || loc.contains("solaris united")
        || loc.contains("ostron")
        || loc.contains("(hok)")
        || loc.contains("the quills")
    {
        return SRC_VENDOR;
    }
    if loc.contains("angst")
        || loc.contains("malice")
        || loc.contains("mania")
        || loc.contains("misery")
        || loc.contains("torment")
        || loc.contains("violence")
    {
        return SRC_ACOLYTE;
    }
    if loc.contains("ascension") || loc.contains("sister of parvos") {
        return SRC_ASCENSION;
    }
    if loc.contains("arbitration") {
        return SRC_ARBITRATION;
    }
    if loc.contains("circulus") || loc.contains("conjunction") {
        return SRC_LUA;
    }
    SRC_OTHER
}

fn primary_source(drops: &[WfcdDrop]) -> i32 {
    let mut kind = SRC_OTHER;
    for d in drops {
        if let Some(loc) = d.location.as_deref() {
            kind = prefer_source(kind, source_from_drop_location(loc));
        }
    }
    kind
}

fn max_rank_from_stats(level_stats: &[WfcdLevelStat]) -> i32 {
    if level_stats.is_empty() {
        return 5;
    }
    (level_stats.len() as i32 - 1).max(0)
}

fn should_skip(a: &WfcdArcane) -> bool {
    if a.exclude_from_codex == Some(true) {
        return true;
    }
    if a.name.trim().is_empty() || a.unique_name.trim().is_empty() {
        return true;
    }
    // 无 rarity 且无 drops：废案/占位
    if a.rarity.is_none() && a.drops.is_empty() {
        return true;
    }
    false
}

pub fn import_arcanes_json(
    conn: &mut Connection,
    arcanes_json: &[u8],
    snowflake: &Snowflake,
) -> Result<ArcaneImportStats, String> {
    let arr: Vec<Value> =
        serde_json::from_slice(arcanes_json).map_err(|e| format!("Arcanes.json 解析失败: {e}"))?;
    let zh = ZhLookup::load(conn)?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("赋能事务失败: {e}"))?;

    let mut stats = ArcaneImportStats::default();
    let mut sort = 0i32;

    for raw in arr {
        let a: WfcdArcane = match serde_json::from_value(raw) {
            Ok(v) => v,
            Err(_) => {
                stats.skipped += 1;
                continue;
            }
        };
        if should_skip(&a) {
            stats.skipped += 1;
            continue;
        }
        let item_type = a.item_type.as_deref().unwrap_or("");
        let kind = map_arcane_kind(item_type);
        let rarity = map_rarity(a.rarity.as_deref());
        let source = primary_source(&a.drops);
        let max_rank = max_rank_from_stats(&a.level_stats);
        let name_zh = zh.get(&a.name);

        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM arcane WHERE unique_name = ?1",
                [&a.unique_name],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;

        if let Some(id) = existing {
            tx.execute(
                "UPDATE arcane SET name_en=?2, name_zh=COALESCE(?3, name_zh),
                 arcane_kind=?4, rarity=?5, source_kind=?6, max_rank=?7, sort_order=?8
                 WHERE id=?1",
                params![
                    id,
                    a.name,
                    name_zh,
                    kind,
                    rarity,
                    source,
                    max_rank,
                    sort
                ],
            )
            .map_err(|e| format!("更新赋能失败: {e}"))?;
        } else {
            let id = snowflake.next_id();
            tx.execute(
                "INSERT INTO arcane (
                   id, unique_name, name_en, name_zh, arcane_kind, rarity,
                   source_kind, max_rank, sort_order
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    id,
                    a.unique_name,
                    a.name,
                    name_zh,
                    kind,
                    rarity,
                    source,
                    max_rank,
                    sort
                ],
            )
            .map_err(|e| format!("插入赋能失败: {e}"))?;
        }
        sort += 1;
        stats.arcanes += 1;
    }

    tx.commit().map_err(|e| format!("赋能提交失败: {e}"))?;
    Ok(stats)
}

/// 等级 `r` 时累计已投入张数：`T(r+1) = (r+1)(r+2)/2`（Wiki：R0=1 … R5=21）。
fn cumulative_copies_at_rank(rank: i32) -> i32 {
    let n = (rank.max(0) + 1) as i64;
    ((n * (n + 1)) / 2) as i32
}

/// 从当前等级升到满级还需多少张。
/// 未获取：满级累计 `T(max_rank+1)`（max=5 → 21）；已有 rank：差额；已满级：0。
pub fn copies_to_max(status: i32, rank: i32, max_rank: i32) -> i32 {
    let max_rank = max_rank.max(0);
    if status == 2 || rank >= max_rank {
        return 0;
    }
    let need = cumulative_copies_at_rank(max_rank);
    if status == 0 {
        return need;
    }
    let have = cumulative_copies_at_rank(rank.max(0));
    (need - have).max(0)
}

/// 用 `name_i18n` 回填 `arcane.name_zh`（空、或等于英文名时覆盖）。
pub fn backfill_arcane_name_zh(conn: &Connection) -> Result<usize, String> {
    let zh = ZhLookup::load(conn)?;
    let mut stmt = conn
        .prepare("SELECT id, name_en, name_zh FROM arcane")
        .map_err(|e| format!("读赋能失败: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut n = 0usize;
    for (id, name_en, name_zh) in rows {
        let Some(lookup) = zh.get(&name_en) else {
            continue;
        };
        let cur = name_zh.as_deref().map(str::trim).unwrap_or("");
        if !cur.is_empty() && cur != name_en {
            continue;
        }
        conn.execute(
            "UPDATE arcane SET name_zh = ?2 WHERE id = ?1",
            params![id, lookup],
        )
        .map_err(|e| format!("回填赋能中文失败: {e}"))?;
        n += 1;
    }
    Ok(n)
}

/// 由 rank / max_rank / 是否拥有 推导三态。
pub fn status_from_rank(owned: bool, rank: i32, max_rank: i32) -> i32 {
    if !owned {
        return 0;
    }
    if rank >= max_rank.max(0) {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_formula() {
        assert_eq!(copies_to_max(2, 5, 5), 0);
        // R2=6 → R5=21，还差 15（升阶 +4+5+6）
        assert_eq!(copies_to_max(1, 2, 5), 15);
        assert_eq!(copies_to_max(0, 0, 5), 21);
        // max_rank=3 → 满级累计 10；已有 R0=1，还差 9
        assert_eq!(copies_to_max(1, 0, 3), 9);
        assert_eq!(copies_to_max(0, 0, 3), 10);
    }

    #[test]
    fn source_buckets() {
        assert_eq!(
            source_from_drop_location("Eidolon Hydrolyst (Capture)"),
            SRC_EIDOLON
        );
        assert_eq!(
            source_from_drop_location("Deimos/Cambion Drift (Level 30 - 40 Isolation Vault)"),
            SRC_ISOLATION
        );
        assert_eq!(source_from_drop_location("Duviri/Endless: Tier 6"), SRC_DUVIRI);
        assert_eq!(
            source_from_drop_location("The Descendia: Stage 5"),
            SRC_HEX
        );
    }

    #[test]
    fn kind_mapping() {
        assert_eq!(map_arcane_kind("Warframe Arcane"), ARCANE_KIND_WARFRAME);
        assert_eq!(map_arcane_kind("Operator Arcane"), ARCANE_KIND_OPERATOR);
        assert_eq!(map_arcane_kind("Bow Arcane"), ARCANE_KIND_OTHER);
    }
}
