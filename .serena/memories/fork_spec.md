# Fork spec invariants

- Canonical spec: `docs/fork-spec.md`. It overrides upstream `doggy8088/TokenUsageInsights` code/docs and older notes. Upstream = code source only.
- Platforms: macOS + Linux only. No CI/CD; releases manual. `.github/workflows/` stays absent/empty.
- Upstream sync: remote diff review → cherry-pick-style port, one reviewable commit per item → apply spec §4 removal checklist (CI/CD, `*.ps1/*.bat/*.cmd`, Windows installer/service runner, new `cfg(windows)`/`target_os = "windows"` branches, Windows docs, extra README locales, upstream download URLs, upstream `1.x.y` versions) → run §4.3 checks. Never merge/rebase upstream wholesale.
- Claude multi-profile: `get_claude_sources()` scans `~/.claude` + `~/.claude-profiles/<name>/` with `projects/` on every sync; `source_kind` `claude-default` / `claude-profile:<name>`; UI badge via `getSessionSourceBadge()`. `CLAUDE_DIR` set ⇒ profiles skipped; service must not set it.
- Auto import = resident server. `scripts/install.sh --service` → launchd `com.tokenusageinsights` (macOS) / systemd user service (Linux); HOST default `0.0.0.0`. Never run `install.sh` inside the install dir (it `rm -rf`s `static/` etc. before copying).
- Only surviving transcripts import; Claude default `cleanupPeriodDays` 30 deletes them; `history.jsonl` has no token usage. Keep 365+ in every Claude root. Known gap: 2026-01..2026-07 Claude transcripts are gone.
- Known residue (spec §5, remove only in dedicated tested commit): `src/updater.rs` `GITHUB_OWNER = "doggy8088"` (update/auto-update query upstream; semver 1.x < 10.x prevents overwrite) plus Windows service-runner logic; Windows branches in `browser.rs`, `paths.rs`, `main.rs`, `handlers/daily.rs`, `db/cursor.rs`, `vscode.rs`, `db.rs`, `static/app.js`.
