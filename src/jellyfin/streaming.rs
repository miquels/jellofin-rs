use axum::{
    body::Body,
    extract::{Path, State, Request},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::server::AppState;

pub async fn stream_video_with_range(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    req: Request,
) -> Result<Response, StatusCode> {
    let (file_path, _file_size) = find_video_file(&state, &item_id).await?;
    
    let service = ServeFile::new(file_path);
    let response = service
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
    Ok(response.map(Body::new))
}

pub async fn stream_subtitle(
    State(state): State<AppState>,
    Path((item_id, index)): Path<(String, usize)>,
) -> Result<Response, StatusCode> {
    let subtitle_path = find_subtitle_file(&state, &item_id, index).await?;
    
    let content = tokio::fs::read(&subtitle_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let content_type = if subtitle_path.extension().and_then(|e| e.to_str()) == Some("vtt") {
        "text/vtt"
    } else {
        "application/x-subrip"
    };
    
    Ok((
        [(header::CONTENT_TYPE, content_type)],
        content,
    ).into_response())
}

async fn find_video_file(state: &AppState, item_id: &str) -> Result<(PathBuf, u64), StatusCode> {
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(item_id) {
            if let Some(ms) = movie.media_sources.first() {
                let metadata = tokio::fs::metadata(&ms.path).await
                    .map_err(|_| StatusCode::NOT_FOUND)?;
                return Ok((ms.path.clone(), metadata.len()));
            }
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if episode.id == item_id {
                        if let Some(ms) = episode.media_sources.first() {
                            let metadata = tokio::fs::metadata(&ms.path).await
                                .map_err(|_| StatusCode::NOT_FOUND)?;
                            return Ok((ms.path.clone(), metadata.len()));
                        }
                    }
                }
            }
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}

async fn find_subtitle_file(state: &AppState, item_id: &str, index: usize) -> Result<PathBuf, StatusCode> {
    for collection in state.collections.list_collections().await {
        if let Some(movie) = collection.movies.get(item_id) {
            if let Some(ms) = movie.media_sources.first() {
                if let Some(subtitle) = ms.subtitles.get(index) {
                    return Ok(subtitle.path.clone());
                }
            }
        }
        
        for show in collection.shows.values() {
            for season in show.seasons.values() {
                for episode in season.episodes.values() {
                    if episode.id == item_id {
                        if let Some(ms) = episode.media_sources.first() {
                            if let Some(subtitle) = ms.subtitles.get(index) {
                                return Ok(subtitle.path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    
    Err(StatusCode::NOT_FOUND)
}
