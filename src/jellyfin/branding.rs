use axum::{
    extract::State,
    http::{StatusCode, HeaderMap, header},
    Json,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BrandingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login_disclaimer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splashscreen_enabled: Option<bool>,
}

pub async fn get_branding_configuration(
    State(_state): State<AppState>,
) -> Result<Json<BrandingOptions>, StatusCode> {
    Ok(Json(BrandingOptions {
        login_disclaimer: None,
        custom_css: None,
        splashscreen_enabled: Some(false),
    }))
}

pub async fn get_branding_css(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let css = "";
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
    (headers, css)
}
