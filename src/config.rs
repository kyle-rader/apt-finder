use std::env;

/// Runtime configuration, sourced from environment variables with sensible
/// defaults so the app runs with zero setup (`cargo run`).
#[derive(Clone, Debug)]
pub struct Config {
    /// Port the HTTP server binds to.
    pub port: u16,
    /// Path to the SQLite database file (created if missing).
    pub db_path: String,
    /// Base URL for a Nominatim geocoding instance.
    pub nominatim_url: String,
    /// Base URL for an OSRM routing instance (foot profile is used).
    pub osrm_url: String,
    /// Executable invoked for AI research (Claude Code CLI by default).
    pub ai_command: String,
    /// Directory of built frontend assets to serve.
    pub static_dir: String,
    /// Optional extra CA certificate (PEM) to trust for outbound HTTPS.
    /// Useful behind a proxy that performs TLS interception.
    pub extra_ca_cert: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            db_path: env::var("DB_PATH").unwrap_or_else(|_| "apt_finder.db".to_string()),
            nominatim_url: env::var("NOMINATIM_URL")
                .unwrap_or_else(|_| "https://nominatim.openstreetmap.org".to_string()),
            osrm_url: env::var("OSRM_URL")
                .unwrap_or_else(|_| "https://router.project-osrm.org".to_string()),
            ai_command: env::var("AI_COMMAND").unwrap_or_else(|_| "claude".to_string()),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "frontend/dist".to_string()),
            extra_ca_cert: env::var("EXTRA_CA_CERT").ok().filter(|s| !s.is_empty()),
        }
    }
}
