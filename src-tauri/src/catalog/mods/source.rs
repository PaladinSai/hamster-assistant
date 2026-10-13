//! Mod 来源映射（drop-data all.json + drops 启发式）。

use std::collections::HashMap;

use serde_json::Value;

use super::types::*;

/// 越小越优先（堕落 Derelict Vault 最高，其次噩梦）。
pub(crate) fn source_rank(kind: i32) -> i32 {
    match kind {
        SRC_KEY => 0,
        SRC_NIGHTMARE => 1,
        SRC_CETUS | SRC_SOLARIS | SRC_DEIMOS | SRC_ZARIMAN | SRC_HEX => 2,
        SRC_SYNDICATE => 3,
        SRC_SORTIE => 4,
        SRC_TRANSIENT => 5,
        SRC_MISSION => 6,
        SRC_ENEMY => 7,
        _ => 99,
    }
}

pub(crate) fn prefer_source(current: i32, candidate: i32) -> i32 {
    if source_rank(candidate) < source_rank(current) {
        candidate
    } else {
        current
    }
}
pub(crate) fn offer_source(map: &mut HashMap<String, i32>, name: &str, kind: i32) {
    let name = name.trim();
    if name.is_empty() {
        return;
    }
    // 跳过明显非 Mod 奖励
    if name.contains(" Relic")
        || name.contains("Endo")
        || name.contains("Credits")
        || name.contains("Oxium")
        || name.contains("Kuva")
        || name.ends_with(" Cache")
        || name.contains("X ")
        || name.contains('×')
    {
        return;
    }
    let key = name_key(name);
    match map.get(&key) {
        Some(&old) if source_rank(old) <= source_rank(kind) => {}
        _ => {
            map.insert(key, kind);
        }
    }
}

pub(crate) fn walk_reward_items(map: &mut HashMap<String, i32>, rewards: &Value, kind: i32) {
    match rewards {
        Value::Object(obj) => {
            for (_k, v) in obj {
                walk_reward_items(map, v, kind);
            }
        }
        Value::Array(arr) => {
            for item in arr {
                if let Some(name) = item.get("itemName").and_then(|x| x.as_str()) {
                    offer_source(map, name, kind);
                }
                if let Some(name) = item.get("modName").and_then(|x| x.as_str()) {
                    offer_source(map, name, kind);
                }
                if let Some(name) = item.get("item").and_then(|x| x.as_str()) {
                    offer_source(map, name, kind);
                }
                if let Some(inner) = item.get("rewards") {
                    walk_reward_items(map, inner, kind);
                }
            }
        }
        _ => {}
    }
}

pub(crate) fn ingest_syndicate_reward_list(map: &mut HashMap<String, i32>, rewards: &Value) {
    match rewards {
        Value::Array(arr) => {
            for item in arr {
                let name = item
                    .get("item")
                    .or_else(|| item.get("itemName"))
                    .or_else(|| item.get("name"))
                    .and_then(|x| x.as_str());
                if let Some(name) = name {
                    offer_source(map, name, SRC_SYNDICATE);
                }
            }
        }
        Value::Object(obj) => {
            for (_k, v) in obj {
                ingest_syndicate_reward_list(map, v);
            }
        }
        _ => {}
    }
}

/// Mods.json `drops[].location` → 来源启发式。
pub fn source_from_drop_location(location: &str) -> Option<i32> {
    let loc = location.to_ascii_lowercase();

    // 德莫斯废船金库（堕落 Mod / 龙钥）
    if loc.contains("derelict vault") || loc.contains("orokin vault") {
        return Some(SRC_KEY);
    }
    if loc.contains("nightmare") {
        return Some(SRC_NIGHTMARE);
    }

    // 开放世界赏金（先于「敌人掉落在平原」）
    if loc.contains("cetus bounty") || loc.contains("plains of eidolon") {
        return Some(SRC_CETUS);
    }
    if loc.contains("fortuna bounty")
        || loc.contains("orb vallis bounty")
        || (loc.contains("orb vallis") && loc.contains("bounty"))
    {
        return Some(SRC_SOLARIS);
    }
    if loc.contains("cambion drift")
        || loc.contains("isolation vault")
        || (loc.contains("deimos/") && loc.contains("bounty"))
    {
        return Some(SRC_DEIMOS);
    }
    if loc.contains("zariman") {
        return Some(SRC_ZARIMAN);
    }
    if loc.contains("hex bounty")
        || loc.contains("höllvania")
        || loc.contains("hollvania")
        || loc.contains("1999")
    {
        return Some(SRC_HEX);
    }

    // 集团 / 声望商人
    const SYNDICATE_MARKERS: &[&str] = &[
        "steel meridian",
        "red veil",
        "new loka",
        "the perrin sequence",
        "arbiters of hexis",
        "cephalon suda",
        "cephalon simaris",
        "ostron",
        "the quills",
        "solaris united",
        "vox solaris",
        "ventkids",
        "entrati",
        "necra",
        "the holdfasts",
        "operational supply",
        "kahl's garrison",
        "conclave,",
        " syndicate",
    ];
    if SYNDICATE_MARKERS.iter().any(|m| loc.contains(m)) {
        return Some(SRC_SYNDICATE);
    }

    if loc.contains("sortie") {
        return Some(SRC_SORTIE);
    }

    None
}

pub(crate) fn source_from_mod_drops(item: &Value, current: i32) -> i32 {
    let Some(drops) = item.get("drops").and_then(|v| v.as_array()) else {
        return current;
    };
    let mut best = current;
    for d in drops {
        if let Some(loc) = d.get("location").and_then(|x| x.as_str()) {
            if let Some(kind) = source_from_drop_location(loc) {
                best = prefer_source(best, kind);
            }
        }
    }
    best
}

/// 从 drop-data `all.json` 建 name_en(lower) → source_kind。
pub fn build_source_map(drops_json: &[u8]) -> Result<HashMap<String, i32>, String> {
    let root: Value =
        serde_json::from_slice(drops_json).map_err(|e| format!("drop-data JSON 解析失败: {e}"))?;
    let mut map = HashMap::new();

    // 敌人掉落（优先级低，先写入）
    if let Some(arr) = root.get("modLocations").and_then(|v| v.as_array()) {
        for entry in arr {
            if let Some(name) = entry.get("modName").and_then(|x| x.as_str()) {
                offer_source(&mut map, name, SRC_ENEMY);
            }
        }
    }
    if let Some(arr) = root.get("enemyModTables").and_then(|v| v.as_array()) {
        for entry in arr {
            if let Some(mods) = entry.get("mods").and_then(|v| v.as_array()) {
                for m in mods {
                    if let Some(name) = m.get("modName").and_then(|x| x.as_str()) {
                        offer_source(&mut map, name, SRC_ENEMY);
                    }
                }
            }
        }
    }

    // 星图
    if let Some(planets) = root.get("missionRewards").and_then(|v| v.as_object()) {
        for (_planet, nodes) in planets {
            if let Some(nodes) = nodes.as_object() {
                for (_node, payload) in nodes {
                    if let Some(rewards) = payload.get("rewards") {
                        walk_reward_items(&mut map, rewards, SRC_MISSION);
                    }
                }
            }
        }
    }

    let bounty_keys: &[(&str, i32)] = &[
        ("cetusBountyRewards", SRC_CETUS),
        ("solarisBountyRewards", SRC_SOLARIS),
        ("deimosRewards", SRC_DEIMOS),
        ("entratiLabRewards", SRC_DEIMOS),
        ("zarimanRewards", SRC_ZARIMAN),
        ("hexRewards", SRC_HEX),
    ];
    for &(key, kind) in bounty_keys {
        if let Some(arr) = root.get(key).and_then(|v| v.as_array()) {
            for entry in arr {
                if let Some(rewards) = entry.get("rewards") {
                    walk_reward_items(&mut map, rewards, kind);
                }
            }
        }
    }

    if let Some(arr) = root.get("sortieRewards").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(name) = item.get("itemName").and_then(|x| x.as_str()) {
                offer_source(&mut map, name, SRC_SORTIE);
            }
        }
    }

    if let Some(arr) = root.get("transientRewards").and_then(|v| v.as_array()) {
        for entry in arr {
            let objective = entry
                .get("objectiveName")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            // 堕落 / 噩梦 从 transient 拆出；其余仍为限时/活动
            let obj_lc = objective.to_ascii_lowercase();
            let kind = if obj_lc.contains("derelict vault") {
                SRC_KEY
            } else if obj_lc.contains("nightmare") {
                SRC_NIGHTMARE
            } else {
                SRC_TRANSIENT
            };
            if let Some(rewards) = entry.get("rewards") {
                walk_reward_items(&mut map, rewards, kind);
            }
            if let Some(name) = entry.get("itemName").and_then(|x| x.as_str()) {
                offer_source(&mut map, name, kind);
            }
        }
    }

    // syndicates：all.json 为 dict（派系名 → 奖励数组）；兼容旧 array 形态
    if let Some(synd) = root.get("syndicates") {
        match synd {
            Value::Object(obj) => {
                for (_faction, rewards) in obj {
                    ingest_syndicate_reward_list(&mut map, rewards);
                }
            }
            Value::Array(arr) => {
                for entry in arr {
                    if let Some(rewards) = entry.get("rewards") {
                        ingest_syndicate_reward_list(&mut map, rewards);
                    }
                    if let Some(items) = entry.get("items") {
                        ingest_syndicate_reward_list(&mut map, items);
                    }
                }
            }
            _ => {}
        }
    }

    // 不再把整段 keyRewards 标成龙钥：那是各类钥匙任务，不是 Derelict Vault。

    Ok(map)
}
