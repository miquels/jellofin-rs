// Authentication specs:
// Emby - https://dev.emby.media/doc/restapi/User-Authentication.html.
// Jellyfin - https://gist.github.com/nielsvanvelzen/ea047d9028f676185832e51ffaf12a6f

use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use bcrypt;

use crate::db::{AccessToken, User, UserRepo, AccessTokenRepo};
use crate::server::AppState;
use crate::util::{QueryParams, generate_id};
use super::types::*;

pub async fn authenticate_by_name(
    State(state): State<AppState>,
    Json(req): Json<AuthenticationRequest>,
) -> Result<Json<AuthenticationResult>, StatusCode> {
    let username = req.username.trim().to_lowercase();

    let now = chrono::Utc::now();
    let now_text = now.to_rfc3339();

    let user = match state.db.get_user(&username).await {
        Ok(user) => {
            let auth_ok = match bcrypt::verify(&req.pw, &user.password) {
                Ok(value) => value,
                Err(_) => false,
            };
            if !auth_ok {
                return Err(StatusCode::UNAUTHORIZED);
            }
            user
        },
        Err(_) => {
            if state.config.jellyfin.autoregister {
                let new_user = User {
                    id: generate_id(&username),
                    username: username.to_string(),
                    password: bcrypt::hash(&req.pw, bcrypt::DEFAULT_COST)
                                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    created: Some(now_text.clone()),
                    lastlogin: None,
                    lastused: None,
                };
                state.db.upsert_user(&new_user).await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                new_user
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    };
    
    let token = AccessToken {
        token: uuid::Uuid::new_v4().to_string(),
        userid: user.id.clone(),
        deviceid: None,
        devicename: None,
        applicationname: None,
        applicationversion: None,
        remoteaddress: None,
        created: Some(now),
        lastused: Some(now),
    };
    
    state.db.upsert_token(&token).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let session_id = "e3a869b7a901f8894de8ee65688db6c0"; // Hardcoded session ID matching Go implementation
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string());
    
    let result = AuthenticationResult {
        user: UserDto {
            name: user.username.clone(),
            server_id: server_id.clone(),
            id: user.id.clone(),
            has_password: false,
            has_configured_password: false,
            has_configured_easy_password: false,
            enable_auto_login: false,
            last_login_date: now_text.clone(),
            last_activity_date: now_text.clone(),
            configuration: UserConfiguration {
                grouped_folders: vec![],
                subtitle_mode: "Default".to_string(),
                ordered_views: vec![],
                my_media_excludes: vec![],
                latest_items_excludes: vec![],
                subtitle_language_preference: String::new(),
                cast_receiver_id: String::new(),
                play_default_audio_track: true,
                display_missing_episodes: false,
                display_collections_view: false,
                enable_local_password: false,
                hide_played_in_latest: false,
                remember_audio_selections: true,
                remember_subtitle_selections: true,
                enable_next_episode_auto_play: false,
            },
            policy: UserPolicy {
                is_administrator: false,
                is_hidden: false,
                enable_collection_management: false,
                enable_subtitle_management: false,
                enable_lyric_management: false,
                is_disabled: false,
                blocked_tags: vec![],
                allowed_tags: vec![],
                enable_user_preference_access: false,
                access_schedules: vec![],
                block_unrated_items: vec![],
                enable_remote_control_of_other_users: false,
                enable_shared_device_control: false,
                enable_remote_access: true,
                enable_live_tv_management: false,
                enable_live_tv_access: false,
                enable_media_playback: true,
                enable_audio_playback_transcoding: false,
                enable_video_playback_transcoding: false,
                enable_playback_remuxing: false,
                force_remote_source_transcoding: false,
                enable_content_deletion: false,
                enable_content_deletion_from_folders: vec![],
                enable_content_downloading: true,
                enable_sync_transcoding: false,
                enable_media_conversion: false,
                enabled_devices: vec![],
                enable_all_devices: true,
                enabled_channels: vec![],
                enable_all_channels: false,
                enabled_folders: vec![],
                enable_all_folders: true,
                invalid_login_attempt_count: 0,
                login_attempts_before_lockout: 0,
                max_active_sessions: 0,
                enable_public_sharing: false,
                blocked_media_folders: vec![],
                blocked_channels: vec![],
                remote_client_bitrate_limit: 0,
                authentication_provider_id: "DefaultAuthenticationProvider".to_string(),
                password_reset_provider_id: "DefaultPasswordResetProvider".to_string(),
                sync_play_access: "CreateAndJoinGroups".to_string(),
            },
        },
        session_info: AuthSessionInfo {
            play_state: PlayState {
                can_seek: false,
                is_paused: false,
                is_muted: false,
                repeat_mode: "RepeatNone".to_string(),
                playback_order: "Default".to_string(),
            },
            additional_users: vec![],
            capabilities: Capabilities {
                playable_media_types: vec![],
                supported_commands: vec![],
                supports_media_control: false,
                supports_persistent_identifier: true,
            },
            remote_end_point: String::new(),
            playable_media_types: vec![],
            id: session_id.to_string(),
            user_id: user.id.clone(),
            user_name: user.username.clone(),
            client: String::new(),
            last_activity_date: now_text.clone(),
            last_playback_check_in: "0001-01-01T00:00:00Z".to_string(),
            device_name: String::new(),
            device_id: String::new(),
            application_version: String::new(),
            is_active: true,
            supports_media_control: false,
            supports_remote_control: false,
            now_playing_queue: vec![],
            now_playing_queue_full_items: vec![],
            has_custom_device_name: false,
            server_id: server_id.clone(),
            supported_commands: vec![],
        },
        access_token: token.token.clone(),
        server_id,
    };
    
    Ok(Json(result))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token(&req, &params);
    
    if let Some(token_str) = token {
        if let Ok(token) = state.db.get_token(&token_str).await {
            req.extensions_mut().insert(token.userid.clone());
        }
    }
    
    Ok(next.run(req).await)
}

fn extract_token<B>(req: &Request<B>, params: &QueryParams) -> Option<String> {
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = parse_emby_auth(auth_str) {
                return Some(token);
            }
        }
    }
    
    if let Some(auth_header) = req.headers().get("X-Emby-Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = parse_emby_auth(auth_str) {
                return Some(token);
            }
        }
    }
    
    if let Some(token) = req.headers().get("X-Emby-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string()) {
        return Some(token);
    }
    
    if let Some(token) = req.headers().get("X-MediaBrowser-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string()) {
        return Some(token);
    }
    
    if let Some(token) = params.get("ApiKey").or_else(|| params.get("api_key")) {
        return Some(token.to_string());
    }
    
    None
}

fn parse_emby_auth(auth_str: &str) -> Option<String> {
    for part in auth_str.split(',') {
        let part = part.trim();
        if let Some(token_part) = part.strip_prefix("Token=") {
            return Some(token_part.trim_matches('"').to_string());
        }
    }
    None
}

pub fn get_user_id<B>(req: &Request<B>) -> Option<String> {
    req.extensions().get::<String>().cloned()
}

use axum::response::IntoResponse;

pub async fn quick_connect_enabled(
    State(_state): State<AppState>,
) -> Json<bool> {
    // Stub: Quick Connect not yet fully implemented
    Json(false)
}

pub async fn quick_connect_initiate(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED.into_response()
}

pub async fn quick_connect_authorize(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED.into_response()
}

pub async fn quick_connect_connect(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED.into_response()
}
