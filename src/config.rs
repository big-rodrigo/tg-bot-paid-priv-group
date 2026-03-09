use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    /// Telegram Bot API key from @BotFather
    pub bot_api_key: String,

    /// Database connection URL. Defaults to a local SQLite file.
    /// Set to `postgres://user:pass@host/db` to use PostgreSQL instead.
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// Telegram username of the admin user (without @)
    pub admin_telegram_username: String,

    /// External payment API endpoint (POST request with user data)
    pub payment_api_url: Option<String>,

    /// Bearer token / API key for the external payment API
    pub payment_api_key: Option<String>,

    /// If set, enables Telegram native payments using this provider token
    pub telegram_payment_provider_token: Option<String>,

    /// LivePix OAuth2 client ID (enables LivePix payment provider when both are set)
    pub livepix_client_id: Option<String>,

    /// LivePix OAuth2 client secret
    pub livepix_client_secret: Option<String>,

    /// LivePix donation page URL (fallback when DB setting is not configured)
    pub livepix_account_url: Option<String>,

    /// LivePix minimum price in cents (fallback when DB setting is not configured)
    pub livepix_price_cents: Option<String>,

    /// LivePix currency code (fallback when DB setting is not configured)
    pub livepix_currency: Option<String>,

    /// Port the admin web interface listens on
    #[serde(default = "default_web_port")]
    pub web_interface_port: u16,

    /// Password for the admin web interface (Basic auth: admin:<secret>)
    pub web_interface_secret: String,

    /// Public base URL for this server (e.g. ngrok tunnel URL).
    /// When set, the webhook endpoint is logged on startup for easy copy-pasting.
    pub webhook_base_url: Option<String>,
}

fn default_database_url() -> String {
    "sqlite:./data.db".to_string()
}

fn default_web_port() -> u16 {
    3000
}
