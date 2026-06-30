use crate::config::Config;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Shared application state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub http: reqwest::Client,
    pub config: Arc<Config>,
}
