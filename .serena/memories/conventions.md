# Conventions

- Rust: rustfmt, four-space indentation, `snake_case`; keep application modules private where possible and favor existing `pub(crate)` seams.
- Handler pattern: normalize/validate → `tokio::task::spawn_blocking` for sync work → map domain/join failures to established status + JSON errors. Keep handlers thin.
- DTOs use serde; use `#[serde(default)]` for compatible optional inputs. Prefer contextual established errors over new framework-wide error abstractions.
- Stream JSONL with `BufReader`; provider parsers may skip malformed individual records and must emit common models.
- Use SQLite transactions for writes, imports, sync-state, and rollback boundaries.
- Frontend: descriptive `camelCase`, checked async fetches, existing DOM/API/assistant maps, persisted UI state; preserve bilingual zh-TW-oriented copy.
- Source/fixture roots are environment-configured. Avoid reading or writing developers' real assistant data in tests.
- Do not rename canonical assistant identifiers or bypass transcript path-security checks.