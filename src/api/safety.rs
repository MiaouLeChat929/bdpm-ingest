//! Safety information endpoint (stub).
//!
//! CIS_InfoImportantes data requires dedicated scraping from the BDPM website.
//! This is a minimal stub that validates the CIS exists and returns placeholder data.

use axum::{extract::{Path, State}, http::StatusCode, Json};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::cache::TtlCache;
use crate::api::AppState;

/// Safety info response (stub).
#[derive(Serialize, utoipa::ToSchema)]
pub struct SafetyResponse {
    pub cis: String,
    pub data_available: bool,
    pub message: String,
}

/// Cache for safety data — 6-hour TTL per BRIEF.md Phase 3.5.
pub type SafetyCache = TtlCache<String, SafetyData>;

/// Cached safety data.
#[derive(Clone, Serialize, Deserialize)]
pub struct SafetyData {
    pub cis: String,
    pub warnings: Vec<String>,
    pub contraindications: Vec<String>,
    pub pregnancy_category: Option<String>,
    pub breastfeeding: Option<String>,
    pub fetched_at: i64,  // Unix timestamp (seconds)
}

/// Check if a CIS exists in the database.
fn cis_exists(db_path: &std::path::Path, cis: &str) -> bool {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    conn.query_row(
        "SELECT 1 FROM drugs WHERE cis = ?1 LIMIT 1",
        [cis],
        |_| Ok(true),
    )
    .unwrap_or(false)
}

/// GET /drugs/{cis}/safety
///
/// Returns safety information for a drug.
///
/// - **404** if CIS does not exist in the database
/// - **200** with `data_available: false` if real data is not yet available
/// - **200** with actual safety data (future, when scraping is implemented)
pub async fn drug_safety(
    Path(cis): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<SafetyResponse>, StatusCode> {
    // Validate CIS exists
    let db_path = state.db_path.clone();
    let cis_for_check = cis.clone();

    let exists = tokio::task::spawn_blocking(move || cis_exists(&db_path, &cis_for_check))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !exists {
        return Err(StatusCode::NOT_FOUND);
    }

    // Check cache for existing data
    let cache = &state.safety_cache;
    if let Some(cached) = cache.get(&cis) {
        // Check if cached data is stale
        if cache.is_stale(&cis) {
            return Ok(Json(SafetyResponse {
                cis: cached.cis,
                data_available: true,
                message: "Stale data (cache expired, refetch pending)".to_string(),
            }));
        }
        return Ok(Json(SafetyResponse {
            cis: cached.cis,
            data_available: true,
            message: "Safety data from cache".to_string(),
        }));
    }

    // No cached data — return stub response
    Ok(Json(SafetyResponse {
        cis,
        data_available: false,
        message: "Safety data not yet available. This is a stub response.".to_string(),
    }))
}