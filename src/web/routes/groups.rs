use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    db::models::Group,
    db_execute, db_query_as,
    error::{AppError, Result},
    web::state::WebState,
};

pub async fn list(State(s): State<WebState>) -> Result<Json<serde_json::Value>> {
    let groups = db_query_as!(&s.db, Group, "SELECT * FROM groups ORDER BY id ASC", [], fetch_all)?;
    Ok(Json(serde_json::json!(groups)))
}

#[derive(Deserialize)]
pub struct CreateGroup {
    pub telegram_id: i64,
    pub title: String,
}

pub async fn create(
    State(s): State<WebState>,
    Json(body): Json<CreateGroup>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let (id,): (i64,) = db_query_as!(&s.db, (i64,),
        "INSERT INTO groups (telegram_id, title) VALUES (?, ?) RETURNING id",
        [body.telegram_id, &body.title], fetch_one)?;

    let group = db_query_as!(&s.db, Group, "SELECT * FROM groups WHERE id = ?", [id], fetch_one)?;

    Ok((StatusCode::CREATED, Json(serde_json::json!(group))))
}

#[derive(Deserialize)]
pub struct UpdateGroup {
    pub title: String,
    pub active: bool,
}

pub async fn update(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateGroup>,
) -> Result<Json<serde_json::Value>> {
    db_execute!(&s.db, "UPDATE groups SET title = ?, active = ? WHERE id = ?", [&body.title, body.active, id])?;

    let group = db_query_as!(&s.db, Group, "SELECT * FROM groups WHERE id = ?", [id], fetch_optional)?
        .ok_or_else(|| AppError::NotFound(format!("group {id} not found")))?;

    Ok(Json(serde_json::json!(group)))
}

pub async fn delete(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    db_execute!(&s.db, "DELETE FROM groups WHERE id = ?", [id])?;
    Ok(StatusCode::NO_CONTENT)
}
