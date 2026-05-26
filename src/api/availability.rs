use crate::api::AppState;
use axum::{extract::{Query, State}, Json};
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
pub struct AvailParams {
    pub cis: Option<String>,
    pub status: Option<i32>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct AvailabilityRow {
    pub cis: String,
    pub name: Option<String>,
    pub cip: Option<String>,
    pub status_type: i32,
    pub status: Option<String>,
    pub date_start: Option<String>,
    pub date_remise: Option<String>,
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
) -> Json<Vec<AvailabilityRow>> {
    let rows = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&state.db_path).unwrap();

        let rows: Vec<AvailabilityRow> = match (&params.cis, params.status) {
            (Some(cis), _) => {
                let mut stmt = conn.prepare(
                    "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                     FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                     WHERE a.cis = ?1
                     ORDER BY a.date_start DESC"
                ).unwrap();
                stmt.query_map([cis], |row| Ok(AvailabilityRow {
                    cis: row.get(0)?,
                    name: row.get(1)?,
                    cip: row.get(2)?,
                    status_type: row.get(3)?,
                    status: row.get(4)?,
                    date_start: row.get(5)?,
                    date_remise: row.get(6)?,
                })).unwrap().filter_map(|r| r.ok()).collect()
            }
            (None, Some(status)) => {
                let mut stmt = conn.prepare(
                    "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                     FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                     WHERE a.status_type = ?1
                     ORDER BY a.date_start DESC
                     LIMIT 200"
                ).unwrap();
                stmt.query_map([status], |row| Ok(AvailabilityRow {
                    cis: row.get(0)?,
                    name: row.get(1)?,
                    cip: row.get(2)?,
                    status_type: row.get(3)?,
                    status: row.get(4)?,
                    date_start: row.get(5)?,
                    date_remise: row.get(6)?,
                })).unwrap().filter_map(|r| r.ok()).collect()
            }
            _ => {
                let mut stmt = conn.prepare(
                    "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                     FROM availability a LEFT JOIN drugs d ON a.cis = d.cis
                     ORDER BY a.date_start DESC
                     LIMIT 200"
                ).unwrap();
                stmt.query_map([], |row| Ok(AvailabilityRow {
                    cis: row.get(0)?,
                    name: row.get(1)?,
                    cip: row.get(2)?,
                    status_type: row.get(3)?,
                    status: row.get(4)?,
                    date_start: row.get(5)?,
                    date_remise: row.get(6)?,
                })).unwrap().filter_map(|r| r.ok()).collect()
            }
        };
        rows
    }).await.unwrap();
    Json(rows)
}
