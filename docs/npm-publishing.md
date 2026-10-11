# npm 手動發布

此 fork 的 npm 發布不使用 GitHub Actions 或 npm Trusted Publishing。維護者從本機手動發布 npm 套件。

一般使用者應使用本 fork 的 GitHub 安裝方式：

```sh
npx --yes github:fun-ed/TokenUsageInsights
```

## 發布前置條件

- npm 帳號擁有 `token-usage-insights` 套件名稱，或已決定新的套件名稱。
- `Cargo.toml`、`Cargo.lock`、`package.json` 與 `package-lock.json` 使用相同版本。
- 對應 `v10.x.y` Git tag 的 GitHub Release 已公開。
- Release 包含 npm 安裝器 `npm/install.cjs` 的 `TARGETS` 所列平台壓縮包，以及 `SHA256SUMS`。目前 npm 發布檢查要求 Apple Silicon macOS、Intel macOS、Linux x86_64；僅有 macOS 資產的 release 不符合 npm 發布前置條件。

## 發布步驟

先完成手動 GitHub Release 建置、上傳與各平台安裝驗證。此 fork 沒有 Release workflow，npm 發布也不由 GitHub Actions 代為執行。

```sh
git status --short
git describe --tags --exact-match HEAD
npm ci --ignore-scripts
npm test
npm pack --dry-run --ignore-scripts
npm login
npm whoami
npm publish --access public
```

`npm publish` 會執行 `prepublishOnly`，再次檢查 npm 測試、套件內容、版本與目前 Git tag。

發布後驗證 Registry 的版本與啟動器：

```sh
npm view token-usage-insights version dist-tags.latest
npx --yes token-usage-insights@X.Y.Z --help
```

如果 npm 套件名稱仍屬於 upstream，請勿覆蓋或嘗試取得其控制權。改用 GitHub 安裝指令，或先選擇本 fork 專用的新套件名稱。

## v10.0.18 的發布範圍

本次只發行兩種 macOS 架構的 `.tar.gz`、DMG 與 `SHA256SUMS`，不發布 npm Registry 套件或 Linux 二進位。macOS 使用者可用上述 GitHub 安裝指令或 README 的 `scripts/get.sh`；Linux 使用者需從原始碼建置，直到 Release 提供對應 tarball。不要將套件版本同步誤認為已完成 npm 發布。
