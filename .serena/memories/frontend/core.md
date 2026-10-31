# Frontend map

- `static/index.html` is the dashboard document; `static/app.js` is the main plain ES-module application. `styles.css` composes dashboard CSS, with utilities in adjacent modules.
- UI state is mutable module-level state persisted through URL parameters, cookies, and `localStorage`; API calls use `fetch` with stale-request protection.
- Use existing DOM IDs/classes and assistant maps (`assistantAliasMap`/`assistantMeta`) rather than creating divergent names or parallel state.
- Chart.js renders charts; Marked + DOMPurify render response content.
- Preserve existing bilingual, predominantly zh-TW text and canonical assistant IDs (`antigravity`, `copilot`, `codex`, etc.).
- No frontend build framework exists; do not add package dependencies for ordinary dashboard changes. Manual local-browser verification is required for static UI changes; see `mem:task_completion`.
- `main.rs` 對 static 回應附加 `Cache-Control: no-cache`，讓瀏覽器重新驗證檔案；既有 `?v=N` 查詢仍存在，但不是套件版本或內容正確性的證明。前端部署需核對實際服務的 app.js／CSS／i18n 內容，不能只看側欄版本。
- Source badges: `getSessionSourceBadge()` in `app.js` plus escaped helpers in `static/source-utils.js` (OMP default/profile/`omp-source:`, Claude `claude-source:` hex-path decode); tests in `tests/source-utils.test.mjs`.
- Session title presentation uses `formatSessionNameDisplay(name, cwd)` from `static/session-utils.js` across list, chart, parent tooltip, drawer, and model-session views. Since v10.0.20 it removes pasted-content wrapper tags while retaining their text, and labels reminder-prefixed titles `Magpie` only for CWDs containing `magpie` case-insensitively. It does not sanitize transcript bodies or change database values.
- Daily model breakdown (v10.0.23): `renderDashboard` passes daily `models` and date into `renderDailyModelsTable`, which reuses `appendModelSummaryRows` and the monthly/yearly Session drilldown. Keep Agent/profile/source badges, full identity and standalone effort badges; missing/empty `models` renders the daily placeholder.
- 區間分頁以 `agent`／`tab=range`／`start`／`end` 還原 URL；ISO 週以 UTC 計算，支援跨年、最近 2／4 週與前後區間。共用 monthly DOM／renderer，因此 range 與月報各自的 request sequence、assistant 和 activeTab 都要驗證；日／年遲到回應也不得覆蓋目前畫面。重新整理成功通知只能在資料確實載入後顯示，匯入／回滾後重載目前區間。
- all 總覽沿用 Harness 排名、同期間 Agent 切換、來源 badge 與 `agents` 堆疊圖，但沒有 Session 詳情。模型會話按鈕需 disabled／aria-disabled，共用 `openSessionTimeline()` 也先拒絕 all，防止其他呼叫點繞過 UI。range 不提供一般匯出動作；仍保留單一 Agent 的匯入／歷史功能。
- 日／月／年與區間的分頁下拉各佔全寬一列，期間欄位另列；共用 CSS 覆寫固定 dropdown 寬度。日期邊界、ISO 跨年與 all 詳情拒絕測試在 `tests/range-analysis.test.mjs`，該檔須納入 package.json 的明確 npm test 清單。