# Token War Room

Token War Room is a local-first dashboard for AI coding-agent usage. It imports local usage records into SQLite and shows token counts, estimated costs, and session timelines.

English is the canonical README. Short translations are available in [繁體中文](README.zh-TW.md) and [简体中文](README.zh-CN.md).

## Install a precompiled CLI on macOS

The [v10.0.7 release](https://github.com/fun-ed/TokenUsageInsights/releases/tag/v10.0.7) provides CLI packages for Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) Macs. No Rust toolchain is needed.

For a quick local install from this fork's latest release (not the upstream repository), run:

```bash
curl -fsSL https://raw.githubusercontent.com/fun-ed/TokenUsageInsights/main/scripts/get.sh | bash && "$HOME/.local/bin/token-usage-insights"
```

The installer selects the matching macOS CLI, copies the dashboard into `~/.local/share/token-usage-insights`, and links the executable at `~/.local/bin/token-usage-insights`. Open <http://localhost:3003> if the browser does not open automatically.

To verify the checksum before installing, download the release package manually (set `target=x86_64-apple-darwin` on an Intel Mac):

```bash
version=v10.0.7
target=aarch64-apple-darwin
archive="token-usage-insights-${version}-${target}.tar.gz"
mkdir -p "$HOME/Downloads/token-usage-insights"
cd "$HOME/Downloads/token-usage-insights"
curl -fLO "https://github.com/fun-ed/TokenUsageInsights/releases/download/${version}/${archive}"
curl -fLO "https://github.com/fun-ed/TokenUsageInsights/releases/download/${version}/SHA256SUMS"
grep -F "  ${archive}" SHA256SUMS | shasum -a 256 -c -
tar -xzf "$archive"
bash "token-usage-insights-${version}-${target}/scripts/install.sh"
"$HOME/.local/bin/token-usage-insights" --version
"$HOME/.local/bin/token-usage-insights"
```

To invoke the CLI by name from zsh, add `export PATH="$HOME/.local/bin:$PATH"` to `~/.zshrc` and start a new terminal. Run the quick installer again to upgrade. Both architectures are also available as [Apple Silicon DMG](https://github.com/fun-ed/TokenUsageInsights/releases/download/v10.0.7/token-usage-insights-v10.0.7-aarch64-apple-darwin.dmg) and [Intel DMG](https://github.com/fun-ed/TokenUsageInsights/releases/download/v10.0.7/token-usage-insights-v10.0.7-x86_64-apple-darwin.dmg). For Linux, build from source unless a matching Linux asset is listed in the release.

## Run from source

```bash
git clone https://github.com/fun-ed/TokenUsageInsights.git
cd TokenUsageInsights
cargo run --release
```

Open <http://localhost:3003>.

## What it reads

The dashboard reads local data from Antigravity, Copilot, Codex, Claude Code, Cursor, Grok Build, Pi, OMP, Muse Code, and MiniMax Code. It supports macOS and Linux.

It does not send usage logs to AI providers. Prices are estimates. When a source does not report a cost, the dashboard uses a matching local pricing rule. The server refreshes its models.dev price cache when available.

## Development

```bash
make fmt
make check
make test
```
