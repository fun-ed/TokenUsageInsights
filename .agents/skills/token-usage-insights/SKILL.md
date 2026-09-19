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

This fork owns the `v10.x.y` tag namespace. The current release is `v10.0.4`.

- Cargo and npm package metadata use `10.x.y`; release tags use matching `v10.x.y`.
- Never reuse upstream `v1.x.y` release tags.
- Use `https://github.com/doggy8088/TokenUsageInsights` as an upstream source. Fetch and review its diff before integrating it.
- Merge compatible upstream fixes only after validation. Port optional upstream features as separate, reviewable commits.
- Resolve conflicts in favor of this fork's local-first behavior, `v10.x.y` namespace, and fork-specific features such as multi-profile Claude discovery and the all-harness overview.

## Manual releases

- GitHub Actions workflows are intentionally absent. Build and verify release assets locally, then create the GitHub Release and upload the checked artifacts manually.

## Fork-specific product behavior

- Claude Code scans the default config root and discovered `~/.claude-profiles/*/projects` profile roots. Profile sessions must retain independent source identity and path-safe transcript lookup.
- The sidebar **總覽** uses the pseudo-assistant `all` to combine daily, monthly, and yearly reports across every assistant and profile. It is read-only.
- In total overview mode, preserve per-session assistant/profile badges and show the Harness ranking by total tokens with the existing agent icon metadata.

## Verification and delivery

For Rust changes, run a focused test first, then `TMPDIR=/private/tmp cargo test`, `TMPDIR=/private/tmp cargo clippy --all-targets --all-features -- -D warnings`, and the relevant release build when shipping a binary.

For `static/` changes, run `node --check static/app.js`, the relevant Node tests, and manually inspect the affected dashboard flow. Commit verified work using a detailed zh-TW Conventional Commit without AI attribution.
