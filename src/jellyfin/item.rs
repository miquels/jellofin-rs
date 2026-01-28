use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{self, Request, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::auth::get_user_id;
use super::types::*;
use super::userdata::get_default_user_data;
use crate::collection::find_image_path;
use crate::collection::item::MediaSource;
use crate::collection::repo::FoundItem;
use crate::db::UserDataRepo;
use crate::server::AppState;
use crate::util::QueryParams;

pub async fn get_item_ancestors(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<Vec<BaseItemDto>> {
    // Stub: Returning empty list for now.
    // Real implementation requires traversing up the tree (Episode -> Season -> Series -> Collection)
    Json(vec![])
}

pub async fn get_items(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Json<QueryResult<BaseItemDto>> {
    let parent_id = params.get("parentId");
    let recursive = params
        .get("recursive")
        .and_then(|s| s.parse::<bool>().ok())
        .unwrap_or(false);
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);

    let mut include_item_types = Vec::new();
    if let Some(value) = params.get("includeItemTypes") {
        for t in value.split(',') {
            let t = t.trim();
            if !t.is_empty() {
                include_item_types.push(t.to_string());
            }
        }
    }

    let mut items = Vec::new();

    let ids_param = params.get("ids");

    if let Some(ids_str) = ids_param {
        let requested_ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();
        let collections = state.collections.list_collections().await;

        for collection in &collections {
            // Check movies
            for movie in collection.movies.values() {
                if requested_ids.contains(&movie.id.as_str()) {
                    items.push(convert_movie_to_dto(
                        movie,
                        &collection.id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }
            }

            // Check shows
            for show in collection.shows.values() {
                if requested_ids.contains(&show.id.as_str()) {
                    items.push(convert_show_to_dto(
                        show,
                        &collection.id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }

                // Check seasons
                for season in show.seasons.values() {
                    if requested_ids.contains(&season.id.as_str()) {
                        items.push(convert_season_to_dto(
                            season,
                            &show.id,
                            &collection.id,
                            &show.name,
                            &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                        ));
                    }

                    // Check episodes
                    for episode in season.episodes.values() {
                        if requested_ids.contains(&episode.id.as_str()) {
                            items.push(convert_episode_to_dto(
                                episode,
                                &season.id,
                                &show.id,
                                &collection.id,
                                &season.name,
                                &show.name,
                                &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                            ));
                        }
                    }
                }
            }
        }
    } else if let Some(parent_id) = parent_id {
        // Get items from specific collection, series, or season

        let collections = state.collections.list_collections().await;

        // 1. Check if ParentId is a Collection
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            if include_item_types.is_empty()
                || include_item_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("Movie"))
            {
                for movie in collection.movies.values().take(limit) {
                    items.push(convert_movie_to_dto(
                        movie,
                        parent_id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }
            }

            if include_item_types.is_empty()
                || include_item_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("Series"))
            {
                for show in collection.shows.values().take(limit - items.len()) {
                    items.push(convert_show_to_dto(
                        show,
                        parent_id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }
            }
        } else {
            // 2. Check if ParentId is a Series (return Seasons)
            let mut found = false;
            for collection in &collections {
                if let Some(show) = collection.shows.get(parent_id) {
                    // Found the series, return its seasons
                    if include_item_types.is_empty()
                        || include_item_types
                            .iter()
                            .any(|t| t.eq_ignore_ascii_case("Season"))
                    {
                        // Sort seasons by index number
                        let mut seasons: Vec<_> = show.seasons.values().collect();
                        seasons.sort_by_key(|s| s.season_number);

                        for season in seasons {
                            items.push(convert_season_to_dto(
                                season,
                                &show.id,
                                &collection.id,
                                &show.name,
                                &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                            ));
                        }
                    }
                    found = true;
                    break;
                }
            }

            // 3. Check if ParentId is a Season (return Episodes)
            if !found {
                for collection in &collections {
                    for show in collection.shows.values() {
                        if let Some(season) = show.seasons.values().find(|s| s.id == *parent_id) {
                            // Found the season, return its episodes
                            if include_item_types.is_empty()
                                || include_item_types
                                    .iter()
                                    .any(|t| t.eq_ignore_ascii_case("Episode"))
                            {
                                // Sort episodes by index number
                                let mut episodes: Vec<_> = season.episodes.values().collect();
                                episodes.sort_by_key(|e| e.episode_number);

                                for episode in episodes {
                                    items.push(convert_episode_to_dto(
                                        episode,
                                        &season.id,
                                        &show.id,
                                        &collection.id,
                                        &season.name,
                                        &show.name,
                                        &state
                                            .config
                                            .jellyfin
                                            .server_id
                                            .clone()
                                            .unwrap_or_default(),
                                    ));
                                }
                            }
                            found = true;
                            break;
                        }
                    }
                    if found {
                        break;
                    }
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

            if include_item_types.is_empty()
                || include_item_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("Movie"))
            {
                for movie in collection.movies.values() {
                    if items.len() >= limit {
                        break;
                    }
                    items.push(convert_movie_to_dto(
                        movie,
                        &collection.id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }
            }

            if include_item_types.is_empty()
                || include_item_types
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case("Series"))
            {
                for show in collection.shows.values() {
                    if items.len() >= limit {
                        break;
                    }
                    items.push(convert_show_to_dto(
                        show,
                        &collection.id,
                        &state.config.jellyfin.server_id.clone().unwrap_or_default(),
                    ));
                }
            }
        }
    }

    if let Some(user_id) = get_user_id(&req) {
        for item in &mut items {
            if let Ok(user_data) = state.db.get_user_data(&user_id, &item.id).await {
                item.user_data = Some(UserData {
                    playback_position_ticks: user_data.position.unwrap_or(0),
                    played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                    play_count: user_data.playcount.unwrap_or(0),
                    is_favorite: user_data.favorite.unwrap_or(false),
                    last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
                    played: user_data.played.unwrap_or(false),
                    key: item.id.clone(),
                    unplayed_item_count: None,
                });
            }
        }
    }

    Json(QueryResult {
        total_record_count: items.len(),
        items,
    })
}

pub async fn get_user_item_by_id(
    State(state): State<AppState>,
    Path((user_id, item_id)): Path<(String, String)>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    fetch_item_by_id(&state, &item_id, Some(&user_id)).await
}

pub async fn get_item_by_id(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let user_id = get_user_id(&req);
    fetch_item_by_id(&state, &item_id, user_id.as_deref()).await
}

async fn fetch_item_by_id(
    state: &AppState,
    item_id: &str,
    user_id: Option<&str>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();

    if let Some((collection_id, item)) = state.collections.get_item(item_id) {
        let mut dto = match item {
            FoundItem::Movie(movie) => convert_movie_to_dto(&movie, &collection_id, &server_id),
            FoundItem::Show(show) => convert_show_to_dto(&show, &collection_id, &server_id),
            FoundItem::Season(season) => {
                // We need show name
                let show_name = if let Some((_, FoundItem::Show(show))) =
                    state.collections.get_item(&season.show_id)
                {
                    show.name
                } else {
                    String::new()
                };
                convert_season_to_dto(
                    &season,
                    &season.show_id,
                    &collection_id,
                    &show_name,
                    &server_id,
                )
            }
            FoundItem::Episode(episode) => {
                // We need show name and season name
                let show_name = if let Some((_, FoundItem::Show(show))) =
                    state.collections.get_item(&episode.show_id)
                {
                    show.name
                } else {
                    String::new()
                };

                let season_name = if let Some((_, FoundItem::Season(season))) =
                    state.collections.get_item(&episode.season_id)
                {
                    season.name
                } else {
                    String::new()
                };

                convert_episode_to_dto(
                    &episode,
                    &episode.season_id,
                    &episode.show_id,
                    &collection_id,
                    &season_name,
                    &show_name,
                    &server_id,
                )
            }
        };

        if let Some(uid) = user_id {
            if let Ok(user_data) = state.db.get_user_data(uid, item_id).await {
                let data = UserData {
                    playback_position_ticks: user_data.position.unwrap_or(0),
                    played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                    play_count: user_data.playcount.unwrap_or(0),
                    is_favorite: user_data.favorite.unwrap_or(false),
                    last_played_date: user_data
                        .timestamp
                        .map(|t: chrono::DateTime<chrono::Utc>| t.to_rfc3339()),
                    played: user_data.played.unwrap_or(false),
                    key: item_id.to_string(),
                    unplayed_item_count: None,
                };
                dto.user_data = Some(data);
            }
        }

        return Ok(Json(dto));
    }

    Err(StatusCode::NOT_FOUND)
}

pub async fn get_latest_items(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Json<Vec<BaseItemDto>> {
    let parent_id = params.get("parentId");
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(16);

    let mut all_items = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();

    if let Some(parent_id) = parent_id {
        // Get latest from specific collection
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            for movie in collection.movies.values() {
                all_items.push((
                    movie.date_created,
                    convert_movie_to_dto(movie, parent_id, &server_id),
                ));
            }
            for show in collection.shows.values() {
                all_items.push((
                    show.date_created,
                    convert_show_to_dto(show, parent_id, &server_id),
                ));
            }
        }
    } else {
        // Get latest from all collections
        let collections = state.collections.list_collections().await;
        for collection in collections {
            let coll_id = &collection.id;
            for movie in collection.movies.values() {
                all_items.push((
                    movie.date_created,
                    convert_movie_to_dto(movie, coll_id, &server_id),
                ));
            }
            for show in collection.shows.values() {
                all_items.push((
                    show.date_created,
                    convert_show_to_dto(show, coll_id, &server_id),
                ));
            }
        }
    }

    // Sort by date descending and take limit
    all_items.sort_by(|a, b| b.0.cmp(&a.0));
    let mut items: Vec<BaseItemDto> = all_items
        .into_iter()
        .take(limit)
        .map(|(_, dto)| dto)
        .collect();

    if let Some(user_id) = get_user_id(&req) {
        for item in &mut items {
            if let Ok(user_data) = state.db.get_user_data(&user_id, &item.id).await {
                item.user_data = Some(UserData {
                    playback_position_ticks: user_data.position.unwrap_or(0),
                    played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                    play_count: user_data.playcount.unwrap_or(0),
                    is_favorite: user_data.favorite.unwrap_or(false),
                    last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
                    played: user_data.played.unwrap_or(false),
                    key: item.id.clone(),
                    unplayed_item_count: None,
                });
            }
        }
    }

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

fn convert_to_media_source_info(
    ms: &MediaSource,
    item_id: &str,
    runtime_ticks: Option<i64>,
) -> MediaSourceInfo {
    let filename = ms
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4")
        .to_string();
    MediaSourceInfo {
        id: item_id.to_string(),
        path: filename.clone(),
        name: filename,
        source_type: "Default".to_string(),
        protocol: Some("File".to_string()),
        container: ms
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_string(),
        video_type: Some("VideoFile".to_string()),
        size: Some(ms.size as i64),
        bitrate: ms.bitrate.map(|b| b as i32),
        run_time_ticks: runtime_ticks,
        etag: Some(item_id.to_string()),
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
        direct_stream_url: Some(format!(
            "/Videos/{}/stream?mediaSourceId={}&static=true",
            item_id, item_id
        )),
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
}

pub async fn get_playback_info(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Response, StatusCode> {
    let mut sources = Vec::new();

    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            sources.extend(
                movie.media_sources.iter().map(|ms| {
                    convert_to_media_source_info(ms, &item_id, movie.runtime_ticks.clone())
                }),
            );
        }

        if sources.len() == 0 {
            for show in collection.shows.values() {
                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        if episode.id == item_id {
                            sources.extend(episode.media_sources.iter().map(|ms| {
                                convert_to_media_source_info(
                                    ms,
                                    &item_id,
                                    episode.runtime_ticks.clone(),
                                )
                            }));
                        }
                    }
                }
            }
        }

        if sources.len() > 0 {
            break;
        }
    }

    if sources.len() > 0 {
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

    Err(StatusCode::NOT_FOUND)
}

pub async fn get_similar_items(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Query(params): Query<QueryParams>,
) -> Json<QueryResult<BaseItemDto>> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    let results = state
        .collections
        .find_similar(&item_id, limit)
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

fn convert_media_sources(
    sources: &[crate::collection::MediaSource],
    item_id: &str,
) -> Option<Vec<MediaSourceInfo>> {
    if sources.is_empty() {
        return None;
    }

    Some(
        sources
            .iter()
            .map(|ms| convert_to_media_source_info(ms, item_id, None))
            .collect(),
    )
}

pub fn convert_movie_to_dto(
    movie: &crate::collection::Movie,
    parent_id: &str,
    server_id: &str,
) -> BaseItemDto {
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
        genre_items: Some(
            movie
                .genres
                .iter()
                .map(|g| NameIdPair {
                    name: g.clone(),
                    id: format!("genre_{}", g), // Deterministic ID
                })
                .collect(),
        ),
        studios: Some(
            movie
                .studios
                .iter()
                .map(|s| NameIdPair {
                    name: s.clone(),
                    id: s.clone(),
                })
                .collect(),
        ),
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
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    }
}

pub fn convert_show_to_dto(
    show: &crate::collection::Show,
    parent_id: &str,
    server_id: &str,
) -> BaseItemDto {
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
        studios: Some(
            show.studios
                .iter()
                .map(|s| NameIdPair {
                    name: s.clone(),
                    id: s.clone(),
                })
                .collect(),
        ),
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
        recursive_item_count: Some(
            show.seasons
                .iter()
                .map(|(_, s)| s.episodes.len() as i32)
                .sum(),
        ),
        official_rating: Some("TV-MA".to_string()),
        sort_name: Some(show.name.to_lowercase()),
        forced_sort_name: Some(show.name.to_lowercase()),
        original_title: Some(show.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: Some(vec![]),
        channel_id: None,
        genre_items: Some(
            show.genres
                .iter()
                .map(|g| NameIdPair {
                    name: g.clone(),
                    id: format!("genre_{}", g),
                })
                .collect(),
        ),
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    }
}

pub fn convert_season_to_dto(
    season: &crate::collection::Season,
    show_id: &str,
    _parent_id: &str,
    series_name: &str,
    server_id: &str,
) -> BaseItemDto {
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
        play_access: Some("Full".to_string()),
        enable_media_source_display: None,
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
        container: None,
        genre_items: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: None,
    }
}

pub async fn get_theme_songs(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<QueryResult<BaseItemDto>> {
    // Stub: No theme songs
    Json(QueryResult {
        items: vec![],
        total_record_count: 0,
    })
}

pub async fn get_special_features(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<QueryResult<BaseItemDto>> {
    // Stub: No special features
    Json(QueryResult {
        items: vec![],
        total_record_count: 0,
    })
}

#[derive(serde::Deserialize)]
pub struct ImageParams {
    #[serde(rename = "type")]
    image_type: Option<String>,
    tag: Option<String>,
}

pub async fn get_image(
    State(state): State<AppState>,
    Path((item_id, image_type)): Path<(String, String)>,
    Query(params): Query<ImageParams>,
    req: http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    if let Some(tag) = params.tag {
        // Jellyfin redirect tag.
        if let Some(url) = tag.strip_prefix("redirect_") {
            return Ok(Redirect::to(url).into_response().map(axum::body::Body::new));
        }

        // Jellyfin 'open local file' tag.
        if let Some(file) = tag.strip_prefix("file_") {
            // Let's only allow images, shall we.
            const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
            if let Some((_, ext)) = file.rsplit_once(".") {
                if IMAGE_EXTENSIONS.contains(&ext) {
                    let service = ServeFile::new(file);
                    let response = service
                        .oneshot(req)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Ok(response.map(axum::body::Body::new));
                }
            }
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let image_path =
        find_image_path(&state.collections, &item_id, &image_type).ok_or(StatusCode::NOT_FOUND)?;

    let quality = match params.image_type.as_deref() {
        Some("primary") | Some("logo") => state.config.jellyfin.image_quality_poster,
        _ => None,
    };

    let serve_path = state
        .image_resizer
        .resize_image(&image_path, None, None, quality)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Use ServeFile for proper ETag and Range header support
    let service = ServeFile::new(serve_path);
    let response = service
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response.map(axum::body::Body::new))
}

pub async fn get_image_indexed(
    state: State<AppState>,
    Path((item_id, image_type, _index)): Path<(String, String, String)>,
    query: Query<ImageParams>,
    req: http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    // Ignore index for now, as we don't support multiple images per type yet
    get_image(state, Path((item_id, image_type)), query, req).await
}

pub async fn get_suggestions(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Json<QueryResult<BaseItemDto>> {
    // Stub: Return latest items as suggestions for now
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(12);

    // Reuse get_latest_items logic but return QueryResult
    // We can just call the public get_latest_items if we construct the query,
    // but cleaner to just reimplement the simple fetch here.

    let mut all_items = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();

    // Get all collections
    let collections = state.collections.list_collections().await;
    for collection in collections {
        let coll_id = &collection.id;
        for movie in collection.movies.values() {
            all_items.push((
                movie.date_created,
                convert_movie_to_dto(movie, coll_id, &server_id),
            ));
        }
        for show in collection.shows.values() {
            all_items.push((
                show.date_created,
                convert_show_to_dto(show, coll_id, &server_id),
            ));
        }
    }

    // Sort by date descending
    all_items.sort_by(|a, b| b.0.cmp(&a.0));

    // Take random slice or just top keys?
    // "Suggestions" usually implies "Similar", but without a similarity engine,
    // returning *something* valid is better than 404.
    // Let's return the latest items.

    let items: Vec<BaseItemDto> = all_items
        .into_iter()
        .take(limit)
        .map(|(_, dto)| dto)
        .collect();

    Json(QueryResult {
        items,
        total_record_count: limit, // Approximation
    })
}

pub async fn get_media_segments(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<QueryResult<serde_json::Value>> {
    // Stub: No segments (Intro/Outro) known yet
    Json(QueryResult {
        items: vec![],
        total_record_count: 0,
    })
}

pub async fn search_hints(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Json<QueryResult<SearchHint>> {
    let search_term = params
        .get("SearchTerm")
        .or_else(|| params.get("searchTerm"))
        .unwrap_or("");

    let limit = params
        .get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);

    let results = state
        .collections
        .search(search_term, limit)
        .unwrap_or_default();

    let hints: Vec<SearchHint> = results
        .iter()
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

pub async fn get_resume_items(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let mut resume_items = Vec::new();

    if let Ok(db_user_data) = state
        .db
        .get_user_data_resume(&user_id, Some(limit as u32 * 2))
        .await
    {
        let collections = state.collections.list_collections().await;
        let server_id = state
            .config
            .jellyfin
            .server_id
            .as_deref()
            .unwrap_or_default();

        // Movies are easy and efficient to scan.
        for collection in &collections {
            for data in &db_user_data {
                if let Some(movie) = collection.movies.get(&data.itemid) {
                    let mut dto = convert_movie_to_dto(movie, &collection.id, server_id);
                    dto.user_data = Some(UserData {
                        playback_position_ticks: data.position.unwrap_or(0),
                        played_percentage: data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                        play_count: data.playcount.unwrap_or(0),
                        is_favorite: data.favorite.unwrap_or(false),
                        last_played_date: data.timestamp.map(|t| t.to_rfc3339()),
                        played: data.played.unwrap_or(false),
                        key: data.itemid.clone(),
                        unplayed_item_count: None,
                    });
                    resume_items.push(dto);
                }
            }

            // Shows are more complex, as we need to scan the seasons and episodes.
            for show in collection.shows.values() {
                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        for data in &db_user_data {
                            if data.itemid == episode.id {
                                let mut dto = convert_episode_to_dto(
                                    episode,
                                    &season.id,
                                    &show.id,
                                    &collection.id,
                                    &season.name,
                                    &show.name,
                                    server_id,
                                );
                                dto.user_data = Some(UserData {
                                    playback_position_ticks: data.position.unwrap_or(0),
                                    played_percentage: data
                                        .playedpercentage
                                        .map(|p| p as f64)
                                        .unwrap_or(0.0),
                                    play_count: data.playcount.unwrap_or(0),
                                    is_favorite: data.favorite.unwrap_or(false),
                                    last_played_date: data.timestamp.map(|t| t.to_rfc3339()),
                                    played: data.played.unwrap_or(false),
                                    key: episode.id.clone(),
                                    unplayed_item_count: None,
                                });
                                resume_items.push(dto);
                            }
                        }
                    }
                }
            }
        }
    }

    let items: Vec<BaseItemDto> = resume_items.into_iter().take(limit).collect();
    let count = items.len();

    Ok(Json(QueryResult {
        items,
        total_record_count: count,
    }))
}
