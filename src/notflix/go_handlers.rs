use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::path::Path as StdPath;
use crate::server::AppState;
use super::go_types::*;

pub async fn get_item_go(
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
        let video_filename = movie.media_sources.first()
            .and_then(|ms| ms.path.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        // Get the actual poster filename from images
        let poster_filename = movie.images.primary.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string());
        
        let thumb_path = movie.images.thumb.as_ref()
            .and_then(|t| t.file_name())
            .map(|n| n.to_string_lossy().to_string());
        
        let detail = GoMovieDetail {
            id: movie.id.clone(),
            name: movie.name.clone(),
            path: urlencoding::encode(&movie.name).to_string(),
            baseurl: collection.base_url.clone().unwrap_or_default(),
            item_type: "movie".to_string(),
            firstvideo: movie.date_created.timestamp_millis(),
            lastvideo: movie.date_modified.timestamp_millis(),
            sort_name: movie.sort_name.clone().unwrap_or_else(|| movie.name.to_lowercase()),
            nfo: GoNfo {
                id: movie.id.clone(),
                title: movie.original_title.clone().unwrap_or_else(|| movie.name.clone()),
                plot: movie.overview.clone(),
                premiered: movie.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: movie.mpaa.clone(),
                aired: movie.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                studio: movie.studios.first().cloned(),
                rating: movie.community_rating,
            },
            fanart: movie.images.backdrop.as_ref().map(|_| "fanart.jpg".to_string()),
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
        let mut seasons: Vec<GoSeason> = Vec::new();
        let mut first_video = i64::MAX;
        let mut last_video = i64::MIN;
        
        for season in show.seasons.values() {
            let mut episodes: Vec<GoEpisode> = Vec::new();
            
            for episode in season.episodes.values() {
                let video_path = episode.media_sources.first()
                    .map(|ms| {
                        // Get path relative to show directory
                        if let Ok(rel) = ms.path.strip_prefix(&show.path) {
                            rel.to_string_lossy().to_string()
                        } else {
                            ms.path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default()
                        }
                    })
                    .unwrap_or_default();
                
                let thumb_path = {
                    let video_stem = StdPath::new(&video_path).file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    if !video_stem.is_empty() {
                        Some(format!("{}-thumb.jpg", video_stem))
                    } else {
                        None
                    }
                };
                
                let ep_timestamp = episode.date_created.timestamp_millis();
                if ep_timestamp < first_video {
                    first_video = ep_timestamp;
                }
                if ep_timestamp > last_video {
                    last_video = ep_timestamp;
                }
                
                episodes.push(GoEpisode {
                    name: format!("{:02}x{:02}", episode.season_number, episode.episode_number),
                    seasonno: episode.season_number,
                    episodeno: episode.episode_number,
                    nfo: GoEpisodeNfo {
                        title: episode.name.clone(),
                        plot: episode.overview.clone(),
                        season: episode.season_number.to_string(),
                        episode: episode.episode_number.to_string(),
                        aired: episode.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                    },
                    video: video_path,
                    thumb: thumb_path,
                });
            }
            
            // Sort episodes by episode number
            episodes.sort_by_key(|e| e.episodeno);
            
            let poster_path = season.images.primary.as_ref()
                .map(|_| format!("season{:02}-poster.jpg", season.season_number));
            
            seasons.push(GoSeason {
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
        
        let season_all_banner = show.images.banner.as_ref().map(|_| "season-all-banner.jpg".to_string());
        let season_all_poster = show.images.primary.as_ref().map(|_| "season-all-poster.jpg".to_string());
        
        let detail = GoShowDetail {
            id: show.id.clone(),
            name: show.name.clone(),
            path: urlencoding::encode(&show.name).to_string(),
            baseurl: collection.base_url.clone().unwrap_or_default(),
            item_type: "show".to_string(),
            firstvideo: first_video,
            lastvideo: last_video,
            sort_name: show.sort_name.clone().unwrap_or_else(|| show.name.to_lowercase()),
            nfo: GoNfo {
                id: show.id.clone(),
                title: show.name.clone(),
                plot: show.overview.clone(),
                premiered: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: None,
                aired: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                studio: show.studios.first().cloned(),
                rating: show.community_rating,
            },
            fanart: show.images.backdrop.as_ref().map(|_| "fanart.jpg".to_string()),
            poster: show.images.primary.as_ref().map(|_| "poster.jpg".to_string()),
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

pub async fn get_collection_items_go(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<Json<Vec<GoItemSummary>>, StatusCode> {
    let collection = state
        .collections
        .get_collection(&collection_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    
    let mut items = Vec::new();
    
    for movie in collection.movies.values() {
        items.push(GoItemSummary {
            id: movie.id.clone(),
            name: movie.name.clone(),
            path: urlencoding::encode(&movie.name).to_string(),
            baseurl: collection.base_url.clone().unwrap_or_default(),
            item_type: "movie".to_string(),
            firstvideo: movie.date_created.timestamp_millis(),
            lastvideo: movie.date_modified.timestamp_millis(),
            sort_name: movie.sort_name.clone().unwrap_or_else(|| movie.name.to_lowercase()),
            nfo: GoNfo {
                id: movie.id.clone(),
                title: movie.original_title.clone().unwrap_or_else(|| movie.name.clone()),
                plot: movie.overview.clone(),
                premiered: movie.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: movie.mpaa.clone(),
                aired: movie.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                studio: movie.studios.first().cloned(),
                rating: movie.community_rating,
            },
            fanart: movie.images.backdrop.as_ref().map(|_| "fanart.jpg".to_string()),
            poster: movie.images.primary.as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string()),
            rating: movie.community_rating,
            genre: movie.genres.clone(),
            year: movie.production_year,
            season_all_banner: None,
            season_all_poster: None,
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
        
        items.push(GoItemSummary {
            id: show.id.clone(),
            name: show.name.clone(),
            path: urlencoding::encode(&show.name).to_string(),
            baseurl: collection.base_url.clone().unwrap_or_default(),
            item_type: "show".to_string(),
            firstvideo: first_video,
            lastvideo: last_video,
            sort_name: show.sort_name.clone().unwrap_or_else(|| show.name.to_lowercase()),
            nfo: GoNfo {
                id: show.id.clone(),
                title: show.name.clone(),
                plot: show.overview.clone(),
                premiered: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                mpaa: None,
                aired: show.premiere_date.map(|d| d.format("%Y-%m-%d").to_string()),
                studio: show.studios.first().cloned(),
                rating: show.community_rating,
            },
            fanart: show.images.backdrop.as_ref().map(|_| "fanart.jpg".to_string()),
            poster: show.images.primary.as_ref().map(|_| "poster.jpg".to_string()),
            rating: show.community_rating,
            genre: show.genres.clone(),
            year: show.production_year,
            season_all_banner: show.images.banner.as_ref().map(|_| "season-all-banner.jpg".to_string()),
            season_all_poster: show.images.primary.as_ref().map(|_| "season-all-poster.jpg".to_string()),
        });
    }
    
    Ok(Json(items))
}
