# Suggested commands

```sh
mise exec -- make dev            # dashboard, PORT defaults to 3003
mise exec -- make build
mise exec -- make build-release
mise exec -- make test
mise exec -- make fmt
mise exec -- make clippy         # all targets/features
mise exec -- make check          # all targets/features
mise exec -- make all            # fmt, check, test, release build

mise exec -- cargo test <name-filter>  # targeted colocated Rust test
mise exec -- node --test tests/session-utils.test.mjs
mise exec -- npm ci --ignore-scripts && mise exec -- npm test && mise exec -- npm pack --dry-run --ignore-scripts
mise exec -- npm run check:package
```

- `PORT=3004 mise exec -- make dev` changes the local port.
- Linux systemd targets require `sudo`; render the source template with `make service-file`.
- 開發 runtime 與套件工具一律經過 `mise exec --`，包含 Rust/rustc/cargo/rustup、Python/uv/uvx、Node/npm/npx/pnpm/yarn、Bun/bunx 與 Go；Makefile 間接呼叫 runtime 時也由外層 `mise exec -- make ...` 繼承環境。uv 專案使用 `mise exec -- uv run ...`。
- 優先遵守專案版本，否則沿用既有全域設定；不擅自升級或改用系統／Homebrew runtime。權威規則為共用 `CLAUDE.md` 的 Shell 節；使用者於 2026-10-23 再次確認，來源為 KB `session-notes/general/2026-10-23-development-runtimes-mise-exec-preference.md`，此處只是專案執行提示。