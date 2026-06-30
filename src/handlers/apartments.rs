use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::compute;
use crate::error::AppError;
use crate::models::{Apartment, NewApartment};
use crate::services::nominatim;
use crate::state::AppState;

/// `GET /api/apartments` — the full search list.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Apartment>>, AppError> {
    let apartments = sqlx::query_as::<_, Apartment>("SELECT * FROM apartments ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(apartments))
}

/// `POST /api/apartments` — geocode the address, store the apartment, then
/// backfill all existing stats for it.
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewApartment>,
) -> Result<Json<Apartment>, AppError> {
    let address = body.address.trim();
    if address.is_empty() {
        return Err(AppError::bad_request("address is required"));
    }

    let mut candidates = nominatim::geocode(&state.http, &state.config.nominatim_url, address)
        .await
        .map_err(|e| AppError::new(StatusCode::BAD_GATEWAY, e.to_string()))?;

    if candidates.is_empty() {
        return Err(AppError::bad_request(format!(
            "could not find a location for '{address}'"
        )));
    }
    let best = candidates.remove(0);

    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| best.label.clone());

    let apt = sqlx::query_as::<_, Apartment>(
        r#"
        INSERT INTO apartments (name, address, lat, lng, source_url, notes)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&name)
    .bind(&best.label)
    .bind(best.lat)
    .bind(best.lng)
    .bind(body.source_url.as_deref().filter(|s| !s.is_empty()))
    .bind(body.notes.as_deref().filter(|s| !s.is_empty()))
    .fetch_one(&state.db)
    .await?;

    // Backfill existing stats for the new apartment.
    if let Err(e) = compute::run_all_stats_for_apartment(&state, &apt).await {
        tracing::error!("failed to backfill stats for apartment {}: {e}", apt.id);
    }

    Ok(Json(apt))
}

/// `DELETE /api/apartments/:id` — remove an apartment (its stat values cascade).
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM apartments WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::new(StatusCode::NOT_FOUND, "apartment not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
