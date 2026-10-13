//! HTTP 拉取（多 URL 回退 + ETag）。

use std::sync::OnceLock;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use reqwest::StatusCode;

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub url: String,
    /// true = 304 Not Modified，body 为空
    pub not_modified: bool,
}

fn shared_client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent("cangshu-assistant/0.1 (Warframe catalog sync; +https://github.com/)")
            // 单次请求超时由 RequestBuilder::timeout 覆盖；此处给连接池一个宽松上限
            .timeout(Duration::from_secs(180))
            .build()
            .expect("创建 HTTP 客户端失败")
    })
}

pub fn fetch_archwings(
    urls: &[&str],
    prev_etag: Option<&str>,
) -> Result<FetchResult, String> {
    fetch_warframes(urls, prev_etag)
}

pub fn fetch_sentinels(
    urls: &[&str],
    prev_etag: Option<&str>,
) -> Result<FetchResult, String> {
    fetch_warframes(urls, prev_etag)
}

pub fn fetch_pets(
    urls: &[&str],
    prev_etag: Option<&str>,
) -> Result<FetchResult, String> {
    fetch_warframes(urls, prev_etag)
}

pub fn fetch_warframes(
    urls: &[&str],
    if_none_match: Option<&str>,
) -> Result<FetchResult, String> {
    fetch_json_urls(urls, if_none_match, 60)
}

/// 通用 JSON 拉取（多 URL 回退）。共用 Client 连接池；`timeout_secs` 作用于单次请求。
pub fn fetch_json_urls(
    urls: &[&str],
    if_none_match: Option<&str>,
    timeout_secs: u64,
) -> Result<FetchResult, String> {
    let client = shared_client();
    let timeout = Duration::from_secs(timeout_secs.max(1));

    let mut last_err = String::from("无可用上游 URL");
    for url in urls {
        match fetch_one(client, url, if_none_match, timeout) {
            Ok(r) => return Ok(r),
            Err(e) => {
                last_err = format!("{url}: {e}");
                eprintln!("[仓鼠] 拉取失败，尝试下一源: {last_err}");
            }
        }
    }
    Err(last_err)
}

fn fetch_one(
    client: &Client,
    url: &str,
    if_none_match: Option<&str>,
    timeout: Duration,
) -> Result<FetchResult, String> {
    let mut req = client.get(url).timeout(timeout);
    if let Some(etag) = if_none_match {
        if !etag.is_empty() {
            req = req.header(IF_NONE_MATCH, etag);
        }
    }
    let resp = req.send().map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let etag = resp
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    if status == StatusCode::NOT_MODIFIED {
        return Ok(FetchResult {
            body: Vec::new(),
            etag,
            url: url.to_string(),
            not_modified: true,
        });
    }
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let body = resp
        .bytes()
        .map_err(|e| format!("读响应体失败: {e}"))?
        .to_vec();
    Ok(FetchResult {
        body,
        etag,
        url: url.to_string(),
        not_modified: false,
    })
}
