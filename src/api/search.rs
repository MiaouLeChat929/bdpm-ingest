use crate::api::AppState;
use super::sort_clause;
use axum::{extract::{Query, State}, response::IntoResponse, Json};
use rusqlite::params;
use serde::Deserialize;
use utoipa::IntoParams;


#[derive(Deserialize, IntoParams)]
pub struct SearchParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub sort: Option<String>,
    pub order: Option<String>,
}

fn default_limit() -> usize {
    20
}

/// Sanitizes user input for FTS5 query building.
/// FTS5 interprets special characters as operators: - + " * ( ) : ^ | ~
/// Stripping them prevents query errors and unexpected behavior.
fn sanitize_fts_query(input: &str) -> String {
    input
        .chars()
        .filter(|c| !matches!(c, '-' | '+' | '"' | '*' | '(' | ')' | ':' | '^' | '|' | '~'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_strips_fts_operators() {
        assert_eq!(sanitize_fts_query("-DOLIPRANE"), "DOLIPRANE");
        assert_eq!(sanitize_fts_query("test+more"), "testmore");
        assert_eq!(sanitize_fts_query("a\"b"), "ab");
        assert_eq!(sanitize_fts_query("a(b)c"), "abc");
        assert_eq!(sanitize_fts_query("col:value"), "colvalue");
        assert_eq!(sanitize_fts_query("normal search"), "normal search");
        assert_eq!(sanitize_fts_query("DOLIPRANE"), "DOLIPRANE");
        assert_eq!(sanitize_fts_query("CAFÉ"), "CAFÉ");
        assert_eq!(sanitize_fts_query(""), "");
    }
}
#[derive(serde::Serialize, utoipa::ToSchema, Clone)]
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
#[utoipa::path(
    get,
    path = "/drugs",
    params(
        ("q" = String, Query, description = "Search query"),
        ("limit" = Option<usize>, Query, description = "Max results (default 20)"),
        ("sort" = Option<String>, Query, description = "Sort by: name, form, lab_name, atc_code"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
        (status = 200, description = "List of matching drugs", body = Vec<DrugSearchResult>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn search_drugs(
    Query(params): Query<SearchParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let q = params.q.trim();
    if q.is_empty() {
        return Json(vec![] as Vec<DrugSearchResult>);
    }

    // Build FTS5 prefix query for partial matching
    // Sanitize user input to prevent FTS5 operator injection
    let sanitized = sanitize_fts_query(q);
    let fts_query = format!("{}*", sanitized);
    let limit = params.limit as i64;

    let results = tokio::task::spawn_blocking(move || -> Vec<DrugSearchResult> {
        let conn = match crate::db::open_api_conn(&state.db_path) {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        // Sort clause for results after FTS5 matching
        let allowed = [
            ("name", "d.name"),
            ("form", "d.form"),
            ("lab_name", "d.lab_name"),
            ("atc_code", "d.atc_code"),
        ];
        let order_by = sort_clause(params.sort.as_deref(), params.order.as_deref(), &allowed);

        let sql = format!(
            "SELECT d.cis, d.name, d.form, d.lab_name FROM drugs d \
             INNER JOIN drugs_fts fts ON d.cis = fts.cis WHERE drugs_fts MATCH ?1 {} LIMIT ?2",
            order_by
        );

        let mut stmt = match conn.prepare(&sql) {
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
    .unwrap_or_else(|e| {
        tracing::error!("Search task failed: {e}");
        vec![]
    });

    Json(results)
}