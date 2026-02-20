mod config;
mod error;
mod pipeline;
mod routes;
mod state;
mod types;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rideviz_rs=info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env();
    let state = state::AppState::new();

    // Start cache eviction task
    let eviction_state = state.clone();
    let eviction_ttl = config.cache_ttl;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await; // Every 5 minutes
            eviction_state.evict_expired(eviction_ttl);
        }
    });

    // Build router
    let serve_dir = ServeDir::new("assets/web")
        .not_found_service(ServeFile::new("assets/web/index.html"));

    let app = Router::new()
        .merge(routes::health::router())
        .merge(routes::upload::router())
        .merge(routes::visualize::router())
        .fallback_service(serve_dir)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(axum::extract::DefaultBodyLimit::max(config.max_file_size))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!("RideViz-RS listening on {}", addr);
    tracing::info!("Health check: http://{}/health", addr);
    tracing::info!("Upload: POST http://{}/api/upload", addr);
    tracing::info!("Visualize: POST http://{}/api/visualize", addr);

    axum::serve(listener, app).await.unwrap();
}
