# Technology stack

- Single Rust 2021 Cargo binary: `token-usage-insights`; stable toolchain in CI, no in-repo toolchain pin. Axum 0.7 + Tokio; `rusqlite` bundles SQLite; reqwest uses rustls.
- Static dashboard is vanilla HTML/CSS/ES modules; no bundler, TypeScript, or frontend framework.
- npm package is an ESM thin npx launcher (`npm/cli.cjs`) with Node `>=18.18`; committed npm lockfile, CI uses Node 24. It downloads matching native release assets rather than compiling Rust.
- Release package contract: executable + `static/`, `shell/`, `scripts/`, `pricing.csv`; relocation/removal requires coordinated installer/workflow changes.
- `public/` is a separate GitHub Pages site.
- CI contracts live in `.github/workflows/release.yml`, `npm-package.yml`, and `pages.yml`; Cargo release work uses committed lockfile with `--locked`.