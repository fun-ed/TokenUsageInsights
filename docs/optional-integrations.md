# 選用外部用量整合

此頁面是預設關閉的本機唯讀檢查。按右上角齒輪進入「外部用量整合」，勾選「啟用本機檢查」後，瀏覽器才會要求伺服器檢查下列檔案。取消勾選後不會再發出檢查請求。

本功能不會安裝工具、不會執行第三方命令、不會讀取帳戶、設定或憑證檔，也不會把外部來源寫入 TokenUsageInsights 的 SQLite。每個來源都會顯示「尚未匯入」，所以不可能影響既有總覽、日報、月報或年報。伺服器端 gate 在啟動時預設關閉，且只接受 loopback 瀏覽器的啟用請求；重新啟動後需由本機頁面再次啟用。

## ccusage

ccusage 直接分析各 Coding Agent 的本機使用紀錄。因此它的報告可能與本專案已匯入的 Claude Code、Codex 或其他 Agent sessions 重疊。此頁不尋找或匯入 ccusage 報告，以避免將同一流量相加。

不需全域安裝即可驗證：

```sh
npx ccusage@latest daily --json
```

成功時會輸出 JSON 報告。保留它做人工比較，不要把輸出的總額加到 TokenUsageInsights 的 API 等效值。請參閱 [ccusage 官方文件](https://github.com/ccusage/ccusage)。

## CodeBurn

CodeBurn 的官方 CLI 需要 Node.js 22.13 或更新版本。安裝與驗證：

```sh
npm install -g codeburn
codeburn report --format json
```

檢查頁只會查看 `$CODEBURN_CACHE_DIR/daily-cache.json`（預設 `~/.cache/codeburn/daily-cache.json`）是否存在、可讀且為 JSON，以及 cache 目錄是否存在。它不解析歷史 session 或將結果加入月報。

若命令有輸出而檢查頁顯示找不到快取，先確認 CodeBurn 使用的 `CODEBURN_CACHE_DIR`，再重新開啟整合頁。請參閱 [CodeBurn 官方文件](https://codeburn.app/agent/docs)。

## Magpie

沒有 CLI 的使用者可以從 [Magpie 官方下載頁](https://usemagpie.ai/) 安裝桌面應用程式。啟動 gateway 並完成至少一個 request 後，這個頁面才會檢查：

- `$XDG_CONFIG_HOME/magpie/usage.jsonl`，預設為 `~/.config/magpie/usage.jsonl`
- `$XDG_CONFIG_HOME/magpie/quotas.json`，預設為 `~/.config/magpie/quotas.json`

`usage.jsonl` 的驗證只讀取最多 256 KiB，並只確認第一筆非空 JSONL 紀錄具有 `t`、`agent`、`provider`、`model` 和 `rid` 欄位。`quotas.json` 也只驗證為 JSON。這些欄位值、帳號、request 內容與額度數字不會回傳到瀏覽器。

Magpie 是 gateway 來源，不是 Agent；其 rolling period、gateway-only 呼叫與既有 session 紀錄不可直接相加。請參閱 [Magpie 官方參考](https://github.com/yetone/magpie/blob/main/docs/reference.md)。

## 狀態的意義

| 狀態 | 意義 |
| --- | --- |
| 已驗證 | 找到檔案，且有限的格式檢查成功。 |
| 找不到 | 路徑不存在；安裝工具、產生資料或確認環境變數後再試。 |
| 格式無效 | 找到檔案，但不是預期的 JSON 或 JSONL 最小格式。保留原檔案並以其工具重新產生。 |
| 尚未驗證 | 此來源沒有安全可辨識的獨立輸出，或檔案過大／無法讀取。 |

這是安裝與可讀性的驗證，不是使用量匯入或付款歸因驗證。
