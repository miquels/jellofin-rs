use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticationRequest {
    pub username: String,
    #[serde(rename = "Pw")]
    pub pw: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticationResult {
    pub user: UserDto,
    pub access_token: String,
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserDto {
    pub name: String,
    pub id: String,
    pub has_password: bool,
    pub has_configured_password: bool,
    pub has_configured_easy_password: bool,
    pub policy: UserPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPolicy {
    pub is_administrator: bool,
    pub is_disabled: bool,
    pub enable_all_folders: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SystemInfo {
    pub server_name: String,
    pub version: String,
    pub id: String,
    pub operating_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PublicSystemInfo {
    pub server_name: String,
    pub version: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemDto {
    pub name: String,
    pub id: String,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premiere_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_rating: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_ticks: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studios: Option<Vec<NameIdPair>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub people: Option<Vec<BaseItemPerson>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub season_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_index_number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_count: Option<i32>,
    pub image_tags: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop_image_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_image_aspect_ratio: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_blur_hashes: Option<ImageBlurHashes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_folder: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_data: Option<UserItemData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_sources: Option<Vec<MediaSourceInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_ids: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct ImageBlurHashes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub art: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NameIdPair {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BaseItemPerson {
    pub name: String,
    pub id: String,
    #[serde(rename = "Type")]
    pub person_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserItemData {
    pub played: bool,
    pub is_favorite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_position_ticks: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub play_count: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSourceInfo {
    pub id: String,
    pub path: String,
    pub container: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    pub supports_direct_stream: bool,
    pub supports_transcoding: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_streams: Option<Vec<MediaStream>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    #[serde(rename = "Type")]
    pub stream_type: String,
    pub codec: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total_record_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackInfoResponse {
    pub media_sources: Vec<MediaSourceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemCounts {
    pub movie_count: usize,
    pub series_count: usize,
    pub episode_count: usize,
    pub album_count: usize,
    pub song_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SearchHint {
    pub item_id: String,
    pub name: String,
    #[serde(rename = "Type")]
    pub item_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub production_year: Option<i32>,
}
