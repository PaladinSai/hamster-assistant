//! 武器家族 / 变体规则、部件元数据、灵化之源白名单与 WFCD DTO。

use serde::Deserialize;

pub const SLOT_PRIMARY: i32 = 0;
pub const SLOT_SECONDARY: i32 = 1;
pub const SLOT_MELEE: i32 = 2;
/// 自制精通部件（组合枪枪膛 / Zaw 击打部 / 增幅器棱镜）
pub const SLOT_MODULAR: i32 = 3;

pub const VARIANT_BASE: i32 = 0;
pub const VARIANT_PRIME: i32 = 1;
pub const VARIANT_KUVA: i32 = 2;
pub const VARIANT_TENET: i32 = 3;
pub const VARIANT_CODA: i32 = 4;
pub const VARIANT_MK1: i32 = 5;
pub const VARIANT_DEX: i32 = 6;
pub const VARIANT_VANDAL: i32 = 7;
pub const VARIANT_WRAITH: i32 = 8;
pub const VARIANT_PRISMA: i32 = 9;

#[derive(Debug, Default, Clone)]
pub struct WeaponImportStats {
    pub families: usize,
    pub weapons: usize,
    pub parts: usize,
}

#[derive(Debug, Deserialize)]
pub struct WfcdWeapon {
    pub name: String,
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub vaulted: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub category: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub item_type: Option<String>,
    #[serde(rename = "productCategory")]
    #[serde(default)]
    pub product_category: Option<String>,
    #[serde(default)]
    pub components: Vec<WfcdWeaponComponent>,
}

#[derive(Debug, Deserialize)]
pub struct WfcdWeaponComponent {
    /// WFCD 现多为轻量引用，仅有 uniqueName；缺省时由后缀归一化填入。
    #[serde(default)]
    pub name: String,
    #[serde(rename = "uniqueName")]
    pub unique_name: Option<String>,
    #[serde(rename = "itemCount")]
    pub item_count: Option<i32>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub item_type: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct WeaponPartMeta {
    pub key: &'static str,
    pub sort: i32,
    pub name_zh: &'static str,
}

/// 铸造部件（材料不在此表）。
pub fn weapon_part_meta(name: &str) -> Option<WeaponPartMeta> {
    let (key, sort, name_zh) = match name {
        "Blueprint" => ("blueprint", 0, "蓝图"),
        "Barrel" => ("barrel", 1, "枪管"),
        "Receiver" => ("receiver", 2, "枪机"),
        "Stock" => ("stock", 3, "枪托"),
        "Blade" => ("blade", 4, "刀刃"),
        "Blades" => ("blades", 5, "刀刃"),
        "Handle" => ("handle", 6, "握柄"),
        "Grip" => ("grip", 7, "握把"),
        "Link" => ("link", 8, "链接"),
        "String" => ("string", 9, "弓弦"),
        "Lower Limb" => ("lower_limb", 10, "下臂"),
        "Upper Limb" => ("upper_limb", 11, "上臂"),
        "Gauntlet" => ("gauntlet", 12, "护手"),
        "Guard" => ("guard", 13, "护手"),
        "Hilt" => ("hilt", 14, "柄"),
        "Head" => ("head", 15, "头部"),
        "Chain" => ("chain", 16, "链条"),
        "Disc" => ("disc", 17, "圆盘"),
        "Ornament" => ("ornament", 18, "饰物"),
        "Pouch" => ("pouch", 19, "弹药包"),
        "Stars" => ("stars", 20, "飞星"),
        "Band" => ("band", 21, "束带"),
        "Boot" => ("boot", 22, "靴部"),
        "Buckle" => ("buckle", 23, "扣环"),
        _ => return None,
    };
    Some(WeaponPartMeta {
        key,
        sort,
        name_zh,
    })
}

/// 家族键与变体码。
pub fn weapon_family_and_variant(name: &str) -> (String, i32) {
    let n = name.trim();
    if let Some(base) = strip_prefix_ci(n, "Kuva ") {
        return (base.to_string(), VARIANT_KUVA);
    }
    if let Some(base) = strip_prefix_ci(n, "Tenet ") {
        return (base.to_string(), VARIANT_TENET);
    }
    if let Some(base) = strip_prefix_ci(n, "Coda ") {
        return (base.to_string(), VARIANT_CODA);
    }
    if let Some(base) = strip_prefix_ci(n, "Dex ") {
        return (base.to_string(), VARIANT_DEX);
    }
    if let Some(base) = strip_prefix_ci(n, "Prisma ") {
        return (base.to_string(), VARIANT_PRISMA);
    }
    if let Some(base) = strip_mk1(n) {
        return (base.to_string(), VARIANT_MK1);
    }
    if let Some(base) = n.strip_suffix(" Prime") {
        return (base.to_string(), VARIANT_PRIME);
    }
    if let Some(base) = n.strip_suffix(" Vandal") {
        return (base.to_string(), VARIANT_VANDAL);
    }
    if let Some(base) = n.strip_suffix(" Wraith") {
        return (base.to_string(), VARIANT_WRAITH);
    }
    (n.to_string(), VARIANT_BASE)
}

fn strip_prefix_ci<'a>(name: &'a str, prefix: &str) -> Option<&'a str> {
    if name.len() < prefix.len() {
        return None;
    }
    if name
        .get(..prefix.len())?
        .eq_ignore_ascii_case(prefix)
    {
        return Some(name[prefix.len()..].trim_start());
    }
    None
}

fn strip_mk1(name: &str) -> Option<&str> {
    for p in ["Mk1-", "MK1-", "mk1-"] {
        if let Some(rest) = name.strip_prefix(p) {
            let rest = rest.trim_start();
            if !rest.is_empty() {
                return Some(rest);
            }
        }
    }
    None
}

pub(crate) fn is_excluded_weapon(w: &WfcdWeapon) -> bool {
    if w.unique_name.contains("PvPVariant") {
        return true;
    }
    if w.item_type.as_deref() == Some("Zaw Component") {
        return true;
    }
    if w.product_category.as_deref() == Some("OperatorAmps") {
        return true;
    }
    false
}

/// 真·曲翼槽：WFCD `productCategory` 为 SpaceGuns / SpaceMelee。
/// Arch-Melee.json 会误收地面近战（如闪劫 Prime，productCategory=Melee）。
pub(crate) fn is_space_product_category(w: &WfcdWeapon, expected: &str) -> bool {
    w.product_category
        .as_deref()
        .is_some_and(|c| c.eq_ignore_ascii_case(expected))
}

/// Steel Path Circuit「灵化之源」适配器对应的家族键（Duviri 轮换表）。
const INCARNON_GENESIS_FAMILIES: &[&str] = &[
    // Week 1
    "Braton",
    "Lato",
    "Skana",
    "Paris",
    "Kunai",
    // Week 2
    "Boar",
    "Gammacor",
    "Angstrum",
    "Gorgon",
    "Anku",
    // Week 3
    "Bo",
    "Latron",
    "Furis",
    "Furax",
    "Strun",
    // Week 4
    "Lex",
    "Magistar",
    "Boltor",
    "Bronco",
    "Ceramic Dagger",
    // Week 5
    "Torid",
    "Dual Toxocyst",
    "Dual Ichor",
    "Miter",
    "Atomos",
    // Week 6
    "Ack & Brunt",
    "Soma",
    "Vasto",
    "Nami Solo",
    "Burston",
    // Week 7
    "Zylok",
    "Sibear",
    "Dread",
    "Despair",
    "Hate",
    // Week 8
    "Dera",
    "Sybaris",
    "Cestra",
    "Sicarus",
    "Okina",
    // Week 9
    "Vectis",
    "Stug",
    "Ballistica",
    "Destreza",
    "Obex",
];

const SYNDICATE_FAMILY_PREFIXES: &[&str] = &["Synoid ", "Sancti ", "Telos "];

fn genesis_family_key(family_key: &str) -> &str {
    for prefix in SYNDICATE_FAMILY_PREFIXES {
        if let Some(rest) = family_key.strip_prefix(prefix) {
            if !rest.is_empty() {
                return rest;
            }
        }
    }
    family_key
}

fn family_has_incarnon_genesis(family_key: &str) -> bool {
    let key = genesis_family_key(family_key);
    INCARNON_GENESIS_FAMILIES.iter().any(|&k| k == key)
}

/// 是否靠「灵化之源」适配器（非 Zariman/Sanctum 自带灵化）。
pub fn is_incarnon_genesis_family(family_key: &str) -> bool {
    family_has_incarnon_genesis(family_key)
}

/// 自带灵化 tag，或家族在灵化之源白名单内；赤毒/信条/终幕不可灵化。
pub(crate) fn can_incarnon(family_key: &str, variant: i32, tags: &[String]) -> i32 {
    if skip_craft_parts(variant) {
        return 0;
    }
    if tags.iter().any(|t| t.eq_ignore_ascii_case("Incarnon")) {
        return 1;
    }
    if family_has_incarnon_genesis(family_key) {
        return 1;
    }
    0
}

pub(crate) fn skip_craft_parts(variant: i32) -> bool {
    matches!(
        variant,
        VARIANT_KUVA | VARIANT_TENET | VARIANT_CODA
    )
}

pub(crate) fn is_material_component(c: &WfcdWeaponComponent) -> bool {
    if c.item_type.as_deref() == Some("Resource") {
        return true;
    }
    let uniq = c.unique_name.as_deref().unwrap_or("");
    uniq.contains("/MiscItems/") || uniq.contains("/Items/MiscItems/")
}

/// 从 WFCD 部件 uniqueName 叶子还原给 `weapon_part_meta` 用的英文名。
pub fn resolve_weapon_component_name(unique_name: &str) -> Option<String> {
    let leaf = unique_name.rsplit('/').next().unwrap_or("");
    if leaf.is_empty() {
        return None;
    }
    // 较长后缀优先，避免 Blade 吃掉 Blades 等
    const SUFFIXES: &[(&str, &str)] = &[
        ("LowerLimb", "Lower Limb"),
        ("UpperLimb", "Upper Limb"),
        ("Gauntlet", "Gauntlet"),
        ("Ornament", "Ornament"),
        ("Receiver", "Receiver"),
        ("Blueprint", "Blueprint"),
        ("Blades", "Blades"),
        ("Barrel", "Barrel"),
        ("Handle", "Handle"),
        ("String", "String"),
        ("Guard", "Guard"),
        ("Stock", "Stock"),
        ("Blade", "Blade"),
        ("Grip", "Grip"),
        ("Link", "Link"),
        ("Hilt", "Hilt"),
        ("Head", "Head"),
        ("Chain", "Chain"),
        ("Disc", "Disc"),
        ("Pouch", "Pouch"),
        ("Stars", "Stars"),
        ("Band", "Band"),
        ("Boot", "Boot"),
        ("Buckle", "Buckle"),
    ];
    for (suf, name) in SUFFIXES {
        if leaf.ends_with(suf) {
            return Some((*name).into());
        }
    }
    None
}

pub(crate) fn normalize_weapon_component(c: &mut WfcdWeaponComponent) {
    if !c.name.trim().is_empty() {
        return;
    }
    let uniq = c.unique_name.as_deref().unwrap_or("");
    if let Some(n) = resolve_weapon_component_name(uniq) {
        c.name = n;
    }
}
