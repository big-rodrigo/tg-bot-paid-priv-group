use std::sync::Arc;
use teloxide::Bot;
use tokio::sync::RwLock;

use crate::{backup::BackupManager, config::AppConfig, db::DbPool, i18n::Lang, payment::PaymentProvider};

/// Shared state injected into all Axum handlers.
#[derive(Clone)]
pub struct WebState {
    pub db: DbPool,
    pub bot: Bot,
    pub config: Arc<AppConfig>,
    pub payment_provider: Arc<dyn PaymentProvider + Send + Sync>,
    pub lang: Arc<RwLock<Lang>>,
    pub backup_manager: Arc<BackupManager>,
}
