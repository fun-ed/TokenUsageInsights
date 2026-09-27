# Fork spec invariants

- Canonical spec: `docs/fork-spec.md`. It overrides upstream `doggy8088/TokenUsageInsights` code/docs and older notes. Upstream = code source only.
- Platforms: macOS + Linux only. No CI/CD; releases manual. `.github/workflows/` stays absent/empty.
- Since v10.0.8 the code has zero Windows branches (no `cfg(windows)`, `cfg!(windows)`, `target_os = "windows"`, PowerShell runner, ZIP assets, `.exe` naming, `%USERPROFILE%` expansion, WSL `cmd.exe`, backslash path normalization) and no `zip` crate. Non-unix builds are unsupported; do not add fallbacks for them.
- Updater `GITHUB_OWNER = "fun-ed"`; `update` and background auto-update query only fork releases (`.tar.gz` assets + `SHA256SUMS`).
- Upstream sync: remote diff review → port one reviewable commit per item → apply spec §4 removal checklist (CI/CD, `*.ps1/*.bat/*.cmd`, Windows installer/service runner, Windows cfg branches, Windows docs, extra README locales, upstream download URLs, upstream `1.x.y` versions) → run §4.3 checks. Never merge/rebase upstream wholesale.
- Claude multi-profile: `get_claude_sources()` scans `~/.claude` + `~/.claude-profiles/<name>/` with `projects/` on every sync; `source_kind` `claude-default` / `claude-profile:<name>`; UI badge via `getSessionSourceBadge()`. `CLAUDE_DIR` set ⇒ profiles skipped; service must not set it.
- Auto import = resident server. `scripts/install.sh --service` → launchd `com.tokenusageinsights` (macOS) / systemd user service (Linux); HOST default `0.0.0.0`. Never run `install.sh` inside the install dir (it `rm -rf`s `static/` etc. before copying).
- Only surviving transcripts import; Claude default `cleanupPeriodDays` 30 deletes them; `history.jsonl` has no token usage. Keep 365+ in every Claude root. Known gap: 2026-01..2026-07 Claude transcripts are gone.
