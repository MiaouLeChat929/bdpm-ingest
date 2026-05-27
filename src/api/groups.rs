use crate::api::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// Whitelist-based sort helper — safely builds ORDER BY clause from user input
fn sort_clause(sort: Option<&str>, order: Option<&str>, allowed: &[(&str, &str)]) -> String {
    let col = sort
        .and_then(|s| allowed.iter().find(|(k, _)| *k == s))
        .map(|(_, v)| *v)
        .unwrap_or(allowed[0].1);
    let dir = match order {
        Some("desc") => "DESC",
        _ => "ASC",
    };
    format!("ORDER BY {} {}", col, dir)
}

#[derive(Deserialize, IntoParams)]
pub struct GroupListParams {
    pub q: Option<String>,
    pub sort: Option<String>,
    pub order: Option<String>,
    #[serde(default = "default_group_limit")]
    pub limit: usize,
}

#[derive(Deserialize, IntoParams)]
pub struct GroupDetailParams {
    pub sort: Option<String>,
    pub order: Option<String>,
}

fn default_group_limit() -> usize {
    100
}

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
    params(
        ("q" = Option<String>, Query, description = "Filter by group name (substring match)"),
        ("sort" = Option<String>, Query, description = "Sort by: group_id, group_name, cis_count"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc"),
        ("limit" = Option<usize>, Query, description = "Max results (default 100)")
    ),
    responses(
        (status = 200, description = "List of generic groups", body = Vec<GenericGroupList>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn list_generic_groups(
    Query(params): Query<GroupListParams>,
    State(state): State<AppState>,
) -> Result<Json<Vec<GenericGroupList>>, GroupError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<GenericGroupList>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;

        let allowed = [
            ("group_id", "group_id"),
            ("group_name", "group_name"),
            ("cis_count", "cis_count"),
        ];
        let order_by = sort_clause(params.sort.as_deref(), params.order.as_deref(), &allowed);

        let (sql, has_filter) = if params.q.is_some() {
            let filter_sql = "SELECT group_id, group_name, COUNT(cis) as cis_count
                             FROM generic_groups
                             WHERE group_name LIKE ?1
                             GROUP BY group_id, group_name";
            (filter_sql, true)
        } else {
            let filter_sql = "SELECT group_id, group_name, COUNT(cis) as cis_count
                             FROM generic_groups
                             GROUP BY group_id, group_name";
            (filter_sql, false)
        };
        let sql = format!("{} {}", sql, order_by);
        let limit = params.limit as i64;
        let sql = format!("{} LIMIT ?", sql);

        let mut stmt = conn.prepare(&sql)?;
        let rows = if has_filter {
            let q_pattern = format!("%{}%", params.q.as_ref().unwrap());
            stmt.query_map(params![&q_pattern, limit], |row| Ok(GenericGroupList {
                group_id: row.get(0)?,
                group_name: row.get(1)?,
                cis_count: row.get(2)?,
            }))?.collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![limit], |row| Ok(GenericGroupList {
                group_id: row.get(0)?,
                group_name: row.get(1)?,
                cis_count: row.get(2)?,
            }))?.collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }).await.map_err(|_| GroupError::Internal("Internal server error".to_string()))?
      .map_err(|_| GroupError::Internal("Internal server error".to_string()))?;
    Ok(Json(rows))
}

/// GET /generic-groups/:group_id — drugs in a specific generic group
#[utoipa::path(
    get,
    path = "/generic-groups/{group_id}",
    params(
        ("group_id" = String, Path, description = "Generic group ID"),
        ("sort" = Option<String>, Query, description = "Sort by: sort_order, name, type"),
        ("order" = Option<String>, Query, description = "Sort order: asc, desc")
    ),
    responses(
        (status = 200, description = "Members of the generic group", body = Vec<GenericGroupMember>)
    ),
    tag = "bdpm-ingest"
)]
pub async fn generic_group_detail(
    Path(group_id): Path<String>,
    Query(params): Query<GroupDetailParams>,
    State(state): State<AppState>,
) -> Result<Json<Vec<GenericGroupMember>>, GroupError> {
    let rows = tokio::task::spawn_blocking(move || -> Result<Vec<GenericGroupMember>, rusqlite::Error> {
        let conn = crate::db::open_api_conn(&state.db_path)?;

        let allowed = [
            ("sort_order", "g.sort_order"),
            ("name", "d.name"),
            ("type", "g.type"),
        ];
        let order_by = sort_clause(params.sort.as_deref(), params.order.as_deref(), &allowed);

        let sql = format!(
            "SELECT g.cis, d.name, g.type, g.sort_order, g.is_orphan
             FROM generic_groups g
             LEFT JOIN drugs d ON g.cis = d.cis
             WHERE g.group_id = ?1
             {}",
            order_by
        );
        let mut stmt = conn.prepare(&sql)?;
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