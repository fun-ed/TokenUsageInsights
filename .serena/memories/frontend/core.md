# Frontend map

- `static/index.html` is the dashboard document; `static/app.js` is the main plain ES-module application. `styles.css` composes dashboard CSS, with utilities in adjacent modules.
- UI state is mutable module-level state persisted through URL parameters, cookies, and `localStorage`; API calls use `fetch` with stale-request protection.
- Use existing DOM IDs/classes and assistant maps (`assistantAliasMap`/`assistantMeta`) rather than creating divergent names or parallel state.
- Chart.js renders charts; Marked + DOMPurify render response content.
- Preserve existing bilingual, predominantly zh-TW text and canonical assistant IDs (`antigravity`, `copilot`, `codex`, etc.).
- No frontend build framework exists; do not add package dependencies for ordinary dashboard changes. Manual local-browser verification is required for static UI changes; see `mem:task_completion`.
- Static assets are cache-busted by `?v=N` query strings: `index.html` → `styles.css`/`app.js`; `styles.css` → each `css/*.css`; `app.js` → `i18n.js`, `source-utils.js`, etc. Bump every link on the chain to the edited file; upstream ports often conflict on these numbers — take max(fork, upstream)+1, never the upstream value.
- Source badges: `getSessionSourceBadge()` in `app.js` plus escaped helpers in `static/source-utils.js` (OMP default/profile/`omp-source:`, Claude `claude-source:` hex-path decode); tests in `tests/source-utils.test.mjs`.
- Session title presentation uses `formatSessionNameDisplay(name, cwd)` from `static/session-utils.js` across list, chart, parent tooltip, drawer, and model-session views. Since v10.0.20 it removes pasted-content wrapper tags while retaining their text, and labels reminder-prefixed titles `Magpie` only for CWDs containing `magpie` case-insensitively. It does not sanitize transcript bodies or change database values. Cache links for this release: app.js v73, session-utils.js v8.