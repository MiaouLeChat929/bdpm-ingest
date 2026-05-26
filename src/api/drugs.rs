use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
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
    pub presentations: Vec<Presentation>,
    pub compositions: Vec<Composition>,
}

#[derive(Serialize)]
pub struct Presentation {
    pub cip: String,
    pub cip_raw: Option<String>,
    pub labels: Option<String>,
    pub pres_status: Option<String>,
    pub prix_ville_cents: Option<i64>,
    pub prix_rate_cents: Option<i64>,
    pub ean13: Option<String>,
    pub reimbursable: Option<String>,
    pub reimb_rate: Option<f32>,
}

#[derive(Serialize)]
pub struct Composition {
    pub substance_name: String,
    pub dosage: Option<String>,
    pub pharm_code: String,
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
pub async fn drug_detail(
    Path(cis): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DrugDetail>, ApiError> {
    let cis = cis.trim().to_string();

    let detail = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&state.db_path)
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        // Drug row
        let drug = conn.query_row(
            "SELECT cis, name, form, route, auth_status, procedure_type,
                    auth_date, lab_name, is_patent, atc_code
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
                presentations: Vec::new(),
                compositions: Vec::new(),
            }),
        ).map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => ApiError::NotFound(format!("CIS {cis} not found")),
            _ => ApiError::Internal(e.to_string()),
        })?;

        // Presentations
        let mut stmt = conn.prepare(
            "SELECT cip, cip_raw, labels, pres_status, prix_ville_cents,
                    prix_rate_cents, ean13, reimbursable, reimb_rate
             FROM presentations WHERE cis = ?1"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let presentations = stmt.query_map([&cis], |row| Ok(Presentation {
            cip: row.get(0)?,
            cip_raw: row.get(1)?,
            labels: row.get(2)?,
            pres_status: row.get(3)?,
            prix_ville_cents: row.get(4)?,
            prix_rate_cents: row.get(5)?,
            ean13: row.get(6)?,
            reimbursable: row.get(7)?,
            reimb_rate: row.get(8)?,
        })).map_err(|e| ApiError::Internal(e.to_string()))?
          .filter_map(|r| r.ok()).collect();

        // Compositions
        let mut stmt = conn.prepare(
            "SELECT substance_name, dosage, pharm_code FROM compositions WHERE cis = ?1"
        ).map_err(|e| ApiError::Internal(e.to_string()))?;
        let compositions = stmt.query_map([&cis], |row| Ok(Composition {
            substance_name: row.get(0)?,
            dosage: row.get(1)?,
            pharm_code: row.get::<_, String>(2).unwrap_or_default(),
        })).map_err(|e| ApiError::Internal(e.to_string()))?
          .filter_map(|r| r.ok()).collect();

        Ok(DrugDetail { presentations, compositions, ..drug })
    }).await.map_err(|e| ApiError::Internal(e.to_string()))??;

    Ok(Json(detail))
}