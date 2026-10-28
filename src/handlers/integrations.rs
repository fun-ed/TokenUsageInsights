use axum::{
    extract::{connect_info::ConnectInfo, State},
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
