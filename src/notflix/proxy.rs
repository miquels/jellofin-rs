use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, Request, StatusCode},
    response::Response,
};
use reqwest;
use std::time::Duration;

use crate::server::AppState;

// Hop-by-hop headers that should be removed when proxying
const HOP_HEADERS: &[&str] = &[
    "Connection",
    "Keep-Alive",
    "Proxy-Authenticate",
    "Proxy-Authorization",
    "Te",
    "Trailers",
    "Transfer-Encoding",
    "Upgrade",
];

pub async fn hls_proxy(
    State(state): State<AppState>,
    Path((source, path)): Path<(String, String)>,
    req: Request<Body>,
) -> Result<Response, StatusCode> {
    // Check if this is an HLS request (contains .mp4/)
    if !path.contains(".mp4/") {
        return Err(StatusCode::NOT_FOUND);
    }

    // Get the collection
    let collections = state.collections.list_collections().await;
    let collection = collections
        .iter()
        .find(|c| c.id == source)
        .ok_or(StatusCode::NOT_FOUND)?;

    // Get HLS server URL
    let hls_server = collection
        .hls_server
        .as_ref()
        .ok_or(StatusCode::NOT_FOUND)?;

    // Build the target URL
    let target_url = build_url(hls_server, &path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Prepare headers for proxying
    let mut proxy_headers = reqwest::header::HeaderMap::new();

    // Copy headers from original request, excluding hop-by-hop headers
    for (name, value) in req.headers().iter() {
        let name_str = name.as_str();
        if !HOP_HEADERS.contains(&name_str)
            && name_str != "access-control-allow-origin"
            && name_str != "access-control-allow-methods"
        {
            if let Ok(header_name) =
                reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes())
            {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_bytes(value.as_bytes())
                {
                    proxy_headers.insert(header_name, header_value);
                }
            }
        }
    }

    // Add X-Forwarded-For header
    if let Some(remote_addr) = req.headers().get("x-forwarded-for") {
        if let Ok(value) = remote_addr.to_str() {
            if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(b"x-forwarded-for") {
                if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value) {
                    proxy_headers.insert(header_name, header_value);
                }
            }
        }
    }

    // Make the proxy request
    let proxy_response = client
        .get(&target_url)
        .headers(proxy_headers)
        .send()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Build response
    let status_code = proxy_response.status();
    let mut response_headers = HeaderMap::new();

    // Copy response headers, excluding hop-by-hop headers
    for (name, value) in proxy_response.headers().iter() {
        let name_str = name.as_str();
        if !HOP_HEADERS.contains(&name_str) {
            if let Ok(header_name) = header::HeaderName::from_bytes(name.as_str().as_bytes()) {
                if let Ok(header_value) = header::HeaderValue::from_bytes(value.as_bytes()) {
                    response_headers.insert(header_name, header_value);
                }
            }
        }
    }

    // Get response body
    let body_bytes = proxy_response
        .bytes()
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Build final response
    let mut response = Response::new(Body::from(body_bytes));
    *response.status_mut() =
        StatusCode::from_u16(status_code.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    *response.headers_mut() = response_headers;

    Ok(response)
}

fn build_url(server: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path_segments: Vec<String> = path
        .split('/')
        .map(|s| urlencoding::encode(s).to_string())
        .collect();

    let encoded_path = path_segments.join("/");
    let url = format!("{}{}", server, encoded_path);

    Ok(url)
}
