use axum::{
    extract::{Query, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use std::collections::HashMap;

use crate::db::{AccessToken, User, UserRepo, AccessTokenRepo};
use crate::server::AppState;
use super::types::*;

pub async fn authenticate_by_name(
    State(state): State<AppState>,
    Json(req): Json<AuthenticationRequest>,
) -> Result<Json<AuthenticationResult>, StatusCode> {
    let username = req.username.trim();
    
    let user = match state.db.get_user(username).await {
        Ok(user) => user,
        Err(_) => {
            if state.config.jellyfin.autoregister {
                let new_user = User {
                    id: uuid::Uuid::new_v4().to_string(),
                    username: username.to_string(),
                    password: String::new(),
                    created: Some(chrono::Utc::now().to_rfc3339()),
                    lastlogin: None,
                    lastused: None,
                };
                state.db.upsert_user(&new_user).await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                new_user
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    };
    
    let token = AccessToken {
        token: uuid::Uuid::new_v4().to_string(),
        userid: user.id.clone(),
        deviceid: Some(String::new()),
        devicename: Some(String::new()),
        applicationname: Some(String::new()),
        applicationversion: Some(String::new()),
        remoteaddress: None,
        created: Some(chrono::Utc::now()),
        lastused: None,
    };
    
    state.db.upsert_token(&token).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let result = AuthenticationResult {
        user: UserDto {
            name: user.username.clone(),
            id: user.id.clone(),
            has_password: false,
            has_configured_password: false,
            policy: UserPolicy {
                is_administrator: false,
                is_disabled: false,
                enable_all_folders: true,
            },
        },
        access_token: token.token.clone(),
        server_id: state.config.jellyfin.server_id.clone().unwrap_or_else(|| "jellyfin-rs".to_string()),
    };
    
    Ok(Json(result))
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_token(&req, &params);
    
    if let Some(token_str) = token {
        if let Ok(token) = state.db.get_token(&token_str).await {
            req.extensions_mut().insert(token.userid.clone());
        }
    }
    
    Ok(next.run(req).await)
}

fn extract_token<B>(req: &Request<B>, params: &HashMap<String, String>) -> Option<String> {
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = parse_emby_auth(auth_str) {
                return Some(token);
            }
        }
    }
    
    if let Some(auth_header) = req.headers().get("X-Emby-Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = parse_emby_auth(auth_str) {
                return Some(token);
            }
        }
    }
    
    if let Some(token) = req.headers().get("X-Emby-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string()) {
        return Some(token);
    }
    
    if let Some(token) = req.headers().get("X-MediaBrowser-Token")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string()) {
        return Some(token);
    }
    
    if let Some(token) = params.get("ApiKey").or_else(|| params.get("api_key")) {
        return Some(token.clone());
    }
    
    None
}

fn parse_emby_auth(auth_str: &str) -> Option<String> {
    for part in auth_str.split(',') {
        let part = part.trim();
        if let Some(token_part) = part.strip_prefix("Token=") {
            return Some(token_part.trim_matches('"').to_string());
        }
    }
    None
}

pub fn get_user_id<B>(req: &Request<B>) -> Option<String> {
    req.extensions().get::<String>().cloned()
}
