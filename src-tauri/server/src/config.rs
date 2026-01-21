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
        }
    }
}
