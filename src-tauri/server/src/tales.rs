//! Читатель небылиц после последней строки — чистые помощники.
//!
//! Без базы и без сети, как `sheet.rs` и `telegram.rs`: здесь собирается
//! текст письма о байке, ссылка на канал и крупная фотография для ленты
//! Pinterest. Служба решает, кому и когда; здесь — что именно.

use crate::lang::Lang;

/// Почему человеку пришло письмо о байке. Порядок — порядок старшинства: если
/// адрес подходит под два повода, письмо уходит по первому, потому что он
/// ближе к тому, о чём человек просил сам.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterReason {
    /// Просил продолжение той байки, которую эта продолжает.
    Sequel,
    /// Голосовал, о ком записать следующую, — и вот она.
    Chosen,
    /// Вписан в книгу дома.
    New,
}

impl LetterReason {
    pub fn as_str(self) -> &'static str {
        match self {
            LetterReason::Sequel => "sequel",
            LetterReason::Chosen => "chosen",
            LetterReason::New => "new",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "sequel" => Some(LetterReason::Sequel),
            "chosen" => Some(LetterReason::Chosen),
            "new" => Some(LetterReason::New),
            _ => None,
        }
    }
}

/// Всё, из чего собирается письмо. Заглавие и эпиграф — на языке письма,
/// ссылки — уже целиком.
pub struct TaleLetter<'a> {
    pub reason: LetterReason,
    pub lang: Lang,
    pub title: &'a str,
    pub dek: Option<&'a str>,
    pub link: &'a str,
    /// Заглавие начала — только у продолжения.
    pub original: Option<&'a str>,
    /// Имя работы, которую выбрали читатели, — только у выбранной.
    pub chosen_work: Option<&'a str>,
    /// Дверь из книги дома, если адрес в ней стоит.
    pub unsubscribe: Option<&'a str>,
}

/// Тема и тело письма. Текст простой, без разметки: `send_mail` шлёт
/// `text/plain`, и письмо должно читаться так же, как напечатано.
pub fn compose_letter(l: &TaleLetter) -> (String, String) {
    let title = l.title.trim();
    let dek = l.dek.map(str::trim).filter(|d| !d.is_empty());
    let original = l.original.map(str::trim).filter(|s| !s.is_empty());
    let chosen = l.chosen_work.map(str::trim).filter(|s| !s.is_empty());

    let subject = match (l.reason, l.lang) {
        (LetterReason::Sequel, Lang::Ru) => format!("Продолжение: {title}"),
        (LetterReason::Sequel, Lang::En) => format!("The story goes on: {title}"),
        (LetterReason::Sequel, Lang::Uk) => format!("Продовження: {title}"),
        (LetterReason::Chosen, Lang::Ru) => format!("Небылица, которую выбрали читатели: {title}"),
        (LetterReason::Chosen, Lang::En) => format!("The tale the readers chose: {title}"),
        (LetterReason::Chosen, Lang::Uk) => format!("Небилиця, яку обрали читачі: {title}"),
        (LetterReason::New, Lang::Ru) => format!("Новая небылица: {title}"),
        (LetterReason::New, Lang::En) => format!("A new tall tale: {title}"),
        (LetterReason::New, Lang::Uk) => format!("Нова небилиця: {title}"),
    };

    let opening = match (l.reason, l.lang) {
        (LetterReason::Sequel, Lang::Ru) => match original {
            Some(o) => format!("Вы просили продолжение небылицы «{o}». Оно записано."),
            None => "Вы просили продолжение небылицы. Оно записано.".to_string(),
        },
        (LetterReason::Sequel, Lang::En) => match original {
            Some(o) => format!("You asked for more of “{o}”. It has been written down."),
            None => "You asked for more of a tale. It has been written down.".to_string(),
        },
        (LetterReason::Sequel, Lang::Uk) => match original {
            Some(o) => format!("Ви просили продовження небилиці «{o}». Воно записане."),
            None => "Ви просили продовження небилиці. Воно записане.".to_string(),
        },
        (LetterReason::Chosen, Lang::Ru) => match chosen {
            Some(w) => format!(
                "Вы голосовали, о ком записать следующую небылицу. Читатели выбрали «{w}» — и вот она."
            ),
            None => "Вы голосовали, о ком записать следующую небылицу. Вот она.".to_string(),
        },
        (LetterReason::Chosen, Lang::En) => match chosen {
            Some(w) => format!(
                "You voted on whose tale should be written next. The readers chose “{w}” — and here it is."
            ),
            None => "You voted on whose tale should be written next. Here it is.".to_string(),
        },
        (LetterReason::Chosen, Lang::Uk) => match chosen {
            Some(w) => format!(
                "Ви голосували, про кого записати наступну небилицю. Читачі обрали «{w}» — і ось вона."
            ),
            None => "Ви голосували, про кого записати наступну небилицю. Ось вона.".to_string(),
        },
        (LetterReason::New, Lang::Ru) => "В доме записана новая небылица.".to_string(),
        (LetterReason::New, Lang::En) => "A new tall tale has been written down in the house.".to_string(),
        (LetterReason::New, Lang::Uk) => "У домі записана нова небилиця.".to_string(),
    };

    let read = l.lang.pick("Read it", "Читать", "Читати");
    let mut body = format!("{opening}\n\n{title}\n");
    if let Some(d) = dek {
        body.push_str(d);
        body.push('\n');
    }
    body.push_str(&format!("\n{read}: {}\n", l.link));

    // Подвал. У письма из книги дома — дверь из неё; у письма по просьбе —
    // слова о том, что оно одно: человек просил сообщить один раз, и дом не
    // заводит на этом рассылку.
    body.push_str("\n—\n");
    match (l.unsubscribe, l.reason, l.lang) {
        (Some(door), _, Lang::Ru) => body.push_str(&format!(
            "Письмо пришло, потому что ваше имя вписано в книгу дома. Покинуть её: {door}\n"
        )),
        (Some(door), _, Lang::En) => body.push_str(&format!(
            "This letter came because your name is in the house book. To leave it: {door}\n"
        )),
        (Some(door), _, Lang::Uk) => body.push_str(&format!(
            "Лист прийшов, бо ваше ім’я вписане до книги дому. Залишити її: {door}\n"
        )),
        (None, LetterReason::New, Lang::Ru) => {
            body.push_str("Письмо пришло, потому что ваше имя вписано в книгу дома.\n")
        }
        (None, LetterReason::New, Lang::En) => {
            body.push_str("This letter came because your name is in the house book.\n")
        }
        (None, LetterReason::New, Lang::Uk) => {
            body.push_str("Лист прийшов, бо ваше ім’я вписане до книги дому.\n")
        }
        (None, _, Lang::Ru) => body.push_str("Это письмо одно: вы просили сообщить, и дом сообщил.\n"),
        (None, _, Lang::En) => {
            body.push_str("This is the only letter: you asked to be told, and the house has told you.\n")
        }
        (None, _, Lang::Uk) => {
            body.push_str("Цей лист один: ви просили повідомити, і дім повідомив.\n")
        }
    }
    (subject, body)
}

/// Записка в Telegram о вышедшем продолжении и подпись её кнопки. Бот пишет
/// HTML-разметкой (`parse_mode: HTML`), поэтому заглавия экранируются; ссылка
/// едет кнопкой, а не строкой в тексте.
pub fn compose_note(lang: Lang, title: &str, original: Option<&str>) -> (String, &'static str) {
    use crate::telegram::esc;
    let title = esc(title.trim());
    let original = original.map(str::trim).filter(|s| !s.is_empty()).map(esc);
    let text = match (lang, original) {
        (Lang::Ru, Some(o)) => format!(
            "<b>Продолжение записано</b>\nВы просили продолжение небылицы «{o}». Вот оно: «{title}»."
        ),
        (Lang::Ru, None) => format!("<b>Продолжение записано</b>\nВы просили продолжение — вот оно: «{title}»."),
        (Lang::En, Some(o)) => format!(
            "<b>The story goes on</b>\nYou asked for more of “{o}”. Here it is: “{title}”."
        ),
        (Lang::En, None) => format!("<b>The story goes on</b>\nYou asked for more — here it is: “{title}”."),
        (Lang::Uk, Some(o)) => format!(
            "<b>Продовження записане</b>\nВи просили продовження небилиці «{o}». Ось воно: «{title}»."
        ),
        (Lang::Uk, None) => format!("<b>Продовження записане</b>\nВи просили продовження — ось воно: «{title}»."),
    };
    (text, lang.pick("Read it", "Читать", "Читати"))
}

/// Записка подписчику книги в Telegram о новой небылице. Заглавие и вводка
/// экранируются (бот пишет HTML-разметкой), выход — командой, которая
/// работает без сайта.
pub fn compose_book_note(lang: Lang, title: &str, dek: Option<&str>) -> (String, &'static str) {
    use crate::telegram::esc;
    let title = esc(title.trim());
    let dek = dek.map(str::trim).filter(|s| !s.is_empty()).map(esc);
    let lead = lang.pick("A new tall tale", "Новая небылица", "Нова небилиця");
    let stop = lang.pick("To stop these: /stop", "Не писать мне: /stop", "Не писати мені: /stop");
    let text = match dek {
        Some(d) => format!("<b>{lead}</b>\n«{title}»\n{d}\n\n{stop}"),
        None => format!("<b>{lead}</b>\n«{title}»\n\n{stop}"),
    };
    (text, lang.pick("Read it", "Читать", "Читати"))
}

/// Публичная ссылка на канал по его имени в настройках. `@имя` даёт
/// `https://t.me/имя`; числовой `-100…` ссылки не имеет вовсе — у закрытого
/// канала её нет, а выдуманная вела бы в никуда.
pub fn channel_link(channel_id: &str) -> Option<String> {
    let name = channel_id.trim().strip_prefix('@')?.trim();
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_');
    ok.then(|| format!("https://t.me/{name}"))
}

/// Самая крупная версия загруженной фотографии.
///
/// Лист хранит миниатюру (`/images/thumb/…`), а Pinterest из миниатюры делает
/// мутный пин. Версии лежат рядом под тем же именем, и крупнейшая из
/// публичных — `preview`; то же самое делает `resolveLargestImageUrl` на
/// клиенте. Чужой адрес и адрес без знакомого каталога возвращаются как есть.
pub fn largest_rendition(url: &str) -> String {
    for small in ["/images/thumb/", "/images/medium/"] {
        if let Some(at) = url.find(small) {
            let mut out = String::with_capacity(url.len() + 4);
            out.push_str(&url[..at]);
            out.push_str("/images/preview/");
            out.push_str(&url[at + small.len()..]);
            return out;
        }
    }
    url.to_string()
}

/// Тело байки без её скромной разметки: строка `# …` — заголовок, `**…**` —
/// жирный, строка из одного `✦` — орнамент. В описании пина и в письме это
/// печаталось бы звёздочками и решётками.
pub fn plain_prose(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .filter(|line| *line != "✦")
        .map(|line| line.strip_prefix("# ").unwrap_or(line).replace("**", ""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Описание пина: эпиграф, затем начало текста. Pinterest режет описание
/// около пятисот знаков, поэтому — меньше, и по слову.
pub fn pin_description(dek: Option<&str>, body: Option<&str>, max: usize) -> String {
    let dek = dek.map(str::trim).filter(|d| !d.is_empty());
    let prose = body.map(plain_prose).filter(|p| !p.trim().is_empty());
    let joined = match (dek, prose) {
        (Some(d), Some(p)) => format!("{d} — {p}"),
        (Some(d), None) => d.to_string(),
        (None, Some(p)) => p,
        (None, None) => String::new(),
    };
    crate::gazette::excerpt(&joined, max)
}

/// Адрес, по которому дом готов написать. Та же мера, что у книги дома и у
/// слежки за эскизом: знак `@` и не длиннее двухсот знаков.
pub fn usable_email(raw: Option<&str>) -> Option<String> {
    let email = raw?.trim();
    (email.contains('@') && email.len() <= 200 && !email.contains(char::is_whitespace))
        .then(|| email.to_string())
}

/// Строка на языке читателя, а если её нет — на другом: байка без русского
/// заглавия всё равно байка. Украинскому читателю прежде русского показывают
/// английское: непереведённое не должно быть русским по умолчанию. Пустые и
/// пробельные строки не в счёт.
pub fn in_lang(
    lang: Lang,
    en_side: Option<&str>,
    ru_side: Option<&str>,
    uk_side: Option<&str>,
) -> Option<String> {
    crate::lang::in_lang(lang, en_side, ru_side, uk_side)
}

/// Язык письма: три, всё остальное — английский.
pub fn letter_lang(raw: Option<&str>) -> &'static str {
    raw.and_then(|r| Lang::parse(r)).unwrap_or(Lang::En).code()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letter(reason: LetterReason, lang: Lang, unsubscribe: Option<&str>) -> (String, String) {
        compose_letter(&TaleLetter {
            reason,
            lang,
            title: "Свидетель физики",
            dek: Some("Мне не нужны факты"),
            link: "https://ritunia.com/tales/ya-veryu?src=letter",
            original: Some("Лесник"),
            chosen_work: Some("Witch"),
            unsubscribe,
        })
    }

    #[test]
    fn every_letter_carries_its_tale_and_the_way_to_it() {
        for reason in [LetterReason::Sequel, LetterReason::Chosen, LetterReason::New] {
            for lang in [Lang::Ru, Lang::En, Lang::Uk] {
                let (subject, body) = letter(reason, lang, None);
                assert!(subject.contains("Свидетель физики"), "{subject}");
                assert!(body.contains("Мне не нужны факты"), "{body}");
                assert!(body.contains("https://ritunia.com/tales/ya-veryu?src=letter"), "{body}");
            }
        }
    }

    #[test]
    fn the_letter_names_what_was_asked_for() {
        let (_, sequel) = letter(LetterReason::Sequel, Lang::Ru, None);
        assert!(sequel.contains("«Лесник»"), "{sequel}");
        let (_, chosen) = letter(LetterReason::Chosen, Lang::En, None);
        assert!(chosen.contains("“Witch”"), "{chosen}");
    }

    #[test]
    fn a_book_letter_carries_the_door_out_and_a_requested_one_says_it_is_single() {
        let (_, book) = letter(LetterReason::New, Lang::Ru, Some("https://x/unsubscribe/abc"));
        assert!(book.contains("https://x/unsubscribe/abc"), "{book}");
        let (_, asked) = letter(LetterReason::Sequel, Lang::Ru, None);
        assert!(asked.contains("Это письмо одно"), "{asked}");
        // Просил продолжение и вписан в книгу — дверь всё равно печатается.
        let (_, both) = letter(LetterReason::Sequel, Lang::En, Some("https://x/unsubscribe/abc"));
        assert!(both.contains("https://x/unsubscribe/abc"), "{both}");
    }

    #[test]
    fn a_letter_without_a_dek_has_no_empty_line_for_it() {
        let (_, body) = compose_letter(&TaleLetter {
            reason: LetterReason::New,
            lang: Lang::En,
            title: "A tale",
            dek: Some("   "),
            link: "https://x/tales/a",
            original: None,
            chosen_work: None,
            unsubscribe: None,
        });
        assert!(body.contains("A tale\n\nRead it: https://x/tales/a"), "{body}");
    }

    #[test]
    fn a_note_names_both_tales_and_escapes_them() {
        let (text, button) = compose_note(Lang::Ru, "Кот <и> пёс", Some("Начало & конец"));
        assert!(text.contains("«Начало &amp; конец»"), "{text}");
        assert!(text.contains("«Кот &lt;и&gt; пёс»"), "{text}");
        assert_eq!(button, "Читать");
        let (text, button) = compose_note(Lang::En, "The end", None);
        assert!(text.contains("“The end”"), "{text}");
        assert_eq!(button, "Read it");
    }

    #[test]
    fn a_ukrainian_letter_speaks_ukrainian() {
        let (subject, body) = letter(LetterReason::Sequel, Lang::Uk, None);
        assert!(subject.starts_with("Продовження: "), "{subject}");
        assert!(body.contains("«Лесник»"), "{body}");
        assert!(body.contains("Читати: "), "{body}");
        assert!(body.contains("Цей лист один"), "{body}");
        let (text, button) = compose_note(Lang::Uk, "Кіт", None);
        assert!(text.contains("Продовження записане"), "{text}");
        assert_eq!(button, "Читати");
        let (text, _) = compose_book_note(Lang::Uk, "Кіт", Some("Вступ"));
        assert!(text.contains("Нова небилиця"), "{text}");
        assert!(text.contains("Не писати мені: /stop"), "{text}");
    }

    #[test]
    fn only_a_named_channel_has_a_link() {
        assert_eq!(channel_link("@ritunia_tales").as_deref(), Some("https://t.me/ritunia_tales"));
        assert_eq!(channel_link(" @ritunia "), Some("https://t.me/ritunia".to_string()));
        assert_eq!(channel_link("-1001234567890"), None);
        assert_eq!(channel_link("@"), None);
        assert_eq!(channel_link("@bad/name"), None);
    }

    #[test]
    fn the_largest_rendition_is_the_preview_beside_it() {
        assert_eq!(
            largest_rendition("https://ritunia.com/static/images/thumb/witch-1.jpg"),
            "https://ritunia.com/static/images/preview/witch-1.jpg"
        );
        assert_eq!(
            largest_rendition("/static/images/medium/a.webp"),
            "/static/images/preview/a.webp"
        );
        assert_eq!(
            largest_rendition("/static/images/preview/a.jpg"),
            "/static/images/preview/a.jpg"
        );
        assert_eq!(largest_rendition("https://elsewhere.org/p.jpg"), "https://elsewhere.org/p.jpg");
    }

    #[test]
    fn a_pin_reads_as_prose_not_as_markup() {
        let body = "# Глава\n\nОн был **очень** стар.\n\n✦\n\nКонец.";
        let d = pin_description(Some("Эпиграф"), Some(body), 480);
        assert_eq!(d, "Эпиграф — Глава Он был очень стар. Конец.");
        assert!(pin_description(None, Some(&"слово ".repeat(200)), 60).chars().count() <= 60);
        assert_eq!(pin_description(Some("  "), None, 480), "");
    }

    #[test]
    fn an_address_is_usable_only_when_it_looks_like_one() {
        assert_eq!(usable_email(Some("  a@b.c ")).as_deref(), Some("a@b.c"));
        assert_eq!(usable_email(Some("no-at-sign")), None);
        assert_eq!(usable_email(Some("a b@c.d")), None);
        assert_eq!(usable_email(Some("")), None);
        assert_eq!(usable_email(None), None);
    }
}
