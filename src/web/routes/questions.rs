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

// ── Questions ──────────────────────────────────────────────────────────────

pub async fn list_by_phase(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let questions = queries::questions::list_by_phase(&s.db, phase_id).await?;
    Ok(Json(serde_json::json!(questions)))
}

#[derive(Deserialize)]
pub struct CreateQuestion {
    pub text: String,
    pub question_type: String,
    pub position: Option<i64>,
    pub required: Option<bool>,
}

pub async fn create(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
    Json(body): Json<CreateQuestion>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let position = body.position.unwrap_or(0);
    let required = body.required.unwrap_or(true);
    let id = queries::questions::create(
        &s.db,
        phase_id,
        &body.text,
        &body.question_type,
        position,
        required,
    )
    .await?;
    let question = queries::questions::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::Other("question not found after create".into()))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(question))))
}

#[derive(Deserialize)]
pub struct UpdateQuestion {
    pub text: String,
    pub question_type: String,
    pub position: i64,
    pub required: bool,
}

pub async fn update(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateQuestion>,
) -> Result<Json<serde_json::Value>> {
    queries::questions::update(
        &s.db,
        id,
        &body.text,
        &body.question_type,
        body.position,
        body.required,
    )
    .await?;
    let question = queries::questions::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("question {id} not found")))?;
    Ok(Json(serde_json::json!(question)))
}

pub async fn delete(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    queries::questions::delete(&s.db, id).await?;
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
    queries::questions::reorder(&s.db, &items).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Options ────────────────────────────────────────────────────────────────

pub async fn list_options(
    State(s): State<WebState>,
    Path(question_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let options = queries::questions::list_options(&s.db, question_id).await?;
    Ok(Json(serde_json::json!(options)))
}

#[derive(Deserialize)]
pub struct CreateOption {
    pub label: String,
    pub value: String,
    pub position: Option<i64>,
}

pub async fn create_option(
    State(s): State<WebState>,
    Path(question_id): Path<i64>,
    Json(body): Json<CreateOption>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let position = body.position.unwrap_or(0);
    let id =
        queries::questions::create_option(&s.db, question_id, &body.label, &body.value, position)
            .await?;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": id, "question_id": question_id, "label": body.label, "value": body.value, "position": position })),
    ))
}

#[derive(Deserialize)]
pub struct UpdateOption {
    pub label: String,
    pub value: String,
    pub position: i64,
}

pub async fn update_option(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateOption>,
) -> Result<StatusCode> {
    queries::questions::update_option(&s.db, id, &body.label, &body.value, body.position).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_option(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    queries::questions::delete_option(&s.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
