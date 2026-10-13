use super::*;


const FIXTURE: &str = r#"{
  "Sorties": [{
"Activation": {"$date":{"$numberLong":"1790697600000"}},
"Expiry": {"$date":{"$numberLong":"1790784000000"}},
"Boss": "SORTIE_BOSS_VOR",
"Variants": [
  {"missionType":"MT_MOBILE_DEFENSE","modifierType":"SORTIE_MODIFIER_HAZARD_RADIATION","node":"SolNode747"},
  {"missionType":"MT_DEFENSE","modifierType":"SORTIE_MODIFIER_FIRE","node":"SolNode149"},
  {"missionType":"MT_ASSASSINATION","modifierType":"SORTIE_MODIFIER_LOW_ENERGY","node":"SolNode108"}
]
  }],
  "SeasonInfo": {
"ActiveChallenges": [
  {
    "_id": {"$oid":"daily1"},
    "Daily": true,
    "Activation": {"$date":{"$numberLong":"1790553600000"}},
    "Expiry": {"$date":{"$numberLong":"1790812800000"}},
    "Challenge": "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyCodexScan"
  },
  {
    "_id": {"$oid":"weekly1"},
    "Activation": {"$date":{"$numberLong":"1790553600000"}},
    "Expiry": {"$date":{"$numberLong":"1791158400000"}},
    "Challenge": "/Lotus/Types/Challenges/Seasons/Weekly/SeasonWeeklyHardTerminated"
  }
]
  },
  "LiteSorties": [{
"Activation": {"$date":{"$numberLong":"1789948800000"}},
"Expiry": {"$date":{"$numberLong":"1790553600000"}},
"Boss": "SORTIE_BOSS_BOREAL",
"Missions": [
  {"missionType":"MT_SABOTAGE","node":"SolNode149"},
  {"missionType":"MT_TERRITORY","node":"SolNode108"},
  {"missionType":"MT_ASSASSINATION","node":"SolNode747"}
]
  }],
  "Conquests": [{
"Activation": {"$date":{"$numberLong":"1789948800000"}},
"Expiry": {"$date":{"$numberLong":"1790553600000"}},
"Type": "CT_LAB",
"Missions": [{
  "faction":"FC_MITW",
  "missionType":"MT_EXTERMINATION",
  "difficulties":[
    {"type":"CD_NORMAL","deviation":"FortifiedFoes","risks":["Quicksand"]},
    {"type":"CD_HARD","deviation":"FortifiedFoes","risks":["Quicksand","RegeneratingEnemies"]}
  ]
}],
"Variables": ["OperatorLockout","Withering"]
  }],
  "EndlessXpSchedule": [{
"Activation": {"$date":{"$numberLong":"1789948800000"}},
"Expiry": {"$date":{"$numberLong":"1790553600000"}},
"CategoryChoices": [
  {"Category":"EXC_NORMAL","Choices":["Loki","Mag","Rhino"]},
  {"Category":"EXC_HARD","Choices":["Lex","Magistar","Boltor","Bronco","CeramicDagger"]}
]
  }],
  "KnownCalendarSeasons": [{
"Activation": {"$date":{"$numberLong":"1789948800000"}},
"Expiry": {"$date":{"$numberLong":"1790553600000"}},
"Days": [
  {"day":13,"events":[{"type":"CET_CHALLENGE","challenge":"/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy"}]},
  {"day":17,"events":[
    {"type":"CET_REWARD","reward":"/Lotus/StoreItems/Types/BoosterPacks/CalendarArtifactPack"},
    {"type":"CET_REWARD","reward":"/Lotus/StoreItems/Types/Gameplay/NarmerSorties/ArchonCrystalOrange"}
  ]},
  {"day":98,"events":[
    {"type":"CET_UPGRADE","upgrade":"/Lotus/Upgrades/Calendar/AbilityStrength"},
    {"type":"CET_UPGRADE","upgrade":"/Lotus/Upgrades/Calendar/Armor"},
    {"type":"CET_UPGRADE","upgrade":"/Lotus/Upgrades/Calendar/EnergyRestoration"}
  ]}
]
  }],
  "SyndicateMissions": [
{
  "Tag": "SteelMeridianSyndicate",
  "Activation": {"$date":{"$numberLong":"1789948800000"}},
  "Expiry": {"$date":{"$numberLong":"1790553600000"}},
  "Jobs": [],
  "Nodes": ["SolNode1"]
},
{
  "Tag": "CetusSyndicate",
  "Activation": {"$date":{"$numberLong":"1789948800000"}},
  "Expiry": {"$date":{"$numberLong":"1790035200000"}},
  "Jobs": [
    {
      "jobType": "/Lotus/Types/Gameplay/Eidolon/Jobs/CaptureBountyCapOne",
      "minEnemyLevel": 5,
      "maxEnemyLevel": 15,
      "xpAmounts": [400, 400, 490]
    },
    {
      "jobType": "/Lotus/Types/Gameplay/Eidolon/Jobs/AttritionBountySab",
      "minEnemyLevel": 100,
      "maxEnemyLevel": 100,
      "xpAmounts": [1000, 1200, 2820]
    },
    {
      "minEnemyLevel": 30,
      "maxEnemyLevel": 40,
      "xpAmounts": [2, 2, 2, 4],
      "isVault": true
    }
  ]
},
{
  "Tag": "SolarisSyndicate",
  "Activation": {"$date":{"$numberLong":"1789948800000"}},
  "Expiry": {"$date":{"$numberLong":"1790035200000"}},
  "Jobs": [
    {
      "jobType": "/Lotus/Types/Gameplay/Venus/Jobs/VenusWetworkJobSpy",
      "minEnemyLevel": 5,
      "maxEnemyLevel": 15,
      "xpAmounts": [300, 350, 400]
    }
  ]
}
  ]
}"#;

#[test]
fn parses_sortie_and_nightwave() {
    let f = parse_worldstate(FIXTURE.as_bytes()).unwrap();
    assert_eq!(f.sortie.boss.as_deref(), Some("沃尔队长"));
    assert_eq!(f.sortie.variants.len(), 3);
    assert!(f.steel.incursions.is_some());
    assert!(!f.source_url.contains("warframestat"));
    // 无词典时仍有手写回退，不该是 CamelCase 英文
    let daily = f
        .nightwave
        .active_challenges
        .iter()
        .find(|c| c.is_daily)
        .expect("daily nw");
    assert_eq!(daily.title.as_deref(), Some("研究者"));
    assert!(!daily.desc.as_deref().unwrap_or("").is_empty());
}

#[test]
fn nightwave_uses_challenge_dict_for_zh_and_standing() {
    let ch = r#"{
      "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyCodexScan": {
        "name": "/Lotus/Language/NightwaveChallenges/Challenge_SeasonDailyCodexScan_Name",
        "requiredCount": 15,
        "standing": 1000
      },
      "/Lotus/Types/Challenges/Seasons/WeeklyHard/SeasonWeeklyHardTerminated": {
        "name": "/Lotus/Language/NightwaveChallenges/Challenge_SeasonWeeklyHardTerminated_Name",
        "requiredCount": 3,
        "standing": 7000
      }
    }"#;
    let dict = r#"{
      "/Lotus/Language/NightwaveChallenges/Challenge_SeasonDailyCodexScan_Name": "研究者",
      "/Lotus/Language/NightwaveChallenges/Challenge_SeasonDailyCodexScan_Description": "扫描 |COUNT| 次物件或敌人。",
      "/Lotus/Language/NightwaveChallenges/Challenge_SeasonWeeklyHardTerminated_Name": "已终止",
      "/Lotus/Language/NightwaveChallenges/Challenge_SeasonWeeklyHardTerminated_Description": "摧毁 |COUNT| 个尸鬼金库守卫。"
    }"#;
    let loc =
        super::super::challenge_i18n::ChallengeLocalizer::from_bytes(ch.as_bytes(), dict.as_bytes())
            .unwrap();
    let f = parse_worldstate_with_cycle(FIXTURE.as_bytes(), None, Some(&loc), None).unwrap();
    let daily = f
        .nightwave
        .active_challenges
        .iter()
        .find(|c| c.is_daily)
        .expect("daily");
    assert_eq!(daily.title.as_deref(), Some("研究者"));
    assert_eq!(daily.desc.as_deref(), Some("扫描 15 次物件或敌人。"));
    assert_eq!(daily.reputation, Some(1000));
    let elite = f
        .nightwave
        .active_challenges
        .iter()
        .find(|c| c.is_elite)
        .expect("elite");
    assert_eq!(elite.title.as_deref(), Some("已终止"));
    assert_eq!(elite.desc.as_deref(), Some("摧毁 3 个尸鬼金库守卫。"));
    assert_eq!(elite.reputation, Some(7000));
}

#[test]
fn parses_weekly_bundle() {
    let ch = r#"{
      "/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy": {
        "name": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name",
        "description": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description",
        "requiredCount": 10
      }
    }"#;
    let dict = r#"{
      "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name": "逝去卓越者",
      "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description": "击杀 |COUNT| 名卓越者"
    }"#;
    let loc =
        super::super::challenge_i18n::ChallengeLocalizer::from_bytes(ch.as_bytes(), dict.as_bytes())
            .unwrap();
    let f = parse_worldstate_with_cycle(FIXTURE.as_bytes(), None, Some(&loc), None).unwrap();
    let a = f.weekly.archon_hunt.expect("archon");
    assert_eq!(a.boss, "Archon Boreal");
    assert_eq!(a.shard, "Azure Archon Shard");
    assert_eq!(a.missions.len(), 3);
    assert_eq!(a.missions[0].mission_type, "Sabotage");
    assert_eq!(a.missions[1].mission_type, "Interception");

    let deep = f.weekly.deep_archimedea.expect("deep");
    assert_eq!(deep.kind_label, "深层科研");
    assert!(!deep.personal_modifiers.is_empty());
    assert_eq!(deep.missions[0].mission_type, "Exterminate");
    // 无 Language 缓存时英文兜底（humanize）
    assert!(
        deep.missions[0].deviation.title.contains("Fortified"),
        "{:?}",
        deep.missions[0].deviation.title
    );
    assert!(deep.missions[0].deviation.description.is_empty());
    assert!(deep.missions[0]
        .risks_elite
        .iter()
        .any(|r| r.title.contains("Regenerating")));
    assert!(deep
        .personal_modifiers
        .iter()
        .any(|p| p.title.contains("Operator")));

    let normal = f.weekly.normal_circuit.expect("normal circuit");
    assert_eq!(normal.this_week.len(), 3);
    assert!(!normal.next_week.is_empty());
    assert_eq!(normal.rotation_weeks.len(), 11);
    assert!(normal.current_week_index.is_some());

    let steel = f.weekly.steel_circuit.expect("steel circuit");
    assert_eq!(steel.this_week.len(), 5);
    assert!(!steel.next_week.is_empty());
    assert_eq!(steel.rotation_weeks.len(), 9);

    let cal = f.weekly.hex_calendar.expect("calendar");
    assert_eq!(cal.days.len(), 3);
    let d0 = &cal.days[0];
    assert_eq!(d0.day, 13);
    assert_eq!(d0.groups.len(), 1);
    assert_eq!(d0.groups[0].kind, "challenge");
    assert!(d0.groups[0].choices[0].title.contains("卓越"));
    assert_eq!(d0.groups[0].choices[0].title, "逝去卓越者");
    assert_eq!(d0.groups[0].choices[0].description, "击杀 10 名卓越者");
    let d1 = &cal.days[1];
    assert_eq!(d1.groups.len(), 1);
    assert_eq!(d1.groups[0].kind, "reward");
    assert_eq!(d1.groups[0].choices.len(), 2);
    let d2 = &cal.days[2];
    assert_eq!(d2.day, 98);
    assert_eq!(d2.groups[0].kind, "upgrade");
    assert_eq!(d2.groups[0].choices.len(), 3);
    // 增益只保留英文名供前端悬停，无中文说明
    assert!(
        d2.groups[0]
            .choices
            .iter()
            .all(|c| c.description.is_empty() && !c.title.is_empty()),
        "{:?}",
        d2.groups[0].choices
    );
    assert!(
        d2.groups[0]
            .choices
            .iter()
            .any(|c| c.title.contains("Ability") || c.title.contains("Strength")),
        "{:?}",
        d2.groups[0].choices
    );
}

#[test]
fn parses_bounty_boards_whitelist_and_advanced() {
    let f = parse_worldstate(FIXTURE.as_bytes()).unwrap();
    assert_eq!(f.bounties.boards.len(), 11);
    assert!(f
        .bounties
        .boards
        .iter()
        .any(|b| b.tag == "CetusSyndicate"));
    assert!(!f
        .bounties
        .boards
        .iter()
        .any(|b| b.tag == "SteelMeridianSyndicate"));

    let cetus = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "CetusSyndicate")
        .expect("cetus");
    assert_eq!(cetus.name, "Ostron");
    assert_eq!(cetus.jobs.len(), 2);
    let capture = cetus
        .jobs
        .iter()
        .find(|j| j.job_type.contains("CaptureBountyCapOne"))
        .expect("capture");
    assert_eq!(capture.title, "捕获 Grineer 指挥官");
    let adv = cetus.jobs.iter().find(|j| j.is_advanced).expect("advanced");
    assert_eq!(adv.standing_total, 5020);
    assert_eq!(adv.kind, "Attrition");
    assert!(adv.is_steel_path);
    assert!(adv.title.contains("破坏 Grineer 的补给线"));
    assert!(adv.title.starts_with("钢路·"));
}

#[test]
fn steel_incursion_is_utc_day_window() {
    let steel = synthesize_steel_incursion();
    let inc = steel.incursions.expect("incursions");
    let act = inc.activation.expect("activation");
    let exp = inc.expiry.expect("expiry");
    assert!(act.contains("T00:00:00"), "activation={act}");
    assert!(exp.contains("T23:59:59"), "expiry={exp}");
    assert_eq!(&act[..10], &exp[..10]);
}

#[test]
fn parses_fissures_normal_steel_storm() {
    let body = r#"{
  "Sorties": [],
  "ActiveMissions": [
{
  "Activation": {"$date":{"$numberLong":"1791479942433"}},
  "Expiry": {"$date":{"$numberLong":"1791486938573"}},
  "Node": "SolNode11",
  "MissionType": "MT_MOBILE_DEFENSE",
  "Modifier": "VoidT1",
  "Hard": false
},
{
  "Activation": {"$date":{"$numberLong":"1791479942433"}},
  "Expiry": {"$date":{"$numberLong":"1791486938573"}},
  "Node": "SolNode42",
  "MissionType": "MT_DEFENSE",
  "Modifier": "VoidT4",
  "Hard": true
}
  ],
  "VoidStorms": [
{
  "Activation": {"$date":{"$numberLong":"1791483603351"}},
  "Expiry": {"$date":{"$numberLong":"1791489003351"}},
  "Node": "CrewBattleNode518",
  "ActiveMissionTier": "VoidT2"
}
  ]
}"#;
    let f = parse_worldstate(body.as_bytes()).unwrap();
    assert_eq!(f.fissures.len(), 3);
    let normal = f
        .fissures
        .iter()
        .find(|x| x.kind == "normal")
        .expect("normal");
    assert_eq!(normal.tier_zh, "古纪");
    assert_eq!(normal.mission_type, "移动防御");
    assert!(normal.node.contains("Tharsis"));
    let steel = f
        .fissures
        .iter()
        .find(|x| x.kind == "steel")
        .expect("steel");
    assert_eq!(steel.tier_zh, "后纪");
    let storm = f
        .fissures
        .iter()
        .find(|x| x.kind == "storm")
        .expect("storm");
    assert_eq!(storm.tier_zh, "前纪");
    assert_eq!(storm.node_code, "CrewBattleNode518");
    // worldState 本身无 MissionType；由 fill_storm_fissure_mission_types 补
    assert!(storm.mission_type.is_empty());
}

const CYCLE_FIXTURE: &str = r#"{
  "expiry": 1790759788872,
  "rot": "B",
  "vaultRot": "C",
  "zarimanFaction": "FC_GRINEER",
  "bounties": {
"ZarimanSyndicate": [
  {"node": "SolNode231", "challenge": "/Lotus/Types/Challenges/Zariman/ZarimanExterminateFastCompleteChallenge"},
  {"node": "SolNode232", "challenge": "/Lotus/Types/Challenges/Zariman/ZarimanKillAsOperatorEasyChallenge"},
  {"node": "SolNode235", "challenge": "/Lotus/Types/Challenges/Zariman/ZarimanFindMelicaCacheChallenge"},
  {"node": "SolNode230", "challenge": "/Lotus/Types/Challenges/Zariman/ZarimanCorruptionCollectOrbsHardChallenge"},
  {"node": "SolNode233", "challenge": "/Lotus/Types/Challenges/Zariman/ZarimanAssassinateCollectScrapCratesVeryHardChallenge"}
],
"EntratiLabSyndicate": [
  {"node": "SolNode717", "challenge": "/Lotus/Types/Challenges/EntratiLab/EntratiLabKillVialedEnemyChallenge"},
  {"node": "SolNode715", "challenge": "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge"},
  {"node": "SolNode719", "challenge": "/Lotus/Types/Challenges/EntratiLab/EntratiLabDestroyDecorationChallenge"},
  {"node": "SolNode718", "challenge": "/Lotus/Types/Challenges/EntratiLab/EntratiLabAlchemyGrenadeElectricHardChallenge"},
  {"node": "SolNode721", "challenge": "/Lotus/Types/Challenges/EntratiLab/EntratiLabNearConduitVulnerabilityChallenge"}
],
"HexSyndicate": [
  {"node": "SolNode856", "challenge": "/Lotus/Types/Challenges/Vania/VaniaDestroyHazardsEasy", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/QuincyAllyAgent"},
  {"node": "SolNode850", "challenge": "/Lotus/Types/Challenges/Vania/VaniaCaptureTargetsEasy", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/AoiAllyAgent"},
  {"node": "SolNode858", "challenge": "/Lotus/Types/Challenges/Vania/VaniaHighKillNormal", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/EleanorAllyAgent"},
  {"node": "SolNode852", "challenge": "/Lotus/Types/Challenges/Vania/VaniaHighKillHard", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/ArthurAllyAgent"},
  {"node": "SolNode851", "challenge": "/Lotus/Types/Challenges/Vania/VaniaDestroyPropsVeryHard", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/LettieAllyAgent"},
  {"node": "SolNode854", "challenge": "/Lotus/Types/Challenges/Vania/VaniaExplodingInfested", "ally": "/Lotus/Types/Gameplay/1999Wf/ProtoframeAllies/AmirAllyAgent"},
  {"node": "SolNode852", "challenge": "/Lotus/Types/Challenges/Vania/Lich/LichVaniaSafeCracker"}
]
  }
}"#;

#[test]
fn cycle_fills_empty_zariman_cavia_hex_boards() {
    let f = parse_worldstate_with_cycle(
        FIXTURE.as_bytes(),
        Some(CYCLE_FIXTURE.as_bytes()),
        None,
        None,
    )
    .unwrap();
    let hex = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "HexSyndicate")
        .expect("hex");
    assert_eq!(hex.jobs.len(), 7);
    assert_eq!(hex.name, "六人组");
    assert!(hex.expiry.is_some());
    let adv = hex.jobs.iter().find(|j| j.is_advanced).expect("hex advanced");
    assert_eq!(adv.standing_total, 7500);
    assert!(adv.title.contains("科腐者") || adv.title.contains("破壳") || adv.job_type.contains("SafeCracker"));
    assert_eq!(
        hex.jobs.iter().map(|j| j.standing_total).max(),
        Some(7500)
    );
    // 档位表最高档标题：最后一档 node 为 SolNode852 → 地狱净化：科腐者
    assert!(hex.jobs.iter().any(|j| j.title.contains("刺杀")));

    let zar = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "ZarimanSyndicate")
        .expect("zariman");
    assert_eq!(zar.jobs.len(), 5);
    assert!(zar.jobs.iter().any(|j| j.is_advanced));

    let cavia = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "EntratiLabSyndicate")
        .expect("cavia");
    assert_eq!(cavia.jobs.len(), 5);

    // Cetus 仍走 worldState Jobs，不受 cycle 影响
    let cetus = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "CetusSyndicate")
        .expect("cetus");
    assert_eq!(cetus.jobs.len(), 2);
}

#[test]
fn cycle_missing_leaves_empty_boards() {
    let f = parse_worldstate_with_cycle(FIXTURE.as_bytes(), None, None, None).unwrap();
    let hex = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "HexSyndicate")
        .expect("hex");
    assert!(hex.jobs.is_empty());
    let zar = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "ZarimanSyndicate")
        .expect("zariman");
    assert!(zar.jobs.is_empty());
}

#[test]
fn cycle_invalid_json_degrades() {
    let f =
        parse_worldstate_with_cycle(FIXTURE.as_bytes(), Some(b"not-json"), None, None).unwrap();
    let hex = f
        .bounties
        .boards
        .iter()
        .find(|b| b.tag == "HexSyndicate")
        .expect("hex");
    assert!(hex.jobs.is_empty());
}
