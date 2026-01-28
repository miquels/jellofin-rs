use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};

use super::types::*;
use crate::server::AppState;

pub async fn system_info(State(state): State<AppState>) -> Json<SystemInfo> {
    Json(SystemInfo {
        server_name: state.config.jellyfin.server_name.clone(),
        version: "10.10.7".to_string(),
        id: state
            .config
            .jellyfin
            .server_id
            .clone()
            .unwrap_or_else(|| "jellyfin-rs".to_string()),
        operating_system: std::env::consts::OS.to_string(),
    })
}

pub async fn public_system_info(State(state): State<AppState>) -> Json<PublicSystemInfo> {
    Json(PublicSystemInfo {
        server_name: state.config.jellyfin.server_name.clone(),
        version: "10.10.7".to_string(),
        id: state
            .config
            .jellyfin
            .server_id
            .clone()
            .unwrap_or_else(|| "jellyfin-rs".to_string()),
    })
}

pub async fn plugins() -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

pub async fn display_preferences() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
}

pub async fn system_ping_handler() -> impl IntoResponse {
    (StatusCode::OK, "\"Jellyfin Server\"")
}

pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-cache, no-store")],
        "Healthy",
    )
}
