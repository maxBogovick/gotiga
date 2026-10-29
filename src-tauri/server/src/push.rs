//! Web Push: шифрование сообщения для браузера (RFC 8291, `aes128gcm` по RFC 8188) и
//! подпись сервера (VAPID, RFC 8292). Без базы и без правил сайта — как `telegram.rs` и
//! `oauth.rs`: что и кому слать, решает `services/push.rs`.
//!
//! Всё на `ring`, который сервер уже тянет через `reqwest` и `jsonwebtoken`: отдельный
//! крейт для шифрования уведомлений не нужен.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use ring::{
    aead, agreement, hkdf,
    rand::{SecureRandom, SystemRandom},
    signature::{self, EcdsaKeyPair, KeyPair},
};

/// Размер записи по RFC 8188. Сообщение у нас всегда в одну запись.
const RECORD_SIZE: u32 = 4096;
/// Предел текста: запись минус 16 байт метки, 1 байт разделителя и заголовок ключа (86 байт).
pub const MAX_PLAINTEXT: usize = 3993;
/// Сколько живёт подпись сервера. Предел RFC 8292 — сутки; половина даёт запас на часы.
const VAPID_TTL_SECS: u64 = 12 * 3600;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Ключи подписчика не того размера или вида.
    BadKeys,
    /// Текст не помещается в одну запись.
    TooLong,
    /// Сбой шифрования или подписи.
    Crypto,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Error::BadKeys => "the subscription keys are not valid",
            Error::TooLong => "the notification text is too long",
            Error::Crypto => "cryptographic failure",
        })
    }
}

impl std::error::Error for Error {}

// ── Подпись сервера (VAPID) ────────────────────────────────────────────────

/// Пара ключей сервера. Открытая половина уходит браузеру при подписке и входит в каждый
/// запрос; закрытая подписывает короткий токен для каждого узла рассылки.
pub struct Vapid {
    key: EcdsaKeyPair,
}

impl Vapid {
    /// Новая пара; результат — PKCS#8, его кладут в настройки и читают `from_pkcs8`.
    pub fn generate_pkcs8() -> Result<Vec<u8>, Error> {
        let rng = SystemRandom::new();
        let doc = EcdsaKeyPair::generate_pkcs8(&signature::ECDSA_P256_SHA256_FIXED_SIGNING, &rng)
            .map_err(|_| Error::Crypto)?;
        Ok(doc.as_ref().to_vec())
    }

    pub fn from_pkcs8(pkcs8: &[u8]) -> Result<Self, Error> {
        let rng = SystemRandom::new();
        let key = EcdsaKeyPair::from_pkcs8(&signature::ECDSA_P256_SHA256_FIXED_SIGNING, pkcs8, &rng)
            .map_err(|_| Error::Crypto)?;
        Ok(Self { key })
    }

    /// Открытый ключ в виде, которого ждёт `pushManager.subscribe` (65 байт, base64url).
    pub fn public_key(&self) -> String {
        B64.encode(self.key.public_key().as_ref())
    }

    /// Заголовок `Authorization` для узла рассылки `audience` (схема и узел, без пути).
    /// `subject` — `mailto:` или https-адрес того, кто отвечает за рассылку.
    pub fn authorization(&self, audience: &str, subject: &str, now_unix: u64) -> Result<String, Error> {
        let header = B64.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims = B64.encode(
            format!(
                r#"{{"aud":"{}","exp":{},"sub":"{}"}}"#,
                json_escape(audience),
                now_unix + VAPID_TTL_SECS,
                json_escape(subject)
            )
            .as_bytes(),
        );
        let signing_input = format!("{header}.{claims}");
        let rng = SystemRandom::new();
        let sig = self.key.sign(&rng, signing_input.as_bytes()).map_err(|_| Error::Crypto)?;
        Ok(format!(
            "vapid t={signing_input}.{}, k={}",
            B64.encode(sig.as_ref()),
            self.public_key()
        ))
    }
}

fn json_escape(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            c if (c as u32) < 0x20 => vec![' '],
            c => vec![c],
        })
        .collect()
}

// ── Шифрование сообщения ───────────────────────────────────────────────────

struct Len(usize);

impl hkdf::KeyType for Len {
    fn len(&self) -> usize {
        self.0
    }
}

fn expand(prk: &hkdf::Prk, info: &[u8], len: usize) -> Result<Vec<u8>, Error> {
    let info = [info];
    let okm = prk.expand(&info, Len(len)).map_err(|_| Error::Crypto)?;
    let mut out = vec![0u8; len];
    okm.fill(&mut out).map_err(|_| Error::Crypto)?;
    Ok(out)
}

/// Разбирает ключи подписки, как их присылает браузер (`p256dh` — 65 байт, `auth` — 16).
pub fn decode_keys(p256dh: &str, auth: &str) -> Result<(Vec<u8>, Vec<u8>), Error> {
    let public = B64.decode(p256dh.trim_end_matches('=')).map_err(|_| Error::BadKeys)?;
    let secret = B64.decode(auth.trim_end_matches('=')).map_err(|_| Error::BadKeys)?;
    if public.len() != 65 || public[0] != 4 || secret.len() != 16 {
        return Err(Error::BadKeys);
    }
    Ok((public, secret))
}

/// Тело запроса: заголовок записи (соль, размер, открытый ключ сервера) и шифртекст.
/// Новая пара ключей на каждое сообщение, как требует RFC 8291.
pub fn encrypt(ua_public: &[u8], auth_secret: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
    if ua_public.len() != 65 || ua_public[0] != 4 || auth_secret.len() != 16 {
        return Err(Error::BadKeys);
    }
    if plaintext.len() > MAX_PLAINTEXT {
        return Err(Error::TooLong);
    }
    let rng = SystemRandom::new();
    let as_private = agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng)
        .map_err(|_| Error::Crypto)?;
    let as_public = as_private.compute_public_key().map_err(|_| Error::Crypto)?;
    let as_public = as_public.as_ref().to_vec();
    let ua = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, ua_public);

    let mut salt = [0u8; 16];
    rng.fill(&mut salt).map_err(|_| Error::Crypto)?;

    let (cek, nonce) = agreement::agree_ephemeral(as_private, &ua, |ecdh| -> Result<_, Error> {
        // IKM = HKDF(auth_secret, ecdh, "WebPush: info" 0x00 ua_public as_public, 32)
        let mut key_info = b"WebPush: info\0".to_vec();
        key_info.extend_from_slice(ua_public);
        key_info.extend_from_slice(&as_public);
        let prk_key = hkdf::Salt::new(hkdf::HKDF_SHA256, auth_secret).extract(ecdh);
        let ikm = expand(&prk_key, &key_info, 32)?;
        // CEK и NONCE — из соли записи (RFC 8188).
        let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, &salt).extract(&ikm);
        let cek = expand(&prk, b"Content-Encoding: aes128gcm\0", 16)?;
        let nonce = expand(&prk, b"Content-Encoding: nonce\0", 12)?;
        Ok((cek, nonce))
    })
    .map_err(|_| Error::BadKeys)??;

    // Запись: текст и разделитель 0x02 (последняя запись), без добавочных нулей.
    let mut record = Vec::with_capacity(plaintext.len() + 1 + 16);
    record.extend_from_slice(plaintext);
    record.push(0x02);
    let key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_128_GCM, &cek).map_err(|_| Error::Crypto)?,
    );
    let nonce = aead::Nonce::try_assume_unique_for_key(&nonce).map_err(|_| Error::Crypto)?;
    key.seal_in_place_append_tag(nonce, aead::Aad::empty(), &mut record)
        .map_err(|_| Error::Crypto)?;

    let mut body = Vec::with_capacity(16 + 4 + 1 + 65 + record.len());
    body.extend_from_slice(&salt);
    body.extend_from_slice(&RECORD_SIZE.to_be_bytes());
    body.push(as_public.len() as u8);
    body.extend_from_slice(&as_public);
    body.extend_from_slice(&record);
    Ok(body)
}

// ── Адрес подписки ─────────────────────────────────────────────────────────

/// Узлы рассылки, которым мы доверяем. Адрес подписки присылает браузер, то есть кто угодно,
/// а сервер сам идёт по нему POST-запросом: без этого списка ручка подписки — способ
/// заставить сервер стучаться в любой адрес внутри сети (SSRF).
const PUSH_HOSTS: &[&str] = &[
    "fcm.googleapis.com",           // Chrome, Edge, Opera, Samsung, Brave и прочие на Chromium
    "push.services.mozilla.com",    // Firefox (updates.push.services.mozilla.com и др.)
    "push.apple.com",               // Safari, приложения на экране «Домой» (web.push.apple.com)
    "notify.windows.com",           // Edge на Windows (wns*.notify.windows.com)
];

/// `https://узел/путь` → `(схема и узел, узел)`; `None`, если адрес не годится для рассылки.
pub fn endpoint_audience(endpoint: &str) -> Option<(String, String)> {
    if endpoint.len() > 1000 {
        return None;
    }
    let rest = endpoint.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    // Без имени пользователя, без порта (кроме обычного) и без чего-либо, кроме имени узла.
    let host = authority.strip_suffix(":443").unwrap_or(authority).to_ascii_lowercase();
    if host.is_empty()
        || !host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
    {
        return None;
    }
    let trusted = PUSH_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")));
    trusted.then(|| (format!("https://{host}"), host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_whitelist() {
        for ok in [
            "https://fcm.googleapis.com/fcm/send/abc",
            "https://updates.push.services.mozilla.com/wpush/v2/xyz",
            "https://web.push.apple.com/QL5",
            "https://wns2-par02p.notify.windows.com/w/?token=1",
            "https://fcm.googleapis.com:443/fcm/send/abc",
        ] {
            assert!(endpoint_audience(ok).is_some(), "{ok}");
        }
        for bad in [
            "http://fcm.googleapis.com/x",
            "https://evil.example/fcm.googleapis.com",
            "https://fcm.googleapis.com.evil.example/x",
            "https://user@fcm.googleapis.com/x",
            "https://fcm.googleapis.com:8443/x",
            "https://127.0.0.1/x",
            "https://localhost/x",
            "https://notfcm.googleapis.com.example/x",
            "https://xfcm.googleapis.com/x",
            "",
        ] {
            assert!(endpoint_audience(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn audience_is_scheme_and_host() {
        let (aud, host) = endpoint_audience("https://FCM.googleapis.com/fcm/send/abc?x=1").unwrap();
        assert_eq!(aud, "https://fcm.googleapis.com");
        assert_eq!(host, "fcm.googleapis.com");
    }

    #[test]
    fn vapid_header_is_a_valid_es256_token() {
        let pkcs8 = Vapid::generate_pkcs8().unwrap();
        let vapid = Vapid::from_pkcs8(&pkcs8).unwrap();
        let header = vapid
            .authorization("https://fcm.googleapis.com", "https://example.com", 1_000)
            .unwrap();
        let rest = header.strip_prefix("vapid t=").unwrap();
        let (token, key) = rest.split_once(", k=").unwrap();
        assert_eq!(key, vapid.public_key());
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let sig = B64.decode(parts[2]).unwrap();
        assert_eq!(sig.len(), 64, "ES256 в JWT — r‖s по 32 байта");
        let public = B64.decode(key).unwrap();
        assert_eq!(public.len(), 65);
        let verify = signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_FIXED, &public);
        verify
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig)
            .expect("подпись сходится с открытым ключом");
        let claims = String::from_utf8(B64.decode(parts[1]).unwrap()).unwrap();
        assert!(claims.contains(r#""aud":"https://fcm.googleapis.com""#));
        assert!(claims.contains(&format!(r#""exp":{}"#, 1_000 + VAPID_TTL_SECS)));
    }

    #[test]
    fn keys_must_have_the_right_shape() {
        assert!(decode_keys("AAAA", "AAAA").is_err());
        let ua = vec![4u8; 65];
        assert_eq!(encrypt(&ua[..64], &[0; 16], b"x"), Err(Error::BadKeys));
        assert_eq!(encrypt(&ua, &[0; 15], b"x"), Err(Error::BadKeys));
        assert_eq!(encrypt(&ua, &[0; 16], &vec![0; MAX_PLAINTEXT + 1]), Err(Error::TooLong));
    }

    #[test]
    fn message_decrypts_with_the_subscriber_keys() {
        // Подписчик: своя пара ключей и секрет `auth`.
        let rng = SystemRandom::new();
        let ua_private =
            agreement::EphemeralPrivateKey::generate(&agreement::ECDH_P256, &rng).unwrap();
        let ua_public = ua_private.compute_public_key().unwrap().as_ref().to_vec();
        let mut auth = [0u8; 16];
        rng.fill(&mut auth).unwrap();

        let text = "Новая работа · New work".as_bytes();
        let body = encrypt(&ua_public, &auth, text).unwrap();

        // Заголовок записи.
        let salt = &body[..16];
        assert_eq!(&body[16..20], &RECORD_SIZE.to_be_bytes());
        assert_eq!(body[20], 65);
        let as_public = &body[21..86];
        let ciphertext = &body[86..];

        // Расшифровка со стороны браузера.
        let as_pub = agreement::UnparsedPublicKey::new(&agreement::ECDH_P256, as_public);
        let (cek, nonce) = agreement::agree_ephemeral(ua_private, &as_pub, |ecdh| -> Result<_, Error> {
            let mut key_info = b"WebPush: info\0".to_vec();
            key_info.extend_from_slice(&ua_public);
            key_info.extend_from_slice(as_public);
            let prk_key = hkdf::Salt::new(hkdf::HKDF_SHA256, &auth).extract(ecdh);
            let ikm = expand(&prk_key, &key_info, 32)?;
            let prk = hkdf::Salt::new(hkdf::HKDF_SHA256, salt).extract(&ikm);
            Ok((
                expand(&prk, b"Content-Encoding: aes128gcm\0", 16)?,
                expand(&prk, b"Content-Encoding: nonce\0", 12)?,
            ))
        })
        .unwrap()
        .unwrap();
        let key = aead::LessSafeKey::new(aead::UnboundKey::new(&aead::AES_128_GCM, &cek).unwrap());
        let mut buf = ciphertext.to_vec();
        let plain = key
            .open_in_place(
                aead::Nonce::try_assume_unique_for_key(&nonce).unwrap(),
                aead::Aad::empty(),
                &mut buf,
            )
            .unwrap();
        assert_eq!(plain.last(), Some(&0x02), "разделитель последней записи");
        assert_eq!(&plain[..plain.len() - 1], text);
    }
}
