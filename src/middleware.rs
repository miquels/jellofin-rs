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
    headers.insert("Cross-Origin-Resource-Policy", "cross-origin".parse().unwrap());
    headers.insert("Cache-Control", "max-age=86400, stale-while-revalidate=600".parse().unwrap());

    response
}

pub async fn etag_validation(req: Request, next: Next) -> Response {
    // Get the If-None-Match header from the request
    let if_none_match = req.headers()
        .get(axum::http::header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    
    let response = next.run(req).await;
    
    // If there's an If-None-Match header, check it against the response ETag
    if let Some(client_etag) = if_none_match {
        if let Some(response_etag) = response.headers().get(axum::http::header::ETAG) {
            if let Ok(response_etag_str) = response_etag.to_str() {
                // Compare ETags (handle both strong and weak ETags)
                if etags_match(&client_etag, response_etag_str) {
                    // ETags match - return 304 Not Modified with empty body
                    let mut not_modified = Response::new(axum::body::Body::empty());
                    *not_modified.status_mut() = axum::http::StatusCode::NOT_MODIFIED;
                    
                    // Copy relevant headers from original response
                    let headers = not_modified.headers_mut();
                    if let Some(etag) = response.headers().get(axum::http::header::ETAG) {
                        headers.insert(axum::http::header::ETAG, etag.clone());
                    }
                    if let Some(cache_control) = response.headers().get(axum::http::header::CACHE_CONTROL) {
                        headers.insert(axum::http::header::CACHE_CONTROL, cache_control.clone());
                    }
                    if let Some(vary) = response.headers().get(axum::http::header::VARY) {
                        headers.insert(axum::http::header::VARY, vary.clone());
                    }
                    
                    return not_modified;
                }
            }
        }
    }
    
    response
}

fn etags_match(client_etag: &str, server_etag: &str) -> bool {
    // Handle multiple ETags in If-None-Match (comma-separated)
    for etag in client_etag.split(',') {
        let etag = etag.trim();
        
        // Check for exact match
        if etag == server_etag {
            return true;
        }
        
        // Handle weak ETag comparison (W/"..." matches "...")
        let client_stripped = etag.strip_prefix("W/").unwrap_or(etag);
        let server_stripped = server_etag.strip_prefix("W/").unwrap_or(server_etag);
        
        if client_stripped == server_stripped {
            return true;
        }
    }
    
    false
}
