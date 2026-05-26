use crate::api::AppState;
use axum::{extract::{Query, State}, response::IntoResponse, Json};
use rusqlite::{params, Connection};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(serde::Serialize, Clone)]
pub struct DrugSearchResult {
    pub cis: String,
    pub name: String,
    pub form: Option<String>,
    pub lab_name: Option<String>,
}

/// FTS5-powered drug search endpoint.
///
/// Returns up to `limit` drugs matching the query string.
/// Uses `spawn_blocking` because rusqlite is blocking, not async.
pub async fn search_drugs(
    Query(params): Query<SearchParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let q = params.q.trim();
    if q.is_empty() {
        return Json(vec![] as Vec<DrugSearchResult>);
    }

    // Build FTS5 prefix query for partial matching
    let fts_query = format!("{}*", q);
    let limit = params.limit as i64;

    let results = tokio::task::spawn_blocking(move || -> Vec<DrugSearchResult> {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let mut stmt = match conn.prepare(
            "SELECT cis, name, form, lab_name FROM drugs_fts WHERE drugs_fts MATCH ?1 LIMIT ?2"
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let query_result: Vec<DrugSearchResult> = stmt.query_map(params![&fts_query, limit], |row| {
            Ok(DrugSearchResult {
                cis: row.get(0)?,
                name: row.get(1)?,
                form: row.get(2).ok(),
                lab_name: row.get(3).ok(),
            })
        }).map(|iter| iter.filter_map(|r| r.ok()).collect()).unwrap_or_default();

        query_result
    })
    .await
    .unwrap_or_default();

    Json(results)
}