use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use crate::error::AppError;
use crate::services::nominatim::{self, GeoResult};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GeocodeQuery {
    pub q: String,
}

/// `GET /api/geocode?q=...` — proxy to Nominatim for the address/place search
/// boxes (adding an apartment, or picking a walking-stat target).
pub async fn geocode(
    State(state): State<AppState>,
    Query(query): Query<GeocodeQuery>,
) -> Result<Json<Vec<GeoResult>>, AppError> {
    let q = query.q.trim();
    if q.is_empty() {
        return Err(AppError::bad_request("query 'q' is required"));
    }
    let results = nominatim::geocode(&state.http, &state.config.nominatim_url, q)
        .await
        .map_err(|e| AppError::new(StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(results))
}
