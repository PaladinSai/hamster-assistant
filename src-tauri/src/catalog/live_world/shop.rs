//! 周限商店：钢铁之路阁下实时轮换 + 中文对照。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::catalog::upstream_cache;

use super::types::USER_AGENT;

const STEEL_PATH_URL: &str = "https://api.warframestat.us/pc/steelPath";
const STEEL_PATH_FILE: &str = "steelPath.json";
/// 阁下轮换按周变；缓存一小时足够，读路径绝不阻塞拉网。
const STEEL_PATH_MAX_AGE: Duration = Duration::from_secs(3600);
static STEEL_PATH_REFRESHING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopOfferDto {
    pub name_en: String,
    pub name_zh: String,
    pub cost: i32,
    pub cost_zh: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyShopBrief {
    /// live = 拉到 warframestat；fallback = 本地周序推算
    pub source: String,
    pub teshin_activation: Option<String>,
    pub teshin_expiry: Option<String>,
    pub teshin_current: ShopOfferDto,
    pub teshin_rotation: Vec<ShopOfferDto>,
    pub bird3_current: ShopOfferDto,
    pub bird3_rotation: Vec<ShopOfferDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SteelPathApi {
    current_reward: Option<SimpleReward>,
    activation: Option<String>,
    expiry: Option<String>,
    #[serde(default)]
    rotation: Vec<SimpleReward>,
}

#[derive(Debug, Deserialize)]
struct SimpleReward {
    name: String,
    cost: i32,
}

fn localize_teshin(name_en: &str, cost: i32) -> ShopOfferDto {
    let name_zh = teshin_name_zh(name_en);
    ShopOfferDto {
        name_en: name_en.to_string(),
        name_zh: name_zh.to_string(),
        cost,
        cost_zh: format!("{cost} 钢铁精华"),
    }
}

/// 灰机译名；未知英文原样返回（避免瞎译）。
fn teshin_name_zh(name_en: &str) -> &str {
    match name_en.trim() {
        "Umbra Forma Blueprint" => "Umbra Forma 蓝图",
        "50,000 Kuva" => "50,000 赤毒",
        "Kitgun Riven Mod" => "组合枪裂罅 Mod",
        "3x Forma" | "3× Forma" | "3 x Forma" => "3×Forma",
        "Zaw Riven Mod" => "Zaw 裂罅 Mod",
        "30,000 Endo" => "30,000 内融核心",
        "Rifle Riven Mod" => "步枪裂罅 Mod",
        "Shotgun Riven Mod" => "霰弹枪裂罅 Mod",
        "Veiled Riven Cipher" => "裂罅破解器",
        "10k Kuva" | "10,000 Kuva" => "10,000 赤毒",
        "Relic Pack" => "遗物组合包（3 枚）",
        "Primary Arcane Adapter" => "主要武器赋能槽连接器",
        "Secondary Arcane Adapter" => "次要武器赋能槽连接器",
        "Stance Forma Blueprint" => "架势 Forma 蓝图",
        other => other,
    }
}

const TESHIN_FALLBACK: &[(&str, i32)] = &[
    ("Umbra Forma Blueprint", 150),
    ("50,000 Kuva", 55),
    ("Kitgun Riven Mod", 75),
    ("3x Forma", 75),
    ("Zaw Riven Mod", 75),
    ("30,000 Endo", 150),
    ("Rifle Riven Mod", 75),
    ("Shotgun Riven Mod", 75),
];

/// 2026-09-14 周一 = Umbra 周（与前端一致）。
const TESHIN_EPOCH_MS: i64 = 1_789_344_000_000; // 2026-09-14T00:00:00Z
const BIRD3_EPOCH_MS: i64 = 1_662_336_000_000; // 2022-09-05T00:00:00Z
const WEEK_MS: i64 = 604_800_000;

const BIRD3_ROTATION: &[(&str, &str, i32)] = &[
    ("Azure Archon Shard", "蔚蓝执刑官源力石", 30_000),
    ("Amber Archon Shard", "琥珀执刑官源力石", 30_000),
    ("Crimson Archon Shard", "深红执刑官源力石", 30_000),
];

fn monday_utc_ms(now_ms: i64) -> i64 {
    // 1970-01-01 为周四；weekday: 0=周日 … 1=周一
    let days = (now_ms / 1000).div_euclid(86_400);
    let weekday = (4 + days).rem_euclid(7);
    let back = if weekday == 0 { 6 } else { weekday - 1 };
    (days - back) * 86_400 * 1000
}

fn teshin_index(now_ms: i64) -> usize {
    let weeks = (monday_utc_ms(now_ms) - TESHIN_EPOCH_MS).div_euclid(WEEK_MS);
    weeks.rem_euclid(8) as usize
}

fn bird3_index(now_ms: i64) -> usize {
    let weeks = (monday_utc_ms(now_ms) - BIRD3_EPOCH_MS).div_euclid(WEEK_MS);
    // epoch 周为深红（下标 2）
    ((weeks.rem_euclid(3) + 2) % 3) as usize
}

fn bird3_offer(idx: usize) -> ShopOfferDto {
    let (en, zh, cost) = BIRD3_ROTATION[idx % BIRD3_ROTATION.len()];
    ShopOfferDto {
        name_en: en.to_string(),
        name_zh: zh.to_string(),
        cost,
        cost_zh: format!("{cost} 科维兽声望"),
    }
}

fn bird3_rotation_all() -> Vec<ShopOfferDto> {
    (0..BIRD3_ROTATION.len()).map(bird3_offer).collect()
}

fn fallback_brief(now_ms: i64) -> WeeklyShopBrief {
    let idx = teshin_index(now_ms);
    let rotation: Vec<ShopOfferDto> = TESHIN_FALLBACK
        .iter()
        .map(|(n, c)| localize_teshin(n, *c))
        .collect();
    let current = rotation[idx].clone();
    let mon = monday_utc_ms(now_ms);
    WeeklyShopBrief {
        source: "fallback".into(),
        teshin_activation: Some(
            chrono::DateTime::from_timestamp_millis(mon)
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        ),
        teshin_expiry: Some(
            chrono::DateTime::from_timestamp_millis(mon + WEEK_MS - 1000)
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        ),
        teshin_current: current,
        teshin_rotation: rotation,
        bird3_current: bird3_offer(bird3_index(now_ms)),
        bird3_rotation: bird3_rotation_all(),
    }
}

fn fetch_steel_path_bytes() -> Result<Vec<u8>, String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(STEEL_PATH_URL)
        .send()
        .map_err(|e| format!("请求钢铁之路阁下失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("钢铁之路阁下 HTTP {}", resp.status()));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("读钢铁之路阁下响应失败: {e}"))
}

fn steel_path_stale(data_dir: &Path) -> bool {
    let path = crate::db::upstream_dir(data_dir).join(STEEL_PATH_FILE);
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
    age > STEEL_PATH_MAX_AGE
}

fn kick_steel_path_refresh(data_dir: &Path) {
    if !steel_path_stale(data_dir) {
        return;
    }
    if STEEL_PATH_REFRESHING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let dir = data_dir.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("steel-path-shop".into())
        .spawn(move || {
            match fetch_steel_path_bytes() {
                Ok(bytes) => {
                    if let Err(e) = upstream_cache::save(&dir, STEEL_PATH_FILE, &bytes) {
                        eprintln!("[仓鼠] 钢铁之路阁下缓存写入失败: {e}");
                    }
                }
                Err(e) => eprintln!("[仓鼠] 钢铁之路阁下后台拉取失败: {e}"),
            }
            STEEL_PATH_REFRESHING.store(false, Ordering::Release);
        });
}

fn brief_from_api(api: SteelPathApi, now_ms: i64) -> WeeklyShopBrief {
    let bird3_current = bird3_offer(bird3_index(now_ms));
    let bird3_rotation = bird3_rotation_all();
    let rotation: Vec<ShopOfferDto> = if api.rotation.is_empty() {
        TESHIN_FALLBACK
            .iter()
            .map(|(n, c)| localize_teshin(n, *c))
            .collect()
    } else {
        api.rotation
            .iter()
            .map(|r| localize_teshin(&r.name, r.cost))
            .collect()
    };
    let current = if let Some(r) = api.current_reward {
        localize_teshin(&r.name, r.cost)
    } else if !rotation.is_empty() {
        rotation[teshin_index(now_ms) % rotation.len()].clone()
    } else {
        return fallback_brief(now_ms);
    };
    WeeklyShopBrief {
        source: "live".into(),
        teshin_activation: api.activation,
        teshin_expiry: api.expiry,
        teshin_current: current,
        teshin_rotation: rotation,
        bird3_current,
        bird3_rotation,
    }
}

/// 只读本地缓存 / 周序回退；过期时踢后台线程拉网，绝不在 invoke 里 HTTP。
pub fn get_weekly_shop_brief(data_dir: &Path) -> WeeklyShopBrief {
    let now_ms = chrono::Utc::now().timestamp_millis();
    kick_steel_path_refresh(data_dir);
    if let Ok(Some(bytes)) = upstream_cache::read(data_dir, STEEL_PATH_FILE) {
        match serde_json::from_slice::<SteelPathApi>(&bytes) {
            Ok(api) => return brief_from_api(api, now_ms),
            Err(e) => eprintln!("[仓鼠] 钢铁之路阁下缓存损坏，回退周序: {e}"),
        }
    }
    fallback_brief(now_ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teshin_week_of_2026_10_05_is_forma() {
        let ms = chrono::DateTime::parse_from_rfc3339("2026-10-09T12:00:00Z")
            .unwrap()
            .timestamp_millis();
        assert_eq!(teshin_index(ms), 3);
        assert_eq!(TESHIN_FALLBACK[3].0, "3x Forma");
    }

    #[test]
    fn localize_known_names() {
        assert_eq!(teshin_name_zh("Rifle Riven Mod"), "步枪裂罅 Mod");
        assert_eq!(teshin_name_zh("50,000 Kuva"), "50,000 赤毒");
    }
}
