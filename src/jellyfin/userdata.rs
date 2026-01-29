use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    Json,
};

use super::auth::get_user_id;
use super::jfitem::{convert_episode_to_dto, convert_movie_to_dto};
use super::types::*;
use crate::collection::collection::ItemRef;
use crate::collection::repo::FoundItem;
use crate::db::UserDataRepo;
use crate::server::AppState;
use crate::util::QueryParams;

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
        let server_id = state
            .config
            .jellyfin
            .server_id
            .as_deref()
            .unwrap_or_default();

        // Efficiently finding items by ID using the global lookup map
        for data in &db_user_data {
            if let Some((collection_id, found_item)) = state.collections.get_item(&data.itemid) {
                if let Some(collection) = state.collections.get_collection(&collection_id).await {
                    match found_item {
                        FoundItem::Movie(movie) => {
                            let mut dto = convert_movie_to_dto(&movie, &collection.id, server_id);
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
                                key: data.itemid.clone(),
                                unplayed_item_count: None,
                            });
                            resume_items.push(dto);
                        }
                        FoundItem::Episode(episode) => {
                            // Need to find parent season and show for full DTO
                            if let Some(ItemRef::Season(season)) =
                                collection.get_item(&episode.season_id)
                            {
                                if let Some(ItemRef::Show(show)) =
                                    collection.get_item(&episode.show_id)
                                {
                                    let mut dto = convert_episode_to_dto(
                                        &episode,
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
                                        key: data.itemid.clone(),
                                        unplayed_item_count: None,
                                    });
                                    resume_items.push(dto);
                                }
                            }
                        }
                        // We generally don't resume Shows or Seasons/Collections, but if needed they'd go here
                        _ => {}
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

async fn toggle_played(
    state: AppState,
    item_id: String,
    req: Request<axum::body::Body>,
    is_played: bool,
) -> Result<Json<UserData>, StatusCode> {
    let user_id = get_user_id(&req).ok_or(StatusCode::UNAUTHORIZED)?;

    let mut user_data = state
        .db
        .get_user_data(&user_id, &item_id)
        .await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));

    user_data.played = Some(is_played);
    user_data.playcount = Some(user_data.playcount.unwrap_or(0) + is_played as i32);
    user_data.position = None;
    user_data.timestamp = Some(chrono::Utc::now());

    state
        .db
        .upsert_user_data(&user_data)
        .await
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

    let mut user_data = state
        .db
        .get_user_data(&user_id, &item_id)
        .await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));

    user_data.favorite = Some(is_favorite);
    user_data.timestamp = Some(chrono::Utc::now());

    state
        .db
        .upsert_user_data(&user_data)
        .await
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

    let position_ticks = params
        .get("positionTicks")
        .and_then(|s| s.parse::<i64>().ok());

    let mut user_data = state
        .db
        .get_user_data(&user_id, &item_id)
        .await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));

    user_data.position = position_ticks;
    user_data.timestamp = Some(chrono::Utc::now());

    state
        .db
        .upsert_user_data(&user_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Request body for /Sessions/Playing/Progress
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlayingProgressRequest {
    #[serde(alias = "itemId")]
    pub item_id: Option<String>,
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
    axum::Extension(user_id): axum::Extension<String>,
    State(state): State<AppState>,
    Json(progress): Json<PlayingProgressRequest>,
) -> Result<StatusCode, StatusCode> {
    let item_id = match progress.item_id {
        Some(id) => id,
        None => return Ok(StatusCode::BAD_REQUEST), // item_id is required
    };

    let mut user_data = state
        .db
        .get_user_data(&user_id, &item_id)
        .await
        .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));

    user_data.position = Some(progress.position_ticks);
    user_data.timestamp = Some(chrono::Utc::now());

    state
        .db
        .upsert_user_data(&user_data)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_playing_item(
    axum::Extension(user_id): axum::Extension<String>,
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    Query(params): Query<QueryParams>,
) -> Result<StatusCode, StatusCode> {
    // If positionTicks is provided, save it one last time
    let position_ticks = params
        .get("positionTicks")
        .and_then(|s| s.parse::<i64>().ok());

    if let Some(pos) = position_ticks {
        let mut user_data = state
            .db
            .get_user_data(&user_id, &item_id)
            .await
            .unwrap_or_else(|_| get_default_db_user_data(&user_id, &item_id));

        user_data.position = Some(pos);
        user_data.timestamp = Some(chrono::Utc::now());

        state
            .db
            .upsert_user_data(&user_data)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn get_default_db_user_data(user_id: &str, item_id: &str) -> crate::db::UserData {
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
