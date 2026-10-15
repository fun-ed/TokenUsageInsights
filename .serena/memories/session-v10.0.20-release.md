# v10.0.20 release and local installation

Verified 2026-10-16 Taiwan time. Release: https://github.com/fun-ed/TokenUsageInsights/releases/tag/v10.0.20 . Annotated tag/source commit: c31a3c8e6e074238bc90e3345d9433faea687139. Public, non-draft, non-prerelease; five assets: arm64 and x86_64 macOS tar.gz/DMG plus SHA256SUMS. All four downloaded artifact checksums match the published checksum file. No Linux binary or npm Registry publication. Upstream baseline remains v1.3.2.

## Behavior

`formatSessionNameDisplay(name, cwd)` in static/session-utils.js removes complete pasted_content wrapper tags, retaining enclosed text and supporting opening tags without closing tags. Titles beginning with system-reminder display as Magpie only when CWD contains magpie case-insensitively. All title renderers use this helper, including parent tooltip, chart, session list, drawer and model-session drilldown. Cache versions: app.js v73, session-utils.js v8. No raw transcript edits, database migrations, harness reclassification or cross-harness deduplication.

## Verification

- Rust 457 tests; Node 38 tests; fmt check, Clippy warnings denied, npm ci --ignore-scripts and npm pack dry run passed.
- Both macOS release builds have zero warnings/errors. Packaged installers passed isolated installs, CLI --version, API and SQLite creation checks. DMGs passed hdiutil verify. Intel executable ran through Rosetta, not physical Intel hardware.
- Public get.sh pinned to v10.0.20 downloaded the released native archive and installed successfully into isolated directories.
- With explicit service restart approval, the existing local install was backed up, staged native install.sh --service updated the CLI/resources and restarted com.tokenusageinsights. CLI and /api/version report 10.0.20; symlink, HOST/PORT and automatic source discovery preserved.
- Actual localhost:3003 browser smoke observed 33 session rows, 26 Magpie labels, a cleaned pasted-content title and clean detail-drawer title, with no browser errors. Counts reflect a live data snapshot, not fixed acceptance values.

## Scope and follow-up boundary

This release fixes title presentation only. Some Magpie-mediated Claude responses exist in both OMP and Claude source logs; this was verified by matching OMP responseId after removing chatcmpl- with Claude message.id plus token counters. This release does not resolve that double counting. Do not exclude every sdk-cli session or guess attribution solely from working-directory names.

Only project-local Serena memories were updated for this request. No external memory synchronization was requested or performed. Existing unrelated .serena/project.yml and .memsearch changes must remain outside release commits. See `mem:manual_release_and_readme`, `mem:fork_spec`, and `mem:frontend/core`.