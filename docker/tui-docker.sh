#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

IMAGE="${TUI_IMAGE:-token-usage-insights:local}"
PORT_START="${TUI_PORT_START:-3100}"
CONTAINER_PORT=3003
LABEL_PREFIX="io.github.fun-ed.token-usage-insights"
OWNER="fun-ed/TokenUsageInsights"
APP_BIN="/app/token-usage-insights"

usage() {
    cat <<EOF
TokenUsageInsights 本機容器管理

用法:
  $(basename "$0") <命令> [參數]

命令:
  up <名稱> [來源...] [--port <埠號>] [--no-open] [--rebuild]
      建立或啟動容器，依序匯入來源，並預設開啟看板。
      來源可為本機 JSON 檔案或 HTTPS SAS URL；也可用 --file / --url 指定。
  import <名稱> <來源...>      匯入資料到執行中的容器。
  open <名稱>                  以預設瀏覽器開啟看板。
  url <名稱>                   印出看板網址。
  export <名稱> [--out <路徑|HTTPS SAS URL>]
      匯出容器內的全部資料，預設輸出 ./<名稱>-export-all.json。
  logs <名稱> [-f]             檢視容器日誌。
  stop <名稱>                  停止容器，保留資料。
  rm <名稱> [--purge]          移除容器；加上 --purge 才刪除資料磁碟區。
  ls                           列出本腳本建立的容器。
  build [--no-cache]           建置映像檔 ${IMAGE}。
  help                         顯示此說明。

環境變數:
  TUI_IMAGE       映像檔名稱，預設 token-usage-insights:local。
  TUI_PORT_START  自動挑選本機埠號的起點，預設 3100。

範例:
  $(basename "$0") up alice ~/Downloads/alice-usage.json
  $(basename "$0") up bob 'https://storage.example.org/bob.json?<SAS>'
  $(basename "$0") import alice ./alice-usage.json
  $(basename "$0") rm bob --purge
EOF
}

die() {
    printf '錯誤：%s\n' "$*" >&2
    exit 1
}

info() {
    printf '%s\n' "$*"
}

require_docker() {
    command -v docker >/dev/null 2>&1 || die "找不到 docker 指令，請先安裝 Docker。"
    docker info >/dev/null 2>&1 || die "無法連線到 Docker daemon，請確認 Docker 已啟動。"
}

validate_name() {
    local name="${1:-}"
    [[ -n "$name" ]] || die "請指定名稱。"
    [[ "$name" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]*$ ]] || die "名稱格式無效，只能使用英數字、底線、點與連字號，且須以英數字開頭。"
}

validate_port() {
    local port="${1:-}"
    [[ "$port" =~ ^[0-9]{1,5}$ ]] || die "埠號無效，請指定 1 到 65535 之間的數字。"
    local numeric_port=$((10#$port))
    ((numeric_port >= 1 && numeric_port <= 65535)) || die "埠號無效，請指定 1 到 65535 之間的數字。"
}

container_name() {
    printf 'tui-%s\n' "$1"
}

volume_name() {
    printf 'tui-%s-data\n' "$1"
}

container_exists() {
    docker container inspect "$1" >/dev/null 2>&1
}

volume_exists() {
    docker volume inspect "$1" >/dev/null 2>&1
}

container_label() {
    docker container inspect -f "{{ index .Config.Labels \"${LABEL_PREFIX}.$2\" }}" "$1" 2>/dev/null
}

volume_label() {
    docker volume inspect -f "{{ index .Labels \"${LABEL_PREFIX}.$2\" }}" "$1" 2>/dev/null
}

assert_owned_container() {
    local container="$1" name="$2" volume="$3"
    if [[ "$(container_label "$container" managed-by)" != "$OWNER" \
        || "$(container_label "$container" name)" != "$name" \
        || "$(container_label "$container" volume)" != "$volume" ]]; then
        die "容器 $container 不屬於此管理工具，拒絕操作。"
    fi
    local port
    port="$(container_label "$container" port)"
    validate_port "$port"
}

assert_owned_volume() {
    local volume="$1" name="$2"
    if [[ "$(volume_label "$volume" managed-by)" != "$OWNER" \
        || "$(volume_label "$volume" name)" != "$name" ]]; then
        die "資料磁碟區 $volume 不屬於此管理工具，拒絕操作。"
    fi
}

require_container() {
    local container="$1" name="$2" volume
    container_exists "$container" || die "找不到容器 ${container}，請先執行 $(basename "$0") up ${name}。"
    volume="$(volume_name "$name")"
    assert_owned_container "$container" "$name" "$volume"
    volume_exists "$volume" || die "找不到容器 ${container} 的資料磁碟區，拒絕操作。"
    assert_owned_volume "$volume" "$name"
}

container_running() {
    [[ "$(docker container inspect -f '{{.State.Running}}' "$1" 2>/dev/null)" == "true" ]]
}

container_port() {
    container_label "$1" port
}

dashboard_url() {
    printf 'http://127.0.0.1:%s\n' "$1"
}

build_image() {
    local build_date
    build_date="$(git -C "$PROJECT_DIR" log -1 --format=%cs 2>/dev/null || true)"
    local build_args=()
    while (($# > 0)); do
        case "$1" in
            --no-cache) build_args+=(--no-cache); shift ;;
            *) die "build 不接受此參數。" ;;
        esac
    done
    info "建置映像檔 ${IMAGE}。首次建置需要在容器建置階段編譯 Rust。"
    docker build \
        -f "$SCRIPT_DIR/Dockerfile" \
        --build-arg "APP_BUILD_DATE=${build_date}" \
        "${build_args[@]+"${build_args[@]}"}" \
        -t "$IMAGE" \
        "$PROJECT_DIR"
}

ensure_image() {
    if ! docker image inspect "$IMAGE" >/dev/null 2>&1; then
        build_image
    fi
}

host_port_listening() {
    (exec 3<>"/dev/tcp/127.0.0.1/$1") >/dev/null 2>&1
}

port_reserved_by_container() {
    docker ps -a --filter "label=${LABEL_PREFIX}.port=$1" --format '{{.Names}}' | grep -q .
}

pick_free_port() {
    validate_port "$PORT_START"
    local port=$((10#$PORT_START))
    while ((port <= 65535)); do
        if ! host_port_listening "$port" && ! port_reserved_by_container "$port"; then
            printf '%s\n' "$port"
            return 0
        fi
        port=$((port + 1))
    done
    die "找不到可用的本機埠號。"
}

wait_until_ready() {
    local container="$1" attempt
    for ((attempt = 0; attempt < 60; attempt++)); do
        if docker exec "$container" wget -q -O /dev/null "http://127.0.0.1:${CONTAINER_PORT}/api/version" >/dev/null 2>&1; then
            return 0
        fi
        if ! container_running "$container"; then
            die "容器 $container 啟動失敗。請執行 $(basename "$0") logs ${container#tui-} 查看日誌。"
        fi
        sleep 1
    done
    die "等待容器 $container 啟動逾時。請執行 $(basename "$0") logs ${container#tui-} 查看日誌。"
}

import_file() {
    local container="$1" file="$2"
    [[ -f "$file" ]] || die "找不到指定的本機檔案。"
    local staging="/tmp/import-$$-$RANDOM"
    local base status=0
    base="$(basename "$file")"
    info "匯入本機檔案 $file"
    docker exec "$container" mkdir -p "$staging" || status=$?
    if ((status == 0)); then docker cp "$file" "$container:$staging/$base" >/dev/null || status=$?; fi
    if ((status == 0)); then docker exec -u root "$container" chown -R app:app "$staging" || status=$?; fi
    if ((status == 0)); then docker exec "$container" "$APP_BIN" import --file "$staging/$base" || status=$?; fi
    docker exec "$container" rm -rf "$staging" >/dev/null 2>&1 || true
    ((status == 0)) || die "匯入本機檔案失敗，結束碼 ${status}。"
}

import_url() {
    local container="$1" url="$2" status=0
    info "匯入 HTTPS SAS 網址。"
    # Never print child output here. Import diagnostics may echo the signed query string.
    docker exec "$container" "$APP_BIN" import --url "$url" >/dev/null 2>&1 || status=$?
    ((status == 0)) || die "匯入 HTTPS 網址失敗，結束碼 ${status}。請檢查網址與存取權限。"
}

import_source() {
    local container="$1" source="$2"
    case "$source" in
        https://*) import_url "$container" "$source" ;;
        http://*) die "匯入網址必須使用 HTTPS。" ;;
        *) import_file "$container" "$source" ;;
    esac
}

open_browser() {
    local url="$1"
    if [[ "$(uname -s)" == "Darwin" ]] && command -v open >/dev/null 2>&1; then
        open "$url"
    elif command -v xdg-open >/dev/null 2>&1; then
        xdg-open "$url" >/dev/null 2>&1 &
    else
        printf '請手動以瀏覽器開啟：%s\n' "$url"
    fi
}

cmd_up() {
    local name="${1:-}"
    validate_name "$name"
    shift

    local port="" no_open=0 rebuild=0
    local sources=()
    while (($# > 0)); do
        case "$1" in
            --port)
                (($# >= 2)) || die "--port 需要指定埠號。"
                port="$2"
                shift 2
                ;;
            --file|--url)
                (($# >= 2)) || die "$1 需要指定值。"
                sources+=("$2")
                shift 2
                ;;
            --no-open) no_open=1; shift ;;
            --rebuild) rebuild=1; shift ;;
            -*) die "up 包含無效參數。" ;;
            *) sources+=("$1"); shift ;;
        esac
    done
    if [[ -n "$port" ]]; then
        validate_port "$port"
        port=$((10#$port))
    fi

    require_docker
    local container volume existing=0
    container="$(container_name "$name")"
    volume="$(volume_name "$name")"

    if container_exists "$container"; then
        assert_owned_container "$container" "$name" "$volume"
        volume_exists "$volume" || die "找不到容器 $container 的資料磁碟區，拒絕啟動。"
        assert_owned_volume "$volume" "$name"
        existing=1
    elif volume_exists "$volume"; then
        assert_owned_volume "$volume" "$name"
    fi

    if ((rebuild)); then
        build_image
    else
        ensure_image
    fi

    if ((existing)); then
        local existing_port
        existing_port="$(container_port "$container")"
        if [[ -n "$port" && "$port" != "$existing_port" ]]; then
            die "容器 ${container} 已固定使用埠號 ${existing_port}。請先執行 rm ${name}，再以新的埠號建立容器，資料會保留。"
        fi
        port="$existing_port"
        if ((rebuild)); then
            info "映像檔已重建，既有容器仍使用原映像檔。若要套用新映像檔，請先移除容器再重新建立，資料會保留。"
        fi
        if container_running "$container"; then
            info "容器 $container 已在執行中。"
        else
            info "啟動既有容器 ${container}。"
            docker start "$container" >/dev/null
        fi
    else
        if [[ -z "$port" ]]; then
            port="$(pick_free_port)"
        elif host_port_listening "$port" || port_reserved_by_container "$port"; then
            die "本機埠號 $port 已被使用，請改用其他埠號或省略 --port。"
        fi
        if ! volume_exists "$volume"; then
            docker volume create \
                --label "${LABEL_PREFIX}.managed-by=${OWNER}" \
                --label "${LABEL_PREFIX}.name=${name}" \
                "$volume" >/dev/null
        fi
        assert_owned_volume "$volume" "$name"
        info "建立容器 ${container}，資料磁碟區 ${volume}，本機埠號 ${port}。"
        docker run -d \
            --name "$container" \
            --label "${LABEL_PREFIX}.managed-by=${OWNER}" \
            --label "${LABEL_PREFIX}.name=${name}" \
            --label "${LABEL_PREFIX}.volume=${volume}" \
            --label "${LABEL_PREFIX}.port=${port}" \
            -v "$volume:/data" \
            -p "127.0.0.1:${port}:${CONTAINER_PORT}" \
            "$IMAGE" >/dev/null
    fi

    wait_until_ready "$container"

    local source
    for source in "${sources[@]+"${sources[@]}"}"; do
        import_source "$container" "$source"
    done

    local url
    url="$(dashboard_url "$port")"
    printf '\n%s 的看板已就緒：%s\n' "$name" "$url"
    if ((no_open == 0)); then open_browser "$url"; fi
}

cmd_import() {
    local name="${1:-}"
    validate_name "$name"
    shift
    (($# > 0)) || die "請至少指定一個匯入來源。"

    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    container_running "$container" || die "容器 ${container} 未執行，請先執行 up ${name}。"

    local source
    for source in "$@"; do import_source "$container" "$source"; done
    printf '匯入完成。看板網址：%s\n' "$(dashboard_url "$(container_port "$container")")"
}

cmd_open() {
    local name="${1:-}"
    validate_name "$name"
    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    container_running "$container" || die "容器 ${container} 未執行，請先執行 up ${name}。"
    open_browser "$(dashboard_url "$(container_port "$container")")"
}

cmd_url() {
    local name="${1:-}"
    validate_name "$name"
    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    dashboard_url "$(container_port "$container")"
}

cmd_export() {
    local name="${1:-}"
    validate_name "$name"
    shift
    local out="./${name}-export-all.json"
    while (($# > 0)); do
        case "$1" in
            --out)
                (($# >= 2)) || die "--out 需要指定路徑或 HTTPS SAS URL。"
                out="$2"
                shift 2
                ;;
            *) die "export 包含無效參數。" ;;
        esac
    done

    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    container_running "$container" || die "容器 ${container} 未執行，請先執行 up ${name}。"

    case "$out" in
        https://*)
            local status=0
            # The CLI may echo the signed URL in diagnostics or success output.
            docker exec "$container" "$APP_BIN" export-all --out "$out" >/dev/null 2>&1 || status=$?
            ((status == 0)) || die "匯出至 HTTPS 網址失敗，結束碼 ${status}。請檢查網址與寫入權限。"
            info "已匯出至指定的 HTTPS Blob 路徑。"
            ;;
        http://*) die "匯出網址必須使用 HTTPS。" ;;
        *)
            local staging="/tmp/export-$$-$RANDOM.json" status=0
            docker exec "$container" "$APP_BIN" export-all --out "$staging" || status=$?
            if ((status == 0)); then docker cp "$container:$staging" "$out" >/dev/null || status=$?; fi
            docker exec "$container" rm -f "$staging" >/dev/null 2>&1 || true
            ((status == 0)) || die "匯出檔案失敗，結束碼 ${status}。"
            printf '已匯出到 %s\n' "$out"
            ;;
    esac
}

cmd_logs() {
    local name="${1:-}"
    validate_name "$name"
    shift
    local args=()
    while (($# > 0)); do
        case "$1" in
            -f) args+=(-f); shift ;;
            *) die "logs 只接受 -f 參數。" ;;
        esac
    done
    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    docker logs "${args[@]+"${args[@]}"}" "$container"
}

cmd_stop() {
    local name="${1:-}"
    validate_name "$name"
    require_docker
    local container
    container="$(container_name "$name")"
    require_container "$container" "$name"
    docker stop "$container" >/dev/null
    info "已停止容器 ${container}，資料保留在磁碟區 $(volume_name "$name")。"
}

cmd_rm() {
    local name="${1:-}"
    validate_name "$name"
    shift
    local purge=0
    while (($# > 0)); do
        case "$1" in
            --purge) purge=1; shift ;;
            *) die "rm 只接受 --purge 參數。" ;;
        esac
    done

    require_docker
    local container volume has_container=0 has_volume=0
    container="$(container_name "$name")"
    volume="$(volume_name "$name")"
    if container_exists "$container"; then
        assert_owned_container "$container" "$name" "$volume"
        has_container=1
    fi
    if volume_exists "$volume"; then
        assert_owned_volume "$volume" "$name"
        has_volume=1
    fi

    if ((has_container)); then
        docker rm -f "$container" >/dev/null
        info "已移除容器 ${container}。"
    else
        info "容器 $container 不存在，略過。"
    fi
    if ((purge)); then
        if ((has_volume)); then
            docker volume rm "$volume" >/dev/null
            info "已刪除資料磁碟區 ${volume}。"
        fi
    else
        info "資料磁碟區 ${volume} 保留；下次執行 up ${name} 會沿用。"
    fi
}

cmd_ls() {
    (($# == 0)) || die "ls 不接受參數。"
    require_docker
    docker ps -a \
        --filter "label=${LABEL_PREFIX}.managed-by=${OWNER}" \
        --filter "label=${LABEL_PREFIX}.name" \
        --format "table {{.Label \"${LABEL_PREFIX}.name\"}}\t{{.Names}}\t{{.Status}}\thttp://127.0.0.1:{{.Label \"${LABEL_PREFIX}.port\"}}"
}

main() {
    local command="${1:-help}"
    (($# == 0)) || shift
    case "$command" in
        up) cmd_up "$@" ;;
        import) cmd_import "$@" ;;
        open) cmd_open "$@" ;;
        url) cmd_url "$@" ;;
        export) cmd_export "$@" ;;
        logs) cmd_logs "$@" ;;
        stop) cmd_stop "$@" ;;
        rm) cmd_rm "$@" ;;
        ls) cmd_ls "$@" ;;
        build) require_docker; build_image "$@" ;;
        help|-h|--help) usage ;;
        *) usage >&2; exit 2 ;;
    esac
}

main "$@"
