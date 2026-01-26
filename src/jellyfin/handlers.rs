use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;

use crate::db::UserRepo;
use crate::server::AppState;
use super::auth::get_user_id;
use super::types::*;



fn get_default_user_data(item_id: &str) -> UserData {
    UserData {
        playback_position_ticks: 0,
        played_percentage: 0.0,
        play_count: 0,
        is_favorite: false,
        last_played_date: Some("0001-01-01T00:00:00Z".to_string()),
        played: false,
        key: item_id.to_string(),
        unplayed_item_count: Some(0),
    }
}

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

pub async fn system_info(State(state): State<AppState>) -> Json<SystemInfo> {
    Json(SystemInfo {
        server_name: state.config.jellyfin.server_name.clone(),
        version: "10.10.7".to_string(),
        id: state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string()),
        operating_system: std::env::consts::OS.to_string(),
    })
}

pub async fn public_system_info(State(state): State<AppState>) -> Json<PublicSystemInfo> {
    Json(PublicSystemInfo {
        server_name: state.config.jellyfin.server_name.clone(),
        version: "10.10.7".to_string(),
        id: state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string()),
    })
}

pub async fn plugins() -> Json<Vec<serde_json::Value>> {
    Json(vec![])
}

pub async fn display_preferences() -> Json<serde_json::Value> {
    Json(serde_json::json!({}))
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
        user_data: Some(get_default_user_data("collectionfavorites_f4a0b1c2d3e5c4b8a9e6f7d8e9a0b1c2")),
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
        user_data: Some(get_default_user_data("collectionplaylist_2f0340563593c4d98b97c9bfa21ce23c")),
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
    });
    
    Json(QueryResult {
        items,
        total_record_count: collections.len(),
    })
}

pub async fn get_items(
    State(state): State<AppState>,
    Query(params): Query<Vec<(String, String)>>,
) -> Json<QueryResult<BaseItemDto>> {
    let get_param = |key: &str| params.iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v);

    let parent_id = get_param("ParentId");
    let recursive = get_param("Recursive")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let limit = get_param("Limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);
    
    let mut include_item_types = Vec::new();
    for (key, value) in &params {
        if key.eq_ignore_ascii_case("IncludeItemTypes") {
            for t in value.split(',') {
                let t = t.trim();
                if !t.is_empty() {
                    include_item_types.push(t.to_string());
                }
            }
        }
    }
    
    let mut items = Vec::new();
    
    let ids_param = get_param("Ids").or_else(|| get_param("ids"));

    if let Some(ids_str) = ids_param {
        let requested_ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();
        let collections = state.collections.list_collections().await;
        
        for collection in &collections {
            // Check movies
            for movie in collection.movies.values() {
                if requested_ids.contains(&movie.id.as_str()) {
                    items.push(convert_movie_to_dto(movie, &collection.id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
            }
            
            // Check shows
            for show in collection.shows.values() {
                if requested_ids.contains(&show.id.as_str()) {
                    items.push(convert_show_to_dto(show, &collection.id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
                
                // Check seasons
                for season in show.seasons.values() {
                    if requested_ids.contains(&season.id.as_str()) {
                        items.push(convert_season_to_dto(season, &show.id, &collection.id, &show.name, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                    }
                    
                    // Check episodes
                    for episode in season.episodes.values() {
                        if requested_ids.contains(&episode.id.as_str()) {
                            items.push(convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                        }
                    }
                }
            }
        }
    } else if let Some(parent_id) = parent_id {
        // Get items from specific collection
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            if include_item_types.is_empty() || include_item_types.iter().any(|t| t.eq_ignore_ascii_case("Movie")) {
                for movie in collection.movies.values().take(limit) {
                    items.push(convert_movie_to_dto(movie, parent_id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
            }
            
            if include_item_types.is_empty() || include_item_types.iter().any(|t| t.eq_ignore_ascii_case("Series")) {
                for show in collection.shows.values().take(limit - items.len()) {
                    items.push(convert_show_to_dto(show, parent_id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
            }
        }
    } else if recursive {
        // Get items from all collections when recursive=true and no ParentId
        let collections = state.collections.list_collections().await;
        
        for collection in &collections {
            if items.len() >= limit {
                break;
            }
            
            if include_item_types.is_empty() || include_item_types.iter().any(|t| t.eq_ignore_ascii_case("Movie")) {
                for movie in collection.movies.values() {
                    if items.len() >= limit {
                        break;
                    }
                    items.push(convert_movie_to_dto(movie, &collection.id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
            }
            
            if include_item_types.is_empty() || include_item_types.iter().any(|t| t.eq_ignore_ascii_case("Series")) {
                for show in collection.shows.values() {
                    if items.len() >= limit {
                        break;
                    }
                    items.push(convert_show_to_dto(show, &collection.id, &state.config.jellyfin.server_id.clone().unwrap_or_default()));
                }
            }
        }
    }
    
    Json(QueryResult {
        total_record_count: items.len(),
        items,
    })
}

pub async fn get_episodes(
    State(state): State<AppState>,
    Path(show_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let season_id = params.get("SeasonId").or_else(|| params.get("seasonId"));
    let mut episodes = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
    
    // Scan all collections for the show
    let collections = state.collections.list_collections().await;
    for collection in collections {
        if let Some(show) = collection.shows.get(&show_id) {
            if let Some(sid) = season_id {
                // Return episodes for specific season
                let sid_int = sid.parse::<i32>().unwrap_or(-1);
                if let Some(season) = show.seasons.get(&sid_int) {
                    for episode in season.episodes.values() {
                        episodes.push(convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id));
                    }
                }
            } else {
                // Return all episodes from all seasons
                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        episodes.push(convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id));
                    }
                }
            }
            
            // Sort episodes: Season Asc, Episode Asc
            episodes.sort_by(|a, b| {
                let season_a = a.parent_index_number.unwrap_or(0);
                let season_b = b.parent_index_number.unwrap_or(0);
                if season_a != season_b {
                    season_a.cmp(&season_b)
                } else {
                    a.index_number.unwrap_or(0).cmp(&b.index_number.unwrap_or(0))
                }
            });
            
            return Ok(Json(QueryResult {
                total_record_count: episodes.len(),
                items: episodes,
            }));
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn get_item_by_id(
    State(state): State<AppState>,
    Path((_user_id, item_id)): Path<(String, String)>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            return Ok(Json(convert_movie_to_dto(movie, &collection.id, &server_id)));
        }
        
        if let Some(show) = collection.shows.get(&item_id) {
            return Ok(Json(convert_show_to_dto(show, &collection.id, &server_id)));
        }
        
        for show in collection.shows.values() {
            if let Some(season) = show.seasons.get(&item_id.parse::<i32>().unwrap_or(-1)) {
                return Ok(Json(convert_season_to_dto(season, &show.id, &collection.id, &show.name, &server_id)));
            }
            
            for season in show.seasons.values() {
                if let Some(episode) = season.episodes.get(&item_id.parse::<i32>().unwrap_or(-1)) {
                    return Ok(Json(convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id)));
                }
            }
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn get_latest_items(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<BaseItemDto>> {
    let parent_id = params.get("ParentId").or_else(|| params.get("parentId"));
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(16);
    
    let mut all_items = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
    
    if let Some(parent_id) = parent_id {
        // Get latest from specific collection
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            for movie in collection.movies.values() {
                all_items.push((movie.date_created, convert_movie_to_dto(movie, parent_id, &server_id)));
            }
            for show in collection.shows.values() {
                all_items.push((show.date_created, convert_show_to_dto(show, parent_id, &server_id)));
            }
        }
    } else {
        // Get latest from all collections
        let collections = state.collections.list_collections().await;
        for collection in collections {
            let coll_id = &collection.id;
            for movie in collection.movies.values() {
                all_items.push((movie.date_created, convert_movie_to_dto(movie, coll_id, &server_id)));
            }
            for show in collection.shows.values() {
                all_items.push((show.date_created, convert_show_to_dto(show, coll_id, &server_id)));
            }
        }
    }
    
    // Sort by date descending and take limit
    all_items.sort_by(|a, b| b.0.cmp(&a.0));
    let items = all_items.into_iter().take(limit).map(|(_, dto)| dto).collect();
    
    Json(items)
}

pub async fn get_item_counts(State(state): State<AppState>) -> Json<ItemCounts> {
    let collections = state.collections.list_collections().await;
    
    let mut movie_count = 0;
    let mut series_count = 0;
    let mut episode_count = 0;
    
    for collection in collections {
        movie_count += collection.movies.len();
        series_count += collection.shows.len();
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                episode_count += season.episodes.len();
            }
        }
    }
    
    Json(ItemCounts {
        movie_count,
        series_count,
        episode_count,
        album_count: 0,
        song_count: 0,
    })
}

pub async fn get_playback_info(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Response, StatusCode> {
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            let sources = movie.media_sources.iter()
                .map(|ms| {
                    let filename = ms.path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("video.mp4")
                        .to_string();
                    MediaSourceInfo {
                        id: item_id.clone(),
                        path: filename.clone(),
                        name: filename,
                        source_type: "Default".to_string(),
                        protocol: Some("File".to_string()),
                        container: ms.path.extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("mp4")
                            .to_string(),
                        video_type: Some("VideoFile".to_string()),
                        size: Some(ms.size as i64),
                        bitrate: ms.bitrate.map(|b| b as i32),
                        run_time_ticks: movie.runtime_ticks,
                        etag: Some(item_id.clone()),
                        is_remote: false,
                        supports_direct_stream: true,
                        supports_direct_play: true,
                        supports_transcoding: false,
                        media_streams: Some(vec![
                            MediaStream {
                                stream_type: "Video".to_string(),
                                codec: "h264".to_string(),
                                language: None,
                                index: Some(0),
                                width: Some(1920),
                                height: Some(1080),
                                bit_rate: Some(5000000),
                                is_default: Some(true),
                                codec_tag: None,
                                aspect_ratio: Some("16:9".to_string()),
                                profile: Some("High".to_string()),
                                time_base: None,
                                ref_frames: None,
                                is_anamorphic: None,
                                bit_depth: Some(8),
                                display_title: Some("1080p H264".to_string()),
                                video_range: Some("SDR".to_string()),
                                video_range_type: Some("SDR".to_string()),
                                audio_spatial_format: None,
                                localized_default: None,
                                localized_external: None,
                                channel_layout: None,
                                channels: None,
                                sample_rate: None,
                                level: None,
                                average_frame_rate: Some(24.0),
                                real_frame_rate: Some(24.0),
                                title: None,
                                is_external: Some(false),
                                is_text_subtitle_stream: Some(false),
                                supports_external_stream: Some(false),
                                pixel_format: Some("yuv420p".to_string()),
                                is_interlaced: Some(false),
                                is_avc: Some(true),
                                is_hearing_impaired: Some(false),
                                is_forced: Some(false),
                            },
                            MediaStream {
                                stream_type: "Audio".to_string(),
                                codec: "aac".to_string(),
                                language: Some("eng".to_string()),
                                index: Some(1),
                                width: None,
                                height: None,
                                bit_rate: Some(128000),
                                is_default: Some(true),
                                codec_tag: None,
                                aspect_ratio: None,
                                profile: Some("LC".to_string()),
                                time_base: None,
                                ref_frames: None,
                                is_anamorphic: None,
                                bit_depth: None,
                                display_title: Some("AAC - Stereo".to_string()),
                                video_range: None,
                                video_range_type: None,
                                audio_spatial_format: None,
                                localized_default: None,
                                localized_external: None,
                                channel_layout: Some("stereo".to_string()),
                                channels: Some(2),
                                sample_rate: Some(48000),
                                level: None,
                                average_frame_rate: None,
                                real_frame_rate: None,
                                title: None,
                                is_external: Some(false),
                                is_text_subtitle_stream: Some(false),
                                supports_external_stream: Some(false),
                                pixel_format: None,
                                is_interlaced: Some(false),
                                is_avc: Some(false),
                                is_hearing_impaired: Some(false),
                                is_forced: Some(false),
                            },
                        ]),
                        default_audio_stream_index: Some(1),
                        direct_stream_url: Some(format!("/Videos/{}/stream?mediaSourceId={}&static=true", item_id, item_id)),
                        transcoding_sub_protocol: Some("http".to_string()),
                        required_http_headers: None,
                        read_at_native_framerate: None,
                        has_segments: None,
                        ignore_dts: None,
                        ignore_index: None,
                        gen_pts_input: None,
                        is_infinite_stream: None,
                        requires_opening: None,
                        requires_closing: None,
                        requires_looping: None,
                        supports_probing: Some(true),
                        media_attachments: None,
                        formats: None,
                    }
                })
                .collect();
            
            let response = PlaybackInfoResponse { 
                media_sources: sources,
                play_session_id: "e3a869b7a901f8894de8ee65688db6c0".to_string(),
            };

            let bytes = serde_json::to_vec(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let len = bytes.len();
            
            return Response::builder()
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .header(axum::http::header::CONTENT_LENGTH, len.to_string())
                .body(axum::body::Body::from(bytes))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if episode.id == item_id {
                        let sources = episode.media_sources.iter()
                            .map(|ms| {
                                let filename = ms.path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("video.mp4")
                                    .to_string();
                                    MediaSourceInfo {
                                    id: item_id.clone(),
                                    path: filename.clone(),
                                    name: filename,
                                    source_type: "Default".to_string(),
                                    protocol: Some("File".to_string()),
                                    container: ms.path.extension()
                                        .and_then(|e| e.to_str())
                                        .unwrap_or("mp4")
                                        .to_string(),
                                    video_type: Some("VideoFile".to_string()),
                                    size: Some(ms.size as i64),
                                    bitrate: ms.bitrate.map(|b| b as i32),
                                    run_time_ticks: episode.runtime_ticks,
                                    etag: Some(item_id.clone()),
                                    is_remote: false,
                                    supports_direct_stream: true,
                                    supports_direct_play: true,
                                    supports_transcoding: false,
                                    media_streams: Some(vec![
                                        MediaStream {
                                            stream_type: "Video".to_string(),
                                            codec: "h264".to_string(),
                                            language: None,
                                            index: Some(0),
                                            width: Some(1920),
                                            height: Some(1080),
                                            bit_rate: Some(5000000),
                                            is_default: Some(true),
                                            codec_tag: None,
                                            aspect_ratio: Some("16:9".to_string()),
                                            profile: Some("High".to_string()),
                                            time_base: None,
                                            ref_frames: None,
                                            is_anamorphic: None,
                                            bit_depth: Some(8),
                                            display_title: Some("1080p H264".to_string()),
                                            video_range: Some("SDR".to_string()),
                                            video_range_type: Some("SDR".to_string()),
                                            audio_spatial_format: None,
                                            localized_default: None,
                                            localized_external: None,
                                            channel_layout: None,
                                            channels: None,
                                            sample_rate: None,
                                            level: None,
                                            average_frame_rate: Some(24.0),
                                            real_frame_rate: Some(24.0),
                                            title: None,
                                            is_external: Some(false),
                                            is_text_subtitle_stream: Some(false),
                                            supports_external_stream: Some(false),
                                            pixel_format: Some("yuv420p".to_string()),
                                            is_interlaced: Some(false),
                                            is_avc: Some(true),
                                            is_hearing_impaired: Some(false),
                                            is_forced: Some(false),
                                        },
                                        MediaStream {
                                            stream_type: "Audio".to_string(),
                                            codec: "aac".to_string(),
                                            language: Some("eng".to_string()),
                                            index: Some(1),
                                            width: None,
                                            height: None,
                                            bit_rate: Some(128000),
                                            is_default: Some(true),
                                            codec_tag: None,
                                            aspect_ratio: None,
                                            profile: Some("LC".to_string()),
                                            time_base: None,
                                            ref_frames: None,
                                            is_anamorphic: None,
                                            bit_depth: None,
                                            display_title: Some("AAC - Stereo".to_string()),
                                            video_range: None,
                                            video_range_type: None,
                                            audio_spatial_format: None,
                                            localized_default: None,
                                            localized_external: None,
                                            channel_layout: Some("stereo".to_string()),
                                            channels: Some(2),
                                            sample_rate: Some(48000),
                                            level: None,
                                            average_frame_rate: None,
                                            real_frame_rate: None,
                                            title: None,
                                            is_external: Some(false),
                                            is_text_subtitle_stream: Some(false),
                                            supports_external_stream: Some(false),
                                            pixel_format: None,
                                            is_interlaced: Some(false),
                                            is_avc: Some(false),
                                            is_hearing_impaired: Some(false),
                                            is_forced: Some(false),
                                        },
                                    ]),
                                    default_audio_stream_index: Some(1),
                                    direct_stream_url: Some(format!("/Videos/{}/stream?mediaSourceId={}&static=true", item_id, item_id)),
                                    transcoding_sub_protocol: Some("http".to_string()),
                                    required_http_headers: None,
                                    read_at_native_framerate: None,
                                    has_segments: None,
                                    ignore_dts: None,
                                    ignore_index: None,
                                    gen_pts_input: None,
                                    is_infinite_stream: None,
                                    requires_opening: None,
                                    requires_closing: None,
                                    requires_looping: None,
                                    supports_probing: Some(true),
                                    media_attachments: None,
                                    formats: None,
                                }
                            })
                            .collect();
                        
                        let response = PlaybackInfoResponse { 
                            media_sources: sources,
                            play_session_id: "e3a869b7a901f8894de8ee65688db6c0".to_string(),
                        };

                        let bytes = serde_json::to_vec(&response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                        let len = bytes.len();
                        
                        return Response::builder()
                            .header(axum::http::header::CONTENT_TYPE, "application/json")
                            .header(axum::http::header::CONTENT_LENGTH, len.to_string())
                            .body(axum::body::Body::from(bytes))
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn stream_video(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Response, StatusCode> {
    use axum::body::Body;
    use axum::http::header;
    use tokio::fs::File;
    use tokio_util::io::ReaderStream;
    
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            if let Some(ms) = movie.media_sources.first() {
                let file = File::open(&ms.path).await
                    .map_err(|_| StatusCode::NOT_FOUND)?;
                
                let stream = ReaderStream::new(file);
                let body = Body::from_stream(stream);
                
                return Ok((
                    [(header::CONTENT_TYPE, "video/mp4")],
                    body,
                ).into_response());
            }
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if episode.id == item_id {
                        if let Some(ms) = episode.media_sources.first() {
                            let file = File::open(&ms.path).await
                                .map_err(|_| StatusCode::NOT_FOUND)?;
                            
                            let stream = ReaderStream::new(file);
                            let body = Body::from_stream(stream);
                            
                            return Ok((
                                [(header::CONTENT_TYPE, "video/mp4")],
                                body,
                            ).into_response());
                        }
                    }
                }
            }
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn search_hints(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<QueryResult<SearchHint>> {
    let search_term = params.get("SearchTerm")
        .or_else(|| params.get("searchTerm"))
        .map(|s| s.as_str())
        .unwrap_or("");
    
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let results = state.collections.search(search_term, limit)
        .unwrap_or_default();
    
    let hints: Vec<SearchHint> = results.iter()
        .map(|r| SearchHint {
            item_id: r.id.clone(),
            name: "".to_string(),
            item_type: r.item_type.clone(),
            production_year: None,
        })
        .collect();
    
    let count = hints.len();
    Json(QueryResult {
        items: hints,
        total_record_count: count,
    })
}

pub async fn get_similar_items(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<QueryResult<BaseItemDto>> {
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    
    let results = state.collections.find_similar(&item_id, limit)
        .unwrap_or_default();
    
    let collections = state.collections.list_collections().await;
    let mut items = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
    
    for r in &results {
        for collection in &collections {
            if let Some(movie) = collection.movies.get(&r.id) {
                items.push(convert_movie_to_dto(movie, &collection.id, &server_id));
                break;
            }
            if let Some(show) = collection.shows.get(&r.id) {
                items.push(convert_show_to_dto(show, &collection.id, &server_id));
                break;
            }
        }
    }
    
    Json(QueryResult {
        total_record_count: items.len(),
        items,
    })
}


fn convert_media_sources(sources: &[crate::collection::MediaSource], item_id: &str) -> Option<Vec<MediaSourceInfo>> {
    if sources.is_empty() {
        return None;
    }
    
    Some(sources.iter().map(|s| {
        let filename = s.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("video.mp4")
            .to_string();
        MediaSourceInfo {
            id: item_id.to_string(),
            path: filename.clone(),
            name: filename,
            source_type: "Default".to_string(),
            protocol: Some("File".to_string()),
            container: s.container.clone(),
            video_type: Some("VideoFile".to_string()),
            size: Some(s.size as i64),
            bitrate: s.bitrate.map(|b| b as i32),
            run_time_ticks: None,
            etag: Some(item_id.to_string()),
            is_remote: false,
            supports_direct_stream: true,
            supports_direct_play: true,
            supports_transcoding: false,
            media_streams: Some(vec![
                crate::jellyfin::types::MediaStream {
                    stream_type: "Video".to_string(),
                    codec: "h264".to_string(),
                    language: Some("und".to_string()),
                    index: Some(0),
                    width: Some(1920),
                    height: Some(1080),
                    bit_rate: Some(5000000),
                    is_default: Some(true),
                    codec_tag: Some("avc1".to_string()),
                    aspect_ratio: Some("2.35:1".to_string()),
                    profile: Some("High".to_string()),
                    time_base: Some("1/16000".to_string()),
                    ref_frames: Some(1),
                    is_anamorphic: None,
                    bit_depth: Some(8),
                    display_title: Some("H264 - SDR".to_string()),
                    video_range: Some("SDR".to_string()),
                    video_range_type: Some("SDR".to_string()),
                    audio_spatial_format: Some("None".to_string()),
                    localized_default: None,
                    localized_external: None,
                    channel_layout: None,
                    channels: None,
                    sample_rate: None,
                    level: Some(0.0),
                    average_frame_rate: Some(24.0),
                    real_frame_rate: Some(24.0),
                    title: Some("H264".to_string()),
                    is_external: Some(false),
                    is_text_subtitle_stream: Some(false),
                    supports_external_stream: Some(false),
                    pixel_format: None,
                    is_interlaced: Some(false),
                    is_avc: Some(false),
                    is_hearing_impaired: Some(false),
                    is_forced: Some(false),
                },
                crate::jellyfin::types::MediaStream {
                    stream_type: "Audio".to_string(),
                    codec: "aac".to_string(),
                    language: Some("und".to_string()),
                    index: Some(1),
                    width: None,
                    height: None,
                    bit_rate: Some(128000),
                    is_default: Some(true),
                    codec_tag: Some("mp4a".to_string()),
                    aspect_ratio: None,
                    profile: Some("LC".to_string()),
                    time_base: Some("1/48000".to_string()),
                    ref_frames: None,
                    is_anamorphic: None,
                    bit_depth: None,
                    display_title: Some("Stereo - AAC".to_string()),
                    video_range: Some("Unknown".to_string()),
                    video_range_type: Some("Unknown".to_string()),
                    audio_spatial_format: Some("None".to_string()),
                    localized_default: Some("Default".to_string()),
                    localized_external: Some("External".to_string()),
                    channel_layout: Some("stereo".to_string()),
                    channels: Some(2),
                    sample_rate: Some(48000),
                    level: Some(0.0),
                    average_frame_rate: None,
                    real_frame_rate: None,
                    title: Some("Stereo".to_string()),
                    is_external: Some(false),
                    is_text_subtitle_stream: Some(false),
                    supports_external_stream: Some(false),
                    pixel_format: None,
                    is_interlaced: Some(false),
                    is_avc: Some(false),
                    is_hearing_impaired: Some(false),
                    is_forced: Some(false),
                },
            ]),
            default_audio_stream_index: Some(1),
            direct_stream_url: Some(format!("/Videos/{}/stream?mediaSourceId={}&static=true", item_id, item_id)),
            transcoding_sub_protocol: Some("http".to_string()),
            required_http_headers: Some(std::collections::HashMap::new()),
            read_at_native_framerate: Some(false),
            has_segments: Some(false),
            ignore_dts: Some(false),
            ignore_index: Some(false),
            gen_pts_input: Some(false),
            is_infinite_stream: Some(false),
            requires_opening: Some(false),
            requires_closing: Some(false),
            requires_looping: Some(false),
            supports_probing: Some(true),
            media_attachments: Some(vec![]),
            formats: Some(vec![]),
        }
    }).collect())
}

pub fn convert_movie_to_dto(movie: &crate::collection::Movie, parent_id: &str, server_id: &str) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if movie.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), movie.id.clone());
    }
    if movie.images.backdrop.is_some() {
        image_tags.insert("Backdrop".to_string(), movie.id.clone());
    }
    
    let backdrop_image_tags = if movie.images.backdrop.is_some() {
        Some(vec![movie.id.clone()])
    } else {
        None
    };
    
    let provider_ids = HashMap::new();
    
    BaseItemDto {
        name: movie.name.clone(),
        id: movie.id.clone(),
        item_type: "Movie".to_string(),
        collection_type: None,
        overview: movie.overview.clone(),
        production_year: movie.production_year,
        premiere_date: movie.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: movie.community_rating.map(|r| r as f32),
        runtime_ticks: movie.runtime_ticks,
        genres: Some(movie.genres.clone()),
        genre_items: Some(movie.genres.iter().map(|g| NameIdPair {
            name: g.clone(),
            id: format!("genre_{}", g), // Deterministic ID
        }).collect()),
        studios: Some(movie.studios.iter().map(|s| NameIdPair {
            name: s.clone(),
            id: s.clone(),
        }).collect()),
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: None,
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: Some("VideoFile".to_string()),
        width: Some(1920),
        height: Some(1080),
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: Some(true),
        is_4k: Some(false),
        is_folder: Some(false),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(movie.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&movie.id)),
        media_sources: convert_media_sources(&movie.media_sources, &movie.id),
        provider_ids: Some(provider_ids),
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(movie.name.to_lowercase()),
        forced_sort_name: Some(movie.name.to_lowercase()),
        original_title: Some(movie.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),

        taglines: movie.tagline.as_ref().map(|t| vec![t.clone()]),
        channel_id: None,
    }
}

pub fn convert_show_to_dto(show: &crate::collection::Show, parent_id: &str, server_id: &str) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if show.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), show.id.clone());
    }
    if show.images.backdrop.is_some() {
        image_tags.insert("Backdrop".to_string(), show.id.clone());
    }
    
    let backdrop_image_tags = if show.images.backdrop.is_some() {
        Some(vec![show.id.clone()])
    } else {
        None
    };
    
    let provider_ids = HashMap::new();
    
    BaseItemDto {
        name: show.name.clone(),
        id: show.id.clone(),
        item_type: "Series".to_string(),
        collection_type: None,
        overview: show.overview.clone(),
        production_year: show.production_year,
        premiere_date: show.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: show.community_rating.map(|r| r as f32),
        runtime_ticks: None,
        genres: Some(show.genres.clone()),
        studios: Some(show.studios.iter().map(|s| NameIdPair {
            name: s.clone(),
            id: s.clone(),
        }).collect()),
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: Some(show.seasons.len() as i32),
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(show.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&show.id)),
        media_sources: None,
        provider_ids: Some(provider_ids),
        recursive_item_count: Some(show.seasons.iter().map(|(_, s)| s.episodes.len() as i32).sum()),
        official_rating: Some("TV-MA".to_string()),
        sort_name: Some(show.name.to_lowercase()),
        forced_sort_name: Some(show.name.to_lowercase()),
        original_title: Some(show.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: Some(vec![]),
        channel_id: None,
        genre_items: Some(show.genres.iter().map(|g| NameIdPair {
            name: g.clone(),
            id: format!("genre_{}", g),
        }).collect()),
    }
}

pub fn convert_season_to_dto(season: &crate::collection::Season, show_id: &str, _parent_id: &str, series_name: &str, server_id: &str) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if season.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), season.id.clone());
    }
    
    BaseItemDto {
        name: season.name.clone(),
        id: season.id.clone(),
        item_type: "Season".to_string(),
        collection_type: None,
        overview: None,
        production_year: None,
        premiere_date: None,
        community_rating: None,
        runtime_ticks: None,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: Some(vec![]),
        has_subtitles: Some(true),
        parent_logo_item_id: Some(show_id.to_string()),
        parent_id: Some(season.id.to_string()),
        series_id: Some(show_id.to_string()),
        series_name: Some(series_name.to_string()),
        season_id: Some(season.id.to_string()),
        season_name: Some(season.name.to_string()),
        index_number: Some(season.season_number),
        parent_index_number: None,
        child_count: Some(season.episodes.len() as i32),
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: None,
        user_data: Some(get_default_user_data(&season.id)),
        media_sources: None,
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(season.name.to_lowercase()),
        forced_sort_name: Some(season.name.to_lowercase()),
        original_title: Some(season.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: None,
        channel_id: None,
        genre_items: None,
    }
}

pub fn convert_episode_to_dto(
    episode: &crate::collection::Episode,
    season_id: &str,
    show_id: &str,
    _parent_id: &str,
    season_name: &str,
    series_name: &str,
    server_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if episode.images.primary.is_some() || episode.images.thumb.is_some() {
        image_tags.insert("Primary".to_string(), episode.id.clone());
    }
    
    BaseItemDto {
        name: episode.name.clone(),
        id: episode.id.clone(),
        item_type: "Episode".to_string(),
        collection_type: None,
        overview: episode.overview.clone(),
        production_year: None,
        premiere_date: episode.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: episode.community_rating.map(|r| r as f32),
        runtime_ticks: episode.runtime_ticks,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: Some(vec![]),
        has_subtitles: Some(true),
        parent_logo_item_id: Some(show_id.to_string()),
        parent_id: Some(season_id.to_string()),
        series_id: Some(show_id.to_string()),
        series_name: Some(series_name.to_string()),
        season_id: Some(season_id.to_string()),
        season_name: Some(season_name.to_string()),
        index_number: Some(episode.episode_number),
        parent_index_number: Some(episode.season_number),
        child_count: None,
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        video_type: Some("VideoFile".to_string()),
        width: Some(1920),
        height: Some(1080),
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: Some(true),
        is_4k: Some(false),
        is_folder: Some(false),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(episode.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&episode.id)),
        media_sources: convert_media_sources(&episode.media_sources, &episode.id),
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(episode.name.to_lowercase()),
        forced_sort_name: Some(episode.name.to_lowercase()),
        original_title: Some(episode.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: None,
        channel_id: None,
        container: Some("mp4".to_string()),
        genre_items: None,
    }
}

pub async fn get_movie_recommendations(
    State(_state): State<AppState>,
    Query(_params): Query<HashMap<String, String>>,
    _req: Request<axum::body::Body>,
) -> Json<Vec<serde_json::Value>> {
    // Stub implementation - return empty list
    // TODO: Implement recommendation engine
    Json(vec![])
}
