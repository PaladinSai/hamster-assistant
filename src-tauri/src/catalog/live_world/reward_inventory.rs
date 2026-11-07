//! 声望奖励里的 MOD / 蓝图对照玩家库存。其它掉落不进这张表。

use std::collections::HashMap;

use rusqlite::Connection;

/// 展示名 → 库存状态文案。只含 MOD 和蓝图。同名多件且状态不一致的名字会被丢掉。
pub(crate) fn reward_mark_index(
    catalog: &Connection,
    user: &Connection,
) -> Result<HashMap<String, String>, String> {
    let mut raw: HashMap<String, Option<String>> = HashMap::new();
    index_mods(catalog, user, &mut raw)?;
    index_warframe_blueprints(catalog, user, &mut raw)?;
    index_weapon_blueprints(catalog, user, &mut raw)?;
    Ok(raw
        .into_iter()
        .filter_map(|(label, status)| status.map(|status| (label, status)))
        .collect())
}

pub(crate) fn marks_for_labels(
    index: &HashMap<String, String>,
    labels: impl IntoIterator<Item = impl AsRef<str>>,
) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for label in labels {
        let label = label.as_ref();
        if let Some(status) = index.get(label) {
            out.insert(label.to_string(), status.clone());
        }
    }
    out
}

fn note(index: &mut HashMap<String, Option<String>>, label: String, status: &str) {
    let label = label.trim();
    if label.is_empty() {
        return;
    }
    match index.get(label) {
        Some(None) => {}
        Some(Some(prev)) if prev == status => {}
        Some(Some(_)) => {
            index.insert(label.to_string(), None);
        }
        None => {
            index.insert(label.to_string(), Some(status.to_string()));
        }
    }
}

fn display_name(name_zh: String, name_en: String) -> String {
    let zh = name_zh.trim();
    if zh.is_empty() {
        name_en.trim().to_string()
    } else {
        zh.to_string()
    }
}

fn mod_status_label(status: i32) -> &'static str {
    if status == 1 { "已获取" } else { "未获取" }
}

fn part_status_label(status: i32) -> &'static str {
    match status {
        1 => "有蓝图",
        2 => "铸造中",
        3 => "已获取",
        _ => "未获取",
    }
}

fn load_status(conn: &Connection, sql: &str) -> Result<HashMap<i64, i32>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i32>(1)?)))
        .map_err(|e| e.to_string())?;
    let mut map = HashMap::new();
    for row in rows {
        let (id, status) = row.map_err(|e| e.to_string())?;
        map.insert(id, status);
    }
    Ok(map)
}

fn index_mods(
    catalog: &Connection,
    user: &Connection,
    index: &mut HashMap<String, Option<String>>,
) -> Result<(), String> {
    let progress = load_status(user, "SELECT mod_id, status FROM mod_progress")?;
    let mut stmt = catalog
        .prepare("SELECT id, name_zh, name_en, unique_name FROM mod")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?.unwrap_or_default(),
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (id, name_zh, name_en, unique_name) = row.map_err(|e| e.to_string())?;
        let status = mod_status_label(progress.get(&id).copied().unwrap_or(0));
        for label in mod_labels(&name_zh, &name_en, &unique_name) {
            note(index, label, status);
        }
    }
    Ok(())
}

fn index_warframe_blueprints(
    catalog: &Connection,
    user: &Connection,
    index: &mut HashMap<String, Option<String>>,
) -> Result<(), String> {
    let progress = load_status(user, "SELECT part_id, status FROM warframe_part_progress")?;
    let mut stmt = catalog
        .prepare(
            "SELECT wp.id, wp.part_key, wp.name_zh, COALESCE(wp.unique_name, ''),
                    COALESCE(wf.name_zh, ''), COALESCE(wf.name_en, '')
             FROM warframe_part wp
             JOIN warframe wf ON wf.id = wp.warframe_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                display_name(row.get(4)?, row.get(5)?),
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (id, part_key, part_name, unique_name, parent) = row.map_err(|e| e.to_string())?;
        if parent.is_empty() {
            continue;
        }
        let status = part_status_label(progress.get(&id).copied().unwrap_or(0));
        for label in warframe_blueprint_labels(&parent, &part_key, &part_name, &unique_name) {
            note(index, label, status);
        }
    }
    Ok(())
}

fn index_weapon_blueprints(
    catalog: &Connection,
    user: &Connection,
    index: &mut HashMap<String, Option<String>>,
) -> Result<(), String> {
    let progress = load_status(user, "SELECT part_id, status FROM weapon_part_progress")?;
    let mut stmt = catalog
        .prepare(
            "SELECT wp.id, wp.part_key, wp.name_zh, COALESCE(wp.unique_name, ''),
                    COALESCE(w.name_zh, ''), COALESCE(w.name_en, '')
             FROM weapon_part wp
             JOIN weapon w ON w.id = wp.weapon_id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                display_name(row.get(4)?, row.get(5)?),
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (id, part_key, part_name, unique_name, parent) = row.map_err(|e| e.to_string())?;
        if parent.is_empty() {
            continue;
        }
        let status = part_status_label(progress.get(&id).copied().unwrap_or(0));
        for label in weapon_blueprint_labels(&parent, &part_key, &part_name, &unique_name) {
            note(index, label, status);
        }
    }
    Ok(())
}

fn mod_labels(name_zh: &str, name_en: &str, unique_name: &str) -> Vec<String> {
    let mut out = Vec::new();
    add_alias(&mut out, name_zh.trim().to_string());
    add_alias(&mut out, name_en.trim().to_string());
    add_alias(&mut out, unique_name.trim().to_string());
    out
}

fn add_alias(out: &mut Vec<String>, label: String) {
    if !label.is_empty() && !out.iter().any(|item| item == &label) {
        out.push(label);
    }
}

/// 带空格和不带空格都登记，奖励文案两种写法都能对上同一个部件。
fn add_name_forms(out: &mut Vec<String>, parent: &str, suffix: &str) {
    add_alias(out, format!("{parent} {suffix}"));
    add_alias(out, format!("{parent}{suffix}"));
}

fn warframe_blueprint_labels(
    parent: &str,
    part_key: &str,
    part_name: &str,
    unique_name: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    match part_key {
        "chassis" => add_name_forms(&mut out, parent, "机体蓝图"),
        "neuroptics" => {
            add_name_forms(&mut out, parent, "神经光元蓝图");
            add_name_forms(&mut out, parent, "头部神经光元蓝图");
        }
        "systems" => add_name_forms(&mut out, parent, "系统蓝图"),
        "blueprint" => add_name_forms(&mut out, parent, "蓝图"),
        _ => {}
    }
    let part_name = part_name.trim();
    if part_name.ends_with("蓝图") {
        add_name_forms(&mut out, parent, part_name);
    }
    if !out.is_empty() {
        add_alias(&mut out, unique_name.trim().to_string());
    }
    out
}

fn weapon_blueprint_labels(
    parent: &str,
    part_key: &str,
    part_name: &str,
    unique_name: &str,
) -> Vec<String> {
    let mut out = Vec::new();
    let part_name = part_name.trim();
    if part_key == "blueprint" || part_name == "蓝图" {
        add_name_forms(&mut out, parent, "蓝图");
    }
    if part_name.ends_with("蓝图") && part_name != "蓝图" {
        add_name_forms(&mut out, parent, part_name);
    }
    if !out.is_empty() {
        add_alias(&mut out, unique_name.trim().to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> (Connection, Connection) {
        let catalog = Connection::open_in_memory().unwrap();
        let user = Connection::open_in_memory().unwrap();
        catalog
            .execute_batch(
                "CREATE TABLE mod (
                    id INTEGER PRIMARY KEY,
                    name_zh TEXT,
                    name_en TEXT NOT NULL,
                    unique_name TEXT
                 );
                 CREATE TABLE warframe (
                    id INTEGER PRIMARY KEY,
                    name_zh TEXT,
                    name_en TEXT
                 );
                 CREATE TABLE warframe_part (
                    id INTEGER PRIMARY KEY,
                    warframe_id INTEGER,
                    part_key TEXT,
                    name_zh TEXT,
                    unique_name TEXT
                 );
                 CREATE TABLE weapon (
                    id INTEGER PRIMARY KEY,
                    name_zh TEXT,
                    name_en TEXT
                 );
                 CREATE TABLE weapon_part (
                    id INTEGER PRIMARY KEY,
                    weapon_id INTEGER,
                    part_key TEXT,
                    name_zh TEXT,
                    unique_name TEXT
                 );",
            )
            .unwrap();
        user.execute_batch(
            "CREATE TABLE mod_progress (mod_id INTEGER PRIMARY KEY, status INTEGER);
             CREATE TABLE warframe_part_progress (part_id INTEGER PRIMARY KEY, status INTEGER);
             CREATE TABLE weapon_part_progress (part_id INTEGER PRIMARY KEY, status INTEGER);",
        )
        .unwrap();
        (catalog, user)
    }

    #[test]
    fn mods_blueprints_and_other_rewards() {
        let (catalog, user) = schema();
        catalog
            .execute_batch(
                "INSERT INTO mod (id, name_zh, name_en) VALUES
                    (1, '致命一击', 'Pressure Point'),
                    (2, '异况超量', 'Condition Overload');
                 INSERT INTO warframe (id, name_zh, name_en) VALUES
                    (10, '迦楼罗', 'Gara'),
                    (11, '迦楼罗', 'Gara Prime'),
                    (12, '沃班', 'Wukong');
                 INSERT INTO warframe_part (id, warframe_id, part_key, name_zh) VALUES
                    (101, 10, 'chassis', '机体 Chassis'),
                    (102, 10, 'neuroptics', '头部 Neuroptics'),
                    (103, 10, 'systems', '系统 Systems'),
                    (104, 10, 'blueprint', '总蓝图'),
                    (105, 11, 'chassis', '机体 Chassis'),
                    (106, 12, 'chassis', '机体 Chassis');
                 INSERT INTO weapon (id, name_zh, name_en) VALUES (20, '格拉姆', 'Gram');
                 INSERT INTO weapon_part (id, weapon_id, part_key, name_zh) VALUES
                    (201, 20, 'blueprint', '蓝图'),
                    (202, 20, 'blade', '刀刃');",
            )
            .unwrap();
        user.execute_batch(
            "INSERT INTO mod_progress (mod_id, status) VALUES (1, 1);
             INSERT INTO warframe_part_progress (part_id, status) VALUES
                (101, 1),
                (102, 2),
                (103, 3),
                (106, 1);
             INSERT INTO weapon_part_progress (part_id, status) VALUES (201, 0);",
        )
        .unwrap();

        let index = reward_mark_index(&catalog, &user).unwrap();
        assert_eq!(index.get("致命一击").map(String::as_str), Some("已获取"));
        assert_eq!(index.get("异况超量").map(String::as_str), Some("未获取"));
        assert_eq!(index.get("迦楼罗 机体蓝图"), None);
        assert_eq!(
            index.get("沃班 机体蓝图").map(String::as_str),
            Some("有蓝图")
        );
        assert_eq!(
            index.get("迦楼罗 神经光元蓝图").map(String::as_str),
            Some("铸造中")
        );
        assert_eq!(
            index.get("迦楼罗 系统蓝图").map(String::as_str),
            Some("已获取")
        );
        assert_eq!(index.get("迦楼罗 蓝图").map(String::as_str), Some("未获取"));
        assert_eq!(index.get("格拉姆 蓝图").map(String::as_str), Some("未获取"));
        assert!(!index.contains_key("格拉姆 刀刃"));
        assert!(!index.contains_key("内融核心"));

        let marks = marks_for_labels(
            &index,
            ["致命一击", "内融核心", "迦楼罗 系统蓝图"],
        );
        assert_eq!(marks.get("致命一击").map(String::as_str), Some("已获取"));
        assert_eq!(marks.get("迦楼罗 系统蓝图").map(String::as_str), Some("已获取"));
        assert!(!marks.contains_key("内融核心"));
    }

    #[test]
    fn part_status_zero_and_missing_are_unowned() {
        let (catalog, user) = schema();
        catalog
            .execute(
                "INSERT INTO warframe (id, name_zh, name_en) VALUES (1, '希芙', 'Saryn')",
                [],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_part (id, warframe_id, part_key, name_zh)
                 VALUES (1, 1, 'chassis', '机体 Chassis'), (2, 1, 'systems', '系统 Systems')",
                [],
            )
            .unwrap();
        user.execute(
            "INSERT INTO warframe_part_progress (part_id, status) VALUES (1, 0)",
            [],
        )
        .unwrap();
        let index = reward_mark_index(&catalog, &user).unwrap();
        assert_eq!(index.get("希芙 机体蓝图").map(String::as_str), Some("未获取"));
        assert_eq!(index.get("希芙 系统蓝图").map(String::as_str), Some("未获取"));
    }

    #[test]
    fn nightcap_reward_wording_uses_part_unique_name() {
        let (catalog, user) = schema();
        catalog
            .execute_batch(
                "INSERT INTO warframe (id, name_zh, name_en) VALUES (1, 'Nokko', 'Nokko');
                 INSERT INTO warframe_part (id, warframe_id, part_key, name_zh, unique_name)
                 VALUES (1, 1, 'neuroptics', '头部 Neuroptics',
                         '/Lotus/Types/Recipes/WarframeRecipes/NokkoHelmetComponent');
                 INSERT INTO weapon (id, name_zh, name_en) VALUES (2, '蕈菇', 'Arbucep');
                 INSERT INTO weapon_part (id, weapon_id, part_key, name_zh, unique_name)
                 VALUES (2, 2, 'blueprint', '蓝图',
                         '/Lotus/Types/Recipes/Weapons/NokkoArchGunBlueprint');",
            )
            .unwrap();
        user.execute_batch(
            "INSERT INTO warframe_part_progress (part_id, status) VALUES (1, 1);
             INSERT INTO weapon_part_progress (part_id, status) VALUES (2, 3);",
        )
        .unwrap();
        let index = reward_mark_index(&catalog, &user).unwrap();
        assert_eq!(
            index.get("Nokko 头部神经光元蓝图").map(String::as_str),
            Some("有蓝图")
        );
        assert_eq!(
            index
                .get("/Lotus/Types/Recipes/WarframeRecipes/NokkoHelmetComponent")
                .map(String::as_str),
            Some("有蓝图")
        );
        assert_eq!(index.get("蕈菇蓝图").map(String::as_str), Some("已获取"));
        assert_eq!(index.get("蕈菇 蓝图").map(String::as_str), Some("已获取"));
    }
}
