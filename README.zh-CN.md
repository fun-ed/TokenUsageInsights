# Token 战情室

Token 战情室是本地优先的 AI Coding Agent 用量看板。它将本地记录导入 SQLite，显示 Token、预估费用与 Session 时间轴。

支持 Antigravity、Copilot、Codex、Claude Code、Cursor、Grok Build、Pi、OMP、Muse Code 和 MiniMax Code；所有来源均保持本地读取。

本仓库是 `fun-ed` fork，功能以 [docs/fork-spec.md](docs/fork-spec.md) 为准，不以 upstream 为准。仅支持 macOS 与 Linux，没有 CI/CD，发布一律手动。英文 [README](README.md) 是正式说明。

Claude Code 会读取 `~/.claude` 与每个含 `projects/` 的 `~/.claude-profiles/<name>/`，Web UI 以 `Default`、`work`、`p2` 等 badge 标示来源。要后台自动导入，请用 release 内的 `scripts/install.sh --service` 安装服务。Claude Code 默认 30 天删除 transcript，请在各 profile 的 `settings.json` 设置 `"cleanupPeriodDays": 365`。

OMP 同时读取 `~/.omp/agent/sessions` 与 `~/.omp/profiles/<name>/agent/sessions`，保留各 profile 的独立来源。设置 `OMP_DIR` 时停用自动 profile 发现。

Antigravity CLI 无需设置 Status Line 或收集脚本：看板直接读取 `~/.gemini/antigravity-cli/brain/` 的原生会话记录，并只读查询 `conversations/<session-id>.db` 获取每次回复的模型。旧版 `usage/` 收集文件仍可导入，同一会话不会重复计算。费用为依 `pricing.csv` 的预估；模型或价格不明时无法预估，不代表免费。

当前版本为 [v10.0.22](https://github.com/fun-ed/TokenUsageInsights/releases/tag/v10.0.22)，选择性同步 upstream v1.4.1。Copilot CLI 合并事件文件与 SQLite 的逐次用量，保留实际 UTC 时间，修正跨日续用与日内 K 线分布；首次同步自动核对可取得的历史数据，不必清空数据库。来源不完整或无法唯一配对时保留旧数据并重试，保持手动导入、来源隔离、子代理与已知 thinking effort。新增 Ollama `muse-spark-open`／`:cloud` 定价，输入／缓存读取／输出每百万 Token 为 US$1.25／0.15／4.25。保留前版价格与标题修正；优惠期限及预估限制见[官方价格核查](docs/research/2026-10-21-prompt-cache-pricing.md)。完整 macOS 安装命令见英文 README。

要集中分析多个 `CODEX_HOME` 或云盘同步的其他电脑记录，可在 `config.yaml` 加入 `additional_sources`（例如 `codex: ['~/Cloud Drive/laptop/.codex']`）。额外目录会追加到默认来源与 profile 之后，启动、后台同步与「立即同步」都会重新读取设置；额外的 Claude Code 与 OMP 目录各自成为独立来源。完整说明见英文 [README](README.md#additional-sources-multiple-homes-and-computers)。

侧栏 **总览** 合并所有 Agent、Claude 与 OMP profile，按总 Token 排名 Harness，显示 Token 与费用占比，并按 Agent 堆叠每日用量：

![总览月报：Harness 排名与各 Agent 堆叠图](screenshots/dashboard-all-monthly.png)

OMP 与 Claude Code 的单一 Agent 月报：

![OMP 月报](screenshots/dashboard-omp-monthly.png)

![Claude Code 月报](screenshots/dashboard-claude-monthly.png)

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

在浏览器打开 <http://localhost:3003>。

要仅构建并安装本机 CLI，执行 `make install-local`。完整看板资源与后台服务请使用 release 的安装脚本；本次发布提供 Apple Silicon 与 Intel macOS 软件包，不包含 Linux 二进制或 npm Registry 发布。
