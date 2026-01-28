use axum::{
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    Json,
};

use super::auth::get_user_id;
use super::types::*;
use crate::db::UserDataRepo;
use crate::jellyfin::item::convert_episode_to_dto;
use crate::jellyfin::item::convert_season_to_dto;
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

pub async fn get_seasons(
    State(state): State<AppState>,
    Path(show_id): Path<String>,
    Query(_params): Query<QueryParams>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let mut seasons_dto = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();

    // Scan all collections for the show
    let collections = state.collections.list_collections().await;
    for collection in collections {
        if let Some(show) = collection.shows.get(&show_id) {
            // Found the show, getting seasons
            let mut seasons: Vec<_> = show.seasons.values().collect();
            seasons.sort_by_key(|s| s.season_number);

            for season in seasons {
                seasons_dto.push(convert_season_to_dto(
                    season,
                    &show.id,
                    &collection.id, // parent_id for season is usually show_id, but here it might be context dependent.
                    // In convert_season_to_dto logic:
                    // pub fn convert_season_to_dto(season: &crate::collection::Season, show_id: &str, _parent_id: &str, series_name: &str)
                    // It ignores passing parent_id really? Checking impl:
                    // parent_id: Some(show_id.to_string()), (It uses show_id as parent_id inside)
                    &show.name,
                    &server_id,
                ));
            }

            return Ok(Json(QueryResult {
                total_record_count: seasons_dto.len(),
                items: seasons_dto,
            }));
        }
    }

    Err(StatusCode::NOT_FOUND)
}

pub async fn get_episodes(
    State(state): State<AppState>,
    Path(show_id): Path<String>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResult<BaseItemDto>>, StatusCode> {
    let season_id = params.get("seasonId");
    let mut episodes = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();

    // Scan all collections for the show
    let collections = state.collections.list_collections().await;
    for collection in collections {
        if let Some(show) = collection.shows.get(&show_id) {
            if let Some(sid) = season_id {
                // Return episodes for specific season
                // First try to find by ID string
                let mut found_season = None;
                for season in show.seasons.values() {
                    if season.id == *sid {
                        found_season = Some(season);
                        break;
                    }
                }

                // If not found by ID, try parsing as season number (fallback)
                if found_season.is_none() {
                    if let Ok(sid_int) = sid.parse::<i32>() {
                        found_season = show.seasons.get(&sid_int);
                    }
                }

                if let Some(season) = found_season {
                    for episode in season.episodes.values() {
                        episodes.push(convert_episode_to_dto(
                            episode,
                            &season.id,
                            &show.id,
                            &collection.id,
                            &season.name,
                            &show.name,
                            &server_id,
                        ));
                    }
                }
            } else {
                // Return all episodes from all seasons
                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        episodes.push(convert_episode_to_dto(
                            episode,
                            &season.id,
                            &show.id,
                            &collection.id,
                            &season.name,
                            &show.name,
                            &server_id,
                        ));
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
                    a.index_number
                        .unwrap_or(0)
                        .cmp(&b.index_number.unwrap_or(0))
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
                    if episode.season_number > last_watched_season
                        || (episode.season_number == last_watched_season
                            && episode.episode_number > last_watched_episode)
                    {
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
                let dto = convert_episode_to_dto(
                    next_episode,
                    &season.id,
                    &show.id,
                    &collection.id,
                    &season.name,
                    &show.name,
                    &server_id,
                );
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
                        let dto = convert_episode_to_dto(
                            first_episode,
                            &next_season.id,
                            &show.id,
                            &collection.id,
                            &next_season.name,
                            &show.name,
                            &server_id,
                        );
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

                let dto = convert_episode_to_dto(
                    first_episode,
                    &first_season.id,
                    &show.id,
                    &collection.id,
                    &first_season.name,
                    &show.name,
                    &server_id,
                );
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

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    let series_id = params.get("seriesId");

    let mut next_up_items = Vec::new();
    let server_id = state
        .config
        .jellyfin
        .server_id
        .as_deref()
        .unwrap_or_default();

    if let Some(sid) = series_id {
        // Direct lookup
        if let Some((collection_id, item)) = state.collections.get_item(sid) {
            if let crate::collection::repo::FoundItem::Show(show) = item {
                if let Some(collection) = state.collections.get_collection(&collection_id).await {
                    if let Some((_, dto)) =
                        find_next_up_for_show(&state, &user_id, &show, &collection, server_id, true)
                            .await
                    {
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
                if let Some((date, dto)) =
                    find_next_up_for_show(&state, &user_id, show, collection, &server_id, false)
                        .await
                {
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
