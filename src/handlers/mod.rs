pub mod apartments;
pub mod geocode;
pub mod stats;

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::error::AppError;
use crate::models::{Apartment, Stat, StatValue};
use crate::state::AppState;

/// Combined snapshot used for the initial load and for polling. Returning
/// everything in one request keeps the frontend's poll loop simple.
#[derive(Serialize)]
pub struct FullState {
    pub apartments: Vec<Apartment>,
    pub stats: Vec<Stat>,
    pub values: Vec<StatValue>,
}

/// `GET /api/state` — apartments, stats, and all computed values at once.
pub async fn full_state(State(state): State<AppState>) -> Result<Json<FullState>, AppError> {
    let apartments = sqlx::query_as::<_, Apartment>("SELECT * FROM apartments ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;
    let stats = sqlx::query_as::<_, Stat>("SELECT * FROM stats ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;
    let values = sqlx::query_as::<_, StatValue>("SELECT * FROM stat_values")
        .fetch_all(&state.db)
        .await?;

    Ok(Json(FullState {
        apartments,
        stats,
        values,
    }))
}
