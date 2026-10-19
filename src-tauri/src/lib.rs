pub mod catalog;
pub mod catalog_migrate;
pub mod db;
pub mod id;
pub mod progress;
pub mod user_migrate;
mod commands;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use catalog::{
    heal_empty_relic_locations, match_and_record_notifications, refresh_live_world_if_needed,
    refresh_resurgence_if_needed, LiveWorldOutcome, ResurgenceOutcome,
};
use id::Snowflake;
use tauri::{Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_window_state::{Builder as WindowStateBuilder, StateFlags};

pub struct AppState {
    pub data_dir: PathBuf,
    pub snowflake: Arc<Snowflake>,
    /// 日常/电波刷新互斥（ticker 与 kick）
    pub live_busy: Arc<AtomicBool>,
    /// 重生刷新互斥
    pub resurgence_busy: Arc<AtomicBool>,
    /// 图鉴 kick 互斥
    pub catalog_busy: Arc<AtomicBool>,
}

fn try_begin(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
}

struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}









































fn push_bounty_notifications(app: &tauri::AppHandle, data_dir: &std::path::Path) {
    match match_and_record_notifications(data_dir) {
        Ok(alerts) => {
            if alerts.is_empty() {
                return;
            }
            for alert in &alerts {
                let _ = app
                    .notification()
                    .builder()
                    .title("声望赏金")
                    .body(&alert.body)
                    .show();
            }
            let _ = app.emit("backend://bounty-alert", alerts);
        }
        Err(e) => eprintln!("[仓鼠] 赏金关注匹配失败: {e}"),
    }
}

fn emit_live_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://live-world-updated", message.to_string());
}

fn emit_resurgence_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://resurgence-updated", message.to_string());
}

fn emit_nightwave_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://nightwave-updated", message.to_string());
}

fn emit_update_finished(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://update-finished", message.to_string());
}

fn emit_warframes_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://warframes-updated", message.to_string());
}

fn emit_weapons_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://weapons-updated", message.to_string());
}

fn emit_mods_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://mods-updated", message.to_string());
}

fn emit_arcanes_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://arcanes-updated", message.to_string());
}

fn emit_relics_updated(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit("backend://relics-updated", message.to_string());
}

/// 图鉴 / 日常类 kick 共用：BusyGuard + 后台线程 + `backend://update-finished`。
/// `job` 返回 Ok(成功文案) / Err(失败文案)；成功前可自行 emit 域事件（warframes-updated 等）。
fn spawn_catalog_job<F>(
    app: tauri::AppHandle,
    busy: Arc<AtomicBool>,
    thread_name: &str,
    busy_message: &str,
    spawn_fail_label: &str,
    job: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<String, String> + Send + 'static,
{
    if !try_begin(&busy) {
        emit_update_finished(&app, busy_message);
        return Ok(());
    }
    let spawn_busy = Arc::clone(&busy);
    let fail_label = spawn_fail_label.to_string();
    std::thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            let _guard = BusyGuard(busy);
            match job() {
                Ok(msg) => emit_update_finished(&app, &msg),
                Err(e) => emit_update_finished(&app, &e),
            }
        })
        .map_err(|e| {
            spawn_busy.store(false, Ordering::Release);
            format!("{fail_label}: {e}")
        })?;
    Ok(())
}

/// 后台：尊重新鲜度的日常/赏金刷新；有更新才通知前端。
fn soft_refresh_live_bg(
    app: &tauri::AppHandle,
    data_dir: &Path,
    snowflake: &Snowflake,
    busy: &Arc<AtomicBool>,
) {
    if !try_begin(busy) {
        eprintln!("[仓鼠] 日常刷新进行中，跳过本轮");
        return;
    }
    let _guard = BusyGuard(Arc::clone(busy));
    match refresh_live_world_if_needed(data_dir, snowflake) {
        Ok(LiveWorldOutcome::SkippedFresh) => {
            eprintln!("[仓鼠] 日常动态缓存仍新鲜，跳过网络");
        }
        Ok(LiveWorldOutcome::KeptCache) => {
            eprintln!("[仓鼠] 日常动态拉取失败，已保留缓存");
        }
        Ok(LiveWorldOutcome::Updated) => {
            eprintln!("[仓鼠] 日常动态已更新（突击/电波/侵袭/裂隙）");
            push_bounty_notifications(app, data_dir);
            emit_live_updated(app, "日常/周常已更新");
        }
        Err(e) => eprintln!("[仓鼠] 日常动态静默更新失败: {e}"),
    }
}

fn soft_refresh_resurgence_bg(
    app: &tauri::AppHandle,
    data_dir: &Path,
    snowflake: &Snowflake,
    busy: &Arc<AtomicBool>,
) {
    if !try_begin(busy) {
        eprintln!("[仓鼠] 重生刷新进行中，跳过本轮");
        return;
    }
    let _guard = BusyGuard(Arc::clone(busy));
    match refresh_resurgence_if_needed(data_dir, snowflake) {
        Ok(ResurgenceOutcome::SkippedFresh) => {
            eprintln!("[仓鼠] 重生轮次未过期，跳过网络");
        }
        Ok(ResurgenceOutcome::NotModified) => {
            eprintln!("[仓鼠] 重生上游未变（304）");
        }
        Ok(ResurgenceOutcome::Updated {
            warframes,
            weapons,
            relics,
        }) => {
            let msg = format!("重生已更新: 战甲≈{warframes} 武器≈{weapons} 核桃≈{relics}");
            eprintln!("[仓鼠] {msg}");
            emit_resurgence_updated(app, &msg);
        }
        Err(e) => eprintln!("[仓鼠] 重生静默更新失败（保留旧数据）: {e}"),
    }
}

/// 后台强制更新战甲图鉴（不挡 UI）。

/// 后台强制更新武器图鉴（不挡 UI）。

/// 后台强制更新模组图鉴（不挡 UI）。

/// 后台强制更新赋能图鉴（不挡 UI）。

/// 后台强制更新遗物图鉴（不挡 UI）。

/// 后台强制刷新日常/周常/赏金（顶栏「更新数据」）。

/// 顺序占用两个 busy 跑任务，结束时统一 `backend://update-finished`。
fn spawn_dual_busy_job<F>(
    app: tauri::AppHandle,
    busy_a: Arc<AtomicBool>,
    busy_b: Arc<AtomicBool>,
    thread_name: &str,
    spawn_fail_label: &str,
    job: F,
) -> Result<(), String>
where
    F: FnOnce(&tauri::AppHandle, &Arc<AtomicBool>, &Arc<AtomicBool>) -> String + Send + 'static,
{
    std::thread::Builder::new()
        .name(thread_name.into())
        .spawn(move || {
            let msg = job(&app, &busy_a, &busy_b);
            emit_update_finished(&app, &msg);
        })
        .map_err(|e| format!("{spawn_fail_label}: {e}"))?;
    Ok(())
}

/// 后台：动态页强制刷新世界状态（含裂隙/电波）+ 重生。



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        // 不恢复 VISIBLE：配置里 visible=false 防闪白；显隐由首屏就绪后前端/这里负责
        .plugin(
            WindowStateBuilder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;

            if let Ok(resource_dir) = app.path().resource_dir() {
                db::set_bundled_resource_dir(resource_dir);
            }

            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("解析 app_data_dir 失败: {e}"))?;

            let status = db::init_databases(&data_dir)
                .map_err(|e| format!("初始化数据库失败: {e}"))?;
            eprintln!(
                "[仓鼠] 数据目录: {}\ncatalog v{} tables={:?}\nuser v{} tables={:?}",
                status.paths.data_dir,
                status.catalog_schema_version,
                status.catalog_tables,
                status.user_schema_version,
                status.user_tables
            );

            let snowflake = Arc::new(Snowflake::new(1));
            let live_busy = Arc::new(AtomicBool::new(false));
            let resurgence_busy = Arc::new(AtomicBool::new(false));
            let catalog_busy = Arc::new(AtomicBool::new(false));
            let data_dir_heal = data_dir.clone();
            let snowflake_heal = Arc::clone(&snowflake);
            let data_dir_tick = data_dir.clone();
            let snowflake_tick = Arc::clone(&snowflake);
            let live_busy_tick = Arc::clone(&live_busy);
            let surge_busy_tick = Arc::clone(&resurgence_busy);
            let handle_tick = app.handle().clone();

            app.manage(AppState {
                data_dir: data_dir.clone(),
                snowflake,
                live_busy,
                resurgence_busy,
                catalog_busy,
            });

            // 遗物来源自愈：推迟到首屏读库之后，避免启动瞬间抢写锁
            std::thread::Builder::new()
                .name("relic-sources-heal".into())
                .spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(8));
                    match heal_empty_relic_locations(&data_dir_heal, snowflake_heal.as_ref()) {
                        Ok(Some(s)) => {
                            eprintln!(
                                "[仓鼠] 启动自愈遗物来源: locations≈{} mission≈{} bounty≈{}",
                                s.locations, s.mission, s.bounty
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("[仓鼠] 启动自愈遗物来源失败: {e}");
                        }
                    }
                })
                .map_err(|e| format!("启动来源自愈线程失败: {e}"))?;

            // 时效数据后台循环：先让首屏读本地库，再拉网写库
            // 60s 检查一次（未过期则几乎零成本）；不必前端催更
            std::thread::Builder::new()
                .name("时效数据后台".into())
                .spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    loop {
                        soft_refresh_live_bg(
                            &handle_tick,
                            &data_dir_tick,
                            snowflake_tick.as_ref(),
                            &live_busy_tick,
                        );
                        soft_refresh_resurgence_bg(
                            &handle_tick,
                            &data_dir_tick,
                            snowflake_tick.as_ref(),
                            &surge_busy_tick,
                        );
                        std::thread::sleep(std::time::Duration::from_secs(60));
                    }
                })
                .map_err(|e| format!("启动时效数据后台线程失败: {e}"))?;

            Ok(())
        })
        .invoke_handler({
            use commands::*;
            tauri::generate_handler![
            get_db_status,
            next_id,
            list_warframes,
            get_warframe_detail,
            upsert_progress_warframe,
            upsert_progress_part,
            list_weapon_families,
            get_weapon_family_detail,
            upsert_weapon_progress,
            upsert_weapon_part_progress,
            list_mods,
            list_mod_tags,
            create_mod_tag,
            delete_mod_tag,
            upsert_mod_progress,
            list_arcanes,
            upsert_arcane_progress,
            list_user_todos,
            create_user_todo,
            update_user_todo,
            delete_user_todo,
            reorder_user_todos,
            list_routines,
            create_routine,
            update_routine,
            delete_routine,
            reorder_routines,
            get_resurgence,
            get_live_daily_brief_cmd,
            get_weekly_shop_brief_cmd,
            get_bounty_brief_cmd,
            get_fissure_brief_cmd,
            get_arbitration_brief_cmd,
            get_plains_brief_cmd,
            get_bounty_watch_cmd,
            set_bounty_watch_cmd,
            get_standing_ranks_cmd,
            set_standing_ranks_cmd,
            get_standing_daily_cmd,
            set_standing_daily_cmd,
            kick_update_live_world,
            kick_update_feed,
            kick_update_warframes,
            kick_update_weapons,
            kick_update_mods,
            kick_update_arcanes,
            kick_update_relics,
            search_relic_sources_cmd,
            get_relic_source_filters
            ]
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
