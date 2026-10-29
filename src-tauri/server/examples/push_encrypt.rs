//! Шифрует сообщение так, как это делает рассылка уведомлений, и печатает тело запроса
//! (base64url). Нужен `tools/pwa/verify_push.mjs`: он расшифровывает его независимой
//! реализацией (Node) и этим проверяет, что шифрование сходится с RFC 8291, а не «само с собой».
//!
//! `cargo run --example push_encrypt -- <p256dh> <auth> <текст>`

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use gotiga_server::push;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, p256dh, auth, text] = &args[..] else {
        eprintln!("usage: push_encrypt <p256dh> <auth> <text>");
        std::process::exit(2);
    };
    let (public, secret) = push::decode_keys(p256dh, auth).expect("ключи подписки");
    let body = push::encrypt(&public, &secret, text.as_bytes()).expect("шифрование");
    println!("{}", B64.encode(body));
}
