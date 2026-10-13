//! 官方 worldState → LiveWorldFetch（按域拆分 parse）。
//!
//! 仲裁 / 商店读路径见同级 `arbitration` / `shop`，不塞进本树。

mod bounty;
mod de;
mod fissure;
mod sortie;
mod steel;
mod weekly;

#[cfg(test)]
mod tests;

use crate::catalog::live_world::types::LiveWorldFetch;
use crate::catalog::resurgence::WORLDSTATE_URL;

use self::bounty::{parse_bounties, DeBountyCycle};
use self::de::DeWorldState;
use self::fissure::parse_fissures;
use self::sortie::{parse_nightwave, parse_sortie};
use self::weekly::parse_weekly;

pub(crate) use self::steel::synthesize_steel_incursion;

pub(crate) fn parse_worldstate(body: &[u8]) -> Result<LiveWorldFetch, String> {
    parse_worldstate_with_cycle(body, None, None, None)
}

pub(crate) fn parse_worldstate_with_cycle(
    body: &[u8],
    cycle_body: Option<&[u8]>,
    challenge_i18n: Option<&super::challenge_i18n::ChallengeLocalizer>,
    conquest_lang: Option<&super::conquest_i18n::ConquestLang>,
) -> Result<LiveWorldFetch, String> {
    let state: DeWorldState = serde_json::from_slice(body)
        .map_err(|e| format!("世界状态 JSON 解析失败: {e}"))?;
    let sortie = parse_sortie(state.sorties.first())?;
    let nightwave = parse_nightwave(state.season_info.as_ref(), challenge_i18n);
    let steel = self::steel::synthesize_steel_incursion();
    let weekly = parse_weekly(&state, challenge_i18n, conquest_lang);
    let cycle = cycle_body.and_then(|b| match serde_json::from_slice::<DeBountyCycle>(b) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("[仓鼠] bounty-cycle JSON 解析失败（降级）: {e}");
            None
        }
    });
    let bounties = parse_bounties(&state, cycle.as_ref(), challenge_i18n);
    let fissures = parse_fissures(&state);
    Ok(LiveWorldFetch {
        sortie,
        nightwave,
        steel,
        weekly,
        bounties,
        fissures,
        source_url: WORLDSTATE_URL.into(),
    })
}

