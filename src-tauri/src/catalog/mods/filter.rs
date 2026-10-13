//! Mod 过滤与品类映射。

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
    if hay.contains("flawed") {
        return true;
    }
    // 新手/中级/未融合同名变体；Expert 训练变体也跳过，但保留 Primed（也走 /Expert/）
    if hay.contains("/beginner/") || hay.contains("/intermediate/") || hay.contains("unfused") {
        return true;
    }
    if hay.contains("/expert/") && !is_primed_mod(name, unique_name) {
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
    if should_skip_mod(&m.name, &m.unique_name, item_type) {
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
