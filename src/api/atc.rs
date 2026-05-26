use crate::api::AppState;
use axum::{extract::{Path, State}, Json};
use rusqlite::{Connection, params};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct AtcCode {
    pub atc_code: String,
    pub parent_1_char: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AtcDetail {
    pub atc_code: String,
    pub parent_1_char: Option<String>,
    pub children: Vec<String>,
    pub drugs_count: i64,
}

/// GET /atc — top-level ATC codes (1-char chapters)
#[utoipa::path(
    get,
    path = "/atc",
    responses(
        (status = 200, description = "Top-level ATC chapters", body = Vec<AtcCode>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn atc_top_level(
    State(state): State<AppState>,
) -> Json<Vec<AtcCode>> {
    let rows = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&state.db_path).unwrap();
        let mut stmt = conn.prepare(
            "SELECT atc_code, parent_1_char FROM atc_codes WHERE LENGTH(atc_code) = 1 ORDER BY atc_code"
        ).unwrap();
        stmt.query_map([], |row| Ok(AtcCode {
            atc_code: row.get(0)?,
            parent_1_char: row.get(1)?,
        })).unwrap().filter_map(|r| r.ok()).collect()
    }).await.unwrap();
    Json(rows)
}

/// GET /atc/:code — ATC detail with child codes and drug count
#[utoipa::path(
    get,
    path = "/atc/{code}",
    params(
        ("code" = String, Path, description = "ATC code")
    ),
    responses(
        (status = 200, description = "ATC detail with children and drug count", body = AtcDetail)
    ),
    tag = "bdpm-ingest"
)]
pub async fn atc_detail(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Json<AtcDetail> {
    let code_owned = code.clone();
    let detail = tokio::task::spawn_blocking(move || -> AtcDetail {
        let conn = match Connection::open(&state.db_path) {
            Ok(c) => c,
            Err(_) => return AtcDetail { atc_code: code.clone(), parent_1_char: None, children: vec![], drugs_count: 0 },
        };

        // Get ATC info (use code if not found in DB)
        let (atc_code, parent) = conn.query_row(
            "SELECT atc_code, parent_1_char FROM atc_codes WHERE atc_code = ?1",
            [&code],
            |row| Ok((row.get::<_, String>(0)?, row.get(1)?)),
        ).unwrap_or_else(|_| (code.clone(), None));

        // Child codes — next level down
        let child_len = match atc_code.len() {
            1 => 3,
            3 => 4,
            4 => 5,
            5 => 7,
            _ => return AtcDetail { atc_code, parent_1_char: parent, children: vec![], drugs_count: 0 },
        };
        let prefix = format!("{}%", atc_code);
        let mut stmt = conn.prepare(
            "SELECT atc_code FROM atc_codes WHERE atc_code LIKE ?1 AND LENGTH(atc_code) = ?2 ORDER BY atc_code"
        ).unwrap();
        let children = stmt.query_map(params![prefix, child_len], |row| {
            row.get::<_, String>(0)
        }).unwrap().filter_map(|r| r.ok()).collect();

        // Drug count under this ATC (via mitm join)
        let drugs_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT cis) FROM mitm WHERE atc_code LIKE ?1",
            [&prefix],
            |row| row.get(0),
        ).unwrap_or(0);

        AtcDetail { atc_code, parent_1_char: parent, children, drugs_count }
    }).await.unwrap_or(AtcDetail {
        atc_code: code_owned,
        parent_1_char: None,
        children: vec![],
        drugs_count: 0
    });
    Json(detail)
}