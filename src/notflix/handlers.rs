use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use crate::server::AppState;
use super::types::*;

pub async fn list_collections(State(state): State<AppState>) -> Json<Vec<CollectionInfo>> {
    let collections = state.collections.list_collections().await;
    
    let infos: Vec<CollectionInfo> = collections
        .iter()
        .map(|c| CollectionInfo {
            id: c.id.clone(),
            name: c.name.clone(),
            collection_type: c.collection_type.as_str().to_string(),
            path: c.directory.to_string_lossy().to_string(),
        })
        .collect();
    
    Json(infos)
}

pub async fn get_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<Json<CollectionDetail>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let mut items = Vec::new();
    
    for movie in collection.movies.values() {
        items.push(ItemSummary {
            id: movie.id.clone(),
            name: movie.name.clone(),
            item_type: "Movie".to_string(),
            year: movie.production_year,
            overview: movie.overview.clone(),
            rating: movie.community_rating.map(|r| r as f32),
            genres: movie.genres.clone(),
            images: convert_images(&movie.id, &movie.images),
        });
    }
    
    for show in collection.shows.values() {
        items.push(ItemSummary {
            id: show.id.clone(),
            name: show.name.clone(),
            item_type: "Series".to_string(),
            year: show.production_year,
            overview: show.overview.clone(),
            rating: show.community_rating.map(|r| r as f32),
            genres: show.genres.clone(),
            images: convert_images(&show.id, &show.images),
        });
    }
    
    let detail = CollectionDetail {
        id: collection.id.clone(),
        name: collection.name.clone(),
        collection_type: collection.collection_type.as_str().to_string(),
        path: collection.directory.to_string_lossy().to_string(),
        items,
    };
    
    Ok(Json(detail))
}

pub async fn get_collection_genres(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<Json<Vec<GenreCount>>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let genre_counts = collection.get_genres();
    let mut genres: Vec<GenreCount> = genre_counts
        .into_iter()
        .map(|(genre, count)| GenreCount { genre, count })
        .collect();
    
    genres.sort_by(|a, b| b.count.cmp(&a.count).then(a.genre.cmp(&b.genre)));
    
    Ok(Json(genres))
}

pub async fn get_collection_items(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<ItemSummary>>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let genre_filter = params.get("genre");
    
    let mut items = Vec::new();
    
    for movie in collection.movies.values() {
        if let Some(genre) = genre_filter {
            if !movie.genres.iter().any(|g| g == genre) {
                continue;
            }
        }
        
        items.push(ItemSummary {
            id: movie.id.clone(),
            name: movie.name.clone(),
            item_type: "Movie".to_string(),
            year: movie.production_year,
            overview: movie.overview.clone(),
            rating: movie.community_rating.map(|r| r as f32),
            genres: movie.genres.clone(),
            images: convert_images(&movie.id, &movie.images),
        });
    }
    
    for show in collection.shows.values() {
        if let Some(genre) = genre_filter {
            if !show.genres.iter().any(|g| g == genre) {
                continue;
            }
        }
        
        items.push(ItemSummary {
            id: show.id.clone(),
            name: show.name.clone(),
            item_type: "Series".to_string(),
            year: show.production_year,
            overview: show.overview.clone(),
            rating: show.community_rating.map(|r| r as f32),
            genres: show.genres.clone(),
            images: convert_images(&show.id, &show.images),
        });
    }
    
    Ok(Json(items))
}

pub async fn get_item(
    State(state): State<AppState>,
    Path((collection_id, item_id)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    if let Some(movie) = collection.movies.get(&item_id) {
        let detail = MovieDetail {
            id: movie.id.clone(),
            name: movie.name.clone(),
            item_type: "Movie".to_string(),
            year: movie.production_year,
            overview: movie.overview.clone(),
            rating: movie.community_rating.map(|r| r as f32),
            tagline: movie.tagline.clone(),
            genres: movie.genres.clone(),
            studios: movie.studios.clone(),
            people: movie
                .people
                .iter()
                .map(|p| PersonInfo {
                    name: p.name.clone(),
                    person_type: format!("{:?}", p.person_type),
                    role: p.role.clone(),
                })
                .collect(),
            images: convert_images(&movie.id, &movie.images),
            media_sources: movie
                .media_sources
                .iter()
                .map(|ms| MediaSourceInfo {
                    path: ms.path.to_string_lossy().to_string(),
                    size: Some(ms.size),
                    subtitles: ms
                        .subtitles
                        .iter()
                        .map(|s| SubtitleInfo {
                            path: s.path.to_string_lossy().to_string(),
                            language: s.language.clone(),
                            codec: s.codec.clone(),
                        })
                        .collect(),
                })
                .collect(),
        };
        return Ok(Json(detail).into_response());
    }
    
    if let Some(show) = collection.shows.get(&item_id) {
        let seasons: Vec<SeasonInfo> = show
            .seasons
            .values()
            .map(|season| {
                let episodes: Vec<EpisodeInfo> = season
                    .episodes
                    .values()
                    .map(|ep| EpisodeInfo {
                        id: ep.id.clone(),
                        name: ep.name.clone(),
                        season_number: ep.season_number,
                        episode_number: ep.episode_number,
                        overview: ep.overview.clone(),
                        rating: ep.community_rating.map(|r| r as f32),
                        images: convert_images(&ep.id, &ep.images),
                        media_sources: ep
                            .media_sources
                            .iter()
                            .map(|ms| MediaSourceInfo {
                                path: ms.path.to_string_lossy().to_string(),
                                size: Some(ms.size),
                                subtitles: ms
                                    .subtitles
                                    .iter()
                                    .map(|s| SubtitleInfo {
                                        path: s.path.to_string_lossy().to_string(),
                                        language: s.language.clone(),
                                        codec: s.codec.clone(),
                                    })
                                    .collect(),
                            })
                            .collect(),
                    })
                    .collect();
                
                SeasonInfo {
                    id: season.id.clone(),
                    name: season.name.clone(),
                    season_number: season.season_number,
                    episode_count: episodes.len(),
                    images: convert_images(&season.id, &season.images),
                    episodes,
                }
            })
            .collect();
        
        let detail = ShowDetail {
            id: show.id.clone(),
            name: show.name.clone(),
            item_type: "Series".to_string(),
            year: show.production_year,
            overview: show.overview.clone(),
            rating: show.community_rating.map(|r| r as f32),
            genres: show.genres.clone(),
            studios: show.studios.clone(),
            people: show
                .people
                .iter()
                .map(|p| PersonInfo {
                    name: p.name.clone(),
                    person_type: format!("{:?}", p.person_type),
                    role: p.role.clone(),
                })
                .collect(),
            images: convert_images(&show.id, &show.images),
            seasons,
        };
        return Ok(Json(detail).into_response());
    }
    
    Err(StatusCode::NOT_FOUND)
}

pub async fn serve_data_file(
    State(state): State<AppState>,
    Path(path_parts): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    use axum::body::Body;
    use axum::http::header;
    use tokio::fs::File;
    use tokio_util::io::ReaderStream;
    
    let parts: Vec<&str> = path_parts.split('/').collect();
    if parts.len() < 2 {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    let source = parts[0];
    let file_path = parts[1..].join("/");
    
    // Check if this is an HLS proxy request (contains .mp4/)
    if file_path.contains(".mp4/") {
        // Try HLS proxy
        return crate::notflix::hls_proxy(
            axum::extract::State(state),
            axum::extract::Path((source.to_string(), file_path.clone())),
            req,
        ).await;
    }
    
    let collection = state
        .collections
        .get_collection(source)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let full_path = collection.directory.join(&file_path);
    
    if !full_path.starts_with(&collection.directory) {
        return Err(StatusCode::FORBIDDEN);
    }
    
    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }
    
    let is_image = matches!(
        full_path.extension().and_then(|e| e.to_str()),
        Some("jpg") | Some("jpeg") | Some("png") | Some("webp") | Some("gif")
    );
    
    if is_image && (params.contains_key("width") || params.contains_key("height")) {
        let width = params.get("width").and_then(|w| w.parse().ok());
        let height = params.get("height").and_then(|h| h.parse().ok());
        let quality = params.get("quality").and_then(|q| q.parse().ok());
        
        let resized = state
            .image_resizer
            .resize_image(&full_path, width, height, quality)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let content_type = match full_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            _ => "application/octet-stream",
        };
        
        return Ok(([(header::CONTENT_TYPE, content_type)], resized).into_response());
    }
    
    let file = File::open(&full_path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let content_type = mime_guess::from_path(&full_path)
        .first_or_octet_stream()
        .to_string();
    
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    
    Ok(([(header::CONTENT_TYPE, content_type)], body).into_response())
}

fn convert_images(item_id: &str, images: &crate::collection::ImageInfo) -> ImageUrls {
    ImageUrls {
        primary: images
            .primary
            .as_ref()
            .map(|_| format!("/Images/{}/primary", item_id)),
        backdrop: images
            .backdrop
            .as_ref()
            .map(|_| format!("/Images/{}/backdrop", item_id)),
        logo: images
            .logo
            .as_ref()
            .map(|_| format!("/Images/{}/logo", item_id)),
        thumb: images
            .thumb
            .as_ref()
            .map(|_| format!("/Images/{}/thumb", item_id)),
        banner: images
            .banner
            .as_ref()
            .map(|_| format!("/Images/{}/banner", item_id)),
    }
}
