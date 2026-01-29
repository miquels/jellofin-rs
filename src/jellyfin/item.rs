use axum::{
    extract::{Path, Query, State},
    http::{self, Request, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use super::auth::get_user_id;
use super::filter::apply_items_filter;
use super::jfitem::{
    convert_episode_to_dto, convert_movie_to_dto, convert_season_to_dto, convert_show_to_dto,
    convert_to_media_source_info,
};
use super::pagination::apply_pagination;
use super::sort::apply_item_sorting;
use super::types::*;
use crate::collection::collection::ItemRef;
use crate::collection::find_image_path;
use crate::collection::repo::FoundItem;
use crate::db::UserDataRepo;
use crate::server::AppState;
use crate::util::QueryParams;

pub async fn get_item_ancestors(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Json<Vec<BaseItemDto>> {
    let mut ancestors = Vec::new();
    let server_id = state
        .config
        .jellyfin
        .server_id
        .as_deref()
        .unwrap_or_default();

    // 1. Find item and its collection
    let (collection_id, found_item) = match state.collections.get_item(&item_id) {
        Some(res) => res,
        None => return Json(ancestors),
    };

    // 2. Get the collection for parent lookups
    let collection = match state.collections.get_collection(&collection_id).await {
        Some(c) => c,
        None => return Json(ancestors),
    };

    // 3. Build ancestor chain based on item type
    match found_item {
        FoundItem::Episode(episode) => {
            // Episode -> Season -> Series -> Collection
            // Get Season
            if let Some(ItemRef::Season(season)) = collection.get_item(&episode.season_id) {
                // Get Show (needed for season DTO conversion)
                if let Some(ItemRef::Show(show)) = collection.get_item(&episode.show_id) {
                    ancestors.push(convert_season_to_dto(
                        season,
                        &show.id,
                        &collection.id,
                        &show.name,
                        server_id,
                    ));
                    ancestors.push(convert_show_to_dto(show, &collection.id, server_id));
                }
            }
        }
        FoundItem::Season(season) => {
            // Season -> Series -> Collection
            if let Some(ItemRef::Show(show)) = collection.get_item(&season.show_id) {
                ancestors.push(convert_show_to_dto(show, &collection.id, server_id));
            }
        }
        FoundItem::Show(_show) => {
            // Series -> Collection
            // Nothing extra to add before collection
        }
        FoundItem::Movie(_movie) => {
            // Movie -> Collection
            // Nothing extra to add before collection
        }
    }

    // Always add Collection as the root ancestor
    ancestors.push(BaseItemDto {
        id: collection.id.clone(),
        name: collection.name.clone(),
        item_type: "CollectionFolder".to_string(),
        server_id: Some(server_id.to_string()),
        ..Default::default()
    });

    Json(ancestors)
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

    // Apply filtering
    items = apply_items_filter(items, &params);

    // Store total count before pagination
    let total_count = items.len();

    // Apply sorting
    items = apply_item_sorting(items, &params);

    // Apply pagination
    let (items, start_index) = apply_pagination(items, &params);

    Json(QueryResult {
        total_record_count: total_count,
        start_index,
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
                    movie.premiere_date,
                    convert_movie_to_dto(movie, parent_id, &server_id),
                ));
            }
            for show in collection.shows.values() {
                all_items.push((
                    show.premiere_date,
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
                    movie.premiere_date,
                    convert_movie_to_dto(movie, coll_id, &server_id),
                ));
            }
            for show in collection.shows.values() {
                all_items.push((
                    show.premiere_date,
                    convert_show_to_dto(show, coll_id, &server_id),
                ));
            }
        }
    }

    // Sort by premiere date descending (most recent releases first)
    all_items.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply filters before taking limit
    let mut items: Vec<BaseItemDto> = all_items.into_iter().map(|(_, dto)| dto).collect();

    items = apply_items_filter(items, &params);

    // Take limit after filtering
    items.truncate(limit);

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
        start_index: 0,
        items,
    })
}

pub async fn get_theme_songs(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<QueryResult<BaseItemDto>> {
    // Stub: No theme songs
    Json(QueryResult {
        items: vec![],
        total_record_count: 0,
        start_index: 0,
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
        start_index: 0,
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
        start_index: 0,
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
        start_index: 0,
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
            name: r.name.clone(),
            item_type: r.item_type.clone(),
            production_year: None,
        })
        .collect();

    let count = hints.len();
    Json(QueryResult {
        items: hints,
        total_record_count: count,
        start_index: 0,
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
        start_index: 0,
    }))
}
