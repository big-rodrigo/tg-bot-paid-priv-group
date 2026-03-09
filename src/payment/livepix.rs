use async_trait::async_trait;
use axum::http::HeaderMap;
use bytes::Bytes;
use serde::Deserialize;
use std::{sync::Arc, time::Instant};
use tokio::sync::RwLock;

use crate::{
    config::AppConfig,
    db::{models::User, DbPool},
    error::{AppError, Result},
};

use crate::i18n;
use super::{PaymentInitiation, PaymentProvider, WebhookEvent};

pub struct LivePixProvider {
    config: Arc<AppConfig>,
    pool: DbPool,
    client: reqwest::Client,
    token: Arc<RwLock<Option<CachedToken>>>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl LivePixProvider {
    pub fn new(config: Arc<AppConfig>, pool: DbPool) -> Self {
        Self {
            config,
            pool,
            client: reqwest::Client::new(),
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Returns a valid OAuth2 access token, refreshing if needed.
    async fn get_token(&self) -> Result<String> {
        // Fast path: valid cached token
        {
            let guard = self.token.read().await;
            if let Some(ref t) = *guard {
                if t.expires_at > Instant::now() {
                    return Ok(t.access_token.clone());
                }
            }
        }

        // Slow path: fetch a new token
        let client_id = self
            .config
            .livepix_client_id
            .as_deref()
            .ok_or_else(|| AppError::Payment("LIVEPIX_CLIENT_ID not configured".into()))?;
        let client_secret = self
            .config
            .livepix_client_secret
            .as_deref()
            .ok_or_else(|| AppError::Payment("LIVEPIX_CLIENT_SECRET not configured".into()))?;

        let resp = self
            .client
            .post("https://oauth.livepix.gg/oauth2/token")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("scope", "payments:read messages:read webhooks controls"),
            ])
            .send()
            .await?
            .error_for_status()
            .map_err(|e| AppError::Payment(format!("LivePix OAuth error: {e}")))?
            .json::<TokenResponse>()
            .await?;

        let expires_at =
            Instant::now() + std::time::Duration::from_secs(resp.expires_in.saturating_sub(60));

        let mut guard = self.token.write().await;
        *guard = Some(CachedToken {
            access_token: resp.access_token.clone(),
            expires_at,
        });

        Ok(resp.access_token)
    }

    async fn read_setting(&self, key: &str) -> Result<String> {
        sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::Payment(format!("setting '{key}' not found")))
    }
}

// ── Webhook payload shapes ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct WebhookBody {
    #[serde(rename = "clientId")]
    client_id: Option<String>,
    resource: WebhookResource,
}

#[derive(Deserialize)]
struct WebhookResource {
    id: String,
    #[serde(rename = "type")]
    resource_type: String,
}

#[derive(Deserialize)]
struct MessageData {
    text: Option<String>,
    amount: i64,
    currency: String,
}

#[derive(Deserialize)]
struct MessageResponse {
    data: MessageData,
}

#[derive(Deserialize)]
struct PaymentData {
    reference: Option<String>,
    amount: i64,
    currency: String,
}

#[derive(Deserialize)]
struct PaymentResponse {
    data: PaymentData,
}

// ── PaymentProvider impl ──────────────────────────────────────────────────────

#[async_trait]
impl PaymentProvider for LivePixProvider {
    async fn initiate(&self, user: &User, _payment_id: i64) -> Result<PaymentInitiation> {
        let account_url = self.read_setting("livepix_account_url").await?;
        let price_cents_str = self.read_setting("livepix_price_cents").await?;
        let currency = self.read_setting("livepix_currency").await?;

        let lang_code = self.read_setting("language").await.unwrap_or_else(|_| "en".to_string());
        let l = i18n::Lang::from_code(&lang_code);

        let price_cents: i64 = price_cents_str.parse().unwrap_or(0);

        // The identifier is what the user must type as the message on the LivePix page.
        let identifier = user
            .username
            .clone()
            .unwrap_or_else(|| format!("tg{}", user.telegram_id));

        let price_display = format!("{:.2}", price_cents as f64 / 100.0);

        let instructions = if account_url.is_empty() {
            i18n::livepix_not_configured(l).to_string()
        } else {
            i18n::livepix_instructions(l, &identifier, &currency, &price_display)
        };

        Ok(PaymentInitiation {
            external_ref: Some(identifier),
            payment_url: if account_url.is_empty() {
                None
            } else {
                Some(account_url)
            },
            instructions,
        })
    }

    async fn verify_webhook(&self, _headers: &HeaderMap, body: &Bytes) -> Result<WebhookEvent> {
        let body_str = String::from_utf8_lossy(body).to_string();

        let webhook: WebhookBody = serde_json::from_slice(body)
            .map_err(|e| AppError::Payment(format!("invalid LivePix webhook body: {e}")))?;

        // Verify clientId matches our configured client ID
        if let Some(expected_id) = self.config.livepix_client_id.as_deref() {
            if let Some(ref received_id) = webhook.client_id {
                if received_id != expected_id {
                    tracing::warn!(
                        "LivePix webhook clientId mismatch: expected {expected_id}, got {received_id}"
                    );
                    return Err(AppError::Unauthorized);
                }
            }
        }

        let token = self.get_token().await?;
        let resource_id = &webhook.resource.id;

        let (identifier, amount, currency) = match webhook.resource.resource_type.as_str() {
            "message" => {
                let resp = self
                    .client
                    .get(format!("https://api.livepix.gg/v2/messages/{resource_id}"))
                    .bearer_auth(&token)
                    .send()
                    .await?
                    .error_for_status()
                    .map_err(|e| AppError::Payment(format!("LivePix API error: {e}")))?
                    .json::<MessageResponse>()
                    .await?;

                let text = resp.data.text.unwrap_or_default();
                // Strip leading '@' if present
                let ident = text.trim_start_matches('@').trim().to_string();
                (ident, resp.data.amount, resp.data.currency)
            }
            "payment" => {
                let resp = self
                    .client
                    .get(format!("https://api.livepix.gg/v2/payments/{resource_id}"))
                    .bearer_auth(&token)
                    .send()
                    .await?
                    .error_for_status()
                    .map_err(|e| AppError::Payment(format!("LivePix API error: {e}")))?
                    .json::<PaymentResponse>()
                    .await?;

                let reference = resp.data.reference.unwrap_or_default();
                let ident = reference.trim_start_matches('@').trim().to_string();
                (ident, resp.data.amount, resp.data.currency)
            }
            other => {
                tracing::debug!("LivePix webhook: unhandled resource type '{other}' — ignoring");
                return Ok(WebhookEvent::Unknown);
            }
        };

        if identifier.is_empty() {
            tracing::warn!("LivePix webhook: empty identifier in resource {resource_id}");
            return Ok(WebhookEvent::Unknown);
        }

        // Check against configured minimum price
        let price_cents_str = self
            .read_setting("livepix_price_cents")
            .await
            .unwrap_or_else(|_| "0".to_string());
        let price_cents: i64 = price_cents_str.parse().unwrap_or(0);

        if price_cents > 0 && amount < price_cents {
            tracing::warn!(
                "LivePix payment from '{identifier}': amount {amount} < required {price_cents} — ignoring"
            );
            return Ok(WebhookEvent::Unknown);
        }

        tracing::info!(
            "LivePix payment confirmed: identifier='{identifier}' amount={amount} {currency}"
        );

        Ok(WebhookEvent::Completed {
            external_ref: identifier,
            payload: body_str,
            amount: Some(amount),
            currency: Some(currency),
        })
    }

    fn provider_name(&self) -> &'static str {
        "livepix"
    }

    fn get_cached_token(&self) -> Option<String> {
        self.token
            .try_read()
            .ok()
            .and_then(|guard| guard.as_ref().map(|t| t.access_token.clone()))
    }
}
