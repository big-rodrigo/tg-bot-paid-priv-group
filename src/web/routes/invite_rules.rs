use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::queries,
    error::{AppError, Result},
    web::state::WebState,
};

// ── Invite Rules ──────────────────────────────────────────────────────────

pub async fn list_by_phase(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let rules = queries::invite_rules::list_by_phase(&s.db, phase_id).await?;
    Ok(Json(serde_json::json!(rules)))
}

#[derive(Deserialize)]
pub struct CreateInviteRule {
    pub group_id: i64,
    pub position: Option<i64>,
}

pub async fn create(
    State(s): State<WebState>,
    Path(phase_id): Path<i64>,
    Json(body): Json<CreateInviteRule>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let phase = queries::phases::get_by_id(&s.db, phase_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("phase {phase_id} not found")))?;
    if phase.phase_type != "invite" {
        return Err(AppError::Other(
            "Invite rules can only be added to invite phases.".into(),
        ));
    }

    let position = body.position.unwrap_or(0);
    let id =
        queries::invite_rules::create(&s.db, phase_id, body.group_id, position).await?;
    let rule = queries::invite_rules::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::Other("invite rule not found after create".into()))?;
    Ok((StatusCode::CREATED, Json(serde_json::json!(rule))))
}

#[derive(Deserialize)]
pub struct UpdateInviteRule {
    pub group_id: i64,
    pub position: i64,
}

pub async fn update(
    State(s): State<WebState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateInviteRule>,
) -> Result<Json<serde_json::Value>> {
    queries::invite_rules::update(&s.db, id, body.group_id, body.position).await?;
    let rule = queries::invite_rules::get_by_id(&s.db, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("invite rule {id} not found")))?;
    Ok(Json(serde_json::json!(rule)))
}

pub async fn delete(State(s): State<WebState>, Path(id): Path<i64>) -> Result<StatusCode> {
    queries::invite_rules::delete(&s.db, id).await?;
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
    queries::invite_rules::reorder(&s.db, &items).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Conditions ────────────────────────────────────────────────────────────

pub async fn list_conditions(
    State(s): State<WebState>,
    Path(invite_rule_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let conditions = queries::invite_rules::list_conditions(&s.db, invite_rule_id).await?;
    Ok(Json(serde_json::json!(conditions)))
}

#[derive(Deserialize)]
pub struct CreateCondition {
    pub question_id: i64,
    pub condition_type: String,
    pub option_id: Option<i64>,
    pub text_value: Option<String>,
}

pub async fn create_condition(
    State(s): State<WebState>,
    Path(invite_rule_id): Path<i64>,
    Json(body): Json<CreateCondition>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let valid_types = [
        "option_selected",
        "option_not_selected",
        "text_contains",
        "text_not_contains",
    ];
    if !valid_types.contains(&body.condition_type.as_str()) {
        return Err(AppError::Other(format!(
            "Invalid condition_type. Must be one of: {}",
            valid_types.join(", ")
        )));
    }

    let id = queries::invite_rules::create_condition(
        &s.db,
        invite_rule_id,
        body.question_id,
        &body.condition_type,
        body.option_id,
        body.text_value.as_deref(),
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "invite_rule_id": invite_rule_id,
            "question_id": body.question_id,
            "condition_type": body.condition_type,
            "option_id": body.option_id,
            "text_value": body.text_value,
        })),
    ))
}

pub async fn delete_condition(
    State(s): State<WebState>,
    Path(id): Path<i64>,
) -> Result<StatusCode> {
    queries::invite_rules::delete_condition(&s.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ── Available questions for building conditions ───────────────────────────

#[derive(Serialize)]
pub struct AvailableQuestion {
    pub id: i64,
    pub phase_id: i64,
    pub phase_name: String,
    pub text: String,
    pub question_type: String,
    pub options: Vec<AvailableOption>,
}

#[derive(Serialize)]
pub struct AvailableOption {
    pub id: i64,
    pub label: String,
    pub value: String,
}

pub async fn available_questions(
    State(s): State<WebState>,
) -> Result<Json<Vec<AvailableQuestion>>> {
    let phases = queries::phases::list_active_normal(&s.db).await?;
    let mut result = Vec::new();

    for phase in &phases {
        let questions = queries::questions::list_by_phase(&s.db, phase.id).await?;
        for q in &questions {
            if q.question_type != "text" && q.question_type != "button" {
                continue;
            }

            let options = if q.question_type == "button" {
                queries::questions::list_options(&s.db, q.id)
                    .await?
                    .into_iter()
                    .map(|o| AvailableOption {
                        id: o.id,
                        label: o.label,
                        value: o.value,
                    })
                    .collect()
            } else {
                Vec::new()
            };

            result.push(AvailableQuestion {
                id: q.id,
                phase_id: phase.id,
                phase_name: phase.name.clone(),
                text: q.text.clone(),
                question_type: q.question_type.clone(),
                options,
            });
        }
    }

    Ok(Json(result))
}
