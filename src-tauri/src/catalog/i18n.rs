//! 中英名称对照 → catalog.name_i18n
//!
//! - `name_zh_userdict`：打包/灰机词典、任务类型 builtin
//! - `name_zh_wfcd`：WFCD `i18n/zh.json`（uniqueName 对齐到英文名后写入）
//! - 展示 / ZhLookup：`COALESCE(wfcd, userdict)`（国际服优先）
//!
//! 联网拉灰机仅作可选调试（`fetch_huiji_name_i18n` / `HAMSTER_FETCH_HUIJI_I18N`），不推荐。

use std::collections::HashMap;
use std::path::Path;

use chrono::Utc;
use reqwest::blocking::Client;
use rusqlite::{params, Connection, Transaction};
use serde_json::Value;

use crate::db;

const WIKI_API: &str = "https://warframe.huijiwiki.com/api.php";
const USERDICT_TITLE: &str = "Data:UserDict.json";
const USERDICT_RAW: &str = "https://warframe.huijiwiki.com/index.php?title=Data:UserDict.json&action=raw";

const SETTING_USERDICT_FP: &str = "catalog_userdict_fp";

/// 展示用中文：有 wfcd 用 wfcd，否则 userdict。
pub const DISPLAY_ZH_SQL: &str = "COALESCE(NULLIF(TRIM(name_zh_wfcd), ''), NULLIF(TRIM(name_zh_userdict), ''))";

#[derive(Debug, Clone)]
pub struct I18nRow {
    pub name_en: String,
    pub name_zh: String,
    pub category: Option<String>,
}

/// 从灰机拉取 `Data:UserDict.json`（API → raw URL）。
///
/// 仅供维护者调试；seed 默认应读本地文件。易被 Cloudflare 拦截。
pub fn fetch_huiji_name_i18n() -> Result<Vec<I18nRow>, String> {
    let client = Client::builder()
        .user_agent("cangshu-assistant/0.1 (catalog seed; +https://github.com/)")
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let text = fetch_userdict_via_api(&client)
        .or_else(|api_err| {
            eprintln!("[i18n] MediaWiki API 失败，改试 raw URL: {api_err}");
            let resp = client
                .get(USERDICT_RAW)
                .send()
                .map_err(|e| format!("请求 UserDict raw 失败: {e}"))?;
            if !resp.status().is_success() {
                return Err(format!("UserDict raw HTTP {}", resp.status()));
            }
            resp.text()
                .map_err(|e| format!("读 UserDict raw 失败: {e}"))
        })
        .map_err(|e| {
            format!(
                "{e}\n提示: Cloudflare 可能拦截自动化请求。请浏览器打开\n  {USERDICT_RAW}\n另存为临时 JSON 后运行:\n  cargo run --bin seed_catalog -- <out_dir> <userdict.json>"
            )
        })?;

    let rows = parse_userdict_json(&text)?;
    if rows.len() < 100 {
        return Err(format!(
            "UserDict 解析过少（{} 行），可能被拦截或页面结构变化",
            rows.len()
        ));
    }
    Ok(rows)
}

fn fetch_userdict_via_api(client: &Client) -> Result<String, String> {
    let resp = client
        .get(WIKI_API)
        .query(&[
            ("action", "query"),
            ("titles", USERDICT_TITLE),
            ("prop", "revisions"),
            ("rvprop", "content"),
            ("rvslots", "main"),
            ("format", "json"),
            ("formatversion", "2"),
        ])
        .send()
        .map_err(|e| format!("请求灰机 API 失败: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("灰机 API HTTP {}", resp.status()));
    }

    let body: Value = resp
        .json()
        .map_err(|e| format!("灰机 API JSON 解析失败: {e}"))?;

    if let Some(err) = body.get("error") {
        return Err(format!("灰机 API error: {err}"));
    }

    body.pointer("/query/pages/0/revisions/0/slots/main/content")
        .or_else(|| body.pointer("/query/pages/0/revisions/0/content"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "灰机 API 无 UserDict revision content".to_string())
}

/// 从本地文件解析（UserDict 格式 JSON / HTML 表 / wikitext）。
/// 维护者自备词典；勿提交大型临时另存文件进应用仓库亦可。
pub fn parse_i18n_from_file(path: &std::path::Path) -> Result<Vec<I18nRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("读对照文件失败 {}: {e}", path.display()))?;
    let trimmed = text.trim_start();
    let rows = if trimmed.starts_with('{') && trimmed.contains("\"Text\"") {
        parse_userdict_json(&text)?
    } else if text.contains("<table") || text.contains("<tr") {
        parse_i18n_html(&text)
    } else {
        parse_i18n_wikitext(&text)
    };
    if rows.len() < 50 {
        return Err(format!("对照文件解析过少（{} 行）", rows.len()));
    }
    Ok(rows)
}

/// `Data:UserDict.json`：`{ "Text": { "En": "中文", ... }, "Category": { ... } }`
pub fn parse_userdict_json(text: &str) -> Result<Vec<I18nRow>, String> {
    let root: Value =
        serde_json::from_str(text).map_err(|e| format!("UserDict JSON 解析失败: {e}"))?;
    let text_map = root
        .get("Text")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "UserDict 缺少 Text 对象".to_string())?;

    let mut rows = Vec::with_capacity(text_map.len());
    for (en, zh_val) in text_map {
        let Some(zh) = zh_val.as_str() else {
            continue;
        };
        let en = en.trim();
        let zh = zh.trim();
        if en.is_empty() || zh.is_empty() {
            continue;
        }
        if !en.chars().any(|c| c.is_ascii_alphabetic()) {
            continue;
        }
        rows.push(I18nRow {
            name_en: en.to_string(),
            name_zh: zh.to_string(),
            category: None,
        });
    }
    Ok(dedupe_rows(rows))
}

fn parse_i18n_html(html: &str) -> Vec<I18nRow> {
    let mut rows = Vec::new();
    let mut category: Option<String> = None;

    let mut rest = html;
    while let Some(idx) = rest.find('<') {
        let chunk = &rest[idx..];
        if let Some(h) = strip_heading(chunk) {
            category = Some(h.0);
            rest = h.1;
            continue;
        }
        if chunk.starts_with("<tr") || chunk.starts_with("<TR") {
            if let Some((end, cells)) = read_tr_cells(chunk) {
                if cells.len() >= 2 {
                    let en = clean_cell(&cells[0]);
                    let zh = clean_cell(&cells[1]);
                    if !en.is_empty()
                        && !zh.is_empty()
                        && en != "英文"
                        && en != "English"
                        && zh != "中文"
                        && en.chars().any(|c| c.is_ascii_alphabetic())
                    {
                        rows.push(I18nRow {
                            name_en: en,
                            name_zh: zh,
                            category: category.clone(),
                        });
                    }
                }
                rest = &chunk[end..];
                continue;
            }
        }
        rest = &rest[idx + 1..];
    }

    dedupe_rows(rows)
}

fn parse_i18n_wikitext(text: &str) -> Vec<I18nRow> {
    let mut rows = Vec::new();
    let mut category: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("==") {
            let name = rest.trim_matches('=').trim().to_string();
            if !name.is_empty() {
                category = Some(name);
            }
            continue;
        }
        if !t.starts_with('|') || t.starts_with("|+") || t.starts_with("|}") || t.starts_with("|-")
        {
            continue;
        }
        let cells: Vec<String> = t
            .trim_start_matches('|')
            .split("||")
            .flat_map(|p| p.split('|'))
            .map(|s| clean_wikitext_cell(s))
            .filter(|s| !s.is_empty())
            .collect();
        if cells.len() >= 2 {
            let en = &cells[0];
            let zh = &cells[1];
            if en != "英文"
                && en != "English"
                && zh != "中文"
                && en.chars().any(|c| c.is_ascii_alphabetic())
            {
                rows.push(I18nRow {
                    name_en: en.clone(),
                    name_zh: zh.clone(),
                    category: category.clone(),
                });
            }
        }
    }
    dedupe_rows(rows)
}

fn dedupe_rows(rows: Vec<I18nRow>) -> Vec<I18nRow> {
    use std::collections::HashMap;
    let mut map: HashMap<String, I18nRow> = HashMap::new();
    for r in rows {
        map.insert(r.name_en.clone(), r);
    }
    map.into_values().collect()
}

fn strip_heading(chunk: &str) -> Option<(String, &str)> {
    let lower = chunk.to_ascii_lowercase();
    for tag in ["h2", "h3", "h4"] {
        let open = format!("<{tag}");
        if !lower.starts_with(&open) {
            continue;
        }
        let after_gt = chunk.find('>')? + 1;
        let close = format!("</{tag}>");
        let end_rel = lower[after_gt..].find(&close)?;
        let end = after_gt + end_rel;
        let inner = &chunk[after_gt..end];
        let name = clean_cell(inner);
        let after = end + close.len();
        if !name.is_empty() && after <= chunk.len() {
            return Some((name, &chunk[after..]));
        }
    }
    None
}

fn read_tr_cells(chunk: &str) -> Option<(usize, Vec<String>)> {
    let tr_end = chunk.find("</tr>").or_else(|| chunk.find("</TR>"))?;
    let inner = &chunk[..tr_end];
    let mut cells = Vec::new();
    let mut search = inner;
    while let Some(td_rel) = search.find("<td").or_else(|| search.find("<TD")) {
        let from = &search[td_rel..];
        let gt = from.find('>')? + 1;
        let close = from.find("</td>").or_else(|| from.find("</TD>"))?;
        cells.push(from[gt..close].to_string());
        search = &from[close + 5..];
    }
    Some((tr_end + 5, cells))
}

fn clean_cell(raw: &str) -> String {
    let mut s = raw.to_string();
    while let Some(start) = s.find('<') {
        if let Some(end) = s[start..].find('>') {
            s.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    s = s
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&#39;", "'")
        .replace("&quot;", "\"");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_wikitext_cell(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    while let Some(start) = s.find("[[") {
        if let Some(end) = s[start..].find("]]") {
            let inner = s[start + 2..start + end].to_string();
            let display = inner
                .split('|')
                .next_back()
                .unwrap_or(&inner)
                .trim()
                .to_string();
            s.replace_range(start..start + end + 2, &display);
        } else {
            break;
        }
    }
    s = s.replace("'''", "").replace("''", "");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 导入时解析中文：先精确 `name_en`，再大小写不敏感。读 COALESCE(wfcd, userdict)。
#[derive(Debug, Default, Clone)]
pub struct ZhLookup {
    pub(crate) exact: HashMap<String, String>,
    lower: HashMap<String, String>,
}

impl ZhLookup {
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let sql = format!(
            "SELECT name_en, {DISPLAY_ZH_SQL} AS name_zh FROM name_i18n
             WHERE {DISPLAY_ZH_SQL} IS NOT NULL"
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        let mut exact = HashMap::new();
        let mut lower = HashMap::new();
        for r in rows {
            let (en, zh) = r.map_err(|e| e.to_string())?;
            let key = en.to_ascii_lowercase();
            lower.entry(key).or_insert_with(|| zh.clone());
            exact.insert(en, zh);
        }
        Ok(Self { exact, lower })
    }

    pub fn get(&self, en: &str) -> Option<String> {
        let t = en.trim();
        if t.is_empty() {
            return None;
        }
        if let Some(z) = self.exact.get(t) {
            return Some(z.clone());
        }
        self.lower.get(&t.to_ascii_lowercase()).cloned()
    }
}

/// 写入 / 更新 `name_zh_userdict`（保留已有 wfcd 列）。
pub fn upsert_name_i18n(tx: &Transaction<'_>, rows: &[I18nRow]) -> Result<usize, String> {
    upsert_name_i18n_userdict(tx, rows)
}

pub fn upsert_name_i18n_userdict(
    tx: &Transaction<'_>,
    rows: &[I18nRow],
) -> Result<usize, String> {
    let now = Utc::now().to_rfc3339();
    let mut n = 0usize;
    for r in rows {
        tx.execute(
            "INSERT INTO name_i18n (name_en, name_zh_userdict, category, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name_en) DO UPDATE SET
               name_zh_userdict = excluded.name_zh_userdict,
               category = COALESCE(excluded.category, name_i18n.category),
               updated_at = excluded.updated_at",
            params![r.name_en, r.name_zh, r.category, now],
        )
        .map_err(|e| format!("写入 name_zh_userdict {} 失败: {e}", r.name_en))?;
        n += 1;
    }
    Ok(n)
}

/// 写入 / 更新 `name_zh_wfcd`（保留已有 userdict 列）。
pub fn upsert_name_i18n_wfcd(tx: &Transaction<'_>, rows: &[I18nRow]) -> Result<usize, String> {
    let now = Utc::now().to_rfc3339();
    let mut n = 0usize;
    for r in rows {
        tx.execute(
            "INSERT INTO name_i18n (name_en, name_zh_wfcd, category, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(name_en) DO UPDATE SET
               name_zh_wfcd = excluded.name_zh_wfcd,
               category = COALESCE(excluded.category, name_i18n.category),
               updated_at = excluded.updated_at",
            params![r.name_en, r.name_zh, r.category, now],
        )
        .map_err(|e| format!("写入 name_zh_wfcd {} 失败: {e}", r.name_en))?;
        n += 1;
    }
    Ok(n)
}

fn userdict_fingerprint(path: &Path) -> Result<String, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("读词典元数据失败: {e}"))?;
    let len = meta.len();
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(format!("{len}:{mtime}"))
}

/// 从打包 `userdict.json` 合并进 `name_zh_userdict`（指纹在 catalog.setting）。
pub fn ensure_name_i18n_from_bundled(catalog: &mut Connection) -> Result<usize, String> {
    let path = db::bundled_resource_path("userdict.json");
    if !path.is_file() {
        return Ok(0);
    }
    let fp = userdict_fingerprint(&path)?;
    if db::get_setting(catalog, SETTING_USERDICT_FP)?.as_deref() == Some(fp.as_str()) {
        return Ok(0);
    }
    let rows = parse_i18n_from_file(&path)?;
    let tx = catalog
        .transaction()
        .map_err(|e| format!("对照事务失败: {e}"))?;
    let written = upsert_name_i18n_userdict(&tx, &rows)?;
    tx.commit().map_err(|e| e.to_string())?;
    db::set_setting(catalog, SETTING_USERDICT_FP, &fp)?;
    eprintln!(
        "[仓鼠] 已合并对照词典 ≈{} 条 ← {}",
        written,
        path.display()
    );
    Ok(written)
}

/// 解析 WFCD `i18n/zh.json` → uniqueName → 中文名。
pub fn parse_wfcd_unique_zh(i18n_json: &[u8]) -> Result<HashMap<String, String>, String> {
    let root: Value =
        serde_json::from_slice(i18n_json).map_err(|e| format!("WFCD i18n 解析失败: {e}"))?;
    let Value::Object(map) = root else {
        return Err("WFCD i18n 应为对象".into());
    };
    let mut out = HashMap::new();
    for (unique, entry) in &map {
        let zh = entry
            .get("name")
            .and_then(|x| x.as_str())
            .or_else(|| {
                entry
                    .get("zh")
                    .and_then(|v| v.get("name"))
                    .and_then(|x| x.as_str())
            })
            .map(str::trim)
            .filter(|s| !s.is_empty());
        if let Some(zh) = zh {
            out.insert(unique.clone(), zh.to_string());
        }
    }
    Ok(out)
}

/// 按实体 `unique_name`→`name_en` 写入 `name_zh_wfcd`（O(实体)）。
pub fn upsert_wfcd_i18n_by_entities(
    conn: &mut Connection,
    i18n_json: &[u8],
) -> Result<usize, String> {
    let zh_by_unique = parse_wfcd_unique_zh(i18n_json)?;
    if zh_by_unique.is_empty() {
        return Ok(0);
    }

    let mut pairs: Vec<(String, String)> = Vec::new();
    for sql in [
        "SELECT unique_name, name_en FROM warframe WHERE unique_name IS NOT NULL AND name_en IS NOT NULL AND name_en != ''",
        "SELECT unique_name, name_en FROM weapon WHERE unique_name IS NOT NULL AND name_en IS NOT NULL AND name_en != ''",
        "SELECT unique_name, name_en FROM mod WHERE unique_name IS NOT NULL AND name_en IS NOT NULL AND name_en != ''",
        "SELECT unique_name, name_en FROM arcane WHERE unique_name IS NOT NULL AND name_en IS NOT NULL AND name_en != ''",
    ] {
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (unique, name_en) = row.map_err(|e| e.to_string())?;
            if let Some(zh) = zh_by_unique.get(&unique) {
                pairs.push((name_en, zh.clone()));
            }
        }
    }

    let mut seen = HashMap::new();
    let mut rows = Vec::new();
    for (en, zh) in pairs {
        if seen.insert(en.clone(), ()).is_none() {
            rows.push(I18nRow {
                name_en: en,
                name_zh: zh,
                category: Some("wfcd".into()),
            });
        }
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("wfcd i18n 事务失败: {e}"))?;
    let n = upsert_name_i18n_wfcd(&tx, &rows)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(n)
}

/// 按 COALESCE(wfcd,userdict) 写回各实体 `name_zh`（覆盖已有展示名）。
pub fn apply_display_zh_to_entities(conn: &Connection) -> Result<usize, String> {
    let zh = ZhLookup::load(conn)?;
    let mut n = 0usize;
    for (sel, upd) in [
        (
            "SELECT id, name_en FROM warframe WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE warframe SET name_zh = ?2 WHERE id = ?1",
        ),
        (
            "SELECT id, name_en FROM weapon WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE weapon SET name_zh = ?2 WHERE id = ?1",
        ),
        (
            "SELECT id, name_en FROM mod WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE mod SET name_zh = ?2 WHERE id = ?1",
        ),
        (
            "SELECT id, name_en FROM arcane WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE arcane SET name_zh = ?2 WHERE id = ?1",
        ),
        (
            "SELECT id, name_en FROM warframe_family WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE warframe_family SET name_zh = ?2 WHERE id = ?1",
        ),
        (
            "SELECT id, name_en FROM weapon_family WHERE name_en IS NOT NULL AND name_en != ''",
            "UPDATE weapon_family SET name_zh = ?2 WHERE id = ?1",
        ),
    ] {
        let mut stmt = conn.prepare(sel).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        for (id, name_en) in rows {
            let Some(lookup) = zh.get(&name_en) else {
                continue;
            };
            n += conn
                .execute(upd, params![id, lookup])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(n)
}

/// 任务类型 → `name_zh_userdict`（仅当该列仍空时填入）。
pub fn ensure_mission_type_i18n(conn: &Connection) -> Result<(), String> {
    const PAIRS: &[(&str, &str)] = &[
        ("Spy", "间谍"),
        ("Survival", "生存"),
        ("Defense", "防御"),
        ("Capture", "捕获"),
        ("Excavation", "挖掘"),
        ("Interception", "拦截"),
        ("Disruption", "中断"),
        ("Sabotage", "破坏"),
        ("Assassination", "刺杀"),
        ("Rescue", "救援"),
        ("Exterminate", "歼灭"),
        ("Mobile Defense", "移动防御"),
        ("Defection", "叛逃"),
        ("Infested Salvage", "Infested资源回收"),
        ("Sanctuary Onslaught", "圣殿突袭"),
        ("The Circuit", "无尽回廊"),
        ("Alchemy", "炼金"),
        ("Void Flood", "虚空洪流"),
        ("Void Cascade", "虚空串联"),
        ("Void Armageddon", "虚空决战"),
        ("Pursuit", "追击"),
        ("Hive", "清巢"),
        ("Orphix", "奥菲克斯"),
        ("Bounty", "赏金"),
    ];
    let now = Utc::now().to_rfc3339();
    for (en, zh) in PAIRS {
        conn.execute(
            "INSERT INTO name_i18n (name_en, name_zh_userdict, category, updated_at)
             VALUES (?1, ?2, 'mission', ?3)
             ON CONFLICT(name_en) DO UPDATE SET
               name_zh_userdict = COALESCE(NULLIF(TRIM(name_i18n.name_zh_userdict), ''), excluded.name_zh_userdict),
               category = COALESCE(name_i18n.category, excluded.category),
               updated_at = excluded.updated_at",
            params![en, zh, now],
        )
        .map_err(|e| format!("补任务类型 {en} 失败: {e}"))?;
    }
    Ok(())
}

#[allow(dead_code)]
pub fn load_i18n_map(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let z = ZhLookup::load(conn)?;
    Ok(z.exact)
}

#[allow(dead_code)]
pub fn localize_with_map(raw: &str, map: &HashMap<String, String>) -> String {
    let mut s = raw.to_string();
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by(|a, b| b.len().cmp(&a.len()));
    for k in keys {
        if s.contains(k.as_str()) {
            if let Some(zh) = map.get(k) {
                s = s.replace(k, zh);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn mem_i18n_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE name_i18n (
               name_en TEXT NOT NULL PRIMARY KEY,
               name_zh_userdict TEXT,
               name_zh_wfcd TEXT,
               category TEXT,
               updated_at TEXT
             );
             CREATE TABLE warframe (unique_name TEXT, name_en TEXT);
             CREATE TABLE weapon (unique_name TEXT, name_en TEXT);
             CREATE TABLE mod (unique_name TEXT, name_en TEXT);
             CREATE TABLE arcane (
               id INTEGER PRIMARY KEY,
               unique_name TEXT,
               name_en TEXT,
               name_zh TEXT
             );
             CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn parse_userdict_text_map() {
        let raw = r#"{"Text":{"Mercury":"水星","Foo Bar":"测试"},"Category":{"Planet":"行星"}}"#;
        let rows = parse_userdict_json(raw).expect("parse");
        assert_eq!(rows.len(), 2);
        let mercury = rows.iter().find(|r| r.name_en == "Mercury").unwrap();
        assert_eq!(mercury.name_zh, "水星");
    }

    #[test]
    fn wfcd_preferred_over_userdict_in_lookup() {
        let mut conn = mem_i18n_db();
        let ud = [I18nRow {
            name_en: "Arcane Acceleration".into(),
            name_zh: "加速赋能".into(),
            category: None,
        }];
        let wf = [I18nRow {
            name_en: "Arcane Acceleration".into(),
            name_zh: "赋能加速".into(),
            category: None,
        }];
        {
            let tx = conn.transaction().unwrap();
            upsert_name_i18n_userdict(&tx, &ud).unwrap();
            upsert_name_i18n_wfcd(&tx, &wf).unwrap();
            tx.commit().unwrap();
        }
        let zh = ZhLookup::load(&conn).unwrap();
        assert_eq!(zh.get("Arcane Acceleration").as_deref(), Some("赋能加速"));
    }

    #[test]
    fn upsert_wfcd_i18n_aligns_by_unique_name() {
        let mut conn = mem_i18n_db();
        conn.execute(
            "INSERT INTO arcane (id, unique_name, name_en, name_zh)
             VALUES (1, '/Lotus/Upgrades/CosmeticEnhancers/Offensive/OnHeadshotFireRate', 'Arcane Acceleration', NULL)",
            [],
        )
        .unwrap();
        let i18n = r#"{
          "/Lotus/Upgrades/CosmeticEnhancers/Offensive/OnHeadshotFireRate": {
            "name": "加速赋能",
            "description": "x"
          }
        }"#
        .as_bytes();
        let n = upsert_wfcd_i18n_by_entities(&mut conn, i18n).unwrap();
        assert_eq!(n, 1);
        let zh: String = conn
            .query_row(
                "SELECT name_zh_wfcd FROM name_i18n WHERE name_en = 'Arcane Acceleration'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(zh, "加速赋能");
    }
}
