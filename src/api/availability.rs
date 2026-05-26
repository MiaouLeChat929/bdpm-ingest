use crate::api::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use utoipa::IntoParams;
use utoipa::ToSchema;

#[derive(Deserialize, IntoParams)]
pub struct AvailParams {
    pub cis: Option<String>,
    pub status: Option<i32>,
}

#[derive(Serialize, ToSchema)]
pub struct AvailabilityRow {
    pub cis: String,
    pub name: Option<String>,
    pub cip: Option<String>,
    pub status_type: i32,
    pub status: Option<String>,
    pub date_start: Option<String>,
    pub date_remise: Option<String>,
}

pub enum AvailabilityError {
    Internal(String),
}

impl IntoResponse for AvailabilityError {
    fn into_response(self) -> Response {
        match self {
            AvailabilityError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}

/// GET /availability — availability/sales status
/// Query params:
///   ?cis=XXX — availability for a specific drug
///   ?status=1 — all drugs in rupture (status_type=1)
///   no params — recent availability rows (limit 200)
#[utoipa::path(
    get,
    path = "/availability",
    params(
        ("cis" = Option<String>, Query, description = "Filter by CIS code"),
        ("status" = Option<i32>, Query, description = "Filter by status type")
    ),
    responses(
        (status = 200, description = "Availability rows", body = Vec<AvailabilityRow>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn availability(
    Query(params): Query<AvailParams>,
    State(state): State<AppState>,
) -> Result<Json<Vec<AvailabilityRow>>, AvailabilityError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<AvailabilityRow>, rusqlite::Error> {
        let conn = Connection::open(&state.db_path)?;

        let query_str: &str;
        let has_cis = params.cis.is_some();
        let has_status = params.status.is_some();

        let rows = if has_cis {
            query_str = "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                         FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                         WHERE a.cis = ?1
                         ORDER BY a.date_start DESC";
            let cis = params.cis.unwrap();
            let mut stmt = conn.prepare(query_str)?;
            let rows = stmt.query_map([cis], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                cip: row.get(2)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
            }))?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else if has_status {
            query_str = "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                         FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                         WHERE a.status_type = ?1
                         ORDER BY a.date_start DESC
                         LIMIT 200";
            let status = params.status.unwrap();
            let mut stmt = conn.prepare(query_str)?;
            let rows = stmt.query_map([status], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                cip: row.get(2)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
            }))?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            query_str = "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                         FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                         ORDER BY a.date_start DESC
                         LIMIT 200";
            let mut stmt = conn.prepare(query_str)?;
            let rows = stmt.query_map([], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
                cip: row.get(2)?,
            }))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }).await.map_err(|e| AvailabilityError::Internal(e.to_string()))?
      .map_err(|e| AvailabilityError::Internal(e.to_string()))?;
    Ok(Json(rows))
}