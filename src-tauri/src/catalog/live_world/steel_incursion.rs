//! 钢铁之路侵袭：browse.wf `sp-incursions.txt` + ExportRegions / Factions / MissionTypes + dict.zh。
//!
//! 文案一律走官方导出语言 path → dict.zh，节点/任务/阵营换了也能翻，无需手改表。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use chrono::{TimeZone, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::catalog::upstream_cache;

use super::challenge_i18n::{self, DictZh};
use super::localize;
use super::parse::synthesize_steel_incursion;
use super::types::{
    FissureApi, SteelIncursionApi, SteelIncursionMissionApi, SteelPathApi, FISSURE_KIND_STORM,
    USER_AGENT,
};

pub(crate) const SP_INCURSIONS_URL: &str = "https://browse.wf/sp-incursions.txt";
const EXPORT_REGIONS_URL: &str =
    "https://browse.wf/warframe-public-export-plus/ExportRegions.json";
const EXPORT_FACTIONS_URL: &str =
    "https://browse.wf/warframe-public-export-plus/ExportFactions.json";
const EXPORT_MISSION_TYPES_URL: &str =
    "https://browse.wf/warframe-public-export-plus/ExportMissionTypes.json";

pub(crate) const SP_INCURSIONS_FILE: &str = "sp-incursions.txt";
const EXPORT_REGIONS_FILE: &str = "ExportRegions.json";
const EXPORT_FACTIONS_FILE: &str = "ExportFactions.json";
const EXPORT_MISSION_TYPES_FILE: &str = "ExportMissionTypes.json";

/// 导出表缓存最长有效期。
const EXPORT_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

/// 扎里曼：browse.wf 展示时不写阵营。
const ZARIMAN_SYSTEM_INDEX: i32 = 21;

#[derive(Debug, Deserialize)]
struct RegionMeta {
    name: Option<String>,
    #[serde(default, rename = "systemName")]
    system_name: Option<String>,
    #[serde(default, rename = "systemIndex")]
    system_index: Option<i32>,
    #[serde(default, rename = "missionType")]
    mission_type: Option<String>,
    #[serde(default, rename = "missionName")]
    mission_name: Option<String>,
    #[serde(default, rename = "levelOverride")]
    level_override: Option<String>,
    #[serde(default, rename = "enemySpec")]
    enemy_spec: Option<String>,
    faction: Option<String>,
    #[serde(default, rename = "minEnemyLevel")]
    min_enemy_level: Option<i32>,
    #[serde(default, rename = "maxEnemyLevel")]
    max_enemy_level: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct NamedExport {
    #[serde(default)]
    name: Option<String>,
}

struct RegionsIndex {
    by_code: HashMap<String, RegionMeta>,
}

impl RegionsIndex {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let by_code: HashMap<String, RegionMeta> = serde_json::from_slice(bytes)
            .map_err(|e| format!("解析 ExportRegions 失败: {e}"))?;
        Ok(Self { by_code })
    }

    fn get(&self, code: &str) -> Option<&RegionMeta> {
        self.by_code.get(code)
    }
}

/// FC_* / MT_* → 语言 path
struct NameKeyIndex {
    by_code: HashMap<String, String>,
}

impl NameKeyIndex {
    fn from_bytes(bytes: &[u8], label: &str) -> Result<Self, String> {
        let raw: HashMap<String, NamedExport> = serde_json::from_slice(bytes)
            .map_err(|e| format!("解析 {label} 失败: {e}"))?;
        let mut by_code = HashMap::with_capacity(raw.len());
        for (code, meta) in raw {
            if let Some(name) = meta.name.filter(|s| !s.is_empty()) {
                by_code.insert(code, name);
            }
        }
        Ok(Self { by_code })
    }

    fn name_key(&self, code: &str) -> Option<&str> {
        self.by_code.get(code).map(|s| s.as_str())
    }
}

struct ResolveCtx<'a> {
    regions: &'a RegionsIndex,
    factions: &'a NameKeyIndex,
    mission_types: &'a NameKeyIndex,
    dict: Option<&'a DictZh>,
}

/// 拉取日程正文（调用方可并行）。
pub(crate) fn fetch_schedule_bytes(client: &Client) -> Result<Vec<u8>, String> {
    download_bytes(client, SP_INCURSIONS_URL)
}

/// 九重天风暴：worldState 无 MissionType。
/// 每次拉取都用 ExportRegions.missionName → dict.zh 重写（保证译名随词典正确）。
pub(crate) fn fill_storm_fissure_mission_types(data_dir: &Path, fissures: &mut [FissureApi]) {
    let need = fissures
        .iter()
        .any(|f| f.kind == FISSURE_KIND_STORM && !f.node_code.is_empty());
    if !need {
        return;
    }
    let Some(regions) = ensure_regions(data_dir) else {
        eprintln!("[仓鼠] 九重天裂隙缺 ExportRegions，任务类型暂空");
        return;
    };
    let dict = challenge_i18n::ensure_dict_zh(data_dir);
    if dict.is_none() {
        eprintln!("[仓鼠] 九重天裂隙缺 dict.zh，任务类型走路径回退");
    }
    for f in fissures.iter_mut() {
        if f.kind != FISSURE_KIND_STORM || f.node_code.is_empty() {
            continue;
        }
        if let Some(meta) = regions.get(&f.node_code) {
            let zh = railjack_mission_type_zh(meta, dict.as_ref());
            if !zh.is_empty() {
                f.mission_type = zh;
            }
        }
    }
}

/// 优先 missionName 词典；否则从关卡路径回退（文案对齐 dict.zh）。
fn railjack_mission_type_zh(meta: &RegionMeta, dict: Option<&DictZh>) -> String {
    if let Some(s) = lang_or(dict, meta.mission_name.as_deref()) {
        return s;
    }
    railjack_mission_type_from_paths(
        meta.level_override.as_deref().unwrap_or(""),
        meta.enemy_spec.as_deref().unwrap_or(""),
    )
}

fn railjack_mission_type_from_paths(level_override: &str, enemy_spec: &str) -> String {
    let s = format!("{level_override} {enemy_spec}").to_ascii_lowercase();
    // 与 dict.zh MissionName_Railjack* 对齐
    if s.contains("orphix") {
        return "奥影".into();
    }
    if s.contains("volatile") {
        return "爆发".into();
    }
    if s.contains("survival") {
        return "生存".into();
    }
    if s.contains("defense") && !s.contains("mobiledefense") && !s.contains("mobile_defense") {
        return "防御".into();
    }
    if s.contains("spy") {
        return "间谍".into();
    }
    if s.contains("assassinate") || s.contains("assassination") {
        return "刺杀".into();
    }
    if s.contains("skirmish") {
        return "前哨战".into();
    }
    // Combined + Exterminate(+POI) → MissionName_Railjack「前哨战」
    if s.contains("exterminate") && (s.contains("poi") || s.contains("combined")) {
        return "前哨战".into();
    }
    if s.contains("exterminate") {
        return "歼灭".into();
    }
    if s.contains("sabotage") {
        return "破坏".into();
    }
    if s.contains("hijack") {
        return "劫持".into();
    }
    String::new()
}

/// 用日程 + 导出表 + dict.zh 组装侵袭；失败则回退 UTC 日窗（无节点）。
pub(crate) fn build_incursions(data_dir: &Path, schedule: Option<&[u8]>) -> SteelPathApi {
    match build_incursions_inner(data_dir, schedule) {
        Ok(api) => api,
        Err(e) => {
            eprintln!("[仓鼠] 钢铁侵袭日程解析失败（降级日窗）: {e}");
            synthesize_steel_incursion()
        }
    }
}

fn build_incursions_inner(
    data_dir: &Path,
    schedule: Option<&[u8]>,
) -> Result<SteelPathApi, String> {
    let body = match schedule {
        Some(b) if !b.is_empty() => {
            let _ = upstream_cache::save(data_dir, SP_INCURSIONS_FILE, b);
            b.to_vec()
        }
        _ => upstream_cache::read(data_dir, SP_INCURSIONS_FILE)?
            .ok_or_else(|| "无钢铁侵袭日程".to_string())?,
    };

    let regions = ensure_regions(data_dir)
        .ok_or_else(|| "ExportRegions 不可用".to_string())?;
    let factions = ensure_name_index(
        data_dir,
        EXPORT_FACTIONS_FILE,
        EXPORT_FACTIONS_URL,
        "ExportFactions",
    )
    .unwrap_or_default();
    let mission_types = ensure_name_index(
        data_dir,
        EXPORT_MISSION_TYPES_FILE,
        EXPORT_MISSION_TYPES_URL,
        "ExportMissionTypes",
    )
    .unwrap_or_default();
    let dict = challenge_i18n::ensure_dict_zh(data_dir);
    if dict.is_none() {
        eprintln!("[仓鼠] dict.zh 不可用：钢铁侵袭将用英文/回退名");
    }

    let text = String::from_utf8_lossy(&body);
    let by_day = parse_schedule(&text)?;
    let day_start = Utc::now().date_naive();
    let start_naive = day_start.and_hms_opt(0, 0, 0).unwrap();
    let today_ts = Utc.from_utc_datetime(&start_naive).timestamp();
    let tomorrow_ts = today_ts + 86_400;

    let today_codes = by_day
        .get(&today_ts)
        .cloned()
        .ok_or_else(|| format!("日程缺少今日 {today_ts}"))?;
    let tomorrow_codes = by_day.get(&tomorrow_ts).cloned().unwrap_or_default();

    let ctx = ResolveCtx {
        regions: &regions,
        factions: &factions,
        mission_types: &mission_types,
        dict: dict.as_ref(),
    };
    let today = resolve_missions(&today_codes, &ctx);
    let tomorrow = resolve_missions(&tomorrow_codes, &ctx);

    let end = day_start.and_hms_opt(23, 59, 59).unwrap();
    Ok(SteelPathApi {
        incursions: Some(SteelIncursionApi {
            activation: Some(Utc.from_utc_datetime(&start_naive).to_rfc3339()),
            expiry: Some(Utc.from_utc_datetime(&end).to_rfc3339()),
            today,
            tomorrow,
        }),
    })
}

fn parse_schedule(text: &str) -> Result<HashMap<i64, Vec<String>>, String> {
    let mut map = HashMap::new();
    for (line_no, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((ts_s, nodes_s)) = line.split_once(';') else {
            return Err(format!("日程第 {} 行格式错误", line_no + 1));
        };
        let ts: i64 = ts_s
            .parse()
            .map_err(|_| format!("日程第 {} 行时间戳无效", line_no + 1))?;
        let nodes: Vec<String> = nodes_s
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !nodes.is_empty() {
            map.insert(ts, nodes);
        }
    }
    if map.is_empty() {
        return Err("日程为空".into());
    }
    Ok(map)
}

fn resolve_missions(codes: &[String], ctx: &ResolveCtx<'_>) -> Vec<SteelIncursionMissionApi> {
    codes.iter().map(|code| resolve_one(code, ctx)).collect()
}

fn resolve_one(code: &str, ctx: &ResolveCtx<'_>) -> SteelIncursionMissionApi {
    let d = resolve_display(code, ctx);
    let min_base = ctx
        .regions
        .get(code)
        .and_then(|m| m.min_enemy_level)
        .unwrap_or(0);
    let max_base = ctx
        .regions
        .get(code)
        .and_then(|m| m.max_enemy_level)
        .unwrap_or(min_base);
    SteelIncursionMissionApi {
        mission_type: d.mission_type,
        faction: d.faction,
        min_level: 100 + min_base,
        max_level: 100 + max_base,
        node: d.node,
    }
}

/// 节点中文展示（任务类型 / 阵营 / 地点），供仲裁等复用。不含钢铁等级加成。
pub(crate) struct NodeDisplayZh {
    pub mission_type: String,
    pub faction: String,
    pub node: String,
}

pub(crate) struct NodeResolver {
    regions: RegionsIndex,
    factions: NameKeyIndex,
    mission_types: NameKeyIndex,
    dict: Option<DictZh>,
}

impl NodeResolver {
    pub(crate) fn load(data_dir: &Path) -> Option<Self> {
        let regions = ensure_regions(data_dir)?;
        let factions = ensure_name_index(
            data_dir,
            EXPORT_FACTIONS_FILE,
            EXPORT_FACTIONS_URL,
            "ExportFactions",
        )
        .unwrap_or_default();
        let mission_types = ensure_name_index(
            data_dir,
            EXPORT_MISSION_TYPES_FILE,
            EXPORT_MISSION_TYPES_URL,
            "ExportMissionTypes",
        )
        .unwrap_or_default();
        let dict = challenge_i18n::ensure_dict_zh(data_dir);
        Some(Self {
            regions,
            factions,
            mission_types,
            dict,
        })
    }

    pub(crate) fn resolve(&self, code: &str) -> NodeDisplayZh {
        let ctx = ResolveCtx {
            regions: &self.regions,
            factions: &self.factions,
            mission_types: &self.mission_types,
            dict: self.dict.as_ref(),
        };
        resolve_display(code, &ctx)
    }
}

fn resolve_display(code: &str, ctx: &ResolveCtx<'_>) -> NodeDisplayZh {
    let Some(meta) = ctx.regions.get(code) else {
        return NodeDisplayZh {
            mission_type: String::new(),
            faction: String::new(),
            node: localize::node_name(code),
        };
    };

    let mission_type = translate_mission(meta, ctx);
    let hide_faction = meta.system_index == Some(ZARIMAN_SYSTEM_INDEX);
    let faction = if hide_faction {
        String::new()
    } else {
        meta.faction
            .as_deref()
            .map(|fc| translate_faction(fc, ctx))
            .unwrap_or_default()
    };

    let place = lang_or(ctx.dict, meta.name.as_deref())
        .or_else(|| localize::sol_node_place_name(code))
        .unwrap_or_else(|| code.to_string());

    let planet = lang_or(ctx.dict, meta.system_name.as_deref()).unwrap_or_else(|| {
        let full = localize::node_name(code);
        full.split(" · ").nth(1).unwrap_or("").to_string()
    });

    let node = if planet.is_empty() {
        place
    } else if place.is_empty() {
        planet
    } else {
        format!("{place} · {planet}")
    };

    NodeDisplayZh {
        mission_type,
        faction,
        node,
    }
}

fn translate_mission(meta: &RegionMeta, ctx: &ResolveCtx<'_>) -> String {
    if let Some(s) = lang_or(ctx.dict, meta.mission_name.as_deref()) {
        return s;
    }
    if let Some(mt) = meta.mission_type.as_deref() {
        if let Some(key) = ctx.mission_types.name_key(mt) {
            if let Some(s) = lang_or(ctx.dict, Some(key)) {
                return s;
            }
        }
        return localize::mission_type(mt);
    }
    String::new()
}

fn translate_faction(fc: &str, ctx: &ResolveCtx<'_>) -> String {
    if let Some(key) = ctx.factions.name_key(fc) {
        if let Some(s) = lang_or(ctx.dict, Some(key)) {
            return s;
        }
    }
    fc.trim_start_matches("FC_").to_string()
}

fn lang_or(dict: Option<&DictZh>, key: Option<&str>) -> Option<String> {
    let key = key.filter(|k| !k.is_empty())?;
    dict.and_then(|d| d.lang(key).map(|s| s.to_string()))
}

fn ensure_regions(data_dir: &Path) -> Option<RegionsIndex> {
    ensure_cached_json(
        data_dir,
        EXPORT_REGIONS_FILE,
        EXPORT_REGIONS_URL,
        "ExportRegions",
        RegionsIndex::from_bytes,
    )
}

fn ensure_name_index(
    data_dir: &Path,
    file: &str,
    url: &str,
    label: &str,
) -> Option<NameKeyIndex> {
    ensure_cached_json(data_dir, file, url, label, |b| {
        NameKeyIndex::from_bytes(b, label)
    })
}

fn ensure_cached_json<T>(
    data_dir: &Path,
    file: &str,
    url: &str,
    label: &str,
    parse: impl Fn(&[u8]) -> Result<T, String>,
) -> Option<T> {
    if export_stale(data_dir, file) {
        if let Err(e) = refresh_cached_json(data_dir, file, url, label, &parse) {
            eprintln!("[仓鼠] {label} 刷新失败（将用缓存）: {e}");
        }
    }
    match upstream_cache::read(data_dir, file) {
        Ok(Some(bytes)) => match parse(&bytes) {
            Ok(v) => Some(v),
            Err(e) => {
                eprintln!("[仓鼠] {label} 加载失败: {e}");
                None
            }
        },
        Ok(None) => None,
        Err(e) => {
            eprintln!("[仓鼠] {label} 读取失败: {e}");
            None
        }
    }
}

fn refresh_cached_json<T>(
    data_dir: &Path,
    file: &str,
    url: &str,
    label: &str,
    parse: &impl Fn(&[u8]) -> Result<T, String>,
) -> Result<(), String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let bytes = download_bytes(&client, url)?;
    let _ = parse(&bytes).map_err(|e| format!("{label}: {e}"))?;
    upstream_cache::save(data_dir, file, &bytes)?;
    Ok(())
}

fn export_stale(data_dir: &Path, file: &str) -> bool {
    let path = crate::db::upstream_dir(data_dir).join(file);
    if !path.is_file() {
        return true;
    }
    let Ok(meta) = fs::metadata(&path) else {
        return true;
    };
    let Ok(modified) = meta.modified() else {
        return true;
    };
    let Ok(age) = SystemTime::now().duration_since(modified) else {
        return true;
    };
    age > EXPORT_MAX_AGE
}

fn download_bytes(client: &Client, url: &str) -> Result<Vec<u8>, String> {
    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("请求 {url} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("请求 {url} HTTP {}", resp.status()));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| format!("读 {url} 响应失败: {e}"))
}

impl Default for NameKeyIndex {
    fn default() -> Self {
        Self {
            by_code: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_days() {
        let txt = "1791504000;SolNode11,SettlementNode14,SolNode42\n1791590400;SolNode99,SolNode707,SolNode404\n";
        let map = parse_schedule(txt).unwrap();
        assert_eq!(map[&1791504000].len(), 3);
        assert_eq!(map[&1791590400][0], "SolNode99");
    }

    #[test]
    fn resolve_uses_export_tables_and_dict() {
        let regions = RegionsIndex::from_bytes(
            r#"{
          "SolNode11": {
            "name": "/Lotus/Language/Locations/Tharsis",
            "systemName": "/Lotus/Language/Locations/Mars",
            "systemIndex": 3,
            "missionType": "MT_MOBILE_DEFENSE",
            "missionName": "/Lotus/Language/Missions/MissionName_MobileDefense",
            "faction": "FC_GRINEER",
            "minEnemyLevel": 9,
            "maxEnemyLevel": 11
          },
          "SolNode230": {
            "name": "/Lotus/Language/Zariman/ZarimanNodeA",
            "systemName": "/Lotus/Language/Zariman/ZarimanRegionName",
            "systemIndex": 21,
            "missionType": "MT_CORRUPTION",
            "faction": "FC_GRINEER",
            "minEnemyLevel": 50,
            "maxEnemyLevel": 55
          },
          "SolNodeX": {
            "name": "/Lotus/Language/Locations/NewNode",
            "systemName": "/Lotus/Language/Locations/Mars",
            "systemIndex": 3,
            "missionType": "MT_PAINT_FLOOD",
            "faction": "FC_TECHROT",
            "minEnemyLevel": 20,
            "maxEnemyLevel": 25
          }
        }"#
            .as_bytes(),
        )
        .unwrap();
        let factions = NameKeyIndex::from_bytes(
            r#"{
          "FC_GRINEER": { "name": "/Lotus/Language/Game/Faction_GrineerUC" },
          "FC_TECHROT": { "name": "/Lotus/Language/1999/Faction_Techrot" }
        }"#
            .as_bytes(),
            "ExportFactions",
        )
        .unwrap();
        let mission_types = NameKeyIndex::from_bytes(
            r#"{
          "MT_CORRUPTION": { "name": "/Lotus/Language/Missions/MissionName_Corruption" },
          "MT_PAINT_FLOOD": { "name": "/Lotus/Language/Missions/MissionName_PaintFlood" }
        }"#
            .as_bytes(),
            "ExportMissionTypes",
        )
        .unwrap();
        let dict = DictZh::from_bytes(
            r#"{
          "/Lotus/Language/Locations/Tharsis": "Tharsis",
          "/Lotus/Language/Locations/Mars": "火星",
          "/Lotus/Language/Missions/MissionName_MobileDefense": "移动防御",
          "/Lotus/Language/Game/Faction_GrineerUC": "Grineer",
          "/Lotus/Language/Zariman/ZarimanNodeA": "永视弧域",
          "/Lotus/Language/Zariman/ZarimanRegionName": "扎里曼号",
          "/Lotus/Language/Missions/MissionName_Corruption": "虚空洪流",
          "/Lotus/Language/Locations/NewNode": "新节点",
          "/Lotus/Language/Missions/MissionName_PaintFlood": "彩绘洪流",
          "/Lotus/Language/1999/Faction_Techrot": "技腐"
        }"#
            .as_bytes(),
        )
        .unwrap();

        let ctx = ResolveCtx {
            regions: &regions,
            factions: &factions,
            mission_types: &mission_types,
            dict: Some(&dict),
        };

        let m = resolve_one("SolNode11", &ctx);
        assert_eq!(m.mission_type, "移动防御");
        assert_eq!(m.faction, "Grineer");
        assert_eq!(m.min_level, 109);
        assert_eq!(m.node, "Tharsis · 火星");

        // missionName 缺失时走 ExportMissionTypes → dict
        let z = resolve_one("SolNode230", &ctx);
        assert_eq!(z.mission_type, "虚空洪流");
        assert!(z.faction.is_empty());
        assert_eq!(z.node, "永视弧域 · 扎里曼号");

        // 全新 MT_/FC_/地点：只要导出表 + dict 有就能翻
        let n = resolve_one("SolNodeX", &ctx);
        assert_eq!(n.mission_type, "彩绘洪流");
        assert_eq!(n.faction, "技腐");
        assert_eq!(n.node, "新节点 · 火星");
        assert_eq!(n.min_level, 120);
    }

    #[test]
    fn railjack_subtype_from_level_paths() {
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackEarthCombinedExterminate1PoiProc",
                "/Lotus/Types/Game/EnemySpecs/Railjack/GrineerCrewShipExterminate",
            ),
            "前哨战"
        );
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackVenusStanchionSurvival",
                "",
            ),
            "生存"
        );
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackNeptunePillarSpy",
                "",
            ),
            "间谍"
        );
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackVenusStanchionOrphix",
                "",
            ),
            "奥影"
        );
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackVenusStanchionVolatile",
                "",
            ),
            "爆发"
        );
        assert_eq!(
            railjack_mission_type_from_paths(
                "/Lotus/Levels/Railjack/Proc/RailJackVenusStanchionExterminate",
                "",
            ),
            "歼灭"
        );
    }

    #[test]
    fn railjack_prefers_mission_name_dict() {
        let meta = RegionMeta {
            name: None,
            system_name: None,
            system_index: None,
            mission_type: Some("MT_RAILJACK".into()),
            mission_name: Some("/Lotus/Language/Missions/MissionName_RailjackVolatile".into()),
            level_override: Some(
                "/Lotus/Levels/Railjack/Proc/RailJackVenusStanchionVolatile".into(),
            ),
            enemy_spec: None,
            faction: None,
            min_enemy_level: None,
            max_enemy_level: None,
        };
        let dict = DictZh::from_bytes(
            r#"{
          "/Lotus/Language/Missions/MissionName_RailjackVolatile": "爆发",
          "/Lotus/Language/Missions/MissionName_RailjackOrphix": "奥影"
        }"#
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(railjack_mission_type_zh(&meta, Some(&dict)), "爆发");
    }
}
