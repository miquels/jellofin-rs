use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use super::model::*;
use super::repo::*;

pub struct SqliteRepository {
    pool: SqlitePool,
    token_cache: Arc<RwLock<HashMap<String, AccessToken>>>,
    userdata_cache: Arc<RwLock<HashMap<(String, String), UserData>>>,
}

impl SqliteRepository {
    pub async fn new(db_path: &str) -> DbResult<Self> {
        let options = SqliteConnectOptions::from_str(db_path)?
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let repo = Self {
            pool,
            token_cache: Arc::new(RwLock::new(HashMap::new())),
            userdata_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        repo.init_schema().await?;
        
        info!("Database initialized at {}", db_path);
        
        Ok(repo)
    }

    async fn init_schema(&self) -> DbResult<()> {
        let schema = include_str!("schema.sql");
        sqlx::query(schema).execute(&self.pool).await?;
        Ok(())
    }

    pub fn start_background_tasks(self: Arc<Self>) {
        let repo_clone = Arc::clone(&self);
        tokio::spawn(async move {
            repo_clone.token_flush_loop().await;
        });

        let repo_clone = Arc::clone(&self);
        tokio::spawn(async move {
            repo_clone.userdata_flush_loop().await;
        });
    }

    async fn token_flush_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = self.flush_token_cache().await {
                error!("Failed to flush token cache: {}", e);
            }
        }
    }

    async fn userdata_flush_loop(&self) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
            interval.tick().await;
            if let Err(e) = self.flush_userdata_cache().await {
                error!("Failed to flush userdata cache: {}", e);
            }
        }
    }

    async fn flush_token_cache(&self) -> DbResult<()> {
        let cache = self.token_cache.read().await;
        for token in cache.values() {
            sqlx::query(
                "INSERT OR REPLACE INTO accesstokens 
                (token, user_id, device_id, device_name, app_name, app_version, date_created)
                VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&token.token)
            .bind(&token.user_id)
            .bind(&token.device_id)
            .bind(&token.device_name)
            .bind(&token.app_name)
            .bind(&token.app_version)
            .bind(token.date_created.to_rfc3339())
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn flush_userdata_cache(&self) -> DbResult<()> {
        let cache = self.userdata_cache.read().await;
        for ((user_id, item_id), data) in cache.iter() {
            sqlx::query(
                "INSERT OR REPLACE INTO playstate 
                (user_id, item_id, playback_position_ticks, play_count, is_favorite, last_played_date)
                VALUES (?, ?, ?, ?, ?, NULL)"
            )
            .bind(user_id)
            .bind(item_id)
            .bind(data.playback_position_ticks.unwrap_or(0))
            .bind(data.play_count.unwrap_or(0))
            .bind(if data.is_favorite { 1 } else { 0 })
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl UserRepo for SqliteRepository {
    async fn get_user(&self, username: &str) -> DbResult<User> {
        sqlx::query_as::<_, User>("SELECT id, username, password FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => DbError::NotFound(format!("User not found: {}", username)),
                _ => DbError::Sqlx(e),
            })
    }

    async fn get_user_by_id(&self, id: &str) -> DbResult<User> {
        sqlx::query_as::<_, User>("SELECT id, username, password FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => DbError::NotFound(format!("User not found: {}", id)),
                _ => DbError::Sqlx(e),
            })
    }

    async fn upsert_user(&self, user: &User) -> DbResult<()> {
        sqlx::query("INSERT OR REPLACE INTO users (id, username, password) VALUES (?, ?, ?)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.password)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl AccessTokenRepo for SqliteRepository {
    async fn get_token(&self, token: &str) -> DbResult<AccessToken> {
        {
            let cache = self.token_cache.read().await;
            if let Some(t) = cache.get(token) {
                return Ok(t.clone());
            }
        }

        let result = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
            "SELECT token, user_id, device_id, device_name, app_name, app_version, date_created 
             FROM accesstokens WHERE token = ?"
        )
        .bind(token)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound(format!("Token not found: {}", token)),
            _ => DbError::Sqlx(e),
        })?;

        let access_token = AccessToken {
            token: result.0,
            user_id: result.1,
            device_id: result.2,
            device_name: result.3,
            app_name: result.4,
            app_version: result.5,
            date_created: DateTime::parse_from_rfc3339(&result.6)
                .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&Utc),
        };

        let mut cache = self.token_cache.write().await;
        cache.insert(token.to_string(), access_token.clone());

        Ok(access_token)
    }

    async fn list_tokens_by_user(&self, user_id: &str) -> DbResult<Vec<AccessToken>> {
        let results = sqlx::query_as::<_, (String, String, String, String, String, String, String)>(
            "SELECT token, user_id, device_id, device_name, app_name, app_version, date_created 
             FROM accesstokens WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let tokens = results
            .into_iter()
            .filter_map(|r| {
                DateTime::parse_from_rfc3339(&r.6).ok().map(|dt| AccessToken {
                    token: r.0,
                    user_id: r.1,
                    device_id: r.2,
                    device_name: r.3,
                    app_name: r.4,
                    app_version: r.5,
                    date_created: dt.with_timezone(&Utc),
                })
            })
            .collect();

        Ok(tokens)
    }

    async fn upsert_token(&self, token: &AccessToken) -> DbResult<()> {
        let mut cache = self.token_cache.write().await;
        cache.insert(token.token.clone(), token.clone());
        Ok(())
    }

    async fn delete_token(&self, token: &str) -> DbResult<()> {
        {
            let mut cache = self.token_cache.write().await;
            cache.remove(token);
        }

        sqlx::query("DELETE FROM accesstokens WHERE token = ?")
            .bind(token)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl ItemRepo for SqliteRepository {
    async fn get_item(&self, id: &str) -> DbResult<Item> {
        let result = sqlx::query_as::<_, (String, Option<String>, String, String, Option<String>, Option<String>, 
            Option<String>, Option<f64>, Option<i64>, Option<i32>, Option<i32>, Option<i32>, String, String, String)>(
            "SELECT id, parent_id, collection_id, name, sort_name, original_title, premiere_date, 
             community_rating, runtime_ticks, production_year, index_number, parent_index_number, 
             item_type, date_created, date_modified FROM items WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound(format!("Item not found: {}", id)),
            _ => DbError::Sqlx(e),
        })?;

        Ok(Item {
            id: result.0,
            parent_id: result.1,
            collection_id: result.2,
            name: result.3,
            sort_name: result.4,
            original_title: result.5,
            premiere_date: result.6.and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.with_timezone(&Utc))),
            community_rating: result.7,
            runtime_ticks: result.8,
            production_year: result.9,
            index_number: result.10,
            parent_index_number: result.11,
            item_type: result.12,
            date_created: DateTime::parse_from_rfc3339(&result.13)
                .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&Utc),
            date_modified: DateTime::parse_from_rfc3339(&result.14)
                .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&Utc),
        })
    }

    async fn upsert_item(&self, item: &Item) -> DbResult<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO items 
            (id, parent_id, collection_id, name, sort_name, original_title, premiere_date, 
             community_rating, runtime_ticks, production_year, index_number, parent_index_number, 
             item_type, date_created, date_modified)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&item.id)
        .bind(&item.parent_id)
        .bind(&item.collection_id)
        .bind(&item.name)
        .bind(&item.sort_name)
        .bind(&item.original_title)
        .bind(item.premiere_date.map(|d| d.to_rfc3339()))
        .bind(item.community_rating)
        .bind(item.runtime_ticks)
        .bind(item.production_year)
        .bind(item.index_number)
        .bind(item.parent_index_number)
        .bind(&item.item_type)
        .bind(item.date_created.to_rfc3339())
        .bind(item.date_modified.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_item(&self, id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM items WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl UserDataRepo for SqliteRepository {
    async fn get_user_data(&self, user_id: &str, item_id: &str) -> DbResult<UserData> {
        {
            let cache = self.userdata_cache.read().await;
            if let Some(data) = cache.get(&(user_id.to_string(), item_id.to_string())) {
                return Ok(data.clone());
            }
        }

        let result = sqlx::query_as::<_, (String, String, i64, i32, i32, Option<String>)>(
            "SELECT user_id, item_id, playback_position_ticks, play_count, is_favorite, last_played_date 
             FROM playstate WHERE user_id = ? AND item_id = ?"
        )
        .bind(user_id)
        .bind(item_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound(format!("UserData not found: {}/{}", user_id, item_id)),
            _ => DbError::Sqlx(e),
        })?;

        let user_data = UserData {
            user_id: result.0,
            item_id: result.1,
            played: result.4 != 0,
            is_favorite: result.4 != 0,
            playback_position_ticks: Some(result.2),
            play_count: Some(result.3),
        };

        let mut cache = self.userdata_cache.write().await;
        cache.insert((user_id.to_string(), item_id.to_string()), user_data.clone());

        Ok(user_data)
    }

    async fn upsert_user_data(&self, data: &UserData) -> DbResult<()> {
        let mut cache = self.userdata_cache.write().await;
        cache.insert((data.user_id.clone(), data.item_id.clone()), data.clone());
        Ok(())
    }

    async fn get_favorites(&self, user_id: &str) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT item_id FROM playstate WHERE user_id = ? AND is_favorite = 1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    async fn get_recently_watched(&self, user_id: &str, limit: i32) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT item_id FROM playstate 
             WHERE user_id = ? AND last_played_date IS NOT NULL 
             ORDER BY last_played_date DESC LIMIT ?"
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }
}

#[async_trait]
impl PlaylistRepo for SqliteRepository {
    async fn get_playlist(&self, id: &str) -> DbResult<Playlist> {
        let result = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, user_id, name, date_created, date_modified FROM playlist WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound(format!("Playlist not found: {}", id)),
            _ => DbError::Sqlx(e),
        })?;

        Ok(Playlist {
            id: result.0,
            user_id: result.1,
            name: result.2,
            date_created: DateTime::parse_from_rfc3339(&result.3)
                .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&Utc),
            date_modified: DateTime::parse_from_rfc3339(&result.4)
                .map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))?
                .with_timezone(&Utc),
        })
    }

    async fn list_playlists_by_user(&self, user_id: &str) -> DbResult<Vec<Playlist>> {
        let results = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, user_id, name, date_created, date_modified FROM playlist WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let playlists = results
            .into_iter()
            .filter_map(|r| {
                let date_created = DateTime::parse_from_rfc3339(&r.3).ok()?.with_timezone(&Utc);
                let date_modified = DateTime::parse_from_rfc3339(&r.4).ok()?.with_timezone(&Utc);
                Some(Playlist {
                    id: r.0,
                    user_id: r.1,
                    name: r.2,
                    date_created,
                    date_modified,
                })
            })
            .collect();

        Ok(playlists)
    }

    async fn create_playlist(&self, playlist: &Playlist) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO playlist (id, user_id, name, date_created, date_modified) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&playlist.id)
        .bind(&playlist.user_id)
        .bind(&playlist.name)
        .bind(playlist.date_created.to_rfc3339())
        .bind(playlist.date_modified.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_playlist(&self, id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM playlist WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn add_item_to_playlist(&self, playlist_id: &str, item_id: &str) -> DbResult<()> {
        let max_order = sqlx::query_as::<_, (Option<i32>,)>(
            "SELECT MAX(sort_order) FROM playlist_item WHERE playlist_id = ?"
        )
        .bind(playlist_id)
        .fetch_one(&self.pool)
        .await?
        .0
        .unwrap_or(-1);

        sqlx::query(
            "INSERT INTO playlist_item (playlist_id, item_id, sort_order) VALUES (?, ?, ?)"
        )
        .bind(playlist_id)
        .bind(item_id)
        .bind(max_order + 1)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_item_from_playlist(&self, playlist_id: &str, item_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM playlist_item WHERE playlist_id = ? AND item_id = ?")
            .bind(playlist_id)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_playlist_items(&self, playlist_id: &str) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT item_id FROM playlist_item WHERE playlist_id = ? ORDER BY sort_order"
        )
        .bind(playlist_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    async fn move_item_in_playlist(&self, playlist_id: &str, item_id: &str, new_index: i32) -> DbResult<()> {
        sqlx::query(
            "UPDATE playlist_item SET sort_order = ? WHERE playlist_id = ? AND item_id = ?"
        )
        .bind(new_index)
        .bind(playlist_id)
        .bind(item_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

impl Repository for SqliteRepository {
    fn close(&self) {
    }
}
