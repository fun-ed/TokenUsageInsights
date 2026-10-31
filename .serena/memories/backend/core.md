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
- Model-session 查詢支援日、月、年與 `start~end`；沿用 blocking-thread 彙總、Cursor mode、model／effort 配對及完整來源 identity。日查詢先映射為 `DatedUsageEntry`，不另寫聚合規則。
- 區間 API `GET /api/:assistant/range/:start/:end` 包含頭尾日期且最多 366 天；拒絕反向／無效日期。使用 `is_supported_report_assistant()` 支援 all，經 `DateMatch::Range` 參數化查詢與既有 `build_period_report` 產生月報格式回應；保留 `agents`、source identity 與 pricing overlay。沒有資料表結構遷移。
- `blob_export.rs` 的網路操作只由明確 CLI 選項啟用：`export-all --out <HTTPS SAS URL>` 或 `import --url`，一般本機檔案／stdout 行為不變。不自動上傳；HTTPS-only，拒絕 URL 帳密與 fragment，不跟隨 redirect，下載上限 200 MB，成功／錯誤輸出不回顯 SAS query。URL argv 仍可能出現在 history／process list，不能將輸出遮蔽視為全面隱藏憑證。