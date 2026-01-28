use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::{Request, StatusCode},
    response::Response,
    Json,
};

use super::auth::get_user_id;
use super::types::*;
use crate::db::UserRepo;
use crate::jellyfin::userdata::get_default_user_data;
use crate::server::AppState;

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
    let server_id = state
        .config
        .jellyfin
        .server_id
        .clone()
        .unwrap_or_else(|| "jellyfin-rs".to_string());

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

    let user = state
        .db
        .get_user_by_id(&user_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let server_id = state
        .config
        .jellyfin
        .server_id
        .clone()
        .unwrap_or_else(|| "jellyfin-rs".to_string());
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

pub async fn get_user_views(State(state): State<AppState>) -> Json<QueryResult<BaseItemDto>> {
    let collections = state.collections.list_collections().await;

    let mut items: Vec<BaseItemDto> = collections
        .iter()
        .map(|c| {
            // Convert "shows" to "tvshows" for Jellyfin API compatibility
            let collection_type = match c.collection_type.as_str() {
                "shows" => "tvshows",
                other => other,
            };

            BaseItemDto {
                name: c.name.clone(),
                id: c.id.clone(),
                item_type: "CollectionFolder".to_string(),
                collection_type: Some(collection_type.to_string()),
                overview: None,
                production_year: None,
                premiere_date: None,
                community_rating: None,
                runtime_ticks: None,
                genres: None,
                studios: None,
                people: None, // Keep as None for non-items
                chapters: None,
                has_subtitles: None,
                parent_logo_item_id: None,
                parent_id: None,
                series_id: None,
                series_name: None,
                season_id: None,
                season_name: None,
                index_number: None,
                parent_index_number: None,
                child_count: Some(c.item_count() as i32),
                image_tags: HashMap::new(),
                backdrop_image_tags: None,
                primary_image_aspect_ratio: None,
                server_id: None,
                container: None,
                video_type: None,
                width: None,
                height: None,
                image_blur_hashes: None,
                media_type: None,
                is_hd: None,
                is_4k: None,
                is_folder: Some(true),
                location_type: Some("FileSystem".to_string()),
                path: None,
                etag: None,
                date_created: None,
                user_data: Some(get_default_user_data(&c.id)),
                media_sources: None,
                provider_ids: None,
                recursive_item_count: None,
                official_rating: None,
                sort_name: Some(c.name.to_lowercase()),
                forced_sort_name: Some(c.name.to_lowercase()),
                original_title: Some(c.name.clone()),
                can_delete: Some(false),
                can_download: Some(false),
                taglines: None,
                channel_id: None,
                genre_items: None,
                play_access: Some("Full".to_string()),
                enable_media_source_display: Some(false),
            }
        })
        .collect();

    // Add Favorites virtual collection
    items.push(BaseItemDto {
        name: "Favorites".to_string(),
        id: "collectionfavorites_f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2".to_string(),
        item_type: "CollectionFolder".to_string(),
        collection_type: Some("playlists".to_string()),
        overview: None,
        production_year: None,
        premiere_date: None,
        community_rating: None,
        runtime_ticks: None,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: None,
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: Some(0),
        image_tags: HashMap::new(),
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: None,
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: None,
        user_data: Some(get_default_user_data(
            "collectionfavorites_f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2",
        )),
        media_sources: None,
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some("favorites".to_string()),
        forced_sort_name: Some("favorites".to_string()),
        original_title: Some("Favorites".to_string()),
        can_delete: Some(false),
        can_download: Some(false),
        taglines: None,
        channel_id: None,
        genre_items: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    });

    // Add Playlists virtual collection
    items.push(BaseItemDto {
        name: "Playlists".to_string(),
        id: "collectionplaylist_2f0340563593c4d98b97c9bfa21ce23c".to_string(),
        item_type: "CollectionFolder".to_string(),
        collection_type: Some("playlists".to_string()),
        overview: None,
        production_year: None,
        premiere_date: None,
        community_rating: None,
        runtime_ticks: None,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: None,
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: Some(0),
        image_tags: HashMap::new(),
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: None,
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: None,
        user_data: Some(get_default_user_data(
            "collectionplaylist_2f0340563593c4d98b97c9bfa21ce23c",
        )),
        media_sources: None,
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some("playlists".to_string()),
        forced_sort_name: Some("playlists".to_string()),
        original_title: Some("Playlists".to_string()),
        can_delete: Some(false),
        can_download: Some(false),
        taglines: None,
        channel_id: None,
        genre_items: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    });

    Json(QueryResult {
        items,
        total_record_count: collections.len(),
        start_index: 0,
    })
}

pub async fn get_grouping_options(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    // Return list of collections as grouping options, similar to Go's behavior
    let collections = state.collections.list_collections().await;
    let options: Vec<serde_json::Value> = collections
        .iter()
        .map(|c| {
            serde_json::json!({
                "Id": c.id,
                "Name": c.name
            })
        })
        .collect();

    Json(options)
}
