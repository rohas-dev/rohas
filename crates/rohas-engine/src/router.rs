use axum::Router;
use tower_http::cors::{Any, CorsLayer};

pub fn with_cors(router: Router) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    router.layer(cors)
}
