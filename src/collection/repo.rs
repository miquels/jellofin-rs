use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::config::CollectionConfig;
use super::collection::{Collection, CollectionType};
use super::scanner::{scan_collection, ScanError};
use super::search::{SearchIndex, SearchResult};

pub struct CollectionRepo {
    collections: Arc<RwLock<HashMap<String, Collection>>>,
    search_index: Arc<SearchIndex>,
}

impl CollectionRepo {
    pub fn new() -> Result<Self, CollectionRepoError> {
        let search_index = SearchIndex::new()
            .map_err(|e| CollectionRepoError::Search(e.to_string()))?;
        
        Ok(Self {
            collections: Arc::new(RwLock::new(HashMap::new())),
            search_index: Arc::new(search_index),
        })
    }

    pub async fn add_collection(&self, config: &CollectionConfig) -> Result<(), CollectionRepoError> {
        let collection_type = CollectionType::from_str(&config.collection_type)
            .ok_or_else(|| CollectionRepoError::InvalidCollectionType(config.collection_type.clone()))?;

        let id = config.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let collection = Collection::new(
            id.clone(),
            config.name.clone(),
            collection_type,
            config.directory.clone().into(),
            config.baseurl.clone(),
            config.hlsserver.clone(),
        );

        let mut collections = self.collections.write().await;
        collections.insert(id.clone(), collection);

        info!("Added collection: {} ({})", config.name, id);
        Ok(())
    }

    pub async fn scan_all(&self) -> Result<(), CollectionRepoError> {
        let mut collections = self.collections.write().await;
        
        for (id, collection) in collections.iter_mut() {
            info!("Scanning collection: {}", collection.name);
            if let Err(e) = scan_collection(collection) {
                error!("Failed to scan collection {}: {}", id, e);
            }
        }
        
        info!("Rebuilding search index");
        self.search_index.rebuild(&collections).await
            .map_err(|e| CollectionRepoError::Search(e.to_string()))?;

        Ok(())
    }
    
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, CollectionRepoError> {
        self.search_index.search(query, limit)
            .map_err(|e| CollectionRepoError::Search(e.to_string()))
    }
    
    pub fn find_similar(&self, item_id: &str, limit: usize) -> Result<Vec<SearchResult>, CollectionRepoError> {
        self.search_index.find_similar(item_id, limit)
            .map_err(|e| CollectionRepoError::Search(e.to_string()))
    }

    pub async fn get_collection(&self, id: &str) -> Option<Collection> {
        let collections = self.collections.read().await;
        collections.get(id).cloned()
    }

    pub async fn list_collections(&self) -> Vec<Collection> {
        let collections = self.collections.read().await;
        collections.values().cloned().collect()
    }

    pub async fn get_collection_id_for_item(&self, item_id: &str) -> Option<String> {
        let collections = self.collections.read().await;
        
        for (coll_id, collection) in collections.iter() {
            if collection.get_item(item_id).is_some() {
                return Some(coll_id.clone());
            }
        }
        
        None
    }

    pub fn start_background_scan(self: Arc<Self>, interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                info!("Starting background collection scan");
                if let Err(e) = self.scan_all().await {
                    error!("Background scan failed: {}", e);
                }
            }
        });
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
