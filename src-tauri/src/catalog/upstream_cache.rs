//! 上游 JSON 本地落盘（原子写）。

use std::fs;
use std::path::{Path, PathBuf};

use crate::db;

pub const WARFRAMES: &str = "Warframes.json";
pub const ARCHWING: &str = "Archwing.json";
pub const SENTINELS: &str = "Sentinels.json";
pub const PETS: &str = "Pets.json";
pub const PRIMARY: &str = "Primary.json";
pub const SECONDARY: &str = "Secondary.json";
pub const MELEE: &str = "Melee.json";
pub const ARCH_GUN: &str = "Arch-Gun.json";
pub const ARCH_MELEE: &str = "Arch-Melee.json";
pub const MISC: &str = "Misc.json";
pub const ARCANES: &str = "Arcanes.json";
pub const MODS: &str = "Mods.json";
pub const RELICS: &str = "Relics.json";
pub const DROP_ALL: &str = "drop-all.json";
#[allow(dead_code)]
pub const I18N: &str = "i18n-zh.json";
pub const WORLDSTATE: &str = "worldstate.json";
pub const VAULT_TRADER: &str = "vaultTrader.json";

/// 确保 `{data_dir}/upstream` 存在，返回目录路径。
pub fn ensure_dir(data_dir: &Path) -> Result<PathBuf, String> {
    let dir = db::upstream_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("创建 upstream 目录失败: {e}"))?;
    Ok(dir)
}

/// 先写 `{filename}.tmp` 再 rename，覆盖即删旧。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("无效路径: {}", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("无效文件名: {}", path.display()))?;
    let tmp = parent.join(format!("{file_name}.tmp"));
    fs::write(&tmp, bytes).map_err(|e| format!("写临时文件失败 {}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!(
            "原子替换失败 {} → {}: {e}",
            tmp.display(),
            path.display()
        )
    })?;
    Ok(())
}

/// 写入 `{data_dir}/upstream/{filename}`。
pub fn save(data_dir: &Path, filename: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = ensure_dir(data_dir)?;
    let path = dir.join(filename);
    write_atomic(&path, bytes)?;
    Ok(path)
}

/// 读取已落盘的上游文件；不存在则 `Ok(None)`。
pub fn read(data_dir: &Path, filename: &str) -> Result<Option<Vec<u8>>, String> {
    let path = db::upstream_dir(data_dir).join(filename);
    if !path.is_file() {
        return Ok(None);
    }
    fs::read(&path)
        .map(Some)
        .map_err(|e| format!("读 {} 失败: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces() {
        let dir = std::env::temp_dir().join(format!(
            "cangshu_upstream_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Warframes.json");
        write_atomic(&path, b"one").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"one");
        write_atomic(&path, b"two").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"two");
        assert!(!dir.join("Warframes.json.tmp").exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
