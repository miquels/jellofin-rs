use axum::{
    extract::Request,
    http::uri::Uri,
    middleware::Next,
    response::Response,
};
use tracing::info;

pub async fn normalize_path(mut req: Request, next: Next) -> Response {
    let uri = req.uri();
    let path = uri.path();
    
    let mut normalized = path.to_string();
    
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }
    
    if normalized.starts_with("/emby/") {
        normalized = normalized.strip_prefix("/emby").unwrap_or(&normalized).to_string();
    }
    
    if normalized != path {
        let mut parts = uri.clone().into_parts();
        let new_path_and_query = if let Some(query) = uri.query() {
            format!("{}?{}", normalized, query)
        } else {
            normalized
        };
        
        if let Ok(new_uri) = new_path_and_query.parse::<Uri>() {
            parts.path_and_query = new_uri.into_parts().path_and_query;
            if let Ok(new_uri) = Uri::from_parts(parts) {
                *req.uri_mut() = new_uri;
            }
        }
    }
    
    next.run(req).await
}

pub async fn log_request(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    
    info!("{} {}", method, uri);
    
    next.run(req).await
}
