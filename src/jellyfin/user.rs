use axum::{
    extract::{Path, State},
    http::{Request, StatusCode},
    response::Response,
    Json,
};

use crate::db::UserRepo;
use crate::server::AppState;
use super::auth::get_user_id;
use super::types::*;

fn create_user_dto(user_id: String, username: String, server_id: String) -> UserDto {
    let now = chrono::Utc::now().to_rfc3339();
    
    UserDto {
        name: username,
        server_id,
        id: user_id,
        has_password: false,
        has_configured_password: false,
        has_configured_easy_password: false,
        enable_auto_login: false,
        last_login_date: now.clone(),
        last_activity_date: now,
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
    }
}

pub async fn get_users(State(state): State<AppState>) -> Result<Json<Vec<UserDto>>, StatusCode> {
    let users: Vec<crate::db::User> = vec![];
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string());
    
    let user_dtos: Vec<UserDto> = users
        .into_iter()
        .map(|u| create_user_dto(u.id, u.username, server_id.clone()))
        .collect();
    
    Ok(Json(user_dtos))
}

pub async fn get_current_user<B>(
    State(state): State<AppState>,
    req: Request<B>,
) -> Result<Json<UserDto>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let user = state.db.get_user_by_id(&user_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string());
    Ok(Json(create_user_dto(user.id, user.username, server_id)))
}

pub async fn get_user_image(
    State(_state): State<AppState>,
    Path((_user_id, _image_type)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    // Stub: Return 404 for user profile images for now, or a default avatar if we had one.
    // 404 is better than 500/routing error.
    Err(StatusCode::NOT_FOUND)
}

