use crate::api::AppState;
use super::sort_clause;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::params;
use serde::Deserialize;
use serde::Serialize;
use utoipa::{IntoParams, ToSchema};

const DEFAULT_ATC_SORT: &str = "atc_code";


#[derive(Deserialize, IntoParams)]
pub struct AtcDetailParams {
    pub sort: Option<String>,
    pub order: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AtcCode {
    pub atc_code: String,
    pub parent_1_char: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct AtcDetail {
    pub atc_code: String,
    pub parent_1_char: Option<String>,
    pub children: Vec<String>,
    pub drugs_count: i64,
}

pub enum AtcError {
    Internal(String),
}

impl IntoResponse for AtcError {
    fn into_response(self) -> Response {
        match self {
            AtcError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
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
) -> Result<Json<Vec<AtcCode>>, AtcError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<AtcCode>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT atc_code, parent_1_char FROM atc_codes WHERE LENGTH(atc_code) = 1 ORDER BY atc_code"
        )?;
        let rows = stmt.query_map([], |row| Ok(AtcCode {
            atc_code: row.get(0)?,
            parent_1_char: row.get(1)?,
        }))?;
        rows.collect::<Result<Vec<_>, _>>()
    }).await.map_err(|_| AtcError::Internal("Internal server error".to_string()))?
      .map_err(|_| AtcError::Internal("Internal server error".to_string()))?;
    Ok(Json(rows))
}

/// GET /atc/:code — ATC detail with child codes and drug count
#[utoipa::path(
    get,
    path = "/atc/{code}",
    params(
        ("code" = String, Path, description = "ATC code"),
        ("sort" = Option<String>, Query, description = "Sort children by: atc_code, drugs_count"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
        (status = 200, description = "ATC detail with children and drug count", body = AtcDetail)
    ),
    tag = "bdpm-ingest"
)]
pub async fn atc_detail(
    Path(code): Path<String>,
    Query(params): Query<AtcDetailParams>,
    State(state): State<AppState>,
) -> Result<Json<AtcDetail>, AtcError> {
    let detail = tokio::task::spawn_blocking(move || -> Result<AtcDetail, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;

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
            // For 7-char codes (leaf nodes), no children
            _ => 0,
        };
        let prefix = format!("{}%", atc_code);

        let allowed = [
            ("atc_code", "a.atc_code"),
            ("drugs_count", "COALESCE(m.drugs_count, 0)"),
        ];
        let order_by = sort_clause(params.sort.as_deref(), params.order.as_deref(), &allowed);

        let children: Vec<String> = if child_len > 0 {
            let children_sql = format!(
                "SELECT a.atc_code FROM atc_codes a \
                 LEFT JOIN (SELECT atc_code, COUNT(DISTINCT cis) as drugs_count \
                            FROM mitm WHERE atc_code LIKE ?1 GROUP BY atc_code) m \
                 ON a.atc_code = m.atc_code \
                 WHERE a.atc_code LIKE ?1 AND LENGTH(a.atc_code) = ?2 \
                 {}",
                order_by
            );
            let mut stmt = conn.prepare(&children_sql)?;
            let mapped = stmt.query_map(params![&prefix, child_len], |row| {
                row.get::<_, String>(0)
            })?;
            mapped.collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };

        // Drug count under this ATC (via mitm join)
        let drugs_count: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT cis) FROM mitm WHERE atc_code LIKE ?1",
            [&prefix],
            |row| row.get(0),
        ).unwrap_or(0);

        Ok(AtcDetail { atc_code, parent_1_char: parent, children, drugs_count })
    }).await.map_err(|_| AtcError::Internal("Internal server error".to_string()))?
      .map_err(|_| AtcError::Internal("Internal server error".to_string()))?;
    Ok(Json(detail))
}