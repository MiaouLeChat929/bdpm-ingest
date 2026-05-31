use axum::{extract::State, response::Json, Router, routing::get};
use serde::Serialize;
use std::path::PathBuf;

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
}

/// Health check endpoint — static response, no DB queries.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health", body = HealthResponse)
    ),
    tag = "bdpm-ingest"
)]
pub async fn health(_: State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub fn build_app(db_path: PathBuf) -> Router {
    let state = AppState { db_path };
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi::openapi_json))
        .route("/openapi.yaml", get(openapi::openapi_yaml))
        .route("/drugs", get(search::search_drugs))
        .route("/drugs/{cis}", get(drugs::drug_detail))
        .route("/drugs/{cis}/atc", get(drugs::drug_atc_codes))
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
/// All errors here are fatal startup errors — propagate is correct.
pub async fn run_server(addr: &str, db_path: PathBuf) {
    let app = build_app(db_path);
    #[expect(clippy::expect_used, reason = "Fatal startup error: TCP bind failure is unrecoverable")]
    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind address");
    #[expect(clippy::expect_used, reason = "Fatal startup error: server loop error terminates the process")]
    axum::serve(listener, app).await.expect("Server error");
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
