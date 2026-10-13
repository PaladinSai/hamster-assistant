//! 武器目录域：家族 / 变体规则 + JSON parse + sqlite 入库事务。
//!
//! - 上游 URL / HTTP：[`crate::catalog::wfcd`]（本模块不拉网）
//! - 机体入库：[`crate::catalog::import`]
//! - 自制精通部件（kitgun/zaw/amp）概念见 [`crate::catalog::modular`]；同伴侧对称见 `import_modular_companions_json`

mod import;
mod parse;
mod rules;

#[cfg(test)]
mod tests;

pub use import::{
    import_archwing_weapons_json, import_modular_weapons_json, import_weapons_json,
};
#[allow(unused_imports)] // 公共 API 面；部分仅供 crate 外 / 测试使用
pub use rules::{
    is_incarnon_genesis_family, resolve_weapon_component_name, weapon_family_and_variant,
    weapon_part_meta, WeaponImportStats, WeaponPartMeta, WfcdWeapon, WfcdWeaponComponent,
    SLOT_MELEE, SLOT_MODULAR, SLOT_PRIMARY, SLOT_SECONDARY, VARIANT_BASE, VARIANT_CODA,
    VARIANT_DEX, VARIANT_KUVA, VARIANT_MK1, VARIANT_PRIME, VARIANT_PRISMA, VARIANT_TENET,
    VARIANT_VANDAL, VARIANT_WRAITH,
};
