use axum::{Router, routing::get};
use std::path::PathBuf;

pub mod drugs;
pub mod groups;
pub mod search;
pub mod atc;
pub mod availability;

pub use drugs::drug_detail;
pub use search::search_drugs;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
}

/// Start the axum HTTP server on the given address.
pub async fn run_server(addr: &str, db_path: PathBuf) {
    let state = AppState { db_path };
    let app = Router::new()
        .route("/health", get(health))
        .route("/drugs", get(search::search_drugs))
        .route("/drugs/{cis}", get(drugs::drug_detail))
        .route("/generic-groups", get(groups::list_generic_groups))
        .route("/generic-groups/{group_id}", get(groups::generic_group_detail))
        .route("/atc", get(atc::atc_top_level))
        .route("/atc/{code}", get(atc::atc_detail))
        .route("/availability", get(availability::availability))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("Failed to bind address");
    axum::serve(listener, app).await.expect("Server error");
}

async fn health() -> &'static str {
    "OK"
}