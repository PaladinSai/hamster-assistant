//! Mod 常量与 WFCD DTO。

use serde::Deserialize;

/// 品类码（与 11-表结构草案一致）
pub const MOD_KIND_WARFRAME: i32 = 0;
pub const MOD_KIND_PRIMARY: i32 = 1;
pub const MOD_KIND_SECONDARY: i32 = 2;
pub const MOD_KIND_MELEE: i32 = 3;
pub const MOD_KIND_AURA: i32 = 4;
pub const MOD_KIND_STANCE: i32 = 5;
pub const MOD_KIND_COMPANION: i32 = 6;
pub const MOD_KIND_OTHER: i32 = 7;

/// 来源码（优先级：数值越小不一定更高；用 `source_rank` 比较）
pub const SRC_OTHER: i32 = 0;
pub const SRC_ENEMY: i32 = 1;
pub const SRC_MISSION: i32 = 2;
pub const SRC_CETUS: i32 = 3;
pub const SRC_SOLARIS: i32 = 4;
pub const SRC_DEIMOS: i32 = 5;
pub const SRC_ZARIMAN: i32 = 6;
pub const SRC_HEX: i32 = 7;
pub const SRC_KEY: i32 = 8;
pub const SRC_SORTIE: i32 = 9;
pub const SRC_SYNDICATE: i32 = 10;
pub const SRC_TRANSIENT: i32 = 11;
pub const SRC_NIGHTMARE: i32 = 12;

#[derive(Debug, Default, Clone)]
pub struct ModImportStats {
    pub mods: usize,
    pub skipped: usize,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WfcdIntroduced {
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WfcdMod {
    pub(crate) name: String,
    #[serde(rename = "uniqueName")]
    pub(crate) unique_name: String,
    #[serde(default)]
    #[serde(rename = "type")]
    pub(crate) item_type: Option<String>,
    #[serde(default)]
    #[serde(rename = "compatName")]
    pub(crate) compat_name: Option<String>,
    #[serde(default)]
    pub(crate) polarity: Option<String>,
    #[serde(default)]
    #[serde(rename = "isPrime")]
    pub(crate) is_prime: Option<bool>,
    #[serde(default)]
    #[serde(rename = "isAugment")]
    pub(crate) is_augment: Option<bool>,
    #[serde(default)]
    #[serde(rename = "releaseDate")]
    pub(crate) release_date: Option<String>,
    #[serde(default)]
    pub(crate) introduced: Option<WfcdIntroduced>,
    #[serde(default)]
    #[serde(rename = "wikiaUrl")]
    pub(crate) wikia_url: Option<String>,
}

pub(crate) fn name_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}
