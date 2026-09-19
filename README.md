# Token War Room

Token War Room is a local-first dashboard for AI coding-agent usage. It imports local usage records into SQLite and shows token counts, estimated costs, and session timelines.

English is the canonical README. Short translations are available in [繁體中文](README.zh-TW.md), [简体中文](README.zh-CN.md), [日本語](README.ja.md), and [한국어](README.ko.md).

## Run from source

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

Open <http://localhost:3003>.

## What it reads

The dashboard reads local data from Antigravity, Copilot, Codex, Claude Code, Cursor, Grok Build, Pi, OMP, and Muse Code. It supports macOS and Linux.

It does not send usage logs to AI providers. Prices are estimates. When a source does not report a cost, the dashboard uses a matching local pricing rule. The server refreshes its models.dev price cache when available.

## Development

```bash
make fmt
make check
make test
```
