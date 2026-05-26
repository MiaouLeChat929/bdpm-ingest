use crate::api::AppState;
use axum::{extract::{Path, State}, Json};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct GenericGroupList {
    pub group_id: String,
    pub group_name: Option<String>,
    pub cis_count: i64,
}

#[derive(Serialize)]
pub struct GenericGroupMember {
    pub cis: String,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub sort_order: Option<i64>,
    pub is_orphan: bool,
}

/// GET /generic-groups — list all generic groups with CIS count
pub async fn list_generic_groups(
    State(state): State<AppState>,
) -> Json<Vec<GenericGroupList>> {
    let rows = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&state.db_path).unwrap();
        let mut stmt = conn.prepare(
            "SELECT group_id, group_name, COUNT(cis) as cis_count
             FROM generic_groups
             GROUP BY group_id, group_name
             ORDER BY group_id"
        ).unwrap();
        stmt.query_map([], |row| Ok(GenericGroupList {
            group_id: row.get(0)?,
            group_name: row.get(1)?,
            cis_count: row.get(2)?,
        })).unwrap().filter_map(|r| r.ok()).collect()
    }).await.unwrap();
    Json(rows)
}

/// GET /generic-groups/:group_id — drugs in a specific generic group
pub async fn generic_group_detail(
    Path(group_id): Path<String>,
    State(state): State<AppState>,
) -> Json<Vec<GenericGroupMember>> {
    let rows = tokio::task::spawn_blocking(move || {
        let conn = Connection::open(&state.db_path).unwrap();
        let mut stmt = conn.prepare(
            "SELECT g.cis, d.name, g.type, g.sort_order, g.is_orphan
             FROM generic_groups g
             LEFT JOIN drugs d ON g.cis = d.cis
             WHERE g.group_id = ?1
             ORDER BY g.sort_order, d.name"
        ).unwrap();
        stmt.query_map([&group_id], |row| Ok(GenericGroupMember {
            cis: row.get(0)?,
            name: row.get(1)?,
            type_: row.get(2)?,
            sort_order: row.get(3)?,
            is_orphan: row.get::<_, i32>(4).unwrap_or(0) != 0,
        })).unwrap().filter_map(|r| r.ok()).collect()
    }).await.unwrap();
    Json(rows)
}
