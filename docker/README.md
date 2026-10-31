# 以 Docker 分開檢視用量資料

`docker/` 提供一個本機容器映像檔與管理腳本。每個名稱使用自己的容器、資料磁碟區與本機埠號。容器只讀取明確匯入的 JSON，不會讀取主機上的 AI 工具紀錄，也不會與原生執行的 TokenUsageInsights 共用資料庫。

## 需求

- macOS 或 Linux 主機上的 Docker Desktop 或 Docker Engine，並支援 BuildKit。
- Bash。
- 首次建置會在 Docker 的官方 Rust 映像檔內編譯程式，不需要在主機安裝 Rust；主機上不會執行 Cargo。

這是選用功能，不會隨一般安裝或啟動流程自動執行。

## 快速開始

在專案根目錄執行：

```bash
# 匯入 JSON 檔，並預設開啟看板
mise exec -- docker/tui-docker.sh up alice ~/Downloads/alice-usage.json

# 直接匯入 HTTPS SAS URL。請用單引號包住網址，避免 shell 解讀 &
mise exec -- docker/tui-docker.sh up bob 'https://storage.example.org/bob.json?<SAS>'
```

`up` 會在映像檔不存在時建置映像，建立 `tui-<名稱>` 容器與 `tui-<名稱>-data` 資料磁碟區，從 3100 開始挑選可用埠號，等待服務就緒後依序匯入來源。看板只綁定 `127.0.0.1`。匯入 HTTPS 網址時，腳本不會輸出網址或子程式訊息，因此 userinfo 與 SAS query 不會出現在終端機輸出或錯誤訊息中。

不開啟瀏覽器或指定埠號：

```bash
mise exec -- docker/tui-docker.sh up alice ./alice-usage.json --no-open
mise exec -- docker/tui-docker.sh up carol ./carol.json --port 3200
```

## 命令

| 命令 | 說明 |
| --- | --- |
| `up <名稱> [來源...] [--port <埠號>] [--no-open] [--rebuild]` | 建立或啟動容器，再匯入來源。來源可為本機 JSON 檔案或 HTTPS 網址，也可用 `--file`、`--url` 明確指定。 |
| `import <名稱> <來源...>` | 匯入資料到執行中的容器。重複匯入會依程式規則去重。 |
| `open <名稱>` | 以預設瀏覽器開啟看板。 |
| `url <名稱>` | 印出看板網址。 |
| `export <名稱> [--out <路徑\|HTTPS SAS URL>]` | 匯出容器內的全部資料，預設輸出 `./<名稱>-export-all.json`。 |
| `logs <名稱> [-f]` | 檢視容器日誌。 |
| `stop <名稱>` | 停止容器並保留資料。 |
| `rm <名稱> [--purge]` | 移除容器。只有加上 `--purge` 才會刪除資料磁碟區。 |
| `ls` | 列出此管理工具建立的容器、狀態與網址。 |
| `build [--no-cache]` | 建置映像檔。 |

`TUI_IMAGE` 可設定映像檔名稱，預設為 `token-usage-insights:local`。`TUI_PORT_START` 可設定自動挑選埠號的起點，預設為 `3100`。

## 使用 SAS 匯出到 Blob Storage

`export --out` 請使用包含完整 Blob 路徑與固定檔名的 Blob SAS URL，query 必須指定 `sr=b`。若使用容器範圍的 SAS，CLI 會自行產生時間戳檔名；管理腳本會隱藏 CLI 輸出以免洩漏 URL，因此無法顯示該檔名。

```bash
mise exec -- docker/tui-docker.sh export alice --out 'https://storage.example.org/alice-usage.json?<SAS-with-sr-b>'
```

使用固定 Blob 路徑時，匯出檔名就是 URL 中指定的 `alice-usage.json`。請以安全管道保存完整 SAS URL。

## 常見操作

```bash
# 列出容器
mise exec -- docker/tui-docker.sh ls

# 後續匯入新月份資料
mise exec -- docker/tui-docker.sh import alice ./alice-2026-11.json

# 重新啟動已停止的容器，沿用原本的埠號
mise exec -- docker/tui-docker.sh up alice

# 停止容器但保留資料
mise exec -- docker/tui-docker.sh stop alice

# 刪除容器但保留資料磁碟區
mise exec -- docker/tui-docker.sh rm alice

# 刪除容器與資料磁碟區
mise exec -- docker/tui-docker.sh rm alice --purge
```

專案更新後，重新建置映像檔，再移除並重建容器即可套用新程式。未加 `--purge` 的 `rm` 會保留資料；若要沿用原埠號，請在 `up` 指定原本埠號。

```bash
mise exec -- docker/tui-docker.sh build
mise exec -- docker/tui-docker.sh rm alice
mise exec -- docker/tui-docker.sh up alice --port 3100
```

## 手動使用 Docker

建置內容必須是專案根目錄，Dockerfile 的白名單忽略規則會排除本機資料與其他不相關檔案：

```bash
mise exec -- docker build -f docker/Dockerfile -t token-usage-insights:local .
mise exec -- docker volume create \
  --label io.github.fun-ed.token-usage-insights.managed-by=fun-ed/TokenUsageInsights \
  --label io.github.fun-ed.token-usage-insights.name=alice \
  tui-alice-data
mise exec -- docker run -d --name tui-alice \
  --label io.github.fun-ed.token-usage-insights.managed-by=fun-ed/TokenUsageInsights \
  --label io.github.fun-ed.token-usage-insights.name=alice \
  --label io.github.fun-ed.token-usage-insights.volume=tui-alice-data \
  --label io.github.fun-ed.token-usage-insights.port=3100 \
  -v tui-alice-data:/data \
  -p 127.0.0.1:3100:3003 \
  token-usage-insights:local
mise exec -- docker cp alice-usage.json tui-alice:/tmp/alice-usage.json
mise exec -- docker exec tui-alice /app/token-usage-insights import --file /tmp/alice-usage.json
```

使用手動命令時，請自行建立並加上相同管理標籤的具名資料磁碟區，否則管理腳本會拒絕接管該資源。

## 隔離與安全

- 容器資料存放在專屬具名磁碟區 `/data`，環境變數 `INSIGHTS_DIR=/data`。每個名稱的資料互相分開。
- 映像檔沒有主機上的 AI 工具資料，也不掛載主機家目錄。容器只透過 `import` 收到指定資料。
- 主機埠只綁定 `127.0.0.1`，不對區域網路開放。
- 程式以容器內的 `app` 非 root 使用者執行。
- 啟動命令包含 `--no-auto-update`，且映像檔不放 `VERSION` 檔。要更新程式，請重新建置映像檔。
- 容器與磁碟區都帶有 `fun-ed/TokenUsageInsights` 管理標籤。若同名資源缺少正確標籤，腳本會拒絕啟動、匯入、停止或刪除，避免碰觸其他用途的資源。
- HTTPS 匯入與匯出時，腳本會隱藏命令輸出和錯誤內容，避免應用程式診斷訊息印出 SAS query。成功與失敗訊息只說明操作結果，不包含簽章。

## 疑難排解

- **無法連線到 Docker daemon**：啟動 Docker Desktop 或 Docker Engine 後再執行腳本。
- **本機埠號已被使用**：改用其他 `--port`，或省略該參數讓腳本自動挑選。
- **容器或磁碟區不屬於此管理工具**：腳本不會接管同名但缺少 `fun-ed/TokenUsageInsights` 標籤的資源。請選擇其他名稱，或自行檢查並處理該資源。
- **HTTPS 匯入或匯出失敗**：確認網址指向 JSON 檔案，SAS 尚未過期，並有讀取或寫入權限。錯誤輸出刻意不顯示 SAS query。
- **看板沒有資料**：執行 `logs <名稱>` 查看服務日誌，並確認匯入的 JSON 有可用資料。
