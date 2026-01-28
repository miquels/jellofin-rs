use crate::collection::{repo::FoundItem, CollectionRepo};
use std::path::PathBuf;

pub fn find_image_path(
    collections: &CollectionRepo,
    item_id: &str,
    image_type: &str,
) -> Option<PathBuf> {
    if let Some((_, item)) = collections.get_item(item_id) {
        match item {
            FoundItem::Movie(movie) => match image_type.to_lowercase().as_str() {
                "primary" => movie.images.primary.clone(),
                "backdrop" => movie.images.backdrop.clone(),
                "logo" => movie.images.logo.clone(),
                "thumb" => movie.images.thumb.clone(),
                "banner" => movie.images.banner.clone(),
                _ => None,
            },
            FoundItem::Show(show) => match image_type.to_lowercase().as_str() {
                "primary" => show.images.primary.clone(),
                "backdrop" => show.images.backdrop.clone(),
                "logo" => show.images.logo.clone(),
                "thumb" => show.images.thumb.clone(),
                "banner" => show.images.banner.clone(),
                _ => None,
            },
            FoundItem::Season(season) => match image_type.to_lowercase().as_str() {
                "primary" => season.images.primary.clone(),
                "backdrop" => season.images.backdrop.clone(),
                "logo" => season.images.logo.clone(),
                "thumb" => season.images.thumb.clone(),
                "banner" => season.images.banner.clone(),
                _ => None,
            },
            FoundItem::Episode(episode) => match image_type.to_lowercase().as_str() {
                // For episodes, fall back to thumb if primary is None
                // (episode thumbnails are often named with -thumb suffix)
                "primary" => episode
                    .images
                    .primary
                    .clone()
                    .or_else(|| episode.images.thumb.clone()),
                "backdrop" => episode.images.backdrop.clone(),
                "logo" => episode.images.logo.clone(),
                "thumb" => episode.images.thumb.clone(),
                "banner" => episode.images.banner.clone(),
                _ => None,
            },
        }
    } else {
        None
    }
}
