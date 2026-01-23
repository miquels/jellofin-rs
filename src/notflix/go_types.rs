use serde::{Deserialize, Serialize};

// Go server compatible types for Notflix API

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoShowDetail {
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
    pub nfo: GoNfo,
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
    pub seasons: Vec<GoSeason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoMovieDetail {
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
    pub nfo: GoNfo,
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
pub struct GoNfo {
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoSeason {
    pub seasonno: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    pub episodes: Vec<GoEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoEpisode {
    pub name: String,
    pub seasonno: i32,
    pub episodeno: i32,
    pub nfo: GoEpisodeNfo,
    pub video: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoEpisodeNfo {
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
pub struct GoItemSummary {
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
    pub nfo: GoNfo,
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
}
