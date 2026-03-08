use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::{
    db::queries,
    error::{AppError, Result},
    web::state::WebState,
};

pub async fn list(State(s): State<WebState>) -> Result<Json<serde_json::Value>> {
    let phases = queries::phases::list_all(&s.db).await?;
    Ok(Json(serde_json::json!(phases)))
}

#[derive(Deserialize)]
pub struct CreatePhase {
    pub name: String,
    pub description: Option<String>,
    pub position: Option<i64>,
}

pub async fn create(
    State(s): State<WebState>,
    Json(body): Json<CreatePhase>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let position = body.position.unwrap_or(0);
    let id = queries::phases::create(&s.db, &body.name, body.description.as_deref(), position).await?;
    let phase = queries::phases::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::Other("phase not found after create".into()))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(phase))))
}

#[derive(Deserialize)]
pub struct UpdatePhase {
    pub name: String,
    pub description: Option<String>,
    pub position: i64,
    pub active: bool,
}

pub async fn update(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePhase>,
) -> Result<Json<serde_json::Value>> {
    queries::phases::update(
        &s.db,
        id,
        &body.name,
        body.description.as_deref(),
        body.position,
        body.active,
    )
    .await?;
    let phase = queries::phases::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("phase {id} not found")))?;
    Ok(Json(serde_json::json!(phase)))
}

pub async fn delete(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    queries::phases::delete(&s.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReorderItem {
    pub id: i64,
    pub position: i64,
}

pub async fn reorder(
    State(s): State<WebState>,
    Json(body): Json<Vec<ReorderItem>>,
) -> Result<StatusCode> {
    let items: Vec<(i64, i64)> = body.into_iter().map(|i| (i.id, i.position)).collect();
    queries::phases::reorder(&s.db, &items).await?;
    Ok(StatusCode::NO_CONTENT)
}
