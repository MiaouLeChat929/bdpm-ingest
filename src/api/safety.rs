//! Safety information endpoint.
//!
//! Serves safety alerts from the `safety_alerts` table (populated by
//! CIS_InfoImportantes.txt import). Falls back to stub when no data.

use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::AppState;

/// Safety info response.
#[derive(Serialize, ToSchema)]
pub struct SafetyResponse {
    pub cis: String,
    pub data_available: bool,
    pub alerts: Vec<SafetyAlert>,
}

/// A single safety alert.
#[derive(Serialize, ToSchema)]
pub struct SafetyAlert {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub message: String,
    pub source_url: Option<String>,
}

/// GET /drugs/{cis}/safety
///
/// Returns safety alerts for a drug from the `safety_alerts` table.
///
/// - **404** if CIS does not exist in the database
/// - **200** with `data_available: false` if no alerts are stored
/// - **200** with alert list if data exists
#[utoipa::path(
    get,
    path = "/drugs/{cis}/safety",
    params(
        ("cis" = String, Path, description = "CIS drug identifier")
    ),
    responses(
        (status = 200, description = "Safety alerts found", body = SafetyResponse),
        (status = 404, description = "CIS not found in database"),
    ),
    tag = "bdpm-ingest"
)]
pub async fn drug_safety(
    Path(cis): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SafetyResponse>, StatusCode> {
    let cis_owned = cis.clone();

    let alerts = tokio::task::spawn_blocking(move || -> Option<Vec<SafetyAlert>> {
        let conn = crate::db::open_api_conn(&state.db_path).ok()?;

        // Verify CIS exists
        let cis_exists: bool = conn.query_row(
            "SELECT 1 FROM drugs WHERE cis = ?1 LIMIT 1",
            [&cis_owned],
            |_| Ok(true),
        ).unwrap_or(false);

        if !cis_exists {
            return None;
        }

        // Query safety alerts
        let mut stmt = conn.prepare(
            "SELECT start_date, end_date, message_plain, source_url
             FROM safety_alerts WHERE cis = ?1
             ORDER BY start_date DESC"
        ).ok()?;

        let rows = stmt.query_map([&cis_owned], |row| {
            Ok(SafetyAlert {
                start_date: row.get(0).ok(),
                end_date: row.get(1).ok(),
                message: row.get::<_, String>(2).unwrap_or_default(),
                source_url: row.get(3).ok(),
            })
        }).ok()?;

        let alerts: Vec<SafetyAlert> = rows.filter_map(|r| r.ok()).collect();
        Some(alerts)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match alerts {
        None => Err(StatusCode::NOT_FOUND),
        Some(alerts) => {
            let data_available = !alerts.is_empty();
            Ok(Json(SafetyResponse {
                cis,
                data_available,
                alerts,
            }))
        }
    }
}