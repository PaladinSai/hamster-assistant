//! # Modular 精通部件（自制计精通行）
//!
//! 游戏里「拼装后计精通」的关键部件，目录各占一行家族；武器侧与同伴侧**对称**入库：
//!
//! | 侧 | 关键部件 | 标记 | 解析 | 入库 |
//! |----|----------|------|------|------|
//! | 武器 | Kitgun 枪膛 / Zaw 击打部 / Amp 棱镜 | `weapon_family.modular_kind` | [`crate::catalog::weapons`] | `import_modular_weapons_json` |
//! | 同伴 | 恐鸟头 / 猎犬头 | `warframe_family.companion_type` = `moa` \| `hound` | [`crate::catalog::wfcd`] `parse_modular_companions` | [`crate::catalog::import`] `import_modular_companions_json` |
//!
//! **共同点**
//! - 不入库完整组装，只收「计精通」那一件
//! - 一条家族 ≈ 一个英文名 / 一个 uniqueName
//! - 简中：`name_i18n`（[`ZhLookup`]）→ 静态回退表（各侧自带）
//!
//! **常量**：[`crate::catalog::wfcd`] 的 `MODULAR_KIND_*`、`COMPANION_TYPE_MOA` / `HOUND`。
//! 本模块只放概念说明 + 共用小 helper，不新建子系统。

use crate::catalog::i18n::ZhLookup;

/// 自制精通部件简中：先精确查词典，再走静态回退。
pub(crate) fn resolve_name_zh(
    zh: &ZhLookup,
    name_en: &str,
    fallback: fn(&str) -> Option<&'static str>,
) -> Option<String> {
    zh.get(name_en)
        .or_else(|| fallback(name_en).map(str::to_string))
}
