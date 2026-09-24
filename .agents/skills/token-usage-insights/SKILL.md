---
name: token-usage-insights
description: Maintain TokenUsageInsights features, provider imports, dashboard reports, fork releases, or upstream synchronization. Use whenever changing this repository.
---

# TokenUsageInsights Project Skill

## Orientation

TokenUsageInsights is a local-first Rust/Axum dashboard with plain ES-module frontend assets. It imports local provider logs into SQLite and serves daily, monthly, and yearly reports.

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

This fork owns the `v10.x.y` tag namespace. The current release is `v10.0.7`.

- Cargo and npm package metadata use `10.x.y`; release tags use matching `v10.x.y`.
- Never reuse upstream `v1.x.y` release tags.
- Use `https://github.com/doggy8088/TokenUsageInsights` as an upstream source. Inspect its remote commit/diff metadata without fetching the full tree; selectively port validated Linux/macOS fixes and optional features as separate, reviewable commits.
- Upstream sync is Unix-only for this fork: exclude Windows-only scripts, tests, workflows, installer paths, release assets, documentation, and platform-specific code. If a change mixes platforms, extract only the Linux/macOS portion; never merge upstream wholesale.
- Resolve conflicts in favor of this fork's local-first behavior, `v10.x.y` namespace, and fork-specific features such as multi-profile Claude discovery and the all-harness overview.

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

- Claude Code scans the default config root and discovered `~/.claude-profiles/*/projects` profile roots. Profile sessions must retain independent source identity and path-safe transcript lookup.
- The sidebar **總覽** uses the pseudo-assistant `all` to combine daily, monthly, and yearly reports across every assistant and profile. It is read-only.
- In total overview mode, preserve per-session assistant/profile badges and show the Harness ranking by total tokens with the existing agent icon metadata.

## Verification and delivery

For Rust changes, run a focused test first, then `TMPDIR=/private/tmp cargo test`, `TMPDIR=/private/tmp cargo clippy --all-targets --all-features -- -D warnings`, and the relevant release build when shipping a binary.

For `static/` changes, run `node --check static/app.js`, the relevant Node tests, and manually inspect the affected dashboard flow. Commit verified work using a detailed zh-TW Conventional Commit without AI attribution.
