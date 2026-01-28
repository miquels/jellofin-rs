use axum::{extract::Request, http::StatusCode, response::IntoResponse, routing::get, Router};
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
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
        .route(
            "/api/collection/:id/genres",
            get(crate::notflix::get_collection_genres),
        )
        .route(
            "/api/collection/:id/items",
            get(crate::notflix::get_collection_items),
        )
        .route(
            "/api/collection/:coll_id/item/:item_id",
            get(crate::notflix::get_item),
        )
        .route("/data/*path", get(crate::notflix::serve_data_file));

    let jellyfin_routes = crate::jellyfin::jellyfin::build_jellyfin_router(state.clone());

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
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::log_request,
        ))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
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
