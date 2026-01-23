use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::item::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub collection_type: CollectionType,
    pub directory: PathBuf,
    pub base_url: Option<String>,
    pub hls_server: Option<String>,
    pub movies: HashMap<String, Movie>,
    pub shows: HashMap<String, Show>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionType {
    Movies,
    Shows,
}

impl CollectionType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "movies" | "movie" => Some(CollectionType::Movies),
            "shows" | "show" | "tv" | "tvshows" => Some(CollectionType::Shows),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            CollectionType::Movies => "movies",
            CollectionType::Shows => "shows",
        }
    }
}

impl Collection {
    pub fn new(
        id: String,
        name: String,
        collection_type: CollectionType,
        directory: PathBuf,
        base_url: Option<String>,
        hls_server: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            collection_type,
            directory,
            base_url,
            hls_server,
            movies: HashMap::new(),
            shows: HashMap::new(),
        }
    }

    pub fn get_item(&self, id: &str) -> Option<ItemRef> {
        if let Some(movie) = self.movies.get(id) {
            return Some(ItemRef::Movie(movie));
        }

        for show in self.shows.values() {
            if show.id == id {
                return Some(ItemRef::Show(show));
            }
            for season in show.seasons.values() {
                if season.id == id {
                    return Some(ItemRef::Season(season));
                }
                for episode in season.episodes.values() {
                    if episode.id == id {
                        return Some(ItemRef::Episode(episode));
                    }
                }
            }
        }

        None
    }

    pub fn get_genres(&self) -> HashMap<String, usize> {
        let mut genre_counts = HashMap::new();

        match self.collection_type {
            CollectionType::Movies => {
                for movie in self.movies.values() {
                    for genre in &movie.genres {
                        *genre_counts.entry(genre.clone()).or_insert(0) += 1;
                    }
                }
            }
            CollectionType::Shows => {
                for show in self.shows.values() {
                    for genre in &show.genres {
                        *genre_counts.entry(genre.clone()).or_insert(0) += 1;
                    }
                }
            }
        }

        genre_counts
    }

    pub fn item_count(&self) -> usize {
        match self.collection_type {
            CollectionType::Movies => self.movies.len(),
            CollectionType::Shows => self.shows.len(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ItemRef<'a> {
    Movie(&'a Movie),
    Show(&'a Show),
    Season(&'a Season),
    Episode(&'a Episode),
}

impl<'a> ItemRef<'a> {
    pub fn as_item(&self) -> &'a dyn Item {
        match self {
            ItemRef::Movie(m) => *m as &dyn Item,
            ItemRef::Show(s) => *s as &dyn Item,
            ItemRef::Season(_) => panic!("Season does not implement Item trait"),
            ItemRef::Episode(e) => *e as &dyn Item,
        }
    }
}
