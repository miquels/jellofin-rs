use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub created: Option<String>,
    pub lastlogin: Option<String>,
    pub lastused: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccessToken {
    pub token: String,
    pub userid: String,
    pub deviceid: Option<String>,
    pub devicename: Option<String>,
    pub applicationname: Option<String>,
    pub applicationversion: Option<String>,
    pub remoteaddress: Option<String>,
    pub created: Option<DateTime<Utc>>,
    pub lastused: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Item {
    pub id: String,
    pub parent_id: Option<String>,
    pub collection_id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub original_title: Option<String>,
    pub premiere_date: Option<DateTime<Utc>>,
    pub community_rating: Option<f64>,
    pub runtime_ticks: Option<i64>,
    pub production_year: Option<i32>,
    pub index_number: Option<i32>,
    pub parent_index_number: Option<i32>,
    pub item_type: String,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserData {
    pub userid: String,
    pub itemid: String,
    pub position: Option<i64>,
    pub playedpercentage: Option<i32>,
    pub played: Option<bool>,
    pub playcount: Option<i32>,
    pub favorite: Option<bool>,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub userid: String,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlaylistItem {
    pub playlistid: String,
    pub itemid: String,
    pub itemorder: i32,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
}

pub type DbResult<T> = Result<T, DbError>;
