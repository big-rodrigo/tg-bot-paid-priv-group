use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use teloxide::{requests::Requester, types::FileId, prelude::Request};

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

    let l = *s.lang.read().await;
    crate::bot::user::invite::deliver_invites(
        s.bot.clone(),
        s.db.clone(),
        user.id,
        user.telegram_id,
        l,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Reset a user's registration: clears answers, cancels pending payments,
/// and removes the registration row so they can /start fresh.
pub async fn reset_registration(
    State(s): State<WebState>,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let user = queries::users::get_by_id(&s.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id} not found")))?;

    queries::users::reset_registration(&s.db, user.id).await?;

    Ok(Json(serde_json::json!({
        "message": format!("Registration reset for user {}", user.telegram_id)
    })))
}

/// Fully unregister a user: reset registration + delete all invite links and payments.
pub async fn unregister(
    State(s): State<WebState>,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>> {
    let user = queries::users::get_by_id(&s.db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("user {user_id} not found")))?;

    // Revoke active links on Telegram before deleting DB records
    let _ = crate::bot::group::invite_manager::revoke_unused_for_user(&s.bot, &s.db, user.id).await;

    queries::users::unregister(&s.db, user.id).await?;

    Ok(Json(serde_json::json!({
        "message": format!("User {} has been fully unregistered", user.telegram_id)
    })))
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

#[derive(Deserialize)]
pub struct ImageQuery {
    pub file_id: String,
}

/// Proxy endpoint to download a Telegram image by file_id.
/// Calls bot.get_file() then fetches the bytes from Telegram's file API.
pub async fn telegram_image(
    State(s): State<WebState>,
    Query(params): Query<ImageQuery>,
) -> Result<impl IntoResponse> {
    let file = s.bot.get_file(FileId(params.file_id)).send().await?;

    let url = format!(
        "https://api.telegram.org/file/bot{}/{}",
        s.config.bot_api_key, file.path
    );

    let response = reqwest::get(&url).await?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = response.bytes().await?;

    Ok(([(header::CONTENT_TYPE, content_type)], bytes))
}
