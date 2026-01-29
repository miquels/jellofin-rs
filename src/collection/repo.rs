use arc_swap::ArcSwap;
use crate::collection::item::{ImageInfo, Item, ItemType};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

use super::collection::{Collection, CollectionType};
use super::scanner::{scan_collection, ScanError};
use super::search::{SearchIndex, SearchResult};
use crate::config::CollectionConfig;

pub struct CollectionRepo {
    collections: Arc<ArcSwap<HashMap<String, Collection>>>,
    search_index: Arc<SearchIndex>,
}

impl CollectionRepo {
    pub fn new() -> Result<Self, CollectionRepoError> {
        let search_index =
            SearchIndex::new().map_err(|e| CollectionRepoError::Search(e.to_string()))?;

        Ok(Self {
            collections: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            search_index: Arc::new(search_index),
        })
    }

    pub async fn add_collection(
        &self,
        config: &CollectionConfig,
    ) -> Result<(), CollectionRepoError> {
        let collection_type =
            CollectionType::from_str(&config.collection_type).ok_or_else(|| {
                CollectionRepoError::InvalidCollectionType(config.collection_type.clone())
            })?;

        let id = config
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let collection = Collection::new(
            id.clone(),
            config.name.clone(),
            collection_type,
            config.directory.clone().into(),
            config.baseurl.clone(),
            config.hlsserver.clone(),
        );

        // Clone current map, add new collection, and swap
        let mut new_collections = (**self.collections.load()).clone();
        new_collections.insert(id.clone(), collection);
        self.collections.store(Arc::new(new_collections));

        info!("Added collection: {} ({})", config.name, id);
        Ok(())
    }

    pub async fn scan_all(&self) -> Result<(), CollectionRepoError> {
        // Load current collections
        let collections = self.collections.load();
        let collection_ids: Vec<String> = collections.keys().cloned().collect();

        // Scan each collection in spawn_blocking
        for id in collection_ids {
            let collections = self.collections.load();

            if let Some(collection) = collections.get(&id) {
                info!("Scanning collection: {}", collection.name);

                // Clone collection for scanning (keeps original available)
                let mut cloned_collection = collection.clone();

                // Use spawn_blocking to avoid blocking the async runtime during filesystem I/O
                let scan_result = tokio::task::spawn_blocking(move || {
                    let result = scan_collection(&mut cloned_collection);
                    (cloned_collection, result)
                })
                .await;

                // Atomically update the collection after scan
                match scan_result {
                    Ok((scanned_collection, Ok(()))) => {
                        let mut new_collections = (**self.collections.load()).clone();
                        new_collections.insert(id.clone(), scanned_collection);
                        self.collections.store(Arc::new(new_collections));
                    }
                    Ok((scanned_collection, Err(e))) => {
                        error!("Failed to scan collection {}: {}", id, e);
                        // Still update with scanned collection even if there was an error
                        let mut new_collections = (**self.collections.load()).clone();
                        new_collections.insert(id.clone(), scanned_collection);
                        self.collections.store(Arc::new(new_collections));
                    }
                    Err(e) => {
                        error!("Scan task panicked for collection {}: {}", id, e);
                    }
                }
            }
        }

        info!("Rebuilding search index");
        let collections = self.collections.load();
        self.search_index
            .rebuild(&collections)
            .await
            .map_err(|e| CollectionRepoError::Search(e.to_string()))?;

        Ok(())
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, CollectionRepoError> {
        self.search_index
            .search(query, limit)
            .map_err(|e| CollectionRepoError::Search(e.to_string()))
    }

    pub fn find_similar(
        &self,
        item_id: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, CollectionRepoError> {
        self.search_index
            .find_similar(item_id, limit)
            .map_err(|e| CollectionRepoError::Search(e.to_string()))
    }

    pub async fn get_collection(&self, id: &str) -> Option<Collection> {
        let collections = self.collections.load();
        collections.get(id).cloned()
    }

    pub async fn list_collections(&self) -> Vec<Collection> {
        let collections = self.collections.load();
        collections.values().cloned().collect()
    }

    pub async fn get_collection_id_for_item(&self, item_id: &str) -> Option<String> {
        let collections = self.collections.load();

        for (coll_id, collection) in collections.iter() {
            if collection.get_item(item_id).is_some() {
                return Some(coll_id.clone());
            }
        }

        None
    }

    pub fn start_background_scan(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                info!("Starting background collection scan");
                if let Err(e) = self.scan_all().await {
                    error!("Background scan failed: {}", e);
                }
            }
        });
    }
    pub fn get_item(&self, id: &str) -> Option<(String, FoundItem)> {
        let collections = self.collections.load();

        for collection in collections.values() {
            if let Some(item_ref) = collection.get_item(id) {
                let item = match item_ref {
                    super::collection::ItemRef::Movie(m) => FoundItem::Movie(m.clone()),
                    super::collection::ItemRef::Show(s) => FoundItem::Show(s.clone()),
                    super::collection::ItemRef::Season(s) => FoundItem::Season(s.clone()),
                    super::collection::ItemRef::Episode(e) => FoundItem::Episode(e.clone()),
                };
                return Some((collection.id.clone(), item));
            }
        }

        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CollectionRepoError {
    #[error("Invalid collection type: {0}")]
    InvalidCollectionType(String),
    #[error("Scan error: {0}")]
    Scan(#[from] ScanError),
    #[error("Search error: {0}")]
    Search(String),
}

#[derive(Debug, Clone)]
pub enum FoundItem {
    Movie(crate::collection::Movie),
    Show(crate::collection::Show),
    Season(crate::collection::Season),
    Episode(crate::collection::Episode),
}

impl Item for FoundItem {
    fn id(&self) -> &str {
        match self {
            FoundItem::Movie(m) => m.id(),
            FoundItem::Show(s) => s.id(),
            FoundItem::Season(s) => s.id(),
            FoundItem::Episode(e) => e.id(),
        }
    }

    fn name(&self) -> &str {
        match self {
            FoundItem::Movie(m) => m.name(),
            FoundItem::Show(s) => s.name(),
            FoundItem::Season(s) => s.name(),
            FoundItem::Episode(e) => e.name(),
        }
    }

    fn collection_id(&self) -> &str {
        match self {
            FoundItem::Movie(m) => m.collection_id(),
            FoundItem::Show(s) => s.collection_id(),
            FoundItem::Season(s) => s.collection_id(),
            FoundItem::Episode(e) => e.collection_id(),
        }
    }

    fn item_type(&self) -> ItemType {
        match self {
            FoundItem::Movie(m) => m.item_type(),
            FoundItem::Show(s) => s.item_type(),
            FoundItem::Season(s) => s.item_type(),
            FoundItem::Episode(e) => e.item_type(),
        }
    }

    fn parent_id(&self) -> Option<&str> {
        match self {
            FoundItem::Movie(m) => m.parent_id(),
            FoundItem::Show(s) => s.parent_id(),
            FoundItem::Season(s) => s.parent_id(),
            FoundItem::Episode(e) => e.parent_id(),
        }
    }

    fn sort_name(&self) -> &str {
        match self {
            FoundItem::Movie(m) => m.sort_name(),
            FoundItem::Show(s) => s.sort_name(),
            FoundItem::Season(s) => s.sort_name(),
            FoundItem::Episode(e) => e.sort_name(),
        }
    }

    fn premiere_date(&self) -> Option<DateTime<Utc>> {
        match self {
            FoundItem::Movie(m) => m.premiere_date(),
            FoundItem::Show(s) => s.premiere_date(),
            FoundItem::Season(s) => s.premiere_date(),
            FoundItem::Episode(e) => e.premiere_date(),
        }
    }

    fn production_year(&self) -> Option<i32> {
        match self {
            FoundItem::Movie(m) => m.production_year(),
            FoundItem::Show(s) => s.production_year(),
            FoundItem::Season(s) => s.production_year(),
            FoundItem::Episode(e) => e.production_year(),
        }
    }

    fn community_rating(&self) -> Option<f64> {
        match self {
            FoundItem::Movie(m) => m.community_rating(),
            FoundItem::Show(s) => s.community_rating(),
            FoundItem::Season(s) => s.community_rating(),
            FoundItem::Episode(e) => e.community_rating(),
        }
    }

    fn overview(&self) -> Option<&str> {
        match self {
            FoundItem::Movie(m) => m.overview(),
            FoundItem::Show(s) => s.overview(),
            FoundItem::Season(s) => s.overview(),
            FoundItem::Episode(e) => e.overview(),
        }
    }

    fn genres(&self) -> &[String] {
        match self {
            FoundItem::Movie(m) => m.genres(),
            FoundItem::Show(s) => s.genres(),
            FoundItem::Season(s) => s.genres(),
            FoundItem::Episode(e) => e.genres(),
        }
    }

    fn images(&self) -> &ImageInfo {
        match self {
            FoundItem::Movie(m) => m.images(),
            FoundItem::Show(s) => s.images(),
            FoundItem::Season(s) => s.images(),
            FoundItem::Episode(e) => e.images(),
        }
    }
}
