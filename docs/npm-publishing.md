# npm 手動發布

此 fork 的 npm 發布不使用 GitHub Actions 或 npm Trusted Publishing。維護者從本機手動發布 npm 套件。

一般使用者應使用本 fork 的 GitHub 安裝方式：

```sh
npx --yes github:fun-ed/TokenUsageInsights
```

## 發布前置條件

- npm 帳號擁有 `token-usage-insights` 套件名稱，或已決定新的套件名稱。
- `Cargo.toml`、`Cargo.lock`、`package.json` 與 `package-lock.json` 使用相同版本。
- 對應 `vX.Y.Z` Git tag 的 GitHub Release 已公開。
- Release 包含 Apple Silicon macOS、Intel macOS、Linux x86_64 的壓縮包，以及 `SHA256SUMS`。

## 發布步驟

先確認 GitHub Release workflow 已完成並驗證各平台的 Release 包。npm 發布不由 GitHub Actions 代為執行。

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
