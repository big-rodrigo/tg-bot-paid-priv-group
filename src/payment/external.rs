use async_trait::async_trait;
use axum::http::HeaderMap;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    config::AppConfig,
    db::models::User,
    error::{AppError, Result},
};

use super::{ExternalWebhookPayload, PaymentInitiation, PaymentProvider, WebhookEvent};

pub struct ExternalPaymentProvider {
    config: Arc<AppConfig>,
    client: reqwest::Client,
}

impl ExternalPaymentProvider {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct InitiateRequest {
    payment_id: i64,
    user_id: i64,
    telegram_id: i64,
    username: Option<String>,
    first_name: String,
}

#[derive(Deserialize)]
struct InitiateResponse {
    #[serde(rename = "ref")]
    reference: String,
    payment_url: Option<String>,
}

#[async_trait]
impl PaymentProvider for ExternalPaymentProvider {
    async fn initiate(&self, user: &User, payment_id: i64) -> Result<PaymentInitiation> {
        let (api_url, api_key) = match (&self.config.payment_api_url, &self.config.payment_api_key) {
            (Some(url), Some(key)) => (url.clone(), key.clone()),
            _ => {
                // External API not configured — return a stub response for development
                tracing::warn!("PAYMENT_API_URL/PAYMENT_API_KEY not configured, using stub");
                return Ok(PaymentInitiation {
                    external_ref: Some(format!("stub-{payment_id}")),
                    payment_url: None,
                    instructions: "External payment not configured. Contact support.".to_string(),
                });
            }
        };

        let body = InitiateRequest {
            payment_id,
            user_id: user.id,
            telegram_id: user.telegram_id,
            username: user.username.clone(),
            first_name: user.first_name.clone(),
        };

        let response = self
            .client
            .post(&api_url)
            .bearer_auth(&api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Payment(e.to_string()))?
            .json::<InitiateResponse>()
            .await?;

        let instructions = match &response.payment_url {
            Some(url) => format!("Please complete your payment at:\n{url}"),
            None => "Your payment request has been created. You will be notified once confirmed.".to_string(),
        };

        Ok(PaymentInitiation {
            external_ref: Some(response.reference),
            payment_url: response.payment_url,
            instructions,
        })
    }

    async fn verify_webhook(&self, headers: &HeaderMap, body: &Bytes) -> Result<WebhookEvent> {
        // Verify the request comes from the payment provider using the API key.
        // The payment provider should send: Authorization: Bearer <PAYMENT_API_KEY>
        let api_key = self.config.payment_api_key.as_deref().unwrap_or("");
        if !api_key.is_empty() {
            let auth = headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let expected = format!("Bearer {api_key}");
            if auth != expected {
                return Err(AppError::Unauthorized);
            }
        }

        let payload: ExternalWebhookPayload = serde_json::from_slice(body)
            .map_err(|e| AppError::Payment(format!("invalid webhook body: {e}")))?;

        let event = match payload.event.as_str() {
            "payment.completed" | "charge.succeeded" => WebhookEvent::Completed {
                external_ref: payload.reference,
                payload: String::from_utf8_lossy(body).to_string(),
                amount: None,
                currency: None,
            },
            "payment.failed" | "charge.failed" => WebhookEvent::Failed {
                external_ref: payload.reference,
                reason: payload.reason.unwrap_or_default(),
            },
            "payment.refunded" | "charge.refunded" => WebhookEvent::Refunded {
                external_ref: payload.reference,
            },
            _ => WebhookEvent::Unknown,
        };

        Ok(event)
    }

    fn provider_name(&self) -> &'static str {
        "external"
    }
}
