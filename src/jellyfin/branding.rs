use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct BrandingOptions {
    #[serde(rename = "LoginDisclaimer")]
    pub login_disclaimer: String,
    #[serde(rename = "CustomCss")]
    pub custom_css: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CultureDto {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "TwoLetterISOLanguageName")]
    pub two_letter_iso_language_name: String,
    #[serde(rename = "ThreeLetterISOLanguageName")]
    pub three_letter_iso_language_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountryInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "TwoLetterISORegionName")]
    pub two_letter_iso_region_name: String,
    #[serde(rename = "ThreeLetterISORegionName")]
    pub three_letter_iso_region_name: Option<String>,
}

pub async fn get_branding_configuration(
    State(_state): State<AppState>,
) -> Result<Json<BrandingOptions>, StatusCode> {
    Ok(Json(BrandingOptions {
        login_disclaimer: String::new(),
        custom_css: String::new(),
    }))
}

pub async fn get_cultures(
    State(_state): State<AppState>,
) -> Result<Json<Vec<CultureDto>>, StatusCode> {
    let cultures = vec![
        CultureDto {
            name: "en-US".to_string(),
            display_name: "English (United States)".to_string(),
            two_letter_iso_language_name: "en".to_string(),
            three_letter_iso_language_name: Some("eng".to_string()),
        },
        CultureDto {
            name: "en-GB".to_string(),
            display_name: "English (United Kingdom)".to_string(),
            two_letter_iso_language_name: "en".to_string(),
            three_letter_iso_language_name: Some("eng".to_string()),
        },
        CultureDto {
            name: "es-ES".to_string(),
            display_name: "Spanish (Spain)".to_string(),
            two_letter_iso_language_name: "es".to_string(),
            three_letter_iso_language_name: Some("spa".to_string()),
        },
        CultureDto {
            name: "fr-FR".to_string(),
            display_name: "French (France)".to_string(),
            two_letter_iso_language_name: "fr".to_string(),
            three_letter_iso_language_name: Some("fra".to_string()),
        },
        CultureDto {
            name: "de-DE".to_string(),
            display_name: "German (Germany)".to_string(),
            two_letter_iso_language_name: "de".to_string(),
            three_letter_iso_language_name: Some("deu".to_string()),
        },
        CultureDto {
            name: "it-IT".to_string(),
            display_name: "Italian (Italy)".to_string(),
            two_letter_iso_language_name: "it".to_string(),
            three_letter_iso_language_name: Some("ita".to_string()),
        },
        CultureDto {
            name: "nl-NL".to_string(),
            display_name: "Dutch (Netherlands)".to_string(),
            two_letter_iso_language_name: "nl".to_string(),
            three_letter_iso_language_name: Some("nld".to_string()),
        },
    ];
    
    Ok(Json(cultures))
}

pub async fn get_countries(
    State(_state): State<AppState>,
) -> Result<Json<Vec<CountryInfo>>, StatusCode> {
    let countries = vec![
        CountryInfo {
            name: "US".to_string(),
            display_name: "United States".to_string(),
            two_letter_iso_region_name: "US".to_string(),
            three_letter_iso_region_name: Some("USA".to_string()),
        },
        CountryInfo {
            name: "GB".to_string(),
            display_name: "United Kingdom".to_string(),
            two_letter_iso_region_name: "GB".to_string(),
            three_letter_iso_region_name: Some("GBR".to_string()),
        },
        CountryInfo {
            name: "CA".to_string(),
            display_name: "Canada".to_string(),
            two_letter_iso_region_name: "CA".to_string(),
            three_letter_iso_region_name: Some("CAN".to_string()),
        },
        CountryInfo {
            name: "AU".to_string(),
            display_name: "Australia".to_string(),
            two_letter_iso_region_name: "AU".to_string(),
            three_letter_iso_region_name: Some("AUS".to_string()),
        },
        CountryInfo {
            name: "DE".to_string(),
            display_name: "Germany".to_string(),
            two_letter_iso_region_name: "DE".to_string(),
            three_letter_iso_region_name: Some("DEU".to_string()),
        },
        CountryInfo {
            name: "FR".to_string(),
            display_name: "France".to_string(),
            two_letter_iso_region_name: "FR".to_string(),
            three_letter_iso_region_name: Some("FRA".to_string()),
        },
        CountryInfo {
            name: "ES".to_string(),
            display_name: "Spain".to_string(),
            two_letter_iso_region_name: "ES".to_string(),
            three_letter_iso_region_name: Some("ESP".to_string()),
        },
        CountryInfo {
            name: "IT".to_string(),
            display_name: "Italy".to_string(),
            two_letter_iso_region_name: "IT".to_string(),
            three_letter_iso_region_name: Some("ITA".to_string()),
        },
        CountryInfo {
            name: "NL".to_string(),
            display_name: "Netherlands".to_string(),
            two_letter_iso_region_name: "NL".to_string(),
            three_letter_iso_region_name: Some("NLD".to_string()),
        },
    ];
    
    Ok(Json(countries))
}
