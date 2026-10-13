//! 科研词条：oracle.browse.wf Conquest Language 自动对照。
//!
//! Public Export 的 dict.zh/en 几乎不含 `/Lotus/Language/Conquest/`，
//! 故单独缓存 oracle 词典。有中文用中文；没有则英文原文；再没有则 humanize。

use std::path::Path;

use super::challenge_i18n::{self, strip_lotus_markup, DictZh};
use super::localize::humanize_camel_pub;
use super::types::LabeledTerm;

const LANG_PREFIX: &str = "/Lotus/Language/Conquest/";

/// oracle 含完整 Conquest 标题/说明（Public Export 没有）。
const ORACLE_DICT_ZH_URL: &str = "https://oracle.browse.wf/dicts/zh.json";
const ORACLE_DICT_EN_URL: &str = "https://oracle.browse.wf/dicts/en.json";
const ORACLE_DICT_ZH_FILE: &str = "oracle_dict.zh.json";
const ORACLE_DICT_EN_FILE: &str = "oracle_dict.en.json";

/// 科研本地化查表（zh 优先，en 回退）。
#[derive(Debug, Clone)]
pub struct ConquestLang {
    zh: DictZh,
    en: Option<DictZh>,
}

impl ConquestLang {
    pub fn new(zh: DictZh, en: Option<DictZh>) -> Self {
        Self { zh, en }
    }

    /// 从 data_dir 加载 oracle Conquest 词典；无 zh 则 None（调用方走纯英文兜底）。
    pub fn ensure(data_dir: &Path) -> Option<Self> {
        let zh = challenge_i18n::ensure_lang_dict(
            data_dir,
            ORACLE_DICT_ZH_FILE,
            ORACLE_DICT_ZH_URL,
            "oracle dict.zh",
        )?;
        let en = challenge_i18n::ensure_lang_dict(
            data_dir,
            ORACLE_DICT_EN_FILE,
            ORACLE_DICT_EN_URL,
            "oracle dict.en",
        );
        Some(Self::new(zh, en))
    }

    pub fn from_bytes(zh: &[u8], en: Option<&[u8]>) -> Result<Self, String> {
        let zh = DictZh::from_bytes(zh)?;
        let en = match en {
            Some(b) => Some(DictZh::from_bytes(b)?),
            None => None,
        };
        Ok(Self::new(zh, en))
    }

    pub fn deviation(&self, kind: &str, code: &str) -> LabeledTerm {
        let code = term_basename(code);
        let primary = if kind == "CT_HEX" {
            deviation_key("Hex", code)
        } else {
            deviation_key("Lab", code)
        };
        let secondary = if kind == "CT_HEX" {
            deviation_key("Lab", code)
        } else {
            deviation_key("Hex", code)
        };
        if let Some(t) = self.lookup_pair(&primary.0, &primary.1) {
            return t;
        }
        if let Some(t) = self.lookup_pair(&secondary.0, &secondary.1) {
            return t;
        }
        english_or_humanize(self.en.as_ref(), &primary.0, code)
    }

    pub fn risk(&self, code: &str) -> LabeledTerm {
        let code = term_basename(code);
        let (title_key, desc_key) = condition_key(code);
        self.lookup_pair(&title_key, &desc_key)
            .unwrap_or_else(|| english_or_humanize(self.en.as_ref(), &title_key, code))
    }

    pub fn personal_mod(&self, code: &str) -> LabeledTerm {
        let code = term_basename(code);
        let (title_key, desc_key) = personal_key(code);
        self.lookup_pair(&title_key, &desc_key)
            .unwrap_or_else(|| english_or_humanize(self.en.as_ref(), &title_key, code))
    }

    fn lookup_pair(&self, title_key: &str, desc_key: &str) -> Option<LabeledTerm> {
        let title_raw = self.zh.lang(title_key)?;
        let title = strip_lotus_markup(title_raw);
        if title.is_empty() {
            return None;
        }
        let description = self
            .zh
            .lang(desc_key)
            .map(strip_lotus_markup)
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        Some(LabeledTerm { title, description })
    }
}

fn deviation_key(lab_or_hex: &str, code: &str) -> (String, String) {
    let base = format!("{LANG_PREFIX}MissionVariant_{lab_or_hex}Conquest_{code}");
    (base.clone(), format!("{base}_Desc"))
}

fn condition_key(code: &str) -> (String, String) {
    let base = format!("{LANG_PREFIX}Condition_{code}");
    (base.clone(), format!("{base}_Desc"))
}

fn personal_key(code: &str) -> (String, String) {
    let base = format!("{LANG_PREFIX}PersonalMod_{code}");
    (base.clone(), format!("{base}_Desc"))
}

fn english_or_humanize(en: Option<&DictZh>, title_key: &str, code: &str) -> LabeledTerm {
    if let Some(raw) = en.and_then(|d| d.lang(title_key)) {
        let title = strip_lotus_markup(raw);
        if !title.is_empty() {
            return LabeledTerm {
                title,
                description: String::new(),
            };
        }
    }
    LabeledTerm {
        title: humanize_camel_pub(code),
        description: String::new(),
    }
}

fn term_basename(code: &str) -> &str {
    code.rsplit('/').next().unwrap_or(code).trim()
}

/// 无词典时的纯英文/humanize 兜底（与有词典但未命中时一致）。
pub fn fallback_term(code: &str) -> LabeledTerm {
    LabeledTerm {
        title: humanize_camel_pub(term_basename(code)),
        description: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lang() -> ConquestLang {
        let zh = r#"{
          "/Lotus/Language/Conquest/MissionVariant_LabConquest_HostileSecurity": "侵蚀感官",
          "/Lotus/Language/Conquest/MissionVariant_LabConquest_HostileSecurity_Desc": "魔耳和魔瞳受损。",
          "/Lotus/Language/Conquest/MissionVariant_HexConquest_HostileSecurity": "破坏之音",
          "/Lotus/Language/Conquest/MissionVariant_HexConquest_HostileSecurity_Desc": "弗莱尔干扰力场。",
          "/Lotus/Language/Conquest/MissionVariant_LabConquest_GrowingIncursion": "裂缝激增",
          "/Lotus/Language/Conquest/MissionVariant_LabConquest_GrowingIncursion_Desc": "裂缝提升敌人等级。",
          "/Lotus/Language/Conquest/Condition_PointBlank": "短视弹药",
          "/Lotus/Language/Conquest/Condition_PointBlank_Desc": "需在 15 米内才能造成伤害。",
          "/Lotus/Language/Conquest/Condition_MagneticHounds": "迷人弧犬",
          "/Lotus/Language/Conquest/Condition_MagneticHounds_Desc": "弧犬拉扯战甲。",
          "/Lotus/Language/Conquest/PersonalMod_Starvation": "弹药亏空",
          "/Lotus/Language/Conquest/PersonalMod_Starvation_Desc": "弹药恢复减少 75%。",
          "/Lotus/Language/Conquest/PersonalMod_VitalEnergy": "震荡耗能",
          "/Lotus/Language/Conquest/PersonalMod_VitalEnergy_Desc": "受伤耗能。"
        }"#;
        let en = r#"{
          "/Lotus/Language/Conquest/Condition_EMPBlackHole": "Emp Black Hole",
          "/Lotus/Language/Conquest/PersonalMod_ChannelDrain": "Channeled Greed",
          "/Lotus/Language/Conquest/Condition_PointBlank": "Myopic Munitions"
        }"#;
        ConquestLang::from_bytes(zh.as_bytes(), Some(en.as_bytes())).unwrap()
    }

    #[test]
    fn zh_hit_and_kind_fork() {
        let lang = sample_lang();
        let lab = lang.deviation("CT_LAB", "HostileSecurity");
        assert!(lab.title.contains("侵蚀"), "{lab:?}");
        assert!(!lab.description.is_empty());
        let hex = lang.deviation("CT_HEX", "HostileSecurity");
        assert!(hex.title.contains("破坏"), "{hex:?}");
        assert!(lang.risk("PointBlank").title.contains("短视"));
        assert!(lang.personal_mod("Starvation").title.contains("弹药"));
        assert!(lang.personal_mod("VitalEnergy").title.contains("震荡"));
    }

    #[test]
    fn missing_zh_falls_back_to_en() {
        let lang = sample_lang();
        let emp = lang.risk("EMPBlackHole");
        assert_eq!(emp.title, "Emp Black Hole");
        assert!(emp.description.is_empty());
        let ch = lang.personal_mod("ChannelDrain");
        assert_eq!(ch.title, "Channeled Greed");
    }

    #[test]
    fn unknown_code_humanizes() {
        let lang = sample_lang();
        let u = lang.risk("TotallyFakeRisk");
        assert!(u.title.contains("Totally") || u.title.contains("Fake"), "{u:?}");
        assert!(u.description.is_empty());
    }
}
