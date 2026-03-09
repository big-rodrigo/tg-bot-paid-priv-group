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
    pub phase_type: Option<String>,
}

pub async fn create(
    State(s): State<WebState>,
    Json(body): Json<CreatePhase>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let position = body.position.unwrap_or(0);
    let phase_type = body.phase_type.as_deref().unwrap_or("normal");

    if phase_type != "normal" && phase_type != "invite" {
        return Err(AppError::Other("phase_type must be 'normal' or 'invite'".into()));
    }

    let id = queries::phases::create(
        &s.db,
        &body.name,
        body.description.as_deref(),
        position,
        phase_type,
    )
    .await?;

    validate_phase_ordering(&s).await?;

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
    pub phase_type: Option<String>,
}

pub async fn update(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePhase>,
) -> Result<Json<serde_json::Value>> {
    let existing = queries::phases::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("phase {id} not found")))?;

    let phase_type = body.phase_type.as_deref().unwrap_or(&existing.phase_type);

    if phase_type != "normal" && phase_type != "invite" {
        return Err(AppError::Other("phase_type must be 'normal' or 'invite'".into()));
    }

    queries::phases::update(
        &s.db,
        id,
        &body.name,
        body.description.as_deref(),
        body.position,
        body.active,
        phase_type,
    )
    .await?;

    validate_phase_ordering(&s).await?;

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

    validate_phase_ordering(&s).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Validates that no active normal phase appears after any active invite phase.
async fn validate_phase_ordering(s: &WebState) -> Result<()> {
    let phases = queries::phases::list_active(&s.db).await?;
    let mut seen_invite = false;
    for p in &phases {
        if p.phase_type == "invite" {
            seen_invite = true;
        } else if p.phase_type == "normal" && seen_invite {
            return Err(AppError::Other(
                "Normal phases cannot appear after invite phases. Move all invite phases to the end.".into(),
            ));
        }
    }
    Ok(())
}
