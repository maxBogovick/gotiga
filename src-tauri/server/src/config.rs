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
    /// Бот, которым входят. ОТДЕЛЬНЫЙ от бота уведомлений намеренно: сменив
    /// бота, которым дом пишет себе, иначе молча сломали бы вход посетителям.
    pub telegram_login_bot_token: Option<String>,
    /// Его `@имя` — из него собирается ссылка `t.me/<имя>?start=<code>`.
    /// Отдаётся клиенту, чтобы сменить бота можно было без пересборки фронта.
    pub telegram_login_bot_username: Option<String>,
    /// Секрет, которым Telegram доказывает, что обновление пришло от него:
    /// стоит и в адресе webhook, и в заголовке `X-Telegram-Bot-Api-Secret-Token`.
    pub telegram_webhook_secret: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_user: Option<String>,
    pub smtp_pass: Option<String>,
    pub smtp_from: Option<String>,
    /// Path to a MaxMind GeoLite2-City `.mmdb`. Absent → geolocation disabled.
    pub geoip_db_path: Option<String>,
    /// Local SQLite database used by the admin log viewer.
    pub admin_log_db_path: String,
    /// Secret used only for privacy-preserving analytics visitor hashes.
    pub analytics_hash_secret: String,
    /// Перец знаков: секрет, который подмешивается в хеш визуального пароля и
    /// в декой-наборы неизвестных адресов.
    ///
    /// Нужен потому, что знаков всего четыре из шестнадцати — 65 536 сочетаний.
    /// Столько перебирается по утёкшей базе за часы, и никакая стоимость
    /// Argon2 этого не меняет: пространство слишком мало. Перец лежит **вне**
    /// базы, поэтому дамп без него не перебирается вовсе.
    ///
    /// Своя переменная, а не `ADMIN_API_KEY`: ключ админа меняют свободно, а
    /// перец — только вместе со `auth_pepper_old`, иначе знаки перестанут
    /// сходиться у всех разом.
    pub auth_pepper: String,
    /// Прежний перец — на время смены.
    ///
    /// Без него смена перца невозможна: пересчитать уже лежащий хеш нельзя, а
    /// значит менять пришлось бы вместе со сбросом знаков всем сразу. С ним
    /// смена проходит незаметно: знаки, записанные прежним перцем, открывают
    /// дверь и тут же переписываются на нынешний (`SignsMatch::Legacy` →
    /// `refresh_signs`) — той же лестницей, которой подхватываются записи,
    /// сделанные вообще без перца.
    ///
    /// Убирается, когда вошли все, кого ждали. Оставленный навсегда он просто
    /// держит открытой вторую дверь к тем же знакам.
    pub auth_pepper_old: Option<String>,
}

/// Values that must never ship to production. Startup aborts if any secret
/// still holds one of these, instead of silently exposing an open admin panel.
const FORBIDDEN_SECRETS: &[&str] = &[
    "",
    "123",
    "admin",
    "password",
    "change_me",
    "change_me_in_prod",
];

impl Config {
    pub fn from_env() -> Self {
        let public_url = dotenvy::var("PUBLIC_URL").expect("PUBLIC_URL must be set");
        let cors_allowed_origins = dotenvy::var("ALLOWED_ORIGINS")
            .ok()
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| vec![public_url.trim_end_matches('/').to_string()]);

        let admin_api_key = dotenvy::var("ADMIN_API_KEY").expect("ADMIN_API_KEY must be set");
        // Перец знаков. Отката на другой секрет нет и быть не может: подстановка
        // чужого значения — это не «ослабленный режим», а молча переставшие
        // сходиться знаки у всех сразу. Локально разрешено обойтись без него,
        // и об этом говорится вслух.
        let auth_pepper = match dotenvy::var("AUTH_PEPPER") {
            Ok(value) if !value.trim().is_empty() => value,
            _ if is_local_public_url(&public_url) => {
                tracing::warn!(
                    "AUTH_PEPPER is not set; using a fixed development value. Never do this outside localhost."
                );
                "gotiga-local-development-pepper".to_string()
            }
            _ => panic!(
                "AUTH_PEPPER must be set outside local development — and then never changed: \
                 changing it stops every visual password from matching"
            ),
        };
        let auth_pepper_old = dotenvy::var("AUTH_PEPPER_OLD")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let analytics_hash_secret = match dotenvy::var("ANALYTICS_HASH_SECRET") {
            Ok(value) if !value.trim().is_empty() => value,
            _ if is_local_public_url(&public_url) => {
                tracing::warn!(
                    "ANALYTICS_HASH_SECRET is not set; falling back to ADMIN_API_KEY for local development"
                );
                admin_api_key.clone()
            }
            _ => panic!("ANALYTICS_HASH_SECRET must be set outside local development"),
        };

        let config = Self {
            database_url: dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            host: dotenvy::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: dotenvy::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            admin_api_key,
            upload_dir: dotenvy::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()),
            public_url,
            rust_log: dotenvy::var("RUST_LOG").unwrap_or_else(|_| "info,sqlx=warn".into()),
            admin_login: dotenvy::var("ADMIN_LOGIN").unwrap_or_else(|_| "admin".to_string()),
            admin_password: dotenvy::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set"),
            cors_allowed_origins,
            telegram_bot_token: dotenvy::var("TELEGRAM_BOT_TOKEN").ok(),
            telegram_chat_id: dotenvy::var("TELEGRAM_CHAT_ID").ok(),
            telegram_login_bot_token: dotenvy::var("TELEGRAM_LOGIN_BOT_TOKEN")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            telegram_login_bot_username: dotenvy::var("TELEGRAM_LOGIN_BOT_USERNAME")
                .ok()
                .map(|s| s.trim().trim_start_matches('@').to_string())
                .filter(|s| !s.is_empty()),
            telegram_webhook_secret: dotenvy::var("TELEGRAM_WEBHOOK_SECRET")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            smtp_host: dotenvy::var("SMTP_HOST").ok(),
            smtp_port: dotenvy::var("SMTP_PORT").ok().and_then(|v| v.parse().ok()),
            smtp_user: dotenvy::var("SMTP_USER").ok(),
            smtp_pass: dotenvy::var("SMTP_PASS").ok(),
            smtp_from: dotenvy::var("SMTP_FROM").ok(),
            geoip_db_path: dotenvy::var("GEOIP_DB_PATH").ok().filter(|s| !s.is_empty()),
            admin_log_db_path: dotenvy::var("ADMIN_LOG_DB_PATH")
                .unwrap_or_else(|_| "./data/admin_logs.sqlite".to_string()),
            analytics_hash_secret,
            auth_pepper,
            auth_pepper_old,
        };
        config.validate();
        config
    }

    /// Fail fast on weak/default secrets so a forgotten env var can never leave
    /// the admin panel or API open in production.
    fn validate(&self) {
        let weak = |v: &str| FORBIDDEN_SECRETS.contains(&v.trim().to_lowercase().as_str());
        if weak(&self.admin_password) || self.admin_password.len() < 8 {
            panic!(
                "ADMIN_PASSWORD is unset, weak, or a known default — set a strong value (>= 8 chars)"
            );
        }
        if weak(&self.admin_api_key) || self.admin_api_key.len() < 16 {
            panic!(
                "ADMIN_API_KEY is unset, weak, or a known default — set a strong value (>= 16 chars)"
            );
        }
        if weak(&self.analytics_hash_secret) || self.analytics_hash_secret.len() < 16 {
            panic!(
                "ANALYTICS_HASH_SECRET is unset, weak, or a known default — set a strong value (>= 16 chars)"
            );
        }
        if weak(&self.auth_pepper) || self.auth_pepper.len() < 16 {
            panic!(
                "AUTH_PEPPER is unset, weak, or a known default — set a strong value (>= 16 chars)"
            );
        }
        // Прежний перец, равный нынешнему, — это не смена, а описка, и молча
        // она означала бы, что смена как будто прошла.
        if let Some(old) = self.auth_pepper_old.as_deref() {
            if weak(old) || old.len() < 16 {
                panic!("AUTH_PEPPER_OLD is set but weak — remove it or give the real previous value");
            }
            if old == self.auth_pepper {
                panic!("AUTH_PEPPER_OLD equals AUTH_PEPPER — remove it; as written it changes nothing");
            }
        }
        // Почта — часть входа: по ссылке из письма имя открывается и адрес
        // подтверждается. Без неё дверь переходит в ОТКРЫТЫЙ порядок: имя
        // заводится сразу, а занятый адрес отвечает отказом — то есть по ответу
        // снова можно перечислить жильцов дома. Это терпимо и названо здесь
        // вслух, потому что иначе дом без почты просто закрыт.
        //
        // Настройки могут лежать и в базе (админский стол), поэтому здесь
        // только окружение: это подсказка при запуске, а не решение — решает
        // `AppService::mail_works`, который спрашивает оба места.
        if !self.public_url_is_local() && self.smtp_host.is_none() {
            tracing::warn!(
                "SMTP is not configured in the environment. Unless it is set in the admin table, sign-up runs in the OPEN order: names open at once, and a taken address answers with a refusal — which tells a stranger that the address is taken. Restoring signs then works only for those who linked Telegram."
            );
        }
        // Вход через Telegram включается целиком или не включается вовсе.
        //
        // Webhook без секрета — открытая дверь: кто угодно шлёт на него
        // «подтверждаю, я такой-то telegram_id» и получает чужую сессию. Это
        // единственное место во всём замысле, где чужой запрос напрямую
        // превращается в вход, поэтому половина настройки не допускается, а
        // сообщается при запуске.
        if self.telegram_login_bot_token.is_some() {
            match self.telegram_webhook_secret.as_deref() {
                Some(v) if !weak(v) && v.len() >= 16 => {}
                _ => panic!(
                    "TELEGRAM_LOGIN_BOT_TOKEN is set, so TELEGRAM_WEBHOOK_SECRET must be set to a strong value (>= 16 chars) — a webhook without a secret hands out sessions to anyone"
                ),
            }
            if self.telegram_login_bot_username.is_none() {
                panic!(
                    "TELEGRAM_LOGIN_BOT_TOKEN is set, so TELEGRAM_LOGIN_BOT_USERNAME must be set — without it the login link cannot be built"
                );
            }
        }
    }
}

impl Config {
    /// Адрес, до которого снаружи не дотянуться. Саморегистрация webhook по
    /// нему невозможна — Telegram не ходит на localhost, — и вместо неудачной
    /// попытки сервер говорит об этом словами.
    pub fn public_url_is_local(&self) -> bool {
        is_local_public_url(&self.public_url)
    }
}

fn is_local_public_url(public_url: &str) -> bool {
    public_url.contains("localhost")
        || public_url.contains("127.0.0.1")
        || public_url.contains("[::1]")
}
