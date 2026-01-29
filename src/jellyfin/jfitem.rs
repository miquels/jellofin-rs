use std::collections::HashMap;

use super::types::*;
use super::userdata::get_default_user_data;
use crate::collection::item::MediaSource;

pub fn convert_media_sources(
    sources: &[crate::collection::MediaSource],
    item_id: &str,
) -> Option<Vec<MediaSourceInfo>> {
    if sources.is_empty() {
        return None;
    }

    Some(
        sources
            .iter()
            .map(|ms| convert_to_media_source_info(ms, item_id, None))
            .collect(),
    )
}

pub fn convert_movie_to_dto(
    movie: &crate::collection::Movie,
    parent_id: &str,
    server_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if movie.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), movie.id.clone());
    }
    if movie.images.backdrop.is_some() {
        image_tags.insert("Backdrop".to_string(), movie.id.clone());
    }

    let backdrop_image_tags = if movie.images.backdrop.is_some() {
        Some(vec![movie.id.clone()])
    } else {
        None
    };

    let provider_ids = HashMap::new();

    BaseItemDto {
        name: movie.name.clone(),
        id: movie.id.clone(),
        item_type: "Movie".to_string(),
        collection_type: None,
        overview: movie.overview.clone(),
        production_year: movie.production_year,
        premiere_date: movie.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: movie.community_rating.map(|r| r as f32),
        runtime_ticks: movie.runtime_ticks,
        genres: Some(movie.genres.clone()),
        genre_items: Some(
            movie
                .genres
                .iter()
                .map(|g| NameIdPair {
                    name: g.clone(),
                    id: format!("genre_{}", g), // Deterministic ID
                })
                .collect(),
        ),
        studios: Some(
            movie
                .studios
                .iter()
                .map(|s| NameIdPair {
                    name: s.clone(),
                    id: s.clone(),
                })
                .collect(),
        ),
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: None,
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: Some("VideoFile".to_string()),
        width: Some(1920),
        height: Some(1080),
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: Some(true),
        is_4k: Some(false),
        is_folder: Some(false),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(movie.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&movie.id)),
        media_sources: convert_media_sources(&movie.media_sources, &movie.id),
        provider_ids: Some(provider_ids),
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(movie.name.to_lowercase()),
        forced_sort_name: Some(movie.name.to_lowercase()),
        original_title: Some(movie.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),

        taglines: movie.tagline.as_ref().map(|t| vec![t.clone()]),
        channel_id: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    }
}

pub fn convert_show_to_dto(
    show: &crate::collection::Show,
    parent_id: &str,
    server_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if show.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), show.id.clone());
    }
    if show.images.backdrop.is_some() {
        image_tags.insert("Backdrop".to_string(), show.id.clone());
    }

    let backdrop_image_tags = if show.images.backdrop.is_some() {
        Some(vec![show.id.clone()])
    } else {
        None
    };

    let provider_ids = HashMap::new();

    BaseItemDto {
        name: show.name.clone(),
        id: show.id.clone(),
        item_type: "Series".to_string(),
        collection_type: None,
        overview: show.overview.clone(),
        production_year: show.production_year,
        premiere_date: show.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: show.community_rating.map(|r| r as f32),
        runtime_ticks: None,
        genres: Some(show.genres.clone()),
        studios: Some(
            show.studios
                .iter()
                .map(|s| NameIdPair {
                    name: s.clone(),
                    id: s.clone(),
                })
                .collect(),
        ),
        people: Some(vec![]),
        chapters: None,
        has_subtitles: None,
        parent_logo_item_id: None,
        parent_id: Some(parent_id.to_string()),
        series_id: None,
        series_name: None,
        season_id: None,
        season_name: None,
        index_number: None,
        parent_index_number: None,
        child_count: Some(show.seasons.len() as i32),
        image_tags,
        backdrop_image_tags,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(show.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&show.id)),
        media_sources: None,
        provider_ids: Some(provider_ids),
        recursive_item_count: Some(
            show.seasons
                .iter()
                .map(|(_, s)| s.episodes.len() as i32)
                .sum(),
        ),
        official_rating: Some("TV-MA".to_string()),
        sort_name: Some(show.name.to_lowercase()),
        forced_sort_name: Some(show.name.to_lowercase()),
        original_title: Some(show.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: Some(vec![]),
        channel_id: None,
        genre_items: Some(
            show.genres
                .iter()
                .map(|g| NameIdPair {
                    name: g.clone(),
                    id: format!("genre_{}", g),
                })
                .collect(),
        ),
        play_access: Some("Full".to_string()),
        enable_media_source_display: Some(false),
    }
}

pub fn convert_season_to_dto(
    season: &crate::collection::Season,
    show_id: &str,
    _parent_id: &str,
    series_name: &str,
    server_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if season.images.primary.is_some() {
        image_tags.insert("Primary".to_string(), season.id.clone());
    }

    BaseItemDto {
        name: season.name.clone(),
        id: season.id.clone(),
        item_type: "Season".to_string(),
        collection_type: None,
        overview: None,
        production_year: None,
        premiere_date: None,
        community_rating: None,
        runtime_ticks: None,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: Some(vec![]),
        has_subtitles: Some(true),
        parent_logo_item_id: Some(show_id.to_string()),
        parent_id: Some(season.id.to_string()),
        series_id: Some(show_id.to_string()),
        series_name: Some(series_name.to_string()),
        season_id: Some(season.id.to_string()),
        season_name: Some(season.name.to_string()),
        index_number: Some(season.season_number),
        parent_index_number: None,
        child_count: Some(season.episodes.len() as i32),
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        container: None,
        video_type: None,
        width: None,
        height: None,
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: None,
        is_4k: None,
        is_folder: Some(true),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: None,
        user_data: Some(get_default_user_data(&season.id)),
        media_sources: None,
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(season.name.to_lowercase()),
        forced_sort_name: Some(season.name.to_lowercase()),
        original_title: Some(season.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: None,
        channel_id: None,
        genre_items: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: None,
    }
}

pub fn convert_episode_to_dto(
    episode: &crate::collection::Episode,
    season_id: &str,
    show_id: &str,
    _parent_id: &str,
    season_name: &str,
    series_name: &str,
    server_id: &str,
) -> BaseItemDto {
    let mut image_tags = HashMap::new();
    if episode.images.primary.is_some() || episode.images.thumb.is_some() {
        image_tags.insert("Primary".to_string(), episode.id.clone());
    }

    BaseItemDto {
        name: episode.name.clone(),
        id: episode.id.clone(),
        item_type: "Episode".to_string(),
        collection_type: None,
        overview: episode.overview.clone(),
        production_year: None,
        premiere_date: episode.premiere_date.map(|d| d.to_rfc3339()),
        community_rating: episode.community_rating.map(|r| r as f32),
        runtime_ticks: episode.runtime_ticks,
        genres: None,
        studios: None,
        people: Some(vec![]),
        chapters: Some(vec![]),
        has_subtitles: Some(true),
        parent_logo_item_id: Some(show_id.to_string()),
        parent_id: Some(season_id.to_string()),
        series_id: Some(show_id.to_string()),
        series_name: Some(series_name.to_string()),
        season_id: Some(season_id.to_string()),
        season_name: Some(season_name.to_string()),
        index_number: Some(episode.episode_number),
        parent_index_number: Some(episode.season_number),
        child_count: None,
        image_tags,
        backdrop_image_tags: None,
        primary_image_aspect_ratio: None,
        server_id: Some(server_id.to_string()),
        video_type: Some("VideoFile".to_string()),
        width: Some(1920),
        height: Some(1080),
        image_blur_hashes: None,
        media_type: Some("Video".to_string()),
        is_hd: Some(true),
        is_4k: Some(false),
        is_folder: Some(false),
        location_type: Some("FileSystem".to_string()),
        path: None,
        etag: None,
        date_created: Some(episode.date_created.to_rfc3339()),
        user_data: Some(get_default_user_data(&episode.id)),
        media_sources: convert_media_sources(&episode.media_sources, &episode.id),
        provider_ids: None,
        recursive_item_count: None,
        official_rating: None,
        sort_name: Some(episode.name.to_lowercase()),
        forced_sort_name: Some(episode.name.to_lowercase()),
        original_title: Some(episode.name.clone()),
        can_delete: Some(true),
        can_download: Some(true),
        taglines: None,
        channel_id: None,
        container: None,
        genre_items: None,
        play_access: Some("Full".to_string()),
        enable_media_source_display: None,
    }
}

pub fn convert_to_media_source_info(
    ms: &MediaSource,
    item_id: &str,
    runtime_ticks: Option<i64>,
) -> MediaSourceInfo {
    let filename = ms
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("video.mp4")
        .to_string();
    MediaSourceInfo {
        id: item_id.to_string(),
        path: filename.clone(),
        name: filename,
        source_type: "Default".to_string(),
        protocol: Some("File".to_string()),
        container: ms
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_string(),
        video_type: Some("VideoFile".to_string()),
        size: Some(ms.size as i64),
        bitrate: ms.bitrate.map(|b| b as i32),
        run_time_ticks: runtime_ticks,
        etag: Some(item_id.to_string()),
        is_remote: false,
        supports_direct_stream: true,
        supports_direct_play: true,
        supports_transcoding: false,
        media_streams: Some(vec![
            MediaStream {
                stream_type: "Video".to_string(),
                codec: "h264".to_string(),
                language: None,
                index: Some(0),
                width: Some(1920),
                height: Some(1080),
                bit_rate: Some(5000000),
                is_default: Some(true),
                codec_tag: None,
                aspect_ratio: Some("16:9".to_string()),
                profile: Some("High".to_string()),
                time_base: None,
                ref_frames: None,
                is_anamorphic: None,
                bit_depth: Some(8),
                display_title: Some("1080p H264".to_string()),
                video_range: Some("SDR".to_string()),
                video_range_type: Some("SDR".to_string()),
                audio_spatial_format: None,
                localized_default: None,
                localized_external: None,
                channel_layout: None,
                channels: None,
                sample_rate: None,
                level: None,
                average_frame_rate: Some(24.0),
                real_frame_rate: Some(24.0),
                title: None,
                is_external: Some(false),
                is_text_subtitle_stream: Some(false),
                supports_external_stream: Some(false),
                pixel_format: Some("yuv420p".to_string()),
                is_interlaced: Some(false),
                is_avc: Some(true),
                is_hearing_impaired: Some(false),
                is_forced: Some(false),
            },
            MediaStream {
                stream_type: "Audio".to_string(),
                codec: "aac".to_string(),
                language: Some("eng".to_string()),
                index: Some(1),
                width: None,
                height: None,
                bit_rate: Some(128000),
                is_default: Some(true),
                codec_tag: None,
                aspect_ratio: None,
                profile: Some("LC".to_string()),
                time_base: None,
                ref_frames: None,
                is_anamorphic: None,
                bit_depth: None,
                display_title: Some("AAC - Stereo".to_string()),
                video_range: None,
                video_range_type: None,
                audio_spatial_format: None,
                localized_default: None,
                localized_external: None,
                channel_layout: Some("stereo".to_string()),
                channels: Some(2),
                sample_rate: Some(48000),
                level: None,
                average_frame_rate: None,
                real_frame_rate: None,
                title: None,
                is_external: Some(false),
                is_text_subtitle_stream: Some(false),
                supports_external_stream: Some(false),
                pixel_format: None,
                is_interlaced: Some(false),
                is_avc: Some(false),
                is_hearing_impaired: Some(false),
                is_forced: Some(false),
            },
        ]),
        default_audio_stream_index: Some(1),
        direct_stream_url: Some(format!(
            "/Videos/{}/stream?mediaSourceId={}&static=true",
            item_id, item_id
        )),
        transcoding_sub_protocol: Some("http".to_string()),
        required_http_headers: None,
        read_at_native_framerate: None,
        has_segments: None,
        ignore_dts: None,
        ignore_index: None,
        gen_pts_input: None,
        is_infinite_stream: None,
        requires_opening: None,
        requires_closing: None,
        requires_looping: None,
        supports_probing: Some(true),
        media_attachments: None,
        formats: None,
    }
}

