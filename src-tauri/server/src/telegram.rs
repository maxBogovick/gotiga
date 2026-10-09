//! Разговор с ботом входа.
//!
//! Здесь только две вещи: что Telegram прислал и что бот отвечает. Ни базы, ни
//! правил входа — правила лежат в `services`, и эта граница та же, что у
//! `sheet.rs`: разбор листа не знает про склад, а бот не знает про аккаунты.
//!
//! Подписи проверять не нужно вовсе: `telegram_id` приходит не от браузера, а
//! от самого Telegram на наш webhook, и вся защита webhook — секрет в адресе и
//! в заголовке (`config::validate` не даёт включить вход без него).

use serde::Deserialize;

// ============================================================
// ЧТО ПРИСЛАЛИ
// ============================================================

/// Обновление от Telegram. Берём ровно два рода: сообщение (`/start`) и
/// нажатие кнопки под запиской. Остальное — не наше дело и не разбирается.
#[derive(Debug, Deserialize)]
pub struct Update {
    pub message: Option<Message>,
    pub callback_query: Option<CallbackQuery>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub message_id: i64,
    pub chat: Chat,
    pub from: Option<TgUser>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Chat {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct TgUser {
    /// То самое число, по которому узнают вернувшегося.
    pub id: i64,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// `ru`, `en`, `ru-RU`… Язык записки берётся отсюда: спрашивать человека,
    /// на каком языке он хочет прочитать одну строку, — это лишний вопрос.
    pub language_code: Option<String>,
}

impl TgUser {
    /// Имя для профиля при первом приходе. Дальше оно не трогается никогда.
    pub fn display_name(&self) -> String {
        let mut name = self.first_name.clone().unwrap_or_default();
        if let Some(last) = self.last_name.as_deref().filter(|s| !s.trim().is_empty()) {
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(last);
        }
        let name = name.trim().to_string();
        if !name.is_empty() {
            return name;
        }
        // Ни имени, ни фамилии — бывает у скрытых профилей. `@имя` годится,
        // а если нет и его, человек назовётся сам в профиле.
        match self.username.as_deref() {
            Some(u) if !u.trim().is_empty() => u.trim().to_string(),
            _ => "Telegram".to_string(),
        }
    }

    pub fn speaks_russian(&self) -> bool {
        self.language_code
            .as_deref()
            .is_some_and(|l| l.to_ascii_lowercase().starts_with("ru"))
    }
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub id: String,
    pub from: TgUser,
    pub message: Option<Message>,
    pub data: Option<String>,
}

/// Что человек нажал под запиской. В `data` кладётся `ok:<code>` / `no:<code>`
/// — код нужен прямо здесь, потому что записок в переписке может лежать
/// несколько, и нажать можно на любую.
pub enum Answer {
    Yes(String),
    No(String),
}

impl Answer {
    pub fn parse(data: &str) -> Option<Answer> {
        match data.split_once(':') {
            Some(("ok", code)) if !code.is_empty() => Some(Answer::Yes(code.to_string())),
            Some(("no", code)) if !code.is_empty() => Some(Answer::No(code.to_string())),
            _ => None,
        }
    }
}

/// Код из `/start <code>`. Голый `/start` — это просто «открыл бота»,
/// и кода в нём нет.
pub fn start_code(text: &str) -> Option<String> {
    let rest = text.strip_prefix("/start")?.trim();
    // `/start@имя_бота` — так Telegram присылает команду в группах.
    let rest = rest.strip_prefix('@').map_or(rest, |r| {
        r.split_once(' ').map(|(_, tail)| tail).unwrap_or("")
    });
    let code = rest.trim();
    if code.is_empty() || code.len() > 128 {
        return None;
    }
    Some(code.to_string())
}

// ============================================================
// ЧТО ОТВЕЧАЕТ БОТ
// ============================================================

/// Адрес, по которому этот дом принимает обновления Telegram.
///
/// Чистая функция, потому что один и тот же адрес называют саморегистрация при
/// запуске и подпись в панели выкладки: два способа собрать его однажды
/// разошлись бы на косой черте.
pub fn webhook_url(public_url: &str, secret: &str) -> String {
    format!(
        "{}/api/v1/telegram/webhook/{}",
        public_url.trim_end_matches('/'),
        secret
    )
}

/// Тот же адрес, но без секрета — для журнала.
///
/// Секрет стоит в самом пути, и журнал запуска печатал его целиком: в терминал,
/// в файл и в админский просмотрщик (`logs.rs`), то есть туда, где его читают
/// походя. А это единственное место во всём замысле, где чужой запрос напрямую
/// превращается в сессию (§ 16.2).
pub fn webhook_url_for_the_journal(public_url: &str) -> String {
    format!("{}/api/v1/telegram/webhook/…", public_url.trim_end_matches('/'))
}

/// Экранирование для `parse_mode: HTML`. Слово придумываем мы, а вот браузер и
/// город приходят из заголовков запроса — то есть снаружи.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// ============================================================
// КАНАЛ
// ============================================================

/// Сколько знаков Telegram принимает в подписи к фотографии. Считаются
/// видимые знаки, без разметки.
pub const CAPTION_MAX: usize = 1024;

/// Подпись объявления — четыре строки, как страница газеты в миниатюре:
///
/// ```text
/// ✦ Новая байка            ← род: что это, не дожидаясь заглавия
///
/// Заглавие                 ← жирным
///
/// │ вводка                 ← цитатой: отличается от заглавия без картинок
///
/// #байка                   ← метка: по ней в канале ищут «все байки»
/// ```
///
/// Ни восклицаний, ни «успейте», ни цены: дом не магазин (CLAUDE.md § 1), и
/// канал говорит тем же голосом, что и сайт.
///
/// Урезается вводка, а не заглавие, и урезается по слову: подпись, оборванная
/// посреди слова, выглядит как ошибка, а не как «читать дальше». Лимит
/// Telegram считает видимые знаки без разметки, поэтому всё, что печатается
/// кроме вводки, вычитается из места под неё.
pub fn announcement_caption(label: &str, tag: &str, title: &str, lead: Option<&str>) -> String {
    let title: String = title.trim().chars().take(CAPTION_MAX / 4).collect();
    let head = format!("✦ {label}");
    let foot = format!("#{tag}");
    let lead = lead.map(str::trim).filter(|l| !l.is_empty() && *l != title);

    let mut out = format!("{}\n\n<b>{}</b>", esc(&head), esc(&title));
    if let Some(lead) = lead {
        // Три разделителя по две строки перевода — тоже знаки.
        let used = head.chars().count() + title.chars().count() + foot.chars().count() + 6;
        // Заглавие урезано до четверти лимита, поэтому под вводку остаётся
        // не меньше половины подписи.
        let room = CAPTION_MAX - used;
        out.push_str(&format!("\n\n<blockquote>{}</blockquote>", esc(&clip_words(lead, room))));
    }
    out.push_str(&format!("\n\n{}", esc(&foot)));
    out
}

fn clip_words(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    let cut = match cut.rfind(char::is_whitespace) {
        Some(i) if i > cut.len() / 2 => &cut[..i],
        _ => cut.as_str(),
    };
    format!("{}…", cut.trim_end_matches(|c: char| c.is_whitespace() || c == ',' || c == '.'))
}

pub struct Bot {
    token: String,
    client: reqwest::Client,
}

impl Bot {
    pub fn new(token: String, client: reqwest::Client) -> Self {
        Self { token, client }
    }

    fn url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    /// Один разговор с Telegram — и одно место, где слышно, что он не удался.
    ///
    /// Исходящие вызовы были помечены `let _ =` и молчали: Telegram недоступен
    /// или токен отозван — человек видит, что бот не ответил, а в журнале
    /// пусто. Неудача по-прежнему ничего не роняет (вход доходит до конца и
    /// без записки), но теперь она **видна**: `warn` попадает в админский
    /// просмотрщик журналов вместе со всем остальным.
    async fn call(&self, method: &str, body: serde_json::Value) -> Option<serde_json::Value> {
        match self.call_why(method, body).await {
            Ok(value) => Some(value),
            Err(why) => {
                tracing::warn!("Telegram {method}: {why}");
                None
            }
        }
    }

    /// То же, но причина отказа достаётся вызывающему: журнал канала хранит
    /// её в строке объявления, чтобы было видно, почему пост не вышел.
    async fn call_why(&self, method: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let res = self
            .client
            .post(self.url(method))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("failed: {e}"))?;
        let value: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("gave an unreadable answer: {e}"))?;
        if value["ok"].as_bool() != Some(true) {
            // Причину называет сам Telegram: «chat not found», «bot was blocked
            // by the user», «Unauthorized». Угадывать её по коду нечем.
            let why = value["description"].as_str().unwrap_or("no reason given");
            return Err(format!("refused: {why}"));
        }
        Ok(value)
    }

    /// Записка с двумя кнопками. Возвращает её `message_id`, чтобы после
    /// ответа переписать её же, а не сыпать в переписку новые.
    pub async fn ask(&self, chat_id: i64, text: &str, yes: &str, no: &str, code: &str) -> Option<i64> {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": yes, "callback_data": format!("ok:{code}") },
                    { "text": no,  "callback_data": format!("no:{code}") },
                ]]
            }
        });
        self.call("sendMessage", body).await?["result"]["message_id"].as_i64()
    }

    /// Простая строка без кнопок.
    pub async fn say(&self, chat_id: i64, text: &str) {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.call("sendMessage", body).await;
    }

    /// Переписать записку на месте: кнопки снимаются вместе с ней, и нажать
    /// «это я» второй раз становится не на что.
    pub async fn settle(&self, chat_id: i64, message_id: i64, text: &str) {
        let body = serde_json::json!({
            "chat_id": chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.call("editMessageText", body).await;
    }

    /// Сказать Telegram, куда присылать обновления.
    ///
    /// Ручка идемпотентна: тот же адрес с тем же секретом ничего не меняет, а
    /// изменившийся — переписывает. Поэтому вызывается при каждом запуске и
    /// чинит сама себя: сменили домен, перевыпустили секрет, подняли сервер на
    /// новом месте — регистрация догоняет без единой команды руками.
    ///
    /// `allowed_updates` назван явно: бот читает только `/start` и нажатия
    /// кнопок, и просить у Telegram остальное значит получать то, что некому
    /// разбирать.
    ///
    /// В отличие от прочих вызовов, ответ нужен **вызывающему**: запуск
    /// печатает одну строку о том, удалась ли регистрация, и причину в ней
    /// называет сам Telegram.
    pub async fn announce(&self, url: &str, secret: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "url": url,
            "secret_token": secret,
            "allowed_updates": ["message", "callback_query"],
        });
        let res = self
            .client
            .post(self.url("setWebhook"))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let value: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        if value["ok"].as_bool() == Some(true) {
            return Ok(());
        }
        Err(value["description"]
            .as_str()
            .unwrap_or("Telegram refused the webhook")
            .to_string())
    }

    /// Объявление в канале с фотографией. Ссылка на сайт — кнопкой под постом:
    /// в подписи она заняла бы строку, а кнопка переживает пересылку.
    /// Возвращает `message_id` поста.
    pub async fn post_photo(
        &self,
        chat: &str,
        photo: &str,
        caption: &str,
        button: &str,
        url: &str,
    ) -> Result<i64, String> {
        let body = serde_json::json!({
            "chat_id": chat,
            "photo": photo,
            "caption": caption,
            "parse_mode": "HTML",
            "reply_markup": { "inline_keyboard": [[ { "text": button, "url": url } ]] },
        });
        let value = self.call_why("sendPhoto", body).await?;
        value["result"]["message_id"]
            .as_i64()
            .ok_or_else(|| "answer without message_id".to_string())
    }

    /// Объявление без фотографии — или когда Telegram фотографию не взял.
    /// Превью ссылки Telegram строит сам по странице.
    pub async fn post_text(&self, chat: &str, text: &str, button: &str, url: &str) -> Result<i64, String> {
        let body = serde_json::json!({
            "chat_id": chat,
            "text": text,
            "parse_mode": "HTML",
            "link_preview_options": { "url": url, "prefer_large_media": true },
            "reply_markup": { "inline_keyboard": [[ { "text": button, "url": url } ]] },
        });
        let value = self.call_why("sendMessage", body).await?;
        value["result"]["message_id"]
            .as_i64()
            .ok_or_else(|| "answer without message_id".to_string())
    }

    /// Обязательный ответ на нажатие — без него в Telegram кнопка «думает»
    /// до таймаута.
    pub async fn ack(&self, callback_id: &str, text: &str) {
        let body = serde_json::json!({
            "callback_query_id": callback_id,
            "text": text,
        });
        self.call("answerCallbackQuery", body).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_carries_its_code_and_a_bare_start_does_not() {
        assert_eq!(start_code("/start k7f3x9"), Some("k7f3x9".to_string()));
        assert_eq!(start_code("/start"), None);
        assert_eq!(start_code("/start   "), None);
        assert_eq!(start_code("привет"), None);
    }

    #[test]
    fn a_button_says_which_word_it_answers_for() {
        assert!(matches!(Answer::parse("ok:abc"), Some(Answer::Yes(c)) if c == "abc"));
        assert!(matches!(Answer::parse("no:abc"), Some(Answer::No(c)) if c == "abc"));
        assert!(Answer::parse("ok:").is_none());
        assert!(Answer::parse("whatever").is_none());
    }

    #[test]
    fn a_name_is_taken_once_and_never_comes_out_empty() {
        let full = TgUser {
            id: 1,
            username: Some("masha".into()),
            first_name: Some("Маша".into()),
            last_name: Some("Иванова".into()),
            language_code: Some("ru-RU".into()),
        };
        assert_eq!(full.display_name(), "Маша Иванова");
        assert!(full.speaks_russian());

        let hidden = TgUser {
            id: 2,
            username: Some("ghost".into()),
            first_name: None,
            last_name: None,
            language_code: Some("en".into()),
        };
        assert_eq!(hidden.display_name(), "ghost");
        assert!(!hidden.speaks_russian());

        let bare = TgUser {
            id: 3,
            username: None,
            first_name: None,
            last_name: None,
            language_code: None,
        };
        assert_eq!(bare.display_name(), "Telegram");
        assert!(!bare.speaks_russian());
    }

    #[test]
    fn the_webhook_address_is_the_same_however_the_domain_is_written() {
        let plain = webhook_url("https://ritunia.com", "s3cret");
        assert_eq!(plain, "https://ritunia.com/api/v1/telegram/webhook/s3cret");
        // Косая черта в конце PUBLIC_URL — обычное дело, и вторая подряд дала
        // бы адрес, по которому Telegram получал бы 404 молча.
        assert_eq!(webhook_url("https://ritunia.com/", "s3cret"), plain);
    }

    /// Видимые знаки подписи: без разметки, как их считает Telegram.
    fn visible(caption: &str) -> String {
        ["<b>", "</b>", "<blockquote>", "</blockquote>"]
            .iter()
            .fold(caption.to_string(), |s, tag| s.replace(tag, ""))
    }

    #[test]
    fn a_caption_is_a_label_a_title_a_quoted_lead_and_a_tag() {
        assert_eq!(
            announcement_caption("Новая работа", "работа", "Страж <северной> двери", Some("Бронза & воск")),
            "✦ Новая работа\n\n<b>Страж &lt;северной&gt; двери</b>\n\n\
             <blockquote>Бронза &amp; воск</blockquote>\n\n#работа"
        );
        // Вводки нет или она повторяет заглавие — цитаты нет вовсе, а не пустая.
        let bare = "✦ Новая байка\n\n<b>Ворон</b>\n\n#байка";
        assert_eq!(announcement_caption("Новая байка", "байка", "Ворон", None), bare);
        assert_eq!(announcement_caption("Новая байка", "байка", "Ворон", Some("  ")), bare);
        assert_eq!(announcement_caption("Новая байка", "байка", "Ворон", Some("Ворон")), bare);
    }

    #[test]
    fn a_long_lead_is_cut_by_the_word_and_fits_telegram() {
        let lead = "слово ".repeat(400);
        let caption = announcement_caption("Новая байка", "байка", "Байка", Some(&lead));
        assert!(visible(&caption).chars().count() <= CAPTION_MAX);
        assert!(caption.contains("слово…</blockquote>"));
        // Метка стоит в конце даже тогда, когда вводка заняла всё свободное место.
        assert!(caption.ends_with("#байка"));
    }

    #[test]
    fn a_huge_title_is_capped_and_the_caption_still_fits() {
        let title = "а".repeat(CAPTION_MAX * 2);
        let lead = "слово ".repeat(400);
        let caption = announcement_caption("Новая работа", "работа", &title, Some(&lead));
        assert!(visible(&caption).chars().count() <= CAPTION_MAX);
        assert!(caption.contains("<blockquote>") && caption.ends_with("#работа"));
    }

    #[test]
    fn what_comes_from_outside_cannot_carry_tags_into_the_note() {
        assert_eq!(esc("Chrome <b>x</b> & co"), "Chrome &lt;b&gt;x&lt;/b&gt; &amp; co");
    }
}
