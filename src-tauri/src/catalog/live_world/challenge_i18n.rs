//! 挑战中文：browse.wf ExportChallenges + dict.zh 本地缓存查表。
//!
//! 覆盖赏金挑战与午夜电波。电波条目通常只有 `name`，描述靠 `_Name` → `_Description`
//! 在 dict.zh 中推导。新挑战随官方导出更新，无需手改代码。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::catalog::upstream_cache;

use super::types::USER_AGENT;

const EXPORT_CHALLENGES_URL: &str =
    "https://browse.wf/warframe-public-export-plus/ExportChallenges.json";
pub(crate) const DICT_ZH_URL: &str =
    "https://browse.wf/warframe-public-export-plus/dict.zh.json";
pub(crate) const DICT_EN_URL: &str =
    "https://browse.wf/warframe-public-export-plus/dict.en.json";

pub(crate) const EXPORT_CHALLENGES_FILE: &str = "ExportChallenges.json";
pub(crate) const DICT_ZH_FILE: &str = "dict.zh.json";
pub(crate) const DICT_EN_FILE: &str = "dict.en.json";

/// 词典缓存最长有效期（超时则后台刷新；失败仍可用旧缓存）。
const MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

/// browse.wf `dict.zh`：语言 path → 中文；随官方导出更新，无需手改。
#[derive(Debug, Clone, Default)]
pub struct DictZh {
    map: HashMap<String, String>,
}

impl DictZh {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let map: HashMap<String, String> = serde_json::from_slice(bytes)
            .map_err(|e| format!("解析 dict.zh 失败: {e}"))?;
        Ok(Self { map })
    }

    pub fn lang(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|s| s.as_str())
    }

    fn into_map(self) -> HashMap<String, String> {
        self.map
    }
}

/// 确保 dict.zh 可用（可单独刷新，不依赖 ExportChallenges）。
pub fn ensure_dict_zh(data_dir: &Path) -> Option<DictZh> {
    ensure_lang_dict(data_dir, DICT_ZH_FILE, DICT_ZH_URL, "dict.zh")
}

/// 确保 dict.en 可用（科研词条缺中文时回退英文原文）。
pub fn ensure_dict_en(data_dir: &Path) -> Option<DictZh> {
    ensure_lang_dict(data_dir, DICT_EN_FILE, DICT_EN_URL, "dict.en")
}

/// 通用语言词典缓存：指定文件名与 URL（Public Export / oracle 等）。
pub(crate) fn ensure_lang_dict(
    data_dir: &Path,
    file: &str,
    url: &str,
    label: &str,
) -> Option<DictZh> {
    if file_stale(data_dir, file) {
        match refresh_lang_dict(data_dir, file, url) {
            Ok(()) => {}
            Err(e) => eprintln!("[仓鼠] {label} 刷新失败（将用缓存）: {e}"),
        }
    }
    match load_lang_dict_cache(data_dir, file) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[仓鼠] {label} 加载失败: {e}");
            None
        }
    }
}

fn load_lang_dict_cache(data_dir: &Path, file: &str) -> Result<Option<DictZh>, String> {
    let Some(bytes) = upstream_cache::read(data_dir, file)? else {
        return Ok(None);
    };
    Ok(Some(DictZh::from_bytes(&bytes)?))
}

fn refresh_lang_dict(data_dir: &Path, file: &str, url: &str) -> Result<(), String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let dict = download_bytes(&client, url)?;
    let _ = DictZh::from_bytes(&dict)?;
    upstream_cache::save(data_dir, file, &dict)?;
    Ok(())
}

fn file_stale(data_dir: &Path, name: &str) -> bool {
    let path = crate::db::upstream_dir(data_dir).join(name);
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
    age > MAX_AGE
}

#[derive(Debug, Deserialize)]
struct ChallengeMeta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default, rename = "requiredCount")]
    required_count: Option<i32>,
    #[serde(default)]
    standing: Option<i32>,
}

#[derive(Debug, Clone)]
struct ChallengeEntry {
    name_key: Option<String>,
    desc_key: Option<String>,
    required_count: Option<i32>,
    standing: Option<i32>,
}

/// 已加载的挑战本地化表（内存）。
#[derive(Debug, Default, Clone)]
pub struct ChallengeLocalizer {
    challenges: HashMap<String, ChallengeEntry>,
    /// 路径末段 → 完整 path（仅当末段唯一时写入，供路径目录差异回退）
    by_leaf: HashMap<String, String>,
    /// language path → zh
    dict_zh: HashMap<String, String>,
}

impl ChallengeLocalizer {
    /// 从已落盘的 upstream 文件加载；缺文件则 `Ok(None)`。
    pub fn load_from_cache(data_dir: &Path) -> Result<Option<Self>, String> {
        let Some(ch_bytes) = upstream_cache::read(data_dir, EXPORT_CHALLENGES_FILE)? else {
            return Ok(None);
        };
        let Some(dict_bytes) = upstream_cache::read(data_dir, DICT_ZH_FILE)? else {
            return Ok(None);
        };
        Ok(Some(Self::from_bytes(&ch_bytes, &dict_bytes)?))
    }

    pub fn from_bytes(challenges_json: &[u8], dict_json: &[u8]) -> Result<Self, String> {
        let raw: HashMap<String, ChallengeMeta> = serde_json::from_slice(challenges_json)
            .map_err(|e| format!("解析 ExportChallenges 失败: {e}"))?;
        let dict_zh = DictZh::from_bytes(dict_json)?.into_map();
        let mut challenges = HashMap::with_capacity(raw.len());
        let mut leaf_counts: HashMap<String, u32> = HashMap::new();
        for path in raw.keys() {
            if let Some(leaf) = path.rsplit('/').next() {
                *leaf_counts.entry(leaf.to_string()).or_insert(0) += 1;
            }
        }
        let mut by_leaf = HashMap::new();
        for (path, meta) in raw {
            let name = meta.name.filter(|s| !s.is_empty());
            let mut desc = meta.description.filter(|s| !s.is_empty());
            // 电波等：导出无 description，dict 里有同 stem 的 `_Description`
            if desc.is_none() {
                if let Some(ref nk) = name {
                    if let Some(stem) = nk.strip_suffix("_Name") {
                        desc = Some(format!("{stem}_Description"));
                    }
                }
            }
            if name.is_none() && desc.is_none() {
                continue;
            }
            if let Some(leaf) = path.rsplit('/').next() {
                if leaf_counts.get(leaf).copied().unwrap_or(0) == 1 {
                    by_leaf.insert(leaf.to_string(), path.clone());
                }
            }
            challenges.insert(
                path,
                ChallengeEntry {
                    name_key: name,
                    desc_key: desc,
                    required_count: meta.required_count,
                    standing: meta.standing,
                },
            );
        }
        Ok(Self {
            challenges,
            by_leaf,
            dict_zh,
        })
    }

    fn entry(&self, challenge_path: &str) -> Option<&ChallengeEntry> {
        if let Some(e) = self.challenges.get(challenge_path) {
            return Some(e);
        }
        let norm = challenge_path.replace("/StoreItems/", "/");
        if norm != challenge_path {
            if let Some(e) = self.challenges.get(&norm) {
                return Some(e);
            }
        }
        let leaf = challenge_path.rsplit('/').next()?;
        let full = self.by_leaf.get(leaf)?;
        self.challenges.get(full)
    }

    /// 挑战路径 → 官中标题；未命中则 None。
    pub fn challenge_title_zh(&self, challenge_path: &str) -> Option<String> {
        let entry = self.entry(challenge_path)?;
        let key = entry.name_key.as_ref()?;
        let raw = self.dict_zh.get(key)?;
        let s = strip_lotus_markup(raw);
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }

    /// 挑战路径 → 官中描述（已替换 COUNT、去掉 markup）；未命中则 None。
    pub fn challenge_desc_zh(&self, challenge_path: &str) -> Option<String> {
        let entry = self.entry(challenge_path)?;
        let desc_key = entry.desc_key.as_ref()?;
        let raw = self.dict_zh.get(desc_key)?;
        Some(format_challenge_desc(raw, entry.required_count))
    }

    /// 电波声望（standing）；未命中则 None。
    pub fn challenge_standing(&self, challenge_path: &str) -> Option<i32> {
        self.entry(challenge_path).and_then(|e| e.standing)
    }

    /// 电波标题+描述；至少标题命中才返回。
    pub fn nightwave_zh(&self, challenge_path: &str) -> Option<(String, String)> {
        let title = self.challenge_title_zh(challenge_path)?;
        let desc = self.challenge_desc_zh(challenge_path).unwrap_or_default();
        Some((title, desc))
    }

    /// 六人组日历：挑战走 ExportChallenges；礼包走 `/Lotus/Language/1999/{base}Name|Desc`。
    pub fn calendar_zh(&self, path: &str) -> Option<(String, String)> {
        if path.contains("/Challenges/") || path.contains("Calendar1999") {
            return self.nightwave_zh(path);
        }
        let base = path.rsplit('/').next()?;
        // 仅 Calendar* 礼包键；普通物品仍走 ZhLookup
        if !base.starts_with("Calendar") {
            return None;
        }
        let name_key = format!("/Lotus/Language/1999/{base}Name");
        let raw = self.dict_zh.get(&name_key)?;
        let title = strip_lotus_markup(raw);
        if title.is_empty() {
            return None;
        }
        let desc_key = format!("/Lotus/Language/1999/{base}Desc");
        let description = self
            .dict_zh
            .get(&desc_key)
            .map(|s| strip_lotus_markup(s))
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        Some((title, description))
    }
}

/// 确保缓存可用：过期或不存在则拉取；失败时尽量返回旧缓存。
pub fn ensure_loaded(data_dir: &Path) -> Option<ChallengeLocalizer> {
    let need_refresh = cache_stale(data_dir);
    if need_refresh {
        match refresh_cache(data_dir) {
            Ok(()) => {}
            Err(e) => eprintln!("[仓鼠] 挑战词典刷新失败（将用缓存/回退）: {e}"),
        }
    }
    match ChallengeLocalizer::load_from_cache(data_dir) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[仓鼠] 挑战词典加载失败: {e}");
            None
        }
    }
}

fn cache_stale(data_dir: &Path) -> bool {
    file_stale(data_dir, EXPORT_CHALLENGES_FILE) || file_stale(data_dir, DICT_ZH_FILE)
}

fn refresh_cache(data_dir: &Path) -> Result<(), String> {
    let client = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;

    let ch_client = client.clone();
    let dict_client = client;
    let ch_handle =
        std::thread::spawn(move || download_bytes(&ch_client, EXPORT_CHALLENGES_URL));
    let dict_handle = std::thread::spawn(move || download_bytes(&dict_client, DICT_ZH_URL));

    let ch = ch_handle
        .join()
        .map_err(|_| "ExportChallenges 下载线程异常".to_string())??;
    let dict = dict_handle
        .join()
        .map_err(|_| "dict.zh 下载线程异常".to_string())??;

    // 先校验能解析再落盘
    let _ = ChallengeLocalizer::from_bytes(&ch, &dict)?;
    upstream_cache::save(data_dir, EXPORT_CHALLENGES_FILE, &ch)?;
    upstream_cache::save(data_dir, DICT_ZH_FILE, &dict)?;
    Ok(())
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

fn format_challenge_desc(raw: &str, count: Option<i32>) -> String {
    let mut s = raw.to_string();
    if let Some(c) = count {
        s = s.replace("|COUNT|", &c.to_string());
    }
    // 六人组等：|OPEN_COLOR||ALLY|的赏金任务|CLOSE_COLOR| 整段去掉
    s = strip_ally_bounty_preamble(&s);

    let mut lines: Vec<String> = s
        .lines()
        .map(|line| strip_lotus_markup(line.trim()))
        .filter(|l| !l.is_empty() && l != "的赏金任务")
        .collect();
    if lines.is_empty() {
        return strip_lotus_markup(&s);
    }
    // 多行时目标在最后一行
    if lines.len() > 1 {
        lines.pop().unwrap_or_default()
    } else {
        lines.remove(0)
    }
}

/// 去掉 `|OPEN_COLOR|…|ALLY|…的赏金任务…|CLOSE_COLOR|` 这类前缀块。
fn strip_ally_bounty_preamble(s: &str) -> String {
    const MARKER: &str = "的赏金任务";
    let Some(start) = s.find("|OPEN_COLOR|") else {
        return s.to_string();
    };
    let Some(marker_at) = s[start..].find(MARKER).map(|i| start + i) else {
        return s.to_string();
    };
    let after_marker = marker_at + MARKER.len();
    let rest = &s[after_marker..];
    let Some(close_rel) = rest.find("|CLOSE_COLOR|") else {
        return s.to_string();
    };
    let end = after_marker + close_rel + "|CLOSE_COLOR|".len();
    format!("{}{}", &s[..start], &s[end..])
}

/// 去掉 `|TAG|`、`<DT_POISON>` 等标记与多余空白。
pub(crate) fn strip_lotus_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '|' {
            while let Some(n) = chars.next() {
                if n == '|' {
                    break;
                }
            }
            continue;
        }
        // `<DT_POISON>` / `<AIM_WEAPON>`：无空格、非嵌套的尖括号片段
        if c == '<' {
            let mut inner = String::new();
            let mut closed = false;
            while let Some(&n) = chars.peek() {
                if n == '>' {
                    chars.next();
                    closed = true;
                    break;
                }
                if n == '<' || n.is_whitespace() {
                    break;
                }
                inner.push(n);
                chars.next();
            }
            if closed && !inner.is_empty() {
                continue;
            }
            // 不是完整标签：保留已消费字符
            out.push('<');
            out.push_str(&inner);
            continue;
        }
        if c == '\r' {
            continue;
        }
        out.push(c);
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_and_count() {
        let raw = "|OPEN_COLOR||ALLY|的赏金任务|CLOSE_COLOR|\n摧毁 |COUNT| 个容器";
        assert_eq!(format_challenge_desc(raw, Some(2)), "摧毁 2 个容器");
    }

    #[test]
    fn strip_damage_type_angle_tags() {
        assert_eq!(
            format_challenge_desc(
                "使用<DT_POISON>毒素伤害击杀 |COUNT| 名敌人。",
                Some(150)
            ),
            "使用毒素伤害击杀 150 名敌人。"
        );
        assert_eq!(
            format_challenge_desc(
                "使用<DT_MAGNETIC>磁力伤害击杀 |COUNT| 名敌人。",
                Some(150)
            ),
            "使用磁力伤害击杀 150 名敌人。"
        );
    }

    #[test]
    fn hex_ally_preamble_dropped() {
        let raw = "|OPEN_COLOR||ALLY|的赏金任务|CLOSE_COLOR|\r\n摧毁 |COUNT| 个炽蛇军艾弗旺容器";
        assert_eq!(
            format_challenge_desc(raw, Some(10)),
            "摧毁 10 个炽蛇军艾弗旺容器"
        );
    }

    #[test]
    fn resolve_from_mini_dict() {
        let ch = r#"{
          "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge": {
            "name": "/Lotus/Language/X/Name",
            "description": "/Lotus/Language/X/Desc",
            "requiredCount": 1
          }
        }"#;
        let dict = r#"{
          "/Lotus/Language/X/Desc": "击败嘲讽低语或尖刻低语",
          "/Lotus/Language/X/Name": "致命低语"
        }"#;
        let loc = ChallengeLocalizer::from_bytes(ch.as_bytes(), dict.as_bytes()).unwrap();
        assert_eq!(
            loc.challenge_desc_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge"
            )
            .as_deref(),
            Some("击败嘲讽低语或尖刻低语")
        );
        assert_eq!(
            loc.challenge_title_zh(
                "/Lotus/Types/Challenges/EntratiLab/EntratiLabDefeatDoppelgangerChallenge"
            )
            .as_deref(),
            Some("致命低语")
        );
        assert!(loc
            .challenge_desc_zh("/Lotus/Types/Challenges/Missing")
            .is_none());
    }

    #[test]
    fn nightwave_name_only_derives_description() {
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
        let loc = ChallengeLocalizer::from_bytes(ch.as_bytes(), dict.as_bytes()).unwrap();
        let (title, desc) = loc
            .nightwave_zh("/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyCodexScan")
            .unwrap();
        assert_eq!(title, "研究者");
        assert_eq!(desc, "扫描 15 次物件或敌人。");
        assert_eq!(
            loc.challenge_standing(
                "/Lotus/Types/Challenges/Seasons/Daily/SeasonDailyCodexScan"
            ),
            Some(1000)
        );
        // 路径目录差异：Weekly/ vs WeeklyHard/，靠末段唯一回退
        let (t2, d2) = loc
            .nightwave_zh(
                "/Lotus/Types/Challenges/Seasons/Weekly/SeasonWeeklyHardTerminated",
            )
            .unwrap();
        assert_eq!(t2, "已终止");
        assert_eq!(d2, "摧毁 3 个尸鬼金库守卫。");
    }

    #[test]
    fn calendar_challenge_and_pack_from_dict() {
        let ch = r#"{
          "/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy": {
            "name": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name",
            "description": "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description",
            "requiredCount": 10
          }
        }"#;
        let dict = r#"{
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Name": "逝去卓越者",
          "/Lotus/Language/1999Challenges/Challenge_CalendarKillEximus_Description": "击杀 |COUNT| 名卓越者",
          "/Lotus/Language/1999/CalendarArtifactPackName": "赋能助力",
          "/Lotus/Language/1999/CalendarArtifactPackDesc": "获得一枚随机赋能。"
        }"#;
        let loc = ChallengeLocalizer::from_bytes(ch.as_bytes(), dict.as_bytes()).unwrap();
        let (t, d) = loc
            .calendar_zh("/Lotus/Types/Challenges/Calendar1999/CalendarKillEximusEasy")
            .unwrap();
        assert_eq!(t, "逝去卓越者");
        assert!(d.contains("10"), "{d}");
        let (pt, pd) = loc
            .calendar_zh("/Lotus/Types/Items/MiscItems/CalendarArtifactPack")
            .unwrap();
        assert_eq!(pt, "赋能助力");
        assert!(pd.contains("赋能"), "{pd}");
        assert!(loc
            .calendar_zh("/Lotus/Upgrades/Calendar/AbilityStrength")
            .is_none());
    }
}
