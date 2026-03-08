use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    db::models::Group,
    error::{AppError, Result},
    web::state::WebState,
};

pub async fn list(State(s): State<WebState>) -> Result<Json<serde_json::Value>> {
    let groups = sqlx::query_as::<_, Group>("SELECT * FROM groups ORDER BY id ASC")
        .fetch_all(&s.db)
        .await?;
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
    let row = sqlx::query(
        "INSERT INTO groups (telegram_id, title) VALUES (?, ?)",
    )
    .bind(body.telegram_id)
    .bind(&body.title)
    .execute(&s.db)
    .await?;

    let id = row.last_insert_rowid();
    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_one(&s.db)
        .await?;

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
    sqlx::query("UPDATE groups SET title = ?, active = ? WHERE id = ?")
        .bind(&body.title)
        .bind(body.active)
        .bind(id)
        .execute(&s.db)
        .await?;

    let group = sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(id)
        .fetch_optional(&s.db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("group {id} not found")))?;

    Ok(Json(serde_json::json!(group)))
}

pub async fn delete(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    sqlx::query("DELETE FROM groups WHERE id = ?")
        .bind(id)
        .execute(&s.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
