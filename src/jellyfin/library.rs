use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualFolderInfo {
    pub name: String,
    pub locations: Vec<String>,
    pub collection_type: Option<String>,
    pub library_options: Option<LibraryOptions>,
    pub item_id: String,
    pub primary_image_item_id: String,
    pub refresh_progress: Option<f64>,
    pub refresh_status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryOptions {
    pub enabled: bool,
}

pub async fn get_virtual_folders(
    State(state): State<AppState>,
) -> Result<Json<Vec<VirtualFolderInfo>>, StatusCode> {
    let mut folders = Vec::new();

    // Map existing collections to VirtualFolders
    for collection in state.collections.list_collections().await {
         folders.push(VirtualFolderInfo {
             name: collection.name.clone(),
             locations: vec![collection.directory.to_string_lossy().to_string()], // Assuming single location for now
             collection_type: Some(format!("{:?}", collection.collection_type)),
             library_options: Some(LibraryOptions { enabled: true }),
             item_id: collection.id.clone(),
             primary_image_item_id: collection.id.clone(),
             refresh_progress: None,
             refresh_status: None,
         });
    }

    Ok(Json(folders))
}
