# Token 戰情室

Token 戰情室是本機優先的 AI Coding Agent 用量看板。它會將本機記錄匯入 SQLite，顯示 Token、估算費用與 Session 時間軸。

支援 Antigravity、Copilot、Codex、Claude Code、Cursor、Grok Build、Pi、OMP、Muse Code 與 MiniMax Code；所有來源均維持本機讀取。

本 repo 是 `fun-ed` fork，功能以 [docs/fork-spec.md](docs/fork-spec.md) 為準，不以 upstream 為準。只支援 macOS 與 Linux，沒有 CI/CD，發行一律手動。英文 [README](README.md) 是正式說明。

Claude Code 會讀取 `~/.claude` 與每個含 `projects/` 的 `~/.claude-profiles/<name>/`，Web UI 以 `Default`、`work`、`p2` 等 badge 標示來源。要背景自動匯入，請用 release 內的 `scripts/install.sh --service` 安裝服務。Claude Code 預設 30 天刪除 transcript，請在各 profile 的 `settings.json` 設定 `"cleanupPeriodDays": 365`。

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

在瀏覽器開啟 <http://localhost:3003>。
