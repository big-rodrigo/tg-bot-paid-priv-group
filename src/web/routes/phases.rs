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
    pub rejection_text: Option<String>,
    pub clean_chat: Option<bool>,
}

pub async fn create(
    State(s): State<WebState>,
    Json(body): Json<CreatePhase>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    let phase_type = body.phase_type.as_deref().unwrap_or("normal");

    if phase_type != "normal" && phase_type != "invite" && phase_type != "payment" {
        return Err(AppError::Other("phase_type must be 'normal', 'invite', or 'payment'".into()));
    }

    // Pre-flight: reject immediately if an active payment phase already exists.
    if phase_type == "payment" && queries::phases::get_active_payment(&s.db).await?.is_some() {
        return Err(AppError::Other(
            "An active payment gate phase already exists. Delete it before creating a new one.".into(),
        ));
    }

    // If creating a payment phase, auto-insert it before any existing invite phases.
    let position = if phase_type == "payment" {
        let all = queries::phases::list_all(&s.db).await?;
        let first_invite_pos = all.iter()
            .filter(|p| p.phase_type == "invite")
            .map(|p| p.position)
            .min();
        if let Some(invite_pos) = first_invite_pos {
            let shifted: Vec<(i64, i64)> = all.iter()
                .filter(|p| p.phase_type == "invite")
                .map(|p| (p.id, p.position + 1))
                .collect();
            queries::phases::reorder(&s.db, &shifted).await?;
            invite_pos
        } else {
            body.position.unwrap_or(0)
        }
    } else {
        body.position.unwrap_or(0)
    };

    let id = queries::phases::create(
        &s.db,
        &body.name,
        body.description.as_deref(),
        position,
        phase_type,
        body.rejection_text.as_deref(),
        body.clean_chat.unwrap_or(false),
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
    pub rejection_text: Option<String>,
    pub clean_chat: Option<bool>,
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

    if phase_type != "normal" && phase_type != "invite" && phase_type != "payment" {
        return Err(AppError::Other("phase_type must be 'normal', 'invite', or 'payment'".into()));
    }

    // For payment phases, auto-correct position if it ended up after invite phases.
    let position = if phase_type == "payment" {
        let all = queries::phases::list_all(&s.db).await?;
        let first_invite_pos = all.iter()
            .filter(|p| p.phase_type == "invite")
            .map(|p| p.position)
            .min();
        if let Some(invite_pos) = first_invite_pos {
            if existing.position >= invite_pos {
                let shifted: Vec<(i64, i64)> = all.iter()
                    .filter(|p| p.phase_type == "invite")
                    .map(|p| (p.id, p.position + 1))
                    .collect();
                queries::phases::reorder(&s.db, &shifted).await?;
                invite_pos
            } else {
                body.position
            }
        } else {
            body.position
        }
    } else {
        body.position
    };

    queries::phases::update(
        &s.db,
        id,
        &body.name,
        body.description.as_deref(),
        position,
        body.active,
        phase_type,
        body.rejection_text.as_deref(),
        body.clean_chat.unwrap_or(existing.clean_chat),
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

/// Validates phase ordering: normal → payment → invite.
/// At most one active payment phase is allowed.
async fn validate_phase_ordering(s: &WebState) -> Result<()> {
    let phases = queries::phases::list_active(&s.db).await?;
    let mut seen_payment = false;
    let mut seen_invite = false;
    let mut payment_count = 0;

    for p in &phases {
        match p.phase_type.as_str() {
            "normal" => {
                if seen_payment {
                    return Err(AppError::Other(
                        "Normal phases cannot appear after a payment gate phase.".into(),
                    ));
                }
                if seen_invite {
                    return Err(AppError::Other(
                        "Normal phases cannot appear after invite phases.".into(),
                    ));
                }
            }
            "payment" => {
                payment_count += 1;
                if payment_count > 1 {
                    return Err(AppError::Other(
                        "Only one active payment gate phase is allowed.".into(),
                    ));
                }
                if seen_invite {
                    return Err(AppError::Other(
                        "Payment gate phase cannot appear after invite phases.".into(),
                    ));
                }
                seen_payment = true;
            }
            "invite" => {
                seen_invite = true;
            }
            _ => {}
        }
    }
    Ok(())
}
