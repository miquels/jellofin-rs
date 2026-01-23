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
                if let Some(pos) = user_data.playback_position_ticks {
                    if pos > 0 && !user_data.played {
                        let mut dto = convert_movie_to_dto(movie, &collection.id);
                        dto.user_data = Some(UserItemData {
                            played: user_data.played,
                            is_favorite: user_data.is_favorite,
                            playback_position_ticks: Some(pos),
                            play_count: user_data.play_count,
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
                        if let Some(pos) = user_data.playback_position_ticks {
                            if pos > 0 && !user_data.played {
                                let mut dto = convert_episode_to_dto(episode, &season.id, &show.id, &collection.id);
                                dto.user_data = Some(UserItemData {
                                    played: user_data.played,
                                    is_favorite: user_data.is_favorite,
                                    playback_position_ticks: Some(pos),
                                    play_count: user_data.play_count,
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
) -> Result<Json<UserItemData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            user_id: user_id.clone(),
            item_id: item_id.clone(),
            played: false,
            is_favorite: false,
            playback_position_ticks: None,
            play_count: None,
        });
    
    user_data.played = true;
    user_data.play_count = Some(user_data.play_count.unwrap_or(0) + 1);
    user_data.playback_position_ticks = None;
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserItemData {
        played: user_data.played,
        is_favorite: user_data.is_favorite,
        playback_position_ticks: user_data.playback_position_ticks,
        play_count: user_data.play_count,
    }))
}

pub async fn mark_unplayed(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserItemData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            user_id: user_id.clone(),
            item_id: item_id.clone(),
            played: false,
            is_favorite: false,
            playback_position_ticks: None,
            play_count: None,
        });
    
    user_data.played = false;
    user_data.playback_position_ticks = None;
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserItemData {
        played: user_data.played,
        is_favorite: user_data.is_favorite,
        playback_position_ticks: user_data.playback_position_ticks,
        play_count: user_data.play_count,
    }))
}

pub async fn mark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserItemData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            user_id: user_id.clone(),
            item_id: item_id.clone(),
            played: false,
            is_favorite: false,
            playback_position_ticks: None,
            play_count: None,
        });
    
    user_data.is_favorite = true;
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserItemData {
        played: user_data.played,
        is_favorite: user_data.is_favorite,
        playback_position_ticks: user_data.playback_position_ticks,
        play_count: user_data.play_count,
    }))
}

pub async fn unmark_favorite(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<UserItemData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let mut user_data = state.db.get_user_data(&user_id, &item_id).await
        .unwrap_or_else(|_| crate::db::UserData {
            user_id: user_id.clone(),
            item_id: item_id.clone(),
            played: false,
            is_favorite: false,
            playback_position_ticks: None,
            play_count: None,
        });
    
    user_data.is_favorite = false;
    
    state.db.upsert_user_data(&user_data).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(UserItemData {
        played: user_data.played,
        is_favorite: user_data.is_favorite,
        playback_position_ticks: user_data.playback_position_ticks,
        play_count: user_data.play_count,
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
            user_id: user_id.clone(),
            item_id: item_id.clone(),
            played: false,
            is_favorite: false,
            playback_position_ticks: None,
            play_count: None,
        });
    
    user_data.playback_position_ticks = position_ticks;
    
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
                        if user_data.played {
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
                        let dto = convert_episode_to_dto(next_episode, &season.id, &show.id, &collection.id);
                        next_up_items.push(dto);
                    } else {
                        let next_season_num = last_watched_season + 1;
                        if let Some(next_season) = show.seasons.get(&next_season_num) {
                            if let Some(first_episode) = next_season.episodes.values().min_by_key(|e| e.episode_number) {
                                let dto = convert_episode_to_dto(first_episode, &next_season.id, &show.id, &collection.id);
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
