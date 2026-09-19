# Backend map

- Route flow: `main.rs` router → thin `src/handlers/` endpoint → `spawn_blocking` for SQLite/filesystem/subprocess work → `db`/`reporting`/`pricing` → JSON DTO.
- `handlers/mod.rs` owns assistant aliases/support checks and shared request/response DTOs; preserve this canonical normalization.
- Keep provider parsing in `src/db/`, `grok.rs`, `pi.rs`, `muse.rs`, `vscode.rs`; map to common `db::UsageEntry`, not handler-specific types.
- `reporting.rs` owns session aggregation (deltas preferred, cumulative fallback); `pricing.rs` owns cost calculation.
- Session details must go through `session_details.rs` and `session_files.rs`; the latter validates IDs and canonical-root containment before assistant timeline parsing in `timeline.rs`.
- SQLite write/import/sync paths use transactions and incremental `sync_state`; preserve rollback/import-batch behavior.
- OMP intentionally delegates its discovery/parsing to the Pi adapter; preserve that source-kind boundary rather than fork parsing behavior.
- Resolve bundled files with `paths::find_resource`, never by assuming the current working directory.
- Blocking discipline and Result/serde response patterns are in `mem:conventions`; isolated backend test fixtures are in `mem:task_completion`.