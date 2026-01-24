use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenreCount {
    pub genre: String,
    pub count: usize,
}

// Go server compatible types for Notflix API

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowDetail {
    pub id: String,
    pub name: String,
    pub path: String,
    pub baseurl: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub firstvideo: i64,
    pub lastvideo: i64,
    #[serde(rename = "sortName")]
    pub sort_name: String,
    pub nfo: Nfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    pub genre: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "seasonAllBanner")]
    pub season_all_banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "seasonAllPoster")]
    pub season_all_poster: Option<String>,
    pub seasons: Vec<Season>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieDetail {
    pub id: String,
    pub name: String,
    pub path: String,
    pub baseurl: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub firstvideo: i64,
    pub lastvideo: i64,
    #[serde(rename = "sortName")]
    pub sort_name: String,
    pub nfo: Nfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    pub genre: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    pub video: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nfo {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premiered: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mpaa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub studio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originaltitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<Vec<Actor>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub director: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<Vec<FanartItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanartItem {
    pub thumb: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Season {
    pub seasonno: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub name: String,
    pub seasonno: i32,
    pub episodeno: i32,
    pub nfo: EpisodeNfo,
    pub video: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeNfo {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plot: Option<String>,
    pub season: String,
    pub episode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aired: Option<String>,
}

// For collection items listing (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSummary {
    pub id: String,
    pub name: String,
    pub path: String,
    pub baseurl: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub firstvideo: i64,
    pub lastvideo: i64,
    #[serde(rename = "sortName")]
    pub sort_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fanart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    pub genre: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
}
