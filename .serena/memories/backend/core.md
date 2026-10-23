# Backend map

- Route flow: `main.rs` router → thin `src/handlers/` endpoint → `spawn_blocking` for SQLite/filesystem/subprocess work → `db`/`reporting`/`pricing` → JSON DTO.
- `handlers/mod.rs` owns assistant aliases/support checks and shared request/response DTOs; preserve this canonical normalization.
- Keep provider parsing in `src/db/`, `grok.rs`, `pi.rs`, `muse.rs`, `vscode.rs`; map to common `db::UsageEntry`, not handler-specific types.
- `reporting.rs` owns session aggregation (deltas preferred, cumulative fallback); `pricing.rs` owns cost calculation.
- Session details must go through `session_details.rs` and `session_files.rs`; the latter validates IDs and canonical-root containment before assistant timeline parsing in `timeline.rs`.
- SQLite write/import/sync paths use transactions and incremental `sync_state`; preserve rollback/import-batch behavior.
- OMP intentionally delegates its discovery/parsing to the Pi adapter; preserve that source-kind boundary rather than fork parsing behavior.
- `src/config.rs` owns `config.yaml` lookup (`read_config_file`, shared with the updater) and `SourceConfig`; sync goes through `db::sync_configured_sources`, which reloads config each run. Claude/OMP source discovery and identity rules are in `mem:fork_spec`.
- Resolve bundled files with `paths::find_resource`, never by assuming the current working directory.
- Blocking discipline and Result/serde response patterns are in `mem:conventions`; isolated backend test fixtures are in `mem:task_completion`.
- 費率維護：隨附 `pricing.csv` 是現行估算快照，沒有依請求日期或尖峰／離峰選價；更新別名費率可能改變歷史報表估算，但不改來源回報的費用。快取 TTL、模型版本與費率是不同概念，需分開查核。
- 快取費率來源、Gemini 優惠期限及 DeepSeek Pro 路由矛盾見 `docs/research/2026-10-21-prompt-cache-pricing.md`。隨附牌價回歸測試應排除 models.dev 快取，保留執行時原有的動態價格優先序。
- Daily model reports (v10.0.23): `aggregate_usage_details` returns model summaries using `reporting::summarize_models_by_mode`; `/api/:assistant/usage/:date` adds `models`. `get_model_sessions` accepts a valid day, month or year; Day reads `get_usage_entries_by_date` and maps to `DatedUsageEntry` before the existing model-specific collector. Retain blocking-thread work and full source identity; invalid periods return 400, empty dates return an empty Session list. Tests live in daily/monthly handlers and the all-agent report integration test.