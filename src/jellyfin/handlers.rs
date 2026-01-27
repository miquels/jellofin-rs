use axum::{
    extract::{Path, Query, State},
    http::{self, header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use tower::ServiceExt;
use tower_http::services::ServeFile;

use std::collections::HashMap;

use crate::collection::find_image_path;
use crate::server::AppState;
use super::types::*;
use super::items::{convert_movie_to_dto, convert_show_to_dto};

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

pub async fn get_movie_recommendations(
    State(_state): State<AppState>,
    Query(_params): Query<HashMap<String, String>>,
    _req: http::Request<axum::body::Body>,
) -> Json<Vec<serde_json::Value>> {
    // Stub implementation - return empty list
    // TODO: Implement recommendation engine
    Json(vec![])
}

pub async fn get_suggestions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<QueryResult<BaseItemDto>> {
    // Stub: Return latest items as suggestions for now
    let limit = params.get("Limit")
        .or_else(|| params.get("limit"))
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(12);
        
    // Reuse get_latest_items logic but return QueryResult
    // We can just call the public get_latest_items if we construct the query, 
    // but cleaner to just reimplement the simple fetch here.
    
    let mut all_items = Vec::new();
    let server_id = state.config.jellyfin.server_id.clone().unwrap_or_default();
    
    // Get all collections
    let collections = state.collections.list_collections().await;
    for collection in collections {
        let coll_id = &collection.id;
        for movie in collection.movies.values() {
            all_items.push((movie.date_created, convert_movie_to_dto(movie, coll_id, &server_id)));
        }
        for show in collection.shows.values() {
            all_items.push((show.date_created, convert_show_to_dto(show, coll_id, &server_id)));
        }
    }
    
    // Sort by date descending
    all_items.sort_by(|a, b| b.0.cmp(&a.0));
    
    // Take random slice or just top keys? 
    // "Suggestions" usually implies "Similar", but without a similarity engine, 
    // returning *something* valid is better than 404. 
    // Let's return the latest items.
    
    let items: Vec<BaseItemDto> = all_items.into_iter()
        .take(limit)
        .map(|(_, dto)| dto)
        .collect();
        
    Json(QueryResult {
        items,
        total_record_count: limit, // Approximation
    })
}

pub async fn get_media_segments(
    State(_state): State<AppState>,
    Path(_item_id): Path<String>,
) -> Json<QueryResult<serde_json::Value>> {
    // Stub: No segments (Intro/Outro) known yet
    Json(QueryResult {
        items: vec![],
        total_record_count: 0,
    })
}

pub async fn get_grouping_options(
    State(state): State<AppState>,
) -> Json<Vec<serde_json::Value>> {
    // Return list of collections as grouping options, similar to Go's behavior
    let collections = state.collections.list_collections().await;
    let options: Vec<serde_json::Value> = collections.iter().map(|c| {
        serde_json::json!({
            "Id": c.id,
            "Name": c.name
        })
    }).collect();
    
    Json(options)
}

pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "no-cache, no-store")],
        "Healthy",
    )
}

pub async fn system_ping_handler() -> impl IntoResponse {
    (StatusCode::OK, "\"Jellyfin Server\"")
}

#[derive(serde::Deserialize)]
pub struct ImageParams {
    #[serde(rename = "type")]
    image_type: Option<String>,
    tag: Option<String>,
}

pub async fn image_handler(
    State(state): State<AppState>,
    Path((item_id, image_type)): Path<(String, String)>,
    Query(params): Query<ImageParams>,
    req: http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {

    if let Some(tag) = params.tag {
        // Jellyfin redirect tag.
        if let Some(url) = tag.strip_prefix("redirect_") {
            return Ok(Redirect::to(url).into_response().map(axum::body::Body::new));
        }

        // Jellyfin 'open local file' tag.
        if let Some(file) = tag.strip_prefix("file_") {
            // Let's only allow images, shall we.
            const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
            if let Some((_, ext)) = file.rsplit_once(".") {
                if IMAGE_EXTENSIONS.contains(&ext) {
                    let service = ServeFile::new(file);
                    let response = service
                        .oneshot(req)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Ok(response.map(axum::body::Body::new));
                }
            }
            return Err(StatusCode::NOT_FOUND);
        }
    }

    let image_path = find_image_path(&state.collections, &item_id, &image_type)
        .ok_or(StatusCode::NOT_FOUND)?;

    let quality = match params.image_type.as_deref() {
        Some("primary") | Some("logo") => state.config.jellyfin.image_quality_poster,
        _ => None,
    };

    let serve_path = state
        .image_resizer
        .resize_image(&image_path, None, None, quality)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Use ServeFile for proper ETag and Range header support
    let service = ServeFile::new(serve_path);
    let response = service
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response.map(axum::body::Body::new))
}

pub async fn image_handler_indexed(
    state: State<AppState>,
    Path((item_id, image_type, _index)): Path<(String, String, String)>,
    query: Query<ImageParams>,
    req: http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    // Ignore index for now, as we don't support multiple images per type yet
    image_handler(state, Path((item_id, image_type)), query, req).await
}
