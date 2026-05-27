use axum::{extract::State, http::StatusCode, response::Json, Router, routing::get};
use serde::Serialize;
use std::path::PathBuf;
use tokio::task::spawn_blocking;

pub mod atc;
pub mod availability;
pub mod drugs;
pub mod groups;
pub mod openapi;
pub mod safety;
pub mod search;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    status: &'static str,
    last_import: Option<String>,
    drug_count: i64,
}

pub fn build_app(db_path: PathBuf) -> Router {
    let state = AppState { db_path };
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi::openapi_json))
        .route("/openapi.yaml", get(openapi::openapi_yaml))
        .route("/drugs", get(search::search_drugs))
        .route("/drugs/{cis}", get(drugs::drug_detail))
        .route("/drugs/{cis}/safety", get(safety::drug_safety))
        .route("/generic-groups", get(groups::list_generic_groups))
        .route("/generic-groups/{group_id}", get(groups::generic_group_detail))
        .route("/atc", get(atc::atc_top_level))
        .route("/atc/{code}", get(atc::atc_detail))
        .route("/availability", get(availability::availability))
        .with_state(state)
}

/// Whitelist-based sort helper — safely builds ORDER BY clause from user input
pub(crate) fn sort_clause(sort: Option<&str>, order: Option<&str>, allowed: &[(&str, &str)]) -> String {
    let col = sort
        .and_then(|s| allowed.iter().find(|(k, _)| *k == s))
        .map(|(_, v)| *v)
        .unwrap_or(allowed[0].1);
    let dir = match order {
        Some("desc") => "DESC",
        _ => "ASC",
    };
    format!("ORDER BY {} {}", col, dir)
}

/// Start the axum HTTP server on the given address.
pub async fn run_server(addr: &str, db_path: PathBuf) {
    let app = build_app(db_path);
    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind address");
    axum::serve(listener, app).await.expect("Server error");
}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health", body = HealthResponse)
    ),
    tag = "bdpm-ingest"
)]
pub async fn health(State(state): State<AppState>) -> Result<Json<HealthResponse>, StatusCode> {
    let db_path = state.db_path.clone();

    let result = spawn_blocking(move || {
        let conn = crate::db::open_api_conn(&db_path)?;

        let last_import = conn
            .query_row(
                "SELECT imported_at FROM import_log WHERE status = 'success' ORDER BY imported_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok();

        let drug_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM drugs", [], |row| row.get(0))
            .unwrap_or(0);

        Ok::<(Option<String>, i64), rusqlite::Error>((last_import, drug_count))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(HealthResponse {
        status: "ok",
        last_import: result.0,
        drug_count: result.1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_clause_valid_sort() {
        let allowed = [("name", "d.name"), ("form", "d.form")];
        assert_eq!(sort_clause(Some("form"), None, &allowed), "ORDER BY d.form ASC");
    }

    #[test]
    fn test_sort_clause_invalid_sort_fallback() {
        let allowed = [("name", "d.name"), ("form", "d.form")];
        assert_eq!(sort_clause(Some("bogus"), None, &allowed), "ORDER BY d.name ASC");
    }

    #[test]
    fn test_sort_clause_desc() {
        let allowed = [("name", "d.name"), ("form", "d.form")];
        assert_eq!(sort_clause(Some("name"), Some("desc"), &allowed), "ORDER BY d.name DESC");
    }

    #[test]
    fn test_sort_clause_default_asc() {
        let allowed = [("name", "d.name")];
        assert_eq!(sort_clause(Some("name"), None, &allowed), "ORDER BY d.name ASC");
    }

    #[test]
    fn test_sort_clause_none_sort() {
        let allowed = [("name", "d.name"), ("form", "d.form")];
        assert_eq!(sort_clause(None, None, &allowed), "ORDER BY d.name ASC");
    }
}