# 维护者工具（不随软件发布）

本目录与 `src-tauri` 的 `seed_catalog` 二进制仅供维护者**可选**本地灌库 / 调试，**不要打进安装包，也不要给普通用户入口**。发版不依赖此工具。

## 原则

- **运行时**：AppData 无库时从安装包拷贝种子 `catalog.sqlite` / `user.sqlite`；之后可用各页「更新数据」拉 WFCD。
- **安装包资源**：种子 sqlite + `userdict.json` / `warframe_nicknames.json`。
- **resources 上游 JSON**：供 `seed_catalog` 重建种子库；不必打进安装包。
- **seed_catalog**：维护者本地灌库工具，**不要**给普通用户入口。

## 可选：本地灌 catalog

在 `src-tauri` 下（可用本仓库已有 `resources/*.json`；输出建议到临时目录后再拷回 `resources/`）：

```powershell
cargo run --bin seed_catalog -- .\seed_out
```

会写出：

- `seed_out/catalog.sqlite`
- `seed_out/user.sqlite`（空进度）

对照用 `resources/userdict.json`；上游 JSON 缺省时读 `resources/`。

仅调试需要时：`HAMSTER_FETCH_HUIJI_I18N=1`，才会尝试联网拉灰机（易被 Cloudflare 拦，不推荐）。

## 验证（发版路径）

- 清空 `%APPDATA%\hamster-assistant\` 后启动 → 应拷贝种子库，首页/列表有目录数据。
- 不要随意删已有 AppData `user.sqlite`（进度在里面）。
- 运行时不请求灰机。
