use crate::api::AppState;
use super::sort_clause;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};


#[derive(Deserialize, IntoParams)]
pub struct AvailParams {
    pub cis: Option<String>,
    pub status: Option<i32>,
    pub sort: Option<String>,
    pub order: Option<String>,
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
        ("status" = Option<i32>, Query, description = "Filter by status type"),
        ("sort" = Option<String>, Query, description = "Sort by: date_start, status_type, cis"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc")
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
        let conn = crate::db::open_api_conn(&state.db_path)?;

        let allowed = [
            ("date_start", "a.date_start"),
            ("status_type", "a.status_type"),
            ("cis", "a.cis"),
        ];
        let order_by = sort_clause(params.sort.as_deref(), params.order.as_deref(), &allowed);
        let has_cis = params.cis.is_some();
        let has_status = params.status.is_some();

        let base_select = "SELECT a.cis, d.name, a.cip, a.status_type, a.status, a.date_start, a.date_remise
                          FROM availability a LEFT JOIN drugs d ON a.cis = d.cis";

        let rows = if has_cis {
            let where_clause = "WHERE a.cis = ?1";
            let sql = format!("{} {} {}", base_select, where_clause, order_by);
            let cis = params.cis.clone().unwrap();
            let mut stmt = conn.prepare(&sql)?;
            let rows: Vec<AvailabilityRow> = stmt.query_map([cis], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                cip: row.get(2)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else if has_status {
            let where_clause = "WHERE a.status_type = ?1";
            let sql = format!("{} {} {} LIMIT 200", base_select, where_clause, order_by);
            let status = params.status.unwrap();
            let mut stmt = conn.prepare(&sql)?;
            let rows: Vec<AvailabilityRow> = stmt.query_map([status], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                cip: row.get(2)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            rows
        } else {
            let sql = format!("{} {} LIMIT 200", base_select, order_by);
            let mut stmt = conn.prepare(&sql)?;
            let rows: Vec<AvailabilityRow> = stmt.query_map([], |row| Ok(AvailabilityRow {
                cis: row.get(0)?,
                name: row.get(1)?,
                cip: row.get(2)?,
                status_type: row.get(3)?,
                status: row.get(4)?,
                date_start: row.get(5)?,
                date_remise: row.get(6)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            rows
        };
        Ok(rows)
    }).await.map_err(|_| AvailabilityError::Internal("Internal server error".to_string()))?
      .map_err(|_| AvailabilityError::Internal("Internal server error".to_string()))?;
    Ok(Json(rows))
}
