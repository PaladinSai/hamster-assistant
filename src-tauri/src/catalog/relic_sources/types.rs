//! 遗物来源表 DTO。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelicSourceQuery {
    pub query: Option<String>,
    pub relic_key: Option<String>,
    pub relic_keys: Option<Vec<String>>,
    pub eras: Option<Vec<i32>>,
    pub game_modes: Option<Vec<String>>,
    pub rotations: Option<Vec<String>>,
    pub planets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicLocationDto {
    pub source_kind: String,
    pub planet_or_hub: String,
    pub planet_or_hub_zh: Option<String>,
    pub node_or_bounty: String,
    pub node_or_bounty_zh: Option<String>,
    pub game_mode: Option<String>,
    pub game_mode_zh: Option<String>,
    pub rotation: Option<String>,
    pub chance: Option<f64>,
    pub stage: Option<String>,
    pub is_event: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicMatchedItemDto {
    pub item_name_en: String,
    pub item_name_zh: Option<String>,
    pub rarity: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicSourceItemDto {
    pub relic_key: String,
    pub name_zh: String,
    pub era: i32,
    pub vaulted: i32,
    pub matched_items: Vec<RelicMatchedItemDto>,
    pub locations: Vec<RelicLocationDto>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RelicSourceFilterOptions {
    pub relics: Vec<FilterChip>,
    pub modes: Vec<FilterChip>,
    pub planets: Vec<FilterChip>,
    pub rotations: Vec<String>,
    pub has_blank_rotation: bool,
    pub has_bounty: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterChip {
    pub key: String,
    pub label_zh: String,
    pub era: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelicSourceSearchResult {
    pub items: Vec<RelicSourceItemDto>,
    pub truncated: bool,
    pub filter_options: RelicSourceFilterOptions,
}
