pub mod collection;
pub mod config;
pub mod db;
pub mod jellyfin;
pub mod middleware;
pub mod notflix;
pub mod server;
pub mod util;

use std::net::SocketAddr;
use std::sync::Arc;
use std::io::BufReader;
use std::fs::File;
use tracing::info;

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Database error: {0}")]
    Database(#[from] db::DbError),
    #[error("Server error: {0}")]
    Server(String),
}

pub async fn run(config_path: &str, debug_logs: bool) -> Result<(), ServerError> {
    let mut config = config::Config::from_file(config_path)?;
    config.debug_logs = debug_logs;
    
    info!("Using config file: {}", config_path);
    info!("Server name: {}", config.jellyfin.server_name);
    if debug_logs {
        info!("Debug logging enabled");
    }
    
    let db_path = config.get_database_path()
        .ok_or_else(|| ServerError::Server("No database path configured".to_string()))?;
    
    info!("Opening database at {}", db_path);
    let db = Arc::new(db::SqliteRepository::new(&db_path).await?);
    
    db.clone().start_background_tasks();
    
    let collection_repo = Arc::new(collection::CollectionRepo::new()
        .map_err(|e| ServerError::Server(format!("Failed to create collection repo: {}", e)))?);
    
    for coll_config in &config.collections {
        collection_repo.add_collection(coll_config).await
            .map_err(|e| ServerError::Server(format!("Failed to add collection: {}", e)))?;
    }
    
    info!("Performing initial collection scan...");
    collection_repo.scan_all().await
        .map_err(|e| ServerError::Server(format!("Failed to scan collections: {}", e)))?;
    
    collection_repo.clone().start_background_scan(3600);
    
    let cache_dir = std::path::PathBuf::from("./cache/images");
    let image_resizer = Arc::new(util::ImageResizer::new(cache_dir)
        .map_err(|e| ServerError::Server(format!("Failed to create image resizer: {}", e)))?);
    
    let address = config.listen.address.as_deref().unwrap_or("[::]");
    let port = &config.listen.port;
    let addr: SocketAddr = format!("{}:{}", address, port)
        .parse()
        .map_err(|e| ServerError::Server(format!("Invalid address: {}", e)))?;
    
    let has_tls = config.listen.tlscert.is_some() && config.listen.tlskey.is_some();
    
    let state = server::AppState::new(config.clone(), db, collection_repo, image_resizer);
    let app = server::build_router(state);
    
    if has_tls {
        let cert_path = config.listen.tlscert.as_ref().unwrap();
        let key_path = config.listen.tlskey.as_ref().unwrap();
        
        info!("Loading TLS certificate from {}", cert_path);
        info!("Loading TLS key from {}", key_path);
        
        let tls_config = {
            let certs = load_certs(cert_path).map_err(|e| ServerError::Server(format!("Failed to load certs: {}", e)))?;
            let key = load_private_key(key_path).map_err(|e| ServerError::Server(format!("Failed to load key: {}", e)))?;

            let mut config = rustls::ServerConfig::builder()
                .with_no_client_auth()
                .with_single_cert(certs, key)
                .map_err(|e| ServerError::Server(format!("Failed to build TLS config: {}", e)))?;
            
            // CRITICAL: Force HTTP/1.1 ONLY via ALPN to prevent HTTP/2 usage
            config.alpn_protocols = vec![b"http/1.1".to_vec()];

            axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(config))
        };
        
        info!("Serving HTTPS on {}", addr);
        
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .map_err(|e| ServerError::Server(format!("Server error: {}", e)))?;
    } else {
        info!("Serving HTTP on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::Server(format!("Failed to bind: {}", e)))?;
        
        axum::serve(listener, app)
            .await
            .map_err(|e| ServerError::Server(format!("Server error: {}", e)))?;
    }
    
    Ok(())
}

fn load_certs(path: &str) -> std::io::Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    rustls_pemfile::certs(&mut reader).collect()
}

fn load_private_key(path: &str) -> std::io::Result<rustls::pki_types::PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    loop {
        match rustls_pemfile::read_one(&mut reader)? {
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => return Ok(key.into()),
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => return Ok(key.into()),
            Some(rustls_pemfile::Item::Sec1Key(key)) => return Ok(key.into()),
            None => break,
            _ => {}
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::Other, "no private key found"))
}
