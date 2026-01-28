use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
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
        let options = SqliteConnectOptions::from_str(db_path)?.create_if_missing(true);

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

    async fn flush_token_cache(&self) -> DbResult<()> {
        let cache = self.token_cache.read().await;
        for token in cache.values() {
            sqlx::query(
                "INSERT OR REPLACE INTO accesstokens 
                (token, userid, deviceid, devicename, applicationname, applicationversion, created)
                VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&token.token)
            .bind(&token.userid)
            .bind(&token.deviceid)
            .bind(&token.devicename)
            .bind(&token.applicationname)
            .bind(&token.applicationversion)
            .bind(token.created.as_ref().map(|dt| dt.to_rfc3339()))
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl UserRepo for SqliteRepository {
    async fn get_user(&self, username: &str) -> DbResult<User> {
        sqlx::query_as::<_, User>("SELECT id, username, password, created, lastlogin, lastused FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => DbError::NotFound(format!("User not found: {}", username)),
                _ => DbError::Sqlx(e),
            })
    }

    async fn get_user_by_id(&self, id: &str) -> DbResult<User> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, password, created, lastlogin, lastused FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound(format!("User not found: {}", id)),
            _ => DbError::Sqlx(e),
        })
    }

    async fn upsert_user(&self, user: &User) -> DbResult<()> {
        sqlx::query("INSERT OR REPLACE INTO users (id, username, password, created, lastlogin, lastused) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.password)
            .bind(&user.created)
            .bind(&user.lastlogin)
            .bind(&user.lastused)
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

        let result = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT token, userid, deviceid, devicename, applicationname, applicationversion, created 
             FROM accesstokens WHERE token = ?",
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
            userid: result.1,
            deviceid: result.2,
            devicename: result.3,
            applicationname: result.4,
            applicationversion: result.5,
            remoteaddress: None,
            created: result.6.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
            lastused: None,
        };

        let mut cache = self.token_cache.write().await;
        cache.insert(token.to_string(), access_token.clone());

        Ok(access_token)
    }

    async fn list_tokens_by_user(&self, user_id: &str) -> DbResult<Vec<AccessToken>> {
        let results = sqlx::query_as::<_, (String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>(
            "SELECT token, userid, deviceid, devicename, applicationname, applicationversion, created 
             FROM accesstokens WHERE userid = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let tokens = results
            .into_iter()
            .map(|r| AccessToken {
                token: r.0,
                userid: r.1,
                deviceid: r.2,
                devicename: r.3,
                applicationname: r.4,
                applicationversion: r.5,
                remoteaddress: None,
                created: r.6.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
                lastused: None,
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
        let result = sqlx::query_as::<_, (String, String, Option<i32>, Option<i32>, String, Option<f32>, i64, i64, i64)>(
            "SELECT id, name, votes, year, genre, rating, nfotime, firstvideo, lastvideo FROM items WHERE id = ?",
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
            name: result.1,
            votes: result.2,
            year: result.3,
            genre: result.4,
            rating: result.5,
            nfotime: result.6,
            firstvideo: result.7,
            lastvideo: result.8,
        })
    }

    async fn upsert_item(&self, item: &Item) -> DbResult<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO items 
            (id, name, votes, year, genre, rating, nfotime, firstvideo, lastvideo)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&item.id)
        .bind(&item.name)
        .bind(item.votes)
        .bind(item.year)
        .bind(&item.genre)
        .bind(item.rating)
        .bind(item.nfotime)
        .bind(item.firstvideo)
        .bind(item.lastvideo)
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

        let result = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<i64>,
                Option<i32>,
                Option<bool>,
                Option<i32>,
                Option<bool>,
                Option<String>,
            ),
        >(
            "SELECT userid, itemid, position, playedpercentage, played, playcount, favorite, timestamp 
             FROM playstate WHERE userid = ? AND itemid = ?",
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
            userid: result.0,
            itemid: result.1,
            position: result.2,
            playedpercentage: result.3,
            played: result.4,
            playcount: result.5,
            favorite: result.6,
            timestamp: result.7.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
        };

        let mut cache = self.userdata_cache.write().await;
        cache.insert(
            (user_id.to_string(), item_id.to_string()),
            user_data.clone(),
        );

        Ok(user_data)
    }

    async fn get_user_data_resume(
        &self,
        user_id: &str,
        limit: Option<u32>,
    ) -> DbResult<Vec<UserData>> {
        let mut query = "SELECT userid, itemid, position, playedpercentage, played, playcount, favorite, timestamp 
            FROM playstate
            WHERE userid = ?
            AND position > 0
            AND played != true
            ORDER BY position DESC"
            .to_string();
        if let Some(limit) = limit {
            let _ = write!(&mut query, " LIMIT {}", limit);
        }

        let results = sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<i64>,
                Option<i32>,
                Option<bool>,
                Option<i32>,
                Option<bool>,
                Option<String>,
            ),
        >(&query)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut user_data = Vec::new();
        for result in results {
            user_data.push(UserData {
                userid: result.0,
                itemid: result.1,
                position: result.2,
                playedpercentage: result.3,
                played: result.4,
                playcount: result.5,
                favorite: result.6,
                timestamp: result.7.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            });
        }

        if user_data.len() == 0 {
            return Err(DbError::NotFound(format!(
                "UserData not found: {}",
                user_id
            )));
        }
        Ok(user_data)
    }

    async fn upsert_user_data(&self, data: &UserData) -> DbResult<()> {
        let mut cache = self.userdata_cache.write().await;
        cache.insert((data.userid.clone(), data.itemid.clone()), data.clone());

        sqlx::query(
            "INSERT OR REPLACE INTO playstate 
            (userid, itemid, position, playcount, favorite, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&data.userid)
        .bind(&data.itemid)
        .bind(data.position)
        .bind(data.playcount)
        .bind(data.favorite)
        .bind(data.timestamp.as_ref().map(|dt| dt.to_rfc3339()))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_favorites(&self, user_id: &str) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT itemid FROM playstate WHERE userid = ? AND favorite = 1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    async fn get_recently_watched(&self, user_id: &str, limit: i32) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT itemid FROM playstate 
             WHERE userid = ? AND timestamp IS NOT NULL 
             ORDER BY timestamp DESC LIMIT ?",
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
        let result = sqlx::query_as::<_, (String, String, String, Option<String>)>(
            "SELECT id, name, userid, timestamp FROM playlist WHERE id = ?",
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
            name: result.1,
            userid: result.2,
            timestamp: result.3.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
        })
    }

    async fn list_playlists_by_user(&self, user_id: &str) -> DbResult<Vec<Playlist>> {
        let results = sqlx::query_as::<_, (String, String, String, Option<String>)>(
            "SELECT id, name, userid, timestamp FROM playlist WHERE userid = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let playlists = results
            .into_iter()
            .map(|r| Playlist {
                id: r.0,
                name: r.1,
                userid: r.2,
                timestamp: r.3.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|dt| dt.with_timezone(&Utc))
                }),
            })
            .collect();

        Ok(playlists)
    }

    async fn create_playlist(&self, playlist: &Playlist) -> DbResult<()> {
        sqlx::query("INSERT INTO playlist (id, name, userid, timestamp) VALUES (?, ?, ?, ?)")
            .bind(&playlist.id)
            .bind(&playlist.name)
            .bind(&playlist.userid)
            .bind(playlist.timestamp.as_ref().map(|dt| dt.to_rfc3339()))
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
            "SELECT MAX(itemorder) FROM playlist_item WHERE playlistid = ?",
        )
        .bind(playlist_id)
        .fetch_one(&self.pool)
        .await?
        .0
        .unwrap_or(-1);

        sqlx::query("INSERT INTO playlist_item (playlistid, itemid, itemorder, timestamp) VALUES (?, ?, ?, ?)")
            .bind(playlist_id)
            .bind(item_id)
            .bind(max_order + 1)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_item_from_playlist(&self, playlist_id: &str, item_id: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM playlist_item WHERE playlistid = ? AND itemid = ?")
            .bind(playlist_id)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_playlist_items(&self, playlist_id: &str) -> DbResult<Vec<String>> {
        let results = sqlx::query_as::<_, (String,)>(
            "SELECT itemid FROM playlist_item WHERE playlistid = ? ORDER BY itemorder",
        )
        .bind(playlist_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(|r| r.0).collect())
    }

    async fn move_item_in_playlist(
        &self,
        playlist_id: &str,
        item_id: &str,
        new_index: i32,
    ) -> DbResult<()> {
        sqlx::query("UPDATE playlist_item SET itemorder = ? WHERE playlistid = ? AND itemid = ?")
            .bind(new_index)
            .bind(playlist_id)
            .bind(item_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

impl Repository for SqliteRepository {
    fn close(&self) {}
}
