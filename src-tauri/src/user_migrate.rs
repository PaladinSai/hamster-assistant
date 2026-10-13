//! user.sqlite 增量迁移（不删进度）。
//!
//! - 无库：由调用方新建后再迁到当前版本
//! - version < 当前：逐步 migrate_to_N
//! - version == 当前：不动
//! - version > 当前：报错
//!
//! 需同时读 catalog 的数据迁移（旧族进度 → 变体行）见
//! [`migrate_warframe_family_progress_if_needed`]，由 `db::init_databases_ex` 调用。

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::db::{has_column, has_table, read_user_version};

const WARFRAME_PROGRESS_V14_FLAG: &str = "warframe_progress_v14";

/// 与 [`crate::db`] 中常量保持一致；升级表结构时在此加逐步迁移。
pub const USER_SCHEMA_VERSION: i32 = 16;

/// 内置 Mod 标签 id（与 v7 种子一致）
pub const MOD_TAG_CORRUPTED: i64 = 1;
pub const MOD_TAG_NIGHTMARE: i64 = 2;
pub const MOD_TAG_PRIME: i64 = 3;
pub const MOD_TAG_SYNDICATE: i64 = 4;

pub fn migrate_user_conn(conn: &Connection) -> Result<(), String> {
    let mut current = read_user_version(conn)?;
    if current > USER_SCHEMA_VERSION {
        return Err(format!(
            "user 库 schema 版本 {current} 高于程序支持的 {USER_SCHEMA_VERSION}，请升级软件或换回匹配的进度库"
        ));
    }
    while current < USER_SCHEMA_VERSION {
        let next = current + 1;
        match next {
            1 => migrate_user_to_v1(conn)?,
            2 => migrate_user_to_v2(conn)?,
            3 => migrate_user_to_v3(conn)?,
            4 => migrate_user_to_v4(conn)?,
            5 => migrate_user_to_v5(conn)?,
            6 => migrate_user_to_v6(conn)?,
            7 => migrate_user_to_v7(conn)?,
            8 => migrate_user_to_v8(conn)?,
            9 => migrate_user_to_v9(conn)?,
            10 => migrate_user_to_v10(conn)?,
            11 => migrate_user_to_v11(conn)?,
            12 => migrate_user_to_v12(conn)?,
            13 => migrate_user_to_v13(conn)?,
            14 => migrate_user_to_v14(conn)?,
            15 => migrate_user_to_v15(conn)?,
            16 => migrate_user_to_v16(conn)?,
            n => {
                return Err(format!(
                    "缺少 user 迁移步骤 → v{n}（程序目标 {USER_SCHEMA_VERSION}）"
                ));
            }
        }
        conn.pragma_update(None, "user_version", next)
            .map_err(|e| format!("写 user_version={next} 失败: {e}"))?;
        current = next;
    }
    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");
    Ok(())
}

/// 旧 `warframe_family_progress` → 按变体的 `warframe_progress`（需 catalog 查 variant id）。
/// 幂等；仅由 `init_databases` 路径调用，不挂在 progress 读写热路径上。
pub fn migrate_warframe_family_progress_if_needed(
    catalog: &Connection,
    user: &Connection,
) -> Result<(), String> {
    if !has_table(user, "warframe_family_progress") {
        return Ok(());
    }
    let done: Option<String> = user
        .query_row(
            "SELECT value FROM setting WHERE key = ?1",
            [WARFRAME_PROGRESS_V14_FLAG],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if done.as_deref() == Some("1") {
        let _ = user.execute("DROP TABLE IF EXISTS warframe_family_progress", []);
        return Ok(());
    }

    let mut stmt = user
        .prepare(
            "SELECT family_id, base_status, prime_status, helminth, forma_done,
                    wish_base, wish_prime, note, mastered_base, mastered_prime
             FROM warframe_family_progress",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, i32>(5)?,
                row.get::<_, i32>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, i32>(8)?,
                row.get::<_, i32>(9)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let now = Utc::now().to_rfc3339();
    for r in rows {
        let (
            family_id,
            base_status,
            prime_status,
            helminth,
            forma_done,
            wish_base,
            wish_prime,
            note,
            mastered_base,
            mastered_prime,
        ) = r.map_err(|e| e.to_string())?;

        if let Some(wid) = find_warframe_variant_id(catalog, family_id, 0)? {
            upsert_warframe_progress_row(
                user,
                wid,
                base_status,
                forma_done,
                note.as_deref(),
                if wish_base != 0 { 1 } else { 0 },
                mastered_base,
                helminth,
                &now,
            )?;
        }
        if let Some(wid) = find_warframe_variant_id(catalog, family_id, 1)? {
            upsert_warframe_progress_row(
                user,
                wid,
                prime_status,
                0,
                None,
                if wish_prime != 0 { 1 } else { 0 },
                mastered_prime,
                0,
                &now,
            )?;
        }
    }
    drop(stmt);

    user.execute("DROP TABLE IF EXISTS warframe_family_progress", [])
        .map_err(|e| format!("删除 warframe_family_progress 失败: {e}"))?;
    user.execute(
        "INSERT INTO setting (key, value) VALUES (?1, '1')
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [WARFRAME_PROGRESS_V14_FLAG],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn find_warframe_variant_id(
    catalog: &Connection,
    family_id: i64,
    variant: i32,
) -> Result<Option<i64>, String> {
    catalog
        .query_row(
            "SELECT id FROM warframe WHERE family_id = ?1 AND variant = ?2",
            params![family_id, variant],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
}

fn upsert_warframe_progress_row(
    user: &Connection,
    warframe_id: i64,
    status: i32,
    forma_done: i32,
    note: Option<&str>,
    wish: i32,
    mastered: i32,
    helminth: i32,
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
            status,
            forma_done,
            note,
            wish,
            mastered,
            helminth,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// v0 → v1：核心进度表 + setting（旧试验表名一并迁入）。
fn migrate_user_to_v1(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS warframe_family_progress (
  family_id      INTEGER NOT NULL PRIMARY KEY,
  base_status    INTEGER NOT NULL DEFAULT 0,
  prime_status   INTEGER NOT NULL DEFAULT 0,
  helminth       INTEGER NOT NULL DEFAULT 0,
  forma_done     INTEGER NOT NULL DEFAULT 0,
  wish           INTEGER NOT NULL DEFAULT 0,
  note           TEXT,
  updated_at     TEXT
);

CREATE TABLE IF NOT EXISTS warframe_part_progress (
  part_id    INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS setting (
  key   TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL
);
"#,
    )
    .map_err(|e| format!("user → v1 建表失败: {e}"))?;

    // 旧试验表名 → 新表（有则拷、无则跳过）
    if has_table(conn, "progress_family") {
        conn.execute_batch(
            r#"
INSERT OR IGNORE INTO warframe_family_progress (
  family_id, base_status, prime_status, helminth, forma_done, wish, note, updated_at
)
SELECT family_id, base_status, prime_status, helminth, forma_done, wish, note, updated_at
FROM progress_family;
"#,
        )
        .map_err(|e| format!("迁 progress_family 失败: {e}"))?;
    }
    if has_table(conn, "settings") {
        conn.execute_batch(
            r#"
INSERT OR IGNORE INTO setting (key, value)
SELECT key, value FROM settings;
"#,
        )
        .map_err(|e| format!("迁 settings 失败: {e}"))?;
    }
    Ok(())
}

/// v1 → v2：家族进度增加 mastered_* 列。
fn migrate_user_to_v2(conn: &Connection) -> Result<(), String> {
    if !has_column(conn, "warframe_family_progress", "mastered_base") {
        conn.execute(
            "ALTER TABLE warframe_family_progress ADD COLUMN mastered_base INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 mastered_base 失败: {e}"))?;
    }
    if !has_column(conn, "warframe_family_progress", "mastered_prime") {
        conn.execute(
            "ALTER TABLE warframe_family_progress ADD COLUMN mastered_prime INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 mastered_prime 失败: {e}"))?;
    }
    Ok(())
}

/// v2 → v3：武器进度表。
fn migrate_user_to_v3(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS weapon_progress (
  weapon_id  INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  incarnon   INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS weapon_part_progress (
  part_id    INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  have_qty   INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);
"#,
    )
    .map_err(|e| format!("user → v3 建武器进度表失败: {e}"))?;
    Ok(())
}

/// v3 → v4：武器整枪/灵化进度码统一为 0 未获取 / 1 已完成 / 2 进行中。
fn migrate_user_to_v4(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "weapon_progress") {
        return Ok(());
    }
    // status: 旧 3(已有)→1；旧 1(进行中)→2；旧 2(铸造中)→2；0 不变
    // 用临时值避免 1↔2 交叉覆盖
    conn.execute_batch(
        r#"
UPDATE weapon_progress SET status = 90 WHERE status = 1;
UPDATE weapon_progress SET status = 1 WHERE status = 3;
UPDATE weapon_progress SET status = 2 WHERE status = 90 OR status = 2;

UPDATE weapon_progress SET incarnon = 90 WHERE incarnon = 1;
UPDATE weapon_progress SET incarnon = 1 WHERE incarnon = 2;
UPDATE weapon_progress SET incarnon = 2 WHERE incarnon = 90;
"#,
    )
    .map_err(|e| format!("user → v4 武器进度码迁移失败: {e}"))?;
    Ok(())
}

/// v4 → v5：武器心愿；进行中(2)改语义为蓝图已齐，旧进行中清回未获取；部件去掉铸造中。
fn migrate_user_to_v5(conn: &Connection) -> Result<(), String> {
    if has_table(conn, "weapon_progress") && !has_column(conn, "weapon_progress", "wish") {
        conn.execute(
            "ALTER TABLE weapon_progress ADD COLUMN wish INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 weapon_progress.wish 失败: {e}"))?;
    }
    if has_table(conn, "weapon_progress") {
        // 旧「进行中」不再占用 status=2，清回未获取（心愿另记）
        conn.execute("UPDATE weapon_progress SET status = 0 WHERE status = 2", [])
            .map_err(|e| format!("清除旧进行中失败: {e}"))?;
    }
    if has_table(conn, "weapon_part_progress") {
        // 部件旧「铸造中」(2) → 没有
        conn.execute("UPDATE weapon_part_progress SET status = 0 WHERE status = 2", [])
            .map_err(|e| format!("清除部件铸造中失败: {e}"))?;
    }
    Ok(())
}

/// v5 → v6：Mod 进度表。
fn migrate_user_to_v6(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS mod_progress (
  mod_id     INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  wish       INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);
"#,
    )
    .map_err(|e| format!("user → v6 建 mod_progress 失败: {e}"))?;
    Ok(())
}

/// v6 → v7：Mod 用户标签（单选）+ 默认四类。
fn migrate_user_to_v7(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS mod_tag (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_builtin INTEGER NOT NULL DEFAULT 0
);
"#,
    )
    .map_err(|e| format!("user → v7 建 mod_tag 失败: {e}"))?;

    if has_table(conn, "mod_progress") && !has_column(conn, "mod_progress", "tag_id") {
        conn.execute(
            "ALTER TABLE mod_progress ADD COLUMN tag_id INTEGER",
            [],
        )
        .map_err(|e| format!("加列 mod_progress.tag_id 失败: {e}"))?;
    }
    if has_table(conn, "mod_progress") && !has_column(conn, "mod_progress", "tag_manual") {
        conn.execute(
            "ALTER TABLE mod_progress ADD COLUMN tag_manual INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 mod_progress.tag_manual 失败: {e}"))?;
    }

    conn.execute_batch(
        r#"
INSERT OR IGNORE INTO mod_tag (id, name, sort_order, is_builtin) VALUES
  (1, '堕落', 1, 1),
  (2, '噩梦', 2, 1),
  (3, 'Prime版', 3, 1),
  (4, '集团', 4, 1);
"#,
    )
    .map_err(|e| format!("user → v7 种子 mod_tag 失败: {e}"))?;
    Ok(())
}

/// v7 → v8：用户待办。
fn migrate_user_to_v8(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS user_todo (
  id         INTEGER PRIMARY KEY,
  body       TEXT NOT NULL,
  done       INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);
"#,
    )
    .map_err(|e| format!("user → v8 建 user_todo 失败: {e}"))?;
    Ok(())
}

/// v8 → v9：赋能进度。
fn migrate_user_to_v9(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS arcane_progress (
  arcane_id  INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  rank       INTEGER NOT NULL DEFAULT 0,
  wish       INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);
"#,
    )
    .map_err(|e| format!("user → v9 建 arcane_progress 失败: {e}"))?;
    Ok(())
}

/// v9 → v10：战甲心愿分轨 wish_base / wish_prime。
fn migrate_user_to_v10(conn: &Connection) -> Result<(), String> {
    if !has_column(conn, "warframe_family_progress", "wish_base") {
        conn.execute(
            "ALTER TABLE warframe_family_progress ADD COLUMN wish_base INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 wish_base 失败: {e}"))?;
    }
    if !has_column(conn, "warframe_family_progress", "wish_prime") {
        conn.execute(
            "ALTER TABLE warframe_family_progress ADD COLUMN wish_prime INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("加列 wish_prime 失败: {e}"))?;
    }
    // 旧一族一星：迁为双星（无 P 家族的 wish_prime 由读侧按 has_prime 钳制）
    conn.execute(
        "UPDATE warframe_family_progress
         SET wish_base = 1, wish_prime = 1
         WHERE wish = 1 AND wish_base = 0 AND wish_prime = 0",
        [],
    )
    .map_err(|e| format!("迁 wish → wish_base/wish_prime 失败: {e}"))?;
    Ok(())
}

/// v10 → v11：日常 / 周常清单 + 内置种子（无声望）。
fn migrate_user_to_v11(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS routine_item (
  id         INTEGER PRIMARY KEY,
  kind       TEXT NOT NULL,
  title      TEXT NOT NULL,
  is_builtin INTEGER NOT NULL DEFAULT 0,
  ignored    INTEGER NOT NULL DEFAULT 0,
  done       INTEGER NOT NULL DEFAULT 0,
  period_key TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);
"#,
    )
    .map_err(|e| format!("user → v11 建 routine_item 失败: {e}"))?;

    let builtins: &[(&str, &str)] = &[
        ("daily", "突击"),
        ("daily", "午夜电波·日常"),
        ("daily", "钢铁之路侵袭"),
        ("weekly", "午夜电波·周常"),
        ("weekly", "玛鲁·阿耶檀识狩猎"),
        ("weekly", "卡尔周常"),
        ("weekly", "执政官猎杀"),
        ("weekly", "无尽回廊"),
        ("weekly", "钢铁回廊"),
        ("weekly", "六人组日历"),
        ("weekly", "深层科研"),
        ("weekly", "时光科研"),
        ("weekly", "衰退室"),
        ("weekly", "沉沦之地"),
    ];
    for (i, (kind, title)) in builtins.iter().enumerate() {
        conn.execute(
            "INSERT INTO routine_item (kind, title, is_builtin, ignored, done, period_key, sort_order, updated_at)
             VALUES (?1, ?2, 1, 0, 0, NULL, ?3, NULL)",
            rusqlite::params![kind, title, i as i32],
        )
        .map_err(|e| format!("user → v11 种子 routine「{title}」失败: {e}"))?;
    }
    Ok(())
}

/// v11 → v12：执政官狩猎 → 执政官猎杀。
fn migrate_user_to_v12(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE routine_item SET title = '执政官猎杀' WHERE title = '执政官狩猎' AND is_builtin = 1",
        [],
    )
    .map_err(|e| format!("user → v12 改名执政官猎杀失败: {e}"))?;
    Ok(())
}

/// 周常内置默认排序（不含午夜电波·周常，电波项排在最前）。
pub const WEEKLY_BUILTIN_ORDER: &[&str] = &[
    "玛鲁·阿耶檀识狩猎",
    "卡尔周常",
    "执政官猎杀",
    "无尽回廊",
    "钢铁回廊",
    "六人组日历",
    "深层科研",
    "时光科研",
    "衰退室",
    "沉沦之地",
];

/// v12 → v13：周常默认排序。
fn migrate_user_to_v13(conn: &Connection) -> Result<(), String> {
    // 电波周常固定最前；其后按默认清单；其余自定义项追加。
    conn.execute(
        "UPDATE routine_item SET sort_order = 0
         WHERE kind = 'weekly' AND title = '午夜电波·周常'",
        [],
    )
    .map_err(|e| format!("user → v13 电波周常排序失败: {e}"))?;

    for (i, title) in WEEKLY_BUILTIN_ORDER.iter().enumerate() {
        conn.execute(
            "UPDATE routine_item SET sort_order = ?1
             WHERE kind = 'weekly' AND title = ?2",
            rusqlite::params![(i as i32) + 1, *title],
        )
        .map_err(|e| format!("user → v13 排序「{title}」失败: {e}"))?;
    }

    let known: std::collections::HashSet<&str> = WEEKLY_BUILTIN_ORDER
        .iter()
        .copied()
        .chain(std::iter::once("午夜电波·周常"))
        .collect();
    let mut stmt = conn
        .prepare(
            "SELECT id, title FROM routine_item
             WHERE kind = 'weekly'
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| format!("user → v13 读周常失败: {e}"))?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| format!("user → v13 查周常失败: {e}"))?
        .collect::<Result<_, _>>()
        .map_err(|e| format!("user → v13 行周常失败: {e}"))?;
    drop(stmt);

    let mut next = (WEEKLY_BUILTIN_ORDER.len() as i32) + 1;
    for (id, title) in rows {
        if known.contains(title.as_str()) {
            continue;
        }
        conn.execute(
            "UPDATE routine_item SET sort_order = ?1 WHERE id = ?2",
            rusqlite::params![next, id],
        )
        .map_err(|e| format!("user → v13 自定义周常排序失败: {e}"))?;
        next += 1;
    }
    Ok(())
}

/// v13 → v14：建按变体的 `warframe_progress`；旧表数据拷贝见 [`migrate_warframe_family_progress_if_needed`]。
fn migrate_user_to_v14(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
CREATE TABLE IF NOT EXISTS warframe_progress (
  warframe_id INTEGER NOT NULL PRIMARY KEY,
  status      INTEGER NOT NULL DEFAULT 0,
  forma_done  INTEGER NOT NULL DEFAULT 0,
  note        TEXT,
  wish        INTEGER NOT NULL DEFAULT 0,
  mastered    INTEGER NOT NULL DEFAULT 0,
  helminth    INTEGER NOT NULL DEFAULT 0,
  updated_at  TEXT
);
"#,
    )
    .map_err(|e| format!("user → v14 建 warframe_progress 失败: {e}"))?;
    Ok(())
}

/// v14 → v15：武器独立「已掌握」勾选。
fn migrate_user_to_v15(conn: &Connection) -> Result<(), String> {
    if has_table(conn, "weapon_progress") && !has_column(conn, "weapon_progress", "mastered") {
        conn.execute(
            "ALTER TABLE weapon_progress ADD COLUMN mastered INTEGER NOT NULL DEFAULT 0",
            [],
        )
        .map_err(|e| format!("user → v15 加列 weapon_progress.mastered 失败: {e}"))?;
    }
    Ok(())
}

/// v15 → v16：佩里塔叛乱无周限，移出周常内置清单。
fn migrate_user_to_v16(conn: &Connection) -> Result<(), String> {
    if !has_table(conn, "routine_item") {
        return Ok(());
    }
    conn.execute(
        "DELETE FROM routine_item
         WHERE kind = 'weekly' AND title = '佩里塔叛乱' AND is_builtin = 1",
        [],
    )
    .map_err(|e| format!("user → v16 删除佩里塔叛乱失败: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_from_zero_reaches_current() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), 0);
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_table(&conn, "warframe_family_progress"));
        assert!(has_table(&conn, "warframe_progress"));
        assert!(has_column(&conn, "warframe_progress", "helminth"));
        assert!(has_column(
            &conn,
            "warframe_family_progress",
            "mastered_base"
        ));
        assert!(has_table(&conn, "weapon_progress"));
        assert!(has_table(&conn, "weapon_part_progress"));
        assert!(has_column(&conn, "weapon_progress", "wish"));
        assert!(has_column(&conn, "weapon_progress", "mastered"));
        assert!(has_table(&conn, "mod_progress"));
        assert!(has_table(&conn, "mod_tag"));
        assert!(has_column(&conn, "mod_progress", "tag_id"));
        assert!(has_column(&conn, "mod_progress", "tag_manual"));
        assert!(has_table(&conn, "user_todo"));
        assert!(has_column(
            &conn,
            "warframe_family_progress",
            "wish_base"
        ));
        assert!(has_column(
            &conn,
            "warframe_family_progress",
            "wish_prime"
        ));
        assert!(has_table(&conn, "routine_item"));
    }

    #[test]
    fn higher_version_errors() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", USER_SCHEMA_VERSION + 1)
            .unwrap();
        let err = migrate_user_conn(&conn).unwrap_err();
        assert!(err.contains("高于程序支持"), "{err}");
    }

    #[test]
    fn equal_version_noop() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_conn(&conn).unwrap();
        conn.execute(
            "INSERT INTO warframe_family_progress (family_id, base_status) VALUES (1, 3)",
            [],
        )
        .unwrap();
        migrate_user_conn(&conn).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM warframe_family_progress WHERE family_id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn v4_rewrites_weapon_progress_codes() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        conn.execute(
            "INSERT INTO weapon_progress (weapon_id, status, incarnon) VALUES (1, 3, 2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO weapon_progress (weapon_id, status, incarnon) VALUES (2, 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO weapon_progress (weapon_id, status, incarnon) VALUES (3, 2, 0)",
            [],
        )
        .unwrap();
        migrate_user_to_v4(&conn).unwrap();
        let rows: Vec<(i64, i32, i32)> = {
            let mut stmt = conn
                .prepare("SELECT weapon_id, status, incarnon FROM weapon_progress ORDER BY weapon_id")
                .unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert_eq!(rows, vec![(1, 1, 1), (2, 2, 2), (3, 2, 0)]);
    }

    #[test]
    fn v5_adds_wish_and_clears_in_progress() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        conn.pragma_update(None, "user_version", 4).unwrap();
        conn.execute(
            "INSERT INTO weapon_progress (weapon_id, status, incarnon) VALUES (1, 2, 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO weapon_part_progress (part_id, status, have_qty) VALUES (10, 2, 0)",
            [],
        )
        .unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_column(&conn, "weapon_progress", "wish"));
        let st: i32 = conn
            .query_row(
                "SELECT status FROM weapon_progress WHERE weapon_id=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(st, 0);
        let pst: i32 = conn
            .query_row(
                "SELECT status FROM weapon_part_progress WHERE part_id=10",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(pst, 0);
        assert!(has_table(&conn, "mod_progress"));
        assert!(has_table(&conn, "mod_tag"));
        assert!(has_table(&conn, "user_todo"));
    }

    #[test]
    fn v6_adds_mod_progress() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        migrate_user_to_v5(&conn).unwrap();
        conn.pragma_update(None, "user_version", 5).unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_table(&conn, "mod_progress"));
    }

    #[test]
    fn v7_adds_mod_tags() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        migrate_user_to_v5(&conn).unwrap();
        migrate_user_to_v6(&conn).unwrap();
        conn.pragma_update(None, "user_version", 6).unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_table(&conn, "mod_tag"));
        assert!(has_column(&conn, "mod_progress", "tag_id"));
        assert!(has_column(&conn, "mod_progress", "tag_manual"));
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM mod_tag WHERE is_builtin=1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(n, 4);
    }

    #[test]
    fn v8_adds_user_todo() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        migrate_user_to_v5(&conn).unwrap();
        migrate_user_to_v6(&conn).unwrap();
        migrate_user_to_v7(&conn).unwrap();
        conn.pragma_update(None, "user_version", 7).unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_table(&conn, "user_todo"));
    }

    #[test]
    fn v10_splits_warframe_wish() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        migrate_user_to_v5(&conn).unwrap();
        migrate_user_to_v6(&conn).unwrap();
        migrate_user_to_v7(&conn).unwrap();
        migrate_user_to_v8(&conn).unwrap();
        migrate_user_to_v9(&conn).unwrap();
        conn.execute(
            "INSERT INTO warframe_family_progress (family_id, wish) VALUES (1, 1)",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 9).unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        let (wb, wp): (i32, i32) = conn
            .query_row(
                "SELECT wish_base, wish_prime FROM warframe_family_progress WHERE family_id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(wb, 1);
        assert_eq!(wp, 1);
    }

    #[test]
    fn v11_seeds_routine_item() {
        let conn = Connection::open_in_memory().unwrap();
        migrate_user_to_v1(&conn).unwrap();
        migrate_user_to_v2(&conn).unwrap();
        migrate_user_to_v3(&conn).unwrap();
        migrate_user_to_v4(&conn).unwrap();
        migrate_user_to_v5(&conn).unwrap();
        migrate_user_to_v6(&conn).unwrap();
        migrate_user_to_v7(&conn).unwrap();
        migrate_user_to_v8(&conn).unwrap();
        migrate_user_to_v9(&conn).unwrap();
        migrate_user_to_v10(&conn).unwrap();
        conn.pragma_update(None, "user_version", 10).unwrap();
        migrate_user_conn(&conn).unwrap();
        assert_eq!(read_user_version(&conn).unwrap(), USER_SCHEMA_VERSION);
        assert!(has_table(&conn, "routine_item"));
        let daily: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM routine_item WHERE kind='daily' AND is_builtin=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let weekly: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM routine_item WHERE kind='weekly' AND is_builtin=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(daily, 3);
        assert_eq!(weekly, 11);
    }
}
