use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct NameIdPair {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResultNameIdPair {
    #[serde(rename = "Items")]
    pub items: Vec<NameIdPair>,
    #[serde(rename = "TotalRecordCount")]
    pub total_record_count: usize,
    #[serde(rename = "StartIndex")]
    pub start_index: usize,
}

pub async fn get_genres(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
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
            let id = generate_metadata_id(&name);
            NameIdPair { name, id }
        })
        .collect();
    
    genre_list.sort_by(|a, b| a.name.cmp(&b.name));
    
    let start_index = params.get("StartIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    let limit = params.get("Limit")
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

pub async fn get_studios(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<QueryResultNameIdPair>, StatusCode> {
    let mut studios = HashSet::new();
    
    for collection in state.collections.list_collections().await {
        for movie in collection.movies.values() {
            for studio in &movie.studios {
                studios.insert(studio.clone());
            }
        }
        
        for show in collection.shows.values() {
            for studio in &show.studios {
                studios.insert(studio.clone());
            }
        }
    }
    
    let mut studio_list: Vec<NameIdPair> = studios
        .into_iter()
        .map(|name| {
            let id = generate_metadata_id(&name);
            NameIdPair { name, id }
        })
        .collect();
    
    studio_list.sort_by(|a, b| a.name.cmp(&b.name));
    
    let start_index = params.get("StartIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    let limit = params.get("Limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(studio_list.len());
    
    let total = studio_list.len();
    let items = studio_list
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

pub async fn get_persons(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
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
            let id = generate_metadata_id(&name);
            NameIdPair { name, id }
        })
        .collect();
    
    person_list.sort_by(|a, b| a.name.cmp(&b.name));
    
    let start_index = params.get("StartIndex")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);
    
    let limit = params.get("Limit")
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

fn generate_metadata_id(name: &str) -> String {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(format!("metadata:{}", name).as_bytes());
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
