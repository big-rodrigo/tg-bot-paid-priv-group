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

pub async fn list(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let phase = queries::phases::get_by_id(&s.db, phase_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("phase {phase_id} not found")))?;
    if phase.phase_type != "payment" {
        return Err(AppError::Other("phase is not a payment gate phase".into()));
    }
    let conditions = queries::payment_gate::list_conditions(&s.db, phase_id).await?;
    Ok(Json(serde_json::json!(conditions)))
}

#[derive(Deserialize)]
pub struct CreateCondition {
    pub question_id: i64,
    pub condition_type: String,
    pub option_id: Option<i64>,
    pub text_value: Option<String>,
}

pub async fn create(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
    Json(body): Json<CreateCondition>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let phase = queries::phases::get_by_id(&s.db, phase_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("phase {phase_id} not found")))?;
    if phase.phase_type != "payment" {
        return Err(AppError::Other("phase is not a payment gate phase".into()));
    }

    let valid_types = ["option_selected", "option_not_selected", "text_contains", "text_not_contains"];
    if !valid_types.contains(&body.condition_type.as_str()) {
        return Err(AppError::Other(format!(
            "invalid condition_type '{}'. Must be one of: {}",
            body.condition_type,
            valid_types.join(", ")
        )));
    }

    let id = queries::payment_gate::create_condition(
        &s.db,
        phase_id,
        body.question_id,
        &body.condition_type,
        body.option_id,
        body.text_value.as_deref(),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}

pub async fn delete(
    State(s): State<WebState>,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    queries::payment_gate::delete_condition(&s.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
