//! 仲裁日程：browse.wf `arbys.txt`（unix,Node）+ ExportRegions / dict.zh。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use chrono::{TimeZone, Utc};
use reqwest::blocking::Client;
use serde::Serialize;

use rusqlite::Connection;

use crate::catalog::upstream_cache;
use crate::db;

use super::steel_incursion::NodeResolver;
use super::types::{SETTING_ARBITRATION_EXPIRY, USER_AGENT};

pub(crate) const ARBYS_URL: &str = "https://browse.wf/arbys.txt";
const ARBYS_FILE: &str = "arbys.txt";
/// 日程很长，缓存一天足够；手动更新会强制重拉。
const ARBYS_MAX_AGE: Duration = Duration::from_secs(24 * 3600);
const UPCOMING_COUNT: usize = 12;
static ARBYS_REFRESHING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitrationSlotDto {
    pub activation: String,
    pub expiry: String,
    pub node: String,
    pub mission_type: String,
    pub faction: String,
    pub node_code: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArbitrationBriefDto {
    pub source: String,
    pub current: Option<ArbitrationSlotDto>,
    pub upcoming: Vec<ArbitrationSlotDto>,
}

#[derive(Debug, Clone)]
struct Slot {
    start: i64,
    end: i64,
    node_code: String,
}

/// 读缓存日程并解析当前 / 后续；缺缓存 / 过期时踢后台线程，invoke 本身不 HTTP。
pub fn get_arbitration_brief(data_dir: &Path) -> Result<ArbitrationBriefDto, String> {
    kick_schedule_refresh(data_dir, false);
    let brief = match build_brief(data_dir) {
        Ok(b) => b,
        Err(_) => ArbitrationBriefDto {
            source: "empty".into(),
            current: None,
            upcoming: Vec::new(),
        },
    };
    if brief.current.is_some() || !brief.upcoming.is_empty() {
        if let Ok(catalog) = db::open_catalog(&db::catalog_path(data_dir)) {
            let _ = write_expiry_setting(&catalog, &brief);
        }
    }
    Ok(brief)
}

/// 手动更新 / 后台 soft_refresh：在已有工作线程里同步重拉。
pub fn force_refresh_arbitration_schedule(data_dir: &Path) -> Result<(), String> {
    ensure_schedule(data_dir, true)
}

/// 从日程缓存写入「下一档刷新点」，供 `local_live_fresh` 到期判断。
pub(crate) fn sync_expiry_from_cache(
    data_dir: &Path,
    catalog: &Connection,
) -> Result<(), String> {
    kick_schedule_refresh(data_dir, false);
    let Ok(brief) = build_brief(data_dir) else {
        return Ok(());
    };
    write_expiry_setting(catalog, &brief)
}

fn write_expiry_setting(
    catalog: &Connection,
    brief: &ArbitrationBriefDto,
) -> Result<(), String> {
    let when = brief
        .current
        .as_ref()
        .map(|c| c.expiry.as_str())
        .or_else(|| brief.upcoming.first().map(|u| u.activation.as_str()));
    match when {
        Some(ts) if !ts.is_empty() => db::set_setting(catalog, SETTING_ARBITRATION_EXPIRY, ts),
        _ => Ok(()),
    }
}

fn has_schedule_cache(data_dir: &Path) -> bool {
    upstream_cache::read(data_dir, ARBYS_FILE)
        .ok()
        .flatten()
        .map(|b| !b.is_empty())
        .unwrap_or(false)
}

/// 读路径用：有缓存就立刻返回；过期则另开线程拉，不堵前端。
fn kick_schedule_refresh(data_dir: &Path, force: bool) {
    if !force && has_schedule_cache(data_dir) && !schedule_stale(data_dir) {
        return;
    }
    if ARBYS_REFRESHING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let dir: PathBuf = data_dir.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("arbitration-schedule".into())
        .spawn(move || {
            if let Err(e) = download_and_save(&dir) {
                if has_schedule_cache(&dir) {
                    eprintln!("[仓鼠] 仲裁日程后台拉取失败（用缓存）: {e}");
                } else {
                    eprintln!("[仓鼠] 仲裁日程后台拉取失败: {e}");
                }
            }
            ARBYS_REFRESHING.store(false, Ordering::Release);
        });
}

fn ensure_schedule(data_dir: &Path, force: bool) -> Result<(), String> {
    if !force && !schedule_stale(data_dir) && has_schedule_cache(data_dir) {
        return Ok(());
    }
    match download_and_save(data_dir) {
        Ok(()) => Ok(()),
        Err(e) => {
            if has_schedule_cache(data_dir) {
                eprintln!("[仓鼠] 仲裁日程拉取失败（用缓存）: {e}");
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn schedule_stale(data_dir: &Path) -> bool {
    let path = crate::db::upstream_dir(data_dir).join(ARBYS_FILE);
    if !path.is_file() {
        return true;
    }
    let Ok(meta) = std::fs::metadata(&path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return true;
    };
    age > ARBYS_MAX_AGE
}

fn download_and_save(data_dir: &Path) -> Result<(), String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(ARBYS_URL)
        .send()
        .map_err(|e| format!("请求 {ARBYS_URL} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("请求 {ARBYS_URL} HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .map_err(|e| format!("读 {ARBYS_URL} 失败: {e}"))?
        .to_vec();
    let text = String::from_utf8_lossy(&bytes);
    let _ = parse_hourly_slots(&text)?;
    upstream_cache::save(data_dir, ARBYS_FILE, &bytes)?;
    Ok(())
}

fn build_brief(data_dir: &Path) -> Result<ArbitrationBriefDto, String> {
    let bytes = upstream_cache::read(data_dir, ARBYS_FILE)?
        .ok_or_else(|| "无仲裁日程缓存".to_string())?;
    let text = String::from_utf8_lossy(&bytes);
    let hourly = parse_hourly_slots(&text)?;
    let runs = merge_consecutive(hourly);
    let now = Utc::now().timestamp();

    let cur_idx = runs
        .iter()
        .position(|s| s.start <= now && now < s.end)
        .or_else(|| {
            // 整点间隙：取即将开始的下一档
            runs.iter().position(|s| s.start > now)
        });

    let resolver = NodeResolver::load(data_dir);
    let to_dto = |s: &Slot| -> ArbitrationSlotDto {
        let (mission_type, faction, node) = match resolver.as_ref() {
            Some(r) => {
                let d = r.resolve(&s.node_code);
                (d.mission_type, d.faction, d.node)
            }
            None => (
                String::new(),
                String::new(),
                super::localize::node_name(&s.node_code),
            ),
        };
        ArbitrationSlotDto {
            activation: ts_rfc3339(s.start),
            expiry: ts_rfc3339(s.end),
            node,
            mission_type,
            faction,
            node_code: s.node_code.clone(),
        }
    };

    let (current, upcoming) = match cur_idx {
        Some(i) => {
            let current = if runs[i].start <= now && now < runs[i].end {
                Some(to_dto(&runs[i]))
            } else {
                None
            };
            let start_up = if current.is_some() { i + 1 } else { i };
            let upcoming: Vec<_> = runs[start_up..]
                .iter()
                .take(UPCOMING_COUNT)
                .map(to_dto)
                .collect();
            (current, upcoming)
        }
        None => (None, Vec::new()),
    };

    Ok(ArbitrationBriefDto {
        source: "browse.wf/arbys.txt".into(),
        current,
        upcoming,
    })
}

fn ts_rfc3339(ts: i64) -> String {
    Utc.timestamp_opt(ts, 0)
        .single()
        .map(|t| t.to_rfc3339())
        .unwrap_or_default()
}

/// `unix,NodeCode` 每行一档；结束时间为下一档开始（缺省 +3600）。
fn parse_hourly_slots(text: &str) -> Result<Vec<(i64, String)>, String> {
    let mut out = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((ts_s, node)) = line.split_once(',') else {
            return Err(format!("仲裁日程第 {} 行格式错误", line_no + 1));
        };
        let ts: i64 = ts_s
            .trim()
            .parse()
            .map_err(|_| format!("仲裁日程第 {} 行时间戳无效", line_no + 1))?;
        let node = node.trim();
        if node.is_empty() {
            continue;
        }
        out.push((ts, node.to_string()));
    }
    if out.is_empty() {
        return Err("仲裁日程为空".into());
    }
    out.sort_by_key(|(ts, _)| *ts);
    Ok(out)
}

fn merge_consecutive(hourly: Vec<(i64, String)>) -> Vec<Slot> {
    let mut runs: Vec<Slot> = Vec::new();
    for (i, (start, code)) in hourly.iter().enumerate() {
        let end = hourly
            .get(i + 1)
            .map(|(t, _)| *t)
            .unwrap_or(start + 3600);
        if let Some(last) = runs.last_mut() {
            if last.node_code == *code && last.end == *start {
                last.end = end;
                continue;
            }
        }
        runs.push(Slot {
            start: *start,
            end,
            node_code: code.clone(),
        });
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_merge_hourly() {
        let text = "\
1000,SolNode1
4600,SolNode1
8200,SolNode2
11800,SolNode3
";
        let hourly = parse_hourly_slots(text).unwrap();
        assert_eq!(hourly.len(), 4);
        let runs = merge_consecutive(hourly);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].node_code, "SolNode1");
        assert_eq!(runs[0].start, 1000);
        assert_eq!(runs[0].end, 8200);
        assert_eq!(runs[1].node_code, "SolNode2");
        assert_eq!(runs[2].node_code, "SolNode3");
    }
}
