//! Mod 过滤与品类映射。

use std::collections::{HashMap, HashSet};

use super::types::*;

/// Baro 的 Primed（P 版）Mod：名称 `Primed …`，或 uniqueName 含 primed。
pub fn is_primed_mod(name: &str, unique_name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    if n.starts_with("primed ") {
        return true;
    }
    unique_name.to_ascii_lowercase().contains("primed")
}

pub fn should_skip_mod(name: &str, unique_name: &str, item_type: &str) -> bool {
    if item_type.contains("Riven") {
        return true;
    }
    if name.eq_ignore_ascii_case("Focus Way") || item_type == "Focus Way" {
        return true;
    }
    let hay = format!("{name} {unique_name}").to_ascii_lowercase();
    if hay.contains("flawed") || hay.contains("unfused") {
        return true;
    }
    false
}

/// `/Beginner/`、`/Intermediate/`、`/Expert/` 是融合等级变体。
/// 同名卡已有目录外副本时才丢；镀层近战和只剩 Beginner 的磁浮、随机应变留下。
pub(crate) fn is_rank_variant_path(unique_name: &str) -> bool {
    let u = unique_name.to_ascii_lowercase();
    u.contains("/beginner/") || u.contains("/intermediate/") || u.contains("/expert/")
}

fn path_has(unique_name: &str, needle: &str) -> bool {
    unique_name.to_ascii_lowercase().contains(needle)
}

fn has_text(value: Option<&str>) -> bool {
    value.map(|s| !s.trim().is_empty()).unwrap_or(false)
}

/// 套装效果定义、竞拍、Tau 强化副本、图鉴外条目、无资料的隐藏旧卡。
fn should_skip_non_card(m: &WfcdMod) -> bool {
    let item_type = m.item_type.as_deref().unwrap_or("");
    if item_type.eq_ignore_ascii_case("Mod Set Mod")
        || m.unique_name.to_ascii_lowercase().ends_with("setmod")
    {
        return true;
    }
    if path_has(&m.unique_name, "/pvpmods/") || path_has(&m.unique_name, "/upgrades/tau/") {
        return true;
    }
    if m.exclude_from_codex {
        return true;
    }
    if m.codex_secret && !has_text(m.wikia_url.as_deref()) && !has_text(m.release_date.as_deref())
    {
        return true;
    }
    false
}

fn is_zero_date(s: &str) -> bool {
    s.trim() == "0000-00-00"
}

/// 未放出 / 废案：releaseDate 或 introduced 为占位 TBA / 0000-00-00。
pub fn is_unreleased_mod(
    release_date: Option<&str>,
    introduced_name: Option<&str>,
    introduced_date: Option<&str>,
) -> bool {
    if release_date.map(is_zero_date).unwrap_or(false) {
        return true;
    }
    if introduced_date.map(is_zero_date).unwrap_or(false) {
        return true;
    }
    if introduced_name
        .map(|n| n.trim().eq_ignore_ascii_case("TBA"))
        .unwrap_or(false)
    {
        return true;
    }
    false
}

/// Primed 残缺桩：无发布日且无 wiki（种子里少数内部条目）。
pub fn is_incomplete_primed_stub(
    name: &str,
    unique_name: &str,
    release_date: Option<&str>,
    wikia_url: Option<&str>,
) -> bool {
    if !is_primed_mod(name, unique_name) {
        return false;
    }
    let has_release = release_date.map(|s| !s.trim().is_empty()).unwrap_or(false);
    let has_wiki = wikia_url.map(|s| !s.trim().is_empty()).unwrap_or(false);
    !has_release && !has_wiki
}

pub(crate) fn should_skip_wfcd_mod(m: &WfcdMod) -> bool {
    let item_type = m.item_type.as_deref().unwrap_or("");
    if should_skip_mod(&m.name, &m.unique_name, item_type) || should_skip_non_card(m) {
        return true;
    }
    let intro_name = m.introduced.as_ref().and_then(|i| i.name.as_deref());
    let intro_date = m.introduced.as_ref().and_then(|i| i.date.as_deref());
    if is_unreleased_mod(m.release_date.as_deref(), intro_name, intro_date) {
        return true;
    }
    if is_incomplete_primed_stub(
        &m.name,
        &m.unique_name,
        m.release_date.as_deref(),
        m.wikia_url.as_deref(),
    ) {
        return true;
    }
    false
}

fn dedupe_key(m: &WfcdMod) -> (String, String, String) {
    (
        name_key(&m.name),
        m.compat_name
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
        m.item_type
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase(),
    )
}

/// 越小越该留下。重击误名和 Nemesis 重复导出让位给正式路径。
fn duplicate_penalty(m: &WfcdMod) -> i32 {
    let u = m.unique_name.to_ascii_lowercase();
    let mut penalty = 0;
    if u.contains("onheavykill") {
        penalty += 10;
    }
    if u.contains("/nemesis/") {
        penalty += 5;
    }
    penalty
}

/// 单条过滤之后：同名已有正式路径则丢掉等级变体；同名同类型同兼容只留一条。
pub(crate) fn unique_names_to_keep(mods: &[WfcdMod]) -> HashSet<String> {
    let passed: Vec<&WfcdMod> = mods.iter().filter(|m| !should_skip_wfcd_mod(m)).collect();
    let canonical_names: HashSet<String> = passed
        .iter()
        .filter(|m| !is_rank_variant_path(&m.unique_name))
        .map(|m| name_key(&m.name))
        .collect();
    let after_rank: Vec<&WfcdMod> = passed
        .into_iter()
        .filter(|m| {
            !(is_rank_variant_path(&m.unique_name)
                && canonical_names.contains(&name_key(&m.name)))
        })
        .collect();

    let mut best: HashMap<(String, String, String), &WfcdMod> = HashMap::new();
    for m in after_rank {
        let key = dedupe_key(m);
        match best.get(&key) {
            Some(current) if duplicate_penalty(m) >= duplicate_penalty(current) => {}
            _ => {
                best.insert(key, m);
            }
        }
    }
    best.into_values().map(|m| m.unique_name.clone()).collect()
}

pub fn map_mod_kind(item_type: &str, compat_name: Option<&str>) -> i32 {
    if compat_name.map(|c| c.eq_ignore_ascii_case("AURA")).unwrap_or(false) {
        return MOD_KIND_AURA;
    }
    let t = item_type;
    if t.contains("Stance") || t.contains("Posture") {
        return MOD_KIND_STANCE;
    }
    if t == "Warframe Mod" {
        return MOD_KIND_WARFRAME;
    }
    if t == "Primary Mod" || t == "Shotgun Mod" {
        return MOD_KIND_PRIMARY;
    }
    if t == "Secondary Mod" {
        return MOD_KIND_SECONDARY;
    }
    if t == "Melee Mod" {
        return MOD_KIND_MELEE;
    }
    if t.contains("Companion") {
        return MOD_KIND_COMPANION;
    }
    MOD_KIND_OTHER
}
