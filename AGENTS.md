# Repository Guidelines

## Project Overview
TokenUsageInsights is a local-first Rust application that imports token-usage data from Antigravity, Copilot, Codex, Claude, Cursor, Grok, Pi, OMP, and Muse. It provides a CLI plus an Axum-served dashboard/API, normalizing source data into SQLite and calculating costs from `pricing.csv`.


## Project Skill
- For any repository change, read and apply `skills/token-usage-insights/SKILL.md`. It is the shared operational reference for local-data boundaries, Claude profile sources, all-harness reporting, fork versioning, and controlled upstream integration.

## Architecture & Data Flow
- **Entry path:** `src/main.rs` runs `cli::run` first. A recognized subcommand (`export`, `export-all`, `import`, `update`, help/version) exits through `src/cli.rs`; otherwise it initializes SQLite, serves Axum routes/static assets, and starts periodic sync.
- **Read path:** browser (`static/index.html` → `static/app.js`) calls `/api/...` → thin `src/handlers/` endpoint → `spawn_blocking` for synchronous work → `db`, `reporting`, and `pricing` → JSON DTO.
- **Write/sync path:** provider adapters parse local JSON/JSONL/SQLite data into `db::UsageEntry`; `db::sync_usage_logs` persists entries and incremental state transactionally. Keep collector-specific parsing outside handlers.
- **Session details:** handler → `session_details` → `session_files` (ID/path containment validation) → `timeline` parser. Do not bypass these safety boundaries for transcript access.
- **Reporting:** use `reporting.rs` for session grouping and delta-versus-cumulative aggregation, and `pricing.rs` for costs; do not duplicate aggregation/cost rules in endpoints.

## Key Directories
- `src/` — Rust application. `main.rs` owns lifecycle/routes; `db.rs` owns schema, migrations, imports, sync, and source discovery.
  - `src/handlers/` — daily/monthly/yearly/misc HTTP handlers and shared assistant DTOs/normalization.
  - `src/db/`, `grok.rs`, `pi.rs`, `muse.rs`, `vscode.rs` — provider-specific adapters.
  - `session_files.rs`, `session_details.rs`, `timeline.rs` — session reconstruction and path-safe transcript parsing.
- `static/` — dashboard: plain ES modules, HTML, CSS, i18n, and assistant assets. `app.js` is the main frontend entry.
- `tests/` — Node built-in-test suites for package installer and frontend utilities; Rust tests are colocated under `#[cfg(test)]` in `src/`.
- `npm/` — thin npx wrapper, release downloader/checksum validation, and prepublish guard; it does **not** build Rust.
- `scripts/` — cross-platform download/install/service/build/smoke-test scripts. Treat `install.*` and `run-service.ps1` as service-lifecycle-sensitive.
- `shell/` — status-line collectors and the source-build systemd template. Do not change the collectors' protected input parsing, JSONL writes, or state-update logic without preserving synchronization behavior.
- `public/` — independently deployed GitHub Pages landing site; `docs/npm-publishing.md` is the npm release runbook.

## Development Commands
Run from the repository root:

```sh
make dev                         # debug dashboard at http://localhost:3003
make build                       # cargo build
make build-release               # cargo build --release
make test                        # cargo test
make fmt                         # cargo fmt
make clippy                      # cargo clippy --all-targets --all-features
make check                       # cargo check --all-targets --all-features
make all                         # fmt, check, test, release build
```

- `PORT=3004 make dev` changes the local port. Direct source development can use `cargo run` or `cargo build --release --bin token-usage-insights`.
- For npm/package changes: `npm ci --ignore-scripts && npm test && npm pack --dry-run --ignore-scripts`. `npm run check:package` also verifies the release/version/tag/assets preconditions.
- Windows release gate: `./scripts/build.ps1`; run `./scripts/test-windows.ps1` when changing Windows installers or collectors. `-AllowWarnings` is local-iteration-only.
- The systemd Make targets are Linux-only and require `sudo`; render the source template with `make service-file` or `sed "s|<PROJECT_DIR>|$PWD|g" shell/token-usage-insights.service`.

## Code Conventions & Common Patterns
- **Rust:** standard rustfmt, four-space indentation, and `snake_case`. Keep `main.rs` module ownership clear; prefer existing `pub(crate)` seams over widening APIs.
- **Handlers:** normalize/validate inputs, then use `tokio::task::spawn_blocking` for `rusqlite`, filesystem, transcript parsing, and subprocess/Git work. Convert domain and join errors into the existing status/JSON response pattern.
- **Data/error handling:** use `serde` DTOs (`#[serde(default)]` for compatible optional inputs); return contextual errors in the established style. Use `Connection::transaction()` for writes, imports, and sync-state updates.
- **Parsers:** stream JSONL via `BufReader`, tolerate malformed individual records where the existing adapter does, then map to common `UsageEntry`/timeline models.
- **Frontend:** plain ES modules, descriptive `camelCase`, `async`/`await` + checked `fetch` responses, and module-level UI state persisted through URL parameters, cookies, and `localStorage`. Reuse the existing assistant alias/metadata maps.
- **UI/content:** preserve the established bilingual (predominantly zh-TW) text and canonical identifiers such as `antigravity`, `copilot`, and `codex`.

## Important Files
- `src/main.rs` — CLI/server dispatch, Axum routes, CORS, static serving, background sync, shutdown.
- `src/cli.rs` — CLI parsing and import/export behavior.
- `src/db.rs` — SQLite schema/migrations, sync orchestration, imports, shared usage model.
- `src/handlers/mod.rs` — supported assistants, aliases, shared HTTP DTOs.
- `src/reporting.rs`, `src/pricing.rs` — usage aggregation and pricing; `pricing.csv` is runtime data.
- `src/session_files.rs` — transcript path-security boundary; `src/paths.rs` — resource/path lookup. Use `find_resource` instead of assuming the current directory.
- `Cargo.toml` / `Cargo.lock` — single Rust 2021 binary crate; preserve the committed lockfile.
- `package.json` / `package-lock.json` — ESM npx package, Node `>=18.18`; preserve version synchronization.
- `.github/workflows/release.yml`, `npm-package.yml`, `pages.yml` — release, package, and Pages CI contracts.

## Runtime/Tooling Preferences
- Use stable Rust/Cargo (the repository does not pin a toolchain) and npm; CI uses Node 24, while the package requires Node 18.18 or newer.
- SQLite is bundled through `rusqlite`; do not introduce a separate database service.
- The dashboard requires no frontend build framework. Do not add Node dependencies merely for UI changes.
- Releases package the binary together with `static/`, `shell/`, `scripts/`, and `pricing.csv`; preserve this layout when modifying installation/release behavior.
- The project reads local data by default. Use environment overrides (`INSIGHTS_DIR`, `ANTIGRAVITY_DIR`, `COPILOT_DIR`, `CODEX_DIR`, `CLAUDE_DIR`, `CURSOR_DIR`, `GROK_DIR`, `PI_DIR`, `OMP_DIR`) for isolated development and tests. Never commit local databases, logs, sessions, or personal paths.

## Testing & QA
- Add Rust tests beside the affected module under `#[cfg(test)]`; use `#[tokio::test]` only for async behavior. Use `cargo test <name-filter>` for a focused check.
- Use deterministic fixtures: `rusqlite::Connection::open_in_memory()` for database tests; unique temporary directories for file tests. If mutating environment variables, serialize access, save/restore prior values, and point `INSIGHTS_DIR`/source roots at test data.
- JavaScript tests use Node's native runner (`node:test` and `node:assert/strict`):
  ```sh
  node --test tests/session-utils.test.mjs
  node --test tests/chart-utils.test.mjs
  node --test tests/npm-package.test.cjs
  ```
- There is no configured browser E2E or coverage threshold. For `static/` changes, manually verify the affected flow at the local dashboard and include screenshots in a PR.
- Treat every Cargo warning as a failure. Before completing backend work, run the narrow relevant test, then the appropriate `cargo test`, `cargo clippy --all-targets --all-features`, and build path with **zero warnings and zero errors**.

## Delivery and Release Guardrails
- After editing and verification, create a detailed Traditional Chinese (zh-TW) Conventional Commit. Include the user impact, file-by-file changes, and commands/results; do not leave completed work uncommitted unless explicitly told otherwise.
- PRs must state user-visible and schema/environment impacts, list verification, and include screenshots for dashboard changes.
- **Fork versioning and upstream sync:** This fork reserves the `v10.x.y` release-tag namespace; its baseline is `v10.0.1` and the next patch is `v10.0.2`. Keep Cargo/npm package versions as `10.x.y` (without the tag prefix), use a matching `v10.x.y` tag only for releases, and never reuse upstream `v1.x.y` tags. Treat `https://github.com/doggy8088/TokenUsageInsights` as an upstream source: fetch and review its diff first, then merge compatible changes or selectively port optional features in separate commits. Preserve this fork's versioning, local-first behavior, and fork-specific features when resolving conflicts.
- For releases, synchronize `CHANGELOG.md`, Cargo/npm versions and lockfiles, README version examples, and release assets. Verify the workflow, a non-draft public GitHub Release, all platform archives plus `SHA256SUMS`, real zh-TW release notes with the compare link, and (when enabled) npm publish/npx smoke-test completion.
