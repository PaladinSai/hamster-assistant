use super::*;
use crate::db::{self, open_catalog};
use crate::id::Snowflake;
use rusqlite::params;
use std::fs;

use crate::progress::common::cmp_relic_drops;

fn drop_dto(name: &str, vaulted: i32, in_resurgence: bool) -> PartDropDto {
    PartDropDto {
        relic_name: name.into(),
        relic_key: name.into(),
        rarity: Some(0),
        vaulted,
        in_resurgence,
    }
}

#[test]
fn relic_drops_sort_unvaulted_resurgence_vaulted() {
    let mut drops = vec![
        drop_dto("已入库A", 1, false),
        drop_dto("重生B", 1, true),
        drop_dto("未入库C", 0, false),
        drop_dto("重生A", 0, true),
        drop_dto("未入库A", 0, false),
    ];
    drops.sort_by(cmp_relic_drops);
    let names: Vec<_> = drops.iter().map(|d| d.relic_name.as_str()).collect();
    assert_eq!(
        names,
        ["未入库A", "未入库C", "重生A", "重生B", "已入库A"]
    );
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "cangshu_progress_{tag}_{}_{}",
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
fn empty_appdata_lists_empty_until_catalog_update() {
    let dir = temp_dir("empty");
    db::init_databases_ex(&dir, false).unwrap();
    let list = list_warframes(&dir, Some("warframe")).unwrap();
    assert!(list.is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn upsert_survives_reload() {
    let dir = temp_dir("upsert");
    db::init_databases_ex(&dir, false).unwrap();
    let snow = Snowflake::new(1);
    let family_id = snow.next_id();
    let base_id = snow.next_id();
    let prime_id = snow.next_id();
    let part_id = snow.next_id();

    {
        let catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_family (id, family_key, name_en, nickname)
                 VALUES (?1, 'ash', 'Ash', '阿屎')",
                params![family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe (id, family_id, variant, name_en, vault_status)
                 VALUES (?1, ?2, 0, 'Ash', 0)",
                params![base_id, family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe (id, family_id, variant, name_en, vault_status)
                 VALUES (?1, ?2, 1, 'Ash Prime', 1)",
                params![prime_id, family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_part (id, warframe_id, part_key, name_zh, sort_order)
                 VALUES (?1, ?2, 'blueprint', '总蓝图', 0)",
                params![part_id, base_id],
            )
            .unwrap();
    }

    let bid = base_id.to_string();
    let pid = part_id.to_string();

    upsert_progress_warframe(
        &dir,
        UpsertWarframeProgress {
            warframe_id: bid.clone(),
            status: Some(3),
            helminth: Some(1),
            forma_done: Some(1),
            wish: Some(1),
            note: Some("冒烟".into()),
            mastered: None,
        },
    )
    .unwrap();

    let detail_after = get_warframe_detail(&dir, &bid).unwrap();
    assert_eq!(detail_after.parts[0].status, 3);

    upsert_progress_part(
        &dir,
        UpsertPartProgress {
            part_id: pid,
            status: Some(2),
            note: Some("齐".into()),
        },
    )
    .unwrap();

    let list = list_warframes(&dir, Some("warframe")).unwrap();
    assert_eq!(list.len(), 2);
    let base = list.iter().find(|x| x.variant == 0).unwrap();
    let prime = list.iter().find(|x| x.variant == 1).unwrap();
    assert_eq!(base.name_en, "Ash");
    assert_eq!(base.nickname.as_deref(), Some("阿屎"));
    assert_eq!(base.status, 2);
    assert_eq!(base.helminth, 1);
    assert_eq!(base.forma_done, 1);
    assert_eq!(base.wish, 1);
    assert_eq!(base.note.as_deref(), Some("冒烟"));
    assert_eq!(prime.name_en, "Ash Prime");
    assert_eq!(prime.vault_status, Some(1));

    let detail = get_warframe_detail(&dir, &bid).unwrap();
    assert_eq!(detail.parts.len(), 1);
    assert_eq!(detail.parts[0].status, 2);
    assert_eq!(detail.parts[0].note.as_deref(), Some("齐"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn parent_cascades_to_parts_and_parts_recompute_parent() {
    let dir = temp_dir("cascade");
    db::init_databases_ex(&dir, false).unwrap();
    let snow = Snowflake::new(1);
    let family_id = snow.next_id();
    let base_id = snow.next_id();
    let part_a = snow.next_id();
    let part_b = snow.next_id();

    {
        let catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_family (id, family_key, name_en)
                 VALUES (?1, 'excal', 'Excalibur')",
                params![family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe (id, family_id, variant, name_en)
                 VALUES (?1, ?2, 0, 'Excalibur')",
                params![base_id, family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_part (id, warframe_id, part_key, name_zh, sort_order)
                 VALUES (?1, ?2, 'neuroptics', '头部', 0), (?3, ?2, 'systems', '系统', 1)",
                params![part_a, base_id, part_b],
            )
            .unwrap();
    }

    let bid = base_id.to_string();

    upsert_progress_warframe(
        &dir,
        UpsertWarframeProgress {
            warframe_id: bid.clone(),
            status: Some(1),
            helminth: None,
            forma_done: None,
            wish: None,
            note: None,
            mastered: None,
        },
    )
    .unwrap();

    let detail = get_warframe_detail(&dir, &bid).unwrap();
    assert_eq!(detail.warframe.status, 1);
    assert_eq!(detail.parts.len(), 2);
    assert!(detail.parts.iter().all(|p| p.status == 1));

    upsert_progress_part(
        &dir,
        UpsertPartProgress {
            part_id: part_a.to_string(),
            status: Some(3),
            note: None,
        },
    )
    .unwrap();
    upsert_progress_part(
        &dir,
        UpsertPartProgress {
            part_id: part_b.to_string(),
            status: Some(3),
            note: None,
        },
    )
    .unwrap();

    let list = list_warframes(&dir, Some("warframe")).unwrap();
    assert_eq!(list[0].status, 3);

    upsert_progress_part(
        &dir,
        UpsertPartProgress {
            part_id: part_a.to_string(),
            status: Some(0),
            note: None,
        },
    )
    .unwrap();

    let list = list_warframes(&dir, Some("warframe")).unwrap();
    assert_eq!(list[0].status, 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn migrates_family_progress_to_variant_rows() {
    let dir = temp_dir("mig");
    db::init_databases_ex(&dir, false).unwrap();
    let snow = Snowflake::new(1);
    let family_id = snow.next_id();
    let base_id = snow.next_id();
    let prime_id = snow.next_id();

    {
        let catalog = open_catalog(&db::catalog_path(&dir)).unwrap();
        catalog
            .execute(
                "INSERT INTO warframe_family (id, family_key, name_en)
                 VALUES (?1, 'mag', 'Mag')",
                params![family_id],
            )
            .unwrap();
        catalog
            .execute(
                "INSERT INTO warframe (id, family_id, variant, name_en)
                 VALUES (?1, ?2, 0, 'Mag'), (?3, ?2, 1, 'Mag Prime')",
                params![base_id, family_id, prime_id],
            )
            .unwrap();
    }

    {
        let user = db::open_user(&db::user_path(&dir)).unwrap();
        // 模拟尚未跑数据迁移的旧表
        user.execute_batch(
            r#"
CREATE TABLE IF NOT EXISTS warframe_family_progress (
  family_id INTEGER NOT NULL PRIMARY KEY,
  base_status INTEGER NOT NULL DEFAULT 0,
  prime_status INTEGER NOT NULL DEFAULT 0,
  helminth INTEGER NOT NULL DEFAULT 0,
  forma_done INTEGER NOT NULL DEFAULT 0,
  wish INTEGER NOT NULL DEFAULT 0,
  wish_base INTEGER NOT NULL DEFAULT 0,
  wish_prime INTEGER NOT NULL DEFAULT 0,
  note TEXT,
  mastered_base INTEGER NOT NULL DEFAULT 0,
  mastered_prime INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT
);
DELETE FROM setting WHERE key = 'warframe_progress_v14';
"#,
        )
        .unwrap();
        user.execute(
            "INSERT INTO warframe_family_progress (
                family_id, base_status, prime_status, helminth, forma_done,
                wish, wish_base, wish_prime, note, mastered_base, mastered_prime
             ) VALUES (?1, 3, 1, 1, 1, 1, 1, 1, '旧备注', 0, 0)",
            params![family_id],
        )
        .unwrap();
    }

    // 数据迁移在 init 路径执行（不再挂在 list 热路径）
    db::init_databases_ex(&dir, false).unwrap();

    let list = list_warframes(&dir, Some("warframe")).unwrap();
    let base = list.iter().find(|x| x.variant == 0).unwrap();
    let prime = list.iter().find(|x| x.variant == 1).unwrap();
    assert_eq!(base.status, 3);
    assert_eq!(base.helminth, 1);
    assert_eq!(base.forma_done, 1);
    assert_eq!(base.note.as_deref(), Some("旧备注"));
    assert_eq!(base.wish, 1);
    assert_eq!(prime.status, 1);
    assert_eq!(prime.forma_done, 0);
    assert!(prime.note.is_none());
    assert_eq!(prime.wish, 1);

    let user = db::open_user(&db::user_path(&dir)).unwrap();
    assert!(!db::has_table(&user, "warframe_family_progress"));

    let _ = fs::remove_dir_all(&dir);
}
