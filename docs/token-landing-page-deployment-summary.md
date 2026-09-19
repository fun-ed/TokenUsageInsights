# Token 戰情室一頁式介紹頁面實作總結

## 摘要

已完成 `Token 戰情室` 一頁式介紹頁面並放置於 `public/`，加入 SEO 與分享設定，並保留四組 `imagegen` 版面嘗試與模型評分結果。

選定方案為 `Concept C（Field Guide）`。最終頁面包含 favicon、`og:image`、`robots.txt`、`sitemap.xml` 與 manifest。此 fork 不使用 GitHub Actions 發佈 Pages。

* * *

## 目標（依你要求）

1. 從 README 重點整理內容，建立一頁式介紹頁。  
2. 使用 `imagegen` 先產出 4 組版本並由不同模型評分。  
3. 取最高分版本實作，輸出到 `public/`。  
4. 產出 favicon。  
5. 補齊 SEO、OpenGraph、JSON-LD、`robots.txt`。  
6. 設定 GitHub Pages + 自訂網域 `token.gh.miniasp.com`。  
7. 以 Chrome 截圖後製作 Facebook 專用 `og:image`。  
8. 準備可由 GitHub Pages 或其他靜態網站服務部署的檔案。

* * *

## 產出檔案

### 一頁式頁面

- `public/index.html`
- `public/assets/site.css`
- `public/assets/site.js`

### 搜尋與 SEO

- `public/robots.txt`
- `public/sitemap.xml`
- `public/site.webmanifest`

### 靜態網站部署

- `public/CNAME`（值為 `token.gh.miniasp.com`）
- `public/.nojekyll`
- `public/404.html`

### 圖片資源

- `public/favicon.ico`
- `public/assets/favicon-32.png`
- `public/assets/apple-touch-icon.png`
- `public/assets/icon-192.png`
- `public/assets/icon-512.png`
- `public/assets/og-token-usage-insights.png`
- `public/assets/dashboard-desktop.webp`
- `public/assets/dashboard-mobile.webp`

### 設計資料（imagegen）

- `design/landing-page/concept-a-editorial-ledger.png`
- `design/landing-page/concept-b-instrument-panel.png`
- `design/landing-page/concept-c-field-guide.png`
- `design/landing-page/concept-d-modular-index.png`
- `design/landing-page/favicon-master.png`
- `design/landing-page/og-imagegen-source.png`
- `design/landing-page/render-desktop.png`
- `design/landing-page/render-mobile.png`

* * *

## 評分結果（四組設計）

依「版面清晰、資訊密度、品牌識別、可讀性、可行性」五面向平均評分如下：

- Concept A：86 / 68 / 78 → 平均 77.3  
- Concept B：79 / 58 / 70 → 平均 69.0  
- Concept C：90 / 75 / 92 → 平均 85.7（最高）  
- Concept D：84 / 65 / 79 → 平均 76.0  

最終採用 **Concept C**，因此實作頁面以該風格為主軸。

* * *

## 技術與內容對應

- README 重點被壓縮為「功能導向」敘述：本機/SQLite 架構、token 與 session 分析、使用者可讀的分析視覺化。  
- 刻意避免直接複製原始 README 的長段落，頁面採重點導覽型資訊架構。  
- 保留可安裝指示、資料導向流程、隱私與安全宣告等主要賣點。  
- 前端維持響應式，並含簡化動畫與可停用的 reduced-motion 支援。  

* * *

## GitHub Pages 部署

此 fork 沒有 GitHub Actions workflow。若使用 GitHub Pages，請在 Repository Settings 將來源設為 `main` 分支的 `/public` 目錄，或自行以其他靜態網站服務部署 `public/`。

- `CNAME` 已建立，預期網址為 `https://token.gh.miniasp.com/`。
- 圖片與 OG 資產包含 1200x630 `og:image`。

### Chrome 截圖與 meta 驗證重點

- 已以 Chrome 進行桌機與行動版快照。
- `canonical`、`og:*`、`twitter:*` 與 JSON-LD 在頁面 metadata 中存在。
- `robots.txt` 與 `sitemap.xml` 包含在 `public/`。

