use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    Json,
};
use std::collections::HashMap;

use crate::db::UserDataRepo;
use crate::server::AppState;
use super::auth::get_user_id;
use super::handlers::{convert_movie_to_dto, convert_episode_to_dto};
use super::types::*;

pub async fn get_resume_items(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    
    let mut resume_items = Vec::new();
    let collections = state.collections.list_collections().await;
    
    for collection in &collections {
        for movie in collection.movies.values() {
            if let Ok(user_data) = state.db.get_user_data(&user_id, &movie.id).await {
                if let Some(pos) = user_data.position {
                    if pos > 0 && user_data.played != Some(true) {
                        let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
                        let mut dto = convert_movie_to_dto(movie, &collection.id, &server_id);
                        dto.user_data = Some(UserData {
                            playback_position_ticks: pos,
                            played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                            play_count: user_data.playcount.unwrap_or(0),
                            is_favorite: user_data.favorite.unwrap_or(false),
                            last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
                            played: user_data.played.unwrap_or(false),
                            key: movie.id.clone(),
                            unplayed_item_count: None,
                        });
                        resume_items.push((pos, dto));
                    }
                }
            }
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if let Ok(user_data) = state.db.get_user_data(&user_id, &episode.id).await {
                        if let Some(pos) = user_data.position {
                            if pos > 0 && user_data.played != Some(true) {
                                let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
                                let mut dto = convert_episode_to_dto(episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id);
                                dto.user_data = Some(UserData {
                                    playback_position_ticks: pos,
                                    played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
                                    play_count: user_data.playcount.unwrap_or(0),
                                    is_favorite: user_data.favorite.unwrap_or(false),
                                    last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
                                    played: user_data.played.unwrap_or(false),
                                    key: episode.id.clone(),
                                    unplayed_item_count: None,
                                });
                                resume_items.push((pos, dto));
                            }
                        }
                    }
                }
            }
        }
    }
    
    resume_items.sort_by(|a, b| b.0.cmp(&a.0));
    let items: Vec<BaseItemDto> = resume_items.into_iter().take(limit).map(|(_, dto)| dto).collect();
    let count = items.len();
    
    Ok(Json(QueryResult {
        items,
        total_record_count: count,
    }))
}

pub async fn mark_played(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
    user_data.played = Some(true);
    user_data.playcount = Some(user_data.playcount.unwrap_or(0) + 1);
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

pub async fn mark_unplayed(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
    user_data.played = Some(false);
    user_data.position = None;
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserData {
        playback_position_ticks: 0,
        played_percentage: 0.0,
        play_count: user_data.playcount.unwrap_or(0),
        is_favorite: user_data.favorite.unwrap_or(false),
        last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
        played: false,
        key: item_id,
        unplayed_item_count: None,
    }))
}

pub async fn mark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
    user_data.favorite = Some(true);
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserData {
        playback_position_ticks: user_data.position.unwrap_or(0),
        played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
        play_count: user_data.playcount.unwrap_or(0),
        is_favorite: true,
        last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
        played: user_data.played.unwrap_or(false),
        key: item_id,
        unplayed_item_count: None,
    }))
}

pub async fn unmark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
    user_data.favorite = Some(false);
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserData {
        playback_position_ticks: user_data.position.unwrap_or(0),
        played_percentage: user_data.playedpercentage.map(|p| p as f64).unwrap_or(0.0),
        play_count: user_data.playcount.unwrap_or(0),
        is_favorite: false,
        last_played_date: user_data.timestamp.map(|t| t.to_rfc3339()),
        played: user_data.played.unwrap_or(false),
        key: item_id,
        unplayed_item_count: None,
    }))
}

pub async fn update_playback_position(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let position_ticks = params.get("PositionTicks")
        .or_else(|| params.get("positionTicks"))
        .and_then(|s| s.parse::<i64>().ok());
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
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
        .unwrap_or_else(|_| crate::db::UserData {
            userid: user_id.clone(),
            itemid: progress.item_id.clone(),
            position: None,
            playedpercentage: None,
            played: None,
            playcount: None,
            favorite: None,
            timestamp: Some(chrono::Utc::now()),
        });
    
    user_data.position = Some(progress.position_ticks);
    user_data.timestamp = Some(chrono::Utc::now());
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_next_up(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    
    let mut next_up_items = Vec::new();
    let collections = state.collections.list_collections().await;
    
    for collection in &collections {
        for show in collection.shows.values() {
            let mut last_watched_season = 0;
            let mut last_watched_episode = 0;
            let mut found_watched = false;
            
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if let Ok(user_data) = state.db.get_user_data(&user_id, &episode.id).await {
                        if user_data.played == Some(true) {
                            if episode.season_number > last_watched_season || 
                               (episode.season_number == last_watched_season && episode.episode_number > last_watched_episode) {
                                last_watched_season = episode.season_number;
                                last_watched_episode = episode.episode_number;
                                found_watched = true;
                            }
                        }
                    }
                }
            }
            
            if found_watched {
                if let Some(season) = show.seasons.get(&last_watched_season) {
                    let next_episode_num = last_watched_episode + 1;
                    if let Some(next_episode) = season.episodes.get(&next_episode_num) {
                        let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
                        let dto = convert_episode_to_dto(next_episode, &season.id, &show.id, &collection.id, &season.name, &show.name, &server_id);
                        next_up_items.push(dto);
                    } else {
                        let next_season_num = last_watched_season + 1;
                        if let Some(next_season) = show.seasons.get(&next_season_num) {
                            if let Some(first_episode) = next_season.episodes.values().min_by_key(|e| e.episode_number) {
                                let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
                                let dto = convert_episode_to_dto(first_episode, &next_season.id, &show.id, &collection.id, &next_season.name, &show.name, &server_id);
                                next_up_items.push(dto);
                            }
                        }
                    }
                }
            }
        }
    }
    
    let items: Vec<BaseItemDto> = next_up_items.into_iter().take(limit).collect();
    let count = items.len();
    
    Ok(Json(QueryResult {
        items,
        total_record_count: count,
    }))
}
