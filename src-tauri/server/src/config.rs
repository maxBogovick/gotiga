use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub admin_api_key: String,
    pub upload_dir: String,
    pub public_url: String,
    pub rust_log: String,
    pub admin_login: String,
    pub admin_password: String,
    /// Origins allowed to call the API via CORS. Defaults to `public_url`.
    pub cors_allowed_origins: Vec<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_user: Option<String>,
    pub smtp_pass: Option<String>,
    pub smtp_from: Option<String>,
    /// Path to a MaxMind GeoLite2-City `.mmdb`. Absent → geolocation disabled.
    pub geoip_db_path: Option<String>,
}

/// Values that must never ship to production. Startup aborts if any secret
/// still holds one of these, instead of silently exposing an open admin panel.
const FORBIDDEN_SECRETS: &[&str] = &["", "123", "admin", "password", "change_me", "change_me_in_prod"];

impl Config {
    pub fn from_env() -> Self {
        let public_url = dotenvy::var("PUBLIC_URL").expect("PUBLIC_URL must be set");
        let cors_allowed_origins = dotenvy::var("ALLOWED_ORIGINS")
            .ok()
            .map(|v| v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>())
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| vec![public_url.trim_end_matches('/').to_string()]);

        let config = Self {
            database_url: dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: dotenvy::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: dotenvy::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            admin_api_key: dotenvy::var("ADMIN_API_KEY").expect("ADMIN_API_KEY must be set"),
            upload_dir: dotenvy::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            public_url,
            rust_log: dotenvy::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".into()),
            admin_login: dotenvy::var("ADMIN_LOGIN").unwrap_or_else(|_| "admin".to_string()),
            admin_password: dotenvy::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set"),
            cors_allowed_origins,
            telegram_bot_token: dotenvy::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: dotenvy::var("TELEGRAM_CHAT_ID").ok(),
            smtp_host: dotenvy::var("SMTP_HOST").ok(),
            smtp_port: dotenvy::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()),
            smtp_user: dotenvy::var("SMTP_USER").ok(),
            smtp_pass: dotenvy::var("SMTP_PASS").ok(),
            smtp_from: dotenvy::var("SMTP_FROM").ok(),
            geoip_db_path: dotenvy::var("GEOIP_DB_PATH").ok().filter(|s| !s.is_empty()),
        };
        config.validate();
        config
    }

    /// Fail fast on weak/default secrets so a forgotten env var can never leave
    /// the admin panel or API open in production.
    fn validate(&self) {
        let weak = |v: &str| FORBIDDEN_SECRETS.contains(&v.trim().to_lowercase().as_str());
        if weak(&self.admin_password) || self.admin_password.len() < 8 {
            panic!("ADMIN_PASSWORD is unset, weak, or a known default — set a strong value (>= 8 chars)");
        }
        if weak(&self.admin_api_key) || self.admin_api_key.len() < 16 {
            panic!("ADMIN_API_KEY is unset, weak, or a known default — set a strong value (>= 16 chars)");
        }
    }
}
