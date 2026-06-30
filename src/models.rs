use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// An apartment in the user's search list.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Apartment {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

/// A saved metric definition that applies to every apartment in the list.
/// `kind` is either `"walking"` (distance to a target location) or `"ai"`
/// (a free-form research question answered by the LLM).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Stat {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub target_label: Option<String>,
    pub target_lat: Option<f64>,
    pub target_lng: Option<f64>,
    pub prompt: Option<String>,
    pub created_at: String,
}

/// One computed cell: the value of a stat for a particular apartment.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StatValue {
    pub id: i64,
    pub stat_id: i64,
    pub apartment_id: i64,
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
    pub status: String,
    pub detail: Option<String>,
    pub updated_at: String,
}

/// Request body for adding an apartment (looked up by address).
#[derive(Debug, Deserialize)]
pub struct NewApartment {
    pub address: String,
    pub name: Option<String>,
    pub source_url: Option<String>,
    pub notes: Option<String>,
}

/// Request body for creating a stat.
#[derive(Debug, Deserialize)]
pub struct NewStat {
    pub name: String,
    pub kind: String,
    pub target_label: Option<String>,
    pub target_lat: Option<f64>,
    pub target_lng: Option<f64>,
    pub prompt: Option<String>,
}
