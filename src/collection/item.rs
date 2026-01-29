use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movie {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub original_title: Option<String>,
    pub path: PathBuf,
    pub premiere_date: Option<DateTime<Utc>>,
    pub production_year: Option<i32>,
    pub community_rating: Option<f64>,
    pub mpaa: Option<String>,
    pub runtime_ticks: Option<i64>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub people: Vec<Person>,
    pub images: ImageInfo,
    pub media_sources: Vec<MediaSource>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Show {
    pub id: String,
    pub collection_id: String,
    pub name: String,
    pub sort_name: Option<String>,
    pub original_title: Option<String>,
    pub path: PathBuf,
    pub premiere_date: Option<DateTime<Utc>>,
    pub production_year: Option<i32>,
    pub community_rating: Option<f64>,
    pub mpaa: Option<String>,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub genres: Vec<String>,
    pub studios: Vec<String>,
    pub people: Vec<Person>,
    pub images: ImageInfo,
    pub seasons: HashMap<i32, Season>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub id: String,
    pub show_id: String,
    pub collection_id: String,
    pub name: String,
    pub season_number: i32,
    pub path: PathBuf,
    pub premiere_date: Option<DateTime<Utc>>,
    pub overview: Option<String>,
    pub images: ImageInfo,
    pub episodes: HashMap<i32, Episode>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub show_id: String,
    pub season_id: String,
    pub collection_id: String,
    pub name: String,
    pub season_number: i32,
    pub episode_number: i32,
    pub path: PathBuf,
    pub premiere_date: Option<DateTime<Utc>>,
    pub community_rating: Option<f64>,
    pub runtime_ticks: Option<i64>,
    pub overview: Option<String>,
    pub images: ImageInfo,
    pub media_sources: Vec<MediaSource>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageInfo {
    pub primary: Option<PathBuf>,
    pub backdrop: Option<PathBuf>,
    pub logo: Option<PathBuf>,
    pub thumb: Option<PathBuf>,
    pub banner: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSource {
    pub path: PathBuf,
    pub container: String,
    pub size: u64,
    pub bitrate: Option<i64>,
    pub subtitles: Vec<SubtitleStream>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStream {
    pub path: PathBuf,
    pub language: Option<String>,
    pub codec: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub role: Option<String>,
    pub person_type: PersonType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PersonType {
    Actor,
    Director,
    Writer,
    Producer,
}

pub trait Item {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn collection_id(&self) -> &str;
    fn item_type(&self) -> ItemType;
    fn parent_id(&self) -> Option<&str>;
    fn sort_name(&self) -> &str;
    fn premiere_date(&self) -> Option<DateTime<Utc>>;
    fn production_year(&self) -> Option<i32>;
    fn community_rating(&self) -> Option<f64>;
    fn overview(&self) -> Option<&str>;
    fn genres(&self) -> &[String];
    fn images(&self) -> &ImageInfo;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    Movie,
    Series,
    Season,
    Episode,
}

impl ItemType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemType::Movie => "Movie",
            ItemType::Series => "Series",
            ItemType::Season => "Season",
            ItemType::Episode => "Episode",
        }
    }
}

impl Item for Movie {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn collection_id(&self) -> &str {
        &self.collection_id
    }
    fn item_type(&self) -> ItemType {
        ItemType::Movie
    }
    fn parent_id(&self) -> Option<&str> {
        None
    }
    fn sort_name(&self) -> &str {
        self.sort_name.as_deref().unwrap_or(&self.name)
    }
    fn premiere_date(&self) -> Option<DateTime<Utc>> {
        self.premiere_date
    }
    fn production_year(&self) -> Option<i32> {
        self.production_year
    }
    fn community_rating(&self) -> Option<f64> {
        self.community_rating
    }
    fn overview(&self) -> Option<&str> {
        self.overview.as_deref()
    }
    fn genres(&self) -> &[String] {
        &self.genres
    }
    fn images(&self) -> &ImageInfo {
        &self.images
    }
}

impl Item for Show {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn collection_id(&self) -> &str {
        &self.collection_id
    }
    fn item_type(&self) -> ItemType {
        ItemType::Series
    }
    fn parent_id(&self) -> Option<&str> {
        None
    }
    fn sort_name(&self) -> &str {
        self.sort_name.as_deref().unwrap_or(&self.name)
    }
    fn premiere_date(&self) -> Option<DateTime<Utc>> {
        self.premiere_date
    }
    fn production_year(&self) -> Option<i32> {
        self.production_year
    }
    fn community_rating(&self) -> Option<f64> {
        self.community_rating
    }
    fn overview(&self) -> Option<&str> {
        self.overview.as_deref()
    }
    fn genres(&self) -> &[String] {
        &self.genres
    }
    fn images(&self) -> &ImageInfo {
        &self.images
    }
}

impl Item for Season {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn collection_id(&self) -> &str {
        &self.collection_id
    }
    fn item_type(&self) -> ItemType {
        ItemType::Season
    }
    fn parent_id(&self) -> Option<&str> {
        Some(&self.show_id)
    }
    fn sort_name(&self) -> &str {
        &self.name
    }
    fn premiere_date(&self) -> Option<DateTime<Utc>> {
        self.premiere_date
    }
    fn production_year(&self) -> Option<i32> {
        None
    }
    fn community_rating(&self) -> Option<f64> {
        None
    }
    fn overview(&self) -> Option<&str> {
        self.overview.as_deref()
    }
    fn genres(&self) -> &[String] {
        &[]
    }
    fn images(&self) -> &ImageInfo {
        &self.images
    }
}

impl Item for Episode {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn collection_id(&self) -> &str {
        &self.collection_id
    }
    fn item_type(&self) -> ItemType {
        ItemType::Episode
    }
    fn parent_id(&self) -> Option<&str> {
        Some(&self.season_id)
    }
    fn sort_name(&self) -> &str {
        &self.name
    }
    fn premiere_date(&self) -> Option<DateTime<Utc>> {
        self.premiere_date
    }
    fn production_year(&self) -> Option<i32> {
        None
    }
    fn community_rating(&self) -> Option<f64> {
        self.community_rating
    }
    fn overview(&self) -> Option<&str> {
        self.overview.as_deref()
    }
    fn genres(&self) -> &[String] {
        &[]
    }
    fn images(&self) -> &ImageInfo {
        &self.images
    }
}
