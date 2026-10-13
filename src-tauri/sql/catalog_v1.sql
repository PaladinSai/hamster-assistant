-- schema_version: 22
-- catalog.sqlite · 官方目录种子快照（新库 v0→v1 使用；升级走 catalog_migrate 逐步步骤）

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS warframe_family (
  id           INTEGER NOT NULL PRIMARY KEY,
  family_key   TEXT    NOT NULL UNIQUE,
  name_en      TEXT    NOT NULL,
  name_zh      TEXT,
  nickname     TEXT,
  vault_status INTEGER NOT NULL DEFAULT 0,
  -- warframe | archwing | necramech | companion
  suit_kind    TEXT    NOT NULL DEFAULT 'warframe',
  -- 同伴细分：sentinel | kubrow | kavat | vulpaphyla | predasite | moa | hound；非同伴为 NULL
  companion_type TEXT,
  created_at   TEXT,
  updated_at   TEXT
);

CREATE TABLE IF NOT EXISTS warframe (
  id           INTEGER NOT NULL PRIMARY KEY,
  family_id    INTEGER NOT NULL REFERENCES warframe_family(id),
  variant      INTEGER NOT NULL,
  name_en      TEXT,
  name_zh      TEXT,
  unique_name  TEXT,
  vault_status INTEGER NOT NULL DEFAULT 0,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  UNIQUE (family_id, variant)
);

CREATE INDEX IF NOT EXISTS idx_warframe_family ON warframe(family_id);

CREATE TABLE IF NOT EXISTS warframe_part (
  id          INTEGER NOT NULL PRIMARY KEY,
  warframe_id INTEGER NOT NULL REFERENCES warframe(id),
  part_key    TEXT    NOT NULL,
  name_zh     TEXT    NOT NULL,
  sort_order  INTEGER NOT NULL DEFAULT 0,
  UNIQUE (warframe_id, part_key)
);

CREATE INDEX IF NOT EXISTS idx_part_wf ON warframe_part(warframe_id);

CREATE TABLE IF NOT EXISTS void_relic (
  id          INTEGER NOT NULL PRIMARY KEY,
  relic_key   TEXT    NOT NULL UNIQUE,
  era         INTEGER NOT NULL,
  code        TEXT    NOT NULL,
  name_zh     TEXT    NOT NULL,
  vaulted     INTEGER NOT NULL DEFAULT 0,
  unique_name TEXT
);

CREATE INDEX IF NOT EXISTS idx_void_relic_era ON void_relic(era, code);

CREATE TABLE IF NOT EXISTS warframe_part_relic (
  id         INTEGER NOT NULL PRIMARY KEY,
  part_id    INTEGER NOT NULL REFERENCES warframe_part(id),
  relic_id   INTEGER NOT NULL REFERENCES void_relic(id),
  rarity     INTEGER,
  sort_order INTEGER NOT NULL DEFAULT 0,
  UNIQUE (part_id, relic_id)
);

CREATE INDEX IF NOT EXISTS idx_part_relic_part ON warframe_part_relic(part_id);
CREATE INDEX IF NOT EXISTS idx_part_relic_relic ON warframe_part_relic(relic_id);

-- 双译名：展示 COALESCE(wfcd, userdict)；实体表 name_zh 导入时写入解析结果
CREATE TABLE IF NOT EXISTS name_i18n (
  name_en           TEXT NOT NULL PRIMARY KEY,
  name_zh_userdict  TEXT,
  name_zh_wfcd      TEXT,
  category          TEXT,
  updated_at        TEXT
);

CREATE TABLE IF NOT EXISTS setting (
  key   TEXT NOT NULL PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS resurgence_rotation (
  id          INTEGER NOT NULL PRIMARY KEY,
  activation  TEXT,
  expiry      TEXT,
  updated_at  TEXT,
  source_url  TEXT
);

CREATE TABLE IF NOT EXISTS resurgence_item (
  id           INTEGER NOT NULL PRIMARY KEY,
  rotation_id  INTEGER NOT NULL REFERENCES resurgence_rotation(id) ON DELETE CASCADE,
  kind         TEXT    NOT NULL,
  name_en      TEXT    NOT NULL,
  name_zh      TEXT,
  unique_name  TEXT,
  family_key   TEXT,
  relic_key    TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_resurgence_item_rot ON resurgence_item(rotation_id);
CREATE INDEX IF NOT EXISTS idx_resurgence_item_kind ON resurgence_item(kind);

CREATE TABLE IF NOT EXISTS relic_reward (
  id            INTEGER NOT NULL PRIMARY KEY,
  relic_id      INTEGER NOT NULL REFERENCES void_relic(id) ON DELETE CASCADE,
  rarity        INTEGER,
  item_name_en  TEXT    NOT NULL,
  item_name_zh  TEXT,
  unique_name   TEXT,
  sort_order    INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_relic_reward_relic ON relic_reward(relic_id);
CREATE INDEX IF NOT EXISTS idx_relic_reward_item ON relic_reward(item_name_en);
CREATE INDEX IF NOT EXISTS idx_relic_reward_item_zh ON relic_reward(item_name_zh);

CREATE TABLE IF NOT EXISTS relic_location (
  id                 INTEGER NOT NULL PRIMARY KEY,
  relic_id           INTEGER NOT NULL REFERENCES void_relic(id) ON DELETE CASCADE,
  source_kind        TEXT    NOT NULL,
  planet_or_hub      TEXT    NOT NULL,
  planet_or_hub_zh   TEXT,
  node_or_bounty     TEXT    NOT NULL,
  node_or_bounty_zh  TEXT,
  game_mode          TEXT,
  game_mode_zh       TEXT,
  rotation           TEXT,
  chance             REAL,
  stage              TEXT,
  is_event           INTEGER NOT NULL DEFAULT 0,
  sort_order         INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_relic_loc_relic ON relic_location(relic_id);
CREATE INDEX IF NOT EXISTS idx_relic_loc_hub ON relic_location(planet_or_hub);
CREATE INDEX IF NOT EXISTS idx_relic_loc_kind ON relic_location(source_kind);

-- —— 武器 ——
CREATE TABLE IF NOT EXISTS weapon_family (
  id         INTEGER NOT NULL PRIMARY KEY,
  family_key TEXT    NOT NULL UNIQUE,
  name_en    TEXT    NOT NULL,
  name_zh    TEXT,
  nickname   TEXT,
  -- 0 主 / 1 副 / 2 近战 / 3 自制（精通部件）
  slot       INTEGER NOT NULL,
  -- weapon | archwing
  weapon_kind TEXT   NOT NULL DEFAULT 'weapon',
  -- kitgun | zaw | amp；仅 slot=3 自制
  modular_kind TEXT,
  created_at TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS weapon (
  id           INTEGER NOT NULL PRIMARY KEY,
  family_id    INTEGER NOT NULL REFERENCES weapon_family(id),
  variant      INTEGER NOT NULL,
  name_en      TEXT,
  name_zh      TEXT,
  unique_name  TEXT,
  can_incarnon INTEGER NOT NULL DEFAULT 0,
  vault_status INTEGER NOT NULL DEFAULT 0,
  sort_order   INTEGER NOT NULL DEFAULT 0,
  UNIQUE (family_id, variant)
);

CREATE INDEX IF NOT EXISTS idx_weapon_family ON weapon(family_id);
CREATE INDEX IF NOT EXISTS idx_weapon_unique ON weapon(unique_name);

CREATE TABLE IF NOT EXISTS weapon_part (
  id         INTEGER NOT NULL PRIMARY KEY,
  weapon_id  INTEGER NOT NULL REFERENCES weapon(id),
  part_key   TEXT    NOT NULL,
  name_zh    TEXT    NOT NULL,
  need_qty   INTEGER NOT NULL DEFAULT 1,
  sort_order INTEGER NOT NULL DEFAULT 0,
  UNIQUE (weapon_id, part_key)
);

CREATE INDEX IF NOT EXISTS idx_weapon_part_wep ON weapon_part(weapon_id);

CREATE TABLE IF NOT EXISTS weapon_part_relic (
  id         INTEGER NOT NULL PRIMARY KEY,
  part_id    INTEGER NOT NULL REFERENCES weapon_part(id),
  relic_id   INTEGER NOT NULL REFERENCES void_relic(id),
  rarity     INTEGER,
  sort_order INTEGER NOT NULL DEFAULT 0,
  UNIQUE (part_id, relic_id)
);

CREATE INDEX IF NOT EXISTS idx_weapon_part_relic_part ON weapon_part_relic(part_id);
CREATE INDEX IF NOT EXISTS idx_weapon_part_relic_relic ON weapon_part_relic(relic_id);

-- —— Mod ——
CREATE TABLE IF NOT EXISTS mod (
  id           INTEGER NOT NULL PRIMARY KEY,
  unique_name  TEXT    NOT NULL UNIQUE,
  name_en      TEXT    NOT NULL,
  name_zh      TEXT,
  mod_kind     INTEGER NOT NULL DEFAULT 7,
  source_kind  INTEGER NOT NULL DEFAULT 0,
  is_prime     INTEGER NOT NULL DEFAULT 0,
  is_augment   INTEGER NOT NULL DEFAULT 0,
  polarity     TEXT,
  sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_mod_source ON mod(source_kind);
CREATE INDEX IF NOT EXISTS idx_mod_kind ON mod(mod_kind);
CREATE INDEX IF NOT EXISTS idx_mod_name ON mod(name_en);

-- —— 赋能 Arcane ——
CREATE TABLE IF NOT EXISTS arcane (
  id           INTEGER NOT NULL PRIMARY KEY,
  unique_name  TEXT    NOT NULL UNIQUE,
  name_en      TEXT    NOT NULL,
  name_zh      TEXT,
  arcane_kind  INTEGER NOT NULL DEFAULT 8,
  rarity       INTEGER NOT NULL DEFAULT 0,
  source_kind  INTEGER NOT NULL DEFAULT 0,
  max_rank     INTEGER NOT NULL DEFAULT 5,
  sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_arcane_source ON arcane(source_kind);
CREATE INDEX IF NOT EXISTS idx_arcane_kind ON arcane(arcane_kind);
CREATE INDEX IF NOT EXISTS idx_arcane_rarity ON arcane(rarity);
CREATE INDEX IF NOT EXISTS idx_arcane_name ON arcane(name_en);

-- —— 世界状态日常（突击 / 电波 / 钢铁侵袭计时）——
CREATE TABLE IF NOT EXISTS live_sortie (
  id          INTEGER NOT NULL PRIMARY KEY,
  activation  TEXT,
  expiry      TEXT,
  boss        TEXT,
  faction     TEXT,
  fetched_at  TEXT,
  source_url  TEXT
);

CREATE TABLE IF NOT EXISTS live_sortie_mission (
  id            INTEGER NOT NULL PRIMARY KEY,
  sortie_id     INTEGER NOT NULL REFERENCES live_sortie(id) ON DELETE CASCADE,
  sort_order    INTEGER NOT NULL DEFAULT 0,
  mission_type  TEXT,
  modifier      TEXT,
  node          TEXT
);

CREATE INDEX IF NOT EXISTS idx_live_sortie_mission ON live_sortie_mission(sortie_id, sort_order);

CREATE TABLE IF NOT EXISTS live_nightwave_challenge (
  id             INTEGER NOT NULL PRIMARY KEY,
  challenge_key  TEXT    NOT NULL UNIQUE,
  activation     TEXT,
  expiry         TEXT,
  is_daily       INTEGER NOT NULL DEFAULT 0,
  is_elite       INTEGER NOT NULL DEFAULT 0,
  title          TEXT,
  description    TEXT,
  reputation     INTEGER,
  fetched_at     TEXT
);

CREATE TABLE IF NOT EXISTS live_steel_incursion (
  id            INTEGER NOT NULL PRIMARY KEY,
  activation    TEXT,
  expiry        TEXT,
  fetched_at    TEXT,
  source_url    TEXT,
  today_json    TEXT NOT NULL DEFAULT '[]',
  tomorrow_json TEXT NOT NULL DEFAULT '[]'
);

-- 虚空裂隙：普通 / 钢铁（ActiveMissions）+ 九重天风暴（VoidStorms）
CREATE TABLE IF NOT EXISTS live_fissure (
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
  ON live_fissure(kind, sort_order);

-- —— 赏金（正规表，不再用 setting JSON）——
CREATE TABLE IF NOT EXISTS live_bounty_board (
  tag         TEXT NOT NULL PRIMARY KEY,
  name        TEXT NOT NULL,
  activation  TEXT,
  expiry      TEXT,
  fetched_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS live_bounty_job (
  id               INTEGER NOT NULL PRIMARY KEY,
  board_tag        TEXT    NOT NULL REFERENCES live_bounty_board(tag) ON DELETE CASCADE,
  sort_order       INTEGER NOT NULL DEFAULT 0,
  job_type         TEXT    NOT NULL,
  kind             TEXT    NOT NULL,
  kind_zh          TEXT    NOT NULL,
  title            TEXT    NOT NULL,
  min_enemy_level  INTEGER NOT NULL DEFAULT 0,
  max_enemy_level  INTEGER NOT NULL DEFAULT 0,
  standing_total   INTEGER NOT NULL DEFAULT 0,
  is_advanced      INTEGER NOT NULL DEFAULT 0,
  is_steel_path    INTEGER NOT NULL DEFAULT 0,
  challenge_zh     TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_live_bounty_job_board
  ON live_bounty_job(board_tag, sort_order);

CREATE TABLE IF NOT EXISTS live_bounty_job_reward (
  job_id      INTEGER NOT NULL REFERENCES live_bounty_job(id) ON DELETE CASCADE,
  sort_order  INTEGER NOT NULL,
  name        TEXT    NOT NULL,
  PRIMARY KEY (job_id, sort_order)
);

-- —— 周常（表头可查询；嵌套列表用列内 JSON，仅后端读写）——
CREATE TABLE IF NOT EXISTS live_archon_hunt (
  id          INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
  activation  TEXT,
  expiry      TEXT,
  boss        TEXT    NOT NULL DEFAULT '',
  shard       TEXT    NOT NULL DEFAULT '',
  fetched_at  TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS live_archon_mission (
  id            INTEGER NOT NULL PRIMARY KEY,
  hunt_id       INTEGER NOT NULL REFERENCES live_archon_hunt(id) ON DELETE CASCADE,
  sort_order    INTEGER NOT NULL DEFAULT 0,
  mission_type  TEXT    NOT NULL,
  node          TEXT    NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_archon_mission
  ON live_archon_mission(hunt_id, sort_order);

CREATE TABLE IF NOT EXISTS live_archimedea (
  kind                     TEXT NOT NULL PRIMARY KEY,
  activation               TEXT,
  expiry                   TEXT,
  kind_label               TEXT NOT NULL,
  personal_modifiers_json  TEXT NOT NULL DEFAULT '[]',
  fetched_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS live_archimedea_mission (
  id                     INTEGER NOT NULL PRIMARY KEY,
  archimedea_kind        TEXT    NOT NULL REFERENCES live_archimedea(kind) ON DELETE CASCADE,
  sort_order             INTEGER NOT NULL DEFAULT 0,
  mission_type           TEXT    NOT NULL,
  faction                TEXT    NOT NULL,
  deviation_title        TEXT    NOT NULL DEFAULT '',
  deviation_description  TEXT    NOT NULL DEFAULT '',
  risks_normal_json      TEXT    NOT NULL DEFAULT '[]',
  risks_elite_json       TEXT    NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_live_archimedea_mission
  ON live_archimedea_mission(archimedea_kind, sort_order);

CREATE TABLE IF NOT EXISTS live_circuit (
  kind                 TEXT NOT NULL PRIMARY KEY,
  activation           TEXT,
  expiry               TEXT,
  this_week_json       TEXT NOT NULL DEFAULT '[]',
  next_week_json       TEXT NOT NULL DEFAULT '[]',
  rotation_weeks_json  TEXT NOT NULL DEFAULT '[]',
  current_week_index   INTEGER,
  fetched_at           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS live_hex_calendar (
  id          INTEGER NOT NULL PRIMARY KEY CHECK (id = 1),
  activation  TEXT,
  expiry      TEXT,
  fetched_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS live_hex_calendar_day (
  id           INTEGER NOT NULL PRIMARY KEY,
  calendar_id  INTEGER NOT NULL REFERENCES live_hex_calendar(id) ON DELETE CASCADE,
  day          INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_live_hex_day
  ON live_hex_calendar_day(calendar_id, day);

CREATE TABLE IF NOT EXISTS live_hex_calendar_group (
  id            INTEGER NOT NULL PRIMARY KEY,
  day_id        INTEGER NOT NULL REFERENCES live_hex_calendar_day(id) ON DELETE CASCADE,
  sort_order    INTEGER NOT NULL DEFAULT 0,
  kind          TEXT    NOT NULL,
  choices_json  TEXT    NOT NULL DEFAULT '[]'
);

CREATE INDEX IF NOT EXISTS idx_live_hex_group
  ON live_hex_calendar_group(day_id, sort_order);
