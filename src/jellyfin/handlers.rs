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

pub async fn get_users(State(_state): State<AppState>) -> Result<Json<Vec<UserDto>>, StatusCode> {
    let users: Vec<crate::db::User> = vec![];
    
    let user_dtos: Vec<UserDto> = users
        .into_iter()
        .map(|u| UserDto {
            name: u.username,
            id: u.id,
            has_password: false,
            has_configured_password: false,
            has_configured_easy_password: false,
            policy: UserPolicy {
                is_administrator: false,
                is_disabled: false,
                enable_all_folders: true,
            },
        })
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
    
    Ok(Json(UserDto {
        name: user.username,
        id: user.id,
        has_password: false,
        has_configured_password: false,
        has_configured_easy_password: false,
        policy: UserPolicy {
            is_administrator: false,
            is_disabled: false,
            enable_all_folders: true,
        },
    }))
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
                people: None,
                parent_id: None,
                series_id: None,
                season_id: None,
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
                user_data: None,
                media_sources: None,
                provider_ids: None,
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
        people: None,
        parent_id: None,
        series_id: None,
        season_id: None,
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
        user_data: None,
        media_sources: None,
        provider_ids: None,
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
        people: None,
        parent_id: None,
        series_id: None,
        season_id: None,
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
        user_data: None,
        media_sources: None,
        provider_ids: None,
    });
    
    Json(QueryResult {
        items,
        total_record_count: collections.len(),
    })
}

pub async fn get_items(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<QueryResult<BaseItemDto>> {
    let parent_id = params.get("ParentId").or_else(|| params.get("parentId"));
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100);
    
    let mut items = Vec::new();
    
    if let Some(parent_id) = parent_id {
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            for movie in collection.movies.values().take(limit) {
                items.push(convert_movie_to_dto(movie, parent_id));
            }
            
            for show in collection.shows.values().take(limit) {
                items.push(convert_show_to_dto(show, parent_id));
            }
        }
    }
    
    Json(QueryResult {
        total_record_count: items.len(),
        items,
    })
}

pub async fn get_item_by_id(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            return Ok(Json(convert_movie_to_dto(movie, &collection.id)));
        }
        
        if let Some(show) = collection.shows.get(&item_id) {
            return Ok(Json(convert_show_to_dto(show, &collection.id)));
        }
        
        for show in collection.shows.values() {
            if let Some(season) = show.seasons.get(&item_id.parse::<i32>().unwrap_or(-1)) {
                return Ok(Json(convert_season_to_dto(season, &show.id, &collection.id)));
            }
            
            for season in show.seasons.values() {
                if let Some(episode) = season.episodes.get(&item_id.parse::<i32>().unwrap_or(-1)) {
                    return Ok(Json(convert_episode_to_dto(episode, &season.id, &show.id, &collection.id)));
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
    
    if let Some(parent_id) = parent_id {
        // Get latest from specific collection
        if let Some(collection) = state.collections.get_collection(parent_id).await {
            for movie in collection.movies.values() {
                all_items.push((movie.date_created, convert_movie_to_dto(movie, parent_id)));
            }
            for show in collection.shows.values() {
                all_items.push((show.date_created, convert_show_to_dto(show, parent_id)));
            }
        }
    } else {
        // Get latest from all collections
        let collections = state.collections.list_collections().await;
        for collection in collections {
            let coll_id = &collection.id;
            for movie in collection.movies.values() {
                all_items.push((movie.date_created, convert_movie_to_dto(movie, coll_id)));
            }
            for show in collection.shows.values() {
                all_items.push((show.date_created, convert_show_to_dto(show, coll_id)));
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
) -> Result<Json<PlaybackInfoResponse>, StatusCode> {
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(&item_id) {
            let sources = movie.media_sources.iter()
                .map(|ms| MediaSourceInfo {
                    id: item_id.clone(),
                    path: ms.path.to_string_lossy().to_string(),
                    container: ms.path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("mkv")
                        .to_string(),
                    size: Some(ms.size as i64),
                    supports_direct_stream: true,
                    supports_transcoding: false,
                    media_streams: Some(vec![]),
                })
                .collect();
            
            return Ok(Json(PlaybackInfoResponse { media_sources: sources }));
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if episode.id == item_id {
                        let sources = episode.media_sources.iter()
                            .map(|ms| MediaSourceInfo {
                                id: item_id.clone(),
                                path: ms.path.to_string_lossy().to_string(),
                                container: ms.path.extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("mkv")
                                    .to_string(),
                                size: Some(ms.size as i64),
                                supports_direct_stream: true,
                                supports_transcoding: false,
                                media_streams: Some(vec![]),
                            })
                            .collect();
                        
                        return Ok(Json(PlaybackInfoResponse { media_sources: sources }));
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
                    [(header::CONTENT_TYPE, "video/x-matroska")],
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
                                [(header::CONTENT_TYPE, "video/x-matroska")],
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
    
    for r in &results {
        for collection in &collections {
            if let Some(movie) = collection.movies.get(&r.id) {
                items.push(convert_movie_to_dto(movie, &collection.id));
                break;
            }
            if let Some(show) = collection.shows.get(&r.id) {
                items.push(convert_show_to_dto(show, &collection.id));
                break;
            }
        }
    }
    
    Json(QueryResult {
        total_record_count: items.len(),
        items,
    })
}

pub fn convert_movie_to_dto(movie: &crate::collection::Movie, parent_id: &str) -> BaseItemDto {
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
        studios: Some(movie.studios.iter().map(|s| NameIdPair {
            name: s.clone(),
            id: s.clone(),
        }).collect()),
        people: Some(movie.people.iter().map(|p| BaseItemPerson {
            name: p.name.clone(),
            id: p.name.clone(),
            person_type: format!("{:?}", p.person_type),
            role: p.role.clone(),
        }).collect()),
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        season_id: None,
        index_number: None,
        parent_index_number: None,
        child_count: None,
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: Some("VideoFile".to_string()),
        width: None,
        height: None,
        user_data: None,
        media_sources: None,
        provider_ids: Some(provider_ids),
    }
}

pub fn convert_show_to_dto(show: &crate::collection::Show, parent_id: &str) -> BaseItemDto {
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
        people: Some(show.people.iter().map(|p| BaseItemPerson {
            name: p.name.clone(),
            id: p.name.clone(),
            person_type: format!("{:?}", p.person_type),
            role: p.role.clone(),
        }).collect()),
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        season_id: None,
        index_number: None,
        parent_index_number: None,
        child_count: Some(show.seasons.len() as i32),
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: None,
        width: None,
        height: None,
        user_data: None,
        media_sources: None,
        provider_ids: Some(provider_ids),
    }
}

pub fn convert_season_to_dto(season: &crate::collection::Season, show_id: &str, parent_id: &str) -> BaseItemDto {
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
        people: None,
        parent_id: Some(parent_id.to_string()),
        series_id: Some(show_id.to_string()),
        season_id: None,
        index_number: Some(season.season_number),
        parent_index_number: None,
        child_count: Some(season.episodes.len() as i32),
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: None,
        width: None,
        height: None,
        user_data: None,
        media_sources: None,
        provider_ids: None,
    }
}

pub fn convert_episode_to_dto(
    episode: &crate::collection::Episode,
    season_id: &str,
    show_id: &str,
    parent_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if episode.images.primary.is_some() {
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
        people: None,
        parent_id: Some(parent_id.to_string()),
        series_id: Some(show_id.to_string()),
        season_id: Some(season_id.to_string()),
        index_number: Some(episode.episode_number),
        parent_index_number: Some(episode.season_number),
        child_count: None,
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: None,
        container: None,
        video_type: Some("VideoFile".to_string()),
        width: None,
        height: None,
        user_data: None,
        media_sources: None,
        provider_ids: None,
    }
}
