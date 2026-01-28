use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub listen: ListenConfig,
    #[serde(default)]
    pub appdir: Option<String>,
    #[serde(default)]
    pub cachedir: Option<String>,
    #[serde(default)]
    pub dbdir: Option<String>,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default = "default_logfile")]
    pub logfile: String,
    #[serde(default)]
    pub collections: Vec<CollectionConfig>,
    #[serde(default)]
    pub jellyfin: JellyfinConfig,
    #[serde(skip)]
    pub debug_logs: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenConfig {
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default = "default_port")]
    pub port: String,
    #[serde(default)]
    pub tlscert: Option<String>,
    #[serde(default)]
    pub tlskey: Option<String>,
}

impl Default for ListenConfig {
    fn default() -> Self {
        Self {
            address: None,
            port: default_port(),
            tlscert: None,
            tlskey: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub sqlite: Option<SqliteConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqliteConfig {
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CollectionConfig {
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(rename = "type")]
    pub collection_type: String,
    pub directory: String,
    #[serde(default)]
    pub baseurl: Option<String>,
    #[serde(default)]
    pub hlsserver: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JellyfinConfig {
    #[serde(alias = "serverid", rename = "serverId")]
    #[serde(default)]
    pub server_id: Option<String>,
    #[serde(alias = "servername", rename = "servername")]
    #[serde(default = "default_server_name")]
    pub server_name: String,
    #[serde(default)]
    pub autoregister: bool,
    #[serde(alias = "imagequalityposter", rename = "imagequalityposter")]
    #[serde(default)]
    pub image_quality_poster: Option<u32>,
}

impl Default for JellyfinConfig {
    fn default() -> Self {
        Self {
            server_id: None,
            server_name: default_server_name(),
            autoregister: false,
            image_quality_poster: None,
        }
    }
}

fn default_port() -> String {
    "8096".to_string()
}

fn default_logfile() -> String {
    "stdout".to_string()
}

fn default_server_name() -> String {
    "Jellofin".to_string()
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::ReadError(path.to_string(), e))?;

        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(path.to_string(), e))?;

        Ok(config)
    }

    pub fn get_database_path(&self) -> Option<String> {
        if let Some(ref sqlite) = self.database.sqlite {
            return Some(sqlite.filename.clone());
        }

        if let Some(ref dbdir) = self.dbdir {
            let path = PathBuf::from(dbdir).join("tink-items.db");
            return Some(path.to_string_lossy().to_string());
        }

        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file {0}: {1}")]
    ReadError(String, std::io::Error),
    #[error("Failed to parse config file {0}: {1}")]
    ParseError(String, serde_yaml::Error),
}
