use anyhow::{anyhow, Context};
use serde::Deserialize;

/// Result of a foot-routing query.
#[derive(Debug, Clone, Copy)]
pub struct WalkResult {
    /// Walking distance in meters.
    pub meters: f64,
    /// Walking time in seconds.
    pub seconds: f64,
}

#[derive(Debug, Deserialize)]
struct OsrmResponse {
    code: String,
    #[serde(default)]
    routes: Vec<OsrmRoute>,
}

#[derive(Debug, Deserialize)]
struct OsrmRoute {
    distance: f64,
    duration: f64,
}

/// Compute the walking route between two `(lat, lng)` points via OSRM's foot
/// profile. OSRM expects coordinates as `lng,lat`.
pub async fn walk(
    client: &reqwest::Client,
    base_url: &str,
    from: (f64, f64),
    to: (f64, f64),
) -> anyhow::Result<WalkResult> {
    let url = format!(
        "{}/route/v1/foot/{},{};{},{}",
        base_url.trim_end_matches('/'),
        from.1,
        from.0,
        to.1,
        to.0,
    );

    let data: OsrmResponse = client
        .get(url)
        .query(&[("overview", "false")])
        .send()
        .await
        .context("requesting OSRM")?
        .error_for_status()
        .context("OSRM returned an error status")?
        .json()
        .await
        .context("parsing OSRM response")?;

    if data.code != "Ok" {
        return Err(anyhow!("OSRM could not find a route (code: {})", data.code));
    }

    let route = data
        .routes
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("OSRM returned no route"))?;

    Ok(WalkResult {
        meters: route.distance,
        seconds: route.duration,
    })
}
