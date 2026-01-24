use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::server::AppState;

pub async fn stream_video_with_range(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    let (file_path, file_size) = find_video_file(&state, &item_id).await?;
    
    let range_header = headers.get(header::RANGE);
    
    if let Some(range_value) = range_header {
        stream_with_range(file_path, file_size, range_value).await
    } else {
        stream_full_file(file_path, file_size).await
    }
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

async fn stream_with_range(
    file_path: PathBuf,
    file_size: u64,
    range_header: &header::HeaderValue,
) -> Result<Response, StatusCode> {
    let range_str = range_header.to_str().map_err(|_| StatusCode::BAD_REQUEST)?;
    
    if !range_str.starts_with("bytes=") {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let range_spec = &range_str[6..];
    let parts: Vec<&str> = range_spec.split('-').collect();
    
    if parts.len() != 2 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let start = if parts[0].is_empty() {
        0
    } else {
        parts[0].parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?
    };
    
    let end = if parts[1].is_empty() {
        file_size - 1
    } else {
        parts[1].parse::<u64>().map_err(|_| StatusCode::BAD_REQUEST)?
    };
    
    if start > end || end >= file_size {
        return Err(StatusCode::RANGE_NOT_SATISFIABLE);
    }
    
    let mut file = File::open(&file_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    file.seek(std::io::SeekFrom::Start(start)).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let content_length = end - start + 1;
    let limited_reader = file.take(content_length);
    let stream = ReaderStream::new(limited_reader);
    let body = Body::from_stream(stream);
    
    let content_range = format!("bytes {}-{}/{}", start, end, file_size);
    let content_type = get_content_type(&file_path);
    
    Ok((
        StatusCode::PARTIAL_CONTENT,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_LENGTH, &content_length.to_string()),
            (header::CONTENT_RANGE, &content_range),
            (header::ACCEPT_RANGES, "bytes"),
        ],
        body,
    ).into_response())
}

async fn stream_full_file(file_path: PathBuf, file_size: u64) -> Result<Response, StatusCode> {
    let file = File::open(&file_path).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    let content_type = get_content_type(&file_path);
    
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_LENGTH, &file_size.to_string()),
            (header::ACCEPT_RANGES, "bytes"),
        ],
        body,
    ).into_response())
}

fn get_content_type(path: &std::path::Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("avi") => "video/x-msvideo",
        Some("mov") => "video/quicktime",
        _ => "video/x-matroska",
    }
}
