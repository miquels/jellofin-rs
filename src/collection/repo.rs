use std::collections::HashMap;
use std::sync::Arc;
use arc_swap::ArcSwap;
use tracing::{error, info};

use crate::config::CollectionConfig;
use super::collection::{Collection, CollectionType};
use super::scanner::{scan_collection, ScanError};
use super::search::{SearchIndex, SearchResult};

pub struct CollectionRepo {
    collections: Arc<ArcSwap<HashMap<String, Collection>>>,
    search_index: Arc<SearchIndex>,
}

impl CollectionRepo {
    pub fn new() -> Result<Self, CollectionRepoError> {
        let search_index = SearchIndex::new()
            .map_err(|e| CollectionRepoError::Search(e.to_string()))?;
        
        Ok(Self {
            collections: Arc::new(ArcSwap::from_pointee(HashMap::new())),
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
                }).await;
                
                // Atomically update the collection after scan
                match scan_result {
                    Ok((scanned_collection, Ok(()))) => {
                        let mut new_collections = (**self.collections.load()).clone();
                        new_collections.insert(id.clone(), scanned_collection);
                        self.collections.store(Arc::new(new_collections));
                    },
                    Ok((scanned_collection, Err(e))) => {
                        error!("Failed to scan collection {}: {}", id, e);
                        // Still update with scanned collection even if there was an error
                        let mut new_collections = (**self.collections.load()).clone();
                        new_collections.insert(id.clone(), scanned_collection);
                        self.collections.store(Arc::new(new_collections));
                    },
                    Err(e) => {
                        error!("Scan task panicked for collection {}: {}", id, e);
                    },
                }
            }
        }
        
        info!("Rebuilding search index");
        let collections = self.collections.load();
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
