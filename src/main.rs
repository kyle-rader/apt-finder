mod compute;
mod config;
mod db;
mod error;
mod handlers;
mod models;
mod services;
mod state;

use std::sync::Arc;
use std::time::Duration;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "apt_finder=info,tower_http=warn".into()),
        )
        .init();

    let config = Config::from_env();
    tracing::info!(?config, "starting apt-finder");

    let db = db::init(&config.db_path).await?;
    let http = build_http_client(&config)?;

    let state = AppState {
        db,
        http,
        config: Arc::new(config.clone()),
    };

    let app = build_router(state, &config.static_dir);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Build the reqwest client used for Nominatim/OSRM. Nominatim requires a
/// descriptive User-Agent. An optional extra CA cert supports proxied TLS.
fn build_http_client(config: &Config) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent("apt-finder/0.1 (apartment comparison tool)")
        .timeout(Duration::from_secs(30));

    if let Some(path) = &config.extra_ca_cert {
        let pem = std::fs::read(path)?;
        let cert = reqwest::Certificate::from_pem(&pem)?;
        builder = builder.add_root_certificate(cert);
    }

    Ok(builder.build()?)
}

/// Assemble the API routes plus static SPA serving.
fn build_router(state: AppState, static_dir: &str) -> Router {
    let api = Router::new()
        .route("/state", get(handlers::full_state))
        .route(
            "/apartments",
            get(handlers::apartments::list).post(handlers::apartments::create),
        )
        .route("/apartments/{id}", delete(handlers::apartments::delete))
        .route("/geocode", get(handlers::geocode::geocode))
        .route(
            "/stats",
            get(handlers::stats::list).post(handlers::stats::create),
        )
        .route("/stats/{id}", delete(handlers::stats::delete))
        .route("/stats/{id}/recompute", post(handlers::stats::recompute));

    // Serve the built SPA, falling back to index.html so client-side routing
    // (and a direct hit on `/`) works.
    let index = format!("{}/index.html", static_dir.trim_end_matches('/'));
    let static_service = ServeDir::new(static_dir).fallback(ServeFile::new(index));

    Router::new()
        .nest("/api", api)
        .fallback_service(static_service)
        // Permissive CORS so the Vite dev server (different port) can call the API.
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
