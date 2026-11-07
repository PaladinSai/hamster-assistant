use std::path::Path;

use chrono::{DateTime, Duration, Utc};

use crate::catalog::arcanes::{backfill_arcane_name_zh, import_arcanes_json};
use crate::catalog::drop_locations::{
    fetch_drop_all, refresh_drop_locations_conn, DEFAULT_DROP_ALL_URL,
};
use crate::catalog::import::{
    count_families, import_archwing_json, import_modular_companions_json, import_pets_json,
    import_sentinels_json, import_warframes_json, ImportStats,
};
use crate::catalog::mods::import_mods_json;
use crate::catalog::nicknames::apply_bundled_warframe_nicknames;
use crate::catalog::relics::{
    fetch_relics, import_relics_json, RelicImportStats, DEFAULT_RELICS_URL,
};
use crate::catalog::upstream_cache;
use crate::catalog::weapons::{
    import_archwing_weapons_json, import_modular_weapons_json, import_weapons_json,
};
use crate::catalog::wfcd::{
    fetch_archwings, fetch_json_url, fetch_pets, fetch_sentinels, fetch_warframes, FetchResult,
    DEFAULT_ARCANES_URL, DEFAULT_ARCHWING_URL, DEFAULT_ARCH_GUN_URL, DEFAULT_ARCH_MELEE_URL,
    DEFAULT_MELEE_URL, DEFAULT_MISC_URL, DEFAULT_MODS_URL, DEFAULT_PETS_URL, DEFAULT_PRIMARY_URL,
    DEFAULT_SECONDARY_URL, DEFAULT_SENTINELS_URL, DEFAULT_WARFRAMES_URL,
};
use crate::db::{self, open_catalog, open_user};
use crate::id::Snowflake;

const SETTING_ETAG: &str = "catalog_warframes_etag";
const SETTING_SYNCED_AT: &str = "catalog_warframes_synced_at";
const SETTING_CHECKED_AT: &str = "catalog_warframes_checked_at";
const SETTING_SOURCE_URL: &str = "catalog_warframes_source_url";
const SETTING_RELICS_ETAG: &str = "catalog_relics_etag";
const SETTING_PRIMARY_ETAG: &str = "catalog_primary_etag";
const SETTING_SECONDARY_ETAG: &str = "catalog_secondary_etag";
const SETTING_MELEE_ETAG: &str = "catalog_melee_etag";
const SETTING_ARCH_GUN_ETAG: &str = "catalog_arch_gun_etag";
const SETTING_ARCH_MELEE_ETAG: &str = "catalog_arch_melee_etag";
const SETTING_MISC_ETAG: &str = "catalog_misc_etag";
/// 旧库部件补过配方路径之后置 1，避免每次 304 都重导。
const SETTING_WEAPON_PART_UID: &str = "catalog_weapon_part_uid_backfill";
const SETTING_SUIT_PART_UID: &str = "catalog_suit_part_uid_backfill";
const SETTING_MODS_ETAG: &str = "catalog_mods_etag";
const SETTING_ARCANES_ETAG: &str = "catalog_arcanes_etag";
const SETTING_DROP_ETAG: &str = "catalog_drop_locations_etag";

/// 满此间隔才再次访问网络做目录检查（空库除外）。
const CHECK_INTERVAL: Duration = Duration::days(30);

#[derive(Debug, Clone)]
pub enum RefreshOutcome {
    /// 距上次检查未满月，未访问网络
    SkippedFresh,
    /// 已检查上游，ETag 未变
    NotModified,
    Updated {
        stats: ImportStats,
        relic_stats: RelicImportStats,
        etag: Option<String>,
    },
}

/// 静默刷新：空库强制全量；否则距上次检查未满 30 天则跳过网络；
/// 满月后带 ETag 检查，未变则只更新 checked_at。
pub fn refresh_catalog_if_needed(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<RefreshOutcome, String> {
    db::init_databases(data_dir)?;

    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);

    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;

    let family_count = count_families(&catalog)?;
    let force = family_count == 0;

    if !force && !due_for_check(&catalog)? {
        let relic_n: i64 = catalog
            .query_row("SELECT COUNT(*) FROM void_relic", [], |r| r.get(0))
            .unwrap_or(0);
        if relic_n > 0 {
            let s = refresh_drop_locations_conn(data_dir, &mut catalog, &user, snowflake, false)?;
            if s.locations > 0 {
                eprintln!(
                    "[仓鼠] 遗物来源补全: locations≈{} mission≈{} bounty≈{}",
                    s.locations, s.mission, s.bounty
                );
            }
        }
        return Ok(RefreshOutcome::SkippedFresh);
    }

    let prev_etag = if force {
        None
    } else {
        db::get_setting(&catalog, SETTING_ETAG)?
    };

    let fetched = fetch_warframes(DEFAULT_WARFRAMES_URL, prev_etag.as_deref())?;
    let now = Utc::now().to_rfc3339();
    db::set_setting(&catalog, SETTING_CHECKED_AT, &now)?;

    if fetched.not_modified {
        // 战甲未变时仍刷 Archwing / 守护 + 遗物。任一拉取失败则停掉后面的步骤。
        refresh_archwing(data_dir, &mut catalog, snowflake)?;
        refresh_companions(data_dir, &mut catalog, snowflake)?;
        let _ = refresh_relics(data_dir, &mut catalog, &user, snowflake, false, true)?;
        return Ok(RefreshOutcome::NotModified);
    }

    upstream_cache::save(data_dir, upstream_cache::WARFRAMES, &fetched.body)?;
    let stats = import_warframes_json(&mut catalog, &fetched.body, snowflake)?;
    refresh_archwing(data_dir, &mut catalog, snowflake)?;
    refresh_companions(data_dir, &mut catalog, snowflake)?;
    let relic_stats = refresh_relics(data_dir, &mut catalog, &user, snowflake, true, true)?;

    if let Some(ref etag) = fetched.etag {
        db::set_setting(&catalog, SETTING_ETAG, etag)?;
    }
    db::set_setting(&catalog, SETTING_SYNCED_AT, &now)?;
    db::set_setting(&catalog, SETTING_SOURCE_URL, &fetched.url)?;

    Ok(RefreshOutcome::Updated {
        stats,
        relic_stats,
        etag: fetched.etag,
    })
}

fn refresh_archwing(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    snowflake: &Snowflake,
) -> Result<(), String> {
    let fetched = fetch_archwings(DEFAULT_ARCHWING_URL, None)?;
    if fetched.not_modified {
        return Ok(());
    }
    upstream_cache::save(data_dir, upstream_cache::ARCHWING, &fetched.body)?;
    let s = import_archwing_json(catalog, &fetched.body, snowflake)?;
    eprintln!(
        "[仓鼠] Archwing 导入: 家族+{} 变体+{} 部件+{}",
        s.families, s.warframes, s.parts
    );
    Ok(())
}

fn refresh_sentinels(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    snowflake: &Snowflake,
) -> Result<(), String> {
    let fetched = fetch_sentinels(DEFAULT_SENTINELS_URL, None)?;
    if fetched.not_modified {
        return Ok(());
    }
    upstream_cache::save(data_dir, upstream_cache::SENTINELS, &fetched.body)?;
    let s = import_sentinels_json(catalog, &fetched.body, snowflake)?;
    eprintln!(
        "[仓鼠] 哨兵导入: 家族+{} 变体+{} 部件+{}",
        s.families, s.warframes, s.parts
    );
    Ok(())
}

fn refresh_pets(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    snowflake: &Snowflake,
) -> Result<(), String> {
    let fetched = fetch_pets(DEFAULT_PETS_URL, None)?;
    let body = if fetched.not_modified {
        match upstream_cache::read(data_dir, upstream_cache::PETS)? {
            Some(cached) => cached,
            None => return Ok(()),
        }
    } else {
        upstream_cache::save(data_dir, upstream_cache::PETS, &fetched.body)?;
        let s = import_pets_json(catalog, &fetched.body, snowflake)?;
        eprintln!(
            "[仓鼠] 宠物同伴导入: 家族+{} 变体+{} 部件+{}",
            s.families, s.warframes, s.parts
        );
        fetched.body
    };
    let s = import_modular_companions_json(catalog, &body, snowflake)?;
    eprintln!(
        "[仓鼠] 自制同伴导入: 家族+{} 型号+{}",
        s.families, s.warframes
    );
    Ok(())
}

fn refresh_companions(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    snowflake: &Snowflake,
) -> Result<(), String> {
    refresh_sentinels(data_dir, catalog, snowflake)?;
    refresh_pets(data_dir, catalog, snowflake)?;
    Ok(())
}

/// 维护者脚本 / 测试：忽略月限与 ETag，强制拉取并导入。
pub fn force_refresh_catalog(
    data_dir: &Path,
    snowflake: &Snowflake,
) -> Result<ImportStats, String> {
    db::init_databases(data_dir)?;
    let catalog_path = db::catalog_path(data_dir);
    let user_path = db::user_path(data_dir);
    let mut catalog = open_catalog(&catalog_path)?;
    let user = open_user(&user_path)?;

    let fetched = fetch_warframes(DEFAULT_WARFRAMES_URL, None)?;
    if fetched.not_modified {
        return Err("上游返回未修改，但强制刷新未带 ETag".into());
    }
    upstream_cache::save(data_dir, upstream_cache::WARFRAMES, &fetched.body)?;
    let stats = import_warframes_json(&mut catalog, &fetched.body, snowflake)?;
    refresh_archwing(data_dir, &mut catalog, snowflake)?;
    refresh_companions(data_dir, &mut catalog, snowflake)?;
    let nick_n = apply_bundled_warframe_nicknames(&catalog)?;
    if nick_n > 0 {
        eprintln!("[仓鼠] 俗称写回 ≈{nick_n}");
    }
    let relic_stats = refresh_relics(data_dir, &mut catalog, &user, snowflake, true, true)?;
    eprintln!(
        "[仓鼠] 遗物导入: relics≈{} drops≈{}",
        relic_stats.relics, relic_stats.drops
    );
    let now = Utc::now().to_rfc3339();
    if let Some(ref etag) = fetched.etag {
        db::set_setting(&catalog, SETTING_ETAG, etag)?;
    }
    db::set_setting(&catalog, SETTING_SYNCED_AT, &now)?;
    db::set_setting(&catalog, SETTING_CHECKED_AT, &now)?;
    db::set_setting(&catalog, SETTING_SOURCE_URL, &fetched.url)?;
    Ok(stats)
}

/// 手动：战甲 + 遗物（部件链接，不强制刷来源）。
pub fn update_warframes_catalog(data_dir: &Path, snowflake: &Snowflake) -> Result<String, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let user = open_user(&db::user_path(data_dir))?;
    let prev = db::get_setting(&catalog, SETTING_ETAG)?;
    let fetched = fetch_warframes(DEFAULT_WARFRAMES_URL, prev.as_deref())?;
    if fetched.not_modified {
        refresh_archwing(data_dir, &mut catalog, snowflake)?;
        refresh_companions(data_dir, &mut catalog, snowflake)?;
        let nick_n = apply_bundled_warframe_nicknames(&catalog)?;
        return Ok(if nick_n > 0 {
            format!("战甲已是最新；俗称≈{nick_n}")
        } else {
            format!("战甲已是最新")
        });
    }
    upstream_cache::save(data_dir, upstream_cache::WARFRAMES, &fetched.body)?;
    let stats = import_warframes_json(&mut catalog, &fetched.body, snowflake)?;
    refresh_archwing(data_dir, &mut catalog, snowflake)?;
    refresh_companions(data_dir, &mut catalog, snowflake)?;
    let nick_n = apply_bundled_warframe_nicknames(&catalog)?;
    let relic_stats = refresh_relics(data_dir, &mut catalog, &user, snowflake, false, false)?;
    let now = Utc::now().to_rfc3339();
    if let Some(ref etag) = fetched.etag {
        db::set_setting(&catalog, SETTING_ETAG, etag)?;
    }
    db::set_setting(&catalog, SETTING_SYNCED_AT, &now)?;
    db::set_setting(&catalog, SETTING_CHECKED_AT, &now)?;
    db::set_setting(&catalog, SETTING_SOURCE_URL, &fetched.url)?;
    Ok(if nick_n > 0 {
        format!(
            "战甲家族+{} 变体+{} 部件+{}；遗物≈{}；俗称≈{}",
            stats.families, stats.warframes, stats.parts, relic_stats.relics, nick_n
        )
    } else {
        format!(
            "战甲家族+{} 变体+{} 部件+{}；遗物≈{}",
            stats.families, stats.warframes, stats.parts, relic_stats.relics
        )
    })
}

/// 手动：武器三槽 + 空战武器 + 遗物（部件链接）。
pub fn update_weapons_catalog(data_dir: &Path, snowflake: &Snowflake) -> Result<String, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let user = open_user(&db::user_path(data_dir))?;

    let primary = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_PRIMARY_URL,
        SETTING_PRIMARY_ETAG,
        upstream_cache::PRIMARY,
        90,
    )?;
    let secondary = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_SECONDARY_URL,
        SETTING_SECONDARY_ETAG,
        upstream_cache::SECONDARY,
        90,
    )?;
    let melee = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_MELEE_URL,
        SETTING_MELEE_ETAG,
        upstream_cache::MELEE,
        90,
    )?;
    let arch_gun = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_ARCH_GUN_URL,
        SETTING_ARCH_GUN_ETAG,
        upstream_cache::ARCH_GUN,
        90,
    )?;
    let arch_melee = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_ARCH_MELEE_URL,
        SETTING_ARCH_MELEE_ETAG,
        upstream_cache::ARCH_MELEE,
        90,
    )?;
    let misc = fetch_slot_with_cache(
        data_dir,
        &catalog,
        DEFAULT_MISC_URL,
        SETTING_MISC_ETAG,
        upstream_cache::MISC,
        90,
    )?;

    let need_ground = !(primary.all_cached && secondary.all_cached && melee.all_cached);
    let need_arch = !(arch_gun.all_cached && arch_melee.all_cached);
    let need_modular = !(misc.all_cached && melee.all_cached);
    let backfill = needs_weapon_part_uid_backfill(&catalog)?;

    if !need_ground && !need_arch && !need_modular && !backfill {
        return Ok("武器已是最新".into());
    }

    let mut fam = 0usize;
    let mut wep = 0usize;
    let mut parts = 0usize;
    if need_ground || backfill {
        let stats = import_weapons_json(
            &mut catalog,
            &primary.body,
            &secondary.body,
            &melee.body,
            snowflake,
        )?;
        fam += stats.families;
        wep += stats.weapons;
        parts += stats.parts;
    }
    if need_arch || backfill {
        let stats = import_archwing_weapons_json(
            &mut catalog,
            &arch_gun.body,
            &arch_melee.body,
            snowflake,
        )?;
        fam += stats.families;
        wep += stats.weapons;
        parts += stats.parts;
    }
    if need_modular || backfill {
        let stats =
            import_modular_weapons_json(&mut catalog, &misc.body, &melee.body, snowflake)?;
        fam += stats.families;
        wep += stats.weapons;
    }
    if backfill {
        relink_weapon_parts_after_import(data_dir, &mut catalog, snowflake)?;
        db::set_setting(&catalog, SETTING_WEAPON_PART_UID, "1")?;
    }
    let relic_stats = refresh_relics(data_dir, &mut catalog, &user, snowflake, false, false)?;
    Ok(format!(
        "武器家族+{} 枪+{} 部件+{}；遗物≈{}",
        fam, wep, parts, relic_stats.relics
    ))
}

/// 手动：Mods + drop-data 来源归桶。
pub fn update_mods_catalog(data_dir: &Path, snowflake: &Snowflake) -> Result<String, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let _user = open_user(&db::user_path(data_dir))?;
    let mods_prev = db::get_setting(&catalog, SETTING_MODS_ETAG)?;
    let drops_prev = db::get_setting(&catalog, SETTING_DROP_ETAG)?;
    let mods = fetch_json_url(DEFAULT_MODS_URL, mods_prev.as_deref(), 120)?;
    let drops = fetch_drop_all(DEFAULT_DROP_ALL_URL, drops_prev.as_deref())?;

    if mods.not_modified && drops.not_modified {
        return Ok("Mod 已是最新".into());
    }

    let mods_body = resolve_body(data_dir, &mods, DEFAULT_MODS_URL, upstream_cache::MODS, 120)?;
    let drops_body =
        resolve_drop_body(data_dir, &drops, DEFAULT_DROP_ALL_URL, upstream_cache::DROP_ALL)?;

    if !mods.not_modified {
        upstream_cache::save(data_dir, upstream_cache::MODS, &mods_body)?;
        if let Some(ref etag) = mods.etag {
            db::set_setting(&catalog, SETTING_MODS_ETAG, etag)?;
        }
    }
    if !drops.not_modified {
        upstream_cache::save(data_dir, upstream_cache::DROP_ALL, &drops_body)?;
        if let Some(ref etag) = drops.etag {
            db::set_setting(&catalog, SETTING_DROP_ETAG, etag)?;
        } else {
            let mark = format!("len:{}", drops_body.len());
            db::set_setting(&catalog, SETTING_DROP_ETAG, &mark)?;
        }
    }

    let stats = import_mods_json(&mut catalog, &mods_body, &drops_body, snowflake)?;
    Ok(format!(
        "Mod≈{}（跳过≈{}）",
        stats.mods, stats.skipped
    ))
}

/// 手动：赋能 Arcanes.json。
pub fn update_arcanes_catalog(data_dir: &Path, snowflake: &Snowflake) -> Result<String, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let _user = open_user(&db::user_path(data_dir))?;

    let prev = db::get_setting(&catalog, SETTING_ARCANES_ETAG)?;
    let fetched = fetch_json_url(DEFAULT_ARCANES_URL, prev.as_deref(), 90)?;
    if fetched.not_modified {
        let filled = backfill_arcane_name_zh(&catalog)?;
        return Ok(if filled > 0 {
            format!("赋能已是最新；回填中文≈{filled}")
        } else {
            format!("赋能已是最新")
        });
    }
    upstream_cache::save(data_dir, upstream_cache::ARCANES, &fetched.body)?;
    if let Some(ref etag) = fetched.etag {
        db::set_setting(&catalog, SETTING_ARCANES_ETAG, etag)?;
    }
    let stats = import_arcanes_json(&mut catalog, &fetched.body, snowflake)?;
    let filled = backfill_arcane_name_zh(&catalog)?;
    Ok(if filled > 0 {
        format!(
            "赋能≈{}（跳过≈{}）；回填中文≈{}",
            stats.arcanes, stats.skipped, filled
        )
    } else {
        format!("赋能≈{}（跳过≈{}）", stats.arcanes, stats.skipped)
    })
}

/// 手动：遗物 + 来源表。
pub fn update_relics_catalog(data_dir: &Path, snowflake: &Snowflake) -> Result<String, String> {
    db::init_databases(data_dir)?;
    let mut catalog = open_catalog(&db::catalog_path(data_dir))?;
    let user = open_user(&db::user_path(data_dir))?;
    let prev = db::get_setting(&catalog, SETTING_RELICS_ETAG)?;
    let fetched = fetch_relics(DEFAULT_RELICS_URL, prev.as_deref())?;
    if fetched.not_modified {
        let s = refresh_drop_locations_conn(data_dir, &mut catalog, &user, snowflake, false)?;
        if s.locations > 0 {
            return Ok(format!(
                "遗物已是最新；来源≈{} mission≈{} bounty≈{}",
                s.locations, s.mission, s.bounty
            ));
        }
        return Ok("遗物已是最新".into());
    }
    upstream_cache::save(data_dir, upstream_cache::RELICS, &fetched.body)?;
    let relic_stats = import_relics_json(&mut catalog, &fetched.body, snowflake)?;
    if let Some(ref etag) = fetched.etag {
        db::set_setting(&catalog, SETTING_RELICS_ETAG, etag)?;
    }
    let drop_stats =
        refresh_drop_locations_conn(data_dir, &mut catalog, &user, snowflake, true).map_err(
            |e| {
                format!(
                    "遗物≈{} 已导入，来源更新失败，已保留原来源: {e}",
                    relic_stats.relics
                )
            },
        )?;
    eprintln!(
        "[仓鼠] 遗物来源已更新: locations≈{} mission≈{} bounty≈{}",
        drop_stats.locations, drop_stats.mission, drop_stats.bounty
    );
    Ok(format!(
        "遗物≈{} 奖励≈{} 战甲掉落≈{} 武器掉落≈{}；来源≈{} mission≈{} bounty≈{}",
        relic_stats.relics,
        relic_stats.rewards,
        relic_stats.drops,
        relic_stats.weapon_drops,
        drop_stats.locations,
        drop_stats.mission,
        drop_stats.bounty
    ))
}

/// `with_drops`：是否在导入遗物后强制/补全 `relic_location`。
fn refresh_relics(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    user: &rusqlite::Connection,
    snowflake: &Snowflake,
    force: bool,
    with_drops: bool,
) -> Result<RelicImportStats, String> {
    let prev = if force {
        None
    } else {
        db::get_setting(catalog, SETTING_RELICS_ETAG)?
    };
    let fetched = fetch_relics(DEFAULT_RELICS_URL, prev.as_deref())?;
    if fetched.not_modified {
        if with_drops {
            let s = refresh_drop_locations_conn(data_dir, catalog, user, snowflake, false)?;
            if s.locations > 0 {
                eprintln!(
                    "[仓鼠] 遗物来源补全: locations≈{} mission≈{} bounty≈{}",
                    s.locations, s.mission, s.bounty
                );
            }
        }
        return Ok(RelicImportStats::default());
    }
    upstream_cache::save(data_dir, upstream_cache::RELICS, &fetched.body)?;
    let stats = import_relics_json(catalog, &fetched.body, snowflake)?;
    if let Some(ref etag) = fetched.etag {
        db::set_setting(catalog, SETTING_RELICS_ETAG, etag)?;
    }
    if with_drops {
        let drop_stats = refresh_drop_locations_conn(data_dir, catalog, user, snowflake, true)
            .map_err(|e| format!("遗物已导入，来源更新失败，已保留原来源: {e}"))?;
        eprintln!(
            "[仓鼠] 遗物来源已更新: locations≈{} mission≈{} bounty≈{}",
            drop_stats.locations, drop_stats.mission, drop_stats.bounty
        );
    }
    Ok(stats)
}

struct SlotFetch {
    body: Vec<u8>,
    /// 304 且直接用了本地缓存（或本槽未变更）
    all_cached: bool,
}

/// 条件请求；200 落盘并更新 ETag；304 则读本地缓存，缺失则无条件重拉。
fn fetch_slot_with_cache(
    data_dir: &Path,
    catalog: &rusqlite::Connection,
    url: &str,
    etag_key: &str,
    filename: &str,
    timeout_secs: u64,
) -> Result<SlotFetch, String> {
    let prev = db::get_setting(catalog, etag_key)?;
    let fetched = fetch_json_url(url, prev.as_deref(), timeout_secs)?;
    if fetched.not_modified {
        if let Some(cached) = upstream_cache::read(data_dir, filename)? {
            return Ok(SlotFetch {
                body: cached,
                all_cached: true,
            });
        }
        let fresh = fetch_json_url(url, None, timeout_secs)?;
        if fresh.not_modified {
            return Err(format!("上游 304 且本地无 {filename}"));
        }
        upstream_cache::save(data_dir, filename, &fresh.body)?;
        if let Some(ref etag) = fresh.etag {
            db::set_setting(catalog, etag_key, etag)?;
        }
        return Ok(SlotFetch {
            body: fresh.body,
            all_cached: false,
        });
    }
    upstream_cache::save(data_dir, filename, &fetched.body)?;
    if let Some(ref etag) = fetched.etag {
        db::set_setting(catalog, etag_key, etag)?;
    }
    Ok(SlotFetch {
        body: fetched.body,
        all_cached: false,
    })
}

fn resolve_body(
    data_dir: &Path,
    fetched: &FetchResult,
    url: &str,
    filename: &str,
    timeout_secs: u64,
) -> Result<Vec<u8>, String> {
    if !fetched.not_modified {
        return Ok(fetched.body.clone());
    }
    if let Some(cached) = upstream_cache::read(data_dir, filename)? {
        return Ok(cached);
    }
    let fresh = fetch_json_url(url, None, timeout_secs)?;
    if fresh.not_modified {
        return Err(format!("上游 304 且本地无 {filename}"));
    }
    upstream_cache::save(data_dir, filename, &fresh.body)?;
    Ok(fresh.body)
}

fn resolve_drop_body(
    data_dir: &Path,
    fetched: &FetchResult,
    url: &str,
    filename: &str,
) -> Result<Vec<u8>, String> {
    if !fetched.not_modified {
        return Ok(fetched.body.clone());
    }
    if let Some(cached) = upstream_cache::read(data_dir, filename)? {
        return Ok(cached);
    }
    let fresh = fetch_drop_all(url, None)?;
    if fresh.not_modified {
        return Err(format!("上游 304 且本地无 {filename}"));
    }
    upstream_cache::save(data_dir, filename, &fresh.body)?;
    Ok(fresh.body)
}

fn prime_weapon_parts_missing_unique(catalog: &rusqlite::Connection) -> Result<bool, String> {
    let n: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM weapon_part p
             JOIN weapon w ON w.id = p.weapon_id
             WHERE w.variant = ?1
               AND (p.unique_name IS NULL OR TRIM(p.unique_name) = '')",
            [super::weapons::VARIANT_PRIME],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn needs_weapon_part_uid_backfill(catalog: &rusqlite::Connection) -> Result<bool, String> {
    if db::get_setting(catalog, SETTING_WEAPON_PART_UID)?.as_deref() == Some("1") {
        return Ok(false);
    }
    prime_weapon_parts_missing_unique(catalog)
}

/// 奖励已在库里就按路径重挂；没有奖励时才用本地 Relics.json，避免清掉来源表。
fn relink_weapon_parts_after_import(
    data_dir: &Path,
    catalog: &mut rusqlite::Connection,
    snowflake: &Snowflake,
) -> Result<(), String> {
    let rewards: i64 = catalog
        .query_row("SELECT COUNT(*) FROM relic_reward", [], |r| r.get(0))
        .unwrap_or(0);
    if rewards > 0 {
        let n = super::relics::relink_weapon_parts_by_unique(catalog, snowflake)?;
        eprintln!("[仓鼠] 武器部件按路径重链 ≈{n}");
        return Ok(());
    }
    if let Some(body) = upstream_cache::read(data_dir, upstream_cache::RELICS)? {
        let stats = import_relics_json(catalog, &body, snowflake)?;
        eprintln!(
            "[仓鼠] 用本地遗物缓存重链: relics≈{} 武器掉落≈{}",
            stats.relics, stats.weapon_drops
        );
    }
    Ok(())
}

/// 启动自愈：Prime 部件还没有配方路径时，只用本地缓存重导并重链，不发请求。
pub fn heal_weapon_part_uniques(data_dir: &Path, snowflake: &Snowflake) -> Result<bool, String> {
    let catalog_path = db::catalog_path(data_dir);
    if !catalog_path.is_file() {
        return Ok(false);
    }
    let mut catalog = open_catalog(&catalog_path)?;
    if !needs_weapon_part_uid_backfill(&catalog)? {
        return Ok(false);
    }
    let Some(primary) = upstream_cache::read(data_dir, upstream_cache::PRIMARY)? else {
        return Ok(false);
    };
    let Some(secondary) = upstream_cache::read(data_dir, upstream_cache::SECONDARY)? else {
        return Ok(false);
    };
    let Some(melee) = upstream_cache::read(data_dir, upstream_cache::MELEE)? else {
        return Ok(false);
    };
    let stats = import_weapons_json(&mut catalog, &primary, &secondary, &melee, snowflake)?;
    eprintln!(
        "[仓鼠] 武器部件路径回填: 家族+{} 枪+{} 部件+{}",
        stats.families, stats.weapons, stats.parts
    );
    if let (Some(arch_gun), Some(arch_melee)) = (
        upstream_cache::read(data_dir, upstream_cache::ARCH_GUN)?,
        upstream_cache::read(data_dir, upstream_cache::ARCH_MELEE)?,
    ) {
        let arch =
            import_archwing_weapons_json(&mut catalog, &arch_gun, &arch_melee, snowflake)?;
        eprintln!(
            "[仓鼠] 空战武器部件路径回填: 家族+{} 枪+{} 部件+{}",
            arch.families, arch.weapons, arch.parts
        );
    }
    relink_weapon_parts_after_import(data_dir, &mut catalog, snowflake)?;
    db::set_setting(&catalog, SETTING_WEAPON_PART_UID, "1")?;
    Ok(true)
}

fn suit_parts_missing_unique(catalog: &rusqlite::Connection) -> Result<bool, String> {
    let n: i64 = catalog
        .query_row(
            "SELECT COUNT(*) FROM warframe_part
             WHERE unique_name IS NULL OR TRIM(unique_name) = ''",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

/// 启动自愈：机体部件还没有配方路径时，用本地缓存重导并按路径挂遗物，不发请求。
pub fn heal_suit_part_uniques(data_dir: &Path, snowflake: &Snowflake) -> Result<bool, String> {
    let catalog_path = db::catalog_path(data_dir);
    if !catalog_path.is_file() {
        return Ok(false);
    }
    let mut catalog = open_catalog(&catalog_path)?;
    if db::get_setting(&catalog, SETTING_SUIT_PART_UID)?.as_deref() == Some("1") {
        return Ok(false);
    }
    if !suit_parts_missing_unique(&catalog)? {
        return Ok(false);
    }
    let Some(warframes) = upstream_cache::read(data_dir, upstream_cache::WARFRAMES)? else {
        return Ok(false);
    };
    let Some(archwing) = upstream_cache::read(data_dir, upstream_cache::ARCHWING)? else {
        return Ok(false);
    };
    let Some(sentinels) = upstream_cache::read(data_dir, upstream_cache::SENTINELS)? else {
        return Ok(false);
    };
    let Some(pets) = upstream_cache::read(data_dir, upstream_cache::PETS)? else {
        return Ok(false);
    };
    import_warframes_json(&mut catalog, &warframes, snowflake)?;
    import_archwing_json(&mut catalog, &archwing, snowflake)?;
    import_sentinels_json(&mut catalog, &sentinels, snowflake)?;
    import_pets_json(&mut catalog, &pets, snowflake)?;
    let n = super::relics::link_suit_relics_from_rewards(&mut catalog, snowflake)?;
    eprintln!("[仓鼠] 机体部件按路径重链 ≈{n}");
    db::set_setting(&catalog, SETTING_SUIT_PART_UID, "1")?;
    Ok(true)
}

fn due_for_check(catalog: &rusqlite::Connection) -> Result<bool, String> {
    let raw = db::get_setting(catalog, SETTING_CHECKED_AT)?
        .or(db::get_setting(catalog, SETTING_SYNCED_AT)?);
    let Some(raw) = raw else {
        return Ok(true);
    };
    let Ok(ts) = DateTime::parse_from_rfc3339(&raw) else {
        return Ok(true);
    };
    let age = Utc::now().signed_duration_since(ts.with_timezone(&Utc));
    Ok(age >= CHECK_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::Snowflake;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_refresh_{tag}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn skips_network_when_checked_recently() {
        let dir = temp_dir("skip");
        db::init_databases_ex(&dir, false).unwrap();
        {
            let catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
            catalog
                .execute(
                    "INSERT INTO warframe_family (id, family_key, name_en)
                     VALUES (1, 'ash', 'Ash')",
                    [],
                )
                .unwrap();
            let _user = open_user(&db::user_path(&dir)).unwrap();
            db::set_setting(&catalog, SETTING_CHECKED_AT, &Utc::now().to_rfc3339()).unwrap();
        }

        let snow = Snowflake::new(1);
        // 未满月：不应访问网络
        let outcome = refresh_catalog_if_needed(&dir, &snow).unwrap();
        assert!(matches!(outcome, RefreshOutcome::SkippedFresh));
        let _ = fs::remove_dir_all(&dir);
    }
}
