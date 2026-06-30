use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

/// The schema, applied on startup. Idempotent (`IF NOT EXISTS`) so it doubles
/// as a lightweight migration for this single-version app.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS apartments (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    address     TEXT    NOT NULL,
    lat         REAL    NOT NULL,
    lng         REAL    NOT NULL,
    source_url  TEXT,
    notes       TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS stats (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    kind         TEXT    NOT NULL,
    target_label TEXT,
    target_lat   REAL,
    target_lng   REAL,
    prompt       TEXT,
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS stat_values (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    stat_id      INTEGER NOT NULL REFERENCES stats(id)      ON DELETE CASCADE,
    apartment_id INTEGER NOT NULL REFERENCES apartments(id) ON DELETE CASCADE,
    value_text   TEXT,
    value_number REAL,
    status       TEXT    NOT NULL DEFAULT 'pending',
    detail       TEXT,
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE(stat_id, apartment_id)
);
"#;

/// Open (creating if needed) the SQLite database and apply the schema.
pub async fn init(db_path: &str) -> anyhow::Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(db_path)?
        .create_if_missing(true)
        // Enable cascade deletes on every pooled connection.
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    // `raw_sql` runs the whole multi-statement batch (plain `query` only
    // compiles the first statement).
    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    Ok(pool)
}
