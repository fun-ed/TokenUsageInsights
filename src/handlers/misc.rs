use axum::{
    extract::{rejection::JsonRejection, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use super::{is_supported_assistant, is_supported_report_assistant, normalize_assistant_name};

use crate::db::{self, SessionPricingError, UsageDayExportRecord};
use crate::pricing::load_pricing_entries;

#[derive(Serialize)]
struct UsageDayExportResponse {
    version: u8,
    assistant: String,
    date: String,
    exported_at: String,
    records: Vec<UsageDayExportRecord>,
}

#[derive(Serialize)]
pub struct AppVersionResponse {
    pub version: &'static str,
}

pub async fn get_app_version() -> Json<AppVersionResponse> {
    Json(AppVersionResponse {
        version: crate::APP_VERSION,
    })
}

#[derive(Deserialize)]
pub struct UsageDayImportRequest {
    #[serde(default)]
    pub assistant: Option<String>,
    #[serde(default)]
    pub confirmed_assistant: Option<String>,
    #[serde(default)]
    pub source_file_name: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub records: Vec<UsageDayExportRecord>,
}

/// API 7: 獲取模型價格清單 ( pricing.csv 資訊)
pub async fn get_pricing(Path(assistant): Path<String>) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_report_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    match tokio::task::spawn_blocking(load_pricing_entries).await {
        Ok(entries) => Json(entries).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "執行緒執行失敗" })),
        )
            .into_response(),
    }
}

fn deserialize_required_nullable_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Deserialize)]
pub struct SessionPricingRequest {
    session_id: String,
    source_kind: String,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    source_dir_key: Option<String>,
    #[serde(deserialize_with = "deserialize_required_nullable_string")]
    pricing_model: Option<String>,
}

fn session_pricing_error_response(error: SessionPricingError) -> Response {
    let status = if error.is_bad_request() {
        StatusCode::BAD_REQUEST
    } else if error.is_not_found() {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (
        status,
        Json(serde_json::json!({ "error": error.message() })),
    )
        .into_response()
}

pub async fn set_session_pricing(
    Path(assistant): Path<String>,
    payload: Result<Json<SessionPricingRequest>, JsonRejection>,
) -> Response {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    let Json(payload) = match payload {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("無效的工作階段定價請求: {error}")
                })),
            )
                .into_response();
        }
    };

    let assistant_type = assistant;
    let source_kind = payload.source_kind;
    let source_dir_key = payload.source_dir_key;
    let session_id = payload.session_id;
    let pricing_model = payload.pricing_model;
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = db::get_db_conn().map_err(SessionPricingError::Database)?;
        db::set_session_pricing_assignment(
            &mut conn,
            &assistant_type,
            &source_kind,
            source_dir_key.as_deref(),
            &session_id,
            pricing_model.as_deref(),
        )
    })
    .await
    .unwrap_or_else(|error| {
        Err(SessionPricingError::Database(format!(
            "工作階段定價執行緒失敗: {error}"
        )))
    });

    match result {
        Ok(()) => Json(serde_json::json!({ "status": "ok" })).into_response(),
        Err(error) => session_pricing_error_response(error),
    }
}

/// API 8: 手動觸發日誌增量同步
pub async fn trigger_manual_sync(Path(assistant): Path<String>) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_report_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    let sync_res = tokio::task::spawn_blocking(|| {
        if let Ok(mut conn) = db::get_db_conn() {
            db::sync_usage_logs(&mut conn)
        } else {
            Err("無法連接至 SQLite 資料庫".to_string())
        }
    })
    .await;

    match sync_res {
        Ok(Ok(_)) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "success", "message": "手動增量同步已成功完成！" })),
        )
            .into_response(),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": format!("同步失敗: {}", e) })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": "執行緒執行失敗" })),
        )
            .into_response(),
    }
}

/// API: 獲取 Codex 的 rate limit 資料
pub async fn get_rate_limit(Path(assistant): Path<String>) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    if assistant != "codex" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Only codex is supported" })),
        )
            .into_response();
    }

    let res = tokio::task::spawn_blocking(db::get_latest_codex_rate_limit)
        .await
        .unwrap();

    match res {
        Some(val) => (StatusCode::OK, Json(val)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "No rate limit data found" })),
        )
            .into_response(),
    }
}

fn is_valid_date(date: &str) -> bool {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    let year: i32 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let month: i32 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let day: i32 = match parts[2].parse() {
        Ok(v) => v,
        Err(_) => return false,
    };

    if year <= 0 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return false;
    }

    true
}

fn is_valid_period(period: &str) -> bool {
    match period.len() {
        4 => period.parse::<i32>().is_ok_and(|year| year > 0),
        7 => {
            let Some((year, month)) = period.split_once('-') else {
                return false;
            };
            year.parse::<i32>().is_ok_and(|year| year > 0)
                && month
                    .parse::<i32>()
                    .is_ok_and(|month| (1..=12).contains(&month))
        }
        10 => is_valid_date(period),
        _ => false,
    }
}

fn validate_import_assistant(
    route_assistant: &str,
    payload_assistant: Option<&str>,
    confirmed_assistant: Option<&str>,
) -> Result<Option<String>, String> {
    let confirmed_assistant = confirmed_assistant
        .map(normalize_assistant_name)
        .filter(|assistant| is_supported_assistant(assistant))
        .ok_or_else(|| "匯入前必須明確確認目標助理類型".to_string())?;
    if confirmed_assistant != route_assistant {
        return Err(format!(
            "已確認的目標助理 {confirmed_assistant} 與 API 路徑 {route_assistant} 不一致"
        ));
    }

    let Some(payload_assistant) = payload_assistant else {
        return Ok(None);
    };
    if payload_assistant.trim().is_empty() {
        return Err("匯入檔案的 assistant 欄位不可為空".to_string());
    }
    let payload_assistant = normalize_assistant_name(payload_assistant);
    if !is_supported_assistant(&payload_assistant) {
        return Err(format!("匯入檔案包含不支援的助理類型：{payload_assistant}"));
    }
    if payload_assistant != route_assistant {
        return Err(format!(
            "匯入檔案屬於 {payload_assistant}，不得匯入至 {route_assistant}"
        ));
    }
    Ok(Some(payload_assistant))
}

pub async fn export_usage_day(
    Path((assistant, date)): Path<(String, String)>,
) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    if !is_valid_period(&date) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "資料範圍格式不正確，請使用 YYYY、YYYY-MM 或 YYYY-MM-DD" })),
        )
            .into_response();
    }

    let assistant_clone = assistant.clone();
    let date_clone = date.clone();
    let export_res = tokio::task::spawn_blocking(move || {
        let conn = db::get_db_conn()?;
        let records = db::export_usage_period_entries(&conn, &assistant_clone, &date_clone)?;
        Ok::<Vec<crate::db::UsageDayExportRecord>, String>(records)
    })
    .await
    .unwrap_or_else(|_| Err("導出任務執行失敗".to_string()));

    match export_res {
        Ok(records) => {
            if records.is_empty() {
                (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "error": "指定資料範圍沒有可匯出的使用紀錄" })),
                )
                    .into_response()
            } else {
                let payload = UsageDayExportResponse {
                    version: 1,
                    assistant: assistant.clone(),
                    date: date.clone(),
                    exported_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
                    records,
                };
                (StatusCode::OK, Json(payload)).into_response()
            }
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": err })),
        )
            .into_response(),
    }
}

pub async fn import_usage_day(
    Path((assistant, date)): Path<(String, String)>,
    Json(payload): Json<UsageDayImportRequest>,
) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    let source_assistant = match validate_import_assistant(
        &assistant,
        payload.assistant.as_deref(),
        payload.confirmed_assistant.as_deref(),
    ) {
        Ok(value) => value,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": err })),
            )
                .into_response();
        }
    };

    let import_date = payload
        .date
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(date);

    if payload.records.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "匯入資料為空" })),
        )
            .into_response();
    }

    let assistant_clone = assistant.clone();
    let import_date_clone = import_date.clone();
    let records = payload.records;
    let source_file_name = payload.source_file_name;
    let import_res = tokio::task::spawn_blocking(move || {
        let mut conn = db::get_db_conn()?;
        let summary = db::import_usage_day_entries(
            &mut conn,
            &assistant_clone,
            &import_date_clone,
            records,
            db::UsageImportMetadata {
                source_assistant,
                source_file_name,
            },
        )?;
        Ok::<crate::db::UsageDayImportSummary, String>(summary)
    })
    .await
    .unwrap_or_else(|_| Err("匯入任務執行失敗".to_string()));

    match import_res {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(err) => {
            let status = if err.contains("日期") || err.contains("無效") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(serde_json::json!({ "error": err }))).into_response()
        }
    }
}

pub async fn get_usage_import_batches(Path(assistant): Path<String>) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }

    let result = tokio::task::spawn_blocking(move || {
        let conn = db::get_db_conn()?;
        db::list_usage_import_batches(&conn, &assistant, 50)
    })
    .await
    .unwrap_or_else(|_| Err("匯入紀錄查詢任務執行失敗".to_string()));

    match result {
        Ok(batches) => (StatusCode::OK, Json(batches)).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
    }
}

pub async fn rollback_usage_import_batch(
    Path((assistant, batch_id)): Path<(String, String)>,
) -> impl IntoResponse {
    let assistant = normalize_assistant_name(&assistant);
    if !is_supported_assistant(&assistant) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "不支援的助理類型" })),
        )
            .into_response();
    }
    if batch_id.is_empty()
        || batch_id.len() > 128
        || !batch_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "匯入批次 ID 格式不正確" })),
        )
            .into_response();
    }

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = db::get_db_conn()?;
        db::rollback_usage_import_batch(&mut conn, &assistant, &batch_id)
    })
    .await
    .unwrap_or_else(|_| Err("撤銷匯入任務執行失敗".to_string()));

    match result {
        Ok(summary) => (StatusCode::OK, Json(summary)).into_response(),
        Err(error) => {
            let status = if error.contains("找不到") {
                StatusCode::NOT_FOUND
            } else if error.contains("已撤銷") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(serde_json::json!({ "error": error }))).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::Json;

    use super::{get_app_version, is_valid_period, validate_import_assistant};

    #[test]
    fn session_pricing_requires_explicit_nullable_fields() {
        let complete = serde_json::json!({
            "session_id": "session",
            "source_kind": "omp-session",
            "source_dir_key": null,
            "pricing_model": null
        });
        let request: super::SessionPricingRequest =
            serde_json::from_value(complete.clone()).unwrap();
        assert!(request.source_dir_key.is_none());
        assert!(request.pricing_model.is_none());
        for field in ["source_dir_key", "pricing_model"] {
            let mut incomplete = complete.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<super::SessionPricingRequest>(incomplete).is_err());
        }
    }

    #[tokio::test]
    async fn app_version_uses_cargo_package_version() {
        let Json(response) = get_app_version().await;

        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn export_period_accepts_day_month_and_year() {
        assert!(is_valid_period("2026-08-01"));
        assert!(is_valid_period("2026-08"));
        assert!(is_valid_period("2026"));
        assert!(!is_valid_period("2026-13"));
        assert!(!is_valid_period("all"));
    }

    #[test]
    fn import_requires_explicit_matching_target_confirmation() {
        let missing = validate_import_assistant("codex", Some("codex"), None).unwrap_err();
        assert_eq!(missing, "匯入前必須明確確認目標助理類型");

        let mismatch =
            validate_import_assistant("codex", Some("codex"), Some("copilot")).unwrap_err();
        assert!(mismatch.contains("與 API 路徑 codex 不一致"));
    }

    #[test]
    fn import_rejects_payload_assistant_mismatch() {
        let error = validate_import_assistant("antigravity", Some("codex"), Some("antigravity"))
            .unwrap_err();
        assert_eq!(error, "匯入檔案屬於 codex，不得匯入至 antigravity");
    }

    #[test]
    fn import_accepts_matching_alias_and_legacy_payload_without_assistant() {
        assert_eq!(
            validate_import_assistant("claude", Some("claude-code"), Some("claude")).unwrap(),
            Some("claude".to_string())
        );
        assert_eq!(
            validate_import_assistant("cursor", None, Some("cursor")).unwrap(),
            None
        );
    }
}
