mod bot;
mod config;
mod db;
mod error;
mod i18n;
mod payment;
mod web;

use std::{net::SocketAddr, str::FromStr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use config::AppConfig;
use db::DbPool;
use payment::{
    external::ExternalPaymentProvider,
    livepix::LivePixProvider,
    telegram::TelegramPaymentProvider,
    PaymentProvider,
};
use web::state::WebState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present (silently skip if not found)
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Parse configuration from environment variables
    let config: AppConfig =
        envy::from_env().expect("Missing required environment variables — check your .env file");
    let config = Arc::new(config);

    tracing::info!("Starting tg-bot");
    tracing::info!("Database: {}", config.database_url);
    tracing::info!("Admin: @{}", config.admin_telegram_username);
    tracing::info!("Web interface port: {}", config.web_interface_port);

    // Connect to the database and run migrations.
    // Backend is selected automatically from DATABASE_URL scheme.
    let pool: DbPool = if config.database_url.starts_with("sqlite:") {
        let connect_options = sqlx::sqlite::SqliteConnectOptions::from_str(&config.database_url)
            .expect("Invalid DATABASE_URL")
            .create_if_missing(true);
        let p = sqlx::SqlitePool::connect_with(connect_options)
            .await
            .expect("Failed to connect to SQLite database");
        sqlx::migrate!("./migrations/sqlite")
            .run(&p)
            .await
            .expect("Failed to run SQLite migrations");
        DbPool::Sqlite(p)
    } else {
        // Run migrations on a separate connection when DATABASE_MIGRATION_URL is set.
        // This avoids PgBouncer's prepared-statement conflicts during migration.
        let migration_url = config.database_migration_url.as_deref()
            .unwrap_or(&config.database_url);
        let migration_opts = sqlx::postgres::PgConnectOptions::from_str(migration_url)
            .expect("Invalid DATABASE_MIGRATION_URL");
        let migration_pool = sqlx::PgPool::connect_with(migration_opts)
            .await
            .expect("Failed to connect to database for migrations");
        sqlx::migrate!("./migrations/postgres")
            .run(&migration_pool)
            .await
            .expect("Failed to run PostgreSQL migrations");
        migration_pool.close().await;

        // App pool: disable statement cache for PgBouncer compatibility.
        let mut pg_opts = sqlx::postgres::PgConnectOptions::from_str(&config.database_url)
            .expect("Invalid DATABASE_URL");
        if config.database_url.contains("pgbouncer=true") {
            pg_opts = pg_opts.statement_cache_capacity(0);
        }
        let p = sqlx::PgPool::connect_with(pg_opts)
            .await
            .expect("Failed to connect to PostgreSQL database");
        DbPool::Postgres(p)
    };

    tracing::info!("Database migrations applied");

    // Load language setting
    let lang_code: String =
        db_query_scalar!(&pool, String, "SELECT value FROM settings WHERE key = 'bot_language'", [], fetch_optional)?
            .unwrap_or_else(|| "en".to_string());
    let lang = Arc::new(RwLock::new(i18n::Lang::from_code(&lang_code)));
    tracing::info!("Bot language: {}", lang_code);

    // Create in-memory dialogue storage (state resets on restart; swap for a
    // persistent implementation if needed — see src/bot/state.rs for notes).
    let storage = bot::state::create_storage();

    // Build the Telegram bot
    let telegram_bot = teloxide::Bot::new(&config.bot_api_key);

    // Select payment provider based on configuration
    let payment_provider: Arc<dyn PaymentProvider + Send + Sync> =
        if config.telegram_payment_provider_token.is_some() {
            tracing::info!("Payment provider: Telegram Payments");
            Arc::new(TelegramPaymentProvider::new(
                telegram_bot.clone(),
                Arc::clone(&config),
            ))
        } else if config.livepix_client_id.is_some() && config.livepix_client_secret.is_some() {
            tracing::info!("Payment provider: LivePix");
            Arc::new(LivePixProvider::new(Arc::clone(&config), pool.clone()))
        } else {
            tracing::info!(
                "Payment provider: External API ({})",
                config
                    .payment_api_url
                    .as_deref()
                    .unwrap_or("not configured")
            );
            Arc::new(ExternalPaymentProvider::new(Arc::clone(&config)))
        };

    if let Some(ref base) = config.webhook_base_url {
        tracing::info!("Webhook endpoint: {base}/api/webhooks/payment");
    }

    // Build shared web state
    let web_state = WebState {
        db: pool.clone(),
        bot: telegram_bot.clone(),
        config: Arc::clone(&config),
        payment_provider: Arc::clone(&payment_provider),
        lang: Arc::clone(&lang),
    };

    // ── Spawn tasks ──────────────────────────────────────────────────────

    let bot_pool = pool.clone();
    let bot_config = Arc::clone(&config);
    let bot_provider = Arc::clone(&payment_provider);
    let bot_storage = Arc::clone(&storage);
    let bot_lang = Arc::clone(&lang);

    let bot_task = tokio::spawn(async move {
        tracing::info!("Bot dispatcher starting");
        bot::run_dispatcher(
            telegram_bot,
            bot_storage,
            bot_pool,
            bot_config,
            bot_provider,
            bot_lang,
        )
        .await;
    });

    let web_port = config.web_interface_port;

    let web_task = tokio::spawn(async move {
        let router = web::create_router(web_state);
        let addr: SocketAddr = format!("0.0.0.0:{web_port}").parse().unwrap();
        let listener = TcpListener::bind(addr)
            .await
            .expect("Failed to bind web interface port");
        tracing::info!("Web interface listening on http://{addr}");
        axum::serve(listener, router)
            .await
            .expect("Web server error");
    });

    // If either task exits, log the error and shut down
    tokio::select! {
        res = bot_task => {
            tracing::error!("Bot task exited unexpectedly: {:?}", res);
        }
        res = web_task => {
            tracing::error!("Web task exited unexpectedly: {:?}", res);
        }
    }

    Ok(())
}
