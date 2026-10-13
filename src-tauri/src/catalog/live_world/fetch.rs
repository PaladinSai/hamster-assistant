//! 拉取官方 worldState.php 与 browse.wf bounty-cycle / 钢铁侵袭日程。

use std::path::Path;

use reqwest::blocking::Client;

use crate::catalog::resurgence::WORLDSTATE_URL;
use crate::catalog::upstream_cache;

use super::challenge_i18n;
use super::conquest_i18n;
use super::parse::parse_worldstate_with_cycle;
use super::steel_incursion;
use super::types::{LiveWorldFetch, BOUNTY_CYCLE_URL, USER_AGENT};

pub(crate) fn fetch_live_world(data_dir: &Path) -> Result<LiveWorldFetch, String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    // cycle / 挑战词典 / 钢铁日程失败可降级；与 worldState 并行
    let cycle_client = client.clone();
    let cycle_handle = std::thread::spawn(move || fetch_bounty_cycle_bytes(&cycle_client));
    let data_dir_dict = data_dir.to_path_buf();
    let dict_handle = std::thread::spawn(move || challenge_i18n::ensure_loaded(&data_dir_dict));
    let steel_client = client.clone();
    let steel_handle =
        std::thread::spawn(move || steel_incursion::fetch_schedule_bytes(&steel_client));

    let resp = client
        .get(WORLDSTATE_URL)
        .send()
        .map_err(|e| format!("请求 {WORLDSTATE_URL} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "请求 {WORLDSTATE_URL} HTTP {}",
            resp.status()
        ));
    }
    let body = resp
        .bytes()
        .map_err(|e| format!("读 {WORLDSTATE_URL} 响应失败: {e}"))?
        .to_vec();

    let _ = upstream_cache::save(data_dir, upstream_cache::WORLDSTATE, &body);

    let cycle = match cycle_handle.join() {
        Ok(Ok(bytes)) => Some(bytes),
        Ok(Err(e)) => {
            eprintln!("[仓鼠] bounty-cycle 拉取失败（降级空板）: {e}");
            None
        }
        Err(_) => {
            eprintln!("[仓鼠] bounty-cycle 线程异常（降级空板）");
            None
        }
    };

    let challenge_i18n = match dict_handle.join() {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[仓鼠] 挑战词典线程异常（降级手写回退）");
            None
        }
    };
    // 科研走 oracle Conquest 词典（Public Export 几乎无 Conquest 键）
    let conquest_lang = conquest_i18n::ConquestLang::ensure(data_dir);

    let steel_schedule = match steel_handle.join() {
        Ok(Ok(bytes)) => Some(bytes),
        Ok(Err(e)) => {
            eprintln!("[仓鼠] 钢铁侵袭日程拉取失败（将用缓存/日窗）: {e}");
            None
        }
        Err(_) => {
            eprintln!("[仓鼠] 钢铁侵袭日程线程异常（将用缓存/日窗）");
            None
        }
    };

    let mut fetch = parse_worldstate_with_cycle(
        &body,
        cycle.as_deref(),
        challenge_i18n.as_ref(),
        conquest_lang.as_ref(),
    )?;
    fetch.steel = steel_incursion::build_incursions(data_dir, steel_schedule.as_deref());
    // VoidStorms 无 MissionType，借钢铁侵袭已缓存的 ExportRegions 补九重天任务类型
    steel_incursion::fill_storm_fissure_mission_types(data_dir, &mut fetch.fissures);
    Ok(fetch)
}

fn fetch_bounty_cycle_bytes(client: &Client) -> Result<Vec<u8>, String> {
    let resp = client
        .get(BOUNTY_CYCLE_URL)
        .send()
        .map_err(|e| format!("请求 {BOUNTY_CYCLE_URL} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "请求 {BOUNTY_CYCLE_URL} HTTP {}",
            resp.status()
        ));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("读 {BOUNTY_CYCLE_URL} 响应失败: {e}"))
}
