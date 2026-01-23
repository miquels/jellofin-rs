use async_trait::async_trait;

use super::model::*;

#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn get_user(&self, username: &str) -> DbResult<User>;
    async fn get_user_by_id(&self, id: &str) -> DbResult<User>;
    async fn upsert_user(&self, user: &User) -> DbResult<()>;
}

#[async_trait]
pub trait AccessTokenRepo: Send + Sync {
    async fn get_token(&self, token: &str) -> DbResult<AccessToken>;
    async fn list_tokens_by_user(&self, user_id: &str) -> DbResult<Vec<AccessToken>>;
    async fn upsert_token(&self, token: &AccessToken) -> DbResult<()>;
    async fn delete_token(&self, token: &str) -> DbResult<()>;
}

#[async_trait]
pub trait ItemRepo: Send + Sync {
    async fn get_item(&self, id: &str) -> DbResult<Item>;
    async fn upsert_item(&self, item: &Item) -> DbResult<()>;
    async fn delete_item(&self, id: &str) -> DbResult<()>;
}

#[async_trait]
pub trait UserDataRepo: Send + Sync {
    async fn get_user_data(&self, user_id: &str, item_id: &str) -> DbResult<UserData>;
    async fn upsert_user_data(&self, data: &UserData) -> DbResult<()>;
    async fn get_favorites(&self, user_id: &str) -> DbResult<Vec<String>>;
    async fn get_recently_watched(&self, user_id: &str, limit: i32) -> DbResult<Vec<String>>;
}

#[async_trait]
pub trait PlaylistRepo: Send + Sync {
    async fn get_playlist(&self, id: &str) -> DbResult<Playlist>;
    async fn list_playlists_by_user(&self, user_id: &str) -> DbResult<Vec<Playlist>>;
    async fn create_playlist(&self, playlist: &Playlist) -> DbResult<()>;
    async fn delete_playlist(&self, id: &str) -> DbResult<()>;
    async fn add_item_to_playlist(&self, playlist_id: &str, item_id: &str) -> DbResult<()>;
    async fn remove_item_from_playlist(&self, playlist_id: &str, item_id: &str) -> DbResult<()>;
    async fn get_playlist_items(&self, playlist_id: &str) -> DbResult<Vec<String>>;
    async fn move_item_in_playlist(&self, playlist_id: &str, item_id: &str, new_index: i32) -> DbResult<()>;
}

pub trait Repository: UserRepo + AccessTokenRepo + ItemRepo + UserDataRepo + PlaylistRepo + Send + Sync {
    fn close(&self);
}
