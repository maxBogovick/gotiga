//! Вход через Google и Apple — только протокол, без базы и без правил дома.
//!
//! Тот же закон, что у `telegram.rs` и `sheet.rs`: модуль не знает ни про
//! `users`, ни про сессии. Он умеет три вещи — собрать ссылку к провайдеру,
//! обменять вернувшийся код на личность и проверить, что личность настоящая.
//! Кого впускать и что с этим делать, решает `services`.
//!
//! **Поток — код авторизации с PKCE, через переход, а не всплывающее окно и не
//! чужой скрипт на странице.**
//! - Страница ничего не грузит у провайдера: ни `gsi/client`, ни `appleid.auth.js`.
//!   Нет стороннего JS (а значит, нет строки в CSP и нет слежки провайдера за
//!   каждым посетителем), нет зависимости от сторонних cookie, которые браузеры
//!   режут, и нет всплывающих окон, которые режут блокировщики.
//! - Секрет клиента лежит на сервере. Браузер видит только одноразовый `code`,
//!   который без секрета ничего не стоит.
//! - Личность берётся из `id_token`, пришедшего **с сервера провайдера** в ответ
//!   на обмен, а не из браузера. Подпись при этом всё равно проверяется (RS256
//!   по ключам провайдера): TLS-соединение — это защита, а не доказательство, и
//!   проверка нужна на случай, когда эту строку однажды начнут принимать
//!   откуда-то ещё.
//! - Алгоритм подписи **закреплён** (`RS256`), а не берётся из заголовка токена:
//!   иначе токен сам выбирал бы, чем его проверять.

use crate::error::{AppError, Result};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use jsonwebtoken::jwk::{Jwk, JwkSet};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const GOOGLE_AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_KEYS: &str = "https://www.googleapis.com/oauth2/v3/certs";
/// Google называет себя двумя способами — оба настоящие.
const GOOGLE_ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

const APPLE_AUTHORIZE: &str = "https://appleid.apple.com/auth/authorize";
const APPLE_TOKEN: &str = "https://appleid.apple.com/auth/token";
const APPLE_KEYS: &str = "https://appleid.apple.com/auth/keys";
const APPLE_ISSUER: &str = "https://appleid.apple.com";

/// Сколько ключи провайдера живут в памяти. Они меняются редко и заранее
/// объявляются новыми, а неизвестный `kid` всё равно вынуждает перечитать.
const KEYS_TTL: Duration = Duration::from_secs(6 * 3600);
/// Чаще раза в минуту список ключей не перечитывается, как бы ни просили:
/// токен с выдуманным `kid` иначе заставлял бы сервер ходить к провайдеру на
/// каждый свой запрос.
const KEYS_REFETCH_GAP: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Provider {
    Google,
    Apple,
}

impl Provider {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "google" => Some(Self::Google),
            "apple" => Some(Self::Apple),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Apple => "apple",
        }
    }
}

/// Кто пришёл — в том виде, в каком это сказал провайдер, и больше ничего.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Постоянный номер человека у провайдера. По нему и только по нему узнают
    /// вернувшегося.
    pub subject: String,
    pub email: Option<String>,
    /// Провайдер ручается, что человек владеет этим ящиком.
    pub email_verified: bool,
    /// Apple: адрес-прокси (`…@privaterelay.appleid.com`). Письма на него
    /// доходят, только если домен отправителя зарегистрирован у Apple, —
    /// поэтому дом не делает его своей почтой.
    pub private_email: bool,
    /// Только у Google: имя приходит в `id_token`. Apple отдаёт его отдельно и
    /// один раз (`AppleUser`).
    pub name: Option<String>,
}

#[derive(Clone)]
struct Google {
    client_id: String,
    client_secret: String,
}

#[derive(Clone)]
struct Apple {
    /// Services ID, а не Bundle ID приложения.
    client_id: String,
    team_id: String,
    key_id: String,
    key: Arc<EncodingKey>,
}

/// Адреса провайдеров, к которым ходит сервер сам (обмен кода и ключи подписи).
/// Ссылки для браузера (`*_AUTHORIZE`) сюда не входят: их открывает человек, а
/// не сервер. Меняется набор только в проверках — они поднимают поддельного
/// провайдера на своём порту (`Oauth::with_endpoints`).
#[derive(Clone, Debug)]
pub struct Endpoints {
    pub google_token: String,
    pub google_keys: String,
    pub apple_token: String,
    pub apple_keys: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            google_token: GOOGLE_TOKEN.into(),
            google_keys: GOOGLE_KEYS.into(),
            apple_token: APPLE_TOKEN.into(),
            apple_keys: APPLE_KEYS.into(),
        }
    }
}

#[derive(Clone)]
pub struct Oauth {
    http: reqwest::Client,
    base: String,
    google: Option<Google>,
    apple: Option<Apple>,
    endpoints: Endpoints,
    keys: Arc<Mutex<HashMap<Provider, (Instant, JwkSet)>>>,
}

/// Что нужно знать модулю о настройках: только то, что лежит в окружении.
pub struct Settings<'a> {
    pub public_url: &'a str,
    pub google: Option<(&'a str, &'a str)>,
    pub apple: Option<AppleSettings<'a>>,
}

pub struct AppleSettings<'a> {
    pub client_id: &'a str,
    pub team_id: &'a str,
    pub key_id: &'a str,
    pub private_key: &'a str,
}

impl Oauth {
    pub fn new(http: reqwest::Client, s: Settings<'_>) -> Result<Self> {
        let apple = match s.apple {
            None => None,
            Some(a) => {
                let key = apple_key(a.private_key).map_err(AppError::Internal)?;
                Some(Apple {
                    client_id: a.client_id.to_string(),
                    team_id: a.team_id.to_string(),
                    key_id: a.key_id.to_string(),
                    key: Arc::new(key),
                })
            }
        };
        Ok(Self {
            http,
            base: s.public_url.trim_end_matches('/').to_string(),
            google: s.google.map(|(id, secret)| Google {
                client_id: id.to_string(),
                client_secret: secret.to_string(),
            }),
            apple,
            endpoints: Endpoints::default(),
            keys: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Только для проверок: подменить адреса провайдеров поддельным.
    pub fn with_endpoints(mut self, endpoints: Endpoints) -> Self {
        self.endpoints = endpoints;
        self
    }

    pub fn enabled(&self, p: Provider) -> bool {
        match p {
            Provider::Google => self.google.is_some(),
            Provider::Apple => self.apple.is_some(),
        }
    }

    /// Куда провайдер вернёт человека. Этот адрес должен быть записан у
    /// провайдера **буква в букву** — иначе он откажет, не объясняя причины.
    pub fn redirect_uri(&self, p: Provider) -> String {
        format!("{}/api/v1/auth/oauth/{}/callback", self.base, p.as_str())
    }

    /// Ссылка, на которую уходит браузер.
    pub fn authorize_url(&self, p: Provider, state: &str, nonce: &str, challenge: &str) -> Result<String> {
        let redirect = self.redirect_uri(p);
        let (base, params): (&str, Vec<(&str, &str)>) = match p {
            Provider::Google => {
                let g = self.google.as_ref().ok_or_else(|| not_set_up(p))?;
                (
                    GOOGLE_AUTHORIZE,
                    vec![
                        ("client_id", g.client_id.as_str()),
                        ("redirect_uri", redirect.as_str()),
                        ("response_type", "code"),
                        // Имя и почта, больше ничего: ни картинки, ни списка контактов.
                        ("scope", "openid email profile"),
                        ("state", state),
                        ("nonce", nonce),
                        ("code_challenge", challenge),
                        ("code_challenge_method", "S256"),
                        // Выбор аккаунта показывается всегда: на чужом
                        // компьютере тихий вход под тем, кто там уже вошёл,
                        // был бы входом не в то имя.
                        ("prompt", "select_account"),
                    ],
                )
            }
            Provider::Apple => {
                let a = self.apple.as_ref().ok_or_else(|| not_set_up(p))?;
                (
                    APPLE_AUTHORIZE,
                    vec![
                        ("client_id", a.client_id.as_str()),
                        ("redirect_uri", redirect.as_str()),
                        ("response_type", "code"),
                        // Имя и почту Apple отдаёт только в `form_post`.
                        ("response_mode", "form_post"),
                        ("scope", "name email"),
                        ("state", state),
                        ("nonce", nonce),
                    ],
                )
            }
        };
        let url = reqwest::Url::parse_with_params(base, &params)
            .map_err(|e| AppError::Internal(format!("oauth authorize url: {e}")))?;
        Ok(url.into())
    }

    /// Обменять вернувшийся код на личность. Всё, что идёт не так, — одна и та
    /// же ошибка наружу: странице незачем знать, на каком шаге оборвалось, а
    /// журнал знает.
    pub async fn exchange(&self, p: Provider, code: &str, verifier: &str, nonce: &str) -> Result<Identity> {
        let redirect = self.redirect_uri(p);
        let (url, form, audience): (String, Vec<(&str, String)>, String) = match p {
            Provider::Google => {
                let g = self.google.as_ref().ok_or_else(|| not_set_up(p))?;
                (
                    self.endpoints.google_token.clone(),
                    vec![
                        ("grant_type", "authorization_code".into()),
                        ("code", code.into()),
                        ("redirect_uri", redirect),
                        ("client_id", g.client_id.clone()),
                        ("client_secret", g.client_secret.clone()),
                        ("code_verifier", verifier.into()),
                    ],
                    g.client_id.clone(),
                )
            }
            Provider::Apple => {
                let a = self.apple.as_ref().ok_or_else(|| not_set_up(p))?;
                let secret = apple_client_secret(&a.key, &a.team_id, &a.key_id, &a.client_id)?;
                (
                    self.endpoints.apple_token.clone(),
                    vec![
                        ("grant_type", "authorization_code".into()),
                        ("code", code.into()),
                        ("redirect_uri", redirect),
                        ("client_id", a.client_id.clone()),
                        ("client_secret", secret),
                    ],
                    a.client_id.clone(),
                )
            }
        };

        let res = self
            .http
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("{} token request: {e}", p.as_str())))?;
        let ok = res.status().is_success();
        let body: TokenResponse = res
            .json()
            .await
            .map_err(|e| AppError::BadRequest(format!("{} token reply: {e}", p.as_str())))?;
        if !ok || body.id_token.is_none() {
            // Код ошибки называет сам провайдер (`invalid_grant`, `invalid_client`…).
            // Секретов в нём нет, и по нему видно, что чинить: чаще всего это
            // `redirect_uri_mismatch` или неверный секрет.
            return Err(AppError::BadRequest(format!(
                "{} refused the code: {}",
                p.as_str(),
                body.error.as_deref().unwrap_or("no id_token")
            )));
        }
        let id_token = body.id_token.unwrap_or_default();

        let header = jsonwebtoken::decode_header(&id_token)
            .map_err(|e| AppError::BadRequest(format!("{} id_token header: {e}", p.as_str())))?;
        let kid = header
            .kid
            .ok_or_else(|| AppError::BadRequest(format!("{} id_token has no kid", p.as_str())))?;
        let jwk = self.key(p, &kid).await?;
        check_id_token(p, &id_token, &jwk, &audience, nonce)
    }

    /// Ключ провайдера по `kid`: из памяти, а если такого нет или список
    /// устарел, то перечитанный.
    async fn key(&self, p: Provider, kid: &str) -> Result<Jwk> {
        let mut cache = self.keys.lock().await;
        if let Some((at, set)) = cache.get(&p) {
            let age = at.elapsed();
            if let Some(jwk) = set.find(kid)
                && age < KEYS_TTL
            {
                return Ok(jwk.clone());
            }
            if age < KEYS_REFETCH_GAP {
                return Err(AppError::BadRequest(format!("{} signing key {kid} is unknown", p.as_str())));
            }
        }
        let url = match p {
            Provider::Google => &self.endpoints.google_keys,
            Provider::Apple => &self.endpoints.apple_keys,
        };
        let raw: serde_json::Value = self
            .http
            .get(url.as_str())
            .send()
            .await
            .map_err(|e| AppError::BadRequest(format!("{} keys: {e}", p.as_str())))?
            .json()
            .await
            .map_err(|e| AppError::BadRequest(format!("{} keys reply: {e}", p.as_str())))?;
        let set = parse_key_set(&raw);
        let found = set.find(kid).cloned();
        cache.insert(p, (Instant::now(), set));
        found.ok_or_else(|| AppError::BadRequest(format!("{} signing key {kid} is unknown", p.as_str())))
    }
}

fn not_set_up(p: Provider) -> AppError {
    AppError::BadRequest(format!("{} sign-in is not set up", p.as_str()))
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: Option<String>,
    error: Option<String>,
}

/// Список ключей читается по одному: ключ незнакомого рода (у провайдера их
/// бывает несколько) не должен ронять разбор остальных.
fn parse_key_set(raw: &serde_json::Value) -> JwkSet {
    let keys = raw
        .get("keys")
        .and_then(|k| k.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|k| serde_json::from_value::<Jwk>(k.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    JwkSet { keys }
}

// ── Проверка личности ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Claims {
    sub: String,
    nonce: Option<String>,
    email: Option<String>,
    #[serde(default, deserialize_with = "flexible_bool")]
    email_verified: bool,
    #[serde(default, deserialize_with = "flexible_bool")]
    is_private_email: bool,
    name: Option<String>,
}

/// Apple присылает булевы строками (`"true"`), Google — настоящими. Читаются
/// оба вида, потому что «не разобрали» здесь значило бы «почта не подтверждена».
fn flexible_bool<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Bool(bool),
        Text(String),
    }
    Ok(match Either::deserialize(d)? {
        Either::Bool(b) => b,
        Either::Text(s) => s.eq_ignore_ascii_case("true"),
    })
}

/// Подпись, издатель, адресат, срок и одноразовое число — всё разом.
///
/// Отдельной функцией без сети: именно на ней стоит проверка, и именно её
/// можно прогнать на подделанных токенах, не поднимая провайдера.
pub fn check_id_token(p: Provider, id_token: &str, jwk: &Jwk, audience: &str, nonce: &str) -> Result<Identity> {
    let bad = |what: &str| AppError::BadRequest(format!("{} id_token: {what}", p.as_str()));
    let key = DecodingKey::from_jwk(jwk).map_err(|e| bad(&format!("key: {e}")))?;

    // Алгоритм закреплён. Оба провайдера подписывают RS256, и всё прочее —
    // это не «другой допустимый вариант», а попытка подменить проверку.
    let mut v = Validation::new(Algorithm::RS256);
    v.set_audience(&[audience]);
    match p {
        Provider::Google => v.set_issuer(&GOOGLE_ISSUERS),
        Provider::Apple => v.set_issuer(&[APPLE_ISSUER]),
    }
    v.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    v.leeway = 30;

    let claims = jsonwebtoken::decode::<Claims>(id_token, &key, &v)
        .map_err(|e| bad(&e.to_string()))?
        .claims;

    // Число, которое мы положили в ссылку, обязано вернуться тем же: иначе
    // токен был выписан не для этого входа, а для чужого (повторное
    // предъявление).
    if claims.nonce.as_deref() != Some(nonce) {
        return Err(bad("nonce does not match"));
    }
    if claims.sub.is_empty() || claims.sub.len() > 255 {
        return Err(bad("bad subject"));
    }
    Ok(Identity {
        subject: claims.sub,
        email: claims.email.filter(|e| !e.trim().is_empty()),
        email_verified: claims.email_verified,
        private_email: claims.is_private_email,
        name: claims.name.filter(|n| !n.trim().is_empty()),
    })
}

// ── PKCE и случайные числа ───────────────────────────────────────────────

/// Случайная строка из `bytes` байт, безопасная для адреса.
pub fn random_token(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Проверочная строка PKCE и её отпечаток (метод `S256`, RFC 7636).
pub fn pkce_pair() -> (String, String) {
    let verifier = random_token(48);
    let challenge = pkce_challenge(&verifier);
    (verifier, challenge)
}

pub fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

// ── Apple: ключ и секрет клиента ─────────────────────────────────────────

/// Секрет клиента Apple — не строка из консоли, а подписанная подпись: JWT,
/// который сервер выпускает сам закрытым ключом `.p8`. Живёт пять минут и
/// выпускается на каждый обмен, поэтому кэшировать его, обновлять и хранить
/// нечего (Apple разрешает до шести месяцев — это для тех, кто не хочет
/// подписывать каждый раз).
pub fn apple_client_secret(key: &EncodingKey, team_id: &str, key_id: &str, client_id: &str) -> Result<String> {
    #[derive(Serialize)]
    struct Claims<'a> {
        iss: &'a str,
        iat: i64,
        exp: i64,
        aud: &'static str,
        sub: &'a str,
    }
    let now = chrono::Utc::now().timestamp();
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key_id.to_string());
    jsonwebtoken::encode(
        &header,
        &Claims { iss: team_id, iat: now, exp: now + 300, aud: APPLE_ISSUER, sub: client_id },
        key,
    )
    .map_err(|e| AppError::Internal(format!("apple client secret: {e}")))
}

/// Закрытый ключ из окружения.
///
/// Файл `.p8` многострочный, а `.env` не любит многострочных значений, поэтому
/// принимаются все удобные виды: сам PEM, PEM с `\n` вместо переводов строк,
/// base64 от всего файла и base64 от одного тела ключа. Угадывать, какой из
/// них положили, должен дом, а не человек в три часа ночи.
pub fn apple_key(raw: &str) -> std::result::Result<EncodingKey, String> {
    let from_pem = |pem: &str| {
        EncodingKey::from_ec_pem(pem.as_bytes())
            .map_err(|e| format!("APPLE_PRIVATE_KEY is not a P-256 key in PKCS#8 form: {e}"))
    };
    let text = raw.trim().trim_matches('"').replace("\\n", "\n");
    if text.contains("-----BEGIN") {
        return from_pem(&text);
    }
    // Разобрать как base64 можно и тело ключа, и весь файл: смотрим, что вышло.
    // Пробелы и переводы строк внутри base64 не мешают.
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = STANDARD.decode(compact.as_bytes()).map_err(|_| {
        "APPLE_PRIVATE_KEY is neither a PEM nor base64 — paste the contents of the AuthKey_XXXX.p8 file".to_string()
    })?;
    match String::from_utf8(bytes.clone()) {
        Ok(pem) if pem.contains("-----BEGIN") => from_pem(&pem),
        _ => Ok(EncodingKey::from_ec_der(&bytes)),
    }
}

/// Проверка при запуске: ключ подписывает по-настоящему. Разбор PEM ключ не
/// проверяет — ошибка вылезла бы на первом входе первого человека.
pub fn check_apple_setup(raw_key: &str, team_id: &str, key_id: &str, client_id: &str) -> std::result::Result<(), String> {
    let key = apple_key(raw_key)?;
    apple_client_secret(&key, team_id, key_id, client_id)
        .map(|_| ())
        .map_err(|e| format!("APPLE_PRIVATE_KEY cannot sign: {e}"))
}

/// Apple присылает имя только при самом первом согласии, и только в теле
/// callback: `user={"name":{"firstName":"…","lastName":"…"},"email":"…"}`.
#[derive(Debug, Deserialize)]
pub struct AppleUser {
    pub name: Option<AppleName>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppleName {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

impl AppleUser {
    /// Имя для профиля. Приходит из браузера, поэтому остаётся именем и
    /// только: ничего, кроме имени, из этого поля не берётся.
    pub fn display_name(raw: &str) -> Option<String> {
        let user: AppleUser = serde_json::from_str(raw).ok()?;
        let name = user.name?;
        let joined = [name.first_name, name.last_name]
            .into_iter()
            .flatten()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        (!joined.is_empty()).then_some(joined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::jwk::{AlgorithmParameters, CommonParameters, RSAKeyParameters, RSAKeyType};

    // Пара ключей только для этих тестов: ни к чему настоящему не подходит.
    const RSA_PRIVATE: &str = include_str!("../tests/fixtures/oauth_test_rsa.pem");
    const RSA_N: &str = include_str!("../tests/fixtures/oauth_test_rsa.n");
    const RSA_OTHER_N: &str = include_str!("../tests/fixtures/oauth_test_rsa2.n");
    const EC_PRIVATE: &str = include_str!("../tests/fixtures/oauth_test_ec.p8");

    const AUD: &str = "client-123";

    fn jwk(kid: &str) -> Jwk {
        Jwk {
            common: CommonParameters { key_id: Some(kid.into()), ..Default::default() },
            algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                key_type: RSAKeyType::RSA,
                n: RSA_N.trim().into(),
                e: "AQAB".into(),
            }),
        }
    }

    fn token(iss: &str, aud: &str, nonce: &str, extra: serde_json::Value, exp_in: i64, alg: Algorithm) -> String {
        let now = chrono::Utc::now().timestamp();
        let mut claims = serde_json::json!({
            "iss": iss, "aud": aud, "sub": "10769150350006150715113082367",
            "iat": now, "exp": now + exp_in, "nonce": nonce,
        });
        for (k, v) in extra.as_object().cloned().unwrap_or_default() {
            claims[k] = v;
        }
        let mut h = Header::new(alg);
        h.kid = Some("k1".into());
        jsonwebtoken::encode(&h, &claims, &EncodingKey::from_rsa_pem(RSA_PRIVATE.as_bytes()).unwrap()).unwrap()
    }

    #[test]
    fn pkce_matches_rfc_7636_appendix_b() {
        assert_eq!(
            pkce_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let (v, c) = pkce_pair();
        assert_eq!(c, pkce_challenge(&v));
        assert!(v.len() >= 43, "RFC 7636: the verifier is 43–128 characters");
    }

    #[test]
    fn a_good_google_token_gives_the_person() {
        let t = token("https://accounts.google.com", AUD, "n1",
            serde_json::json!({"email": "masha@example.com", "email_verified": true, "name": "Маша"}), 600, Algorithm::RS256);
        let who = check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").unwrap();
        assert_eq!(who.subject, "10769150350006150715113082367");
        assert_eq!(who.email.as_deref(), Some("masha@example.com"));
        assert!(who.email_verified && !who.private_email);
        assert_eq!(who.name.as_deref(), Some("Маша"));
    }

    #[test]
    fn google_may_name_itself_without_the_scheme() {
        let t = token("accounts.google.com", AUD, "n1", serde_json::json!({}), 600, Algorithm::RS256);
        assert!(check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").is_ok());
    }

    #[test]
    fn apple_booleans_arrive_as_strings() {
        let t = token("https://appleid.apple.com", AUD, "n1",
            serde_json::json!({"email": "x@privaterelay.appleid.com", "email_verified": "true", "is_private_email": "true"}),
            600, Algorithm::RS256);
        let who = check_id_token(Provider::Apple, &t, &jwk("k1"), AUD, "n1").unwrap();
        assert!(who.email_verified && who.private_email);
    }

    #[test]
    fn unverified_mail_stays_unverified_when_the_field_is_missing() {
        let t = token("https://accounts.google.com", AUD, "n1", serde_json::json!({"email": "a@b.c"}), 600, Algorithm::RS256);
        let who = check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").unwrap();
        assert!(!who.email_verified);
    }

    #[test]
    fn a_token_for_someone_else_is_refused() {
        let ok = |iss: &str, aud: &str, nonce: &str, exp: i64| {
            let t = token(iss, aud, nonce, serde_json::json!({}), exp, Algorithm::RS256);
            check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").is_ok()
        };
        assert!(ok("https://accounts.google.com", AUD, "n1", 600));
        assert!(!ok("https://evil.example", AUD, "n1", 600), "wrong issuer");
        assert!(!ok("https://accounts.google.com", "other-client", "n1", 600), "wrong audience");
        assert!(!ok("https://accounts.google.com", AUD, "n2", 600), "wrong nonce");
        assert!(!ok("https://accounts.google.com", AUD, "n1", -3600), "expired");
    }

    #[test]
    fn apple_issuer_does_not_pass_as_google() {
        let t = token("https://appleid.apple.com", AUD, "n1", serde_json::json!({}), 600, Algorithm::RS256);
        assert!(check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").is_err());
    }

    #[test]
    fn a_token_without_the_nonce_is_refused() {
        let mut claims = serde_json::json!({
            "iss": "https://accounts.google.com", "aud": AUD, "sub": "1",
            "exp": chrono::Utc::now().timestamp() + 600,
        });
        claims["iat"] = 0.into();
        let mut h = Header::new(Algorithm::RS256);
        h.kid = Some("k1".into());
        let t = jsonwebtoken::encode(&h, &claims, &EncodingKey::from_rsa_pem(RSA_PRIVATE.as_bytes()).unwrap()).unwrap();
        assert!(check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").is_err());
    }

    #[test]
    fn a_signature_by_another_key_is_refused() {
        let t = token("https://accounts.google.com", AUD, "n1", serde_json::json!({}), 600, Algorithm::RS256);
        let mut other = jwk("k1");
        if let AlgorithmParameters::RSA(p) = &mut other.algorithm {
            // Тот же показатель, другой модуль — ключ, которым этот токен не подписывали.
            p.n = RSA_OTHER_N.trim().into();
        }
        assert!(check_id_token(Provider::Google, &t, &other, AUD, "n1").is_err());
    }

    #[test]
    fn the_algorithm_is_pinned_not_taken_from_the_token() {
        // HS256, подписанный открытым ключом как секретом, — классическая
        // подмена. Закреплённый RS256 такой токен не принимает.
        let now = chrono::Utc::now().timestamp();
        let claims = serde_json::json!({"iss": "https://accounts.google.com", "aud": AUD, "sub": "1", "exp": now + 600, "nonce": "n1"});
        let mut h = Header::new(Algorithm::HS256);
        h.kid = Some("k1".into());
        let t = jsonwebtoken::encode(&h, &claims, &EncodingKey::from_secret(RSA_N.trim().as_bytes())).unwrap();
        assert!(check_id_token(Provider::Google, &t, &jwk("k1"), AUD, "n1").is_err());
    }

    #[test]
    fn the_key_set_survives_a_key_of_an_unknown_kind() {
        let raw = serde_json::json!({"keys": [
            {"kty": "OKP", "crv": "Ed25519", "x": "AAAA", "kid": "weird"},
            {"kty": "RSA", "kid": "k1", "n": RSA_N.trim(), "e": "AQAB", "alg": "RS256", "use": "sig"},
        ]});
        let set = parse_key_set(&raw);
        assert!(set.find("k1").is_some());
    }

    #[test]
    fn apple_key_is_taken_in_every_convenient_shape() {
        let pem = EC_PRIVATE.trim();
        let body: String = pem.lines().filter(|l| !l.starts_with("-----")).collect::<Vec<_>>().join("");
        let shapes = [
            pem.to_string(),
            pem.replace('\n', "\\n"),
            format!("\"{}\"", pem.replace('\n', "\\n")),
            STANDARD.encode(pem),
            body,
        ];
        for s in shapes {
            let key = apple_key(&s).unwrap_or_else(|e| panic!("shape not accepted: {e}\n{s}"));
            let secret = apple_client_secret(&key, "TEAM123456", "KEY1234567", "com.example.web").unwrap();
            let h = jsonwebtoken::decode_header(&secret).unwrap();
            assert_eq!(h.alg, Algorithm::ES256);
            assert_eq!(h.kid.as_deref(), Some("KEY1234567"));
        }
    }

    #[test]
    fn a_wrong_apple_key_is_named_at_startup() {
        assert!(check_apple_setup("not a key at all!", "T", "K", "C").is_err());
        assert!(check_apple_setup(EC_PRIVATE, "T", "K", "C").is_ok());
        // Ключ другого рода (RSA) годится для разбора, но не для ES256.
        assert!(check_apple_setup(RSA_PRIVATE, "T", "K", "C").is_err());
    }

    #[test]
    fn apple_name_comes_only_as_a_name() {
        let raw = r#"{"name":{"firstName":"Мария","lastName":"Иванова"},"email":"x@y.z","isAdmin":true}"#;
        assert_eq!(AppleUser::display_name(raw).as_deref(), Some("Мария Иванова"));
        assert_eq!(AppleUser::display_name(r#"{"name":{"firstName":"  "}}"#), None);
        assert_eq!(AppleUser::display_name("not json"), None);
        assert_eq!(AppleUser::display_name("{}"), None);
    }

    #[test]
    fn the_links_carry_what_the_providers_require() {
        let o = Oauth::new(
            reqwest::Client::new(),
            Settings {
                public_url: "https://ritunia.com/",
                google: Some(("g-id", "g-secret")),
                apple: Some(AppleSettings {
                    client_id: "com.ritunia.web",
                    team_id: "TEAM123456",
                    key_id: "KEY1234567",
                    private_key: EC_PRIVATE,
                }),
            },
        )
        .unwrap();
        assert_eq!(o.redirect_uri(Provider::Google), "https://ritunia.com/api/v1/auth/oauth/google/callback");

        let g = reqwest::Url::parse(&o.authorize_url(Provider::Google, "st", "no", "ch").unwrap()).unwrap();
        let q: HashMap<_, _> = g.query_pairs().into_owned().collect();
        assert_eq!(g.host_str(), Some("accounts.google.com"));
        assert_eq!(q["response_type"], "code");
        assert_eq!(q["code_challenge_method"], "S256");
        assert_eq!(q["scope"], "openid email profile");
        assert_eq!(q["state"], "st");
        assert!(!g.as_str().contains("g-secret"), "the client secret never goes into a link");

        let a = reqwest::Url::parse(&o.authorize_url(Provider::Apple, "st", "no", "ch").unwrap()).unwrap();
        let q: HashMap<_, _> = a.query_pairs().into_owned().collect();
        assert_eq!(q["response_mode"], "form_post");
        assert_eq!(q["scope"], "name email");
        assert_eq!(q["redirect_uri"], "https://ritunia.com/api/v1/auth/oauth/apple/callback");
    }
}
