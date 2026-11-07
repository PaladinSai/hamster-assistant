//! Relics.json 拉取。只请求一个代理地址；奖励为空则视为失败，避免清空已有掉落。

use crate::catalog::wfcd::{fetch_json_url, FetchResult};

use super::parse::parse_intact_relics;

pub fn fetch_relics(url: &str, if_none_match: Option<&str>) -> Result<FetchResult, String> {
    let fetched = fetch_json_url(url, if_none_match, 90)?;
    if !fetched.not_modified {
        reject_empty_intact_rewards(&fetched.body)?;
    }
    Ok(fetched)
}

/// Intact 遗物存在但奖励条数为 0：正文不可用，调用方不得导入。
pub(crate) fn reject_empty_intact_rewards(body: &[u8]) -> Result<(), String> {
    let relics = parse_intact_relics(body)?;
    let rewards: usize = relics.iter().map(|r| r.all_rewards.len()).sum();
    if !relics.is_empty() && rewards == 0 {
        return Err("遗物正文没有 Intact 奖励，已取消导入并保留旧数据".into());
    }
    Ok(())
}
