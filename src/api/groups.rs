use crate::api::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct GenericGroupList {
    pub group_id: String,
    pub group_name: Option<String>,
    pub cis_count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GenericGroupMember {
    pub cis: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub sort_order: Option<i64>,
    pub is_orphan: bool,
}

pub enum GroupError {
    Internal(String),
}

impl IntoResponse for GroupError {
    fn into_response(self) -> Response {
        match self {
            GroupError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response()
            }
        }
    }
}

/// GET /generic-groups — list all generic groups with CIS count
#[utoipa::path(
    get,
    path = "/generic-groups",
    responses(
        (status = 200, description = "List of generic groups", body = Vec<GenericGroupList>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn list_generic_groups(
    State(state): State<AppState>,
) -> Result<Json<Vec<GenericGroupList>>, GroupError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<GenericGroupList>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT group_id, group_name, COUNT(cis) as cis_count
             FROM generic_groups
             GROUP BY group_id, group_name
             ORDER BY group_id"
        )?;
        let rows = stmt.query_map([], |row| Ok(GenericGroupList {
            group_id: row.get(0)?,
            group_name: row.get(1)?,
            cis_count: row.get(2)?,
        }))?;
        rows.collect::<Result<Vec<_>, _>>()
    }).await.map_err(|_| GroupError::Internal("Internal server error".to_string()))?
      .map_err(|_| GroupError::Internal("Internal server error".to_string()))?;
    Ok(Json(rows))
}

/// GET /generic-groups/:group_id — drugs in a specific generic group
#[utoipa::path(
    get,
    path = "/generic-groups/{group_id}",
    params(
        ("group_id" = String, Path, description = "Generic group ID")
    ),
    responses(
        (status = 200, description = "Members of the generic group", body = Vec<GenericGroupMember>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn generic_group_detail(
    Path(group_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<Vec<GenericGroupMember>>, GroupError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<GenericGroupMember>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT g.cis, d.name, g.type, g.sort_order, g.is_orphan
             FROM generic_groups g
             LEFT JOIN drugs d ON g.cis = d.cis
             WHERE g.group_id = ?1
             ORDER BY g.sort_order, d.name"
        )?;
        let rows = stmt.query_map([&group_id], |row| Ok(GenericGroupMember {
            cis: row.get(0)?,
            name: row.get(1)?,
            type_: row.get(2)?,
            sort_order: row.get(3)?,
            is_orphan: row.get::<_, i32>(4).unwrap_or(0) != 0,
        }))?;
        rows.collect::<Result<Vec<_>, _>>()
    }).await.map_err(|_| GroupError::Internal("Internal server error".to_string()))?
      .map_err(|_| GroupError::Internal("Internal server error".to_string()))?;
    Ok(Json(rows))
}