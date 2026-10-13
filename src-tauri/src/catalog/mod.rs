//! Catalog 域：WFCD 上游 / 入库 / 时效世界状态。
//!
//! - [`wfcd`]：URL 常量 + fetch + 轻量 parse（不写库）
//! - [`import`]：机体入库事务；[`weapons`]：武器家族规则 + 入库
//! - [`modular`]：自制精通部件（武器 kitgun/zaw/amp ↔ 同伴 moa/hound）概念与共用小 helper
//! - [`live_world`]：worldState；其中 `parse` 按突击/赏金/裂隙等拆子模块

mod arcanes;
mod drop_locations;
mod i18n;
mod import;
mod live_world;
mod mods;
mod modular;
mod nicknames;
mod refresh;
mod relic_sources;
mod relics;
mod resurgence;
mod upstream_cache;
mod weapons;
mod wfcd;

pub use arcanes::{
    backfill_arcane_name_zh, copies_to_max, import_arcanes_json, status_from_rank,
    ArcaneImportStats,
};
pub use drop_locations::{
    heal_empty_relic_locations, import_drop_locations_json, refresh_drop_locations,
    refresh_drop_locations_conn, DropLocationStats, DEFAULT_DROP_ALL_URLS,
};
pub use i18n::{
    apply_display_zh_to_entities, ensure_mission_type_i18n, ensure_name_i18n_from_bundled,
    fetch_huiji_name_i18n, parse_i18n_from_file, parse_wfcd_unique_zh, upsert_name_i18n,
    upsert_name_i18n_userdict, upsert_name_i18n_wfcd, upsert_wfcd_i18n_by_entities, ZhLookup,
    DISPLAY_ZH_SQL,
};


pub use import::{
    import_archwing_json, import_modular_companions_json, import_pets_json,
    import_sentinels_json, import_warframes_json,
    ImportStats,
};
pub use live_world::{
    force_refresh_live_nightwave, force_refresh_live_routines, force_refresh_live_world,
    get_arbitration_brief, get_bounty_brief, get_bounty_watch, get_fissure_brief,
    get_live_daily_brief, get_plains_brief, get_standing_daily, get_standing_ranks,
    get_weekly_shop_brief, match_and_record_notifications, refresh_live_world_if_needed,
    set_bounty_watch, set_standing_daily, set_standing_ranks, ArbitrationBriefDto, BountyAlertDto,
    BountyBriefApi, BountyWatchPrefs, FissureBriefDto, LiveDailyBrief, LiveWorldOutcome,
    PlainsBriefDto, StandingDailyPrefs, StandingRanksPrefs, WeeklyShopBrief,
};
pub use mods::{import_mods_json, ModImportStats, SRC_KEY, SRC_NIGHTMARE, SRC_SYNDICATE};
pub use nicknames::{
    apply_bundled_warframe_nicknames, apply_nicknames_map, read_nicknames_json,
};
pub use refresh::{
    force_refresh_catalog, refresh_catalog_if_needed, update_arcanes_catalog, update_mods_catalog,
    update_relics_catalog, update_warframes_catalog, update_weapons_catalog, RefreshOutcome,
};
pub use relic_sources::{
    get_relic_source_filter_options, search_relic_sources, RelicSourceFilterOptions,
    RelicSourceQuery, RelicSourceSearchResult,
};
pub use relics::{import_relics_json, RelicImportStats, DEFAULT_RELICS_URLS};
pub use resurgence::{
    force_refresh_resurgence, get_resurgence_summary, load_resurgence_relic_keys,
    refresh_resurgence_if_needed, ResurgenceOutcome, ResurgenceSummary,
};
pub use weapons::{
    import_archwing_weapons_json, import_modular_weapons_json, import_weapons_json,
    is_incarnon_genesis_family, weapon_family_and_variant, WeaponImportStats,
};
pub use wfcd::{
    fetch_archwings, fetch_pets, fetch_sentinels, fetch_warframes, DEFAULT_ARCHWING_URLS,
    DEFAULT_I18N_URLS, DEFAULT_MISC_URLS, DEFAULT_PETS_URLS, DEFAULT_SENTINELS_URLS,
    DEFAULT_WARFRAMES_URLS,
};
