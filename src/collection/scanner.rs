use chrono::{DateTime, Datelike, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use super::collection::{Collection, CollectionType};
use super::item::*;
use super::nfo::parse_nfo_file;
use super::parse_filename::{clean_title, parse_episode_from_filename};

const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m4v", "mov", "wmv", "flv", "webm"];
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp"];
const SUBTITLE_EXTENSIONS: &[&str] = &["srt", "vtt"];

pub fn scan_collection(collection: &mut Collection) -> Result<(), ScanError> {
    match collection.collection_type {
        CollectionType::Movies => scan_movies(collection),
        CollectionType::Shows => scan_shows(collection),
    }
}

fn scan_movies(collection: &mut Collection) -> Result<(), ScanError> {
    let dir = &collection.directory;
    if !dir.exists() {
        return Err(ScanError::DirectoryNotFound(dir.clone()));
    }

    collection.movies.clear();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if let Some(movie) = scan_movie_dir(&path, &collection.id) {
            collection.movies.insert(movie.id.clone(), movie);
        }
    }

    debug!("Scanned {} movies in collection {}", collection.movies.len(), collection.name);
    Ok(())
}

fn scan_movie_dir(dir: &Path, collection_id: &str) -> Option<Movie> {
    let movie_name = dir.file_name()?.to_str()?.to_string();
    let movie_id = generate_id(collection_id, &movie_name);

    let mut video_files = Vec::new();
    let mut nfo_path = None;
    let mut images = ImageInfo::default();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = path.file_name()?.to_str()?;
            let extension = path.extension()?.to_str()?.to_lowercase();

            if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
                video_files.push(path.clone());
            } else if extension == "nfo" {
                nfo_path = Some(path.clone());
            } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                assign_image(&mut images, filename, path.clone());
            }
        }
    }

    if video_files.is_empty() {
        return None;
    }

    video_files.sort();

    let mut movie = Movie {
        id: movie_id,
        collection_id: collection_id.to_string(),
        name: movie_name.clone(),
        sort_name: None,
        original_title: None,
        path: dir.to_path_buf(),
        premiere_date: None,
        production_year: None,
        community_rating: None,
        mpaa: None,
        runtime_ticks: None,
        overview: None,
        tagline: None,
        genres: Vec::new(),
        studios: Vec::new(),
        people: Vec::new(),
        images,
        media_sources: Vec::new(),
        date_created: Utc::now(),
        date_modified: Utc::now(),
    };

    if let Some(nfo_path) = nfo_path {
        if let Some(metadata) = parse_nfo_file(&nfo_path) {
            // Keep directory name as movie.name (matching Go server)
            // Store NFO title as original_title if different
            if let Some(title) = metadata.title {
                if title != movie_name {
                    movie.original_title = Some(title);
                }
            }
            movie.sort_name = metadata.sort_title;
            movie.overview = metadata.plot;
            movie.tagline = metadata.tagline;
            movie.community_rating = metadata.rating;
            movie.mpaa = metadata.mpaa;
            movie.production_year = metadata.year;
            movie.premiere_date = metadata.premiered;
            movie.genres = metadata.genres;
            movie.studios = metadata.studios;
            movie.people = metadata.people;
            // Parse runtime (in minutes) to ticks (100ns units)
            if let Some(runtime_str) = metadata.runtime {
                if let Ok(minutes) = runtime_str.parse::<i64>() {
                    movie.runtime_ticks = Some(minutes * 600_000_000);
                }
            }
        }
    }

    // Use file ctime for timestamps (matching Go server behavior)
    let mut earliest_time = Utc::now();
    let mut latest_time = Utc::now();
    let mut first = true;
    
    for video_file in video_files {
        if let Ok(metadata) = fs::metadata(&video_file) {
            // Track earliest and latest file ctimes
            let file_time = get_file_ctime(&metadata);
            if first {
                earliest_time = file_time;
                latest_time = file_time;
                first = false;
            } else {
                if file_time < earliest_time {
                    earliest_time = file_time;
                }
                if file_time > latest_time {
                    latest_time = file_time;
                }
            }
            
            let subtitles = find_subtitles(&video_file);
            movie.media_sources.push(MediaSource {
                path: video_file.clone(),
                container: video_file.extension()?.to_str()?.to_string(),
                size: metadata.len(),
                bitrate: None,
                subtitles,
            });
        }
    }
    
    movie.date_created = earliest_time;
    movie.date_modified = latest_time;

    Some(movie)
}

fn scan_shows(collection: &mut Collection) -> Result<(), ScanError> {
    let dir = &collection.directory;
    if !dir.exists() {
        return Err(ScanError::DirectoryNotFound(dir.clone()));
    }

    collection.shows.clear();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if let Some(show) = scan_show_dir(&path, &collection.id) {
            collection.shows.insert(show.id.clone(), show);
        }
    }

    debug!("Scanned {} shows in collection {}", collection.shows.len(), collection.name);
    Ok(())
}

fn scan_show_dir(dir: &Path, collection_id: &str) -> Option<Show> {
    let show_name = dir.file_name()?.to_str()?.to_string();
    let show_id = generate_id(collection_id, &show_name);

    let mut images = ImageInfo::default();
    let mut nfo_path = None;
    let mut seasons = HashMap::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_dir() {
                let dirname = path.file_name()?.to_str()?;
                if let Some(season_num) = parse_season_number(dirname) {
                    if let Some(season) = scan_season_dir(&path, &show_id, collection_id, season_num) {
                        seasons.insert(season_num, season);
                    }
                }
            } else {
                let filename = path.file_name()?.to_str()?;
                let extension = path.extension()?.to_str()?.to_lowercase();

                if extension == "nfo" {
                    nfo_path = Some(path.clone());
                } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                    assign_image(&mut images, filename, path.clone());
                }
            }
        }
    }

    let mut show = Show {
        id: show_id,
        collection_id: collection_id.to_string(),
        name: show_name.clone(),
        sort_name: None,
        original_title: None,
        path: dir.to_path_buf(),
        premiere_date: None,
        production_year: None,
        community_rating: None,
        mpaa: None,
        overview: None,
        tagline: None,
        genres: Vec::new(),
        studios: Vec::new(),
        people: Vec::new(),
        images,
        seasons,
        date_created: Utc::now(),
        date_modified: Utc::now(),
    };

    if let Some(nfo_path) = nfo_path {
        if let Some(metadata) = parse_nfo_file(&nfo_path) {
            // Keep directory name as show.name (matching Go server)
            // Store NFO title as original_title if different
            if let Some(title) = metadata.title {
                if title != show_name {
                    show.original_title = Some(title);
                }
            }
            // Use NFO original_title only if we haven't set it from title
            if show.original_title.is_none() {
                show.original_title = metadata.original_title;
            }
            show.sort_name = metadata.sort_title;
            show.overview = metadata.plot;
            show.tagline = metadata.tagline;
            show.community_rating = metadata.rating;
            show.mpaa = metadata.mpaa;
            show.production_year = metadata.year;
            show.premiere_date = metadata.premiered;
            // Derive year from premiered date if year not explicitly set
            if show.production_year.is_none() {
                if let Some(premiered) = show.premiere_date {
                    show.production_year = Some(premiered.year());
                }
            }
            show.genres = metadata.genres;
            show.studios = metadata.studios;
            show.people = metadata.people;
        }
    }

    Some(show)
}

fn scan_season_dir(dir: &Path, show_id: &str, collection_id: &str, season_num: i32) -> Option<Season> {
    let season_id = format!("{}:S{:02}", show_id, season_num);
    let season_name = if season_num == 0 {
        "Specials".to_string()
    } else {
        format!("Season {}", season_num)
    };

    let mut images = ImageInfo::default();
    let mut episodes = HashMap::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            
            if path.is_file() {
                let filename = path.file_name()?.to_str()?;
                let extension = path.extension()?.to_str()?.to_lowercase();

                if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
                    if let Some(ep_info) = parse_episode_from_filename(filename) {
                        if ep_info.season == season_num {
                            if let Some(episode) = create_episode(
                                &path,
                                show_id,
                                &season_id,
                                collection_id,
                                ep_info.season,
                                ep_info.episode,
                            ) {
                                episodes.insert(ep_info.episode, episode);
                            }
                        }
                    }
                } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
                    if filename.to_lowercase().contains(&format!("season{:02}", season_num))
                        || filename.to_lowercase().contains(&format!("season-{:02}", season_num))
                    {
                        assign_image(&mut images, filename, path.clone());
                    }
                }
            }
        }
    }

    Some(Season {
        id: season_id,
        show_id: show_id.to_string(),
        collection_id: collection_id.to_string(),
        name: season_name,
        season_number: season_num,
        path: dir.to_path_buf(),
        premiere_date: None,
        overview: None,
        images,
        episodes,
        date_created: Utc::now(),
        date_modified: Utc::now(),
    })
}

fn create_episode(
    path: &Path,
    show_id: &str,
    season_id: &str,
    collection_id: &str,
    season_num: i32,
    episode_num: i32,
) -> Option<Episode> {
    let filename = path.file_name()?.to_str()?;
    let episode_id = format!("{}:E{:02}", season_id, episode_num);
    let episode_name = clean_title(filename);

    let nfo_path = path.with_extension("nfo");
    let mut episode_title = episode_name.clone();
    let mut overview = None;
    let mut premiere_date = None;
    let mut community_rating = None;

    if nfo_path.exists() {
        if let Some(metadata) = parse_nfo_file(&nfo_path) {
            if let Some(title) = metadata.title {
                episode_title = title;
            }
            overview = metadata.plot;
            premiere_date = metadata.premiered;
            community_rating = metadata.rating;
        }
    }

    let subtitles = find_subtitles(path);
    let metadata = fs::metadata(path).ok()?;
    let size = metadata.len();
    
    // Use file ctime for timestamps (matching Go server behavior)
    let file_time = get_file_ctime(&metadata);

    Some(Episode {
        id: episode_id,
        show_id: show_id.to_string(),
        season_id: season_id.to_string(),
        collection_id: collection_id.to_string(),
        name: episode_title,
        season_number: season_num,
        episode_number: episode_num,
        path: path.to_path_buf(),
        premiere_date,
        community_rating,
        runtime_ticks: None,
        overview,
        images: find_episode_images(path),
        media_sources: vec![MediaSource {
            path: path.to_path_buf(),
            container: path.extension()?.to_str()?.to_string(),
            size,
            bitrate: None,
            subtitles,
        }],
        date_created: file_time,
        date_modified: file_time,
    })
}

fn parse_season_number(dirname: &str) -> Option<i32> {
    let lower = dirname.to_lowercase();
    
    if lower == "specials" || lower == "season 0" || lower == "s0" {
        return Some(0);
    }
    
    if let Some(stripped) = lower.strip_prefix("season ").or_else(|| lower.strip_prefix("season")) {
        return stripped.trim().parse::<i32>().ok();
    }
    
    if let Some(stripped) = lower.strip_prefix('s') {
        return stripped.parse::<i32>().ok();
    }
    
    None
}

fn assign_image(images: &mut ImageInfo, filename: &str, path: PathBuf) {
    let lower = filename.to_lowercase();
    
    if lower.contains("poster") {
        images.primary = Some(path);
    } else if lower.contains("fanart") || lower.contains("backdrop") {
        images.backdrop = Some(path);
    } else if lower.contains("logo") {
        images.logo = Some(path);
    } else if lower.contains("thumb") {
        images.thumb = Some(path);
    } else if lower.contains("banner") {
        images.banner = Some(path);
    } else if images.primary.is_none() {
        images.primary = Some(path);
    }
}

/// Find thumbnail images for an episode based on video filename.
/// Looks for images that match the video file's base name (e.g., "Show.S01E01-thumb.jpg" for "Show.S01E01.mkv")
fn find_episode_images(video_path: &Path) -> ImageInfo {
    let mut images = ImageInfo::default();
    
    let video_stem = match video_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return images,
    };
    
    let parent = match video_path.parent() {
        Some(p) => p,
        None => return images,
    };

    // Look for images matching video filename patterns:
    // - video_name.jpg, video_name-thumb.jpg
    // - video_name-poster.jpg, video_name-fanart.jpg
    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // Match images that start with the video filename (exact match or with suffix like -thumb)
                        if stem == video_stem || stem.starts_with(&format!("{}-", video_stem)) || stem.starts_with(&format!("{}.", video_stem)) {
                            assign_image(&mut images, stem, path.clone());
                        }
                    }
                }
            }
        }
    }

    images
}

fn find_subtitles(video_path: &Path) -> Vec<SubtitleStream> {
    let mut subtitles = Vec::new();
    
    let video_stem = match video_path.file_stem().and_then(|s| s.to_str()) {
        Some(s) => s,
        None => return subtitles,
    };
    
    let parent = match video_path.parent() {
        Some(p) => p,
        None => return subtitles,
    };

    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    let ext_lower = ext_str.to_lowercase();
                    if SUBTITLE_EXTENSIONS.contains(&ext_lower.as_str()) {
                        if let Some(stem) = path.file_stem() {
                            if let Some(stem_str) = stem.to_str() {
                                if stem_str.starts_with(video_stem) {
                                    let language = extract_language_from_filename(stem_str);
                                    subtitles.push(SubtitleStream {
                                        path: path.clone(),
                                        language,
                                        codec: ext_lower.clone(),
                                        title: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    subtitles
}

fn extract_language_from_filename(filename: &str) -> Option<String> {
    let parts: Vec<&str> = filename.split('.').collect();
    if parts.len() >= 2 {
        let potential_lang = parts[parts.len() - 1];
        if potential_lang.len() == 2 || potential_lang.len() == 3 {
            return Some(potential_lang.to_string());
        }
    }
    None
}

/// Get file creation time (ctime) as DateTime<Utc>
#[cfg(unix)]
fn get_file_ctime(metadata: &std::fs::Metadata) -> DateTime<Utc> {
    let ctime_secs = metadata.ctime();
    let ctime_nsecs = metadata.ctime_nsec();
    DateTime::from_timestamp(ctime_secs, ctime_nsecs as u32)
        .unwrap_or_else(Utc::now)
}

#[cfg(not(unix))]
fn get_file_ctime(metadata: &std::fs::Metadata) -> DateTime<Utc> {
    // Fallback for non-Unix systems
    metadata.modified()
        .ok()
        .and_then(|t| DateTime::<Utc>::from(t).into())
        .unwrap_or_else(Utc::now)
}

fn generate_id(_collection_id: &str, name: &str) -> String {
    use sha2::{Sha256, Digest};
    
    // Create hash from name only (matching Go server behavior)
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    
    // Take first 16 bytes (128 bits) and convert to big integer
    let mut num = [0u8; 16];
    num.copy_from_slice(&hash[..16]);
    
    // Convert to u128 and right shift by 9 bits to get 119 bits
    let mut value = u128::from_be_bytes(num);
    value >>= 9;
    
    // Convert to base62 (20 characters)
    let mut id = String::with_capacity(20);
    for _ in 0..20 {
        let remainder = (value % 62) as u8;
        value /= 62;
        
        let c = if remainder < 10 {
            (remainder + 48) as char  // 0-9
        } else if remainder < 36 {
            (remainder + 65 - 10) as char  // A-Z
        } else {
            (remainder + 97 - 36) as char  // a-z
        };
        id.push(c);
    }
    
    id
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
