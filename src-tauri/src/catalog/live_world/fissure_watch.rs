//! 裂隙关注：每条目标是「种类 + 纪元 + 任务类型」。命中某一条才提醒，每条裂隙单独弹窗。

use std::collections::HashSet;
use std::path::Path;

use crate::db::{self, open_user};

use super::summary::load_fissure_brief;
use super::types::{
    FissureAlertDto, FissureBriefDto, FissureWatchPrefs, FissureWatchTarget, FissureWatchView,
    LiveFissureDto, SETTING_FISSURE_WATCH,
};

/// 关注里的裂隙种类，与板上 `kind` 一致。
pub const FISSURE_WATCH_KINDS: &[&str] = &["normal", "steel", "storm"];

/// 关注里的纪元。九重天同样带纪元；安魂、全能不在关注里。
pub const FISSURE_WATCH_TIERS: &[&str] = &["古纪", "前纪", "中纪", "后纪"];

/// 裂隙会刷出的任务类型（中文，与板上 missionType 一致）。不含开放世界 / 竞速 / 竞技场。
pub const FISSURE_MISSION_TYPES: &[&str] = &[
    "歼灭",
    "捕获",
    "救援",
    "破坏",
    "间谍",
    "防御",
    "移动防御",
    "拦截",
    "挖掘",
    "生存",
    "刺杀",
    "中断",
    "清巢",
    "INFESTED 资源回收",
    "叛逃",
    "劫持",
    "元素转换",
    "虚空洪流",
    "虚空覆涌",
    "虚空决战",
    "爆发",
    "奥影",
    "前哨战",
];

pub fn get_fissure_watch(data_dir: &Path) -> Result<FissureWatchView, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    Ok(FissureWatchView {
        prefs: read_watch(&user)?,
        mission_types: FISSURE_MISSION_TYPES.iter().map(|s| (*s).to_string()).collect(),
    })
}

pub fn set_fissure_watch(
    data_dir: &Path,
    prefs: FissureWatchPrefs,
) -> Result<FissureWatchPrefs, String> {
    db::init_databases(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let clean = sanitize(prefs);
    write_watch(&user, &clean)?;
    Ok(clean)
}

fn read_watch(user: &rusqlite::Connection) -> Result<FissureWatchPrefs, String> {
    let Some(json) = db::get_setting(user, SETTING_FISSURE_WATCH)? else {
        return Ok(FissureWatchPrefs::default());
    };
    let prefs: FissureWatchPrefs =
        serde_json::from_str(&json).map_err(|e| format!("解析裂隙关注失败: {e}"))?;
    Ok(sanitize(prefs))
}

fn write_watch(user: &rusqlite::Connection, prefs: &FissureWatchPrefs) -> Result<(), String> {
    let json = serde_json::to_string(prefs).map_err(|e| e.to_string())?;
    db::set_setting(user, SETTING_FISSURE_WATCH, &json)
}

fn sanitize(mut prefs: FissureWatchPrefs) -> FissureWatchPrefs {
    let mut seen = HashSet::new();
    let mut targets = Vec::new();
    for target in prefs.targets {
        let kind = target.kind.trim();
        let tier = target.tier.trim();
        let mission = target.mission_type.trim();
        if !FISSURE_WATCH_KINDS.contains(&kind)
            || !FISSURE_WATCH_TIERS.contains(&tier)
            || !FISSURE_MISSION_TYPES.contains(&mission)
        {
            continue;
        }
        if !seen.insert(format!("{kind}|{tier}|{mission}")) {
            continue;
        }
        targets.push(FissureWatchTarget {
            kind: kind.to_string(),
            tier: tier.to_string(),
            mission_type: mission.to_string(),
        });
    }
    prefs.targets = targets;
    prefs.last_notified = tidy(prefs.last_notified);
    prefs
}

fn tidy(mut items: Vec<String>) -> Vec<String> {
    for item in &mut items {
        *item = item.trim().to_string();
    }
    items.retain(|s| !s.is_empty());
    items.sort();
    items.dedup();
    items
}

pub fn match_and_record_fissure_notifications(
    data_dir: &Path,
) -> Result<Vec<FissureAlertDto>, String> {
    db::init_databases(data_dir)?;
    let brief = load_fissure_brief(data_dir)?;
    let user = open_user(&db::user_path(data_dir))?;
    let mut prefs = read_watch(&user)?;
    let alerts = collect_new_alerts(&brief, &mut prefs);
    if !alerts.is_empty() {
        write_watch(&user, &prefs)?;
    }
    Ok(alerts)
}

pub(crate) fn collect_new_alerts(
    brief: &FissureBriefDto,
    prefs: &mut FissureWatchPrefs,
) -> Vec<FissureAlertDto> {
    if prefs.targets.is_empty() {
        return Vec::new();
    }
    let mut alerts = Vec::new();
    for fissure in brief
        .normal
        .iter()
        .chain(brief.steel.iter())
        .chain(brief.storm.iter())
    {
        if !fissure_matches(prefs, fissure) {
            continue;
        }
        let fp = fingerprint(fissure);
        if prefs.last_notified.iter().any(|x| x == &fp) {
            continue;
        }
        prefs.last_notified.push(fp);
        alerts.push(FissureAlertDto {
            body: fissure_line(fissure),
            expiry: fissure.expiry.clone(),
        });
    }
    if !alerts.is_empty() && prefs.last_notified.len() > 128 {
        let drain = prefs.last_notified.len() - 128;
        prefs.last_notified.drain(0..drain);
    }
    alerts
}

fn fissure_matches(prefs: &FissureWatchPrefs, fissure: &LiveFissureDto) -> bool {
    prefs.targets.iter().any(|target| {
        target.kind == fissure.kind
            && target.tier == fissure.tier_zh
            && target.mission_type == fissure.mission_type
    })
}

fn fingerprint(fissure: &LiveFissureDto) -> String {
    format!(
        "{}|{}|{}|{}",
        fissure.kind,
        fissure.node,
        fissure.tier,
        fissure.expiry.as_deref().unwrap_or("")
    )
}

fn fissure_line(fissure: &LiveFissureDto) -> String {
    let kind = match fissure.kind.as_str() {
        "steel" => "钢铁",
        "storm" => "九重天",
        _ => "普通",
    };
    let tier = if fissure.tier_zh.is_empty() {
        fissure.tier.as_str()
    } else {
        fissure.tier_zh.as_str()
    };
    let mut line = format!("{} · {} · {}", fissure.mission_type, kind, tier);
    if !fissure.node.is_empty() {
        line.push_str(" · ");
        line.push_str(&fissure.node);
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fissure(kind: &str, mission: &str, node: &str, tier: &str, expiry: &str) -> LiveFissureDto {
        LiveFissureDto {
            kind: kind.into(),
            tier: tier.into(),
            tier_zh: tier.into(),
            mission_type: mission.into(),
            node: node.into(),
            activation: None,
            expiry: Some(expiry.into()),
        }
    }

    fn brief_with(items: Vec<LiveFissureDto>) -> FissureBriefDto {
        let mut brief = FissureBriefDto::default();
        for item in items {
            match item.kind.as_str() {
                "steel" => brief.steel.push(item),
                "storm" => brief.storm.push(item),
                _ => brief.normal.push(item),
            }
        }
        brief
    }

    fn target(kind: &str, tier: &str, mission: &str) -> FissureWatchTarget {
        FissureWatchTarget {
            kind: kind.into(),
            tier: tier.into(),
            mission_type: mission.into(),
        }
    }

    #[test]
    fn only_exact_combo_alerts_and_each_fissure_keeps_its_expiry() {
        let brief = brief_with(vec![
            fissure("steel", "歼灭", "节点甲", "古纪", "2026-11-08T00:00:00Z"),
            fissure("steel", "歼灭", "节点乙", "前纪", "2026-11-08T01:00:00Z"),
            fissure("normal", "歼灭", "节点丙", "古纪", "2026-11-08T02:00:00Z"),
            fissure("storm", "生存", "节点丁", "后纪", "2026-11-08T03:00:00Z"),
            fissure("storm", "生存", "节点戊", "后纪", "2026-11-08T04:00:00Z"),
        ]);
        let mut prefs = FissureWatchPrefs {
            targets: vec![
                target("steel", "古纪", "歼灭"),
                target("storm", "后纪", "生存"),
            ],
            ..Default::default()
        };
        let msgs = collect_new_alerts(&brief, &mut prefs);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].body, "歼灭 · 钢铁 · 古纪 · 节点甲");
        assert_eq!(msgs[0].expiry.as_deref(), Some("2026-11-08T00:00:00Z"));
        assert_eq!(msgs[1].body, "生存 · 九重天 · 后纪 · 节点丁");
        assert_eq!(msgs[1].expiry.as_deref(), Some("2026-11-08T03:00:00Z"));
        assert_eq!(msgs[2].body, "生存 · 九重天 · 后纪 · 节点戊");
        assert_eq!(msgs[2].expiry.as_deref(), Some("2026-11-08T04:00:00Z"));
        assert!(collect_new_alerts(&brief, &mut prefs).is_empty());
    }

    #[test]
    fn new_fissure_of_same_combo_alerts_again() {
        let first = brief_with(vec![fissure(
            "steel",
            "歼灭",
            "节点甲",
            "古纪",
            "2026-11-08T00:00:00Z",
        )]);
        let mut prefs = FissureWatchPrefs {
            targets: vec![target("steel", "古纪", "歼灭")],
            ..Default::default()
        };
        assert_eq!(collect_new_alerts(&first, &mut prefs).len(), 1);
        let second = brief_with(vec![
            fissure("steel", "歼灭", "节点甲", "古纪", "2026-11-08T00:00:00Z"),
            fissure("steel", "歼灭", "节点乙", "古纪", "2026-11-08T03:00:00Z"),
        ]);
        let msgs = collect_new_alerts(&second, &mut prefs);
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].body, "歼灭 · 钢铁 · 古纪 · 节点乙");
        assert_eq!(msgs[0].expiry.as_deref(), Some("2026-11-08T03:00:00Z"));
    }

    #[test]
    fn empty_follow_does_not_alert() {
        let brief = brief_with(vec![fissure(
            "normal",
            "生存",
            "节点",
            "中纪",
            "2026-11-08T00:00:00Z",
        )]);
        let mut prefs = FissureWatchPrefs::default();
        assert!(collect_new_alerts(&brief, &mut prefs).is_empty());
    }
}
