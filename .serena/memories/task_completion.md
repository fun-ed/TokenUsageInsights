# Completion gate

- First run the narrow test covering a changed Rust/parser/API path, then `cargo test`; backend changes also require `cargo clippy --all-targets --all-features` and the applicable Cargo build. All must have zero warnings and zero errors.
- Rust tests are colocated under `#[cfg(test)]`; use in-memory SQLite or unique temporary directories. Serialize environment mutation, preserve/restore prior values, and redirect `INSIGHTS_DIR` plus source-root overrides to fixtures.
- npm/package changes require `npm ci --ignore-scripts`, `npm test`, and `npm pack --dry-run --ignore-scripts`; use `npm run check:package` for the publish gate.
- `static/` changes need manual local dashboard verification and PR screenshots; no browser E2E framework or coverage percentage gate exists.
- Commit completed verified work using a detailed zh-TW Conventional Commit with impact, per-file details, and verification results. Releases additionally require synchronized versions/changelog/README, locally verified macOS/Linux assets with `SHA256SUMS`, a public manually uploaded GitHub Release, and substantive zh-TW notes. Check npm publication separately when requested.