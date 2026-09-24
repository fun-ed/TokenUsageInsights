---
name: bump-and-release
description: 升級 TokenUsageInsights 的 v10.x.y 版本並手動建立 GitHub Release；使用於使用者要求 bump patch、minor、major 或發佈新版本時。
---

# Bump and Release

以 `AGENTS.md` 的交付規範與 `.agents/skills/token-usage-insights/SKILL.md` 的「Manual releases」章節為權威流程。此 fork 只發佈 Linux 與 macOS 成品，從本機建置並手動上傳；不使用上游發行標籤與 CI 發行工作流程。

1. 確認工作樹、`origin/main`、既有 `v10.x.y` 標籤與 Release 狀態。未指定升級類型時預設升級 patch；指定 minor 或 major 時才升級對應欄位。
2. 同步 `Cargo.toml`、`Cargo.lock`、`package.json`、`package-lock.json`、`CHANGELOG.md` 與 README 的版本範例。套件版本為 `10.x.y`，標籤為 `v10.x.y`；不得覆寫已存在的標籤。
3. 按專案技能的驗證清單測試並在本機建置對應 Linux/macOS 資產。只宣稱已實際建置及驗證的目標平台；將 `SHA256SUMS` 隨資產一同發佈。
4. 建立詳述影響與驗證結果的正體中文 Conventional Commit，推送至 fork，再建立並推送附註標籤。於 fork 上建立非草稿 GitHub Release，附正體中文發行說明、比較連結與資產。
5. 從遠端回讀 Release 與下載資產，核對校驗碼及安裝流程，確認工作樹及遠端狀態後才回報正式發佈。
