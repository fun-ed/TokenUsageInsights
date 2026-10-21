# 模型快取費率查核與修正

查核日期：2026-10-21。以下價格為官方直連 API、美元／每 100 萬 tokens，不代表訂閱方案、第三方供應商或實際帳單。

## 隨附牌價修正

| 模型 | 輸入 | 快取讀取 | 輸出 | 採用方式 |
|---|---:|---:|---:|---|
| Gemini 4 Argon | 2.00 | 0.10 | 10.00 | 至 2026-11-30 的 Standard 優惠價，包含 Low／Medium／High 標籤 |
| MiniMax-M3.1-Flash | 0.15 | 0.015 | 0.60 | 官方單一費率 |
| DeepSeek V4.1-Flash-1017 | 0.30 | 0.006 | 1.20 | 尖峰費率估算，離峰實際費率為一半 |
| Claude Fable 5.1／Mythos 5.1 | 10.00 | 0.25 | 50.00 | 快取讀取為輸入價的 2.5% |
| Claude Fable 5／Mythos 5 | 10.00 | 1.00 | 50.00 | 保留舊版 10% 快取讀取比例，避免錯配新版 |

`pricing.csv` 新增 MiniMax 與 Claude 條目，補齊 `deepseek-flash`、`deepseek-v4.1-flash-1017`、`deepseek-v4-flash-vision-exp`，並將 `deepseek-v4-flash` 的現行估算費率更新為 Flash 尖峰價。模型比對沿用既有大小寫、標點正規化及較具體版本優先規則，不新增 API、資料庫欄位或相依套件。

## Gemini 4 Argon

[Google 官方定價](https://ai.google.dev/gemini-api/docs/pricing#gemini-4-argon)列明：

| 期間 | Standard 輸入／快取讀取／輸出 | Batch 輸入／快取讀取／輸出 |
|---|---|---|
| 至 2026-11-30 | 2.00／0.10／10.00 | 1.00／0.05／5.00 |
| 自 2026-12-01 | 4.00／0.20／20.00 | 2.00／0.10／10.00 |

Argon 只支援 implicit caching，不支援 explicit caching，因此不套用 Claude 的快取寫入加價，也不新增 explicit cache 儲存費。本次修正 Standard 與既有 Batch 顯示欄位，沒有新增 Flex、Priority、Contributor 方案辨識。

隨附 CSV 是查核當日的牌價快照，不會在 12 月 1 日自動切換，也不依每筆歷史請求日期選價。屆時需更新牌價；不能提前把未來價格套到目前的估算。

## DeepSeek

[現行官方定價](https://api-docs.deepseek.com/quick_start/pricing/)將 `deepseek-flash` 對應到 V4.1-Flash-1017。`deepseek-v4-flash`、`deepseek-v4-flash-vision-exp` 已改路由到新版本，按現行 Flash 價格計費。專案額外接受版本標籤 `deepseek-v4.1-flash-1017`，供本機記錄比對使用；不表示官方 API 接受此版本標籤作為請求 ID。

Flash 離峰輸入／快取讀取／輸出為 0.15／0.003／0.60，尖峰為 0.30／0.006／1.20。尖峰是週一至週五 01:00–04:00、06:00–10:00 UTC，中國法定假日除外。專案目前沒有分時牌價或中國假日判定，隨附規則採尖峰價；若請求實際全部屬離峰，純 token 估算可能為官方直連實際費用的兩倍。

[官方快取文件](https://api-docs.deepseek.com/guides/kv_cache)說明快取自動啟用，按 `prompt_cache_hit_tokens`／`prompt_cache_miss_tokens` 計量，不保證命中。未使用後通常於數小時至數天內清除，沒有固定或可指定的 12 小時 TTL；現行價目表未列獨立快取寫入費。

[1017 發布公告](https://api-docs.deepseek.com/news/news261017)確認新版本上線，未明確宣布同日調整 token 費率。[9 月 10 日公告](https://api-docs.deepseek.com/news/news260910)才明確記錄新價格於 2026-09-10 04:00 UTC 生效。

### Pro 費率與路由仍未確認

現行定價頁列 `deepseek-v4-pro` 為 V4-Pro-0813：尖峰輸入／快取讀取／輸出 1.32／0.044／3.96，離峰 0.66／0.022／1.98。但 9 月 10 日公告說，自 2026-09-14 04:00 UTC 起，Pro 請求暫時改由 V4.1-Flash 處理並按 Flash 價計費，直到 V4.1-Pro 上線。兩份官方文件沒有一致交代當前實際路由。

因此本次不把 `deepseek-v4-pro` 猜測改為 Pro 新牌價或 Flash 價。CSV 保留原有 0.435／0.003625／0.87；這是尚未確認且與現行價目頁不一致的舊估算值，不應視為目前可靠的官方費率。需以實際回傳模型、平台扣款紀錄或官方澄清確認後再改。

## MiniMax M3.1-Flash

[2026-10-19 官方發布公告](https://www.minimax.io/blog/minimax-m31f)及[官方定價頁](https://platform.minimax.io/docs/pricing/overview#models/text)均列輸入／快取讀取／輸出 0.15／0.015／0.60。

[快取文件](https://platform.minimax.io/docs/api-reference/text-prompt-caching)說明自動前綴快取適用於至少 512 個輸入 tokens 的請求。新輸入按一般輸入價計費，沒有獨立寫入加價或公布可設定的 TTL。Anthropic 相容 API 的 `input_tokens` 是未快取的新輸入，OpenAI 相容 API 的 `prompt_tokens` 則包含快取讀取，不能重複計費。本次不更動既有來源解析器。

## Claude

[官方定價](https://platform.claude.com/docs/en/about-claude/pricing)及[快取文件](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)仍列 5 分鐘與 1 小時 TTL，寫入費分別為一般輸入價的 1.25 倍與 2 倍。命中會刷新保存期限，仍收讀取費，但不再收一次寫入費。文件沒有列出 12 小時 TTL。

| 模型 | 輸入 | 5m 寫入 | 1h 寫入 | 快取讀取 | 輸出 |
|---|---:|---:|---:|---:|---:|
| Fable 5.1／Mythos 5.1 | 10.00 | 12.50 | 20.00 | 0.25 | 50.00 |
| Opus 5.5 | 4.00 | 5.00 | 8.00 | 0.20 | 20.00 |
| Sonnet 5.5 | 2.00 | 2.50 | 4.00 | 0.10 | 10.00 |
| Sonnet 5 | 2.00 | 2.50 | 4.00 | 0.20 | 10.00 |
| Haiku 5.5，prompt ≤100K | 0.10 | 0.125 | 0.20 | 0.01 | 0.50 |
| Haiku 5.5，prompt >100K | 0.50 | 0.625 | 1.00 | 0.05 | 2.50 |

既有 Opus／Sonnet／Haiku 費率及 Claude 寫入倍率已符合此表，不重寫計價公式。Haiku 的 prompt 閾值包含一般輸入、快取讀取與快取寫入，不含輸出。

## 實作邊界

- 不修改或刪除原始逐字稿、SQLite 用量、模型名稱與來源回報的費用，不執行資料遷移。
- 沒有來源費用而需重新估算的歷史報表，會沿用既有行為套用目前規則。更新舊 DeepSeek 別名與 Gemini 牌價可能改變這類歷史估算，不能當作歷史帳單重建。
- models.dev 快取與既有價格選取優先序不變。隨附 CSV 的回歸測試排除 models.dev，避免測試受本機快取影響；這不代表執行中服務一定使用 CSV。
- 本次不新增分時／歷史價格引擎、不變更來源解析器、手動 session 計價選單、UI 或發行版本，也不安裝或重啟服務。
- 使用者提供的 OpenAI Responses API 12 小時快取變更是調查背景，本次未獨立驗證，也未修改任何供應商 API 請求參數。
