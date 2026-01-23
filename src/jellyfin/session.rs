use axum::{
    extract::State,
    http::{Request, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::{AccessTokenRepo, UserRepo};
use crate::server::AppState;
use super::auth::get_user_id;

const SESSION_ID: &str = "e3a869b7a901f8894de8ee65688db6c0";

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfo {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "UserId")]
    pub user_id: String,
    #[serde(rename = "UserName")]
    pub user_name: String,
    #[serde(rename = "LastActivityDate")]
    pub last_activity_date: DateTime<Utc>,
    #[serde(rename = "RemoteEndPoint")]
    pub remote_end_point: String,
    #[serde(rename = "DeviceName")]
    pub device_name: String,
    #[serde(rename = "DeviceId")]
    pub device_id: String,
    #[serde(rename = "Client")]
    pub client: String,
    #[serde(rename = "ApplicationVersion")]
    pub application_version: String,
    #[serde(rename = "IsActive")]
    pub is_active: bool,
    #[serde(rename = "SupportsMediaControl")]
    pub supports_media_control: bool,
    #[serde(rename = "SupportsRemoteControl")]
    pub supports_remote_control: bool,
    #[serde(rename = "HasCustomDeviceName")]
    pub has_custom_device_name: bool,
    #[serde(rename = "ServerId")]
    pub server_id: String,
    #[serde(rename = "AdditionalUsers")]
    pub additional_users: Vec<String>,
    #[serde(rename = "PlayState")]
    pub play_state: SessionPlayState,
    #[serde(rename = "Capabilities")]
    pub capabilities: SessionCapabilities,
    #[serde(rename = "NowPlayingQueue")]
    pub now_playing_queue: Vec<String>,
    #[serde(rename = "NowPlayingQueueFullItems")]
    pub now_playing_queue_full_items: Vec<String>,
    #[serde(rename = "SupportedCommands")]
    pub supported_commands: Vec<String>,
    #[serde(rename = "PlayableMediaTypes")]
    pub playable_media_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionPlayState {
    #[serde(rename = "RepeatMode")]
    pub repeat_mode: String,
    #[serde(rename = "PlaybackOrder")]
    pub playback_order: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionCapabilities {
    #[serde(rename = "PlayableMediaTypes")]
    pub playable_media_types: Vec<String>,
    #[serde(rename = "SupportedCommands")]
    pub supported_commands: Vec<String>,
    #[serde(rename = "SupportsPersistentIdentifier")]
    pub supports_persistent_identifier: bool,
}

pub async fn get_sessions(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> Result<Json<Vec<SessionInfo>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user = state.db.get_user_by_id(&user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Keep most recent token per device ID only
    let mut unique_tokens = HashMap::new();
    for token in tokens {
        let device_id = token.device_id.clone();
        if let Some(&existing) = unique_tokens.get(&device_id) {
            if token.date_created > existing {
                unique_tokens.insert(device_id, token.date_created);
            }
        } else {
            unique_tokens.insert(device_id, token.date_created);
        }
    }
    
    // Get tokens again and filter to unique ones
    let all_tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let mut sessions = Vec::new();
    for token in all_tokens {
        if let Some(&last_activity) = unique_tokens.get(&token.device_id) {
            if token.date_created == last_activity {
                sessions.push(SessionInfo {
                    id: SESSION_ID.to_string(),
                    user_id: token.user_id.clone(),
                    user_name: user.username.clone(),
                    last_activity_date: token.date_created,
                    remote_end_point: String::new(),
                    device_name: token.device_name.clone(),
                    device_id: token.device_id.clone(),
                    client: token.app_name.clone(),
                    application_version: token.app_version.clone(),
                    is_active: true,
                    supports_media_control: false,
                    supports_remote_control: false,
                    has_custom_device_name: false,
                    server_id: state.config.jellyfin.server_id.clone()
                        .unwrap_or_else(|| "jellyfin-rs".to_string()),
                    additional_users: vec![],
                    play_state: SessionPlayState {
                        repeat_mode: "RepeatNone".to_string(),
                        playback_order: "Default".to_string(),
                    },
                    capabilities: SessionCapabilities {
                        playable_media_types: vec![],
                        supported_commands: vec![],
                        supports_persistent_identifier: true,
                    },
                    now_playing_queue: vec![],
                    now_playing_queue_full_items: vec![],
                    supported_commands: vec![],
                    playable_media_types: vec![],
                });
            }
        }
    }
    
    Ok(Json(sessions))
}

pub async fn post_session_capabilities(
    State(_state): State<AppState>,
    _req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    // Accept but ignore capabilities
    Ok(StatusCode::NO_CONTENT)
}

pub async fn post_session_capabilities_full(
    State(_state): State<AppState>,
    _req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    // Accept but ignore capabilities
    Ok(StatusCode::NO_CONTENT)
}
