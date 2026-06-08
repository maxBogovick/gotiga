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
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_user: Option<String>,
    pub smtp_pass: Option<String>,
    pub smtp_from: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: dotenvy::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: dotenvy::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            admin_api_key: dotenvy::var("ADMIN_API_KEY").expect("ADMIN_API_KEY must be set"),
            upload_dir: dotenvy::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            public_url: dotenvy::var("PUBLIC_URL").expect("PUBLIC_URL must be set"),
            rust_log: dotenvy::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".into()),
            admin_login: dotenvy::var("ADMIN_LOGIN").unwrap_or_else(|_| "admin".to_string()),
            admin_password: dotenvy::var("ADMIN_PASSWORD").unwrap_or_else(|_| "123".to_string()),
            telegram_bot_token: dotenvy::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: dotenvy::var("TELEGRAM_CHAT_ID").ok(),
            smtp_host: dotenvy::var("SMTP_HOST").ok(),
            smtp_port: dotenvy::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()),
            smtp_user: dotenvy::var("SMTP_USER").ok(),
            smtp_pass: dotenvy::var("SMTP_PASS").ok(),
            smtp_from: dotenvy::var("SMTP_FROM").ok(),
        }
    }
}
