//! 重生上游拉取与解析。

use chrono::{TimeZone, Utc};
use reqwest::blocking::Client;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use reqwest::StatusCode;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;

use crate::catalog::upstream_cache;

use super::types::*;

pub(crate) fn fetch_vault_trader(
    prev_etag: Option<&str>,
    prev_source: Option<&str>,
) -> Result<VaultHttpFetch, String> {
    let client = Client::builder()
        .user_agent(VAULT_TRADER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(45))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let ws_etag = etag_for_url(prev_etag, prev_source, WORLDSTATE_URL);
    let vt_etag = etag_for_url(prev_etag, prev_source, VAULT_TRADER_URL);

    match fetch_bytes(&client, WORLDSTATE_URL, ws_etag) {
        Ok(raw) if raw.not_modified => {
            return Ok(VaultHttpFetch {
                vault: VaultFetch {
                    current: VaultTrader {
                        activation: None,
                        expiry: None,
                        inventory: Vec::new(),
                        schedule: Vec::new(),
                    },
                    next: None,
                },
                source_url: WORLDSTATE_URL.to_string(),
                raw_body: Vec::new(),
                etag: raw.etag,
                cache_name: upstream_cache::WORLDSTATE,
                not_modified: true,
            });
        }
        Ok(raw) => match trader_from_worldstate(&raw.body) {
            Ok(vault) => {
                return Ok(VaultHttpFetch {
                    vault,
                    source_url: WORLDSTATE_URL.to_string(),
                    raw_body: raw.body,
                    etag: raw.etag,
                    cache_name: upstream_cache::WORLDSTATE,
                    not_modified: false,
                });
            }
            Err(official_err) => {
                match fetch_bytes(&client, VAULT_TRADER_URL, None) {
                    Ok(body) if body.not_modified => {
                        return Err(format!("{official_err}；备用接口意外 304"));
                    }
                    Ok(body) => {
                        let trader: VaultTrader = serde_json::from_slice(&body.body)
                            .map_err(|e| format!("Varzia JSON 解析失败: {e}"))?;
                        let next = next_from_warframestat_schedule(&trader);
                        return Ok(VaultHttpFetch {
                            vault: VaultFetch {
                                current: trader,
                                next,
                            },
                            source_url: VAULT_TRADER_URL.to_string(),
                            raw_body: body.body,
                            etag: body.etag,
                            cache_name: upstream_cache::VAULT_TRADER,
                            not_modified: false,
                        });
                    }
                    Err(stat_err) => {
                        return Err(format!("{official_err}；备用接口也失败：{stat_err}"));
                    }
                }
            }
        },
        Err(official_err) => match fetch_bytes(&client, VAULT_TRADER_URL, vt_etag) {
            Ok(raw) if raw.not_modified => {
                return Ok(VaultHttpFetch {
                    vault: VaultFetch {
                        current: VaultTrader {
                            activation: None,
                            expiry: None,
                            inventory: Vec::new(),
                            schedule: Vec::new(),
                        },
                        next: None,
                    },
                    source_url: VAULT_TRADER_URL.to_string(),
                    raw_body: Vec::new(),
                    etag: raw.etag,
                    cache_name: upstream_cache::VAULT_TRADER,
                    not_modified: true,
                });
            }
            Ok(raw) => {
                let trader: VaultTrader = serde_json::from_slice(&raw.body)
                    .map_err(|e| format!("Varzia JSON 解析失败: {e}"))?;
                let next = next_from_warframestat_schedule(&trader);
                Ok(VaultHttpFetch {
                    vault: VaultFetch {
                        current: trader,
                        next,
                    },
                    source_url: VAULT_TRADER_URL.to_string(),
                    raw_body: raw.body,
                    etag: raw.etag,
                    cache_name: upstream_cache::VAULT_TRADER,
                    not_modified: false,
                })
            }
            Err(stat_err) => Err(format!("{official_err}；备用接口也失败：{stat_err}")),
        },
    }
}

pub(crate) fn etag_for_url<'a>(
    prev_etag: Option<&'a str>,
    prev_source: Option<&str>,
    url: &str,
) -> Option<&'a str> {
    match (prev_etag, prev_source) {
        (Some(etag), Some(src)) if src == url => Some(etag),
        _ => None,
    }
}

pub(crate) struct RawFetch {
    body: Vec<u8>,
    etag: Option<String>,
    not_modified: bool,
}

pub(crate) fn fetch_bytes(client: &Client, url: &str, prev_etag: Option<&str>) -> Result<RawFetch, String> {
    let mut req = client.get(url);
    if let Some(etag) = prev_etag {
        req = req.header(IF_NONE_MATCH, etag);
    }
    let resp = req
        .send()
        .map_err(|e| format!("请求 {url} 失败: {e}"))?;
    let status = resp.status();
    let etag = resp
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    if status == StatusCode::NOT_MODIFIED {
        return Ok(RawFetch {
            body: Vec::new(),
            etag,
            not_modified: true,
        });
    }
    let body = resp
        .bytes()
        .map_err(|e| format!("读 {url} 失败: {e}"))?
        .to_vec();
    if !status.is_success() {
        return Err(format!("HTTP {status} {url}: {}", body_snippet(&body)));
    }
    Ok(RawFetch {
        body,
        etag,
        not_modified: false,
    })
}

pub(crate) fn trader_from_worldstate(body: &[u8]) -> Result<VaultFetch, String> {
    let state: DeWorldState = serde_json::from_slice(body)
        .map_err(|e| format!("世界状态 JSON 解析失败: {e}"))?;
    let raw = state
        .prime_vault_traders
        .into_iter()
        .flatten()
        .next()
        .ok_or_else(|| "世界状态没有 PrimeVaultTraders".to_string())?;
    let inventory = raw
        .manifest
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.item_type.map(|unique_name| VaultItem {
                unique_name: Some(unique_name),
                item: None,
            })
        })
        .collect();
    let activation = raw.activation.as_ref().and_then(de_date_to_rfc3339);
    let expiry = raw.expiry.as_ref().and_then(de_date_to_rfc3339);
    let next = next_from_schedule_info(expiry.as_deref(), raw.schedule_info.as_deref());
    Ok(VaultFetch {
        current: VaultTrader {
            activation,
            expiry,
            inventory,
            schedule: Vec::new(),
        },
        next,
    })
}

/// ScheduleInfo：Expiry 大于当期、且已有 FeaturedItem 的最早一条。
pub(crate) fn next_from_schedule_info(
    current_expiry: Option<&str>,
    schedule: Option<&[DeScheduleEntry]>,
) -> Option<NextPreview> {
    let current_expiry = current_expiry?;
    let schedule = schedule?;
    let mut best: Option<(String, String)> = None;
    for entry in schedule {
        let Some(exp) = entry.expiry.as_ref().and_then(de_date_to_rfc3339) else {
            continue;
        };
        if exp.as_str() <= current_expiry {
            continue;
        }
        let Some(featured) = entry.featured_item.as_ref() else {
            continue;
        };
        if featured.trim().is_empty() {
            continue;
        }
        match &best {
            None => best = Some((exp, featured.clone())),
            Some((prev_exp, _)) if &exp < prev_exp => best = Some((exp, featured.clone())),
            _ => {}
        }
    }
    best.map(|(exp, featured)| NextPreview {
        activation: Some(current_expiry.to_string()),
        expiry: Some(exp),
        featured,
    })
}

pub(crate) fn next_from_warframestat_schedule(trader: &VaultTrader) -> Option<NextPreview> {
    let current_expiry = trader.expiry.as_deref()?;
    let mut best: Option<(String, String)> = None;
    for entry in &trader.schedule {
        let Some(exp) = entry.expiry.as_ref() else {
            continue;
        };
        if exp.as_str() <= current_expiry {
            continue;
        }
        let Some(featured) = entry
            .unique_name
            .clone()
            .or_else(|| entry.item.clone())
            .filter(|s| !s.trim().is_empty())
        else {
            continue;
        };
        match &best {
            None => best = Some((exp.clone(), featured)),
            Some((prev_exp, _)) if exp < prev_exp => best = Some((exp.clone(), featured)),
            _ => {}
        }
    }
    best.map(|(exp, featured)| NextPreview {
        activation: Some(current_expiry.to_string()),
        expiry: Some(exp),
        featured,
    })
}

pub(crate) fn de_date_to_rfc3339(date: &DeDate) -> Option<String> {
    let ms: i64 = date.date.number_long.parse().ok()?;
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.to_rfc3339())
}

pub(crate) fn body_snippet(body: &[u8]) -> String {
    String::from_utf8_lossy(body)
        .chars()
        .filter(|c| !c.is_control())
        .take(300)
        .collect()
}

#[derive(Debug, Deserialize)]
struct DeWorldState {
    #[serde(rename = "PrimeVaultTraders")]
    prime_vault_traders: Option<Vec<DePrimeVaultTrader>>,
}

#[derive(Debug, Deserialize)]
struct DePrimeVaultTrader {
    #[serde(rename = "Activation")]
    activation: Option<DeDate>,
    #[serde(rename = "Expiry")]
    expiry: Option<DeDate>,
    #[serde(rename = "Manifest")]
    manifest: Option<Vec<DeManifestItem>>,
    #[serde(rename = "ScheduleInfo")]
    schedule_info: Option<Vec<DeScheduleEntry>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeScheduleEntry {
    #[serde(rename = "Expiry")]
    expiry: Option<DeDate>,
    #[serde(rename = "FeaturedItem")]
    featured_item: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeDate {
    #[serde(rename = "$date")]
    date: DeNumberLong,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeNumberLong {
    #[serde(rename = "$numberLong")]
    number_long: String,
}

#[derive(Debug, Deserialize)]
struct DeManifestItem {
    #[serde(rename = "ItemType")]
    item_type: Option<String>,
}

pub(crate) fn relic_key_for_projection(
    conn: &Connection,
    unique_name: &str,
) -> Result<Option<String>, String> {
    let tail = unique_name.rsplit('/').next().unwrap_or(unique_name);
    if tail.is_empty() {
        return Ok(None);
    }
    conn.query_row(
        "SELECT relic_key FROM void_relic WHERE unique_name LIKE '%/' || ?1 LIMIT 1",
        [tail],
        |r| r.get(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}

