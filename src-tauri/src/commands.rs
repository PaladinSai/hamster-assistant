//! Tauri command 薄封装（业务在 catalog / progress）。
//!
//! 后台 kick / busy / emit 仍在 [`crate`] 根（`lib.rs`），供本模块与 ticker 共用。

use std::sync::Arc;

use tauri::State;

use crate::catalog::{
    force_refresh_live_routines, force_refresh_live_world, force_refresh_resurgence,
    get_arbitration_brief, get_bounty_brief, get_bounty_watch, get_fissure_brief,
    get_live_daily_brief, get_plains_brief, get_relic_source_filter_options,
    get_resurgence_summary, get_standing_daily, get_standing_ranks, get_weekly_shop_brief,
    search_relic_sources, set_bounty_watch, set_standing_daily, set_standing_ranks,
    update_arcanes_catalog, update_mods_catalog, update_relics_catalog,
    update_warframes_catalog, update_weapons_catalog, ArbitrationBriefDto, BountyBriefApi,
    BountyWatchPrefs, FissureBriefDto, LiveDailyBrief, LiveWorldOutcome, PlainsBriefDto,
    RelicSourceFilterOptions, RelicSourceQuery, RelicSourceSearchResult, ResurgenceOutcome,
    ResurgenceSummary, StandingDailyPrefs, StandingRanksPrefs, WeeklyShopBrief,
};
use crate::db;
use crate::progress::{
    self, CreateRoutine, ReorderRoutines, ReorderUserTodos, UpdateRoutine, UpdateUserTodo,
    UpsertArcaneProgress, UpsertModProgress, UpsertPartProgress, UpsertWarframeProgress,
    UpsertWeaponPartProgress, UpsertWeaponProgress,
};
use crate::{
    emit_arcanes_updated, emit_live_updated, emit_mods_updated, emit_nightwave_updated,
    emit_relics_updated, emit_resurgence_updated, emit_warframes_updated, emit_weapons_updated,
    push_bounty_notifications, spawn_catalog_job, spawn_dual_busy_job, try_begin, AppState,
    BusyGuard,
};

#[tauri::command]
pub fn get_db_status(state: State<'_, AppState>) -> Result<db::DbStatus, String> {
    db::init_databases(&state.data_dir)
}

#[tauri::command]
pub fn next_id(state: State<'_, AppState>) -> Result<i64, String> {
    Ok(state.snowflake.next_id())
}

#[tauri::command]
pub fn list_warframes(
    state: State<'_, AppState>,
    kind: Option<String>,
) -> Result<Vec<progress::WarframeListItem>, String> {
    progress::list_warframes(&state.data_dir, kind.as_deref())
}

#[tauri::command]
pub fn get_warframe_detail(
    state: State<'_, AppState>,
    warframe_id: String,
) -> Result<progress::WarframeDetail, String> {
    progress::get_warframe_detail(&state.data_dir, &warframe_id)
}

#[tauri::command]
pub fn upsert_progress_warframe(
    state: State<'_, AppState>,
    patch: UpsertWarframeProgress,
) -> Result<(), String> {
    progress::upsert_progress_warframe(&state.data_dir, patch)
}

#[tauri::command]
pub fn upsert_progress_part(
    state: State<'_, AppState>,
    patch: UpsertPartProgress,
) -> Result<(), String> {
    progress::upsert_progress_part(&state.data_dir, patch)
}

#[tauri::command]
pub fn list_weapon_families(
    state: State<'_, AppState>,
    kind: Option<String>,
) -> Result<Vec<progress::WeaponFamilyListItem>, String> {
    progress::list_weapon_families(&state.data_dir, kind.as_deref())
}

#[tauri::command]
pub fn get_weapon_family_detail(
    state: State<'_, AppState>,
    family_id: String,
) -> Result<progress::WeaponFamilyDetail, String> {
    progress::get_weapon_family_detail(&state.data_dir, &family_id)
}

#[tauri::command]
pub fn upsert_weapon_progress(
    state: State<'_, AppState>,
    patch: UpsertWeaponProgress,
) -> Result<(), String> {
    progress::upsert_weapon_progress(&state.data_dir, patch)
}

#[tauri::command]
pub fn upsert_weapon_part_progress(
    state: State<'_, AppState>,
    patch: UpsertWeaponPartProgress,
) -> Result<(), String> {
    progress::upsert_weapon_part_progress(&state.data_dir, patch)
}

#[tauri::command]
pub fn list_mods(state: State<'_, AppState>) -> Result<Vec<progress::ModListItem>, String> {
    progress::list_mods(&state.data_dir)
}

#[tauri::command]
pub fn list_mod_tags(state: State<'_, AppState>) -> Result<Vec<progress::ModTagDto>, String> {
    progress::list_mod_tags(&state.data_dir)
}

#[tauri::command]
pub fn create_mod_tag(
    state: State<'_, AppState>,
    name: String,
) -> Result<progress::ModTagDto, String> {
    progress::create_mod_tag(&state.data_dir, &name)
}

#[tauri::command]
pub fn delete_mod_tag(state: State<'_, AppState>, id: String) -> Result<(), String> {
    progress::delete_mod_tag(&state.data_dir, &id)
}

#[tauri::command]
pub fn upsert_mod_progress(
    state: State<'_, AppState>,
    patch: UpsertModProgress,
) -> Result<(), String> {
    progress::upsert_mod_progress(&state.data_dir, patch)
}

#[tauri::command]
pub fn list_arcanes(state: State<'_, AppState>) -> Result<Vec<progress::ArcaneListItem>, String> {
    progress::list_arcanes(&state.data_dir)
}

#[tauri::command]
pub fn upsert_arcane_progress(
    state: State<'_, AppState>,
    patch: UpsertArcaneProgress,
) -> Result<(), String> {
    progress::upsert_arcane_progress(&state.data_dir, patch)
}

#[tauri::command]
pub fn list_user_todos(state: State<'_, AppState>) -> Result<Vec<progress::UserTodoDto>, String> {
    progress::list_user_todos(&state.data_dir)
}

#[tauri::command]
pub fn create_user_todo(
    state: State<'_, AppState>,
    body: String,
) -> Result<progress::UserTodoDto, String> {
    progress::create_user_todo(&state.data_dir, &body)
}

#[tauri::command]
pub fn update_user_todo(
    state: State<'_, AppState>,
    patch: UpdateUserTodo,
) -> Result<progress::UserTodoDto, String> {
    progress::update_user_todo(&state.data_dir, patch)
}

#[tauri::command]
pub fn delete_user_todo(state: State<'_, AppState>, id: String) -> Result<(), String> {
    progress::delete_user_todo(&state.data_dir, &id)
}

#[tauri::command]
pub fn reorder_user_todos(
    state: State<'_, AppState>,
    input: ReorderUserTodos,
) -> Result<(), String> {
    progress::reorder_user_todos(&state.data_dir, input)
}

#[tauri::command]
pub fn list_routines(state: State<'_, AppState>) -> Result<Vec<progress::RoutineItemDto>, String> {
    progress::list_routines(&state.data_dir)
}

#[tauri::command]
pub fn create_routine(
    state: State<'_, AppState>,
    input: CreateRoutine,
) -> Result<progress::RoutineItemDto, String> {
    progress::create_routine(&state.data_dir, input)
}

#[tauri::command]
pub fn update_routine(
    state: State<'_, AppState>,
    patch: UpdateRoutine,
) -> Result<progress::RoutineItemDto, String> {
    progress::update_routine(&state.data_dir, patch)
}

#[tauri::command]
pub fn delete_routine(state: State<'_, AppState>, id: String) -> Result<(), String> {
    progress::delete_routine(&state.data_dir, &id)
}

#[tauri::command]
pub fn reorder_routines(
    state: State<'_, AppState>,
    input: ReorderRoutines,
) -> Result<(), String> {
    progress::reorder_routines(&state.data_dir, input)
}

#[tauri::command]
pub fn get_resurgence(state: State<'_, AppState>) -> Result<ResurgenceSummary, String> {
    get_resurgence_summary(&state.data_dir)
}

#[tauri::command]
pub fn get_live_daily_brief_cmd(
    state: State<'_, AppState>,
) -> Result<LiveDailyBrief, String> {
    get_live_daily_brief(&state.data_dir, state.snowflake.as_ref())
}

#[tauri::command]
pub fn get_weekly_shop_brief_cmd(
    state: State<'_, AppState>,
) -> Result<WeeklyShopBrief, String> {
    Ok(get_weekly_shop_brief(&state.data_dir))
}

#[tauri::command]
pub fn get_bounty_brief_cmd(
    state: State<'_, AppState>,
) -> Result<BountyBriefApi, String> {
    get_bounty_brief(&state.data_dir, state.snowflake.as_ref())
}

#[tauri::command]
pub fn get_fissure_brief_cmd(
    state: State<'_, AppState>,
) -> Result<FissureBriefDto, String> {
    get_fissure_brief(&state.data_dir, state.snowflake.as_ref())
}

#[tauri::command]
pub fn get_arbitration_brief_cmd(
    state: State<'_, AppState>,
) -> Result<ArbitrationBriefDto, String> {
    get_arbitration_brief(&state.data_dir)
}

#[tauri::command]
pub fn get_plains_brief_cmd(
    state: State<'_, AppState>,
) -> Result<PlainsBriefDto, String> {
    get_plains_brief(&state.data_dir)
}

#[tauri::command]
pub fn get_bounty_watch_cmd(
    state: State<'_, AppState>,
) -> Result<BountyWatchPrefs, String> {
    get_bounty_watch(&state.data_dir)
}

#[tauri::command]
pub fn set_bounty_watch_cmd(
    state: State<'_, AppState>,
    prefs: BountyWatchPrefs,
) -> Result<BountyWatchPrefs, String> {
    set_bounty_watch(&state.data_dir, prefs)
}

#[tauri::command]
pub fn get_standing_ranks_cmd(
    state: State<'_, AppState>,
) -> Result<StandingRanksPrefs, String> {
    get_standing_ranks(&state.data_dir)
}

#[tauri::command]
pub fn set_standing_ranks_cmd(
    state: State<'_, AppState>,
    prefs: StandingRanksPrefs,
) -> Result<StandingRanksPrefs, String> {
    set_standing_ranks(&state.data_dir, prefs)
}

#[tauri::command]
pub fn get_standing_daily_cmd(
    state: State<'_, AppState>,
) -> Result<StandingDailyPrefs, String> {
    get_standing_daily(&state.data_dir)
}

#[tauri::command]
pub fn set_standing_daily_cmd(
    state: State<'_, AppState>,
    prefs: StandingDailyPrefs,
) -> Result<StandingDailyPrefs, String> {
    set_standing_daily(&state.data_dir, prefs)
}

#[tauri::command]
pub fn kick_update_warframes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.catalog_busy),
        "kick-warframes",
        "图鉴更新进行中，请稍候",
        "启动战甲更新失败",
        move || match update_warframes_catalog(&data_dir, snowflake.as_ref()) {
            Ok(msg) => {
                emit_warframes_updated(&app_h, &msg);
                Ok(msg)
            }
            Err(e) => Err(format!("战甲更新失败: {e}")),
        },
    )
}

#[tauri::command]
pub fn kick_update_weapons(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.catalog_busy),
        "kick-weapons",
        "图鉴更新进行中，请稍候",
        "启动武器更新失败",
        move || match update_weapons_catalog(&data_dir, snowflake.as_ref()) {
            Ok(msg) => {
                emit_weapons_updated(&app_h, &msg);
                Ok(msg)
            }
            Err(e) => Err(format!("武器更新失败: {e}")),
        },
    )
}

#[tauri::command]
pub fn kick_update_mods(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.catalog_busy),
        "kick-mods",
        "图鉴更新进行中，请稍候",
        "启动模组更新失败",
        move || {
            let msg = update_mods_catalog(&data_dir, snowflake.as_ref())
                .map_err(|e| format!("模组更新失败: {e}"))?;
            let tagged = progress::apply_auto_mod_tags(&data_dir).unwrap_or(0);
            let msg = format!("{msg}；自动标签≈{tagged}");
            emit_mods_updated(&app_h, &msg);
            Ok(msg)
        },
    )
}

#[tauri::command]
pub fn kick_update_arcanes(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.catalog_busy),
        "kick-arcanes",
        "图鉴更新进行中，请稍候",
        "启动赋能更新失败",
        move || match update_arcanes_catalog(&data_dir, snowflake.as_ref()) {
            Ok(msg) => {
                emit_arcanes_updated(&app_h, &msg);
                Ok(msg)
            }
            Err(e) => Err(format!("赋能更新失败: {e}")),
        },
    )
}

#[tauri::command]
pub fn kick_update_relics(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.catalog_busy),
        "kick-relics",
        "图鉴更新进行中，请稍候",
        "启动遗物更新失败",
        move || match update_relics_catalog(&data_dir, snowflake.as_ref()) {
            Ok(msg) => {
                emit_relics_updated(&app_h, &msg);
                Ok(msg)
            }
            Err(e) => Err(format!("遗物更新失败: {e}")),
        },
    )
}

#[tauri::command]
pub fn kick_update_live_world(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    let app_h = app.clone();
    spawn_catalog_job(
        app,
        Arc::clone(&state.live_busy),
        "kick-live-force",
        "日常更新进行中，请稍候",
        "启动后台更新失败",
        move || {
            let msg = match force_refresh_live_routines(&data_dir, snowflake.as_ref()) {
                Ok(LiveWorldOutcome::SkippedFresh) => "日常/周常未过期（未拉取）".to_string(),
                Ok(LiveWorldOutcome::KeptCache) => "拉取失败，已保留本地缓存".to_string(),
                Ok(LiveWorldOutcome::Updated) => {
                    push_bounty_notifications(&app_h, &data_dir);
                    "日常/周常已更新".to_string()
                }
                Err(e) => return Err(format!("更新失败: {e}")),
            };
            emit_live_updated(&app_h, &msg);
            Ok(msg)
        },
    )
}

#[tauri::command]
pub fn kick_update_feed(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let data_dir = state.data_dir.clone();
    let snowflake = Arc::clone(&state.snowflake);
    spawn_dual_busy_job(
        app,
        Arc::clone(&state.live_busy),
        Arc::clone(&state.resurgence_busy),
        "kick-feed-force",
        "启动后台更新失败",
        move |app, live_busy, surge_busy| {
            let nw = if try_begin(live_busy) {
                let _guard = BusyGuard(Arc::clone(live_busy));
                match force_refresh_live_world(&data_dir, snowflake.as_ref()) {
                    Ok(LiveWorldOutcome::SkippedFresh) => "动态未过期（未拉取）".to_string(),
                    Ok(LiveWorldOutcome::KeptCache) => "动态拉取失败，已保留缓存".to_string(),
                    Ok(LiveWorldOutcome::Updated) => {
                        push_bounty_notifications(app, &data_dir);
                        emit_live_updated(app, "动态已更新");
                        emit_nightwave_updated(app, "午夜电波已更新");
                        "动态已更新".to_string()
                    }
                    Err(e) => format!("动态更新失败: {e}"),
                }
            } else {
                "动态更新进行中".to_string()
            };
            let surge = if try_begin(surge_busy) {
                let _guard = BusyGuard(Arc::clone(surge_busy));
                match force_refresh_resurgence(&data_dir, snowflake.as_ref()) {
                    Ok(ResurgenceOutcome::SkippedFresh) => "重生未过期（未拉取）".to_string(),
                    Ok(ResurgenceOutcome::NotModified) => "重生上游未变".to_string(),
                    Ok(ResurgenceOutcome::Updated {
                        warframes,
                        weapons,
                        relics,
                    }) => {
                        let msg = format!(
                            "重生已更新：战甲≈{warframes} 武器≈{weapons} 核桃≈{relics}"
                        );
                        emit_resurgence_updated(app, &msg);
                        msg
                    }
                    Err(e) => format!("重生更新失败: {e}"),
                }
            } else {
                "重生更新进行中".to_string()
            };
            [nw, surge]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" · ")
        },
    )
}

#[tauri::command]
pub fn search_relic_sources_cmd(
    state: State<'_, AppState>,
    query: RelicSourceQuery,
) -> Result<RelicSourceSearchResult, String> {
    search_relic_sources(&state.data_dir, query)
}

#[tauri::command]
pub fn get_relic_source_filters(
    state: State<'_, AppState>,
) -> Result<RelicSourceFilterOptions, String> {
    get_relic_source_filter_options(&state.data_dir)
}
