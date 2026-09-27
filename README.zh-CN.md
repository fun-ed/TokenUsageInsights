# Token 战情室

Token 战情室是本地优先的 AI Coding Agent 用量看板。它将本地记录导入 SQLite，显示 Token、预估费用与 Session 时间轴。

支持 Antigravity、Copilot、Codex、Claude Code、Cursor、Grok Build、Pi、OMP、Muse Code 和 MiniMax Code；所有来源均保持本地读取。

本仓库是 `fun-ed` fork，功能以 [docs/fork-spec.md](docs/fork-spec.md) 为准，不以 upstream 为准。仅支持 macOS 与 Linux，没有 CI/CD，发布一律手动。英文 [README](README.md) 是正式说明。

Claude Code 会读取 `~/.claude` 与每个含 `projects/` 的 `~/.claude-profiles/<name>/`，Web UI 以 `Default`、`work`、`p2` 等 badge 标示来源。要后台自动导入，请用 release 内的 `scripts/install.sh --service` 安装服务。Claude Code 默认 30 天删除 transcript，请在各 profile 的 `settings.json` 设置 `"cleanupPeriodDays": 365`。

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

在浏览器打开 <http://localhost:3003>。
