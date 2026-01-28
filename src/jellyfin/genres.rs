use std::collections::HashSet;
use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    Json,
};

use crate::server::AppState;
use crate::util::{QueryParams, generate_id};
use super::types::{NameIdPair, QueryResultNameIdPair, BaseItemDto};

pub async fn get_genres(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResultNameIdPair>, StatusCode> {
    let mut genres = HashSet::new();
    
    for collection in state.collections.list_collections().await {
        for movie in collection.movies.values() {
            for genre in &movie.genres {
                genres.insert(genre.clone());
            }
        }
        
        for show in collection.shows.values() {
            for genre in &show.genres {
                genres.insert(genre.clone());
            }
        }
    }
    
    let mut genre_list: Vec<NameIdPair> = genres
        .into_iter()
        .map(|name| {
            let id = generate_id(&name);
            NameIdPair { name, id }
        })
        .collect();
    
    genre_list.sort_by(|a, b| a.name.cmp(&b.name));
    
    let start_index = params.get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    let limit = params.get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(genre_list.len());
    
    let total = genre_list.len();
    let items = genre_list
        .into_iter()
        .skip(start_index)
        .take(limit)
        .collect();
    
    Ok(Json(QueryResultNameIdPair {
        items,
        total_record_count: total,
        start_index,
    }))
}

pub async fn get_genre_by_name(
    State(_state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    // Stub implementation
    // Ideally this should look up the genre details. 
    // For now we return a basic DTO with the name and ID.
    let id = generate_id(&name);
    
    let dto = BaseItemDto {
        name: name.clone(),
        id,
        item_type: "Genre".to_string(),
        image_tags: std::collections::HashMap::new(),
        ..Default::default()
    };
    
    Ok(Json(dto))
}
