# Technology stack

- Single Rust 2021 Cargo binary: `token-usage-insights`; stable toolchain, no in-repo toolchain pin. Axum 0.7 + Tokio; `rusqlite` bundles SQLite; reqwest uses rustls. Targets macOS and Linux only.
- Static dashboard is vanilla HTML/CSS/ES modules; no bundler, TypeScript, or frontend framework.
- npm package is an ESM thin npx launcher (`npm/cli.cjs`) with Node `>=18.18` and a committed npm lockfile. It downloads matching native release assets from the fork rather than compiling Rust.
- Release package contract: executable + `static/`, `shell/`, `scripts/`, `pricing.csv`, `README.md`, `LICENSE`, `VERSION`; relocation/removal requires coordinated installer changes.
- Runtime data: `~/.token-usage-insights/token_usage_insights.db` (`INSIGHTS_DIR`); install dir `~/.local/share/token-usage-insights`, link `~/.local/bin/token-usage-insights`.
- `src/updater.rs` still targets upstream releases; see residue in `mem:fork_spec`.
- `public/` is a separate GitHub Pages site.
- No CI/CD by design; build, verify, and upload release assets manually with `--locked`. Pushing a tag triggers nothing.
