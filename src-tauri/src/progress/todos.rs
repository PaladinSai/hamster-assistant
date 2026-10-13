//! 用户待办。

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::{self, open_user};
use super::common::{id_to_str, parse_id};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTodoDto {
    pub id: String,
    pub body: String,
    pub done: bool,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserTodo {
    pub id: String,
    pub body: Option<String>,
    pub done: Option<bool>,
    pub sort_order: Option<i32>,
}

pub fn list_user_todos(data_dir: &Path) -> Result<Vec<UserTodoDto>, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let mut stmt = user
        .prepare(
            "SELECT id, body, done, sort_order FROM user_todo
             ORDER BY done ASC, sort_order ASC, id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(UserTodoDto {
                id: id_to_str(row.get::<_, i64>(0)?),
                body: row.get(1)?,
                done: row.get::<_, i32>(2)? != 0,
                sort_order: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r.map_err(|e| e.to_string())?);
    }
    Ok(out)
}

pub fn create_user_todo(data_dir: &Path, body: &str) -> Result<UserTodoDto, String> {
    let body = body.trim();
    if body.is_empty() {
        return Err("待办内容不能为空".into());
    }
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();
    let next_order: i32 = user
        .query_row(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM user_todo",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    user.execute(
        "INSERT INTO user_todo (body, done, sort_order, updated_at) VALUES (?1, 0, ?2, ?3)",
        params![body, next_order, now],
    )
    .map_err(|e| e.to_string())?;
    let id: i64 = user
        .query_row("SELECT last_insert_rowid()", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(UserTodoDto {
        id: id_to_str(id),
        body: body.to_string(),
        done: false,
        sort_order: next_order,
    })
}

pub fn update_user_todo(data_dir: &Path, patch: UpdateUserTodo) -> Result<UserTodoDto, String> {
    let id = parse_id(&patch.id, "待办 id")?;
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();

    let existing = user
        .query_row(
            "SELECT body, done, sort_order FROM user_todo WHERE id = ?1",
            [id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i32>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("待办 id={id} 不存在"))?;

    let body = patch
        .body
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or(existing.0);
    let done = patch.done.map(|d| if d { 1 } else { 0 }).unwrap_or(existing.1);
    let sort_order = patch.sort_order.unwrap_or(existing.2);

    user.execute(
        "UPDATE user_todo SET body=?1, done=?2, sort_order=?3, updated_at=?4 WHERE id=?5",
        params![body, done, sort_order, now, id],
    )
    .map_err(|e| e.to_string())?;

    Ok(UserTodoDto {
        id: id_to_str(id),
        body,
        done: done != 0,
        sort_order,
    })
}

pub fn delete_user_todo(data_dir: &Path, id: &str) -> Result<(), String> {
    let id = parse_id(id, "待办 id")?;
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let n = user
        .execute("DELETE FROM user_todo WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err(format!("待办 id={id} 不存在"));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderUserTodos {
    pub ids: Vec<String>,
}

/// 同一完成状态下的完整有序列表；写回 sort_order = 0..n-1。
pub fn reorder_user_todos(data_dir: &Path, input: ReorderUserTodos) -> Result<(), String> {
    if input.ids.is_empty() {
        return Err("排序列表不能为空".into());
    }
    db::init_databases(data_dir)?;
    let mut user = open_user(&db::user_path(data_dir))?;
    let now = Utc::now().to_rfc3339();

    let mut seen = std::collections::HashSet::new();
    let mut parsed: Vec<(i64, i32)> = Vec::with_capacity(input.ids.len());
    for raw in &input.ids {
        let id = parse_id(raw, "待办 id")?;
        if !seen.insert(id) {
            return Err(format!("排序列表含重复 id={id}"));
        }
        let done: i32 = user
            .query_row(
                "SELECT done FROM user_todo WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("待办 id={id} 不存在"))?;
        parsed.push((id, done));
    }

    let first_done = parsed[0].1;
    if parsed.iter().any(|(_, d)| *d != first_done) {
        return Err("排序列表须为同一完成状态".into());
    }

    let expected: i64 = user
        .query_row(
            "SELECT COUNT(*) FROM user_todo WHERE done = ?1",
            [first_done],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if parsed.len() as i64 != expected {
        return Err("排序列表须包含该完成状态下的全部待办".into());
    }

    let tx = user.transaction().map_err(|e| e.to_string())?;
    for (i, (id, _)) in parsed.iter().enumerate() {
        tx.execute(
            "UPDATE user_todo SET sort_order=?1, updated_at=?2 WHERE id=?3",
            params![i as i32, now, id],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

