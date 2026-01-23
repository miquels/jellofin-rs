use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::debug;

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
    let primary_video = video_files[0].clone();

    let mut movie = Movie {
        id: movie_id,
        collection_id: collection_id.to_string(),
        name: movie_name.clone(),
        sort_name: None,
        original_title: None,
        path: primary_video.clone(),
        premiere_date: None,
        production_year: None,
        community_rating: None,
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
            if let Some(title) = metadata.title {
                movie.name = title;
            }
            movie.original_title = metadata.original_title;
            movie.sort_name = metadata.sort_title;
            movie.overview = metadata.plot;
            movie.tagline = metadata.tagline;
            movie.community_rating = metadata.rating;
            movie.production_year = metadata.year;
            movie.premiere_date = metadata.premiered;
            movie.genres = metadata.genres;
            movie.studios = metadata.studios;
            movie.people = metadata.people;
        }
    }

    for video_file in video_files {
        if let Ok(metadata) = fs::metadata(&video_file) {
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
            if let Some(title) = metadata.title {
                show.name = title;
            }
            show.original_title = metadata.original_title;
            show.sort_name = metadata.sort_title;
            show.overview = metadata.plot;
            show.tagline = metadata.tagline;
            show.community_rating = metadata.rating;
            show.production_year = metadata.year;
            show.premiere_date = metadata.premiered;
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
    let mut overview = None;
    let mut premiere_date = None;
    let mut community_rating = None;

    if nfo_path.exists() {
        if let Some(metadata) = parse_nfo_file(&nfo_path) {
            overview = metadata.plot;
            premiere_date = metadata.premiered;
            community_rating = metadata.rating;
        }
    }

    let subtitles = find_subtitles(path);
    let size = fs::metadata(path).ok()?.len();

    Some(Episode {
        id: episode_id,
        show_id: show_id.to_string(),
        season_id: season_id.to_string(),
        collection_id: collection_id.to_string(),
        name: episode_name,
        season_number: season_num,
        episode_number: episode_num,
        path: path.to_path_buf(),
        premiere_date,
        community_rating,
        runtime_ticks: None,
        overview,
        images: ImageInfo::default(),
        media_sources: vec![MediaSource {
            path: path.to_path_buf(),
            container: path.extension()?.to_str()?.to_string(),
            size,
            bitrate: None,
            subtitles,
        }],
        date_created: Utc::now(),
        date_modified: Utc::now(),
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

fn generate_id(collection_id: &str, name: &str) -> String {
    use sha2::{Sha256, Digest};
    
    // Create hash from collection_id:name
    let input = format!("{}:{}", collection_id, name);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
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
