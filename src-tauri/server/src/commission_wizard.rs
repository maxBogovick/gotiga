//! Мастер заявки: закрытые списки ответов и разбор способов связи.
//!
//! Без базы и без сети. ЭТОТ СПИСОК ДОЛЖЕН МЕНЯТЬСЯ ВМЕСТЕ с `src/lib/requestChoices.ts`:
//! мастер на странице и стол администратора берут ключи оттуда, а сервер сверяет по этой
//! таблице. Расхождение значит, что выбранное человеком молча пропадёт.

use serde::{Deserialize, Serialize};

pub const CONTACT_CHANNELS: [&str; 9] = [
    "email",
    "telegram",
    "whatsapp",
    "viber",
    "signal",
    "instagram",
    "facebook",
    "x",
    "phone",
];

pub const MAX_CONTACT_LEN: usize = 200;
pub const MAX_CHOICES_PER_LIST: usize = 12;

pub const WISHES: [&str; 2] = ["similar", "new"];
pub const SIZE_KEYS: [&str; 6] = ["xs", "s", "m", "l", "xl", "unsure"];
pub const DEADLINE_KINDS: [&str; 3] = ["none", "date", "asap"];
pub const BUDGET_KEYS: [&str; 5] = ["lt100", "100_200", "200_400", "gt400", "discuss"];
pub const BUDGET_CURRENCIES: [&str; 2] = ["USD", "EUR"];

pub const KEEP_CHANGE_KEYS: [&str; 6] = ["look", "pose", "clothes", "colors", "props", "size"];
pub const WHO_KEYS: [&str; 7] = [
    "witch", "granny", "gnome", "girl", "old_man", "creature", "other",
];
pub const MOOD_KEYS: [&str; 5] = ["kind", "sly", "dark", "funny", "calm"];
pub const PROPS_KEYS: [&str; 8] = [
    "broom", "cat", "pumpkin", "book", "lantern", "herbs", "bird", "other",
];

/// Способ связи, как его прислал мастер.
#[derive(Debug, Clone, Deserialize)]
pub struct ContactInput {
    pub channel: String,
    pub value: String,
}

/// Способ связи после разбора: значение в одном виде на канал.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub channel: String,
    pub value: String,
}

/// Выбранные признаки. Лишние ключи отбрасываются при разборе (`normalize_choices`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Choices {
    #[serde(default)]
    pub keep: Vec<String>,
    #[serde(default)]
    pub change: Vec<String>,
    #[serde(default)]
    pub who: Vec<String>,
    #[serde(default)]
    pub mood: Vec<String>,
    #[serde(default)]
    pub props: Vec<String>,
}

fn only_known(list: &[String], allowed: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in list {
        let key = item.trim();
        if allowed.contains(&key) && !out.iter().any(|k| k == key) {
            out.push(key.to_string());
        }
        if out.len() >= MAX_CHOICES_PER_LIST {
            break;
        }
    }
    out
}

/// Оставляет только известные ключи, без повторов, не больше 12 в списке.
pub fn normalize_choices(c: &Choices) -> Choices {
    Choices {
        keep: only_known(&c.keep, &KEEP_CHANGE_KEYS),
        change: only_known(&c.change, &KEEP_CHANGE_KEYS),
        who: only_known(&c.who, &WHO_KEYS),
        mood: only_known(&c.mood, &MOOD_KEYS),
        props: only_known(&c.props, &PROPS_KEYS),
    }
}

pub fn looks_like_email(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() || s.len() > MAX_CONTACT_LEN || s.chars().any(char::is_whitespace) {
        return false;
    }
    match s.split_once('@') {
        Some((local, domain)) => {
            !local.is_empty()
                && !domain.contains('@')
                && domain.contains('.')
                && !domain.starts_with('.')
                && !domain.ends_with('.')
        }
        None => false,
    }
}

/// Хост и путь из адреса без схемы: `https://www.t.me/name?x=1` → (`t.me`, `name`).
fn split_url(raw: &str) -> Option<(String, String, bool)> {
    let s = raw.trim();
    let lower = s.to_ascii_lowercase();
    let (rest, has_scheme) = if let Some(r) = lower.strip_prefix("https://") {
        (&s[s.len() - r.len()..], true)
    } else if let Some(r) = lower.strip_prefix("http://") {
        (&s[s.len() - r.len()..], true)
    } else {
        (s, false)
    };
    let (host, path) = rest.split_once('/')?;
    let host = host.to_ascii_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    Some((host, path.to_string(), has_scheme))
}

fn first_segment(path: &str) -> &str {
    let cut = path.find(['?', '#']).unwrap_or(path.len());
    path[..cut].split('/').find(|p| !p.is_empty()).unwrap_or("")
}

fn handle_chars_ok(h: &str, min: usize, max: usize, dots: bool) -> bool {
    let n = h.chars().count();
    n >= min
        && n <= max
        && h.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || (dots && c == '.'))
}

fn handle_of(
    value: &str,
    hosts: &[&str],
    min: usize,
    max: usize,
    dots: bool,
) -> Result<String, &'static str> {
    let v = value.trim();
    let handle = match split_url(v) {
        Some((host, path, _)) if hosts.contains(&host.as_str()) => {
            first_segment(&path).to_string()
        }
        Some(_) => return Err("The profile link is from another network"),
        None => {
            if v.contains("://") || hosts.iter().any(|h| v.to_ascii_lowercase() == *h) {
                return Err("The profile link is not valid");
            }
            v.trim_start_matches('@').to_string()
        }
    };
    let handle = handle.trim_start_matches('@').to_string();
    if handle_chars_ok(&handle, min, max, dots) {
        Ok(handle)
    } else {
        Err("The username is not valid")
    }
}

fn phone_of(value: &str) -> Result<String, &'static str> {
    let v = value.trim();
    let mut out = String::new();
    for (i, c) in v.chars().enumerate() {
        match c {
            '+' if i == 0 => out.push('+'),
            '0'..='9' => out.push(c),
            ' ' | '-' | '(' | ')' | '.' | '\u{a0}' => {}
            _ => return Err("The phone number has unexpected characters"),
        }
    }
    let digits = out.chars().filter(char::is_ascii_digit).count();
    if (7..=15).contains(&digits) {
        Ok(out)
    } else {
        Err("The phone number must have 7 to 15 digits")
    }
}

fn facebook_of(value: &str) -> Result<String, &'static str> {
    let v = value.trim();
    match split_url(v) {
        Some((host, path, _)) => {
            if !matches!(host.as_str(), "facebook.com" | "fb.com" | "m.facebook.com") {
                return Err("The profile link is from another network");
            }
            let seg = first_segment(&path);
            if seg == "profile.php" {
                let id = path
                    .split(['?', '&'])
                    .find_map(|p| p.strip_prefix("id="))
                    .map(|p| p.split('#').next().unwrap_or(""))
                    .unwrap_or("");
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && id.len() <= 20 {
                    return Ok(format!("profile.php?id={id}"));
                }
                return Err("The profile link is not valid");
            }
            if handle_chars_ok(seg, 5, 50, true) {
                Ok(seg.to_string())
            } else {
                Err("The profile link is not valid")
            }
        }
        None => {
            if v.contains("://") {
                return Err("The profile link is not valid");
            }
            let name = v.trim_start_matches('@');
            if handle_chars_ok(name, 5, 50, true) {
                Ok(name.to_string())
            } else {
                Err("The username is not valid")
            }
        }
    }
}

/// Приводит одно значение к виду, в котором оно хранится.
pub fn normalize_contact(channel: &str, value: &str) -> Result<String, String> {
    if value.chars().count() > MAX_CONTACT_LEN {
        return Err("A contact value is too long".into());
    }
    let v = value.trim();
    let done = match channel {
        "email" => {
            if looks_like_email(v) {
                Ok(v.to_ascii_lowercase())
            } else {
                Err("A valid email is required")
            }
        }
        "telegram" => handle_of(v, &["t.me", "telegram.me", "telegram.dog"], 5, 32, false),
        "instagram" => handle_of(v, &["instagram.com", "instagr.am"], 1, 30, true),
        "x" => handle_of(v, &["x.com", "twitter.com", "mobile.twitter.com"], 1, 15, false),
        "whatsapp" | "viber" | "signal" | "phone" => phone_of(v),
        "facebook" => facebook_of(v),
        _ => return Err(format!("Unknown contact channel: {channel}")),
    };
    done.map_err(|e| format!("{e} ({channel})"))
}

/// Разбирает список способов связи: пустые значения пропускаются, повтор канала и
/// неизвестный канал — отказ.
pub fn normalize_contacts(input: &[ContactInput]) -> Result<Vec<Contact>, String> {
    if input.len() > CONTACT_CHANNELS.len() {
        return Err("Too many contacts".into());
    }
    let mut out: Vec<Contact> = Vec::new();
    for c in input {
        let channel = c.channel.trim().to_ascii_lowercase();
        if !CONTACT_CHANNELS.contains(&channel.as_str()) {
            return Err(format!("Unknown contact channel: {}", c.channel));
        }
        if c.value.trim().is_empty() {
            continue;
        }
        if out.iter().any(|o| o.channel == channel) {
            return Err(format!("Contact channel repeated: {channel}"));
        }
        let value = normalize_contact(&channel, &c.value)?;
        out.push(Contact { channel, value });
    }
    Ok(out)
}

/// Название канала для письма хозяину.
pub fn channel_label(channel: &str) -> &'static str {
    match channel {
        "email" => "Email",
        "telegram" => "Telegram",
        "whatsapp" => "WhatsApp",
        "viber" => "Viber",
        "signal" => "Signal",
        "instagram" => "Instagram",
        "facebook" => "Facebook",
        "x" => "X",
        "phone" => "Телефон",
        _ => "—",
    }
}

fn word(group: &str, key: &str) -> String {
    let w = match (group, key) {
        ("keep" | "change", "look") => "облик",
        ("keep" | "change", "pose") => "поза",
        ("keep" | "change", "clothes") => "одежда",
        ("keep" | "change", "colors") => "цвета",
        ("keep" | "change", "props") => "предметы",
        ("keep" | "change", "size") => "размер",
        ("who", "witch") => "ведьма",
        ("who", "granny") => "бабушка",
        ("who", "gnome") => "гном",
        ("who", "girl") => "девочка",
        ("who", "old_man") => "старик",
        ("who", "creature") => "существо",
        ("mood", "kind") => "добрая",
        ("mood", "sly") => "хитрая",
        ("mood", "dark") => "мрачная",
        ("mood", "funny") => "смешная",
        ("mood", "calm") => "спокойная",
        ("props", "broom") => "метла",
        ("props", "cat") => "кот",
        ("props", "pumpkin") => "тыква",
        ("props", "book") => "книга",
        ("props", "lantern") => "фонарь",
        ("props", "herbs") => "травы",
        ("props", "bird") => "птица",
        (_, "other") => "другое",
        _ => key,
    };
    w.to_string()
}

pub fn wish_word(wish: &str) -> &'static str {
    match wish {
        "similar" => "похожую на существующую",
        "new" => "совсем новую",
        _ => "—",
    }
}

pub fn size_word(key: &str) -> &'static str {
    match key {
        "xs" => "XS",
        "s" => "S",
        "m" => "M",
        "l" => "L",
        "xl" => "XL",
        "unsure" => "не знает",
        _ => "—",
    }
}

pub fn budget_word(key: &str) -> &'static str {
    match key {
        "lt100" => "до 100",
        "100_200" => "100–200",
        "200_400" => "200–400",
        "gt400" => "от 400",
        "discuss" => "обсудим",
        _ => "—",
    }
}

/// Выбранные признаки простыми словами, по строке на список.
pub fn choices_lines(c: &Choices) -> Vec<String> {
    let mut lines = Vec::new();
    for (title, group, list) in [
        ("Оставить", "keep", &c.keep),
        ("Изменить", "change", &c.change),
        ("Кто", "who", &c.who),
        ("Настроение", "mood", &c.mood),
        ("Предметы", "props", &c.props),
    ] {
        if !list.is_empty() {
            let words: Vec<String> = list.iter().map(|k| word(group, k)).collect();
            lines.push(format!("{title}: {}", words.join(", ")));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(channel: &str, value: &str) -> Result<String, String> {
        normalize_contact(channel, value)
    }

    #[test]
    fn email_is_lowercased() {
        assert_eq!(n("email", " Anna@Mail.COM ").unwrap(), "anna@mail.com");
        assert!(n("email", "anna@mail").is_err());
        assert!(n("email", "an na@mail.com").is_err());
    }

    #[test]
    fn handles_come_from_names_and_links() {
        assert_eq!(n("telegram", "@rita_art").unwrap(), "rita_art");
        assert_eq!(n("telegram", "https://t.me/rita_art").unwrap(), "rita_art");
        assert_eq!(n("telegram", "t.me/rita_art?start=1").unwrap(), "rita_art");
        assert!(n("telegram", "https://instagram.com/rita_art").is_err());
        assert!(n("telegram", "@ab").is_err());
        assert_eq!(n("instagram", "https://www.instagram.com/rita.art/").unwrap(), "rita.art");
        assert_eq!(n("x", "https://twitter.com/rita").unwrap(), "rita");
        assert!(n("x", "@way_too_long_for_x_handle").is_err());
    }

    #[test]
    fn phones_keep_plus_and_digits() {
        assert_eq!(n("whatsapp", "+7 (900) 123-45-67").unwrap(), "+79001234567");
        assert_eq!(n("phone", "8 900 1234567").unwrap(), "89001234567");
        assert!(n("viber", "12345").is_err());
        assert!(n("signal", "+1234567890123456").is_err());
        assert!(n("phone", "call me").is_err());
        assert!(n("phone", "12+3456789").is_err());
    }

    #[test]
    fn facebook_takes_links_and_names() {
        assert_eq!(n("facebook", "https://www.facebook.com/rita.art").unwrap(), "rita.art");
        assert_eq!(n("facebook", "https://m.facebook.com/rita.art/").unwrap(), "rita.art");
        assert_eq!(n("facebook", "rita.art").unwrap(), "rita.art");
        assert_eq!(
            n("facebook", "https://facebook.com/profile.php?id=12345").unwrap(),
            "profile.php?id=12345"
        );
        assert!(n("facebook", "https://example.com/rita.art").is_err());
    }

    #[test]
    fn contacts_reject_unknown_channel_and_repeats() {
        let c = |ch: &str, v: &str| ContactInput { channel: ch.into(), value: v.into() };
        assert!(normalize_contacts(&[c("icq", "123")]).is_err());
        assert!(normalize_contacts(&[c("phone", "+79001234567"), c("phone", "+79001234568")]).is_err());
        let ok = normalize_contacts(&[c("phone", "+7 900 123 45 67"), c("email", "")]).unwrap();
        assert_eq!(ok.len(), 1);
    }

    #[test]
    fn choices_drop_unknown_keys_and_repeats() {
        let c = Choices {
            keep: vec!["look".into(), "look".into(), "nonsense".into()],
            who: vec!["witch".into(), "dragon".into()],
            ..Default::default()
        };
        let n = normalize_choices(&c);
        assert_eq!(n.keep, vec!["look"]);
        assert_eq!(n.who, vec!["witch"]);
        assert!(n.mood.is_empty());
    }
}
