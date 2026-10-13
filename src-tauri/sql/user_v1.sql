-- schema_version: 15
-- user.sqlite · 用户进度与设置（全量 DDL 快照，供种子/文档）
-- 运行时：无库新建；低版本逐步增量（user_migrate）；相等不动；高于程序则报错。
-- 禁止因 schema 过旧删除已有 user.sqlite。

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

CREATE TABLE IF NOT EXISTS warframe_part_progress (
  part_id    INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS weapon_progress (
  weapon_id  INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  incarnon   INTEGER NOT NULL DEFAULT 0,
  wish       INTEGER NOT NULL DEFAULT 0,
  mastered   INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS weapon_part_progress (
  part_id    INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  have_qty   INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS mod_tag (
  id         INTEGER PRIMARY KEY,
  name       TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0,
  is_builtin INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS mod_progress (
  mod_id     INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  wish       INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  tag_id     INTEGER,
  tag_manual INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS arcane_progress (
  arcane_id  INTEGER NOT NULL PRIMARY KEY,
  status     INTEGER NOT NULL DEFAULT 0,
  rank       INTEGER NOT NULL DEFAULT 0,
  wish       INTEGER NOT NULL DEFAULT 0,
  note       TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS user_todo (
  id         INTEGER PRIMARY KEY,
  body       TEXT NOT NULL,
  done       INTEGER NOT NULL DEFAULT 0,
  sort_order INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS setting (
  key   TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL
);

INSERT OR IGNORE INTO mod_tag (id, name, sort_order, is_builtin) VALUES
  (1, '堕落', 1, 1),
  (2, '噩梦', 2, 1),
  (3, 'Prime版', 3, 1),
  (4, '集团', 4, 1);
