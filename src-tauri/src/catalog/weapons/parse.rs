//! 武器 / 自制部件 JSON 轻量 parse（不写库）。

use std::collections::HashMap;

use serde_json::Value;

use crate::catalog::wfcd::{
    MODULAR_KIND_AMP, MODULAR_KIND_KITGUN, MODULAR_KIND_ZAW,
};

use super::rules::{
    is_excluded_weapon, normalize_weapon_component, WfcdWeapon,
};

pub fn parse_weapons(bytes: &[u8], slot: i32) -> Result<Vec<(WfcdWeapon, i32)>, String> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("武器 JSON 解析失败: {e}"))?;
    let Value::Array(arr) = value else {
        return Err("武器 JSON 根节点不是数组".to_string());
    };
    let mut out = Vec::new();
    for item in arr {
        let mut w: WfcdWeapon = match serde_json::from_value(item) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[仓鼠] 跳过单条武器: {e}");
                continue;
            }
        };
        if w.name.trim().is_empty() || w.unique_name.trim().is_empty() {
            continue;
        }
        for c in &mut w.components {
            normalize_weapon_component(c);
        }
        if is_excluded_weapon(&w) {
            continue;
        }
        out.push((w, slot));
    }
    Ok(out)
}

pub(crate) struct ModularPart {
    pub(crate) name_en: String,
    pub(crate) unique_name: String,
    pub(crate) modular_kind: &'static str,
}

pub(crate) fn parse_modular_parts(bytes: &[u8]) -> Result<Vec<ModularPart>, String> {
    let arr: Vec<Value> =
        serde_json::from_slice(bytes).map_err(|e| format!("解析自制部件 JSON 失败: {e}"))?;
    let mut out = Vec::new();
    let mut seen = HashMap::<String, ()>::new();
    for v in arr {
        let name = v
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let unique_name = v
            .get("uniqueName")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() || unique_name.is_empty() {
            continue;
        }
        if unique_name.contains("PvP") {
            continue;
        }
        let item_type = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let tags: Vec<String> = v
            .get("tags")
            .and_then(|t| t.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let kind = if is_kitgun_chamber(&unique_name, item_type) {
            Some(MODULAR_KIND_KITGUN)
        } else if is_zaw_strike(&unique_name, item_type, &tags) {
            Some(MODULAR_KIND_ZAW)
        } else if is_amp_prism(&name, &unique_name) {
            Some(MODULAR_KIND_AMP)
        } else {
            None
        };
        let Some(modular_kind) = kind else {
            continue;
        };
        if seen.contains_key(&unique_name) {
            continue;
        }
        seen.insert(unique_name.clone(), ());
        out.push(ModularPart {
            name_en: name,
            unique_name,
            modular_kind,
        });
    }
    Ok(out)
}

pub(crate) fn is_kitgun_chamber(unique_name: &str, item_type: &str) -> bool {
    if unique_name.contains("SUModularSecondary") && unique_name.contains("/Barrel/") {
        return true;
    }
    if unique_name.contains("InfKitGun") && unique_name.contains("/Barrels/") {
        return true;
    }
    item_type.eq_ignore_ascii_case("Kitgun Component") && unique_name.contains("Barrel")
}

pub(crate) fn is_zaw_strike(unique_name: &str, item_type: &str, tags: &[String]) -> bool {
    let tip_path = unique_name.contains("/Tip/") || unique_name.contains("/Tips/");
    if !tip_path {
        return false;
    }
    if item_type.eq_ignore_ascii_case("Zaw Component") {
        return true;
    }
    tags.iter().any(|t| t.eq_ignore_ascii_case("Zaw"))
}

pub(crate) fn is_amp_prism(name: &str, unique_name: &str) -> bool {
    name.ends_with(" Prism")
        && (unique_name.contains("OperatorAmplifiers") || unique_name.contains("Amp"))
}

/// 国际服简中回退（userdict / WFCD 缺译时）。
pub(crate) fn modular_zh_fallback(name_en: &str) -> Option<&'static str> {
    Some(match name_en {
        "Catchmoon" => "捕月",
        "Gaze" => "凝目",
        "Rattleguts" => "响胆",
        "Tombfinger" => "墓指",
        "Sporelacer" => "孢射",
        "Vermisplicer" => "虫置",
        "Balla" => "宝拉",
        "Cyath" => "西亚什",
        "Dehtat" => "德塔特",
        "Dokrahm" => "多克拉姆",
        "Kronsh" => "客隆什",
        "Mewan" => "密丸",
        "Ooltha" => "乌尔萨",
        "Rabvee" => "拉比威",
        "Sepfahn" => "瑟普梵",
        "Plague Keewar" => "瘟疫奇沃",
        "Plague Kripath" => "瘟疫克里帕丝",
        "Cantic Prism" => "堪提克棱镜",
        "Granmu Prism" => "格兰姆棱镜",
        "Klamora Prism" => "克拉莫阿棱镜",
        "Lega Prism" => "勒咖棱镜",
        "Mote Prism" => "微尘棱镜",
        "Rahn Prism" => "染棱镜",
        "Raplak Prism" => "拉普拉克棱镜",
        "Shwaak Prism" => "施瓦克棱镜",
        _ => return None,
    })
}
