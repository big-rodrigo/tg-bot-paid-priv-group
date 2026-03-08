use std::sync::Arc;
use teloxide::Bot;

use crate::{config::AppConfig, db::DbPool, payment::PaymentProvider};

/// Shared state injected into all Axum handlers.
#[derive(Clone)]
pub struct WebState {
    pub db: DbPool,
    pub bot: Bot,
    pub config: Arc<AppConfig>,
    pub payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
}
