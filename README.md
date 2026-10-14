# 仓鼠小助手

Warframe 收藏管理桌面工具（Tauri 2 + Vue 3 + TypeScript + SQLite）。

本仓库以 [MIT License](./LICENSE) 开源。

## 环境

| 组件 | 说明 |
|------|------|
| Node 20 + npm | 建议 Node 20 LTS |
| Rust (rustup) | `tauri dev` / 打包必需 |
| MSVC C++ Build Tools | Windows 链接报错时再装 |

装 Rust：https://rustup.rs 或 `winget install Rustlang.Rustup`  
装完新开终端检查：`rustc --version` / `cargo --version`

前置说明：[Tauri prerequisites](https://tauri.app/start/prerequisites/)

## 常用命令

```powershell
npm install
npm run tauri dev      # 开发
npm run build          # 仅前端
npm run tauri -- build --bundles nsis   # Windows 安装包
```

安装包 / 安装目录名为英文 `Hamster Assistant`（避免 CI 丢中文文件名）；开始菜单、桌面快捷方式与「应用和功能」通过自定义 NSIS 模板显示「仓鼠小助手」，窗口标题仍为中文。

## 数据库

启动时若 AppData 无库，从安装包种子拷贝 `catalog.sqlite` / `user.sqlite`；版本低则升级。

| 资源 | 用途 |
|------|------|
| `src-tauri/resources/catalog.sqlite` | 目录种子（战甲/武器/Mod/赋能/遗物及来源等） |
| `src-tauri/resources/user.sqlite` | 空进度种子 |
| `src-tauri/resources/userdict.json` | 中英对照 → `name_i18n` |
| `src-tauri/resources/warframe_nicknames.json` | 战甲俗称 |

Windows 数据目录：`%APPDATA%\hamster-assistant\`

- AppData 中的库可按页「更新数据」从 WFCD 等增量刷新
- 对照在 init 时若表空则从 `userdict.json` 导入；俗称在更新战甲后写入

### 维护者可选本地灌库

`resources/` 下另有上游 WFCD JSON（如 `Relics.json`、`drop-all.json`），供 `seed_catalog` 重建种子库；见 [`tools/README.md`](./tools/README.md)。

SQL：`src-tauri/sql/` · 逻辑：`src-tauri/src/catalog/`、`db.rs`、`id.rs`
