# Fork 規格（Canonical Spec）

本文件是 `fun-ed/TokenUsageInsights` 的正式產品與維運規格。README、`AGENTS.md`、專案 skill 與 Serena memory 都以本文件為準。upstream `doggy8088/TokenUsageInsights` 只是可選擇性移植的程式來源，不是規格來源；兩者衝突時一律以本文件為準。

## 1. 產品範圍

- 本機優先：只讀取本機 AI 助理資料，匯入 `~/.token-usage-insights/token_usage_insights.db`（可用 `INSIGHTS_DIR` 覆寫）。不上傳使用紀錄。
- 支援來源：Antigravity、Copilot（CLI、App、VS Code）、Codex、Claude Code、Cursor、Grok Build、Pi、OMP、Muse Code、MiniMax Code。
- 支援平台：**只有 macOS 與 Linux**。Windows 不在支援範圍。
- 發行：只有手動發行。沒有 CI/CD。

## 2. Fork 專屬功能（必須保留）

### 2.1 Claude Code 多 profile

- 來源：預設根目錄 `~/.claude`（或 `CLAUDE_DIR`），加上 `~/.claude-profiles/<name>/`，且該目錄底下要有 `projects/`。
- 每次 sync 都重新掃描 `~/.claude-profiles/*`。新增 profile 不需要改設定；沒有 `projects/` 的 profile（例如空的 `personal`）會略過，直到出現資料。
- 設定 `CLAUDE_DIR` 時只讀該目錄，不掃描 profiles。背景服務不可設定 `CLAUDE_DIR`，否則 profile 會停止匯入。
- 來源識別：`source_kind` 為 `claude-default` 或 `claude-profile:<name>`；每個 profile 保留獨立的 `source_dir_key` 與路徑安全的 transcript 查找。
- Web UI：session 列表與模型明細以 badge 顯示來源，`Default` 或 profile 名稱（`work`、`p2`、`personal`）。
- 實作位置：`src/db.rs` `get_claude_sources()`、`static/app.js` `getSessionSourceBadge()`。

### 2.2 總覽（all harness）

- 側欄 **總覽** 使用 pseudo-assistant `all`，合併所有助理與 profile 的日、月、年報表。唯讀。
- 總覽模式要保留每個 session 的助理與 profile badge，並依總 token 顯示 Harness 排名。

### 2.3 自動匯入

- 自動匯入要靠 server 常駐。server 啟動與執行期間會 sync 所有來源。
- macOS：`scripts/install.sh --service` 安裝 launchd agent `com.tokenusageinsights`（`RunAtLoad`、`KeepAlive`）。Linux：systemd user service。
- `HOST` 預設 `0.0.0.0`（區網可連）；只限本機時用 `HOST=127.0.0.1`。
- 不可在安裝目錄內直接執行 `install.sh`。它會先刪除 `static/`、`shell/`、`scripts/` 再複製；來源與目的相同時會刪掉資源。請從解壓縮的 release 或暫存副本執行。

### 2.4 資料保留

- 本程式只能匯入仍存在的 transcript。Claude Code 預設 `cleanupPeriodDays` 為 30 天，超過就刪除 transcript；`history.jsonl` 只有 prompt，沒有 token 用量，無法補算。
- 每個 Claude 根目錄（`~/.claude` 與每個 profile）的 `settings.json` 都應設定 `"cleanupPeriodDays": 365` 或更長。
- 已知缺口：2026-01 至 2026-07 的 Claude transcript 已被清除，DB 沒有這段資料。

## 3. 版本與發行

- Cargo 與 npm 版本為 `10.x.y`；release tag 為對應的 `v10.x.y`。不可重用 upstream 的 `v1.x.y`。
- 發行資產：macOS `aarch64-apple-darwin` 與 `x86_64-apple-darwin` 的 `.tar.gz`、`.dmg`，以及 `SHA256SUMS`。Linux 只有在 release 列出對應 tarball 時才宣稱可用。
- 發行包內容：執行檔、`static/`、`shell/`、`scripts/`、`pricing.csv`、`README.md`、`LICENSE`、`VERSION`。
- `scripts/get.sh` 與 npm 安裝器從 `fun-ed/TokenUsageInsights` 下載。
- 流程細節見 `.agents/skills/token-usage-insights/SKILL.md`。

## 4. Upstream 同步規則

每次從 upstream 移植後，都要依本節清除不符合本規格的內容，才能 commit。

### 4.1 移植方式

- 先在遠端檢視 upstream commit 與 diff，逐項挑選。不可整包 merge 或 rebase。
- 每個移植項目各自一個可審查的 commit，訊息註明 upstream commit。
- 衝突一律以本規格為準：本機優先、`v10.x.y`、多 profile、總覽、fork 下載來源。

### 4.2 必須移除或拒絕

| 類別 | 處理 |
|---|---|
| CI/CD | 不引入 `.github/workflows/`、release workflow、Dependabot、自動發佈設定。`.github/workflows/` 保持不存在或為空。 |
| Windows 腳本與安裝 | 不引入 `*.ps1`、`*.bat`、`*.cmd`、Windows installer、Windows service runner、Scheduled Task、MSI/zip 資產。 |
| Windows 程式碼 | 不引入新的 `#[cfg(windows)]`、`cfg!(windows)`、`target_os = "windows"` 分支、Windows 路徑處理或 Windows 專用測試。混合平台的變更只取 Linux/macOS 部分。 |
| Windows 文件 | 不引入 Windows 安裝說明、截圖或 README 段落。 |
| 其他語系 README | 只保留 `README.md`、`README.zh-TW.md`、`README.zh-CN.md`。 |
| 發行來源 | 不可把 `get.sh`、npm 安裝器或 README 連結改回 upstream。 |
| 版本號 | 不可採用 upstream 的 `1.x.y` 版本或 tag。 |

### 4.3 同步後檢查

```sh
git ls-files .github | rg workflows            # 應無輸出
git ls-files | rg -i '\.(ps1|bat|cmd)$'         # 應無輸出
git diff <base>..HEAD -- . ':!*.md' | rg '^\+' | rg -n -i 'cfg\(windows|cfg!\(windows|target_os = "windows"|powershell'   # 應無輸出
rg -n 'doggy8088' scripts npm README*.md package.json   # 應無輸出
```

另外要跑 `AGENTS.md` 規定的 Rust 與 npm 驗證。

## 5. 待清除的既有殘留

以下內容早於本規格就已存在，會保留到有專門的清理 commit 為止。清理時要一起補測試。

- `src/updater.rs`：`GITHUB_OWNER` 仍是 `doggy8088`，`update` 指令與背景 auto-update 會查詢 upstream release。因為 semver 比較（upstream `1.x` < `10.x`），目前不會被覆蓋，但更新來源應改成 `fun-ed` 或停用。檔內也有大量 Windows service runner 邏輯。
- Windows 分支：`src/browser.rs`、`src/paths.rs`、`src/main.rs`（Windows 服務 runner）、`src/handlers/daily.rs`、`src/db/cursor.rs`、`src/vscode.rs`、`src/db.rs`；`static/app.js` 的 Windows 路徑判斷。
