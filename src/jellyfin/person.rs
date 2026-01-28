use std::collections::HashSet;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};

use super::types::{BaseItemDto, NameIdPair, QueryResultNameIdPair};
use crate::server::AppState;
use crate::util::{generate_id, QueryParams};

pub async fn get_person_by_name(
    State(_state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<BaseItemDto>, StatusCode> {
    let id = generate_id(&name);
    let dto = BaseItemDto {
        name: name.clone(),
        id,
        item_type: "Person".to_string(),
        image_tags: std::collections::HashMap::new(),
        ..Default::default()
    };
    Ok(Json(dto))
}

pub async fn get_persons(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResultNameIdPair>, StatusCode> {
    let mut persons = HashSet::new();

    for collection in state.collections.list_collections().await {
        for movie in collection.movies.values() {
            for person in &movie.people {
                persons.insert(person.name.clone());
            }
        }

        for show in collection.shows.values() {
            for person in &show.people {
                persons.insert(person.name.clone());
            }
        }
    }

    let mut person_list: Vec<NameIdPair> = persons
        .into_iter()
        .map(|name| {
            let id = generate_id(&name);
            NameIdPair { name, id }
        })
        .collect();

    person_list.sort_by(|a, b| a.name.cmp(&b.name));

    let start_index = params
        .get("startIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(person_list.len());

    let total = person_list.len();
    let items = person_list
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
