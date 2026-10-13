//! WFCD JSON 轻量 parse + 家族 / 部件规则（不写库）。

use serde::Deserialize;
use serde_json::Value;

use super::constants::*;

#[derive(Debug, Deserialize)]
pub struct WfcdWarframe {
    pub name: String,
    #[serde(rename = "uniqueName")]
    pub unique_name: String,
    #[serde(rename = "isPrime")]
    #[allow(dead_code)]
    pub is_prime: Option<bool>,
    pub vaulted: Option<bool>,
    #[serde(default)]
    #[allow(dead_code)]
    pub category: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    #[allow(dead_code)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub components: Vec<WfcdComponent>,
}

#[derive(Debug, Deserialize)]
pub struct WfcdComponent {
    /// WFCD 现多为轻量引用，仅有 uniqueName；缺省时由后缀归一化填入。
    #[serde(default)]
    pub name: String,
    #[serde(rename = "uniqueName")]
    #[allow(dead_code)]
    pub unique_name: Option<String>,
    #[serde(default)]
    pub drops: Vec<WfcdDrop>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WfcdDrop {
    #[serde(default)]
    pub location: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    #[allow(dead_code)]
    pub drop_type: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub rarity: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub chance: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub struct PartMeta {
    pub key: &'static str,
    pub sort: i32,
    pub name_zh: &'static str,
}

/// 部件键、排序、中文名。Equinox 白昼/黑夜面用扩展键；Archwing / 机甲额外部件见下。
pub fn part_meta(name: &str) -> Option<PartMeta> {
    let (key, sort, name_zh) = match name {
        "Blueprint" => ("blueprint", 0, "总蓝图"),
        "Neuroptics" => ("neuroptics", 1, "头部 Neuroptics"),
        "Chassis" => ("chassis", 2, "机体 Chassis"),
        "Systems" => ("systems", 3, "系统 Systems"),
        // Archwing
        "Wings" => ("wings", 4, "双翼 Wings"),
        // Necramech
        "Engine" => ("engine", 4, "引擎 Engine"),
        "Weapon Pod" => ("weapon_pod", 5, "武器舱 Weapon Pod"),
        // Sentinel 守护
        "Carapace" => ("carapace", 1, "外壳 Carapace"),
        "Cerebrum" => ("cerebrum", 2, "大脑 Cerebrum"),
        "Day Aspect Blueprint" => ("day_blueprint", 10, "白昼面蓝图"),
        "Day Aspect Systems" => ("day_systems", 11, "白昼面系统"),
        "Day Aspect Chassis" => ("day_chassis", 12, "白昼面机体"),
        "Day Aspect Neuroptics" => ("day_neuroptics", 13, "白昼面头"),
        "Night Aspect Blueprint" => ("night_blueprint", 20, "黑夜面蓝图"),
        "Night Aspect Systems" => ("night_systems", 21, "黑夜面系统"),
        "Night Aspect Chassis" => ("night_chassis", 22, "黑夜面机体"),
        "Night Aspect Neuroptics" => ("night_neuroptics", 23, "黑夜面头"),
        _ => return None,
    };
    Some(PartMeta {
        key,
        sort,
        name_zh,
    })
}

/// 由 uniqueName / type 判定家族分类（侧栏：战甲 / Archwing / 机甲 / 同伴）。
pub fn suit_kind_for(unique_name: &str, item_type: Option<&str>) -> &'static str {
    let ty = item_type.unwrap_or("");
    if unique_name.contains("/EntratiMech/") || ty.eq_ignore_ascii_case("Necramech") {
        return SUIT_KIND_NECRAMECH;
    }
    if unique_name.contains("/Archwing/") || ty.eq_ignore_ascii_case("Archwing") {
        return SUIT_KIND_ARCHWING;
    }
    if companion_type_for(unique_name, None, ty).is_some() {
        return SUIT_KIND_COMPANION;
    }
    SUIT_KIND_WARFRAME
}

/// 同伴细分类型；非同伴返回 None。
pub fn companion_type_for(
    unique_name: &str,
    name: Option<&str>,
    item_type: &str,
) -> Option<&'static str> {
    let n = name.unwrap_or("");
    if unique_name.contains("/Sentinels/") || item_type.eq_ignore_ascii_case("Sentinel") {
        return Some(COMPANION_TYPE_SENTINEL);
    }
    // Khora 显赫猫：不算同伴目录
    if unique_name.contains("/Khora/") || n.starts_with("Venari") {
        return None;
    }
    // 自制精通型号（恐鸟头 / 猎犬头）
    if let Some(ty) = modular_companion_type(unique_name) {
        return Some(ty);
    }
    if n.contains("Vulpaphyla")
        || unique_name.contains("InfestedCatbrow")
        || unique_name.contains("VulpineInfested")
        || unique_name.contains("ArmoredInfested")
        || unique_name.contains("HornedInfested")
    {
        return Some(COMPANION_TYPE_VULPAPHYLA);
    }
    if n.contains("Predasite") || unique_name.contains("Predator") {
        return Some(COMPANION_TYPE_PREDASITE);
    }
    if unique_name.contains("/CatbrowPet/") || n.contains("Kavat") {
        return Some(COMPANION_TYPE_KAVAT);
    }
    if unique_name.contains("/KubrowPet/")
        || n.contains("Kubrow")
        || n.eq_ignore_ascii_case("Helminth Charger")
    {
        return Some(COMPANION_TYPE_KUBROW);
    }
    None
}

/// 恐鸟 / 猎犬型号部件（镀金后计精通；武器侧对称见 `weapons` kitgun/zaw/amp，概念见 [`crate::catalog::modular`]）。
pub fn modular_companion_type(unique_name: &str) -> Option<&'static str> {
    if unique_name.contains("PvP") {
        return None;
    }
    if unique_name.contains("MoaPetHead") || unique_name.contains("/MoaPetParts/MoaPetHead") {
        return Some(COMPANION_TYPE_MOA);
    }
    if unique_name.contains("ZanukaPetPartHead") {
        return Some(COMPANION_TYPE_HOUND);
    }
    None
}

/// 国际服简中回退（userdict 常只有「Lambeo」「Bhaira Model」等键）。
pub fn modular_companion_zh_fallback(name_en: &str) -> Option<&'static str> {
    Some(match name_en {
        "Lambeo Moa" => "拉姆比欧恐鸟",
        "Nychus Moa" => "尼丘斯恐鸟",
        "Oloro Moa" => "欧罗若恐鸟",
        "Para Moa" => "帕拉恐鸟",
        "Bhaira Hound" | "Bhaira Model" => "拜拉猎犬",
        "Dorma Hound" | "Dorma Model" => "铎玛猎犬",
        "Hec Hound" | "Hec Model" => "骸克猎犬",
        _ => return None,
    })
}

/// 变体码：0 本体 / 1 Prime / 2 Umbra
pub const VARIANT_BASE: i32 = 0;
pub const VARIANT_PRIME: i32 = 1;
pub const VARIANT_UMBRA: i32 = 2;

/// 灰机正式名；WFCD 另有空壳 `Orion & Sirius`，导入时丢弃。
pub const GEMINI_FAMILY_KEY: &str = "Sirius & Orion";

/// 解析家族键与变体码：0 本体 / 1 Prime / 2 Umbra
pub fn family_and_variant(name: &str) -> (String, i32) {
    match name {
        "Sirius & Orion" | "Orion & Sirius" => {
            return (GEMINI_FAMILY_KEY.to_string(), VARIANT_BASE);
        }
        _ => {}
    }
    if let Some(base) = name.strip_suffix(" Prime") {
        return (base.to_string(), VARIANT_PRIME);
    }
    if let Some(base) = name.strip_suffix(" Umbra") {
        return (base.to_string(), VARIANT_UMBRA);
    }
    (name.to_string(), VARIANT_BASE)
}

/// 剔除 Helminth / 空壳 Orion；Necramech 与 Archwing 另行入库（suit_kind）。
pub fn is_excluded_warframe(name: &str, unique_name: &str) -> bool {
    if unique_name.contains("/PowersuitAbilities/Helminth") || name == "Helminth" {
        return true;
    }
    // WFCD 把双子拆成两条；只保留 SiriusSuit（四件套），丢掉空的 OrionSuit
    if unique_name.contains("/SiriusOrion/OrionSuit") || name == "Orion & Sirius" {
        return true;
    }
    false
}

/// 入库码。本体固定 0（无入库含义）。Prime：1 未入库（即可刷），2 已入库。重生 3 由当期轮次另写。
pub fn vault_status_code(is_prime: bool, vaulted: Option<bool>) -> i32 {
    if !is_prime {
        return 0;
    }
    match vaulted {
        Some(true) => 2,
        _ => 1,
    }
}

/// 从 WFCD 部件 uniqueName 叶子还原给 `part_meta` 用的英文名。
pub fn resolve_warframe_component_name(unique_name: &str) -> Option<String> {
    let leaf = unique_name.rsplit('/').next().unwrap_or("");
    if leaf.is_empty() {
        return None;
    }
    // Equinox 昼夜面（须先于通用 Blueprint）
    if leaf.ends_with("AnimusComponent") {
        return Some("Day Aspect".into());
    }
    if leaf.ends_with("AnimaComponent") && !leaf.contains("AnimaAnimus") {
        return Some("Night Aspect".into());
    }
    if leaf.ends_with("HelmetComponent") {
        return Some("Neuroptics".into());
    }
    if leaf.ends_with("ChassisComponent") || leaf.ends_with("PartChassisItem") {
        return Some("Chassis".into());
    }
    if leaf.ends_with("SystemsComponent") || leaf.ends_with("PartSystemsItem") {
        return Some("Systems".into());
    }
    if leaf.ends_with("WingsComponent") {
        return Some("Wings".into());
    }
    if leaf.ends_with("PartEngineItem") {
        return Some("Engine".into());
    }
    if leaf.ends_with("PartWeaponPodItem") {
        return Some("Weapon Pod".into());
    }
    // Sentinel：PrimeCarrierCarapace / …Cerebrum / …Systems
    if leaf.ends_with("Carapace") {
        return Some("Carapace".into());
    }
    if leaf.ends_with("Cerebrum") {
        return Some("Cerebrum".into());
    }
    if leaf.ends_with("Systems") {
        return Some("Systems".into());
    }
    // 守护条目偶发挂上武器蓝图，不当作品件
    if leaf.contains("WeaponBlueprint") {
        return None;
    }
    if leaf.ends_with("Blueprint") {
        return Some("Blueprint".into());
    }
    None
}

fn normalize_warframe_component(c: &mut WfcdComponent) {
    if !c.name.trim().is_empty() {
        return;
    }
    let uniq = c.unique_name.as_deref().unwrap_or("");
    if let Some(n) = resolve_warframe_component_name(uniq) {
        c.name = n;
    }
}

pub fn parse_warframes(bytes: &[u8]) -> Result<Vec<WfcdWarframe>, String> {
    parse_suit_items(
        bytes,
        "Warframes",
        |category, ty| {
            category == "Warframes"
                || ty == "Warframe"
                || ty.eq_ignore_ascii_case("Necramech")
        },
    )
}

/// WFCD `Archwing.json`（SpaceSuits）。
pub fn parse_archwings(bytes: &[u8]) -> Result<Vec<WfcdWarframe>, String> {
    parse_suit_items(bytes, "Archwing", |category, ty| {
        category == "Archwing"
            || ty.eq_ignore_ascii_case("Archwing")
            || category == "SpaceSuits"
    })
}

/// WFCD `Sentinels.json`。
pub fn parse_sentinels(bytes: &[u8]) -> Result<Vec<WfcdWarframe>, String> {
    parse_suit_items(bytes, "Sentinels", |category, ty| {
        category == "Sentinels" || ty.eq_ignore_ascii_case("Sentinel")
    })
}

/// WFCD `Pets.json`：只要完整同伴 `*PetPowerSuit`（跳过部件 / Venari）。
pub fn parse_pets(bytes: &[u8]) -> Result<Vec<WfcdWarframe>, String> {
    let items = parse_suit_items(bytes, "Pets", |category, ty| {
        category == "Pets" || ty.eq_ignore_ascii_case("Pets")
    })?;
    Ok(items
        .into_iter()
        .filter(|wf| {
            if !wf.unique_name.ends_with("PetPowerSuit") {
                return false;
            }
            companion_type_for(
                &wf.unique_name,
                Some(&wf.name),
                wf.item_type.as_deref().unwrap_or(""),
            )
            .is_some()
        })
        .collect())
}

/// WFCD `Pets.json`：自制恐鸟 / 猎犬型号（精通部件，非完整 PetPowerSuit）。
/// 入库见 [`crate::catalog::import::import_modular_companions_json`]；概念见 [`crate::catalog::modular`]。
pub fn parse_modular_companions(bytes: &[u8]) -> Result<Vec<WfcdWarframe>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Pets JSON 解析失败: {e}"))?;
    let Value::Array(arr) = value else {
        return Err("Pets.json 根节点不是数组".to_string());
    };
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for item in arr {
        let unique_name = item
            .get("uniqueName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if unique_name.is_empty() || modular_companion_type(&unique_name).is_none() {
            continue;
        }
        if !seen.insert(unique_name) {
            continue;
        }
        let mut wf: WfcdWarframe = match serde_json::from_value(item) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[仓鼠] 跳过自制同伴: {e}");
                continue;
            }
        };
        if wf.name.trim().is_empty() {
            continue;
        }
        // 型号无铸造部件行
        wf.components.clear();
        out.push(wf);
    }
    Ok(out)
}

fn parse_suit_items(
    bytes: &[u8],
    label: &str,
    accept: impl Fn(&str, &str) -> bool,
) -> Result<Vec<WfcdWarframe>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("{label} JSON 解析失败: {e}"))?;
    let Value::Array(arr) = value else {
        return Err(format!("{label}.json 根节点不是数组"));
    };
    let mut out = Vec::new();
    for item in arr {
        let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("");
        let ty = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if !accept(category, ty) {
            continue;
        }
        let mut wf: WfcdWarframe = match serde_json::from_value(item) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[仓鼠] 跳过单条{label}: {e}");
                continue;
            }
        };
        if wf.name.trim().is_empty() || wf.unique_name.trim().is_empty() {
            continue;
        }
        for c in &mut wf.components {
            normalize_warframe_component(c);
        }
        if is_excluded_warframe(&wf.name, &wf.unique_name) {
            continue;
        }
        if wf.name == "Equinox" {
            expand_equinox_aspects(&mut wf.components);
        }
        out.push(wf);
    }
    Ok(out)
}

fn expand_equinox_aspects(components: &mut Vec<WfcdComponent>) {
    const DAY: &[&str] = &[
        "Day Aspect Blueprint",
        "Day Aspect Systems",
        "Day Aspect Chassis",
        "Day Aspect Neuroptics",
    ];
    const NIGHT: &[&str] = &[
        "Night Aspect Blueprint",
        "Night Aspect Systems",
        "Night Aspect Chassis",
        "Night Aspect Neuroptics",
    ];

    let mut extra = Vec::new();
    for comp in components.iter() {
        let names = match comp.name.as_str() {
            "Day Aspect" => DAY,
            "Night Aspect" => NIGHT,
            _ => continue,
        };
        let drops = simplify_aspect_drops(&comp.drops);
        for name in names {
            extra.push(WfcdComponent {
                name: (*name).to_string(),
                unique_name: None,
                drops: drops.clone(),
            });
        }
    }
    if extra.is_empty() {
        return;
    }
    components.retain(|c| c.name != "Day Aspect" && c.name != "Night Aspect");
    components.extend(extra);
}

fn simplify_aspect_drops(drops: &[WfcdDrop]) -> Vec<WfcdDrop> {
    drops
        .iter()
        .map(|d| {
            let location = d.location.as_ref().map(|s| {
                s.split(',')
                    .next()
                    .unwrap_or(s)
                    .trim()
                    .to_string()
            });
            WfcdDrop {
                location,
                drop_type: d.drop_type.clone(),
                rarity: None,
                chance: None,
            }
        })
        .collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_keeps_necramech_skips_helminth() {
        let json = r#"[
          {"name":"Ash","uniqueName":"/Lotus/Powersuits/Ninja/Ninja","category":"Warframes","type":"Warframe","components":[]},
          {"name":"Bonewidow","uniqueName":"/Lotus/Powersuits/EntratiMech/ThanoTech","category":"Warframes","type":"Necramech","components":[]},
          {"name":"Voidrig","uniqueName":"/Lotus/Powersuits/EntratiMech/NechroTech","category":"Warframes","type":"Necramech","components":[]},
          {"name":"Helminth","uniqueName":"/Lotus/Powersuits/PowersuitAbilities/Helminth","category":"Warframes","type":"Warframe","components":[]}
        ]"#;
        let items = parse_warframes(json.as_bytes()).unwrap();
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, vec!["Ash", "Bonewidow", "Voidrig"]);
        assert_eq!(
            suit_kind_for(&items[1].unique_name, items[1].item_type.as_deref()),
            SUIT_KIND_NECRAMECH
        );
    }

    #[test]
    fn companion_types_from_unique_name() {
        assert_eq!(
            companion_type_for(
                "/Lotus/Types/Sentinels/SentinelPowersuits/CarrierPowerSuit",
                Some("Carrier"),
                "Sentinel"
            ),
            Some(COMPANION_TYPE_SENTINEL)
        );
        assert_eq!(
            companion_type_for(
                "/Lotus/Types/Game/KubrowPet/RetrieverKubrowPetPowerSuit",
                Some("Chesa Kubrow"),
                "Pets"
            ),
            Some(COMPANION_TYPE_KUBROW)
        );
        assert_eq!(
            companion_type_for(
                "/Lotus/Types/Game/CatbrowPet/CheshireCatbrowPetPowerSuit",
                Some("Smeeta Kavat"),
                "Pets"
            ),
            Some(COMPANION_TYPE_KAVAT)
        );
        assert_eq!(
            companion_type_for(
                "/Lotus/Types/Friendly/Pets/CreaturePets/ArmoredInfestedCatbrowPetPowerSuit",
                Some("Panzer Vulpaphyla"),
                "Pets"
            ),
            Some(COMPANION_TYPE_VULPAPHYLA)
        );
        assert_eq!(
            companion_type_for(
                "/Lotus/Types/Friendly/Pets/CreaturePets/MedjayPredatorKubrowPetPowerSuit",
                Some("Medjay Predasite"),
                "Pets"
            ),
            Some(COMPANION_TYPE_PREDASITE)
        );
        assert_eq!(
            companion_type_for(
                "/Lotus/Powersuits/Khora/Kavat/KhoraKavatPowerSuit",
                Some("Venari"),
                "Warframe"
            ),
            None
        );
        assert_eq!(
            modular_companion_type(
                "/Lotus/Types/Friendly/Pets/MoaPets/MoaPetParts/MoaPetHeadPara"
            ),
            Some(COMPANION_TYPE_MOA)
        );
        assert_eq!(
            modular_companion_type(
                "/Lotus/Types/Friendly/Pets/ZanukaPets/ZanukaPetParts/ZanukaPetPartHeadA"
            ),
            Some(COMPANION_TYPE_HOUND)
        );
        assert_eq!(modular_companion_zh_fallback("Para Moa"), Some("帕拉恐鸟"));
        assert_eq!(
            modular_companion_zh_fallback("Dorma Hound"),
            Some("铎玛猎犬")
        );
    }

    #[test]
    fn parse_modular_companions_models_only() {
        let json = r#"[
          {"name":"Para Moa","uniqueName":"/Lotus/Types/Friendly/Pets/MoaPets/MoaPetParts/MoaPetHeadPara","type":"Pets","category":"Pets","components":[]},
          {"name":"Dorma Hound","uniqueName":"/Lotus/Types/Friendly/Pets/ZanukaPets/ZanukaPetParts/ZanukaPetPartHeadA","type":"Pets","category":"Pets","components":[]},
          {"name":"Drex Core","uniqueName":"/Lotus/Types/Friendly/Pets/MoaPets/MoaPetParts/MoaPetEngineThricore","type":"Pet Resource","category":"Pets","components":[]},
          {"name":"Chesa Kubrow","uniqueName":"/Lotus/Types/Game/KubrowPet/RetrieverKubrowPetPowerSuit","type":"Pets","category":"Pets","components":[]}
        ]"#;
        let items = parse_modular_companions(json.as_bytes()).unwrap();
        assert_eq!(items.len(), 2);
        let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Para Moa"));
        assert!(names.contains(&"Dorma Hound"));
    }

    #[test]
    fn parse_archwing_space_suits() {
        let json = r#"[
          {"name":"Itzal","uniqueName":"/Lotus/Powersuits/Archwing/StealthJetPack/StealthJetPack","category":"Archwing","type":"Archwing","components":[
            {"uniqueName":"/Lotus/Types/Recipes/ArchwingRecipes/StealthArchwingWingsComponent"}
          ]}
        ]"#;
        let items = parse_archwings(json.as_bytes()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].components[0].name, "Wings");
        assert_eq!(
            suit_kind_for(&items[0].unique_name, items[0].item_type.as_deref()),
            SUIT_KIND_ARCHWING
        );
    }

    #[test]
    fn gemini_maps_to_one_family() {
        assert_eq!(
            family_and_variant("Sirius & Orion"),
            (GEMINI_FAMILY_KEY.to_string(), VARIANT_BASE)
        );
        assert!(is_excluded_warframe(
            "Orion & Sirius",
            "/Lotus/Powersuits/SiriusOrion/OrionSuit"
        ));
    }

    #[test]
    fn equinox_aspect_parts() {
        let day = part_meta("Day Aspect Neuroptics").unwrap();
        assert_eq!(day.key, "day_neuroptics");
        assert_eq!(day.name_zh, "白昼面头");
        let night = part_meta("Night Aspect Systems").unwrap();
        assert_eq!(night.name_zh, "黑夜面系统");
        assert!(part_meta("Blueprint").unwrap().sort < day.sort);
    }

    #[test]
    fn equinox_expands_collapsed_aspects() {
        let json = r#"[
          {"name":"Equinox","uniqueName":"/Lotus/Powersuits/YinYang/YinYang","category":"Warframes","type":"Warframe","components":[
            {"name":"Blueprint"},
            {"name":"Day Aspect","drops":[{"location":"Uranus/Titania (Assassination), Rotation A Equinox Day Aspect Blueprint"}]},
            {"name":"Night Aspect","drops":[{"location":"Uranus/Titania (Assassination), Rotation B Equinox Night Aspect Blueprint"}]},
            {"name":"Forma"}
          ]}
        ]"#;
        let items = parse_warframes(json.as_bytes()).unwrap();
        let names: Vec<&str> = items[0].components.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Day Aspect Neuroptics"));
        assert!(names.contains(&"Night Aspect Systems"));
        assert!(!names.iter().any(|n| *n == "Day Aspect" || *n == "Night Aspect"));
        let hint = items[0]
            .components
            .iter()
            .find(|c| c.name == "Day Aspect Chassis")
            .unwrap();
        assert_eq!(
            hint.drops[0].location.as_deref(),
            Some("Uranus/Titania (Assassination)")
        );
    }

    #[test]
    fn refs_only_components_normalize_names() {
        let json = r#"[
          {"name":"Ash","uniqueName":"/Lotus/Powersuits/Ninja/Ninja","category":"Warframes","type":"Warframe","components":[
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AshBlueprint","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AshChassisComponent","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AshHelmetComponent","itemCount":1},
            {"uniqueName":"/Lotus/Types/Items/MiscItems/OrokinCell","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AshSystemsComponent","itemCount":1}
          ]},
          {"name":"Equinox","uniqueName":"/Lotus/Powersuits/YinYang/YinYang","category":"Warframes","type":"Warframe","components":[
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AnimaAnimusBlueprint","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AnimusComponent","itemCount":1},
            {"uniqueName":"/Lotus/Types/Items/MiscItems/Forma","itemCount":1},
            {"uniqueName":"/Lotus/Types/Recipes/WarframeRecipes/AnimaComponent","itemCount":1}
          ]}
        ]"#;
        let items = parse_warframes(json.as_bytes()).unwrap();
        assert_eq!(items.len(), 2);
        let ash = &items[0];
        let ash_names: Vec<&str> = ash.components.iter().map(|c| c.name.as_str()).collect();
        assert!(ash_names.contains(&"Blueprint"));
        assert!(ash_names.contains(&"Neuroptics"));
        assert!(ash_names.contains(&"Chassis"));
        assert!(ash_names.contains(&"Systems"));
        assert!(part_meta("Neuroptics").is_some());

        let eq = &items[1];
        let eq_names: Vec<&str> = eq.components.iter().map(|c| c.name.as_str()).collect();
        assert!(eq_names.contains(&"Blueprint"));
        assert!(eq_names.contains(&"Day Aspect Neuroptics"));
        assert!(eq_names.contains(&"Night Aspect Systems"));
        assert!(!eq_names.iter().any(|n| *n == "Day Aspect" || *n == "Night Aspect"));
    }
}
