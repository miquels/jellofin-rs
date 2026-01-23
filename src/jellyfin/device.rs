use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::{AccessTokenRepo, UserRepo};
use crate::server::AppState;
use super::auth::get_user_id;

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceItem {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "LastUserId")]
    pub last_user_id: String,
    #[serde(rename = "LastUserName")]
    pub last_user_name: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "AppName")]
    pub app_name: String,
    #[serde(rename = "AppVersion")]
    pub app_version: String,
    #[serde(rename = "DateLastActivity")]
    pub date_last_activity: DateTime<Utc>,
    #[serde(rename = "Capabilities")]
    pub capabilities: DeviceCapabilities,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    #[serde(rename = "PlayableMediaTypes")]
    pub playable_media_types: Vec<String>,
    #[serde(rename = "SupportedCommands")]
    pub supported_commands: Vec<String>,
    #[serde(rename = "SupportsMediaControl")]
    pub supports_media_control: bool,
    #[serde(rename = "SupportsPersistentIdentifier")]
    pub supports_persistent_identifier: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfoResponse {
    #[serde(rename = "Items")]
    pub items: Vec<DeviceItem>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: usize,
    #[serde(rename = "StartIndex")]
    pub start_index: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceOptions {
    #[serde(rename = "DeviceId")]
    pub device_id: String,
    #[serde(rename = "CustomName")]
    pub custom_name: String,
    #[serde(rename = "DisableAutoLogin")]
    pub disable_auto_login: bool,
}

pub async fn get_devices(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> Result<Json<DeviceInfoResponse>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user = state.db.get_user_by_id(&user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let devices: Vec<DeviceItem> = tokens
        .into_iter()
        .map(|token| DeviceItem {
            id: token.device_id.clone(),
            last_user_id: token.user_id.clone(),
            last_user_name: user.username.clone(),
            name: token.device_name.clone(),
            app_name: token.app_name.clone(),
            app_version: token.app_version.clone(),
            date_last_activity: token.date_created,
            capabilities: DeviceCapabilities {
                playable_media_types: vec![],
                supported_commands: vec![],
                supports_media_control: false,
                supports_persistent_identifier: true,
            },
        })
        .collect();
    
    let count = devices.len();
    
    Ok(Json(DeviceInfoResponse {
        items: devices,
        total_record_count: count,
        start_index: 0,
    }))
}

pub async fn delete_device(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let device_id = params.get("id")
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    for token in tokens {
        if &token.device_id == device_id {
            state.db.delete_token(&token.token).await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(StatusCode::NO_CONTENT);
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn get_device_info(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<Json<DeviceItem>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let device_id = params.get("id")
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let user = state.db.get_user_by_id(&user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    for token in tokens {
        if &token.device_id == device_id {
            return Ok(Json(DeviceItem {
                id: token.device_id.clone(),
                last_user_id: token.user_id.clone(),
                last_user_name: user.username.clone(),
                name: token.device_name.clone(),
                app_name: token.app_name.clone(),
                app_version: token.app_version.clone(),
                date_last_activity: token.date_created,
                capabilities: DeviceCapabilities {
                    playable_media_types: vec![],
                    supported_commands: vec![],
                    supports_media_control: false,
                    supports_persistent_identifier: true,
                },
            }));
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn get_device_options(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<Json<DeviceOptions>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let device_id = params.get("id")
        .ok_or(StatusCode::BAD_REQUEST)?;
    
    let tokens = state.db.list_tokens_by_user(&user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    for token in tokens {
        if &token.device_id == device_id {
            return Ok(Json(DeviceOptions {
                device_id: token.device_id.clone(),
                custom_name: token.device_name.clone(),
                disable_auto_login: false,
            }));
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}
