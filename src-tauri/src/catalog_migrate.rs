//! catalog.sqlite 增量迁移（与 [`crate::user_migrate`] 同范式）。
//!
//! - 无库 / `user_version` = 0：先跑种子 DDL（`sql/catalog_v1.sql`），再逐步升到当前
//! - version < 当前：先幂等补齐种子里的表/索引，再逐步 `migrate_catalog_to_vN`（ALTER / 回填）
//! - version == 当前：不动
//! - version > 当前：报错
//!
//! **约定**：改表结构 = bump [`CATALOG_SCHEMA_VERSION`] + 新增 `migrate_catalog_to_vN`。
//! `catalog_v1.sql` 是当前 schema 快照（仅 `CREATE IF NOT EXISTS`）；升级不再「整包 DDL 一次跳到目标版本」。
//! 中间空步（2..=14、16|17）对齐历史序号；列级变更只走对应 migrate。

use rusqlite::Connection;

use crate::db::{has_column, has_table, read_user_version};

/// 与 [`crate::db`] 导出保持一致。
pub const CATALOG_SCHEMA_VERSION: i32 = 22;

const CATALOG_SEED_SQL: &str = include_str!("../sql/catalog_v1.sql");

pub fn migrate_catalog_conn(conn: &Connection) -> Result<(), String> {
    let mut current = read_user_version(conn)?;
    if current > CATALOG_SCHEMA_VERSION {
        return Err(format!(
            "catalog 库 schema 版本 {current} 高于程序支持的 {CATALOG_SCHEMA_VERSION}，请升级软件或换回匹配的目录库"
        ));
    }
    // 打包种子 / 旧库可能停在中间版本且缺后来才进种子的表；CREATE IF NOT EXISTS 安全补齐。
    // 已有表缺列仍靠下方逐步 ALTER，不会被种子覆盖。
    if current > 0 && current < CATALOG_SCHEMA_VERSION {
        apply_seed_ddl(conn)?;
    }
    while current < CATALOG_SCHEMA_VERSION {
        let next = current + 1;
        match next {
            1 => migrate_catalog_to_v1(conn)?,
            // 历史曾靠整包 SQL 跳版本；空步只对齐序号（表已由 apply_seed_ddl / v1 补齐）
            2..=14 => {}
            15 => migrate_catalog_to_v15(conn)?,
            16 | 17 => {}
            18 => migrate_catalog_to_v18(conn)?,
            19 => migrate_catalog_to_v19(conn)?,
            20 => migrate_catalog_to_v20(conn)?,
            21 => migrate_catalog_to_v21(conn)?,
            22 => migrate_catalog_to_v22(conn)?,
            n => {
                return Err(format!(
                    "缺少 catalog 迁移步骤 → v{n}（程序目标 {CATALOG_SCHEMA_VERSION}）"
                ));
            }
        }
        conn.pragma_update(None, "user_version", next)
            .map_err(|e| format!("写 catalog user_version={next} 失败: {e}"))?;
        current = next;
    }
    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
    Ok(())
}

fn apply_seed_ddl(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(CATALOG_SEED_SQL)
        .map_err(|e| format!("catalog 种子 DDL 失败: {e}"))?;
    Ok(())
}

/// v0 → v1：种子全量 DDL（CREATE IF NOT EXISTS）。
fn migrate_catalog_to_v1(conn: &Connection) -> Result<(), String> {
    apply_seed_ddl(conn)
}

/// v14 → v15：catalog.setting + name_i18n 双译名列。
fn migrate_catalog_to_v15(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "setting") {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS setting (
               key   TEXT NOT NULL PRIMARY KEY,
               value TEXT NOT NULL
             );",
        )
        .map_err(|e| format!("创建 catalog.setting 失败: {e}"))?;
    }
    if has_table(conn, "name_i18n") {
        if !has_column(conn, "name_i18n", "name_zh_userdict") {
            conn.execute(
                "ALTER TABLE name_i18n ADD COLUMN name_zh_userdict TEXT",
                [],
            )
            .map_err(|e| format!("name_i18n 增加 name_zh_userdict 失败: {e}"))?;
        }
        if !has_column(conn, "name_i18n", "name_zh_wfcd") {
            conn.execute("ALTER TABLE name_i18n ADD COLUMN name_zh_wfcd TEXT", [])
                .map_err(|e| format!("name_i18n 增加 name_zh_wfcd 失败: {e}"))?;
        }
        if has_column(conn, "name_i18n", "name_zh") {
            if has_column(conn, "name_i18n", "source") {
                let _ = conn.execute_batch(
                    "UPDATE name_i18n SET name_zh_userdict = name_zh
                     WHERE (name_zh_userdict IS NULL OR trim(name_zh_userdict) = '')
                       AND name_zh IS NOT NULL AND trim(name_zh) != ''
                       AND (source IS NULL OR source = 0 OR source = 1);
                     UPDATE name_i18n SET name_zh_wfcd = name_zh
                     WHERE (name_zh_wfcd IS NULL OR trim(name_zh_wfcd) = '')
                       AND name_zh IS NOT NULL AND trim(name_zh) != ''
                       AND source = 2;",
                );
            }
            let _ = conn.execute_batch(
                "UPDATE name_i18n SET name_zh_userdict = name_zh
                 WHERE (name_zh_userdict IS NULL OR trim(name_zh_userdict) = '')
                   AND (name_zh_wfcd IS NULL OR trim(name_zh_wfcd) = '')
                   AND name_zh IS NOT NULL AND trim(name_zh) != '';",
            );
        }
    }
    Ok(())
}

/// v17 → v18：suit_kind（warframe / archwing / necramech）。
fn migrate_catalog_to_v18(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "warframe_family") {
        return Ok(());
    }
    if !has_column(conn, "warframe_family", "suit_kind") {
        conn.execute(
            "ALTER TABLE warframe_family ADD COLUMN suit_kind TEXT NOT NULL DEFAULT 'warframe'",
            [],
        )
        .map_err(|e| format!("warframe_family 增加 suit_kind 失败: {e}"))?;
    }
    let _ = conn.execute_batch(
        "UPDATE warframe_family SET suit_kind = 'necramech'
         WHERE id IN (
           SELECT f.id FROM warframe_family f
           JOIN warframe w ON w.family_id = f.id
           WHERE w.unique_name LIKE '%/EntratiMech/%'
         );
         UPDATE warframe_family SET suit_kind = 'archwing'
         WHERE id IN (
           SELECT f.id FROM warframe_family f
           JOIN warframe w ON w.family_id = f.id
           WHERE w.unique_name LIKE '%/Archwing/%'
         );",
    );
    Ok(())
}

/// v18 → v19：weapon_kind（weapon / archwing）。
fn migrate_catalog_to_v19(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "weapon_family") {
        return Ok(());
    }
    if !has_column(conn, "weapon_family", "weapon_kind") {
        conn.execute(
            "ALTER TABLE weapon_family ADD COLUMN weapon_kind TEXT NOT NULL DEFAULT 'weapon'",
            [],
        )
        .map_err(|e| format!("weapon_family 增加 weapon_kind 失败: {e}"))?;
    }
    let _ = conn.execute_batch(
        "UPDATE weapon_family SET weapon_kind = 'weapon';
         UPDATE weapon_family SET weapon_kind = 'archwing'
         WHERE id IN (
           SELECT DISTINCT f.id FROM weapon_family f
           JOIN weapon w ON w.family_id = f.id
           WHERE w.unique_name LIKE '%/Archwing/Primary/%'
              OR w.unique_name LIKE '%/HeavyWeapons/%'
              OR (
                   w.unique_name LIKE '%/Archwing/Melee/%'
               AND w.unique_name NOT LIKE '%/ExaltedArchScythe/%'
               AND w.unique_name NOT LIKE '%/ExaltedAWScythe/%'
               AND w.unique_name NOT LIKE '%/PrimeCorufell/%'
               AND w.unique_name NOT LIKE '%Corufell%'
              )
         );",
    );
    Ok(())
}

/// v19 → v20：companion_type；旧 sentinel → companion。
fn migrate_catalog_to_v20(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "warframe_family") {
        return Ok(());
    }
    if !has_column(conn, "warframe_family", "companion_type") {
        conn.execute(
            "ALTER TABLE warframe_family ADD COLUMN companion_type TEXT",
            [],
        )
        .map_err(|e| format!("warframe_family 增加 companion_type 失败: {e}"))?;
    }
    let _ = conn.execute_batch(
        "UPDATE warframe_family
         SET suit_kind = 'companion', companion_type = COALESCE(companion_type, 'sentinel')
         WHERE suit_kind = 'sentinel';

         UPDATE warframe_family SET companion_type = 'kubrow'
         WHERE suit_kind = 'companion'
           AND IFNULL(companion_type, '') = ''
           AND id IN (
             SELECT f.id FROM warframe_family f
             JOIN warframe w ON w.family_id = f.id
             WHERE w.unique_name LIKE '%/KubrowPet/%'
           );
         UPDATE warframe_family SET companion_type = 'kavat'
         WHERE suit_kind = 'companion'
           AND IFNULL(companion_type, '') = ''
           AND id IN (
             SELECT f.id FROM warframe_family f
             JOIN warframe w ON w.family_id = f.id
             WHERE w.unique_name LIKE '%/CatbrowPet/%'
           );
         UPDATE warframe_family SET companion_type = 'vulpaphyla'
         WHERE suit_kind = 'companion'
           AND IFNULL(companion_type, '') = ''
           AND id IN (
             SELECT f.id FROM warframe_family f
             JOIN warframe w ON w.family_id = f.id
             WHERE w.unique_name LIKE '%InfestedCatbrow%'
                OR w.name_en LIKE '%Vulpaphyla%'
           );
         UPDATE warframe_family SET companion_type = 'predasite'
         WHERE suit_kind = 'companion'
           AND IFNULL(companion_type, '') = ''
           AND id IN (
             SELECT f.id FROM warframe_family f
             JOIN warframe w ON w.family_id = f.id
             WHERE w.unique_name LIKE '%Predator%'
                OR w.name_en LIKE '%Predasite%'
           );
         UPDATE warframe_family SET companion_type = 'sentinel'
         WHERE suit_kind = 'companion' AND IFNULL(companion_type, '') = '';",
    );
    Ok(())
}

/// v20 → v21：自制武器 modular_kind。
fn migrate_catalog_to_v21(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "weapon_family") {
        return Ok(());
    }
    if !has_column(conn, "weapon_family", "modular_kind") {
        conn.execute(
            "ALTER TABLE weapon_family ADD COLUMN modular_kind TEXT",
            [],
        )
        .map_err(|e| format!("weapon_family 增加 modular_kind 失败: {e}"))?;
    }
    Ok(())
}

/// v21 → v22：把原先旁路 ensure 的 live 表补丁纳入版本（幂等）。
fn migrate_catalog_to_v22(conn: &Connection) -> Result<(), String> {
    if has_table(conn, "live_steel_incursion") {
        if !has_column(conn, "live_steel_incursion", "today_json") {
            conn.execute(
                "ALTER TABLE live_steel_incursion ADD COLUMN today_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )
            .map_err(|e| format!("live_steel_incursion 增加 today_json 失败: {e}"))?;
        }
        if !has_column(conn, "live_steel_incursion", "tomorrow_json") {
            conn.execute(
                "ALTER TABLE live_steel_incursion ADD COLUMN tomorrow_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )
            .map_err(|e| format!("live_steel_incursion 增加 tomorrow_json 失败: {e}"))?;
        }
    }
    if !has_table(conn, "live_fissure") {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS live_fissure (
               id            INTEGER NOT NULL PRIMARY KEY,
               kind          TEXT    NOT NULL,
               tier          TEXT    NOT NULL DEFAULT '',
               tier_zh       TEXT    NOT NULL DEFAULT '',
               mission_type  TEXT    NOT NULL DEFAULT '',
               node          TEXT    NOT NULL DEFAULT '',
               activation    TEXT,
               expiry        TEXT,
               sort_order    INTEGER NOT NULL DEFAULT 0,
               fetched_at    TEXT    NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_live_fissure_kind
               ON live_fissure(kind, sort_order);",
        )
        .map_err(|e| format!("创建 live_fissure 失败: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{has_column, has_table, open, read_user_version};

    struct HoldPath(std::path::PathBuf);
    impl Drop for HoldPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn open_temp() -> (HoldPath, Connection) {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_cat_mig_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let conn = open(&dir.join("catalog.sqlite")).unwrap();
        (HoldPath(dir), conn)
    }

    #[test]
    fn migrate_from_zero_reaches_current() {
        let (_hold, conn) = open_temp();
        assert_eq!(read_user_version(&conn).unwrap(), 0);
        migrate_catalog_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), CATALOG_SCHEMA_VERSION);
        assert!(has_table(&conn, "warframe_family"));
        assert!(has_table(&conn, "weapon_family"));
        assert!(has_column(&conn, "weapon_family", "modular_kind"));
        assert!(has_table(&conn, "live_fissure"));
    }

    #[test]
    fn migrate_is_idempotent() {
        let (_hold, conn) = open_temp();
        migrate_catalog_conn(&conn).unwrap();
        migrate_catalog_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), CATALOG_SCHEMA_VERSION);
    }

    #[test]
    fn migrate_v21_to_v22_adds_live_patches() {
        let (_hold, conn) = open_temp();
        // 模拟旧 v21：有钢表但无 json 列、无裂隙表
        conn.execute_batch(
            "CREATE TABLE live_steel_incursion (
               id INTEGER PRIMARY KEY,
               activation TEXT,
               expiry TEXT,
               fetched_at TEXT,
               source_url TEXT
             );",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 21).unwrap();
        migrate_catalog_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), CATALOG_SCHEMA_VERSION);
        assert!(has_column(&conn, "live_steel_incursion", "today_json"));
        assert!(has_table(&conn, "live_fissure"));
    }

    #[test]
    fn reject_future_schema() {
        let (_hold, conn) = open_temp();
        conn.pragma_update(None, "user_version", CATALOG_SCHEMA_VERSION + 1)
            .unwrap();
        let err = migrate_catalog_conn(&conn).unwrap_err();
        assert!(err.contains("高于程序支持"));
    }
}
