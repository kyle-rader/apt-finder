use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;

use crate::compute;
use crate::error::AppError;
use crate::models::{NewStat, Stat};
use crate::state::AppState;

/// `GET /api/stats` — all saved stat definitions.
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Stat>>, AppError> {
    let stats = sqlx::query_as::<_, Stat>("SELECT * FROM stats ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(stats))
}

/// `POST /api/stats` — create a stat and fan it out across the search list.
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<NewStat>,
) -> Result<Json<Stat>, AppError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::bad_request("stat name is required"));
    }

    match body.kind.as_str() {
        "walking" => {
            if body.target_lat.is_none() || body.target_lng.is_none() {
                return Err(AppError::bad_request(
                    "walking stats require target_lat and target_lng",
                ));
            }
        }
        "ai" => {
            if body
                .prompt
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err(AppError::bad_request("ai stats require a prompt"));
            }
        }
        other => {
            return Err(AppError::bad_request(format!(
                "unknown stat kind '{other}' (expected 'walking' or 'ai')"
            )));
        }
    }

    let stat = sqlx::query_as::<_, Stat>(
        r#"
        INSERT INTO stats (name, kind, target_label, target_lat, target_lng, prompt)
        VALUES (?, ?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(name)
    .bind(&body.kind)
    .bind(body.target_label.as_deref())
    .bind(body.target_lat)
    .bind(body.target_lng)
    .bind(body.prompt.as_deref().map(str::trim))
    .fetch_one(&state.db)
    .await?;

    compute::run_stat_for_all(&state, stat.clone()).await?;
    Ok(Json(stat))
}

/// `POST /api/stats/:id/recompute` — re-run a stat for the whole list.
pub async fn recompute(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Stat>, AppError> {
    let stat = sqlx::query_as::<_, Stat>("SELECT * FROM stats WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| AppError::new(StatusCode::NOT_FOUND, "stat not found"))?;

    compute::run_stat_for_all(&state, stat.clone()).await?;
    Ok(Json(stat))
}

/// `DELETE /api/stats/:id` — remove a stat (its values cascade).
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM stats WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::new(StatusCode::NOT_FOUND, "stat not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}
