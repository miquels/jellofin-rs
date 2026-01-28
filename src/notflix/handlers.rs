use super::types::*;
use crate::collection::sort_name::make_sort_name;
use crate::server::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use tower::util::ServiceExt;
use tower_http::services::ServeFile;

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
) -> Result<Json<CollectionInfo>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let info = CollectionInfo {
        id: collection.id.clone(),
        name: collection.name.clone(),
        collection_type: collection.collection_type.as_str().to_string(),
        path: collection.directory.to_string_lossy().to_string(),
    };

    Ok(Json(info))
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

pub async fn serve_data_file(
    State(state): State<AppState>,
    Path(path_parts): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
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
        )
        .await;
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
        Some("jpg") | Some("jpeg") | Some("tbn") | Some("png") | Some("webp") | Some("gif")
    );

    // Determine the file path to serve (original or resized)
    let serve_path = if is_image
        && (params.contains_key("w") || params.contains_key("h") || params.contains_key("q"))
    {
        let width = params.get("w").and_then(|w| w.parse().ok());
        let height = params.get("h").and_then(|h| h.parse().ok());
        let quality = params.get("q").and_then(|q| q.parse().ok());

        state
            .image_resizer
            .resize_image(&full_path, width, height, quality)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        full_path
    };

    // Use ServeFile for proper Range header support
    let service = ServeFile::new(&serve_path);

    let mut response = service
        .oneshot(req)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add ETag header based on file metadata
    if let Ok(metadata) = std::fs::metadata(&serve_path) {
        use std::time::UNIX_EPOCH;

        let last_modified = metadata
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let file_size = metadata.len();
        let inode = metadata.ino();

        // Create a unique ETag: "W/size-timestamp"
        let etag = format!("\"{:x}-{:x}-{:x}\"", inode, file_size, last_modified);
        if let Ok(etag_value) = axum::http::HeaderValue::from_str(&etag) {
            response
                .headers_mut()
                .insert(axum::http::header::ETAG, etag_value);
        }
    }

    Ok(response.map(axum::body::Body::new))
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

    // Check if it's a movie
    if let Some(movie) = collection.movies.get(&item_id) {
        // Get the actual video filename (not the full path)
        let video_filename = movie
            .media_sources
            .first()
            .and_then(|ms| ms.path.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Get the actual poster filename from images
        let poster_filename = movie
            .images
            .primary
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());

        let thumb_path = movie
            .images
            .thumb
            .as_ref()
            .and_then(|t| t.file_name())
            .map(|n| n.to_string_lossy().to_string());

        let detail = MovieDetail {
            id: movie.id.clone(),
            name: movie.name.clone(),
            path: urlencoding::encode(&movie.name).to_string(),
            baseurl: format!("/data/{}", collection_id),
            item_type: "movie".to_string(),
            firstvideo: movie.date_created.timestamp_millis(),
            lastvideo: movie.date_modified.timestamp_millis(),
            sort_name: movie
                .sort_name
                .clone()
                .unwrap_or_else(|| make_sort_name(&movie.name)),
            nfo: Nfo {
                id: movie.id.clone(),
                title: movie.name.clone(),
                plot: movie.overview.clone(),
                premiered: movie
                    .premiere_date
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: movie.mpaa.clone(),
                aired: movie
                    .premiere_date
                    .map(|d| d.format("%Y-%m-%d").to_string()),
                studio: movie.studios.first().cloned(),
                rating: movie.community_rating,
                runtime: movie.runtime_ticks.map(|t| (t / 600_000_000).to_string()),
                year: movie.production_year,
                originaltitle: movie.original_title.clone(),
                genre: if movie.genres.is_empty() {
                    None
                } else {
                    Some(movie.genres.clone())
                },
                actor: if movie.people.is_empty() {
                    None
                } else {
                    Some(
                        movie
                            .people
                            .iter()
                            .filter(|p| {
                                matches!(p.person_type, crate::collection::PersonType::Actor)
                            })
                            .map(|p| Actor {
                                name: p.name.clone(),
                                role: p.role.clone(),
                            })
                            .collect(),
                    )
                },
                director: movie
                    .people
                    .iter()
                    .find(|p| matches!(p.person_type, crate::collection::PersonType::Director))
                    .map(|p| p.name.clone()),
                thumb: None,
                fanart: None,
            },
            fanart: movie
                .images
                .backdrop
                .as_ref()
                .map(|_| "fanart.jpg".to_string()),
            poster: poster_filename,
            rating: movie.community_rating,
            genre: movie.genres.clone(),
            year: movie.production_year,
            video: video_filename,
            thumb: thumb_path,
        };
        return Ok(Json(detail).into_response());
    }

    // Check if it's a show
    if let Some(show) = collection.shows.get(&item_id) {
        let mut seasons: Vec<Season> = Vec::new();
        let mut first_video = i64::MAX;
        let mut last_video = i64::MIN;

        for season in show.seasons.values() {
            let mut episodes: Vec<Episode> = Vec::new();

            for episode in season.episodes.values() {
                let video_path = episode
                    .media_sources
                    .first()
                    .map(|ms| {
                        // Get path relative to show directory
                        if let Ok(rel) = ms.path.strip_prefix(&show.path) {
                            rel.to_string_lossy().to_string()
                        } else {
                            ms.path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default()
                        }
                    })
                    .unwrap_or_default();

                let thumb_path = episode.images.thumb.as_ref().and_then(|t| {
                    // Get path relative to show directory
                    if let Ok(rel) = t.strip_prefix(&show.path) {
                        Some(rel.to_string_lossy().to_string())
                    } else {
                        t.file_name().map(|n| n.to_string_lossy().to_string())
                    }
                });

                let ep_timestamp = episode.date_created.timestamp_millis();
                if ep_timestamp < first_video {
                    first_video = ep_timestamp;
                }
                if ep_timestamp > last_video {
                    last_video = ep_timestamp;
                }

                episodes.push(Episode {
                    name: format!("{:02}x{:02}", episode.season_number, episode.episode_number),
                    seasonno: episode.season_number,
                    episodeno: episode.episode_number,
                    nfo: EpisodeNfo {
                        title: episode.name.clone(),
                        plot: episode.overview.clone(),
                        season: episode.season_number.to_string(),
                        episode: episode.episode_number.to_string(),
                        aired: episode
                            .premiere_date
                            .map(|d| d.format("%Y-%m-%d").to_string()),
                    },
                    video: video_path,
                    thumb: thumb_path,
                });
            }

            // Sort episodes by episode number
            episodes.sort_by_key(|e| e.episodeno);

            let poster_path = season
                .images
                .primary
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string());

            seasons.push(Season {
                seasonno: season.season_number,
                poster: poster_path,
                episodes,
            });
        }

        // Sort seasons by season number
        seasons.sort_by_key(|s| s.seasonno);

        if first_video == i64::MAX {
            first_video = show.date_created.timestamp_millis();
        }
        if last_video == i64::MIN {
            last_video = show.date_modified.timestamp_millis();
        }

        let season_all_banner = show
            .images
            .banner
            .as_ref()
            .map(|_| "season-all-banner.jpg".to_string());
        let season_all_poster = show
            .images
            .primary
            .as_ref()
            .map(|_| "season-all-poster.jpg".to_string());

        let detail = ShowDetail {
            id: show.id.clone(),
            name: show.name.clone(),
            path: urlencoding::encode(&show.name).to_string(),
            baseurl: format!("/data/{}", collection_id),
            item_type: "show".to_string(),
            firstvideo: first_video,
            lastvideo: last_video,
            sort_name: show
                .sort_name
                .clone()
                .unwrap_or_else(|| make_sort_name(&show.name)),
            nfo: Nfo {
                id: show.id.clone(),
                title: show.name.clone(),
                plot: show.overview.clone(),
                premiered: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: show.mpaa.clone(),
                aired: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                studio: show.studios.first().cloned(),
                rating: show.community_rating,
                runtime: None,
                year: show.production_year,
                originaltitle: None,
                genre: if show.genres.is_empty() {
                    None
                } else {
                    Some(show.genres.clone())
                },
                actor: if show.people.is_empty() {
                    None
                } else {
                    Some(
                        show.people
                            .iter()
                            .filter(|p| {
                                matches!(p.person_type, crate::collection::PersonType::Actor)
                            })
                            .map(|p| Actor {
                                name: p.name.clone(),
                                role: p.role.clone(),
                            })
                            .collect(),
                    )
                },
                director: show
                    .people
                    .iter()
                    .find(|p| matches!(p.person_type, crate::collection::PersonType::Director))
                    .map(|p| p.name.clone()),
                thumb: None,
                fanart: None,
            },
            banner: show
                .images
                .banner
                .as_ref()
                .map(|_| "banner.jpg".to_string()),
            fanart: show
                .images
                .backdrop
                .as_ref()
                .map(|_| "fanart.jpg".to_string()),
            poster: show
                .images
                .primary
                .as_ref()
                .map(|_| "poster.jpg".to_string()),
            rating: show.community_rating,
            genre: show.genres.clone(),
            year: show.production_year,
            season_all_banner,
            season_all_poster,
            seasons,
        };
        return Ok(Json(detail).into_response());
    }

    Err(StatusCode::NOT_FOUND)
}

pub async fn get_collection_items(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<Json<Vec<ItemSummary>>, StatusCode> {
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
            path: urlencoding::encode(&movie.name).to_string(),
            baseurl: format!("/data/{}", collection_id),
            item_type: "movie".to_string(),
            firstvideo: movie.date_created.timestamp_millis(),
            lastvideo: movie.date_modified.timestamp_millis(),
            sort_name: movie
                .sort_name
                .clone()
                .unwrap_or_else(|| make_sort_name(&movie.name)),
            banner: None,
            fanart: movie
                .images
                .backdrop
                .as_ref()
                .map(|_| "fanart.jpg".to_string()),
            poster: movie
                .images
                .primary
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string()),
            rating: movie.community_rating,
            genre: movie.genres.clone(),
            year: movie.production_year,
        });
    }

    for show in collection.shows.values() {
        let mut first_video = i64::MAX;
        let mut last_video = i64::MIN;

        for season in show.seasons.values() {
            for episode in season.episodes.values() {
                let ep_timestamp = episode.date_created.timestamp_millis();
                if ep_timestamp < first_video {
                    first_video = ep_timestamp;
                }
                if ep_timestamp > last_video {
                    last_video = ep_timestamp;
                }
            }
        }

        if first_video == i64::MAX {
            first_video = show.date_created.timestamp_millis();
        }
        if last_video == i64::MIN {
            last_video = show.date_modified.timestamp_millis();
        }

        items.push(ItemSummary {
            id: show.id.clone(),
            name: show.name.clone(),
            path: urlencoding::encode(&show.name).to_string(),
            baseurl: format!("/data/{}", collection_id),
            item_type: "show".to_string(),
            firstvideo: first_video,
            lastvideo: last_video,
            sort_name: show
                .sort_name
                .clone()
                .unwrap_or_else(|| make_sort_name(&show.name)),
            banner: show
                .images
                .banner
                .as_ref()
                .map(|_| "banner.jpg".to_string()),
            fanart: show
                .images
                .backdrop
                .as_ref()
                .map(|_| "fanart.jpg".to_string()),
            poster: show
                .images
                .primary
                .as_ref()
                .map(|_| "poster.jpg".to_string()),
            rating: show.community_rating,
            genre: show.genres.clone(),
            year: show.production_year,
        });
    }

    Ok(Json(items))
}
