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
    
    let response = next.run(req).await;
    
    let status = response.status().as_u16();
    let content_length = response.headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    
    info!(
        method = %method,
        url = %uri,
        status = status,
        length = content_length,
        "HTTP request"
    );
    
    response
}

pub async fn add_cors_headers(req: Request, next: Next) -> Response {
    // Handle OPTIONS requests for CORS preflight
    if req.method() == axum::http::Method::OPTIONS {
        let mut response = Response::new(axum::body::Body::empty());
        *response.status_mut() = axum::http::StatusCode::OK;
        
        let headers = response.headers_mut();
        headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
        headers.insert("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS, POST, PUT, DELETE".parse().unwrap());
        headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Range, x-playback-session-id".parse().unwrap());
        headers.insert("Access-Control-Expose-Headers", "ETag, Content-Length, Content-Range".parse().unwrap());
        
        return response;
    }
    
    let mut response = next.run(req).await;
    
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert("Access-Control-Allow-Methods", "GET, HEAD, OPTIONS, POST, PUT, DELETE".parse().unwrap());
    headers.insert("Access-Control-Allow-Headers", "Content-Type, Authorization, Range, x-playback-session-id".parse().unwrap());
    headers.insert("Access-Control-Expose-Headers", "ETag, Content-Length, Content-Range".parse().unwrap());
    headers.insert("Cache-Control", "max-age=600".parse().unwrap());
    headers.insert("Vary", "Origin".parse().unwrap());
    
    response
}
