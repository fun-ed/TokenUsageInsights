---
name: token-usage-insights
description: Maintain TokenUsageInsights features, provider imports, dashboard reports, fork releases, or upstream synchronization. Use whenever changing this repository.
---

# TokenUsageInsights Project Skill

## Orientation

TokenUsageInsights is a local-first Rust/Axum dashboard with plain ES-module frontend assets. It imports local provider logs into SQLite and serves daily, monthly, and yearly reports. It supports macOS and Linux only.

`docs/fork-spec.md` is the canonical spec. When it conflicts with upstream code, upstream docs, or older notes, follow the spec.

- Entry/lifecycle: `src/main.rs`; routes use `/api/:assistant/...`.
- Provider parsing and sync: `src/db.rs` and provider adapters; keep handlers thin.
- Reports: reuse `src/reporting.rs`, `src/pricing.rs`, and the existing `SessionIdentity` source boundary.
- Dashboard: `static/index.html`, `static/app.js`, and `static/i18n.js`; no frontend framework or dependency is needed.

## Safety boundaries

- Keep collector parsing outside HTTP handlers. Run synchronous SQLite, filesystem, transcript, and subprocess work via `spawn_blocking`.
- Preserve `session_files.rs` containment checks and source identity (`assistant_type`, `source_kind`, `source_dir_key`) whenever displaying or resolving a session.
- Treat new aggregate views as read-only unless every write action has an explicit, single-source target.
- Use transactions for writes; never commit user session logs, local databases, credentials, or personal paths.

## Fork release and upstream policy

This fork owns the `v10.x.y` tag namespace. The current release is `v10.0.12`.

- Cargo and npm package metadata use `10.x.y`; release tags use matching `v10.x.y`.
- Never reuse upstream `v1.x.y` release tags.
- Use `https://github.com/doggy8088/TokenUsageInsights` only as a code source. Inspect its remote commit/diff metadata without fetching the full tree; selectively port validated Linux/macOS fixes and optional features as separate, reviewable commits.
- After every port, apply `docs/fork-spec.md` §4 before committing: reject CI/CD files, Windows scripts, Windows `cfg` branches, Windows docs, extra README locales, upstream download sources, and upstream version numbers. Run the §4.3 checks. If a change mixes platforms, keep only the Linux/macOS portion.
- Resolve conflicts in favor of the spec: local-first behavior, `v10.x.y`, multi-profile Claude discovery, the all-harness overview, and `fun-ed` release sources.
- v10.0.8 removed every Windows branch and pointed the updater at `fun-ed` (`docs/fork-spec.md` §5). Do not reintroduce them.

## Manual releases

GitHub Actions workflows are intentionally absent. Build, verify, and upload release assets manually.

### macOS Apple Silicon and Intel assets

1. Confirm `main` is clean, the Cargo/npm versions match, and `vX.Y.Z` does not exist on `origin`.
2. Run `TMPDIR=/private/tmp cargo fmt --check`, `cargo test --locked`, `cargo clippy --all-targets --all-features --locked -- -D warnings`, `npm test`, and `git diff --check`. Build `aarch64-apple-darwin` and `x86_64-apple-darwin` with `RUSTFLAGS='-D warnings' cargo build --release --locked --target TARGET --bin token-usage-insights`.
3. For each target, stage its `target/TARGET/release/token-usage-insights`, `static/`, `shell/`, `scripts/`, `pricing.csv`, `README.md`, `LICENSE`, and a `VERSION` file in `token-usage-insights-vX.Y.Z-TARGET/`. Mark the binary, `scripts/install.sh`, `scripts/get.sh`, and shell collectors executable.
4. Create a target-specific `.tar.gz` containing the staged folder and a `.dmg` with `hdiutil create -format UDZO`; write a shared `SHA256SUMS` using `shasum -a 256` for every uploaded asset.
5. Check each binary's architecture and run the staged `scripts/install.sh` with isolated install/data directories. Start the installed executable and verify `/api/antigravity/pricing` plus SQLite creation. Validate the matching `scripts/get.sh` archive path before documenting a one-line install.
6. Create and push the annotated `vX.Y.Z` tag, create the public GitHub Release with zh-TW notes and a valid compare link, upload both targets' archives/DMGs and `SHA256SUMS`, then download the uploaded checksums and verify the assets.

`npx` and `scripts/get.sh` download target-specific `.tar.gz` files, not DMGs. Publish matching tarballs before claiming those installer paths work.

## Fork-specific product behavior

- Claude Code scans the default config root and discovered `~/.claude-profiles/*/projects` profile roots on every sync. Profile sessions must retain independent source identity (`claude-default`, `claude-profile:<name>`) and path-safe transcript lookup. Setting `CLAUDE_DIR` disables profile discovery, so the background service must not set it.
- Automatic import requires the resident server: `scripts/install.sh --service` installs launchd `com.tokenusageinsights` on macOS or a systemd user service on Linux. Never run `install.sh` from inside the install directory; it deletes `static/` before copying.
- Only surviving transcripts can be imported. Each Claude root needs `cleanupPeriodDays` of at least 365 to avoid the 30-day default deletion.
- The sidebar **總覽** uses the pseudo-assistant `all` to combine daily, monthly, and yearly reports across every assistant and profile. `all` is not an `assistantMeta` entry; use `isAllAssistantsScope()` / `getAssistantMeta('all')` in the frontend and `is_supported_report_assistant()` in handlers. Import, export, import history, rollback, session detail, and rate-limit stay single-assistant only.
- In total overview mode, preserve per-session assistant/profile badges. Each view shows the Harness ranking and share table sorted by total tokens (with cost share and click-through to the same period of one agent); monthly and yearly also show per-agent stacked charts backed by the `agents` field of each breakdown bucket. The setup guide lists every data source, including each Claude profile from `claude_sources`; manual sync syncs all sources.

## Verification and delivery

For Rust changes, run a focused test first, then `TMPDIR=/private/tmp cargo test`, `TMPDIR=/private/tmp cargo clippy --all-targets --all-features -- -D warnings`, and the relevant release build when shipping a binary.

For `static/` changes, run `node --check static/app.js`, the relevant Node tests, and manually inspect the affected dashboard flow. Commit verified work using a detailed zh-TW Conventional Commit without AI attribution.
