//! Orchestration for computing stat values across the search list.
//!
//! Walking stats are computed inline (OSRM is fast). AI stats are run in the
//! background with bounded concurrency, since each call spawns a heavy LLM
//! subprocess; the frontend polls `/api/state` to watch `pending` rows fill in.

use crate::models::{Apartment, Stat};
use crate::services::{ai, osrm};
use crate::state::AppState;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Maximum number of AI subprocesses running at once.
const AI_CONCURRENCY: usize = 3;

/// Insert-or-update the value of a stat for one apartment.
async fn upsert_value(
    db: &SqlitePool,
    stat_id: i64,
    apartment_id: i64,
    value_text: Option<&str>,
    value_number: Option<f64>,
    status: &str,
    detail: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO stat_values
            (stat_id, apartment_id, value_text, value_number, status, detail, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
        ON CONFLICT(stat_id, apartment_id) DO UPDATE SET
            value_text   = excluded.value_text,
            value_number = excluded.value_number,
            status       = excluded.status,
            detail       = excluded.detail,
            updated_at   = excluded.updated_at
        "#,
    )
    .bind(stat_id)
    .bind(apartment_id)
    .bind(value_text)
    .bind(value_number)
    .bind(status)
    .bind(detail)
    .execute(db)
    .await?;
    Ok(())
}

/// Mark a stat's value for one apartment as `pending` (used before kicking off
/// an async AI run, so the UI shows a spinner immediately).
pub async fn mark_pending(db: &SqlitePool, stat_id: i64, apartment_id: i64) -> anyhow::Result<()> {
    upsert_value(db, stat_id, apartment_id, None, None, "pending", None).await
}

/// Compute and store a walking stat for a single apartment (inline).
pub async fn compute_walking(state: &AppState, stat: &Stat, apt: &Apartment) -> anyhow::Result<()> {
    let (target_lat, target_lng) = match (stat.target_lat, stat.target_lng) {
        (Some(lat), Some(lng)) => (lat, lng),
        _ => {
            upsert_value(
                &state.db,
                stat.id,
                apt.id,
                None,
                None,
                "error",
                Some("walking stat is missing a target location"),
            )
            .await?;
            return Ok(());
        }
    };

    match osrm::walk(
        &state.http,
        &state.config.osrm_url,
        (apt.lat, apt.lng),
        (target_lat, target_lng),
    )
    .await
    {
        Ok(r) => {
            let miles = r.meters / 1609.344;
            let minutes = r.seconds / 60.0;
            let text = format!("{:.2} mi · {:.0} min", miles, minutes);
            let detail = format!(
                "{{\"meters\":{:.1},\"seconds\":{:.1},\"miles\":{:.3},\"minutes\":{:.1}}}",
                r.meters, r.seconds, miles, minutes
            );
            upsert_value(
                &state.db,
                stat.id,
                apt.id,
                Some(&text),
                Some(r.meters),
                "ok",
                Some(&detail),
            )
            .await?;
        }
        Err(e) => {
            upsert_value(
                &state.db,
                stat.id,
                apt.id,
                None,
                None,
                "error",
                Some(&e.to_string()),
            )
            .await?;
        }
    }
    Ok(())
}

/// Build the research prompt for an AI stat + apartment.
fn ai_prompt(stat: &Stat, apt: &Apartment) -> String {
    let question = stat.prompt.as_deref().unwrap_or(&stat.name);
    let url_line = match &apt.source_url {
        Some(url) if !url.is_empty() => format!(" Listing/source URL: {}.", url),
        _ => String::new(),
    };
    format!(
        "You are researching an apartment for someone deciding where to live.\n\
         Apartment: \"{name}\" at {address}.{url_line}\n\n\
         Question: {question}\n\n\
         Search the building's website, listings, and recent resident reviews to answer. \
         Be concise and specific. If you cannot determine the answer, say so honestly. \
         End your reply with a single line in exactly this format:\n\
         ANSWER: <a short phrase answering the question>",
        name = apt.name,
        address = apt.address,
        url_line = url_line,
        question = question,
    )
}

/// Run an AI research stat for a single apartment (inline await).
pub async fn compute_ai(state: &AppState, stat: &Stat, apt: &Apartment) -> anyhow::Result<()> {
    let prompt = ai_prompt(stat, apt);
    match ai::research(&state.config.ai_command, &prompt).await {
        Ok(result) => {
            upsert_value(
                &state.db,
                stat.id,
                apt.id,
                Some(&result.answer),
                None,
                "ok",
                Some(&result.full),
            )
            .await?;
        }
        Err(e) => {
            upsert_value(
                &state.db,
                stat.id,
                apt.id,
                None,
                None,
                "error",
                Some(&e.to_string()),
            )
            .await?;
        }
    }
    Ok(())
}

/// Fan a stat out across every apartment in the list.
///
/// Walking stats are computed inline and the function returns once all values
/// are stored. AI stats are marked `pending`, then processed on a background
/// task (returning immediately) so the HTTP request doesn't block on the LLM.
pub async fn run_stat_for_all(state: &AppState, stat: Stat) -> anyhow::Result<()> {
    let apartments: Vec<Apartment> =
        sqlx::query_as::<_, Apartment>("SELECT * FROM apartments ORDER BY created_at")
            .fetch_all(&state.db)
            .await?;

    if stat.kind == "ai" {
        for apt in &apartments {
            mark_pending(&state.db, stat.id, apt.id).await?;
        }
        spawn_ai_run(state.clone(), stat, apartments);
    } else {
        for apt in &apartments {
            compute_walking(state, &stat, apt).await?;
        }
    }
    Ok(())
}

/// Apply every existing stat to a newly added apartment.
pub async fn run_all_stats_for_apartment(
    state: &AppState,
    apt: &Apartment,
) -> anyhow::Result<()> {
    let stats: Vec<Stat> = sqlx::query_as::<_, Stat>("SELECT * FROM stats ORDER BY created_at")
        .fetch_all(&state.db)
        .await?;

    for stat in stats {
        if stat.kind == "ai" {
            mark_pending(&state.db, stat.id, apt.id).await?;
            spawn_ai_run(state.clone(), stat, vec![apt.clone()]);
        } else {
            compute_walking(state, &stat, apt).await?;
        }
    }
    Ok(())
}

/// Spawn a background task that runs an AI stat over the given apartments with
/// bounded concurrency.
fn spawn_ai_run(state: AppState, stat: Stat, apartments: Vec<Apartment>) {
    tokio::spawn(async move {
        let semaphore = Arc::new(Semaphore::new(AI_CONCURRENCY));
        let stat = Arc::new(stat);
        let mut set = tokio::task::JoinSet::new();

        for apt in apartments {
            let permit = semaphore.clone().acquire_owned().await;
            let state = state.clone();
            let stat = stat.clone();
            set.spawn(async move {
                let _permit = permit; // held until this apartment finishes
                if let Err(e) = compute_ai(&state, &stat, &apt).await {
                    tracing::error!("AI research failed for apartment {}: {e}", apt.id);
                }
            });
        }

        // Drain so panics are observed and the task lives until all are done.
        while set.join_next().await.is_some() {}
    });
}
