use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;

use crate::{
    db::queries,
    error::Result,
    payment::WebhookEvent,
    web::state::WebState,
};

#[derive(Deserialize)]
pub struct PaymentFilter {
    pub status: Option<String>,
}

pub async fn list(
    State(s): State<WebState>,
    Query(f): Query<PaymentFilter>,
) -> Result<Json<serde_json::Value>> {
    let payments = queries::payments::list(&s.db, f.status.as_deref()).await?;
    Ok(Json(serde_json::json!(payments)))
}

/// External payment provider webhook handler.
/// This endpoint does NOT require Basic auth — it's verified by the payment provider's
/// signature (Bearer token in `Authorization` header).
pub async fn webhook(
    State(s): State<WebState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode> {
    let event = s.payment_provider.verify_webhook(&headers, &body).await?;

    match event {
        WebhookEvent::Completed { external_ref, payload, amount, currency } => {
            let payment = queries::payments::complete_external(
                &s.db,
                &external_ref,
                &payload,
                amount,
                currency.as_deref(),
            )
            .await?;

            if let Some(payment) = payment {
                let user = queries::users::get_by_id(&s.db, payment.user_id).await?;
                if let Some(user) = user {
                    tracing::info!(
                        "Payment {} completed for user {} — delivering invites",
                        payment.id,
                        user.telegram_id
                    );
                    // Spawn so the webhook returns quickly
                    let bot = s.bot.clone();
                    let pool = s.db.clone();
                    let lang = s.lang.clone();
                    tokio::spawn(async move {
                        let l = *lang.read().await;
                        if let Err(e) = crate::bot::user::invite::deliver_invites(
                            bot,
                            pool,
                            user.id,
                            user.telegram_id,
                            l,
                        )
                        .await
                        {
                            tracing::error!("Failed to deliver invites: {e}");
                        }
                    });
                }
            }
        }
        WebhookEvent::Failed { external_ref, reason } => {
            tracing::warn!("Payment {external_ref} failed: {reason}");
            sqlx::query(
                "UPDATE payments SET status = 'failed', updated_at = CURRENT_TIMESTAMP WHERE external_ref = ?",
            )
            .bind(&external_ref)
            .execute(&s.db)
            .await?;
        }
        WebhookEvent::Refunded { external_ref } => {
            sqlx::query(
                "UPDATE payments SET status = 'refunded', updated_at = CURRENT_TIMESTAMP WHERE external_ref = ?",
            )
            .bind(&external_ref)
            .execute(&s.db)
            .await?;
        }
        WebhookEvent::Unknown => {
            tracing::debug!("Received unknown webhook event — ignoring");
        }
    }

    Ok(StatusCode::OK)
}
