use axum::{
    extract::{connect_info::ConnectInfo, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};

use crate::integrations;

/// Optional sources are inspected only when the browser explicitly requests
/// this endpoint after its local opt-in setting is enabled.
pub type OptionalIntegrationGateState = Arc<integrations::OptionalIntegrationGate>;

#[derive(Deserialize)]
pub struct OptionalIntegrationGateRequest {
    enabled: bool,
}

#[derive(Deserialize)]
pub struct MagpieUsageQuery {
    pub period: Option<String>,
    pub agent: Option<String>,
}

fn is_loopback_client(client: SocketAddr) -> bool {
    client.ip().is_loopback()
}

pub async fn set_optional_integrations_enabled(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    State(gate): State<OptionalIntegrationGateState>,
    Json(payload): Json<OptionalIntegrationGateRequest>,
) -> impl IntoResponse {
    if !is_loopback_client(client) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "選用整合只允許本機瀏覽器啟用" })),
        )
            .into_response();
    }
    gate.set_enabled(payload.enabled);
    Json(serde_json::json!({ "enabled": payload.enabled })).into_response()
}

pub async fn get_optional_integrations_status(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    State(gate): State<OptionalIntegrationGateState>,
) -> impl IntoResponse {
    if !is_loopback_client(client) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "選用整合只允許本機瀏覽器使用" })),
        )
            .into_response();
    }
    if !gate.is_enabled() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "尚未啟用選用整合檢查" })),
        )
            .into_response();
    }
    match tokio::task::spawn_blocking(integrations::inspect_optional_sources).await {
        Ok(status) => (StatusCode::OK, Json(status)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "整合來源檢查執行緒失敗" })),
        )
            .into_response(),
    }
}

pub async fn get_magpie_usage(
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    State(gate): State<OptionalIntegrationGateState>,
    Query(query): Query<MagpieUsageQuery>,
) -> impl IntoResponse {
    if !is_loopback_client(client) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "選用整合只允許本機瀏覽器使用" })),
        )
            .into_response();
    }
    if !gate.is_enabled() {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "尚未啟用選用整合檢查" })),
        )
            .into_response();
    }
    let Some(period) = integrations::MagpiePeriod::parse(query.period.as_deref()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "period 必須是 today、7d、30d、month 或 all" })),
        )
            .into_response();
    };
    let agent = query
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    match tokio::task::spawn_blocking(move || {
        integrations::load_magpie_usage(period, agent.as_deref())
    })
    .await
    {
        Ok(Ok(usage)) => (StatusCode::OK, Json(usage)).into_response(),
        Ok(Err(error)) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "Magpie 使用量檢查執行緒失敗" })),
        )
            .into_response(),
    }
}
