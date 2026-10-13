//! 维护者：用 resources 内上游 JSON 重建 catalog.sqlite + 空 user.sqlite。
//!
//! ```text
//! cargo run --bin seed_catalog --
//! cargo run --bin seed_catalog -- path/to/out_dir
//! ```
//!
//! 默认读 `CARGO_MANIFEST_DIR/resources/` 下的 Warframes/武器/Mods/Arcanes/Relics/
//! drop-all、userdict、i18n-zh、俗称等；不联网。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rusqlite::Connection;
use tauri_app_lib::catalog::{
    apply_display_zh_to_entities, apply_nicknames_map, import_arcanes_json,
    import_drop_locations_json, import_mods_json, import_relics_json, import_warframes_json,
    import_weapons_json, parse_i18n_from_file, read_nicknames_json, upsert_name_i18n_userdict,
    upsert_wfcd_i18n_by_entities,
};
use tauri_app_lib::db::{self, open_catalog};
use tauri_app_lib::id::Snowflake;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_out = manifest_dir.join("resources");
    let out_dir = args.next().map(PathBuf::from).unwrap_or(default_out);
    let res_dir = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest_dir.join("resources"));

    eprintln!("[seed_catalog] 输出: {}", out_dir.display());
    eprintln!("[seed_catalog] 资源: {}", res_dir.display());
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("[seed_catalog] 创建目录失败: {e}");
        return ExitCode::FAILURE;
    }

    let snowflake = Snowflake::new(1);
    match run_seed(&out_dir, &res_dir, &snowflake) {
        Ok(()) => {
            eprintln!(
                "[seed_catalog] 完成 → {}\n                 {}",
                out_dir.join("catalog.sqlite").display(),
                out_dir.join("user.sqlite").display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[seed_catalog] 失败: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run_seed(out_dir: &Path, res_dir: &Path, snowflake: &Snowflake) -> Result<(), String> {
    let catalog_path = db::catalog_path(out_dir);
    let user_path = db::user_path(out_dir);

    for path in [&catalog_path, &user_path] {
        for ext in ["", "-wal", "-shm"] {
            let p = PathBuf::from(format!("{}{ext}", path.display()));
            if p.is_file() {
                let _ = fs::remove_file(&p);
            }
        }
    }

    // 空建表，不拷贝旧种子
    db::init_databases_ex(out_dir, false)?;

    let userdict = res_dir.join("userdict.json");
    let i18n_zh = res_dir.join("i18n-zh.json");
    let warframes = read_req(res_dir, "Warframes.json")?;
    let primary = read_req(res_dir, "Primary.json")?;
    let secondary = read_req(res_dir, "Secondary.json")?;
    let melee = read_req(res_dir, "Melee.json")?;
    let mods = read_req(res_dir, "Mods.json")?;
    let drop_all = read_req(res_dir, "drop-all.json")?;
    let arcanes = read_req(res_dir, "Arcanes.json")?;
    let relics = read_req(res_dir, "Relics.json")?;

    {
        let mut catalog = open_catalog(&catalog_path)?;
        if userdict.is_file() {
            let rows = parse_i18n_from_file(&userdict)?;
            let tx = catalog
                .transaction()
                .map_err(|e| format!("userdict 事务失败: {e}"))?;
            let n = upsert_name_i18n_userdict(&tx, &rows)?;
            tx.commit().map_err(|e| e.to_string())?;
            eprintln!("[seed_catalog] name_zh_userdict ≈{n}");
        } else {
            eprintln!("[seed_catalog] 警告: 无 userdict.json");
        }
    }

    {
        let mut catalog = open_catalog(&catalog_path)?;
        let wf = import_warframes_json(&mut catalog, &warframes, snowflake)?;
        eprintln!(
            "[seed_catalog] 战甲 家族≈{} 变体≈{} 部件≈{}",
            wf.families, wf.warframes, wf.parts
        );
        let wep = import_weapons_json(&mut catalog, &primary, &secondary, &melee, snowflake)?;
        eprintln!(
            "[seed_catalog] 武器 家族≈{} 枪≈{} 部件≈{}",
            wep.families, wep.weapons, wep.parts
        );
        let md = import_mods_json(&mut catalog, &mods, &drop_all, snowflake)?;
        eprintln!("[seed_catalog] Mod≈{} 跳过≈{}", md.mods, md.skipped);
        let arc = import_arcanes_json(&mut catalog, &arcanes, snowflake)?;
        eprintln!("[seed_catalog] 赋能≈{} 跳过≈{}", arc.arcanes, arc.skipped);
        let rel = import_relics_json(&mut catalog, &relics, snowflake)?;
        eprintln!("[seed_catalog] 遗物≈{} 掉落≈{}", rel.relics, rel.drops);
        let drops = import_drop_locations_json(&mut catalog, &drop_all, snowflake)?;
        eprintln!(
            "[seed_catalog] 来源 locations≈{} mission≈{} bounty≈{}",
            drops.locations, drops.mission, drops.bounty
        );

        if i18n_zh.is_file() {
            let body = fs::read(&i18n_zh).map_err(|e| e.to_string())?;
            let n = upsert_wfcd_i18n_by_entities(&mut catalog, &body)?;
            eprintln!("[seed_catalog] name_zh_wfcd 对齐 ≈{n}");
        } else {
            eprintln!("[seed_catalog] 警告: 无 i18n-zh.json");
        }

        let applied = apply_display_zh_to_entities(&catalog)?;
        eprintln!("[seed_catalog] 实体展示中文写回 ≈{applied}");
    }

    let nick_path = res_dir.join("warframe_nicknames.json");
    let nicks = if nick_path.is_file() {
        read_nicknames_json(&nick_path)
    } else {
        Default::default()
    };
    {
        let conn = open_catalog(&catalog_path)?;
        let written = apply_nicknames_map(&conn, &nicks)?;
        eprintln!("[seed_catalog] 俗称 ≈{written}");
    }

    verify_seed(&catalog_path)?;
    verify_empty_user(&user_path)?;
    Ok(())
}

fn read_req(dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let p = dir.join(name);
    if !p.is_file() {
        return Err(format!("缺少资源文件: {}", p.display()));
    }
    fs::read(&p).map_err(|e| format!("读 {} 失败: {e}", p.display()))
}

fn verify_seed(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    let families: i64 = conn
        .query_row("SELECT COUNT(*) FROM warframe_family", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let i18n: i64 = conn
        .query_row("SELECT COUNT(*) FROM name_i18n", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let relics: i64 = conn
        .query_row("SELECT COUNT(*) FROM void_relic", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let arcs: i64 = conn
        .query_row("SELECT COUNT(*) FROM arcane", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let arcs_zh: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM arcane WHERE name_zh IS NOT NULL AND trim(name_zh) != '' AND name_zh != name_en",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if families < 20 {
        return Err(format!("种子战甲家族过少: {families}"));
    }
    if i18n < 1000 {
        return Err(format!("种子 name_i18n 过少: {i18n}"));
    }
    if relics < 50 {
        return Err(format!("种子遗物过少: {relics}"));
    }
    if arcs < 50 {
        return Err(format!("种子赋能过少: {arcs}"));
    }
    eprintln!(
        "[seed_catalog] 校验: families={families} i18n={i18n} relics={relics} arcanes={arcs} arcane_zh={arcs_zh}"
    );
    Ok(())
}

fn verify_empty_user(path: &Path) -> Result<(), String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM warframe_progress", [], |r| r.get(0))
        .unwrap_or(0);
    if n != 0 {
        return Err(format!("user 种子应无进度，实际 {n}"));
    }
    Ok(())
}
