use anyhow::Context;
use serde::{Deserialize, Serialize};

/// A geocoding candidate returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct GeoResult {
    pub label: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Deserialize)]
struct NominatimItem {
    display_name: String,
    lat: String,
    lon: String,
}

/// Geocode a free-text query (address or place name) via Nominatim.
/// Returns up to 5 candidates, best match first.
pub async fn geocode(
    client: &reqwest::Client,
    base_url: &str,
    query: &str,
) -> anyhow::Result<Vec<GeoResult>> {
    let url = format!("{}/search", base_url.trim_end_matches('/'));
    let items: Vec<NominatimItem> = client
        .get(url)
        .query(&[
            ("q", query),
            ("format", "jsonv2"),
            ("limit", "5"),
            ("addressdetails", "0"),
        ])
        .send()
        .await
        .context("requesting Nominatim")?
        .error_for_status()
        .context("Nominatim returned an error status")?
        .json()
        .await
        .context("parsing Nominatim response")?;

    Ok(items
        .into_iter()
        .filter_map(|item| {
            Some(GeoResult {
                label: item.display_name,
                lat: item.lat.parse().ok()?,
                lng: item.lon.parse().ok()?,
            })
        })
        .collect())
}
