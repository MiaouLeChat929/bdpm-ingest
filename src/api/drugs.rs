use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct DrugDetail {
    pub cis: String,
    pub name: String,
    pub form: Option<String>,
    pub route: Option<String>,
    pub auth_status: Option<String>,
    pub procedure_type: Option<String>,
    pub auth_date: Option<String>,
    pub lab_name: Option<String>,
    pub is_patent: bool,
    pub atc_code: Option<String>,
    pub comm_status: Option<String>,
    pub presentations: Vec<Presentation>,
    pub compositions: Vec<Composition>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Presentation {
    pub cip: String,
    pub cip_raw: Option<String>,
    pub labels: Option<String>,
    pub labels_clean: Option<String>,
    pub pres_status: Option<String>,
    pub comm_status: Option<String>,
    pub comm_date: Option<String>,
    pub prix_ht_cents: Option<i64>,
    pub prix_ville_cents: Option<i64>,
    pub prix_rate_cents: Option<i64>,
    pub ean13: Option<String>,
    pub reimbursable: Option<String>,
    pub reimb_rate: Option<f64>,
    pub is_orphan: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Composition {
    pub form_label: Option<String>,
    pub substance_code: String,
    pub substance_name: String,
    pub dosage: Option<String>,
    pub per_unit: Option<String>,
    pub pharm_code: String,
    pub seq: i32,
    pub substance_name_clean: Option<String>,
    pub dosage_mg: Option<f64>,
    pub is_orphan: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DrugAtcCode {
    pub atc_code: String,
    pub detail_url: Option<String>,
    pub parent_5_char: Option<String>,
    pub parent_3_char: Option<String>,
    pub parent_1_char: Option<String>,
}

pub enum ApiError {
    NotFound(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
        }
    }
}

/// GET /drugs/:cis — Return full drug detail including presentations and compositions.
#[utoipa::path(
    get,
    path = "/drugs/{cis}",
    params(
        ("cis" = String, Path, description = "Drug CIS code")
    ),
    responses(
        (status = 200, description = "Drug detail", body = DrugDetail),
        (status = 404, description = "Drug not found")
    ),
    tag = "bdpm-ingest"
)]
pub async fn drug_detail(
    Path(cis): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DrugDetail>, ApiError> {
    let cis = cis.trim().to_string();

    let detail = tokio::task::spawn_blocking(move || {
        let conn = crate::db::open_api_conn(&state.db_path)
            .map_err(|_| ApiError::Internal("Internal server error".to_string()))?;

        // Drug row
        let drug = conn.query_row(
            "SELECT cis, name, form, route, auth_status, procedure_type,
                    auth_date, lab_name, is_patent, atc_code, comm_status
             FROM drugs WHERE cis = ?1",
            [&cis],
            |row| Ok(DrugDetail {
                cis: row.get(0)?,
                name: row.get(1)?,
                form: row.get(2)?,
                route: row.get(3)?,
                auth_status: row.get(4)?,
                procedure_type: row.get(5)?,
                auth_date: row.get(6)?,
                lab_name: row.get(7)?,
                is_patent: row.get::<_, i32>(8)? != 0,
                atc_code: row.get(9)?,
                comm_status: row.get(10)?,
                presentations: Vec::new(),
                compositions: Vec::new(),
            }),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ApiError::NotFound(format!("CIS {cis} not found")),
            _ => ApiError::Internal("Internal server error".to_string()),
        })?;

        // Presentations
        let mut stmt = conn.prepare(
            "SELECT cip, cip_raw, labels, labels_clean, pres_status, comm_status, comm_date,
                   prix_ht_cents, prix_ville_cents, prix_rate_cents, ean13, reimbursable,
                   reimb_rate, is_orphan
             FROM presentations WHERE cis = ?1"
        ).map_err(|_| ApiError::Internal("Internal server error".to_string()))?;
        let presentations = stmt.query_map([&cis], |row| Ok(Presentation {
            cip: row.get(0)?,
            cip_raw: row.get(1)?,
            labels: row.get(2)?,
            labels_clean: row.get(3)?,
            pres_status: row.get(4)?,
            comm_status: row.get(5)?,
            comm_date: row.get(6)?,
            prix_ht_cents: row.get(7)?,
            prix_ville_cents: row.get(8)?,
            prix_rate_cents: row.get(9)?,
            ean13: row.get(10)?,
            reimbursable: row.get(11)?,
            reimb_rate: row.get(12)?,
            is_orphan: row.get::<_, i32>(13)? != 0,
        })).map_err(|_| ApiError::Internal("Internal server error".to_string()))?
          .filter_map(|r| r.ok()).collect();

        // Compositions
        let mut stmt = conn.prepare(
            "SELECT form_label, substance_code, substance_name, dosage, per_unit, pharm_code,
                   seq, substance_name_clean, dosage_mg, is_orphan
             FROM compositions WHERE cis = ?1"
        ).map_err(|_| ApiError::Internal("Internal server error".to_string()))?;
        let compositions = stmt.query_map([&cis], |row| Ok(Composition {
            form_label: row.get(0)?,
            substance_code: row.get(1)?,
            substance_name: row.get(2)?,
            dosage: row.get(3)?,
            per_unit: row.get(4)?,
            pharm_code: row.get(5)?,
            seq: row.get(6)?,
            substance_name_clean: row.get(7)?,
            dosage_mg: row.get(8)?,
            is_orphan: row.get::<_, i32>(9)? != 0,
        })).map_err(|_| ApiError::Internal("Internal server error".to_string()))?
          .filter_map(|r| r.ok()).collect();

        Ok(DrugDetail { presentations, compositions, ..drug })
    }).await.map_err(|_| ApiError::Internal("Internal server error".to_string()))??;

    Ok(Json(detail))
}

/// GET /drugs/:cis/atc — Return all ATC codes for a drug via the mitm table.
#[utoipa::path(
    get,
    path = "/drugs/{cis}/atc",
    params(
        ("cis" = String, Path, description = "Drug CIS code")
    ),
    responses(
        (status = 200, description = "ATC codes for this drug", body = Vec<DrugAtcCode>),
        (status = 404, description = "Drug not found")
    ),
    tag = "bdpm-ingest"
)]
pub async fn drug_atc_codes(
    Path(cis): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<DrugAtcCode>>, ApiError> {
    let cis = cis.trim().to_string();
    let cis_for_error = cis.clone();
    let db_path = state.db_path.clone();

    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<DrugAtcCode>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&db_path)?;

        // Verify drug exists
        let exists: bool = conn.query_row(
            "SELECT 1 FROM drugs WHERE cis = ?1",
            [&cis],
            |_| Ok(true),
        ).unwrap_or(false);

        if !exists {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }

        let mut stmt = conn.prepare(
            "SELECT m.atc_code, m.detail_url, a.parent_5_char, a.parent_3_char, a.parent_1_char
             FROM mitm m
             LEFT JOIN atc_codes a ON m.atc_code = a.atc_code
             WHERE m.cis = ?1
             ORDER BY m.atc_code"
        )?;

        let rows = stmt.query_map([&cis], |row| Ok(DrugAtcCode {
            atc_code: row.get(0)?,
            detail_url: row.get(1)?,
            parent_5_char: row.get(2)?,
            parent_3_char: row.get(3)?,
            parent_1_char: row.get(4)?,
        }))?;

        rows.collect()
    }).await.map_err(|_| ApiError::Internal("Internal server error".to_string()))?
      .map_err(|e| match e {
          rusqlite::Error::QueryReturnedNoRows => ApiError::NotFound(format!("CIS {cis_for_error} not found")),
          _ => ApiError::Internal("Internal server error".to_string()),
      })?;

    Ok(Json(rows))
}
