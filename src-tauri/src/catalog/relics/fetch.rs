//! Relics.json 拉取。

use reqwest::blocking::Client;
use reqwest::header::{ETAG, IF_NONE_MATCH};
use reqwest::StatusCode;

use crate::catalog::wfcd::FetchResult;

pub fn fetch_relics(urls: &[&str], if_none_match: Option<&str>) -> Result<FetchResult, String> {
    let client = Client::builder()
        .user_agent("cangshu-assistant/0.1 (Warframe catalog sync; +https://github.com/)")
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let mut last_err = String::from("无可用上游 URL");
    for url in urls {
        match fetch_one(&client, url, if_none_match) {
            Ok(r) => return Ok(r),
            Err(e) => {
                last_err = format!("{url}: {e}");
                eprintln!("[仓鼠] 遗物拉取失败，尝试下一源: {last_err}");
            }
        }
    }
    Err(last_err)
}

pub(crate) fn fetch_one(
    client: &Client,
    url: &str,
    if_none_match: Option<&str>,
) -> Result<FetchResult, String> {
    let mut req = client.get(url);
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
        .map_err(|e| format!("读响应失败: {e}"))?
        .to_vec();
    Ok(FetchResult {
        body,
        etag,
        url: url.to_string(),
        not_modified: false,
    })
}
