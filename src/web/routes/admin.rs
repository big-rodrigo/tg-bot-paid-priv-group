use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{
    db::queries,
    error::{AppError, Result},
    web::state::WebState,
};

/// Manually trigger invite link delivery for a user.
/// Useful when the webhook failed or for admin overrides.
pub async fn send_invites(
    State(s): State<WebState>,
    Path(user_id): Path<i64>,
) -> Result<StatusCode> {
    let user = queries::users::get_by_id(&s.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id} not found")))?;

    crate::bot::user::invite::deliver_invites(
        s.bot.clone(),
        s.db.clone(),
        user.id,
        user.telegram_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Revoke all unused invite links for a user.
pub async fn revoke_links(
    State(s): State<WebState>,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let user = queries::users::get_by_id(&s.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id} not found")))?;

    crate::bot::group::invite_manager::revoke_unused_for_user(&s.bot, &s.db, user.id).await?;

    Ok(Json(serde_json::json!({
        "message": format!("Revoked unused links for user {}", user.telegram_id)
    })))
}
