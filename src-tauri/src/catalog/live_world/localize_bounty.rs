//! 开放世界声望势力与赏金任务种类本地化。

/// 固定 11 势力（侧栏顺序）。
pub const BOUNTY_SYNDICATES: &[(&str, &str)] = &[
    ("CetusSyndicate", "Ostron"),
    ("QuillsSyndicate", "夜羽"),
    ("SolarisSyndicate", "索拉里斯联盟"),
    ("VentKidsSyndicate", "通风小子"),
    ("VoxSyndicate", "索拉里斯之声"),
    ("EntratiSyndicate", "英择谛"),
    ("NecraloidSyndicate", "殁世械灵"),
    ("ZarimanSyndicate", "坚守者"),
    ("KahlSyndicate", "卡尔驻军"),
    ("EntratiLabSyndicate", "科维兽"),
    ("HexSyndicate", "六人组"),
];

pub fn is_tracked_syndicate(tag: &str) -> bool {
    BOUNTY_SYNDICATES.iter().any(|(t, _)| *t == tag)
}

pub fn syndicate_display_name(tag: &str) -> String {
    BOUNTY_SYNDICATES
        .iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, n)| (*n).to_string())
        .unwrap_or_else(|| tag.to_string())
}

/// jobType 路径末段 → 游戏内完整中文名（browse.wf dict.zh / Public Export）。
const JOB_TITLE_BY_BASE: &[(&str, &str)] = &[
    // Ostron / 平野
    ("AssassinateBountyAss", "刺杀指挥官"),
    ("AssassinateBountyCap", "捕获新任 Grineer 指挥官"),
    ("AttritionBountyCap", "捕获他们的领袖"),
    ("AttritionBountyExt", "宰杀敌人"),
    ("AttritionBountyLib", "削弱 Grineer 的据点"),
    ("AttritionBountySab", "破坏 Grineer 的补给线"),
    ("CaptureBountyCapOne", "捕获 Grineer 指挥官"),
    ("CaptureBountyCapTwo", "间谍捕手"),
    ("ReclamationBountyCache", "找出遗失的器物"),
    ("ReclamationBountyCap", "捕获 Grineer 特工"),
    ("ReclamationBountyTheft", "取回被偷的器物"),
    ("RescueBountyResc", "搜索并救援"),
    ("SabotageBountySab", "破坏原型机"),
    // Solaris / 金星（jobType 常带 Venus 前缀）
    ("ArtifactJobAmbush", "伏击信使"),
    ("ArtifactJobExcavation", "考古学"),
    ("ArtifactJobRecovery", "血红遗物"),
    ("ChaosJobAssassinate", "焦土大地"),
    ("ChaosJobExcavation", "埋葬他们"),
    ("CullJobAssassinate", "网络崩溃"),
    ("CullJobExterminate", "猎人杀手"),
    ("CullJobResource", "分散与裂解"),
    ("HelpingJobCache", "鹰嘴板"),
    ("HelpingJobResource", "尘土部队"),
    ("HelpingJobSpy", "貌似合法"),
    ("IntelJobRecovery", "存活证明"),
    ("IntelJobResource", "行动情报"),
    ("IntelJobSpy", "财政解放"),
    ("PreservationJobDefense", "岗哨职责"),
    ("PreservationJobRecovery", "保护无辜"),
    ("PreservationJobResource", "拆散陷阱"),
    ("SpyJobSpy", "特工的下落"),
    ("TheftJobAmbush", "软件诡计"),
    ("TheftJobExcavation", "资源搜掠"),
    ("TheftJobResource", "向税务官课税"),
    ("WetworkJobAssassinate", "冷餐"),
    ("WetworkJobSpy", "陨星"),
    ("VenusWetworkJobSpy", "陨星"),
    ("VenusWetworkJobAssassinate", "冷餐"),
    ("VenusPreservationJobDefense", "岗哨职责"),
    ("VenusPreservationJobResource", "拆散陷阱"),
    ("VenusPreservationJobRecovery", "保护无辜"),
    ("VenusTheftJobAmbush", "软件诡计"),
    ("VenusTheftJobExcavation", "资源搜掠"),
    ("VenusTheftJobResource", "向税务官课税"),
    ("VenusArtifactJobAmbush", "伏击信使"),
    ("VenusArtifactJobExcavation", "考古学"),
    ("VenusCullJobAssassinate", "网络崩溃"),
    ("VenusCullJobExterminate", "猎人杀手"),
    ("VenusCullJobResource", "分散与裂解"),
    ("VenusChaosJobAssassinate", "焦土大地"),
    ("VenusHelpingJobSpy", "貌似合法"),
    ("VenusHelpingJobResource", "尘土部队"),
    ("VenusHelpingJobCache", "鹰嘴板"),
    ("VenusIntelJobSpy", "财政解放"),
    // 合一众 / Narmer
    ("NarmerBountyAssassinate", "大起大落（合一众）"),
    ("NarmerBountyExterminate", "为了乌努母神（合一众）"),
    ("NarmerBountyLiberation", "带他们回家（合一众）"),
    ("NarmerBountyVenusAssassinate", "邪主之音（合一众）"),
    ("NarmerBountyVenusExterminate", "粉碎邪教（合一众）"),
    ("NarmerBountyVenusPreservation", "金属意识（合一众）"),
    ("NarmerBountyVenusTheftExcavation", "掘地三尺（合一众）"),
    ("NarmerVenusTheftJobExcavation", "掘地三尺（合一众）"),
    ("NarmerVenusAssassinateJob", "邪主之音（合一众）"),
    ("NarmerVenusExterminateJob", "粉碎邪教（合一众）"),
    ("NarmerVenusPreservationJob", "金属意识（合一众）"),
    // Deimos / 英择谛
    ("DeimosAreaDefenseBounty", "重夺领地"),
    ("DeimosAssassinateBounty", "清净之地"),
    ("DeimosCrpSurvivorBounty", "为了科学！"),
    ("DeimosEndlessExcavateBounty", "露天开采（无尽）"),
    ("DeimosEndlessAreaDefenseBounty", "泛滥成灾（无尽）"),
    ("DeimosEndlessPurifyBounty", "古物猎人（无尽）"),
    ("DeimosExcavateBounty", "核心样本"),
    ("DeimosGrnSurvivorBounty", "蛮暴之力"),
    ("DeimosKeyPiecesBounty", "物资回收"),
    ("DeimosPurifyBounty", "异物取回"),
];

/// 从 jobType 路径归类任务种类（稳定英文 key，供关注筛选）。
pub fn job_kind_key(job_type: &str) -> String {
    let base = job_type.rsplit('/').next().unwrap_or(job_type);
    let lower = base.to_ascii_lowercase();
    let pairs: &[(&str, &str)] = &[
        ("narmer", "Narmer"),
        ("exterminate", "Exterminate"),
        ("capture", "Capture"),
        ("attrition", "Attrition"),
        ("cull", "Cull"),
        ("helping", "Helping"),
        ("rescue", "Rescue"),
        ("reclamation", "Reclamation"),
        ("sabotage", "Sabotage"),
        ("spy", "Spy"),
        ("areadefense", "Defense"),
        ("defense", "Defense"),
        ("assassinate", "Assassinate"),
        ("excavate", "Excavate"),
        ("excavation", "Excavate"),
        ("preservation", "Preservation"),
        ("theft", "Theft"),
        ("ambush", "Ambush"),
        ("wetwork", "Wetwork"),
        ("crpsurvivor", "Survivor"),
        ("grnsurvivor", "Survivor"),
        ("keypieces", "Cache"),
        ("endless", "Endless"),
        ("cache", "Cache"),
        ("hijack", "Hijack"),
        ("armoured", "Armoured"),
        ("liberation", "Liberation"),
        ("purify", "Purify"),
        ("survival", "Survival"),
        ("alchemy", "Alchemy"),
        ("artifact", "Disruption"),
        ("corruption", "VoidFlood"),
        ("cascade", "VoidCascade"),
        ("armageddon", "VoidArmageddon"),
    ];
    for (needle, kind) in pairs {
        if lower.contains(needle) {
            return (*kind).to_string();
        }
    }
    if base.is_empty() {
        "Unknown".into()
    } else {
        base.to_string()
    }
}

pub fn job_kind_zh(kind: &str) -> String {
    match kind {
        "Narmer" => "纳威",
        "Capture" => "捕获",
        "Attrition" => "消耗",
        "Cull" => "清剿",
        "Helping" => "援助",
        "Rescue" => "救援",
        "Reclamation" => "回收",
        "Sabotage" => "破坏",
        "Spy" => "间谍",
        "Defense" | "MobileDefense" | "DualDefense" => "防御",
        "Assassinate" => "刺杀",
        "Excavate" => "挖掘",
        "Preservation" => "保全",
        "Theft" => "盗取",
        "Ambush" => "伏击",
        "Wetwork" => "暗杀行动",
        "Survivor" => "幸存者",
        "Cache" => "储藏箱",
        "Endless" => "无尽",
        "Hijack" => "劫持",
        "Armoured" => "装甲",
        "Liberation" => "解放",
        "Purify" => "净化",
        "Exterminate" => "歼灭",
        "Survival" => "生存",
        "Alchemy" => "元素转换",
        "Disruption" => "中断",
        "VoidFlood" => "虚空洪流",
        "VoidCascade" => "虚空覆涌",
        "VoidArmageddon" => "虚空决战",
        "Legacyte" => "传承种收割",
        "HellScrub" => "地狱净化",
        // 旧 cycle 挑战种类（兼容已存关注）
        "Hazards" => "高危物品",
        "HighKill" => "天降正义",
        "DestroyProps" => "摧毁掩护",
        "Exploding" => "爆炸性科腐者",
        "SafeCracker" => "破壳专家",
        "Operator" => "脱离战甲",
        "Collect" => "收集",
        "Vial" => "化学课",
        "Doppelganger" => "致命低语",
        "Decoration" => "破坏能手",
        "Conduit" => "坚毅爆破者",
        "Challenge" => "挑战",
        "Unknown" => "未知",
        other => other,
    }
    .to_string()
}

fn lookup_job_title(base: &str) -> Option<&'static str> {
    JOB_TITLE_BY_BASE
        .iter()
        .find(|(k, _)| *k == base)
        .map(|(_, v)| *v)
}

/// worldState Jobs 常为空、需用 bounty-cycle 补齐的势力。
pub fn is_cycle_filled_syndicate(tag: &str) -> bool {
    matches!(
        tag,
        "ZarimanSyndicate" | "EntratiLabSyndicate" | "HexSyndicate"
    )
}

/// 档位：(min, max, standing)。按 cycle 数组下标取。
pub fn cycle_tier_stats(tag: &str, index: usize) -> Option<(i32, i32, i32)> {
    let table: &[(i32, i32, i32)] = match tag {
        "HexSyndicate" => &[
            (65, 75, 1000),
            (75, 80, 2000),
            (85, 90, 3000),
            (95, 100, 4000),
            (105, 110, 5000),
            (115, 120, 6000),
            (125, 130, 7500),
        ],
        "ZarimanSyndicate" => &[
            (50, 55, 1000),
            (60, 65, 2000),
            (70, 75, 3000),
            (80, 85, 4000),
            (90, 100, 5000),
        ],
        "EntratiLabSyndicate" => &[
            (55, 60, 1000),
            (65, 70, 2000),
            (75, 80, 3000),
            (85, 90, 4000),
            (95, 110, 5000),
        ],
        _ => return None,
    };
    table.get(index).copied()
}

/// cycle 赏金节点：标题 + 种类（拉取解析时查表）。
struct CycleNodeEntry {
    node: &'static str,
    title_zh: &'static str,
    kind: &'static str,
    kind_zh: &'static str,
}

const CYCLE_BOUNTY_NODES: &[CycleNodeEntry] = &[
    // Hex / 霍瓦尼亚
    CycleNodeEntry {
        node: "SolNode850",
        title_zh: "传承种收割",
        kind: "Legacyte",
        kind_zh: "传承种收割",
    },
    CycleNodeEntry {
        node: "SolNode851",
        title_zh: "地狱净化：炽蛇军",
        kind: "Survival",
        kind_zh: "生存",
    },
    CycleNodeEntry {
        node: "SolNode852",
        title_zh: "地狱净化：科腐者",
        kind: "Survival",
        kind_zh: "生存",
    },
    CycleNodeEntry {
        node: "SolNode853",
        title_zh: "歼灭：炽蛇军",
        kind: "Exterminate",
        kind_zh: "歼灭",
    },
    CycleNodeEntry {
        node: "SolNode854",
        title_zh: "歼灭：科腐者",
        kind: "Exterminate",
        kind_zh: "歼灭",
    },
    CycleNodeEntry {
        node: "SolNode856",
        title_zh: "刺杀：H-09 坦克",
        kind: "Assassinate",
        kind_zh: "刺杀",
    },
    CycleNodeEntry {
        node: "SolNode858",
        title_zh: "至日广场 · 防御",
        kind: "Defense",
        kind_zh: "防御",
    },
    // Zariman / 坚守者
    CycleNodeEntry {
        node: "SolNode230",
        title_zh: "虚空洪流 · 永视弧域",
        kind: "VoidFlood",
        kind_zh: "虚空洪流",
    },
    CycleNodeEntry {
        node: "SolNode231",
        title_zh: "歼灭 · 哈拉科防线",
        kind: "Exterminate",
        kind_zh: "歼灭",
    },
    CycleNodeEntry {
        node: "SolNode232",
        title_zh: "虚空覆涌 · 涂沃主厅",
        kind: "VoidCascade",
        kind_zh: "虚空覆涌",
    },
    CycleNodeEntry {
        node: "SolNode233",
        title_zh: "虚空决战 · 奥金工场",
        kind: "VoidArmageddon",
        kind_zh: "虚空决战",
    },
    CycleNodeEntry {
        node: "SolNode235",
        title_zh: "移动防御 · 翠径",
        kind: "MobileDefense",
        kind_zh: "移动防御",
    },
    // Cavia / 科维兽
    CycleNodeEntry {
        node: "SolNode715",
        title_zh: "刺杀 · 恶涌",
        kind: "Assassinate",
        kind_zh: "刺杀",
    },
    CycleNodeEntry {
        node: "SolNode716",
        title_zh: "歼灭 · 孽杀",
        kind: "Exterminate",
        kind_zh: "歼灭",
    },
    CycleNodeEntry {
        node: "SolNode717",
        title_zh: "生存 · 不灭之地",
        kind: "Survival",
        kind_zh: "生存",
    },
    CycleNodeEntry {
        node: "SolNode718",
        title_zh: "元素转换 · 异化区",
        kind: "Alchemy",
        kind_zh: "元素转换",
    },
    CycleNodeEntry {
        node: "SolNode719",
        title_zh: "镜像防御 · 墓垒",
        kind: "DualDefense",
        kind_zh: "镜像防御",
    },
    CycleNodeEntry {
        node: "SolNode721",
        title_zh: "中断 · 卫城区",
        kind: "Disruption",
        kind_zh: "中断",
    },
];

fn lookup_cycle_node(node: &str) -> Option<&'static CycleNodeEntry> {
    CYCLE_BOUNTY_NODES.iter().find(|e| e.node == node)
}

pub fn cycle_node_title_zh(node: &str) -> String {
    if let Some(e) = lookup_cycle_node(node) {
        return e.title_zh.to_string();
    }
    // 未收录：有种类则「种类 · 英文地名」，仅有地名则用地名，否则原码
    let (kind, kind_zh) = cycle_node_mission_kind(node);
    if let Some(place) = super::localize::sol_node_place_name(node) {
        if kind != "Challenge" {
            return format!("{kind_zh} · {place}");
        }
        return place;
    }
    node.to_string()
}

/// cycle 节点 → 任务种类（关注筛选用；歼灭单独一类）。
pub fn cycle_node_mission_kind(node: &str) -> (String, String) {
    if let Some(e) = lookup_cycle_node(node) {
        return (e.kind.to_string(), e.kind_zh.to_string());
    }
    ("Challenge".into(), "挑战".into())
}

/// challenge 路径 basename → 可执行说明（优先完整目标，避免「致命低语」这类不知所云的简称）。
const CYCLE_CHALLENGE_DESC: &[(&str, &str)] = &[
    // 科维兽 / EntratiLab（与解剖圣所赏金板一致）
    (
        "EntratiLabKillFlyingMurmurChallenge",
        "杀死 3 个飞行的低语者敌人",
    ),
    (
        "EntratiLabDestroyDecorationChallenge",
        "摧毁 60 个装饰物",
    ),
    (
        "EntratiLabDefeatDoppelgangerChallenge",
        "击败嘲讽低语或尖刻低语",
    ),
    (
        "EntratiLabKillVoidRigEasyChallenge",
        "消灭 2 名暴戾虚空锐将",
    ),
    (
        "EntratiLabNearConduitVulnerabilityChallenge",
        "爆破型殁世机甲在距离导管 40 米内被消灭",
    ),
    (
        "EntratiLabKillVialedEnemyChallenge",
        "消灭带药瓶的敌人",
    ),
    (
        "EntratiLabAlchemyGrenadeElectricHardChallenge",
        "用电击转化完成元素转换挑战",
    ),
];

/// challenge 路径 → 中文说明（展示在任务条上）。
///
/// 优先 browse.wf `ExportChallenges` + `dict.zh`（传入 [`ChallengeLocalizer`]）；
/// 无词典时回退手写表 / 启发式。
pub fn cycle_challenge_zh(
    challenge: &str,
    dict: Option<&super::challenge_i18n::ChallengeLocalizer>,
) -> String {
    if let Some(loc) = dict {
        if let Some(zh) = loc.challenge_desc_zh(challenge) {
            return zh;
        }
    }
    let base = challenge.rsplit('/').next().unwrap_or(challenge);
    if let Some((_, desc)) = CYCLE_CHALLENGE_DESC.iter().find(|(k, _)| *k == base) {
        return (*desc).to_string();
    }
    let lower = base.to_ascii_lowercase();
    // 路径变体启发式（仍写完整目标）
    let label = if lower.contains("killflyingmurmur") || lower.contains("flyingmurmur") {
        "杀死 3 个飞行的低语者敌人"
    } else if lower.contains("destroydecoration") {
        "摧毁 60 个装饰物"
    } else if lower.contains("defeatdoppel") {
        "击败嘲讽低语或尖刻低语"
    } else if lower.contains("killvoidrig") || lower.contains("voidrig") {
        "消灭 2 名暴戾虚空锐将"
    } else if lower.contains("nearconduit") {
        "爆破型殁世机甲在距离导管 40 米内被消灭"
    } else if lower.contains("killvialed") {
        "消灭带药瓶的敌人"
    } else if lower.contains("alchemygrenade") {
        "用电击转化完成元素转换挑战"
    } else if lower.contains("destroyhazards") {
        "摧毁高危物品"
    } else if lower.contains("capturetargets") {
        "捕获目标"
    } else if lower.contains("highkill") {
        "达到击杀数要求"
    } else if lower.contains("destroyprops") {
        "摧毁掩护物"
    } else if lower.contains("explodinginfested") {
        "消灭爆炸性科腐者"
    } else if lower.contains("safecracker") {
        "破解保险柜"
    } else if lower.contains("exterminatefast") {
        "快速完成歼灭"
    } else if lower.contains("killasoperator") {
        "以运算符击杀敌人"
    } else if lower.contains("findmelica") {
        "找到梅利卡并为其加油打气"
    } else if lower.contains("corruptioncollect") || lower.contains("collectorbs") {
        "收集修复凝剂滴球"
    } else if lower.contains("assassinatecollect") || lower.contains("scrapcrates") {
        "收集氩烯废料箱"
    } else if lower.contains("usevoidrifts") {
        "使用虚空裂隙"
    } else if lower.contains("killcorpus") || lower.contains("killgrineer") {
        "击杀指定阵营敌人"
    } else if lower.contains("keepmeterlow") {
        "保持腐化仪表在低位"
    } else if lower.contains("killangels") {
        "击杀虚空天使"
    } else if lower.contains("destroyvehicles") {
        "摧毁载具"
    } else if lower.contains("abilitykill") {
        "用技能击杀敌人"
    } else if lower.contains("infestedcrossfire") {
        "在交叉火力中消灭感染者"
    } else {
        return humanize_challenge_base(base);
    };
    label.to_string()
}

fn humanize_challenge_base(base: &str) -> String {
    let mut out = String::new();
    for (i, ch) in base.chars().enumerate() {
        if i > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// 游戏内完整中文赏金名；未收录则回退种类名。钢路档加「钢路·」前缀。
pub fn job_title_zh(job_type: &str, kind: &str, min_lv: i32, max_lv: i32) -> String {
    let base = job_type.rsplit('/').next().unwrap_or(job_type);
    let mut title = lookup_job_title(base)
        .map(|s| s.to_string())
        .or_else(|| {
            // VenusWetworkJobSpy → WetworkJobSpy
            base.strip_prefix("Venus")
                .and_then(|rest| lookup_job_title(rest).map(|s| s.to_string()))
        })
        .unwrap_or_else(|| job_kind_zh(kind));

    let is_steel = min_lv >= 100 && max_lv >= 100;
    let is_narmer = kind == "Narmer" || base.to_ascii_lowercase().contains("narmer");
    if is_steel && !title.contains("钢路") {
        title = format!("钢路·{title}");
    } else if is_narmer && !title.contains("合一众") && !title.contains("纳威") {
        title = format!("纳威·{title}");
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_bounty_full_zh_title() {
        let t = job_title_zh(
            "/Lotus/Types/Gameplay/Eidolon/Jobs/CaptureBountyCapOne",
            "Capture",
            5,
            15,
        );
        assert_eq!(t, "捕获 Grineer 指挥官");
    }

    #[test]
    fn steel_path_prefixes_title() {
        let t = job_title_zh(
            "/Lotus/Types/Gameplay/Eidolon/Jobs/AttritionBountySab",
            "Attrition",
            100,
            100,
        );
        assert!(t.starts_with("钢路·"));
        assert!(t.contains("破坏 Grineer 的补给线"));
    }

    #[test]
    fn venus_job_maps_without_prefix_table_hit() {
        let t = job_title_zh(
            "/Lotus/Types/Gameplay/Venus/Jobs/VenusWetworkJobSpy",
            "Spy",
            5,
            15,
        );
        assert_eq!(t, "陨星");
    }

    #[test]
    fn cycle_hex_node_and_challenge() {
        assert_eq!(cycle_node_title_zh("SolNode856"), "刺杀：H-09 坦克");
        let (k, zh) = cycle_node_mission_kind("SolNode854");
        assert_eq!(k, "Exterminate");
        assert_eq!(zh, "歼灭");
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/Vania/VaniaDestroyHazardsEasy",
                None,
            ),
            "摧毁高危物品"
        );
        let (min, max, standing) = cycle_tier_stats("HexSyndicate", 6).unwrap();
        assert_eq!((min, max, standing), (125, 130, 7500));
    }

    #[test]
    fn cycle_cavia_solnode716_exterminate() {
        assert_eq!(cycle_node_title_zh("SolNode716"), "歼灭 · 孽杀");
        let (k, zh) = cycle_node_mission_kind("SolNode716");
        assert_eq!(k, "Exterminate");
        assert_eq!(zh, "歼灭");
    }

    #[test]
    fn cycle_cavia_challenge_shows_actionable_desc() {
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge",
                None,
            ),
            "击败嘲讽低语或尖刻低语"
        );
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabKillFlyingMurmurChallenge",
                None,
            ),
            "杀死 3 个飞行的低语者敌人"
        );
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabDestroyDecorationChallenge",
                None,
            ),
            "摧毁 60 个装饰物"
        );
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabKillVoidRigEasyChallenge",
                None,
            ),
            "消灭 2 名暴戾虚空锐将"
        );
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabNearConduitVulnerabilityChallenge",
                None,
            ),
            "爆破型殁世机甲在距离导管 40 米内被消灭"
        );
    }

    #[test]
    fn cycle_challenge_prefers_browse_wf_dict() {
        let ch = r#"{
          "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge": {
            "description": "/Lotus/Language/X/Desc",
            "requiredCount": 1
          }
        }"#;
        let dict = r#"{ "/Lotus/Language/X/Desc": "词典里的完整描述" }"#;
        let loc = super::super::challenge_i18n::ChallengeLocalizer::from_bytes(
            ch.as_bytes(),
            dict.as_bytes(),
        )
        .unwrap();
        assert_eq!(
            cycle_challenge_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge",
                Some(&loc),
            ),
            "词典里的完整描述"
        );
    }

    #[test]
    fn cycle_unknown_node_falls_back_to_sol_nodes_place() {
        // SolNode720 = Testudo，表未收录 → 回退英文地名
        assert_eq!(cycle_node_title_zh("SolNode720"), "Testudo");
        let (k, zh) = cycle_node_mission_kind("SolNode720");
        assert_eq!(k, "Challenge");
        assert_eq!(zh, "挑战");
        assert_eq!(cycle_node_title_zh("SolNode99999"), "SolNode99999");
    }

    #[test]
    fn venus_cull_helping_resource_kinds_zh() {
        let path = "/Lotus/Types/Gameplay/Venus/Jobs/VenusCullJobResource";
        let kind = job_kind_key(path);
        assert_eq!(kind, "Cull");
        assert_eq!(job_kind_zh(&kind), "清剿");
        assert_eq!(job_title_zh(path, &kind, 5, 15), "分散与裂解");

        let path2 = "/Lotus/Types/Gameplay/Venus/Jobs/VenusHelpingJobResource";
        let kind2 = job_kind_key(path2);
        assert_eq!(kind2, "Helping");
        assert_eq!(job_kind_zh(&kind2), "援助");
        assert_eq!(job_title_zh(path2, &kind2, 5, 15), "尘土部队");
    }
}
