---
scope: project:token-usage-insights
project: token-usage-insights
jira: N/A
wiki: N/A
github_owner: fun-ed
github_repo: TokenUsageInsights
repository: fun-ed/TokenUsageInsights
folder: .serena/memories
occurred_start: 2026-09-23
occurred_end: 2026-09-23T08:40:45Z
device_label: macos-arm64-local
harness: API
cli: N/A
model: openai-codex/gpt-6-sol
session_name: v10.0.6 release verification and local install
---

# v10.0.6 release and local install (2026-09-23)

- Fork repository is `fun-ed/TokenUsageInsights`; `main` and `origin/main` at `3c27ead` with annotated `v10.0.6`, clean worktree at verification. Cargo and npm package metadata are 10.0.6.
- Public, non-draft, non-prerelease release: https://github.com/fun-ed/TokenUsageInsights/releases/tag/v10.0.6. Assets checked: Apple Silicon macOS `.dmg`, Apple Silicon macOS `.tar.gz`, `SHA256SUMS`. Intel macOS/Linux assets were not present; npm registry publishing was not verified. GitHub CLI defaults may resolve to upstream; always use `--repo fun-ed/TokenUsageInsights`.
- On 2026-09-23, `make install-local` ran successfully (`cargo build --release`) and installed `target/release/token-usage-insights` to `~/.local/bin/token-usage-insights`. Installed binary `--version` returned `token-usage-insights 10.0.6`; `cmp` confirmed installed and built binaries identical. Previously installed binary was 10.0.5.
- Web UI (`static/app.js:loadAppVersion`) fetches `/api/version` with `cache: no-store`; server version is `env!("CARGO_PKG_VERSION")`. Display follows the running server binary, so restart any older instance to show 10.0.6. Port 3003 was not listening during this session; no live web version was observed. The release-date label in `static/index.html` may remain hardcoded separately and was not changed.
- GitHub Actions workflows are intentionally absent; releases are built/uploaded manually, not by a tag-triggered workflow. `skill://bump-and-release` contains obsolete CI/Windows release instructions; follow repository `AGENTS.md` and `.agents/skills/token-usage-insights/SKILL.md` instead.