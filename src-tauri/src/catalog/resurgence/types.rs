//! 重生相关类型与常量。

use serde::{Deserialize, Serialize};

pub(crate) const VAULT_TRADER_URL: &str = "https://api.warframestat.us/pc/vaultTrader";
/// 官方世界状态。warframestat 常被 Cloudflare 直接 403，这是能拉到 Varzia 的来源。
pub(crate) const WORLDSTATE_URL: &str = "https://api.warframe.com/cdn/worldState.php";
/// warframestat 会拒绝脚本样式的 User-Agent。
pub(crate) const VAULT_TRADER_USER_AGENT: &str = "Mozilla/5.0";
pub(crate) const SETTING_CHECKED_AT: &str = "resurgence_checked_at";
pub(crate) const SETTING_ETAG: &str = "resurgence_etag";
pub(crate) const SETTING_SOURCE_URL: &str = "resurgence_source_url";

#[derive(Debug, Clone)]
pub enum ResurgenceOutcome {
    /// 本地轮次未过期，未访问网络
    SkippedFresh,
    /// 条件请求 304，上游未变
    NotModified,
    Updated {
        warframes: usize,
        weapons: usize,
        relics: usize,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct VaultTrader {
    pub(crate) activation: Option<String>,
    pub(crate) expiry: Option<String>,
    #[serde(default)]
    pub(crate) inventory: Vec<VaultItem>,
    /// warframestat 备用源可能带下期预告
    #[serde(default)]
    pub(crate) schedule: Vec<VaultScheduleEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct VaultScheduleEntry {
    pub(crate) expiry: Option<String>,
    pub(crate) item: Option<String>,
    #[serde(rename = "uniqueName")]
    pub(crate) unique_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct NextPreview {
    pub(crate) activation: Option<String>,
    pub(crate) expiry: Option<String>,
    /// 包路径或战甲展示名
    pub(crate) featured: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VaultFetch {
    pub(crate) current: VaultTrader,
    pub(crate) next: Option<NextPreview>,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct VaultItem {
    #[serde(rename = "uniqueName")]
    pub(crate) unique_name: Option<String>,
    pub(crate) item: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedItem {
    pub(crate) kind: &'static str,
    pub(crate) name_en: String,
    pub(crate) unique_name: String,
    pub(crate) family_key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResurgenceWarframeDto {
    pub family_id: Option<String>,
    pub family_key: String,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResurgenceWeaponDto {
    pub weapon_id: Option<String>,
    pub family_id: Option<String>,
    pub name_en: String,
    pub name_zh: Option<String>,
    pub unique_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResurgenceNextDto {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub warframes: Vec<ResurgenceWarframeDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResurgenceSummary {
    pub activation: Option<String>,
    pub expiry: Option<String>,
    pub warframes: Vec<ResurgenceWarframeDto>,
    pub weapons: Vec<ResurgenceWeaponDto>,
    pub relic_count: i64,
    pub fresh: bool,
    pub next: Option<ResurgenceNextDto>,
}

#[derive(Debug, Clone)]
pub(crate) struct VaultHttpFetch {
    pub(crate) vault: VaultFetch,
    pub(crate) source_url: String,
    pub(crate) raw_body: Vec<u8>,
    pub(crate) etag: Option<String>,
    pub(crate) cache_name: &'static str,
    pub(crate) not_modified: bool,
}
