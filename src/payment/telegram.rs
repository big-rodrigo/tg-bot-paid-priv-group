use async_trait::async_trait;
use axum::http::HeaderMap;
use bytes::Bytes;
use std::sync::Arc;
use teloxide::Bot;

use crate::{
    config::AppConfig,
    db::models::User,
    error::Result,
};

use super::{PaymentInitiation, PaymentProvider, WebhookEvent};

/// Implements Telegram's native payment flow.
/// Requires `TELEGRAM_PAYMENT_PROVIDER_TOKEN` to be configured.
///
/// Flow:
///   1. `initiate()` sends an invoice via `bot.send_invoice()` — no webhook needed.
///   2. The bot receives `pre_checkout_query` updates and `successful_payment` updates
///      directly through the Telegram dispatcher (handled in `bot/user/payment.rs`).
///   3. `verify_webhook` is a no-op for this provider because the confirmation comes
///      through the Telegram update stream, not an HTTP webhook.
pub struct TelegramPaymentProvider {
    pub bot: Bot,
    pub config: Arc<AppConfig>,
}

impl TelegramPaymentProvider {
    pub fn new(bot: Bot, config: Arc<AppConfig>) -> Self {
        Self { bot, config }
    }

    pub fn provider_token(&self) -> &str {
        self.config
            .telegram_payment_provider_token
            .as_deref()
            .unwrap_or("")
    }
}

#[async_trait]
impl PaymentProvider for TelegramPaymentProvider {
    async fn initiate(&self, _user: &User, payment_id: i64) -> Result<PaymentInitiation> {
        // The actual invoice is sent from the bot handler (bot/user/payment.rs)
        // because it needs the ChatId from the incoming Telegram message.
        // This method just prepares the metadata.
        Ok(PaymentInitiation {
            external_ref: Some(payment_id.to_string()),
            payment_url: None,
            instructions: format!(
                "A payment invoice has been sent. Payment ID: {payment_id}"
            ),
        })
    }

    async fn verify_webhook(&self, _headers: &HeaderMap, _body: &Bytes) -> Result<WebhookEvent> {
        // Telegram payments are confirmed via the Telegram update stream,
        // not through an external HTTP webhook.
        Ok(WebhookEvent::Unknown)
    }

    fn provider_name(&self) -> &'static str {
        "telegram"
    }
}
