# Suggested commands

```sh
make CARGO='mise exec -- cargo' dev            # dashboard, PORT defaults to 3003
make CARGO='mise exec -- cargo' build
make CARGO='mise exec -- cargo' build-release
make CARGO='mise exec -- cargo' test
make CARGO='mise exec -- cargo' fmt
make CARGO='mise exec -- cargo' clippy         # all targets/features
make CARGO='mise exec -- cargo' check          # all targets/features
make CARGO='mise exec -- cargo' all            # fmt, check, test, release build

mise exec -- cargo test <name-filter>  # targeted colocated Rust test
mise exec -- node --test tests/session-utils.test.mjs
mise exec -- npm ci --ignore-scripts && mise exec -- npm test && mise exec -- npm pack --dry-run --ignore-scripts
mise exec -- npm run check:package
```

- `PORT=3004 make CARGO='mise exec -- cargo' dev` changes the local port.
- Linux systemd targets require `sudo`; render the source template with `make service-file`.
- 開發 runtime 與套件工具一律直接經過 `mise exec --`，包含 Rust、Python／uv、Node／npm、Bun 與 Go；Makefile 用 `CARGO='mise exec -- cargo'` 覆寫實際呼叫，不只依賴 PATH 或外層 mise 環境。uv 專案使用 `mise exec -- uv run ...`；本機安裝的 symlink／資源／服務保護流程見 `mem:manual_release_and_readme`。
- 優先遵守專案版本，否則沿用既有全域設定；不擅自升級或改用系統／Homebrew runtime。權威規則為共用 `CLAUDE.md` 的 Shell 節；使用者於 2026-10-23 再次確認，來源為 KB `session-notes/general/2026-10-23-development-runtimes-mise-exec-preference.md`，此處只是專案執行提示。