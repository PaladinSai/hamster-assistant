//! WFCD 上游 URL 与 suit / weapon / modular 常量。
//!
//! GitHub raw 只走 gh-proxy.org，不设 jsDelivr 等后备源。

pub const DEFAULT_WARFRAMES_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Warframes.json";
pub const DEFAULT_ARCHWING_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Archwing.json";
pub const DEFAULT_SENTINELS_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Sentinels.json";
pub const DEFAULT_PETS_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Pets.json";

pub const SUIT_KIND_WARFRAME: &str = "warframe";
pub const SUIT_KIND_ARCHWING: &str = "archwing";
pub const SUIT_KIND_NECRAMECH: &str = "necramech";
pub const SUIT_KIND_COMPANION: &str = "companion";

pub const COMPANION_TYPE_SENTINEL: &str = "sentinel";
pub const COMPANION_TYPE_KUBROW: &str = "kubrow";
pub const COMPANION_TYPE_KAVAT: &str = "kavat";
pub const COMPANION_TYPE_VULPAPHYLA: &str = "vulpaphyla";
pub const COMPANION_TYPE_PREDASITE: &str = "predasite";
/// 自制恐鸟型号（精通部件；对称概念见 [`crate::catalog::modular`]）
pub const COMPANION_TYPE_MOA: &str = "moa";
/// 自制猎犬型号（精通部件；对称概念见 [`crate::catalog::modular`]）
pub const COMPANION_TYPE_HOUND: &str = "hound";

pub const DEFAULT_PRIMARY_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Primary.json";
pub const DEFAULT_SECONDARY_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Secondary.json";
pub const DEFAULT_MELEE_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Melee.json";
pub const DEFAULT_ARCH_GUN_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arch-Gun.json";
pub const DEFAULT_ARCH_MELEE_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arch-Melee.json";
pub const DEFAULT_MISC_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Misc.json";

pub const WEAPON_KIND_WEAPON: &str = "weapon";
pub const WEAPON_KIND_ARCHWING: &str = "archwing";

/// 自制精通部件：组合枪枪膛 / Zaw 击打部 / 增幅器棱镜（对称概念见 [`crate::catalog::modular`]）
pub const MODULAR_KIND_KITGUN: &str = "kitgun";
pub const MODULAR_KIND_ZAW: &str = "zaw";
pub const MODULAR_KIND_AMP: &str = "amp";

pub const DEFAULT_MODS_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Mods.json";
pub const DEFAULT_ARCANES_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arcanes.json";

/// WFCD 简中对照（`i18n/zh.json`；只走 GitHub raw 代理，不打进安装包）
pub const DEFAULT_I18N_URL: &str = "https://gh-proxy.org/https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/i18n/zh.json";

