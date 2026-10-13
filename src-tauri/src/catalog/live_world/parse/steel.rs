//! 钢铁侵袭日窗合成（公开 API 无节点时）。

use chrono::{TimeZone, Utc};

use crate::catalog::live_world::types::{SteelIncursionApi, SteelPathApi};

/// 公开 API 无侵袭节点；按 UTC 日切合成今日窗口。
pub(crate) fn synthesize_steel_incursion() -> SteelPathApi {
    let now = Utc::now();
    let day = now.date_naive();
    let start = day.and_hms_opt(0, 0, 0).unwrap();
    let end = day.and_hms_opt(23, 59, 59).unwrap();
    let activation = Utc.from_utc_datetime(&start).to_rfc3339();
    let expiry = Utc.from_utc_datetime(&end).to_rfc3339();
    SteelPathApi {
        incursions: Some(SteelIncursionApi {
            activation: Some(activation),
            expiry: Some(expiry),
            today: Vec::new(),
            tomorrow: Vec::new(),
        }),
    }
}
