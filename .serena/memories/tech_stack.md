# Technology stack

- Single Rust 2021 Cargo binary: `token-usage-insights`; use stable toolchain locally, no in-repo toolchain pin and no CI workflows. Axum 0.7 + Tokio; `rusqlite` bundles SQLite; reqwest uses rustls.
- Static dashboard is vanilla HTML/CSS/ES modules; no bundler, TypeScript, or frontend framework.
- npm package is an ESM thin npx launcher (`npm/cli.cjs`) with Node `>=18.18` and a committed npm lockfile. It downloads matching native release assets rather than compiling Rust.
- Release package contract: executable + `static/`, `shell/`, `scripts/`, `pricing.csv`; relocation/removal requires coordinated installer/workflow changes.
- `public/` is a separate GitHub Pages site.
- `.github/workflows/` has no workflows by design; build, verify, and upload release assets manually. Cargo release work uses committed lockfile with `--locked`. Do not expect pushing a tag to trigger CI.