//! WFCD 上游 URL 与 suit / weapon / modular 常量。

pub const DEFAULT_WARFRAMES_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Warframes.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Warframes.json",
];

pub const DEFAULT_ARCHWING_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Archwing.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Archwing.json",
];

pub const DEFAULT_SENTINELS_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Sentinels.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Sentinels.json",
];

pub const DEFAULT_PETS_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Pets.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Pets.json",
];

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

pub const DEFAULT_PRIMARY_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Primary.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Primary.json",
];

pub const DEFAULT_SECONDARY_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Secondary.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Secondary.json",
];

pub const DEFAULT_MELEE_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Melee.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Melee.json",
];

pub const DEFAULT_ARCH_GUN_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Arch-Gun.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arch-Gun.json",
];

pub const DEFAULT_ARCH_MELEE_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Arch-Melee.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arch-Melee.json",
];

pub const DEFAULT_MISC_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Misc.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Misc.json",
];

pub const WEAPON_KIND_WEAPON: &str = "weapon";
pub const WEAPON_KIND_ARCHWING: &str = "archwing";

/// 自制精通部件：组合枪枪膛 / Zaw 击打部 / 增幅器棱镜（对称概念见 [`crate::catalog::modular`]）
pub const MODULAR_KIND_KITGUN: &str = "kitgun";
pub const MODULAR_KIND_ZAW: &str = "zaw";
pub const MODULAR_KIND_AMP: &str = "amp";

pub const DEFAULT_MODS_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Mods.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Mods.json",
];

pub const DEFAULT_ARCANES_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/Arcanes.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/Arcanes.json",
];

/// WFCD 简中对照（按语言拆分的 `i18n/zh.json`；运行时拉取缓存，不打进安装包）
pub const DEFAULT_I18N_URLS: &[&str] = &[
    "https://cdn.jsdelivr.net/npm/@wfcd/items/data/json/i18n/zh.json",
    "https://cdn.jsdelivr.net/gh/WFCD/warframe-items@master/data/json/i18n/zh.json",
    "https://raw.githubusercontent.com/WFCD/warframe-items/master/data/json/i18n/zh.json",
];

