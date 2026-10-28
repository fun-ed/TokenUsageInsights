# 選用外部用量整合

此頁面是預設關閉的本機唯讀檢查。按右上角齒輪進入「外部用量整合」，勾選「啟用本機檢查」後，瀏覽器才會要求伺服器檢查下列檔案。取消勾選後不會再發出檢查請求。

本功能不會安裝工具、不會執行第三方命令、不會讀取帳戶、設定或憑證檔，也不會把外部來源寫入 TokenUsageInsights 的 SQLite。每個來源都會顯示「尚未匯入」，所以不可能影響既有總覽、日報、月報或年報。伺服器端 gate 在啟動時預設關閉，且只接受 loopback 瀏覽器的啟用請求；重新啟動後需由本機頁面再次啟用。

## ccusage

ccusage 直接分析各 Coding Agent 的本機使用紀錄。因此它的報告可能與本專案已匯入的 Claude Code、Codex 或其他 Agent sessions 重疊。此頁不尋找或匯入 ccusage 報告，以避免將同一流量相加。

若已經自行安裝 ccusage，可匯出標準 daily JSON：

```sh
ccusage daily --json --breakdown > ccusage-daily.json
```

在選用整合頁啟用本機檢查後，可選擇這份 `.json` 檔並只在目前瀏覽器頁面解析。程式不執行 ccusage、不使用 `npx`、不寫入這份檔案，也不接受通用 `.txt`。只接受每日報告的 `daily`，或舊格式的 `type: "daily"` 加 `data`；日期必須唯一且為 `YYYY-MM-DD`，專案或 session 匯出會被拒絕以避免暴露路徑與把多層資料誤加總。

ccusage 日期的時區不會由上傳檔案推論；畫面會明示為未宣告。它只用於來源內的每日／模型檢視，不要把輸出的總額加到 TokenUsageInsights、CodeBurn 或 Magpie 的數字。請參閱 [ccusage 官方文件](https://github.com/ccusage/ccusage)。

## CodeBurn

CodeBurn 的官方 CLI 需要 Node.js 22.13 或更新版本。此版本直接唯讀已安裝版本宣告的 `daily-cache.v74.json`：

```sh
codeburn report --format json
```

檢查頁只會查看 `$CODEBURN_CACHE_DIR/daily-cache.v74.json`（預設 `~/.cache/codeburn/daily-cache.v74.json`）是否存在、可讀且通過 v74 schema 檢查。保留的舊版本 cache 不會依檔名、修改時間或資料量猜測為 active。頁面只讀每日 aggregate、模型、Token、calls 和來源估計 USD；不讀 projects、session 或帳戶資料，也不將結果加入月報。

畫面會顯示 CodeBurn cache 自己記錄的 `tzKey`、完整／watermark 宣告與最後計算日期；這些是 cache metadata，不代表即時資料或實際帳單。若命令有輸出而檢查頁顯示找不到 v74 cache，先確認 CodeBurn 使用的 `CODEBURN_CACHE_DIR`，再重新開啟整合頁。請參閱 [CodeBurn 官方文件](https://codeburn.app/agent/docs)。

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
