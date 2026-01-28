use axum::{
    extract::Request,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    services::ServeDir,
    trace::TraceLayer,
    cors::{CorsLayer, Any},
};

use crate::collection::CollectionRepo;
use crate::config::Config;
use crate::db::SqliteRepository;
use crate::util::ImageResizer;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Arc<SqliteRepository>,
    pub collections: Arc<CollectionRepo>,
    pub image_resizer: Arc<ImageResizer>,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Arc<SqliteRepository>,
        collections: Arc<CollectionRepo>,
        image_resizer: Arc<ImageResizer>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            db,
            collections,
            image_resizer,
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    let notflix_routes = Router::new()
        .route("/api/collections", get(crate::notflix::list_collections))
        .route("/api/collection/:id", get(crate::notflix::get_collection))
        .route("/api/collection/:id/genres", get(crate::notflix::get_collection_genres))
        .route("/api/collection/:id/items", get(crate::notflix::get_collection_items))
        .route("/api/collection/:coll_id/item/:item_id", get(crate::notflix::get_item))
        .route("/data/*path", get(crate::notflix::serve_data_file));
    
    let jellyfin_routes = Router::new()
        .route("/Branding/Configuration", get(crate::jellyfin::branding::get_branding_configuration))
        .route("/Branding/Css", get(crate::jellyfin::branding::get_branding_css))
        .route("/Branding/Css.css", get(crate::jellyfin::branding::get_branding_css))
        .route("/Devices", get(crate::jellyfin::get_devices).delete(crate::jellyfin::delete_device))
        .route("/Devices/Info", get(crate::jellyfin::get_device_info))
        .route("/Devices/Options", get(crate::jellyfin::get_device_options))
        .route("/DisplayPreferences/usersettings", get(crate::jellyfin::display_preferences))
        .route("/Genres", get(crate::jellyfin::genres::get_genres))
        .route("/Genres/:name", get(crate::jellyfin::genres::get_genre_by_name))
        .route("/Images/:item_id/:image_type", get(crate::jellyfin::image_handler))
        .route("/Images/:item_id/:image_type/:index", get(crate::jellyfin::image_handler_indexed))
        .route("/Items", get(crate::jellyfin::get_items))
        .route("/Items/Filters", get(crate::jellyfin::items::get_item_filters))
        .route("/Items/Filters2", get(crate::jellyfin::items::get_item_filters2))
        .route("/Items/:id", get(crate::jellyfin::get_item_by_id))
        .route("/Items/:id/Ancestors", get(crate::jellyfin::items::get_item_ancestors))
        .route("/Items/:id/PlaybackInfo", axum::routing::post(crate::jellyfin::get_playback_info))
        .route("/Items/:id/Similar", get(crate::jellyfin::get_similar_items))
        .route("/Items/:id/SpecialFeatures", get(crate::jellyfin::get_special_features))
        .route("/Items/:id/ThemeSongs", get(crate::jellyfin::get_theme_songs))
        .route("/Items/:item_id/Images/:image_type", get(crate::jellyfin::image_handler))
        .route("/Items/:item_id/Images/:image_type/:index", get(crate::jellyfin::image_handler_indexed))
        .route("/Items/Counts", get(crate::jellyfin::get_item_counts))
        .route("/Items/Latest", get(crate::jellyfin::get_latest_items))
        .route("/Items/Suggestions", get(crate::jellyfin::get_suggestions))
        .route("/Library/VirtualFolders", get(crate::jellyfin::library::get_virtual_folders))
        .route("/Localization/Countries", get(crate::jellyfin::localization::get_countries))
        .route("/Localization/Cultures", get(crate::jellyfin::localization::get_cultures))
        .route("/Localization/Options", get(crate::jellyfin::localization::get_localization_options))
        .route("/MediaSegments/:id", get(crate::jellyfin::get_media_segments))
        .route("/Movies/Recommendations", get(crate::jellyfin::get_movie_recommendations))
        .route("/Persons", get(crate::jellyfin::get_persons))
        .route("/Persons/:name", get(crate::jellyfin::metadata::get_person_by_name))
        .route("/Playlists", axum::routing::post(crate::jellyfin::create_playlist))
        .route("/Playlists/:playlist_id", get(crate::jellyfin::get_playlist).post(crate::jellyfin::update_playlist))
        .route("/Playlists/:playlist_id/Items", get(crate::jellyfin::get_playlist_items).post(crate::jellyfin::add_playlist_items).delete(crate::jellyfin::delete_playlist_items))
        .route("/Playlists/:playlist_id/Users", get(crate::jellyfin::get_playlist_users))
        .route("/Playlists/:playlist_id/Users/:user_id", get(crate::jellyfin::get_playlist_user))
        .route("/Plugins", get(crate::jellyfin::plugins))
        .route("/QuickConnect/Authorize",  get(crate::jellyfin::auth::quick_connect_authorize))
        .route("/QuickConnect/Connect",    get(crate::jellyfin::auth::quick_connect_connect))
        .route("/QuickConnect/Enabled",    get(crate::jellyfin::auth::quick_connect_enabled))
        .route("/QuickConnect/Initiate",   axum::routing::post(crate::jellyfin::auth::quick_connect_initiate))
        .route("/Search/Hints", get(crate::jellyfin::search_hints))
        .route("/Sessions", get(crate::jellyfin::get_sessions))
        .route("/Sessions/Capabilities", axum::routing::post(crate::jellyfin::post_session_capabilities))
        .route("/Sessions/Capabilities/Full", axum::routing::post(crate::jellyfin::post_session_capabilities_full))
        .route("/Sessions/Playing", axum::routing::post(crate::jellyfin::session_playing_progress))
        .route("/Sessions/Playing/Progress", axum::routing::post(crate::jellyfin::session_playing_progress))
        .route("/Sessions/Playing/Stopped", axum::routing::post(crate::jellyfin::session_playing_progress))
        .route("/Shows/:id/Episodes", get(crate::jellyfin::get_episodes))
        .route("/Shows/:id/Seasons", get(crate::jellyfin::get_seasons))
        .route("/Shows/NextUp", get(crate::jellyfin::get_next_up))
        .route("/Studios", get(crate::jellyfin::get_studios))
        .route("/Studios/:name", get(crate::jellyfin::metadata::get_studio_by_name))
        .route("/System/Info", get(crate::jellyfin::system_info))
        .route("/System/Info/Public", get(crate::jellyfin::public_system_info))
        .route("/System/Ping", get(crate::jellyfin::system_ping_handler))
        .route("/UserViews", get(crate::jellyfin::get_user_views))
        .route("/Users", get(crate::jellyfin::get_users))
        .route("/Users/AuthenticateByName", axum::routing::post(crate::jellyfin::authenticate_by_name))
        .route("/Users/Me", get(crate::jellyfin::get_current_user))
        .route("/Users/:user_id/FavoriteItems/:id", axum::routing::post(crate::jellyfin::mark_favorite))
        .route("/Users/:user_id/FavoriteItems/:id", axum::routing::delete(crate::jellyfin::unmark_favorite))
        .route("/Users/:user_id/Images/:image_type", get(crate::jellyfin::get_user_image))
        .route("/Users/:user_id/Items", get(crate::jellyfin::get_items))
        .route("/Users/:user_id/Items/Latest", get(crate::jellyfin::get_latest_items))
        .route("/Users/:user_id/Items/Resume", get(crate::jellyfin::get_resume_items))
        .route("/Users/:user_id/Items/:id", get(crate::jellyfin::get_user_item_by_id))
        .route("/Users/:user_id/PlayedItems/:id", axum::routing::delete(crate::jellyfin::mark_unplayed))
        .route("/Users/:user_id/PlayedItems/:id", axum::routing::post(crate::jellyfin::mark_played))
        .route("/Users/:user_id/PlayingItems/:id/Progress", axum::routing::post(crate::jellyfin::update_playback_position))
        .route("/Users/:user_id/Views", get(crate::jellyfin::get_user_views))
        .route("/Videos/:id/:index/Subtitles", get(crate::jellyfin::stream_subtitle))
        .route("/Videos/:id/Subtitles/:index/Stream", get(crate::jellyfin::stream_subtitle))
        .route("/Videos/:id/stream", get(crate::jellyfin::stream_video_with_range))
        .route("/Videos/:id/stream.m4v", get(crate::jellyfin::stream_video_with_range))
        .route("/Videos/:id/stream.mkv", get(crate::jellyfin::stream_video_with_range))
        .route("/Videos/:id/stream.mp4", get(crate::jellyfin::stream_video_with_range))
        // Legacy/Alias Routes
        .route("/UserViews/GroupingOptions", get(crate::jellyfin::get_grouping_options))
        .route("/UserItems/Resume", get(crate::jellyfin::get_resume_items))
        .route("/UserItems/:id/Userdata", get(crate::jellyfin::get_item_user_data))
        .route("/UserFavoriteItems/:id", axum::routing::post(crate::jellyfin::mark_favorite))
        .route("/UserFavoriteItems/:id", axum::routing::delete(crate::jellyfin::unmark_favorite))
        .route("/UserPlayedItems/:id", axum::routing::post(crate::jellyfin::mark_played))
        .route("/UserPlayedItems/:id", axum::routing::delete(crate::jellyfin::mark_unplayed))
        .route("/health", get(crate::jellyfin::health_handler))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::jellyfin::auth_middleware,
        ))
        .layer(axum::middleware::from_fn(crate::middleware::etag_validation));
    
    let mut router = Router::new()
        .route("/robots.txt", get(robots_txt_handler))
        .merge(notflix_routes)
        .merge(jellyfin_routes)
        .fallback(fallback_handler);

    if let Some(ref appdir) = state.config.appdir {
        // Note: ServeDir will override our fallback for file paths, but OPTIONS will still work
        // because they'll hit our fallback before ServeDir tries to serve
        router = router.fallback_service(ServeDir::new(appdir));
    }

    router
        .layer(axum::middleware::from_fn(crate::middleware::normalize_path))
        .layer(axum::middleware::from_fn_with_state(state.clone(), crate::middleware::log_request))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )

        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn robots_txt_handler() -> &'static str {
    "User-agent: *\nDisallow: /\n"
}

async fn fallback_handler(req: Request<axum::body::Body>) -> impl IntoResponse {
    // Handle OPTIONS requests for CORS preflight
    if req.method() == axum::http::Method::OPTIONS {
        // CORS headers are added by the add_cors_headers middleware
        return StatusCode::OK.into_response();
    }
    // All other unmatched requests get 404
    StatusCode::NOT_FOUND.into_response()
}
