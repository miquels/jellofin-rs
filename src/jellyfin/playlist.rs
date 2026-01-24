use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::{PlaylistRepo, Playlist as DbPlaylist};
use crate::server::AppState;
use super::auth::get_user_id;
use super::handlers::convert_movie_to_dto;
use super::types::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaylistRequest {
    #[serde(rename = "Name")]
    pub name: Option<String>,
    #[serde(rename = "UserId")]
    pub user_id: Option<String>,
    #[serde(rename = "Ids")]
    pub ids: Option<Vec<String>>,
    #[serde(rename = "IsPublic")]
    pub is_public: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePlaylistResponse {
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPlaylistResponse {
    #[serde(rename = "OpenAccess")]
    pub open_access: bool,
    #[serde(rename = "Shares")]
    pub shares: Vec<String>,
    #[serde(rename = "ItemIds")]
    pub item_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlaylistAccess {
    #[serde(rename = "Users")]
    pub users: Vec<String>,
    #[serde(rename = "CanEdit")]
    pub can_edit: bool,
}

pub async fn create_playlist(
    State(state): State<AppState>,
    req: Request<axum::body::Body>,
) -> Result<Json<CreatePlaylistResponse>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    // Parse JSON body
    let (_parts, body) = req.into_parts();
    let bytes = axum::body::to_bytes(body, usize::MAX).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let create_req: CreatePlaylistRequest = serde_json::from_slice(&bytes)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let name = create_req.name.ok_or(StatusCode::BAD_REQUEST)?;
    let playlist_user_id = create_req.user_id.unwrap_or(user_id.clone());
    let item_ids = create_req.ids.unwrap_or_default();
    
    let playlist_id = generate_playlist_id(&name);
    
    let playlist = DbPlaylist {
        id: playlist_id.clone(),
        userid: playlist_user_id,
        name,
        timestamp: Some(Utc::now()),
    };
    
    state.db.create_playlist(&playlist).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    for item_id in &item_ids {
        let _ = state.db.add_item_to_playlist(&playlist_id, item_id).await;
    }
    
    Ok(Json(CreatePlaylistResponse {
        id: playlist_id,
    }))
}

pub async fn get_playlist(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<GetPlaylistResponse>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let playlist = state.db.get_playlist(&playlist_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    if playlist.userid != user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let item_ids = state.db.get_playlist_items(&playlist_id).await
        .unwrap_or_default();
    
    Ok(Json(GetPlaylistResponse {
        open_access: false,
        shares: vec![],
        item_ids,
    }))
}

pub async fn get_playlist_items(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let playlist = state.db.get_playlist(&playlist_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    if playlist.userid != user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let item_ids = state.db.get_playlist_items(&playlist_id).await
        .unwrap_or_default();
    
    let mut items = Vec::new();
    
    for item_id in &item_ids {
        for collection in state.collections.list_collections().await {
            if let Some(movie) = collection.movies.get(item_id) {
                items.push(convert_movie_to_dto(movie, &collection.id));
                break;
            }
            
            for show in collection.shows.values() {
                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        if &episode.id == item_id {
                            items.push(super::handlers::convert_episode_to_dto(
                                episode,
                                &season.id,
                                &show.id,
                                &collection.id,
                                &season.name,
                                &show.name,
                            ));
                            break;
                        }
                    }
                }
            }
        }
    }
    
    let count = items.len();
    
    Ok(Json(QueryResult {
        items,
        total_record_count: count,
    }))
}

pub async fn add_playlist_items(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let playlist = state.db.get_playlist(&playlist_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    if playlist.userid != user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let item_ids = params.get("Ids")
        .map(|ids| {
            ids.split(',')
                .map(|id| id.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    for item_id in item_ids {
        let _ = state.db.add_item_to_playlist(&playlist_id, &item_id).await;
    }
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_playlist_items(
    State(state): State<AppState>,
    Path(playlist_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    req: Request<axum::body::Body>,
) -> Result<StatusCode, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    let playlist = state.db.get_playlist(&playlist_id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    if playlist.userid != user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    
    let item_ids = params.get("Ids")
        .map(|ids| {
            ids.split(',')
                .map(|id| id.trim().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    
    for item_id in item_ids {
        let _ = state.db.remove_item_from_playlist(&playlist_id, &item_id).await;
    }
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_playlist_users(
    State(_state): State<AppState>,
    Path(_playlist_id): Path<String>,
    req: Request<axum::body::Body>,
) -> Result<Json<Vec<PlaylistAccess>>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    Ok(Json(vec![PlaylistAccess {
        users: vec![user_id],
        can_edit: true,
    }]))
}

pub async fn get_playlist_user(
    State(_state): State<AppState>,
    Path((_playlist_id, _user_id)): Path<(String, String)>,
    req: Request<axum::body::Body>,
) -> Result<Json<PlaylistAccess>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    
    Ok(Json(PlaylistAccess {
        users: vec![user_id],
        can_edit: true,
    }))
}

fn generate_playlist_id(name: &str) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(format!("playlist:{}", name).as_bytes());
    let hash = hasher.finalize();
    
    let mut num = [0u8; 16];
    num.copy_from_slice(&hash[..16]);
    
    let mut value = u128::from_be_bytes(num);
    value >>= 9;
    
    let mut id = String::with_capacity(20);
    for _ in 0..20 {
        let remainder = (value % 62) as u8;
        value /= 62;
        
        let c = if remainder < 10 {
            (remainder + 48) as char
        } else if remainder < 36 {
            (remainder + 65 - 10) as char
        } else {
            (remainder + 97 - 36) as char
        };
        id.push(c);
    }
    
    id
}
