# Token 戰情室

Token 戰情室是本機優先的 AI Coding Agent 用量看板。它會將本機記錄匯入 SQLite，顯示 Token、估算費用與 Session 時間軸。

支援 Antigravity、Copilot、Codex、Claude Code、Cursor、Grok Build、Pi、OMP、Muse Code 與 MiniMax Code；所有來源均維持本機讀取。

本 repo 是 `fun-ed` fork，功能以 [docs/fork-spec.md](docs/fork-spec.md) 為準，不以 upstream 為準。只支援 macOS 與 Linux，沒有 CI/CD，發行一律手動。英文 [README](README.md) 是正式說明。

Claude Code 會讀取 `~/.claude` 與每個含 `projects/` 的 `~/.claude-profiles/<name>/`，Web UI 以 `Default`、`work`、`p2` 等 badge 標示來源。要背景自動匯入，請用 release 內的 `scripts/install.sh --service` 安裝服務。Claude Code 預設 30 天刪除 transcript，請在各 profile 的 `settings.json` 設定 `"cleanupPeriodDays": 365`。

OMP 同時讀取 `~/.omp/agent/sessions` 與 `~/.omp/profiles/<name>/agent/sessions`，保留各 profile 的獨立來源。設定 `OMP_DIR` 時停用自動 profile 探索。

目前版本為 [v10.0.18](https://github.com/fun-ed/TokenUsageInsights/releases/tag/v10.0.18)，選擇性對齊 upstream v1.2.1，排除 Windows 與 CI/CD。Claude Code 依每回覆的 `usage.speed` 辨識 Fast Mode，使用 `-fast` 模型計價；同步 Opus 5.5、Sonnet 5.5 與 Sonnet 5 的隨附估算牌價。升級後會一次重新解析仍存在的 Claude transcript，保留已失去 transcript 的歷史資料。完整費率與 macOS 安裝指令見英文 README。

要集中分析多個 `CODEX_HOME` 或雲端硬碟同步的其他電腦紀錄，可在 `config.yaml` 加入 `additional_sources`（例如 `codex: ['~/Cloud Drive/laptop/.codex']`）。額外目錄會追加在預設來源與 profile 之後，啟動、背景同步與「立即同步」都會重新讀取設定；額外的 Claude Code 與 OMP 目錄各自成為獨立來源。完整說明見英文 [README](README.md#additional-sources-multiple-homes-and-computers)。

側欄 **總覽** 合併所有 Agent、Claude 與 OMP profile，依總 Token 排名 Harness，顯示 Token 與費用佔比，並依 Agent 堆疊每日用量：

![總覽月報：Harness 排名與各 Agent 堆疊圖](screenshots/dashboard-all-monthly.png)

OMP 與 Claude Code 的單一 Agent 月報：

![OMP 月報](screenshots/dashboard-omp-monthly.png)

![Claude Code 月報](screenshots/dashboard-claude-monthly.png)

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

在瀏覽器開啟 <http://localhost:3003>。

要只建置並安裝本機 CLI，執行 `make install-local`。完整看板資源與背景服務請使用 release 的安裝腳本；本次發行提供 Apple Silicon 與 Intel macOS 套件，不包含 Linux 二進位或 npm Registry 發布。
