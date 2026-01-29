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

pub trait ItemTrait {
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


#[derive(Debug, Clone)]
pub enum Item {
    Movie(crate::collection::Movie),
    Show(crate::collection::Show),
    Season(crate::collection::Season),
    Episode(crate::collection::Episode),
}

#[derive(Debug, Clone, Copy)]
pub enum ItemRef<'a> {
    Movie(&'a Movie),
    Show(&'a Show),
    Season(&'a Season),
    Episode(&'a Episode),
}

macro_rules! impl_item_trait {
    ($name: ident, $type: ty) => {
        impl ItemTrait for $type {
            fn id(&self) -> &str {
                match self {
                    $name::Movie(m) => &m.id,
                    $name::Show(s) => &s.id,
                    $name::Season(s) => &s.id,
                    $name::Episode(e) => &e.id,
                }
            }

            fn name(&self) -> &str {
                match self {
                    $name::Movie(m) => &m.name,
                    $name::Show(s) => &s.name,
                    $name::Season(s) => &s.name,
                    $name::Episode(e) => &e.name,
                }
            }

            fn collection_id(&self) -> &str {
                match self {
                    $name::Movie(m) => &m.collection_id,
                    $name::Show(s) => &s.collection_id,
                    $name::Season(s) => &s.collection_id,
                    $name::Episode(e) => &e.collection_id,
                }
            }

            fn item_type(&self) -> ItemType {
                match self {
                    $name::Movie(_) => ItemType::Movie,
                    $name::Show(_) => ItemType::Series,
                    $name::Season(_) => ItemType::Season,
                    $name::Episode(_) => ItemType::Episode,
                }
            }

            fn parent_id(&self) -> Option<&str> {
                match self {
                    $name::Movie(_) => None,
                    $name::Show(_) => None,
                    $name::Season(s) => Some(&s.show_id),
                    $name::Episode(e) => Some(&e.season_id),
                }
            }

            fn sort_name(&self) -> &str {
                match self {
                    $name::Movie(m) => m.sort_name.as_deref().unwrap_or(&m.name),
                    $name::Show(s) => s.sort_name.as_deref().unwrap_or(&s.name),
                    $name::Season(s) => &s.name,
                    $name::Episode(e) => &e.name,
                }
            }

            fn premiere_date(&self) -> Option<DateTime<Utc>> {
                match self {
                    $name::Movie(m) => m.premiere_date,
                    $name::Show(s) => s.premiere_date,
                    $name::Season(s) => s.premiere_date,
                    $name::Episode(e) => e.premiere_date,
                }
            }

            fn production_year(&self) -> Option<i32> {
                match self {
                    $name::Movie(m) => m.production_year,
                    $name::Show(s) => s.production_year,
                    $name::Season(_) => None,
                    $name::Episode(_) => None,
                }
            }

            fn community_rating(&self) -> Option<f64> {
                match self {
                    $name::Movie(m) => m.community_rating,
                    $name::Show(s) => s.community_rating,
                    $name::Season(_) => None,
                    $name::Episode(e) => e.community_rating,
                }
            }

            fn overview(&self) -> Option<&str> {
                match self {
                    $name::Movie(m) => m.overview.as_deref(),
                    $name::Show(s) => s.overview.as_deref(),
                    $name::Season(s) => s.overview.as_deref(),
                    $name::Episode(e) => e.overview.as_deref(),
                }
            }

            fn genres(&self) -> &[String] {
                match self {
                    $name::Movie(m) => &m.genres,
                    $name::Show(s) => &s.genres,
                    $name::Season(_) => &[],
                    $name::Episode(_) => &[],
                }
            }

            fn images(&self) -> &ImageInfo {
                match self {
                    $name::Movie(m) => &m.images,
                    $name::Show(s) => &s.images,
                    $name::Season(s) => &s.images,
                    $name::Episode(e) => &e.images,
                }
            }
        }
    }
}

impl_item_trait!(Item, Item);
impl_item_trait!(ItemRef, ItemRef<'_>);
