//! catalog.sqlite + user.sqlite 初始化与迁移。
//!
//! 首次启动若无库则从 resources 拷贝种子 sqlite；版本低则升级。
//! - catalog：[`crate::catalog_migrate`] 逐步 migrate（种子 DDL 仅 v0→v1）
//! - user：[`crate::user_migrate`] 逐步 migrate
//! 维护者可用 `seed_catalog` 用 resources 内 JSON 重灌种子。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::catalog_migrate::{self, migrate_catalog_conn};
use crate::user_migrate::{self, migrate_user_conn, migrate_warframe_family_progress_if_needed};

pub use catalog_migrate::CATALOG_SCHEMA_VERSION;
pub use user_migrate::USER_SCHEMA_VERSION;

/// 空库 / 文档用全量 DDL；运行时升级走 `user_migrate`，不整份重跑此文件。
#[allow(dead_code)]
const USER_SQL: &str = include_str!("../sql/user_v1.sql");

/// 本进程内已完成完整初始化后，热路径跳过迁移/回填。
static DB_FULLY_READY: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbPaths {
    pub data_dir: String,
    pub catalog: String,
    pub user: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatus {
    pub paths: DbPaths,
    pub catalog_schema_version: i32,
    pub user_schema_version: i32,
    pub catalog_tables: Vec<String>,
    pub user_tables: Vec<String>,
}

pub fn paths_in(data_dir: &Path) -> DbPaths {
    DbPaths {
        data_dir: data_dir.display().to_string(),
        catalog: data_dir.join("catalog.sqlite").display().to_string(),
        user: data_dir.join("user.sqlite").display().to_string(),
    }
}

/// 无 user 库则新建；有则按版本增量升级 / 跳过 / 报错（见 `user_migrate`）。
pub fn ensure_and_migrate_user(data_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
    let user_path = data_dir.join("user.sqlite");
    let is_new = !user_path.is_file();
    let conn = open(&user_path)?;
    migrate_user_conn(&conn)?;
    if is_new {
        eprintln!("[仓鼠] 已新建 user 库: {}", user_path.display());
    }
    Ok(())
}

static BUNDLED_RESOURCE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 打包后由 Tauri `resource_dir` 注入；开发期回退 `CARGO_MANIFEST_DIR/resources`。
pub fn set_bundled_resource_dir(dir: PathBuf) {
    let _ = BUNDLED_RESOURCE_DIR.set(dir);
}

/// 安装包 / 开发期 resources 下的文件（种子 sqlite、词典、俗称等）。
pub fn bundled_resource_path(name: &str) -> PathBuf {
    let mut candidates = Vec::new();
    if let Some(dir) = BUNDLED_RESOURCE_DIR.get() {
        candidates.push(dir.join(name));
        candidates.push(dir.join("resources").join(name));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(name),
    );
    for path in &candidates {
        if path.is_file() {
            return path.clone();
        }
    }
    candidates.pop().unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join(name)
    })
}

pub fn init_databases(data_dir: &Path) -> Result<DbStatus, String> {
    // 热路径：启动后已完整初始化过则直接返回，避免每次 IPC 开库/迁移/回填。
    if DB_FULLY_READY.load(Ordering::Acquire) {
        return Ok(DbStatus {
            paths: paths_in(data_dir),
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            user_schema_version: USER_SCHEMA_VERSION,
            catalog_tables: Vec::new(),
            user_tables: Vec::new(),
        });
    }
    let status = init_databases_ex(data_dir, true)?;
    DB_FULLY_READY.store(true, Ordering::Release);
    Ok(status)
}

/// `copy_seed`：AppData 无对应库时从 resources 拷贝种子 sqlite。
pub fn init_databases_ex(data_dir: &Path, copy_seed: bool) -> Result<DbStatus, String> {
    // 测试/重灌可反复走完整路径
    DB_FULLY_READY.store(false, Ordering::Release);
    fs::create_dir_all(data_dir).map_err(|e| format!("创建数据目录失败: {e}"))?;

    let catalog_path = data_dir.join("catalog.sqlite");
    let user_path = data_dir.join("user.sqlite");

    if copy_seed {
        copy_seed_if_missing(&catalog_path, "catalog.sqlite")?;
        copy_seed_if_missing(&user_path, "user.sqlite")?;
    }

    let _ = replace_stale_catalog(data_dir)?;
    ensure_and_migrate_catalog(&catalog_path)?;
    ensure_and_migrate_user(data_dir)?;

    {
        let mut catalog = open_catalog(&catalog_path)?;
        // 种子已含对照时跳过；空库或指纹变了才合并 userdict
        let n: i64 = catalog
            .query_row("SELECT COUNT(*) FROM name_i18n", [], |r| r.get(0))
            .unwrap_or(0);
        if n == 0 {
            let _ = crate::catalog::ensure_name_i18n_from_bundled(&mut catalog)?;
        }
        let _ = crate::catalog::backfill_arcane_name_zh(&catalog)?;
    }

    let catalog = open(&catalog_path)?;
    let user = open(&user_path)?;
    // 需 catalog 的进度数据迁移：只在 init 做，不进 progress 热路径
    migrate_warframe_family_progress_if_needed(&catalog, &user)?;

    let status = DbStatus {
        paths: paths_in(data_dir),
        catalog_schema_version: read_user_version(&catalog)?,
        user_schema_version: read_user_version(&user)?,
        catalog_tables: list_tables(&catalog)?,
        user_tables: list_tables(&user)?,
    };
    DB_FULLY_READY.store(true, Ordering::Release);
    Ok(status)
}

fn copy_seed_if_missing(dest: &Path, resource_name: &str) -> Result<(), String> {
    if dest.is_file() {
        return Ok(());
    }
    let src = bundled_resource_path(resource_name);
    if !src.is_file() {
        return Ok(());
    }
    fs::copy(&src, dest).map_err(|e| {
        format!(
            "拷贝种子 {} → {} 失败: {e}",
            src.display(),
            dest.display()
        )
    })?;
    eprintln!(
        "[仓鼠] 已拷贝种子 {} ← {}",
        dest.file_name().and_then(|s| s.to_str()).unwrap_or("?"),
        src.display()
    );
    Ok(())
}

/// 无 catalog 则新建空库，再按版本逐步升级到 [`CATALOG_SCHEMA_VERSION`]。
pub fn ensure_and_migrate_catalog(catalog_path: &Path) -> Result<(), String> {
    if let Some(parent) = catalog_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建数据目录失败: {e}"))?;
    }
    let is_new = !catalog_path.is_file();
    let conn = open(catalog_path)?;
    migrate_catalog_conn(&conn)?;
    if is_new {
        eprintln!("[仓鼠] 已新建 catalog 库: {}", catalog_path.display());
    }
    Ok(())
}

fn remove_sqlite(path: &Path) -> Result<(), String> {
    for ext in ["", "-wal", "-shm"] {
        let p = if ext.is_empty() {
            path.to_path_buf()
        } else {
            PathBuf::from(format!("{}{ext}", path.display()))
        };
        if p.is_file() {
            fs::remove_file(&p).map_err(|e| format!("删除 {} 失败: {e}", p.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn has_table(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |_| Ok(1i32),
    )
    .optional()
    .ok()
    .flatten()
    .is_some()
}

pub(crate) fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name=?1");
    conn.query_row(&sql, [column], |_| Ok(1i32))
        .optional()
        .ok()
        .flatten()
        .is_some()
}

/// 极旧表形（无法增量迁）时删掉，再由种子/逐步 migrate 重建空库。
fn replace_stale_catalog(data_dir: &Path) -> Result<bool, String> {
    let path = data_dir.join("catalog.sqlite");
    if !path.is_file() {
        return Ok(false);
    }
    let conn = open(&path)?;
    let stale = has_table(&conn, "part_drop") || has_table(&conn, "warframe_colloquial");
    drop(conn);
    if !stale {
        return Ok(false);
    }
    remove_sqlite(&path)?;
    eprintln!("[仓鼠] 已删除无法迁移的旧 catalog");
    Ok(true)
}

pub fn open(path: &Path) -> Result<Connection, String> {
    let conn =
        Connection::open(path).map_err(|e| format!("打开 {} 失败: {e}", path.display()))?;
    // 读写并发：后台拉网写库时，前端读库不要立刻 SQLITE_BUSY 失败
    let _ = conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA busy_timeout = 8000;
         PRAGMA foreign_keys = ON;",
    );
    Ok(conn)
}

pub fn open_catalog(path: &Path) -> Result<Connection, String> {
    open(path)
}

pub fn open_user(path: &Path) -> Result<Connection, String> {
    open(path)
}

pub(crate) fn read_user_version(conn: &Connection) -> Result<i32, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| format!("读 user_version 失败: {e}"))
}

fn list_tables(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    let mut names = Vec::new();
    for r in rows {
        names.push(r.map_err(|e| e.to_string())?);
    }
    Ok(names)
}

#[allow(dead_code)]
pub fn exists(data_dir: &Path) -> bool {
    data_dir.join("catalog.sqlite").is_file() && data_dir.join("user.sqlite").is_file()
}

#[allow(dead_code)]
pub fn catalog_path(data_dir: &Path) -> PathBuf {
    data_dir.join("catalog.sqlite")
}

#[allow(dead_code)]
pub fn user_path(data_dir: &Path) -> PathBuf {
    data_dir.join("user.sqlite")
}

/// 上游 JSON 落盘目录（与 sqlite 同级）。
#[allow(dead_code)]
pub fn upstream_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("upstream")
}

#[allow(dead_code)]
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT value FROM setting WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO setting(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_creates_expected_tables() {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_db_test_{}_{}",
            std::process::id(),
            current_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        // 用 ex：避免并行测试里 DB_FULLY_READY 热路径返回空表列表
        let status = init_databases_ex(&dir, true).expect("init");
        assert_eq!(status.catalog_schema_version, CATALOG_SCHEMA_VERSION);
        assert_eq!(status.user_schema_version, USER_SCHEMA_VERSION);
        for name in [
            "warframe_family",
            "warframe",
            "warframe_part",
            "void_relic",
            "warframe_part_relic",
            "name_i18n",
            "setting",
            "resurgence_rotation",
            "resurgence_item",
            "relic_reward",
            "relic_location",
            "mod",
            "arcane",
            "live_sortie",
            "live_sortie_mission",
            "live_nightwave_challenge",
            "live_steel_incursion",
            "live_fissure",
            "live_bounty_board",
            "live_bounty_job",
            "live_archon_hunt",
            "live_circuit",
        ] {
            assert!(
                status.catalog_tables.iter().any(|t| t == name),
                "missing catalog table {name}"
            );
        }
        for name in [
            "warframe_progress",
            "warframe_part_progress",
            "mod_progress",
            "mod_tag",
            "arcane_progress",
            "user_todo",
            "routine_item",
            "setting",
        ] {
            assert!(
                status.user_tables.iter().any(|t| t == name),
                "missing user table {name}"
            );
        }
        let again = init_databases_ex(&dir, true).expect("re-init");
        assert_eq!(again.catalog_schema_version, CATALOG_SCHEMA_VERSION);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn init_empty_catalog_has_no_warframes_until_update() {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_empty_cat_{}_{}",
            std::process::id(),
            current_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        let status = init_databases_ex(&dir, false).expect("init");
        assert_eq!(status.catalog_schema_version, CATALOG_SCHEMA_VERSION);
        let n: i64 = open_catalog(&dir.join("catalog.sqlite"))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM warframe_family", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "空库不应预置战甲行");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_migrate_preserves_progress_from_v0() {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_user_mig_{}_{}",
            std::process::id(),
            current_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("user.sqlite");
        {
            let conn = open(&path).unwrap();
            // 模拟极旧库：只有旧表名、version=0
            conn.execute_batch(
                r#"
CREATE TABLE progress_family (
  family_id INTEGER PRIMARY KEY,
  base_status INTEGER NOT NULL DEFAULT 0,
  prime_status INTEGER NOT NULL DEFAULT 0,
  helminth INTEGER NOT NULL DEFAULT 0,
  forma_done INTEGER NOT NULL DEFAULT 0,
  wish INTEGER NOT NULL DEFAULT 0,
  note TEXT,
  updated_at TEXT
);
INSERT INTO progress_family (family_id, base_status, wish) VALUES (42, 3, 1);
"#,
            )
            .unwrap();
        }
        ensure_and_migrate_user(&dir).expect("migrate");
        let conn = open_user(&path).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        let (base, wish): (i64, i64) = conn
            .query_row(
                "SELECT base_status, wish FROM warframe_family_progress WHERE family_id=42",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(base, 3);
        assert_eq!(wish, 1);
        assert!(has_column(
            &conn,
            "warframe_family_progress",
            "mastered_base"
        ));
        // 相等：再跑不动数据
        ensure_and_migrate_user(&dir).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM warframe_family_progress WHERE family_id=42",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn user_higher_version_errors() {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_user_high_{}_{}",
            std::process::id(),
            current_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        ensure_and_migrate_user(&dir).unwrap();
        {
            let conn = open_user(&dir.join("user.sqlite")).unwrap();
            conn.pragma_update(None, "user_version", USER_SCHEMA_VERSION + 9)
                .unwrap();
        }
        let err = ensure_and_migrate_user(&dir).unwrap_err();
        assert!(err.contains("高于程序支持"), "{err}");
        // 文件仍在，未被删除
        assert!(dir.join("user.sqlite").is_file());
        let _ = fs::remove_dir_all(&dir);
    }

    fn current_nanos() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
