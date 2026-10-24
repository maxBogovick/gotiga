//! Вход через Google и Apple: путь «старт → callback → выдача» целиком.
//!
//! Провайдер здесь поддельный, но настоящий по форме: на своём порту отвечает
//! обмен кода и список ключей, а `id_token` подписан по-настоящему (RS256).
//! Проверяется то, из-за чего схема устроена так, а не проще:
//!
//! * сессию получает только тот, у кого есть и привязка, и код: ссылка,
//!   присланная жертве, не отдаёт злоумышленнику ни её сессию, ни её Google;
//! * callback ничего не обменивает, а результат отдаётся ровно один раз;
//! * вернувшегося узнают по `sub`, имя в профиле при этом не переписывается;
//! * та же подтверждённая почта — тот же человек, неподтверждённая — нет;
//! * скрытая почта Apple и неподтверждённая почта Google своей почтой не становятся;
//! * заблокированному эта дверь закрыта, как и первая;
//! * отвязать можно только то, что не последняя дверь;
//! * чужой `returnOrigin` перенаправлением не становится.

use axum::extract::{Form, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use gotiga_server::api;
use gotiga_server::config::Config;
use gotiga_server::db::Repository;
use gotiga_server::logs::AdminLogStore;
use gotiga_server::oauth::{AppleSettings, Endpoints, Oauth, Settings};
use gotiga_server::services::AppService;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use uuid::Uuid;

const RSA_PRIVATE: &str = include_str!("fixtures/oauth_test_rsa.pem");
const RSA_N: &str = include_str!("fixtures/oauth_test_rsa.n");
const EC_PRIVATE: &str = include_str!("fixtures/oauth_test_ec.p8");
const GOOGLE_ID: &str = "gid.apps.example";
const APPLE_ID: &str = "com.test.web";

// ── Поддельный провайдер ─────────────────────────────────────────────────

#[derive(Default)]
struct Provider {
    /// Что провайдер подпишет в следующем `id_token`.
    claims: Value,
    /// Что ему прислали на обмен — чтобы проверить, что именно мы отправляем.
    seen: Vec<HashMap<String, String>>,
}

type Shared = Arc<Mutex<Provider>>;

async fn token(State(p): State<Shared>, Form(form): Form<HashMap<String, String>>) -> Json<Value> {
    let mut p = p.lock().unwrap();
    p.seen.push(form);
    let mut h = Header::new(Algorithm::RS256);
    h.kid = Some("k1".into());
    let jwt = jsonwebtoken::encode(&h, &p.claims, &EncodingKey::from_rsa_pem(RSA_PRIVATE.as_bytes()).unwrap()).unwrap();
    Json(json!({ "id_token": jwt }))
}

async fn keys() -> Json<Value> {
    Json(json!({ "keys": [{ "kty": "RSA", "kid": "k1", "use": "sig", "alg": "RS256", "n": RSA_N.trim(), "e": "AQAB" }] }))
}

async fn spawn_provider() -> (String, Shared) {
    let shared: Shared = Arc::default();
    let router = Router::new()
        .route("/token", post(token))
        .route("/keys", get(keys))
        .with_state(shared.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (addr, shared)
}

/// Что подписывает провайдер, когда всё в порядке.
fn google_claims(nonce: &str, sub: &str, extra: Value) -> Value {
    claims("https://accounts.google.com", GOOGLE_ID, nonce, sub, extra)
}

fn apple_claims(nonce: &str, sub: &str, extra: Value) -> Value {
    claims("https://appleid.apple.com", APPLE_ID, nonce, sub, extra)
}

fn claims(iss: &str, aud: &str, nonce: &str, sub: &str, extra: Value) -> Value {
    let now = chrono::Utc::now().timestamp();
    let mut c = json!({ "iss": iss, "aud": aud, "sub": sub, "nonce": nonce, "iat": now, "exp": now + 600 });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        c[k] = v;
    }
    c
}

// ── Дом ──────────────────────────────────────────────────────────────────

async fn spawn(pool: PgPool, provider_addr: &str, mail: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{port}");

    let config = Config {
        database_url: "postgres://localhost/ignored".into(),
        host: "127.0.0.1".into(),
        port,
        admin_api_key: "test-admin-api-key-0123456789".into(),
        upload_dir: format!("/tmp/gotiga-oauth-uploads-{}", Uuid::new_v4()),
        public_url: addr.clone(),
        rust_log: String::new(),
        admin_login: "admin".into(),
        admin_password: "test-password-123".into(),
        cors_allowed_origins: vec![],
        telegram_bot_token: None,
        telegram_chat_id: None,
        telegram_login_bot_token: None,
        telegram_login_bot_username: None,
        telegram_webhook_secret: None,
        telegram_channel_id: None,
        telegram_channel_extra: vec![],
        google_client_id: None,
        google_client_secret: None,
        apple_client_id: None,
        apple_team_id: None,
        apple_key_id: None,
        apple_private_key: None,
        // Почта «настроена» поддельным хостом: письма не уходят, но дом считает,
        // что отправить есть чем, — это и включает строгий порядок заведения имён.
        smtp_host: mail.then(|| "smtp.example.test".to_string()),
        smtp_port: mail.then_some(587),
        smtp_user: mail.then(|| "pochta@example.test".to_string()),
        smtp_pass: mail.then(|| "secret".to_string()),
        smtp_from: mail.then(|| "dom@example.test".to_string()),
        geoip_db_path: None,
        admin_log_db_path: format!("/tmp/gotiga-oauth-logs-{}.sqlite", Uuid::new_v4()),
        analytics_hash_secret: "test-analytics-secret-0123456789".into(),
        auth_pepper: "pepper-for-tests-0123456789".into(),
        auth_pepper_old: None,
    };

    let oauth = Oauth::new(
        reqwest::Client::new(),
        Settings {
            public_url: &addr,
            google: Some((GOOGLE_ID, "google-secret")),
            apple: Some(AppleSettings {
                client_id: APPLE_ID,
                team_id: "TEAM123456",
                key_id: "KEY1234567",
                private_key: EC_PRIVATE,
            }),
        },
    )
    .unwrap()
    .with_endpoints(Endpoints {
        google_token: format!("{provider_addr}/token"),
        google_keys: format!("{provider_addr}/keys"),
        apple_token: format!("{provider_addr}/token"),
        apple_keys: format!("{provider_addr}/keys"),
    });

    let service = AppService::new(Repository::new(pool), config.clone()).with_oauth(oauth);
    let log_store = AdminLogStore::open(&config.admin_log_db_path).await.unwrap();
    let router = api::router(service, config.clone(), log_store);
    tokio::spawn(async move {
        axum::serve(listener, router.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await
            .unwrap();
    });
    addr
}

struct World {
    pool: PgPool,
    addr: String,
    provider: Shared,
    /// Что сервер положил в `#`-часть адреса возврата, по состоянию. Это то,
    /// что увидел бы браузер, вернувшийся от провайдера, — и то, что его вкладка
    /// пришлёт в `redeem`.
    returned: Mutex<HashMap<String, HashMap<String, String>>>,
}

async fn world(pool: PgPool) -> World {
    world_with(pool, false).await
}

async fn world_with(pool: PgPool, mail: bool) -> World {
    sqlx::migrate!("./migrations/").run(&pool).await.unwrap();
    let (paddr, provider) = spawn_provider().await;
    let addr = spawn(pool.clone(), &paddr, mail).await;
    World { pool, addr, provider, returned: Mutex::default() }
}

fn no_redirects() -> reqwest::Client {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap()
}

fn print_of(token: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(token.as_bytes()))
}

impl World {
    async fn start(&self, provider: &str, bearer: Option<&str>, origin: Option<&str>) -> Value {
        self.start_aged(provider, bearer, origin, true).await
    }

    /// `age` — отметил ли человек «мне 16 или больше» там, где просил вход.
    async fn start_aged(&self, provider: &str, bearer: Option<&str>, origin: Option<&str>, age: bool) -> Value {
        let mut req = reqwest::Client::new()
            .post(format!("{}/api/v1/auth/oauth/{provider}/start", self.addr))
            .header("user-agent", "Mozilla/5.0 (Macintosh) Chrome/131.0")
            .json(&json!({ "returnOrigin": origin, "ageConfirmed": age }));
        if let Some(t) = bearer {
            req = req.header("authorization", format!("Bearer {t}"));
        }
        let resp = req.send().await.unwrap();
        assert!(resp.status().is_success(), "ссылка не выдана: {}", resp.status());
        resp.json().await.unwrap()
    }

    async fn nonce_of(&self, state: &str) -> String {
        sqlx::query_scalar("SELECT nonce FROM oauth_logins WHERE state = $1")
            .bind(state)
            .fetch_one(&self.pool)
            .await
            .unwrap()
    }

    /// Запомнить, что сервер положил в `#`-часть адреса возврата.
    fn remember(&self, to: &str) {
        let Some((_, fragment)) = to.split_once('#') else { return };
        let q: HashMap<String, String> = reqwest::Url::parse(&format!("http://x.invalid/?{fragment}"))
            .unwrap()
            .query_pairs()
            .into_owned()
            .collect();
        if let Some(state) = q.get("state").cloned() {
            self.returned.lock().unwrap().insert(state, q);
        }
    }

    /// Провайдер вернул человека на callback. Возвращает, куда сервер отправил браузер.
    async fn callback_get(&self, provider: &str, query: &str) -> (reqwest::StatusCode, String) {
        let resp = no_redirects()
            .get(format!("{}/api/v1/auth/oauth/{provider}/callback?{query}", self.addr))
            .send()
            .await
            .unwrap();
        let to = resp.headers().get("location").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
        self.remember(&to);
        (resp.status(), to)
    }

    /// То же для Apple: провайдер присылает форму методом POST.
    async fn callback_post(&self, provider: &str, form: &[(&str, &str)]) -> reqwest::Response {
        let resp = no_redirects()
            .post(format!("{}/api/v1/auth/oauth/{provider}/callback", self.addr))
            .form(form)
            .send()
            .await
            .unwrap();
        if let Some(to) = resp.headers().get("location") {
            self.remember(to.to_str().unwrap());
        }
        resp
    }

    /// Вкладка забирает результат: привязка её, а код и имя Apple — те, что
    /// вернулись в адресе возврата (если callback их туда положил).
    async fn redeem(&self, state: &str, binder: &str) -> Value {
        let (code, user) = {
            let kept = self.returned.lock().unwrap();
            let q = kept.get(state);
            (
                q.and_then(|q| q.get("code")).cloned().unwrap_or_default(),
                q.and_then(|q| q.get("user")).cloned(),
            )
        };
        self.redeem_with(state, binder, &code, user.as_deref()).await
    }

    async fn redeem_with(&self, state: &str, binder: &str, code: &str, user: Option<&str>) -> Value {
        reqwest::Client::new()
            .post(format!("{}/api/v1/auth/oauth/redeem", self.addr))
            .json(&json!({ "state": state, "binder": binder, "code": code, "user": user }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }

    /// Весь вход Google одним ходом: что бы ни подписал провайдер (`extra`),
    /// человек возвращается на callback и вкладка забирает результат.
    async fn google(&self, sub: &str, extra: Value) -> Value {
        self.google_aged(sub, extra, true).await
    }

    async fn google_aged(&self, sub: &str, extra: Value, age: bool) -> Value {
        let s = self.start_aged("google", None, None, age).await;
        let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
        let nonce = self.nonce_of(state).await;
        self.provider.lock().unwrap().claims = google_claims(&nonce, sub, extra);
        self.callback_get("google", &format!("code=abc&state={state}")).await;
        self.redeem(state, binder).await
    }

    async fn user(&self, name: &str, email: Option<&str>, confirmed: bool) -> (Uuid, String) {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO users (display_name, email, email_confirmed_at, visual_password_hash)
             VALUES ($1, $2, CASE WHEN $3 THEN NOW() END, 'x') RETURNING id",
        )
        .bind(name)
        .bind(email)
        .bind(confirmed)
        .fetch_one(&self.pool)
        .await
        .unwrap();
        let session = Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO user_sessions (user_id, token, expires_at) VALUES ($1, $2, NOW() + INTERVAL '30 days')")
            .bind(id)
            .bind(print_of(&session))
            .execute(&self.pool)
            .await
            .unwrap();
        (id, session)
    }

    async fn me(&self, session: &str) -> reqwest::Response {
        reqwest::Client::new()
            .get(format!("{}/api/v1/auth/me", self.addr))
            .header("authorization", format!("Bearer {session}"))
            .send()
            .await
            .unwrap()
    }
}

impl World {
    /// Вход Apple одним ходом. `form_user` — то, что Apple присылает в теле
    /// callback при первом согласии.
    async fn apple(&self, sub: &str, extra: Value, form_user: Option<&str>) -> Value {
        let s = self.start("apple", None, None).await;
        let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
        let nonce = self.nonce_of(state).await;
        self.provider.lock().unwrap().claims = apple_claims(&nonce, sub, extra);
        let mut form = vec![("state", state), ("code", "apple-code")];
        if let Some(u) = form_user {
            form.push(("user", u));
        }
        self.callback_post("apple", &form).await;
        self.redeem(state, binder).await
    }

    /// Вход по почте и знакам: настоящий обряд — вызов, сетка, четыре выбора.
    /// Возвращает статус и тело ответа на проверку.
    async fn try_signs(&self, email: &str, want: [&str; 4]) -> (reqwest::StatusCode, Value) {
        let client = reqwest::Client::new();
        let ch: Value = client
            .post(format!("{}/api/v1/auth/login/challenge", self.addr))
            .json(&json!({ "email": email }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let tokens: Vec<String> = ch["steps"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, step)| {
                let icons = step["icons"].as_array().unwrap();
                let pick = icons.iter().find(|ic| ic["iconId"] == want[i]).unwrap_or(&icons[0]);
                pick["token"].as_str().unwrap().to_string()
            })
            .collect();
        let resp = client
            .post(format!("{}/api/v1/auth/login/verify", self.addr))
            .json(&json!({ "challengeId": ch["challengeId"], "tokens": tokens }))
            .send()
            .await
            .unwrap();
        (resp.status(), resp.json().await.unwrap_or(Value::Null))
    }

    async fn get(&self, path: &str, session: &str) -> reqwest::StatusCode {
        reqwest::Client::new()
            .get(format!("{}/api/v1{path}", self.addr))
            .header("authorization", format!("Bearer {session}"))
            .send()
            .await
            .unwrap()
            .status()
    }
}

/// Шестнадцать значков на полосу — ровно то, что сервер и ждёт.
const ANIMALS: [&str; 16] = [
    "wolf", "raven", "fox", "owl", "snake", "deer", "bat", "cat", "bear", "hare", "boar", "lynx",
    "crow", "moth", "spider", "frog",
];
const DISHES: [&str; 16] = [
    "mushroom", "apple", "bread", "cup", "fish", "berry", "honey", "herb", "pear", "plum", "egg",
    "cheese", "grapes", "carrot", "onion", "pumpkin",
];
const SEASONS: [&str; 16] = [
    "snowflake", "bare_tree", "sprout", "rain", "sun", "wheat", "leaf", "acorn", "icicle",
    "frost_pane", "bud", "blossom", "cloud", "lightning", "mist", "pinecone",
];
const SYMBOLS: [&str; 16] = [
    "key", "candle", "hourglass", "skull", "moon", "star", "cross", "anchor", "bell", "clock",
    "feather", "inkpot", "scroll", "dagger", "crown", "eye",
];
const SIGNS: [&str; 4] = [ANIMALS[0], DISHES[1], SEASONS[2], SYMBOLS[3]];

// ── Проверки ─────────────────────────────────────────────────────────────

#[sqlx::test]
async fn a_stranger_signs_in_with_google_and_is_inside(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let url = reqwest::Url::parse(s["url"].as_str().unwrap()).unwrap();
    let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(url.host_str(), Some("accounts.google.com"));
    assert_eq!(q["state"], state);
    assert_eq!(q["code_challenge_method"], "S256");
    assert!(!url.as_str().contains("google-secret"), "секрет клиента попал в ссылку");

    let nonce = w.nonce_of(state).await;
    assert_eq!(q["nonce"], nonce);
    w.provider.lock().unwrap().claims = google_claims(
        &nonce,
        "g-1001",
        json!({ "email": "Masha@Example.com", "email_verified": true, "name": "Маша Иванова" }),
    );

    let (status, to) = w.callback_get("google", &format!("code=abc&state={state}")).await;
    assert_eq!(status, reqwest::StatusCode::SEE_OTHER);
    assert_eq!(to, format!("{}/auth/oauth#state={state}&code=abc", w.addr), "браузер вернулся не на сайт");
    assert_eq!(w.provider.lock().unwrap().seen.len(), 0, "callback обменял код, не дождавшись привязки");

    let done = w.redeem(state, binder).await;

    // Что именно мы отправили провайдеру на обмен.
    {
        let seen = w.provider.lock().unwrap();
        let form = &seen.seen[0];
        assert_eq!(form["grant_type"], "authorization_code");
        assert_eq!(form["code"], "abc");
        assert_eq!(form["client_secret"], "google-secret");
        assert_eq!(form["redirect_uri"], format!("{}/api/v1/auth/oauth/google/callback", w.addr));
        // PKCE: проверочная строка подходит к отпечатку, который ушёл в ссылку.
        assert_eq!(gotiga_server::oauth::pkce_challenge(&form["code_verifier"]), q["code_challenge"]);
    }

    assert_eq!(done["state"], "ready");
    assert_eq!(done["isNew"], true);
    assert_eq!(done["user"]["displayName"], "Маша Иванова");
    // Провайдер поручился за ящик — почта названа и подтверждена сразу.
    assert_eq!(done["user"]["email"], "masha@example.com");
    assert_eq!(done["user"]["emailConfirmed"], true);
    assert_eq!(done["user"]["googleLinked"], true);

    let session = done["sessionToken"].as_str().unwrap();
    assert!(w.me(session).await.status().is_success(), "выданная сессия не работает");
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions WHERE token = $1")
        .bind(session)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert_eq!(stored, 0, "сессия лежит в базе открытым ключом");
}

/// Злоумышленник просит вход у себя (у него привязка) и присылает жертве ссылку
/// к провайдеру. Жертва входит у провайдера, и код возвращается в её браузер.
/// Привязка без кода сессии не даёт.
#[sqlx::test]
async fn a_link_sent_to_the_victim_does_not_hand_the_attacker_the_victims_session(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "victim", json!({ "email": "v@example.com", "email_verified": true }));
    // Жертва прошла вход у провайдера: код лежит в адресе её браузера.
    let (_, to) = w.callback_get("google", &format!("code=victims-code&state={state}")).await;
    assert!(to.contains("code=victims-code"));

    // У злоумышленника есть привязка, но кода он не видел.
    let stolen = w.redeem_with(state, binder, "", None).await;
    assert_eq!(stolen["state"], "failed");
    assert!(stolen["sessionToken"].is_null(), "привязка без кода открыла чужой вход");
    assert_eq!(w.provider.lock().unwrap().seen.len(), 0, "обмен состоялся без кода");
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&w.pool).await.unwrap();
    assert_eq!(users, 0, "для жертвы заведено имя, которое получил не тот");

    // Попытка сожжена: и подобранный позже код её уже не откроет.
    let late = w.redeem_with(state, binder, "victims-code", None).await;
    assert_eq!(late["state"], "expired");
}

/// Обратный ход: злоумышленник проходит вход у себя и присылает жертве адрес
/// возврата со своим кодом. У вкладки жертвы привязки нет — она в
/// `sessionStorage` злоумышленника, — и жертва в чужое имя не входит.
#[sqlx::test]
async fn a_planted_callback_does_not_log_the_victim_into_the_attackers_name(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let state = s["state"].as_str().unwrap();
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "attacker", json!({}));
    w.callback_get("google", &format!("code=abc&state={state}")).await;

    let victim = w.redeem(state, "binder-of-the-victims-own-tab").await;
    assert_eq!(victim["state"], "expired");
    assert!(victim["sessionToken"].is_null());
    assert_eq!(w.provider.lock().unwrap().seen.len(), 0, "чужая привязка запустила обмен");

    // Настоящая привязка по-прежнему работает: попытка чужой не сожжена.
    let ok = w.redeem(state, s["binder"].as_str().unwrap()).await;
    assert_eq!(ok["state"], "ready");
}

#[sqlx::test]
async fn the_result_is_handed_over_once_and_the_code_is_exchanged_once(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-once", json!({}));
    w.callback_get("google", &format!("code=abc&state={state}")).await;
    // Двойной переход, «назад»: callback ничего не тратит.
    w.callback_get("google", &format!("code=abc&state={state}")).await;

    assert_eq!(w.redeem(state, binder).await["state"], "ready");
    assert_eq!(w.redeem(state, binder).await["state"], "expired", "сессия отдана дважды");
    assert_eq!(w.provider.lock().unwrap().seen.len(), 1, "код обменян дважды");
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&w.pool).await.unwrap();
    assert_eq!(users, 1);

    // Отработанное состояние callback-ом больше не принимается.
    let (_, again) = w.callback_get("google", &format!("code=abc&state={state}")).await;
    assert_eq!(again, format!("{}/auth/oauth", w.addr), "отработанная попытка принята снова");
}

#[sqlx::test]
async fn a_returning_visitor_is_known_by_sub_and_keeps_the_name_they_chose(pool: PgPool) {
    let w = world(pool).await;

    let first = w.google("g-2002", json!({ "email": "a@example.com", "email_verified": true, "name": "Анна" })).await;
    let id = first["user"]["id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE users SET display_name = 'Анна из Вологды' WHERE id = $1::uuid")
        .bind(&id)
        .execute(&w.pool)
        .await
        .unwrap();

    // Человек переименовался в Google и сменил ящик. `sub` тот же.
    let second = w.google("g-2002", json!({ "email": "new@example.com", "email_verified": true, "name": "Другое Имя" })).await;
    assert_eq!(second["user"]["id"], id.as_str(), "вернувшегося не узнали по sub");
    assert_eq!(second["isNew"], false);
    assert_eq!(second["user"]["displayName"], "Анна из Вологды", "имя в профиле переписано");
    assert_eq!(second["user"]["googleEmail"], "new@example.com", "адрес провайдера не освежился");
    assert_eq!(second["user"]["email"], "a@example.com", "своя почта подменена чужой");
}

#[sqlx::test]
async fn the_same_confirmed_mailbox_is_the_same_person_an_unconfirmed_one_is_not(pool: PgPool) {
    let w = world(pool).await;
    let (owner, _) = w.user("Хозяйка", Some("owner@example.com"), true).await;
    let (squatter, _) = w.user("Занявший", Some("typo@example.com"), false).await;

    let joined = w.google("g-3003", json!({ "email": "owner@example.com", "email_verified": true, "name": "Кто-то" })).await;
    assert_eq!(joined["user"]["id"], owner.to_string().as_str(), "ящик подтверждён с обеих сторон — должно слиться");
    assert_eq!(joined["isNew"], false);
    assert_eq!(joined["user"]["hasSigns"], true, "знаки первой двери пропали");

    let apart = w.google("g-3004", json!({ "email": "typo@example.com", "email_verified": true, "name": "Настоящий" })).await;
    assert_ne!(apart["user"]["id"], squatter.to_string().as_str(), "чужое неподтверждённое имя захвачено");
    assert_eq!(apart["isNew"], true);
    assert!(apart["user"]["email"].is_null(), "почта занята — имя должно быть заведено без неё");
}

#[sqlx::test]
async fn a_mailbox_the_provider_does_not_vouch_for_does_not_become_ours(pool: PgPool) {
    let w = world(pool).await;
    let (owner, _) = w.user("Хозяйка", Some("owner@example.com"), true).await;

    // `email_verified` нет вовсе — читается как «не подтверждено».
    let r = w.google("g-4004", json!({ "email": "owner@example.com", "name": "Самозванец" })).await;
    assert_ne!(r["user"]["id"], owner.to_string().as_str(), "непроверенная почта открыла чужое имя");
    assert!(r["user"]["email"].is_null());
    assert_eq!(r["user"]["emailConfirmed"], false);
}

#[sqlx::test]
async fn apple_hidden_mail_stays_a_hint_and_the_name_comes_from_the_form(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("apple", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let url = reqwest::Url::parse(s["url"].as_str().unwrap()).unwrap();
    let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(q["response_mode"], "form_post");

    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = apple_claims(
        &nonce,
        "001234.abcdef",
        json!({ "email": "xyz@privaterelay.appleid.com", "email_verified": "true", "is_private_email": "true" }),
    );

    // Apple присылает форму методом POST.
    let resp = w
        .callback_post(
            "apple",
            &[
                ("state", state),
                ("code", "apple-code"),
                ("user", r#"{"name":{"firstName":"Мария","lastName":"Петрова"}}"#),
            ],
        )
        .await;
    assert_eq!(resp.status(), reqwest::StatusCode::SEE_OTHER);
    let location = resp.headers()["location"].to_str().unwrap().to_string();
    assert!(location.starts_with(&format!("{}/auth/oauth#state={state}&code=apple-code&user=", w.addr)), "{location}");
    assert_eq!(resp.headers()["referrer-policy"], "no-referrer");

    let done = w.redeem(state, binder).await;

    // Секрет клиента Apple — подпись, а не строка из консоли.
    let secret = w.provider.lock().unwrap().seen[0]["client_secret"].clone();
    let head = jsonwebtoken::decode_header(&secret).unwrap();
    assert_eq!(head.alg, Algorithm::ES256);
    assert_eq!(head.kid.as_deref(), Some("KEY1234567"));

    assert_eq!(done["state"], "ready");
    assert_eq!(done["user"]["displayName"], "Мария Петрова");
    assert!(done["user"]["email"].is_null(), "скрытый адрес-прокси стал почтой дома");
    assert_eq!(done["user"]["appleEmail"], "xyz@privaterelay.appleid.com");
    assert_eq!(done["user"]["appleLinked"], true);
}

#[sqlx::test]
async fn a_token_for_a_different_sign_in_is_refused(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    // Провайдер подписал токен с чужим одноразовым числом.
    w.provider.lock().unwrap().claims = google_claims("nonce-of-someone-else", "g-5005", json!({}));
    let (_, to) = w.callback_get("google", &format!("code=abc&state={state}")).await;
    assert!(to.starts_with(&w.addr), "браузер должен вернуться на сайт и там узнать об отказе");

    let r = w.redeem(state, binder).await;
    assert_eq!(r["state"], "failed");
    assert_eq!(r["failure"], "failed");
    assert!(r["sessionToken"].is_null());
    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&w.pool).await.unwrap();
    assert_eq!(users, 0);
}

#[sqlx::test]
async fn pressing_cancel_at_the_provider_is_a_refusal_not_an_error_page(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let (status, to) = w.callback_get("google", &format!("error=access_denied&state={state}")).await;
    assert_eq!(status, reqwest::StatusCode::SEE_OTHER);
    // Отказ страница читает из адреса: вкладке не нужно ничего спрашивать у сервера.
    assert_eq!(to, format!("{}/auth/oauth#state={state}&error=denied", w.addr));
    assert_eq!(w.provider.lock().unwrap().seen.len(), 0, "код обменян, хотя человек отказался");

    // Попытка закрыта: даже с привязкой и подобранным кодом она не откроется.
    let r = w.redeem_with(state, binder, "abc", None).await;
    assert_eq!(r["state"], "expired");
    let status: String = sqlx::query_scalar("SELECT status FROM oauth_logins WHERE state = $1")
        .bind(state)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert_eq!(status, "failed");
}

#[sqlx::test]
async fn a_blocked_account_is_not_let_in_by_this_door_either(pool: PgPool) {
    let w = world(pool).await;

    let first = w.google("g-6006", json!({ "name": "Закрытый" })).await;
    let id = first["user"]["id"].as_str().unwrap().to_string();
    sqlx::query("UPDATE users SET is_blocked = TRUE WHERE id = $1::uuid").bind(&id).execute(&w.pool).await.unwrap();
    sqlx::query("DELETE FROM user_sessions").execute(&w.pool).await.unwrap();

    let again = w.google("g-6006", json!({ "name": "Закрытый" })).await;
    assert_eq!(again["state"], "failed");
    assert_eq!(again["failure"], "blocked");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions").fetch_one(&w.pool).await.unwrap();
    assert_eq!(sessions, 0, "заблокированному заведена сессия");
}

#[sqlx::test]
async fn a_foreign_return_origin_is_not_an_open_redirect(pool: PgPool) {
    let w = world(pool).await;

    let s = w.start("google", None, Some("https://evil.example")).await;
    let state = s["state"].as_str().unwrap();
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-7007", json!({}));
    let (_, to) = w.callback_get("google", &format!("code=abc&state={state}")).await;
    assert!(to.starts_with(&w.addr), "браузер отправлен на чужой адрес: {to}");
}

#[sqlx::test]
async fn linking_from_the_profile_adds_a_door_and_does_not_open_a_new_session(pool: PgPool) {
    let w = world(pool).await;
    let (id, session) = w.user("Анна", Some("anna@example.com"), true).await;

    let s = w.start("google", Some(&session), None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-8008", json!({ "email": "other@example.com", "email_verified": true }));
    w.callback_get("google", &format!("code=abc&state={state}")).await;

    let r = w.redeem(state, binder).await;
    assert_eq!(r["state"], "ready");
    assert!(r["sessionToken"].is_null(), "привязка завела вторую сессию");
    assert_eq!(r["user"]["id"], id.to_string().as_str());
    assert_eq!(r["user"]["googleLinked"], true);
    assert_eq!(r["user"]["email"], "anna@example.com", "почта имени подменена");

    // Тот же Google на другое имя не садится: одно число значит одного человека.
    let (_, other_session) = w.user("Борис", Some("boris@example.com"), true).await;
    let s = w.start("google", Some(&other_session), None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-8008", json!({}));
    w.callback_get("google", &format!("code=abc&state={state}")).await;
    let r = w.redeem(state, binder).await;
    assert_eq!(r["state"], "failed");
    assert_eq!(r["failure"], "conflict");
}

#[sqlx::test]
async fn a_stale_session_does_not_turn_linking_into_a_new_account(pool: PgPool) {
    let w = world(pool).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/oauth/google/start", w.addr))
        .header("authorization", "Bearer protukhshaya-sessiya")
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn the_last_door_cannot_be_unlinked(pool: PgPool) {
    let w = world(pool).await;

    // Заведён через Google: другой двери нет.
    let r = w.google("g-9009", json!({ "name": "Один" })).await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    let del = |session: String| {
        let addr = w.addr.clone();
        async move {
            reqwest::Client::new()
                .delete(format!("{addr}/api/v1/auth/oauth/google/link"))
                .header("authorization", format!("Bearer {session}"))
                .send()
                .await
                .unwrap()
        }
    };
    assert_eq!(del(session.clone()).await.status(), reqwest::StatusCode::BAD_REQUEST);
    let still: Option<String> = sqlx::query_scalar("SELECT google_sub FROM users").fetch_one(&w.pool).await.unwrap();
    assert!(still.is_some(), "последняя дверь снята");

    // Появилась вторая (подтверждённая почта со знаками) — теперь можно.
    sqlx::query("UPDATE users SET email = 'one@example.com', email_confirmed_at = NOW(), visual_password_hash = 'x'")
        .execute(&w.pool)
        .await
        .unwrap();
    let ok = del(session).await;
    assert!(ok.status().is_success());
    let body: Value = ok.json().await.unwrap();
    assert_eq!(body["googleLinked"], false);
}

#[sqlx::test]
async fn a_provider_that_is_not_set_up_has_no_page(pool: PgPool) {
    let w = world(pool).await;
    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/oauth/facebook/start", w.addr))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);

    let conf: Value = reqwest::get(format!("{}/api/v1/auth/oauth/config", w.addr)).await.unwrap().json().await.unwrap();
    assert_eq!(conf, json!({ "google": true, "apple": true }));
}

// ═════════════════════════════════════════════════════════════════════════
// Имя, заведённое через Google или Apple, по всему дому
//
// У такого имени нет знаков, а почта есть (Google) или её нет (скрытый адрес
// Apple). Ниже — каждое место, которое могло бы предположить обратное.
// ═════════════════════════════════════════════════════════════════════════

fn google_person(email: &str) -> Value {
    json!({ "email": email, "email_verified": true, "name": "Гугловская" })
}

/// Экран знаков для имени без знаков — это та же закрытая дверь, что и для
/// неизвестного адреса: по ответу нельзя узнать ни что имя есть, ни какие у
/// него двери.
#[sqlx::test]
async fn the_signs_door_is_shut_for_a_google_name_and_looks_like_any_stranger(pool: PgPool) {
    let w = world(pool).await;
    w.google("g-a1", google_person("goo@example.com")).await;

    let (known, known_body) = w.try_signs("goo@example.com", SIGNS).await;
    let (unknown, unknown_body) = w.try_signs("nobody@example.com", SIGNS).await;
    assert_eq!(known, reqwest::StatusCode::UNAUTHORIZED, "знаки открыли имя, у которого их нет");
    assert_eq!(known, unknown, "ответы разошлись — по ним можно перечислять жильцов");
    assert_eq!(known_body, unknown_body);
}

/// Набрать чужой адрес при заведении имени не отнимает имя у вошедшего через
/// Google — ни в открытом порядке, ни в строгом.
#[sqlx::test]
async fn registering_with_the_google_address_does_not_take_the_name_over(pool: PgPool) {
    for mail in [false, true] {
        let w = world_with(pool.clone(), mail).await;
        sqlx::query("DELETE FROM users").execute(&w.pool).await.unwrap();
        let r = w.google("g-b1", google_person("taken@example.com")).await;
        let id = r["user"]["id"].as_str().unwrap().to_string();

        let resp = reqwest::Client::new()
            .post(format!("{}/api/v1/auth/register", w.addr))
            .json(&json!({
                "email": "taken@example.com", "displayName": "Самозванец",
                "selections": SIGNS, "pool": [ANIMALS, DISHES, SEASONS, SYMBOLS], "ageConfirmed": true,
            }))
            .send()
            .await
            .unwrap();
        // Открытый порядок отказывает, строгий отвечает «письмо ушло» — и в
        // обоих случаях имя остаётся нетронутым.
        if mail {
            assert!(resp.status().is_success());
        } else {
            assert_eq!(resp.status(), reqwest::StatusCode::CONFLICT);
        }
        let (hash, name, sub): (Option<String>, String, Option<String>) = sqlx::query_as(
            "SELECT visual_password_hash, display_name, google_sub FROM users WHERE id = $1::uuid",
        )
        .bind(&id)
        .fetch_one(&w.pool)
        .await
        .unwrap();
        assert!(hash.is_none(), "самозванец поставил свои знаки (mail={mail})");
        assert_eq!(name, "Гугловская");
        assert_eq!(sub.as_deref(), Some("g-b1"), "дверь Google пропала (mail={mail})");
    }
}

/// «Забыли знаки» для имени из Google ведёт к знакам: подтверждённая почта —
/// доказанный канал. После знаков у имени две двери, и Google можно снять.
#[sqlx::test]
async fn a_google_name_can_add_signs_and_then_has_two_doors(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-c1", google_person("two@example.com")).await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    let id = r["user"]["id"].as_str().unwrap().to_string();
    let client = reqwest::Client::new();

    // Изнутри дома: письмо с ключом просится без адреса.
    let ask = client
        .post(format!("{}/api/v1/auth/signs/letter", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap();
    assert!(ask.status().is_success(), "ключ к знакам не просится: {}", ask.status());
    let issued: Option<String> =
        sqlx::query_scalar("SELECT password_reset_token FROM users WHERE id = $1::uuid")
            .bind(&id)
            .fetch_one(&w.pool)
            .await
            .unwrap();
    assert!(issued.is_some(), "ключ не выдан");

    // И со страницы входа — «забыли знаки» по почте — та же дорога.
    sqlx::query("UPDATE users SET password_reset_token = NULL, password_reset_expires_at = NULL")
        .execute(&w.pool)
        .await
        .unwrap();
    let forgot = client
        .post(format!("{}/api/v1/auth/forgot-password", w.addr))
        .json(&json!({ "email": "two@example.com" }))
        .send()
        .await
        .unwrap();
    assert!(forgot.status().is_success());
    let again: Option<String> =
        sqlx::query_scalar("SELECT password_reset_token FROM users WHERE id = $1::uuid")
            .bind(&id)
            .fetch_one(&w.pool)
            .await
            .unwrap();
    assert!(again.is_some(), "«забыли знаки» не дошли до имени из Google");

    // Ключ из письма нам не достать (в базе только отпечаток) — подставляем
    // свой и проходим то, что делает страница /set-password.
    let key = Uuid::new_v4().to_string();
    sqlx::query("UPDATE users SET password_reset_token = $2, password_reset_expires_at = NOW() + INTERVAL '2 hours' WHERE id = $1::uuid")
        .bind(&id)
        .bind(print_of(&key))
        .execute(&w.pool)
        .await
        .unwrap();
    let set = client
        .post(format!("{}/api/v1/auth/reset-password", w.addr))
        .json(&json!({ "token": key, "selections": SIGNS, "pool": [ANIMALS, DISHES, SEASONS, SYMBOLS] }))
        .send()
        .await
        .unwrap();
    assert!(set.status().is_success(), "знаки не назначены: {}", set.status());
    // Назначение знаков гасит прежние сессии — сброс нужен и тем, кто подозревает
    // чужой доступ…
    assert_eq!(w.get("/auth/me", &session).await, reqwest::StatusCode::UNAUTHORIZED);
    // …но человек не выброшен: в ответе новая сессия, он только что доказал ящик
    // и выбрал знаки.
    let fresh: Value = set.json().await.unwrap();
    let fresh_session = fresh["sessionToken"].as_str().expect("новой сессии нет");
    assert_ne!(fresh_session, session);
    assert!(w.get("/auth/me", fresh_session).await.is_success(), "выданная сессия не работает");
    assert_eq!(fresh["user"]["id"], id.as_str());
    assert_eq!(fresh["user"]["hasSigns"], true);

    // Теперь работает вход по почте и знакам…
    let (status, body) = w.try_signs("two@example.com", SIGNS).await;
    assert!(status.is_success(), "знаки не пускают: {body}");
    assert_eq!(body["user"]["id"], id.as_str());
    assert_eq!(body["user"]["hasSigns"], true);
    assert_eq!(body["user"]["googleLinked"], true, "дверь Google пропала после назначения знаков");

    // …и вход через Google по-прежнему ведёт в то же имя.
    let via = w.google("g-c1", google_person("two@example.com")).await;
    assert_eq!(via["user"]["id"], id.as_str());
    assert_eq!(via["isNew"], false);

    // Дверей две — Google можно снять, и знаками по-прежнему входят.
    let token = via["sessionToken"].as_str().unwrap();
    let del = client
        .delete(format!("{}/api/v1/auth/oauth/google/link", w.addr))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();
    assert!(del.status().is_success(), "вторая дверь есть, а Google не снимается");
    assert!(w.try_signs("two@example.com", SIGNS).await.0.is_success());

    // Заново через Google: та же подтверждённая почта — то же имя, а не двойник.
    let back = w.google("g-c1", google_person("two@example.com")).await;
    assert_eq!(back["user"]["id"], id.as_str(), "после отвязки Google завёл двойника");
    let names: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&w.pool).await.unwrap();
    assert_eq!(names, 1);
}

/// Что даёт имя из Google остальным частям дома: профиль, брони, заказы,
/// желания, сессии, привязка гостевых расписок — всё отвечает, как у любого.
#[sqlx::test]
async fn a_google_name_works_in_the_profile_and_with_guest_receipts(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-d1", google_person("prof@example.com")).await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    assert_eq!(r["user"]["emailConfirmed"], true, "почта поручена провайдером — спрашивать её второй раз незачем");

    for path in ["/auth/me", "/auth/sessions", "/profile/bookings", "/profile/orders", "/profile/wishlist"] {
        let status = w.get(path, &session).await;
        assert!(status.is_success(), "{path} ответил {status}");
    }

    let client = reqwest::Client::new();
    let put = client
        .put(format!("{}/api/v1/profile/wishlist", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .json(&json!({ "figurineIds": [] }))
        .send()
        .await
        .unwrap();
    assert!(put.status().is_success());

    // Расписки гостя привязываются только к ПОДТВЕРЖДЁННОЙ почте — здесь она
    // подтверждена, значит ручка работает, а не отвечает «нечем сверять».
    let link = client
        .post(format!("{}/api/v1/auth/link-bookings", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .json(&json!({ "cancelTokens": [] }))
        .send()
        .await
        .unwrap();
    assert!(link.status().is_success());

    // Имя в профиле правится как у всех.
    let patch = client
        .patch(format!("{}/api/v1/profile/me", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .json(&json!({ "displayName": "Анна из Вологды" }))
        .send()
        .await
        .unwrap();
    assert!(patch.status().is_success(), "имя не правится: {}", patch.status());
    let me: Value = client
        .get(format!("{}/api/v1/auth/me", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["displayName"], "Анна из Вологды");
    assert_eq!(me["email"], "prof@example.com");
}

/// Удаление аккаунта не требует знаков и уносит дверь вместе с именем: тот же
/// `sub` потом заводит нового человека, а не воскрешает старого.
#[sqlx::test]
async fn deleting_the_account_takes_the_google_door_with_it(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-e1", google_person("bye@example.com")).await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    let id = r["user"]["id"].as_str().unwrap().to_string();

    let del = reqwest::Client::new()
        .delete(format!("{}/api/v1/profile/me", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap();
    assert!(del.status().is_success(), "аккаунт без знаков не удаляется: {}", del.status());

    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = $1::uuid")
        .bind(&id)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert_eq!(left, 0);
    let attempts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_logins WHERE user_id = $1::uuid")
        .bind(&id)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert_eq!(attempts, 0, "следы входа остались после удаления");
    assert_eq!(w.get("/auth/me", &session).await, reqwest::StatusCode::UNAUTHORIZED);

    let again = w.google("g-e1", google_person("bye@example.com")).await;
    assert_eq!(again["isNew"], true);
    assert_ne!(again["user"]["id"], id.as_str(), "удалённый человек воскрес");
}

/// Apple со скрытой почтой — это имя без почты и без знаков, как у вошедшего
/// через Telegram: почту называет дело, названный адрес ещё не дверь, и
/// единственную дверь отвязать нельзя.
#[sqlx::test]
async fn an_apple_name_with_hidden_mail_behaves_like_a_telegram_name(pool: PgPool) {
    let w = world(pool).await;
    let r = w
        .apple(
            "ap-1",
            json!({ "email": "abc@privaterelay.appleid.com", "email_verified": "true", "is_private_email": "true" }),
            Some(r#"{"name":{"firstName":"Лиза","lastName":"К"}}"#),
        )
        .await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    assert!(r["user"]["email"].is_null());
    assert_eq!(r["user"]["displayName"], "Лиза К");

    // Дело (бронь, заказ) называет почту — один раз.
    let client = reqwest::Client::new();
    let named = client
        .post(format!("{}/api/v1/auth/email", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .json(&json!({ "email": "liza@example.com" }))
        .send()
        .await
        .unwrap();
    assert!(named.status().is_success());
    let user: Value = named.json().await.unwrap();
    assert_eq!(user["email"], "liza@example.com");
    assert_eq!(user["emailConfirmed"], false, "названный адрес стал дверью без письма");
    assert_eq!(user["appleLinked"], true);

    // Неподтверждённая почта дверью не считается: Apple — единственная дверь.
    let del = client
        .delete(format!("{}/api/v1/auth/oauth/apple/link", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap();
    assert_eq!(del.status(), reqwest::StatusCode::BAD_REQUEST);

    // Дорога назад к знакам закрыта, пока ящик не открыт: канала нет.
    let ask = client
        .post(format!("{}/api/v1/auth/signs/letter", w.addr))
        .header("authorization", format!("Bearer {session}"))
        .send()
        .await
        .unwrap();
    assert_eq!(ask.status(), reqwest::StatusCode::BAD_REQUEST);
}

/// Админ видит, чем это имя открывается.
#[sqlx::test]
async fn the_admin_sees_the_google_door(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-f1", google_person("adm@example.com")).await;
    let id = r["user"]["id"].as_str().unwrap();

    let detail: Value = reqwest::Client::new()
        .get(format!("{}/api/v1/admin/users/{id}", w.addr))
        .header("authorization", "Bearer test-admin-api-key-0123456789")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail["googleLinked"], true);
    assert_eq!(detail["googleEmail"], "adm@example.com");
    assert_eq!(detail["appleLinked"], false);
    assert_eq!(detail["email"], "adm@example.com");

}

/// Что считается дверью: список один на все отвязки.
#[test]
fn the_door_list_counts_every_door_the_same_way() {
    use gotiga_server::models::{Door, User};
    let user = |extra: Value| -> User {
        let mut base = json!({
            "id": Uuid::new_v4(), "displayName": "x", "isBlocked": false,
            "createdAt": "2026-10-12T00:00:00Z",
        });
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        serde_json::from_value(base).unwrap()
    };
    let mail = json!({ "email": "a@b.cd", "emailConfirmedAt": "2026-10-12T00:00:00Z", "visualPasswordHash": "h" });

    // Одна дверь — отвязывать нечего.
    assert_eq!(user(json!({ "googleSub": "g" })).doors_besides(Door::Google), 0);
    assert_eq!(user(json!({ "telegramId": 1 })).doors_besides(Door::Telegram), 0);
    // Почта без знаков или без подтверждения дверью не считается.
    assert_eq!(user(json!({ "googleSub": "g", "email": "a@b.cd", "emailConfirmedAt": "2026-10-12T00:00:00Z" })).doors_besides(Door::Google), 0);
    assert_eq!(user(json!({ "googleSub": "g", "email": "a@b.cd", "visualPasswordHash": "h" })).doors_besides(Door::Google), 0);
    // Почта со знаками, Telegram, второй провайдер — каждый считается.
    let mut with_mail = mail.clone();
    with_mail["googleSub"] = json!("g");
    assert_eq!(user(with_mail).doors_besides(Door::Google), 1);
    assert_eq!(user(json!({ "googleSub": "g", "appleSub": "a" })).doors_besides(Door::Google), 1);
    assert_eq!(user(json!({ "googleSub": "g", "telegramId": 5 })).doors_besides(Door::Google), 1);
    assert_eq!(user(json!({ "googleSub": "g", "telegramId": 5 })).doors_besides(Door::Telegram), 1);
}

// ═════════════════════════════════════════════════════════════════════════
// Возраст и сессия после сброса знаков
// ═════════════════════════════════════════════════════════════════════════

/// Новое имя без отметки возраста не заводится — вовсе, и сессии нет.
#[sqlx::test]
async fn a_new_name_needs_the_age_mark_and_without_it_is_not_created(pool: PgPool) {
    let w = world(pool).await;

    let refused = w.google_aged("g-age1", google_person("young@example.com"), false).await;
    assert_eq!(refused["state"], "failed");
    assert_eq!(refused["failure"], "age");
    assert!(refused["sessionToken"].is_null());
    let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&w.pool).await.unwrap();
    assert_eq!(made, 0, "имя заведено без подтверждения возраста");

    // С отметкой — заводится.
    let ok = w.google_aged("g-age1", google_person("young@example.com"), true).await;
    assert_eq!(ok["state"], "ready");
    assert_eq!(ok["isNew"], true);
}

/// Запрос к API вовсе без поля — то же самое, что «не отмечено»: обойти
/// проверку, не прислав галочку, нельзя.
#[sqlx::test]
async fn a_request_without_the_field_is_the_same_as_not_ticked(pool: PgPool) {
    let w = world(pool).await;
    let s: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/oauth/google/start", w.addr))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-age2", google_person("x@example.com"));
    w.callback_get("google", &format!("code=abc&state={state}")).await;
    let r = w.redeem(state, binder).await;
    assert_eq!(r["failure"], "age");
}

/// Возраст спрашивается только там, где появляется новый человек: вернувшийся,
/// привязка из профиля и слияние с именем по подтверждённой почте его не просят.
#[sqlx::test]
async fn the_age_mark_is_asked_only_where_a_new_person_appears(pool: PgPool) {
    let w = world(pool).await;

    // Вернувшийся.
    let first = w.google("g-age3", google_person("back@example.com")).await;
    let again = w.google_aged("g-age3", google_person("back@example.com"), false).await;
    assert_eq!(again["state"], "ready", "вернувшегося остановил вопрос о возрасте");
    assert_eq!(again["user"]["id"], first["user"]["id"]);

    // Слияние с именем, у которого та же подтверждённая почта.
    let (owner, _) = w.user("Хозяйка", Some("owner2@example.com"), true).await;
    let merged = w.google_aged("g-age4", google_person("owner2@example.com"), false).await;
    assert_eq!(merged["state"], "ready", "слияние остановил вопрос о возрасте");
    assert_eq!(merged["user"]["id"], owner.to_string().as_str());

    // Привязка из профиля.
    let (_, session) = w.user("Анна", Some("anna2@example.com"), true).await;
    let s = w.start_aged("google", Some(&session), None, false).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-age5", json!({}));
    w.callback_get("google", &format!("code=abc&state={state}")).await;
    assert_eq!(w.redeem(state, binder).await["state"], "ready", "привязку остановил вопрос о возрасте");
}

/// Та же проверка стоит и у Apple: дверь одна, правила одни.
#[sqlx::test]
async fn apple_asks_for_the_age_mark_too(pool: PgPool) {
    let w = world(pool).await;
    let s = w.start_aged("apple", None, None, false).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = apple_claims(&nonce, "ap-age", json!({}));
    w.callback_post("apple", &[("state", state), ("code", "c")]).await;
    assert_eq!(w.redeem(state, binder).await["failure"], "age");
}

/// Заблокированному после сброса знаков сессия не выдаётся.
#[sqlx::test]
async fn a_blocked_name_gets_no_session_from_a_reset(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-rst", google_person("blk@example.com")).await;
    let id = r["user"]["id"].as_str().unwrap().to_string();
    let key = Uuid::new_v4().to_string();
    sqlx::query("UPDATE users SET is_blocked = TRUE, password_reset_token = $2, password_reset_expires_at = NOW() + INTERVAL '2 hours' WHERE id = $1::uuid")
        .bind(&id)
        .bind(print_of(&key))
        .execute(&w.pool)
        .await
        .unwrap();
    let set: Value = reqwest::Client::new()
        .post(format!("{}/api/v1/auth/reset-password", w.addr))
        .json(&json!({ "token": key, "selections": SIGNS, "pool": [ANIMALS, DISHES, SEASONS, SYMBOLS] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(set["sessionToken"].is_null(), "заблокированному выдана сессия");
    let sessions: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_sessions").fetch_one(&w.pool).await.unwrap();
    assert_eq!(sessions, 0);
}

// ═════════════════════════════════════════════════════════════════════════
// Имя, блокировка при привязке, гонка отвязок
// ═════════════════════════════════════════════════════════════════════════

/// Имя видно другим людям, почта — нет: без имени от провайдера новое имя
/// называется по провайдеру, а не по части адреса до `@`.
#[sqlx::test]
async fn the_mailbox_is_not_turned_into_a_public_name(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-noname", json!({ "email": "ivan.petrov1987@example.com", "email_verified": true })).await;
    assert_eq!(r["state"], "ready");
    assert_eq!(r["isNew"], true);
    assert_eq!(r["user"]["displayName"], "Google");
    assert_eq!(r["user"]["email"], "ivan.petrov1987@example.com", "почта должна остаться почтой");
}

/// Привязку, начатую до блокировки, блокированному имени не дописывают.
#[sqlx::test]
async fn a_blocked_name_does_not_get_a_door_added(pool: PgPool) {
    let w = world(pool).await;
    let (id, session) = w.user("Анна", Some("anna@example.com"), true).await;

    let s = w.start("google", Some(&session), None).await;
    let (state, binder) = (s["state"].as_str().unwrap(), s["binder"].as_str().unwrap());
    let nonce = w.nonce_of(state).await;
    w.provider.lock().unwrap().claims = google_claims(&nonce, "g-late", json!({}));
    sqlx::query("UPDATE users SET is_blocked = TRUE WHERE id = $1").bind(id).execute(&w.pool).await.unwrap();
    w.callback_get("google", &format!("code=abc&state={state}")).await;

    let r = w.redeem(state, binder).await;
    assert_eq!(r["state"], "failed");
    assert_eq!(r["failure"], "blocked");
    let sub: Option<String> = sqlx::query_scalar("SELECT google_sub FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&w.pool)
        .await
        .unwrap();
    assert!(sub.is_none(), "блокированному имени привязан Google");
}

/// Две двери, две отвязки разом: одна из них обязана получить отказ. Прежде
/// каждая видела у другой её дверь на месте, и имя оставалось без единой.
#[sqlx::test]
async fn two_unlinks_at_once_cannot_take_the_last_two_doors(pool: PgPool) {
    let w = world(pool).await;
    let r = w.google("g-two", json!({ "name": "Двое" })).await;
    let session = r["sessionToken"].as_str().unwrap().to_string();
    // Вторая дверь — Telegram; почты со знаками нет.
    sqlx::query("UPDATE users SET telegram_id = 4242, telegram_username = 'dvoe', telegram_linked_at = NOW()")
        .execute(&w.pool)
        .await
        .unwrap();

    for _ in 0..5 {
        sqlx::query("UPDATE users SET telegram_id = 4242, telegram_username = 'dvoe', google_sub = 'g-two', google_email = NULL")
            .execute(&w.pool)
            .await
            .unwrap();
        let del = |path: &'static str| {
            let (addr, session) = (w.addr.clone(), session.clone());
            async move {
                reqwest::Client::new()
                    .delete(format!("{addr}/api/v1/auth/{path}"))
                    .header("authorization", format!("Bearer {session}"))
                    .send()
                    .await
                    .unwrap()
                    .status()
            }
        };
        let (a, b) = tokio::join!(del("oauth/google/link"), del("telegram/link"));
        let wins = [a, b].iter().filter(|s| s.is_success()).count();
        assert_eq!(wins, 1, "статусы {a} и {b}: должна пройти ровно одна отвязка");
        let doors: (Option<String>, Option<i64>) =
            sqlx::query_as("SELECT google_sub, telegram_id FROM users").fetch_one(&w.pool).await.unwrap();
        assert!(doors.0.is_some() || doors.1.is_some(), "имя осталось без единой двери");
    }
}
