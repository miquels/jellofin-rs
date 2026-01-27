use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    Json,
};

use crate::db::UserDataRepo;
use crate::server::AppState;
use crate::util::QueryParams;
use super::auth::get_user_id;
use super::items::{convert_movie_to_dto, convert_episode_to_dto};
use super::types::*;

pub async fn get_resume_items(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let limit = params.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    
    let mut resume_items = Vec::new();
    
    if let Ok(db_user_data) = state.db.get_user_data_resume(&user_id, Some(limit as u32 * 2)).await {
        let collections = state.collections.list_collections().await;
        let server_id = state.config.jellyfin.server_id.as_deref().unwrap_or_default();

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
                                let mut dto = convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, server_id);
                                    dto.user_data = Some(UserData {
                                        playback_position_ticks: data.position.unwrap_or(0),
                                        played_percentage: data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
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

async fn toggle_played(
    state: AppState,
    item_id: String,
    req: Request<axum::body::Body>,
    is_played: bool,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));
    
    user_data.played = Some(is_played);
    user_data.playcount = Some(user_data.playcount.unwrap_or(0) + is_played as i32);
    user_data.position = None;
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserData {
        playback_position_ticks: 0,
        played_percentage: 100.0,
        play_count: user_data.playcount.unwrap_or(0),
        is_favorite: user_data.favorite.unwrap_or(false),
        last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
        played: true,
        key: item_id,
        unplayed_item_count: None,
    }))
}

pub async fn mark_played(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    toggle_played(state, item_id, req, true).await
}

pub async fn mark_unplayed(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    toggle_played(state, item_id, req, false).await
}

pub async fn toggle_favorite(
    state: AppState,
    item_id: String,
    req: Request<axum::body::Body>,
    is_favorite: bool,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));
    
    user_data.favorite = Some(is_favorite);
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserData {
        playback_position_ticks: user_data.position.unwrap_or(0),
        played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
        play_count: user_data.playcount.unwrap_or(0),
        is_favorite: user_data.favorite.unwrap_or(false),
        last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
        played: user_data.played.unwrap_or(false),
        key: item_id,
        unplayed_item_count: None,
    }))
}

pub async fn mark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
        toggle_favorite(state, item_id, req, true).await
}

pub async fn unmark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
        toggle_favorite(state, item_id, req, false).await
}

pub async fn update_playback_position(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let position_ticks = params.get("positionTicks")
        .and_then(|s| s.parse::<i64>().ok());
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));
    
    user_data.position = position_ticks;
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

/// Request body for /Sessions/Playing/Progress
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayingProgressRequest {
    pub item_id: String,
    #[serde(default)]
    pub position_ticks: i64,
    #[allow(dead_code)]
    pub media_source_id: Option<String>,
    #[allow(dead_code)]
    pub audio_stream_index: Option<i32>,
    #[allow(dead_code)]
    pub subtitle_stream_index: Option<i32>,
    #[allow(dead_code)]
    pub play_session_id: Option<String>,
}

/// POST /Sessions/Playing/Progress
/// Updates playback progress for an item
pub async fn session_playing_progress(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Parse JSON body
    let (_parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, 1024 * 1024)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let progress: PlayingProgressRequest = serde_json::from_slice(&bytes)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &progress.item_id).await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &progress.item_id));
    
    user_data.position = Some(progress.position_ticks);
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

// Helper to determine the Next Up item for a specific show
async fn find_next_up_for_show(
    state: &AppState,
    user_id: &str,
    show: &crate::collection::Show,
    collection: &crate::collection::Collection,
    server_id: &str,
    force_first_if_unwatched: bool,
) -> Option<(chrono::DateTime<chrono::Utc>, BaseItemDto)> {
    let mut last_watched_season = 0;
    let mut last_watched_episode = 0;
    let mut found_watched = false;
    let mut last_played_date = chrono::DateTime::<chrono::Utc>::MIN_UTC;
    
    // 1. Find the highest watched episode index
    for season in show.seasons.values() {
        for episode in season.episodes.values() {
            if let Ok(user_data) = state.db.get_user_data(user_id, &episode.id).await {
                if user_data.played == Some(true) {
                    if episode.season_number > last_watched_season || 
                       (episode.season_number == last_watched_season && episode.episode_number > last_watched_episode) {
                        last_watched_season = episode.season_number;
                        last_watched_episode = episode.episode_number;
                        found_watched = true;
                        if let Some(ts) = user_data.timestamp {
                            if ts > last_played_date {
                                last_played_date = ts;
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 2. Identify the candidate (Next Episode)
    if found_watched {
        if let Some(season) = show.seasons.get(&last_watched_season) {
            let next_episode_num = last_watched_episode + 1;
            
            if let Some(next_episode) = season.episodes.get(&next_episode_num) {
                if let Ok(data) = state.db.get_user_data(user_id, &next_episode.id).await {
                    if let Some(pos) = data.position {
                        if pos > 0 && data.played != Some(true) {
                            return None;
                        }
                    }
                }
                let dto = convert_episode_to_dto(next_episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id);
                return Some((last_played_date, dto));
            } else {
                // Try next season
                let next_season_num = last_watched_season + 1;
                if let Some(next_season) = show.seasons.get(&next_season_num) {
                     // Get first episode of next season
                     let mut episodes: Vec<_> = next_season.episodes.values().collect();
                     episodes.sort_by_key(|e| e.episode_number);
                     if let Some(first_episode) = episodes.first() {
                         if let Ok(data) = state.db.get_user_data(user_id, &first_episode.id).await {
                             if let Some(pos) = data.position {
                                 if pos > 0 && data.played != Some(true) {
                                     return None;
                                 }
                             }
                         }
                         let dto = convert_episode_to_dto(first_episode, &next_season.id, &show.id, &collection.id, &next_season.name, &show.name, &server_id);
                         return Some((last_played_date, dto));
                     }
                }
            }
        }
    } else if force_first_if_unwatched {
        // Find lowest season
        let mut seasons: Vec<_> = show.seasons.values().collect();
        seasons.sort_by_key(|s| s.season_number);
        
        if let Some(first_season) = seasons.first() {
            let mut episodes: Vec<_> = first_season.episodes.values().collect();
            episodes.sort_by_key(|e| e.episode_number);
            
            if let Some(first_episode) = episodes.first() {
                // Check if the very first episode is in progress? Usually yes, apply same rule.
                if let Ok(data) = state.db.get_user_data(user_id, &first_episode.id).await {
                    if let Some(pos) = data.position {
                        if pos > 0 && data.played != Some(true) {
                            return None;
                        }
                    }
                }
                
                let dto = convert_episode_to_dto(first_episode, &first_season.id, &show.id, &collection.id, &first_season.name, &show.name, &server_id);
                return Some((chrono::Utc::now(), dto));
            }
        }
    }
    
    None
}

pub async fn get_next_up(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let limit = params.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
        
    let series_id = params.get("seriesId");
    
    let mut next_up_items = Vec::new();
    let server_id = state.config.jellyfin.server_id.as_deref().unwrap_or_default();
    
    if let Some(sid) = series_id {
        // Direct lookup
        if let Some((collection_id, item)) = state.collections.get_item(sid) {
            if let crate::collection::repo::FoundItem::Show(show) = item {
                if let Some(collection) = state.collections.get_collection(&collection_id).await {
                     if let Some((_, dto)) = find_next_up_for_show(&state, &user_id, &show, &collection, server_id, true).await {
                         next_up_items.push(dto);
                     }
                }
            }
        }
    } else {
        // Scan all shows
        let collections = state.collections.list_collections().await;
        let mut potential_items = Vec::new();
        
        for collection in &collections {
            for show in collection.shows.values() {
                 if let Some((date, dto)) = find_next_up_for_show(&state, &user_id, show, collection, &server_id, false).await {
                     potential_items.push((date, dto));
                 }
            }
        }
        
        // Sort by last played date descending
        potential_items.sort_by(|a, b| b.0.cmp(&a.0));
        next_up_items = potential_items.into_iter().map(|(_, dto)| dto).collect();
    }
    
    let items: Vec<BaseItemDto> = next_up_items.into_iter().take(limit).collect();
    let count = items.len();
    
    Ok(Json(QueryResult {
        items,
        total_record_count: count,
    }))
}

pub(crate) fn get_default_db_user_data(user_id: &str,item_id: &str) -> crate::db::UserData {
    crate::db::UserData {
        userid: user_id.to_string(),
        itemid: item_id.to_string(),
        position: None,
        playedpercentage: None,
        played: None,
        playcount: None,
        favorite: None,
        timestamp: Some(chrono::Utc::now()),
    }
}

pub(crate) fn get_default_user_data(item_id: &str) -> UserData {
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

pub async fn get_item_user_data(
    State(_state): State<AppState>,
    Path(item_id): Path<String>,
) -> Json<UserData> {
    // Return default user data for now
    Json(get_default_user_data(&item_id))
}
