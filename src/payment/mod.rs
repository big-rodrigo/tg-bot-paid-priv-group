use async_trait::async_trait;
use axum::http::HeaderMap;
use bytes::Bytes;

use crate::{
    db::models::{Payment, User},
    error::Result,
};

pub mod livepix;

/// Returned by `PaymentProvider::initiate` to tell the bot what to show the user.
#[derive(Debug)]
pub struct PaymentInitiation {
    /// Reference ID to store in the DB (e.g. an invoice ID from the external API)
    pub external_ref: Option<String>,
    /// Payment URL to show to the user (e.g. a checkout page)
    pub payment_url: Option<String>,
    /// Human-readable message to send the user in Telegram
    pub instructions: String,
}

/// Event parsed from an incoming webhook body.
#[derive(Debug)]
#[allow(dead_code)]
pub enum WebhookEvent {
    Completed {
        external_ref: String,
        payload: String,
        /// Actual amount paid in the provider's smallest currency unit (e.g. cents).
        /// Populated by providers that know the amount at webhook time (e.g. LivePix).
        amount: Option<i64>,
        /// Currency code (e.g. "BRL"). Populated alongside `amount`.
        currency: Option<String>,
    },
    Failed {
        external_ref: String,
        reason: String,
    },
    Refunded {
        external_ref: String,
    },
    /// Unknown / irrelevant event — ignore it
    Unknown,
}

#[async_trait]
pub trait PaymentProvider: Send + Sync {
    /// Initiate a payment for `user`. The returned `PaymentInitiation` contains
    /// instructions to forward to the user and an optional external reference to
    /// store in the DB.
    async fn initiate(&self, user: &User, payment_id: i64) -> Result<PaymentInitiation>;

    /// Verify an incoming webhook and parse it into a `WebhookEvent`.
    async fn verify_webhook(&self, headers: &HeaderMap, body: &Bytes) -> Result<WebhookEvent>;

    /// Proactively check the payment provider's API for a completed payment
    /// matching the given pending payment record. Returns `Some(WebhookEvent::Completed{..})`
    /// if a matching payment is found, `None` otherwise.
    /// Default implementation returns `None` (provider does not support proactive checks).
    async fn check_payment(&self, _payment: &Payment) -> Result<Option<WebhookEvent>> {
        Ok(None)
    }

    /// Returns the currently cached OAuth2 access token, if any.
    /// Default implementation returns `None` (for providers that don't cache tokens).
    fn get_cached_token(&self) -> Option<String> {
        None
    }
}

