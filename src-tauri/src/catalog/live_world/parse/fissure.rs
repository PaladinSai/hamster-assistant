//! 虚空裂隙 / 九重天风暴 parse。

use super::de::*;
use crate::catalog::live_world::localize;
use crate::catalog::live_world::types::{
    FissureApi, FISSURE_KIND_NORMAL, FISSURE_KIND_STEEL, FISSURE_KIND_STORM,
};

pub(super) fn parse_fissures(state: &DeWorldState) -> Vec<FissureApi> {
    let mut out = Vec::with_capacity(state.active_missions.len() + state.void_storms.len());
    for m in &state.active_missions {
        let tier = m.modifier.as_deref().unwrap_or("").to_string();
        let node_code = m.node.as_deref().unwrap_or("");
        let mt_raw = m.mission_type.as_deref().unwrap_or("");
        out.push(FissureApi {
            kind: if m.hard {
                FISSURE_KIND_STEEL.into()
            } else {
                FISSURE_KIND_NORMAL.into()
            },
            tier_zh: void_tier_zh(&tier),
            tier,
            mission_type: localize::mission_type(mt_raw),
            node: localize::node_name(node_code),
            node_code: node_code.to_string(),
            activation: m.activation.as_ref().and_then(de_date_to_rfc3339),
            expiry: m.expiry.as_ref().and_then(de_date_to_rfc3339),
        });
    }
    for s in &state.void_storms {
        let tier = s.active_mission_tier.as_deref().unwrap_or("").to_string();
        let node_code = s.node.as_deref().unwrap_or("");
        // worldState.VoidStorms 不含 MissionType；fetch 阶段用 ExportRegions 补。
        out.push(FissureApi {
            kind: FISSURE_KIND_STORM.into(),
            tier_zh: void_tier_zh(&tier),
            tier,
            mission_type: String::new(),
            node: localize::node_name(node_code),
            node_code: node_code.to_string(),
            activation: s.activation.as_ref().and_then(de_date_to_rfc3339),
            expiry: s.expiry.as_ref().and_then(de_date_to_rfc3339),
        });
    }
    out.sort_by(|a, b| {
        tier_sort_key(&a.tier)
            .cmp(&tier_sort_key(&b.tier))
            .then(a.expiry.cmp(&b.expiry))
            .then(a.node.cmp(&b.node))
    });
    out
}

pub(super) fn void_tier_zh(code: &str) -> String {
    match code {
        "VoidT1" => "古纪".into(),
        "VoidT2" => "前纪".into(),
        "VoidT3" => "中纪".into(),
        "VoidT4" => "后纪".into(),
        "VoidT5" => "安魂".into(),
        "VoidT6" => "全能".into(),
        other if other.is_empty() => String::new(),
        other => other.to_string(),
    }
}

pub(super) fn tier_sort_key(code: &str) -> u8 {
    match code {
        "VoidT1" => 1,
        "VoidT2" => 2,
        "VoidT3" => 3,
        "VoidT4" => 4,
        "VoidT5" => 5,
        "VoidT6" => 6,
        _ => 99,
    }
}

