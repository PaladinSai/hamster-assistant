//! 突击 / 午夜电波 parse。

use super::de::*;
use crate::catalog::live_world::localize;
use crate::catalog::live_world::types::{
    NightwaveApi, NightwaveChallengeApi, SortieApi, SortieVariantApi,
};

pub(super) fn parse_sortie(raw: Option<&DeSortie>) -> Result<SortieApi, String> {
    let Some(raw) = raw else {
        return Ok(SortieApi {
            activation: None,
            expiry: None,
            boss: None,
            faction: None,
            variants: Vec::new(),
        });
    };
    let boss_code = raw.boss.as_deref().unwrap_or("");
    let (boss_zh, faction) = localize::sortie_boss(boss_code);
    let variants = raw
        .variants
        .iter()
        .map(|v| {
            let mt = v.mission_type.as_deref().unwrap_or("");
            let md = v.modifier_type.as_deref().unwrap_or("");
            let node = v.node.as_deref().unwrap_or("");
            SortieVariantApi {
                mission_type: Some(localize::mission_type_en(mt)),
                modifier: Some(localize::sortie_modifier(md)),
                node: Some(localize::node_name(node)),
            }
        })
        .collect();
    Ok(SortieApi {
        activation: raw.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: raw.expiry.as_ref().and_then(de_date_to_rfc3339),
        boss: Some(boss_zh),
        faction: Some(faction),
        variants,
    })
}

pub(super) fn parse_nightwave(
    info: Option<&DeSeasonInfo>,
    challenge_i18n: Option<&crate::catalog::live_world::challenge_i18n::ChallengeLocalizer>,
) -> NightwaveApi {
    let mut active_challenges = Vec::new();
    let Some(info) = info else {
        return NightwaveApi {
            activation: None,
            expiry: None,
            active_challenges,
        };
    };
    for c in &info.active_challenges {
        let path = c.challenge.as_deref().unwrap_or("");
        let (title, desc) = localize::nightwave_challenge(path, challenge_i18n);
        let reputation = challenge_i18n.and_then(|loc| loc.challenge_standing(path));
        let id = c
            .id
            .as_ref()
            .map(|o| o.oid.clone())
            .unwrap_or_else(|| path.to_string());
        active_challenges.push(NightwaveChallengeApi {
            id,
            activation: c.activation.as_ref().and_then(de_date_to_rfc3339),
            expiry: c.expiry.as_ref().and_then(de_date_to_rfc3339),
            is_daily: c.daily.unwrap_or(false),
            is_elite: localize::is_elite_challenge(path),
            title: Some(title),
            desc: Some(desc),
            reputation,
        });
    }
    NightwaveApi {
        activation: info.activation.as_ref().and_then(de_date_to_rfc3339),
        expiry: info.expiry.as_ref().and_then(de_date_to_rfc3339),
        active_challenges,
    }
}

