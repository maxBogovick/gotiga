//! «Спросить сайт»: каталог готовых ответов и выбор записи по вопросу.
//!
//! Чистые помощники без базы и без сети, как `tales.rs`: проверка записи
//! каталога, тексты по языкам и решение, что делать с вопросом (ответить,
//! предложить три записи или не предлагать ничего). Базу, модель эмбеддингов
//! и поиск зовёт `services/ask.rs`.
//!
//! Числа решения (`ANSWER_LEVEL` и остальные) подобраны на e5-small замером
//! `examples/ask_probe.rs`: пороги ищутся на одном наборе вопросов, а результат
//! считается на втором, которого подбор не видел (наборы — `tools/ask/`). Сменили
//! модель или каталог — перемерить, а не переносить: сходство разных моделей
//! лежит на разных шкалах.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::lang::{in_lang, Lang};

/// Ответ без вопросов: сходство вопроса с лучшей записью не ниже…
pub const ANSWER_LEVEL: f32 = 0.89;
/// …и вторая запись отстаёт не меньше, чем на это.
pub const ANSWER_GAP: f32 = 0.01;
/// Если с вектором согласен поиск по словам (BM25 и триграммы), порог ответа
/// ниже на столько. Замер дал 0: на свежих вопросах понижение порога добавляло
/// уверенных ошибок быстрее, чем верных ответов. Согласие поиска всё равно
/// снимает требование разрыва: лучшая запись выше `ANSWER_LEVEL` и поиск по
/// словам с ней согласен — этого достаточно.
pub const RESCUE: f32 = 0.0;
/// Ниже этого сходства лучшая запись не предлагается: вопрос не про каталог.
pub const CANDIDATE_FLOOR: f32 = 0.84;
/// Сколько записей предлагается кнопками, когда уверенности нет.
pub const CANDIDATES: usize = 3;

/// Что клиент собирает сам из данных сайта. Запись с таким ответом текста не
/// несёт: число работ, самое новое, что почитать и с чего начать меняются
/// каждый день и в каталоге устарели бы.
pub const HANDLERS: [&str; 4] = ["count", "new", "read", "start"];

/// Темы вопросов. Подписи на шести языках лежат в словаре страницы.
pub const TOPICS: [&str; 7] = ["works", "order", "rights", "site", "stories", "play", "studio"];
/// Страницы, на которых запись показывается первой в пустом поле.
pub const PAGES: [&str; 4] = ["work", "tale", "play", "gazette"];
/// Тема записи, у которой её не назвали: вопрос не должен пропасть из списка.
pub const TOPIC_DEFAULT: &str = "site";

pub const PHRASES_MAX: usize = 40;
pub const PHRASE_LEN: usize = 200;
pub const ANSWER_LEN: usize = 1500;
pub const LINKS_MAX: usize = 4;
const LABEL_LEN: usize = 60;

/// Один текст на шести языках. Обязательны только английский и русский, как
/// везде на сайте; пустой язык берётся по цепочке `Lang::fallbacks`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LangTexts {
    pub en: String,
    pub ru: String,
    pub uk: String,
    pub es: String,
    pub de: String,
    pub it: String,
}

impl LangTexts {
    /// Текст на языке читателя или, если он пуст, по цепочке откатов.
    pub fn in_lang(&self, lang: Lang) -> Option<String> {
        in_lang(
            lang,
            Some(&self.en),
            Some(&self.ru),
            Some(&self.uk),
            Some(&self.es),
            Some(&self.de),
            Some(&self.it),
        )
    }

    fn all(&self) -> [&String; 6] {
        [&self.en, &self.ru, &self.uk, &self.es, &self.de, &self.it]
    }

    fn trimmed(&self) -> LangTexts {
        LangTexts {
            en: self.en.trim().to_string(),
            ru: self.ru.trim().to_string(),
            uk: self.uk.trim().to_string(),
            es: self.es.trim().to_string(),
            de: self.de.trim().to_string(),
            it: self.it.trim().to_string(),
        }
    }
}

/// Ссылка под ответом: внутренний адрес сайта или https.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AskLinkDef {
    pub href: String,
    pub label: LangTexts,
}

/// Запись каталога. `handler = None` — готовый текст; иначе ответ собирает
/// клиент из данных сайта.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AskAnswer {
    pub id: Option<Uuid>,
    /// Имя записи: латиница, цифры и «_». Постоянное, по нему замер и правки.
    pub key: String,
    pub handler: Option<String>,
    /// Выключенная запись в подборе не участвует. Черновик без ответа так и лежит.
    pub enabled: bool,
    pub sort_order: i32,
    /// Тема из `TOPICS`: по ней вопрос лежит в списке «все вопросы».
    pub topic: String,
    /// Страницы из `PAGES`, на которых вопрос показывается первым.
    pub pages: Vec<String>,
    /// Как люди спрашивают: на любых языках вперемешку. Сравниваются по смыслу.
    pub phrases: Vec<String>,
    /// Вопрос, как он напечатан на кнопке-предложении.
    pub question: LangTexts,
    pub answer: LangTexts,
    pub links: Vec<AskLinkDef>,
}

fn valid_key(key: &str) -> bool {
    let n = key.chars().count();
    (2..=40).contains(&n)
        && key.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn valid_href(href: &str) -> bool {
    (href.starts_with('/') && !href.starts_with("//") && !href.contains(char::is_whitespace))
        || (href.starts_with("https://") && !href.contains(char::is_whitespace))
}

/// Привести запись в порядок перед записью в базу. Отказ — по-человечески,
/// одной строкой: её покажет стол.
pub fn normalize(mut a: AskAnswer) -> Result<AskAnswer, String> {
    a.key = a.key.trim().to_ascii_lowercase();
    if !valid_key(&a.key) {
        return Err("The key must be 2–40 characters: lowercase Latin letters, digits and “_”, starting with a letter.".into());
    }
    a.handler = a.handler.map(|h| h.trim().to_string()).filter(|h| !h.is_empty());
    if let Some(h) = &a.handler {
        if !HANDLERS.contains(&h.as_str()) {
            return Err(format!("Unknown data answer “{h}”: expected one of {}.", HANDLERS.join(", ")));
        }
    }
    a.sort_order = a.sort_order.clamp(0, 10_000);

    a.topic = a.topic.trim().to_ascii_lowercase();
    if a.topic.is_empty() {
        a.topic = TOPIC_DEFAULT.to_string();
    }
    if !TOPICS.contains(&a.topic.as_str()) {
        return Err(format!("Unknown topic “{}”: expected one of {}.", a.topic, TOPICS.join(", ")));
    }
    let mut pages: Vec<String> = Vec::new();
    for p in &a.pages {
        let p = p.trim().to_ascii_lowercase();
        if !PAGES.contains(&p.as_str()) {
            return Err(format!("Unknown page “{p}”: expected one of {}.", PAGES.join(", ")));
        }
        if !pages.contains(&p) {
            pages.push(p);
        }
    }
    a.pages = pages;

    let mut seen = std::collections::HashSet::new();
    let mut phrases = Vec::new();
    for p in &a.phrases {
        let p = p.split_whitespace().collect::<Vec<_>>().join(" ");
        if p.is_empty() {
            continue;
        }
        if p.chars().count() > PHRASE_LEN {
            return Err(format!("A phrase is longer than {PHRASE_LEN} characters."));
        }
        if seen.insert(p.to_lowercase()) {
            phrases.push(p);
        }
    }
    if phrases.len() > PHRASES_MAX {
        return Err(format!("At most {PHRASES_MAX} phrases per answer."));
    }
    a.phrases = phrases;

    a.question = a.question.trimmed();
    a.answer = a.answer.trimmed();
    if a.question.all().iter().any(|t| t.chars().count() > PHRASE_LEN) {
        return Err(format!("A question is longer than {PHRASE_LEN} characters."));
    }
    if a.answer.all().iter().any(|t| t.chars().count() > ANSWER_LEN) {
        return Err(format!("An answer is longer than {ANSWER_LEN} characters."));
    }

    if a.links.len() > LINKS_MAX {
        return Err(format!("At most {LINKS_MAX} links under an answer."));
    }
    for l in &mut a.links {
        l.href = l.href.trim().to_string();
        l.label = l.label.trimmed();
        if !valid_href(&l.href) {
            return Err("A link must be an address on the site (starting with “/”) or start with https://.".into());
        }
        if l.label.all().iter().any(|t| t.chars().count() > LABEL_LEN) {
            return Err(format!("A link label is longer than {LABEL_LEN} characters."));
        }
        if l.label.en.is_empty() || l.label.ru.is_empty() {
            return Err("Every link needs a label in English and Russian.".into());
        }
    }

    if a.enabled {
        if a.phrases.len() < 3 {
            return Err("An answer that is switched on needs at least 3 phrases: with fewer the site recognises it by chance.".into());
        }
        if a.question.en.is_empty() || a.question.ru.is_empty() {
            return Err("An answer that is switched on needs the question in English and Russian: it is printed on the suggestion button.".into());
        }
        if a.handler.is_none() && (a.answer.en.is_empty() || a.answer.ru.is_empty()) {
            return Err("An answer that is switched on needs its text in English and Russian.".into());
        }
    }
    Ok(a)
}

/// Всё, что сравнивается с вопросом: фразы и напечатанные вопросы. Повторы
/// убраны, регистр не различается.
pub fn index_phrases(a: &AskAnswer) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in a.phrases.iter().chain(a.question.all()) {
        let p = p.trim();
        if !p.is_empty() && seen.insert(p.to_lowercase()) {
            out.push(p.to_string());
        }
    }
    out
}

/// Служебные слова шести языков. Поиск по словам без них ищет по содержанию
/// вопроса: «how do I unsubscribe» находит «unsubscribe», а не любую запись, где
/// есть «how do I».
const STOP_WORDS: &[&str] = &[
    // en
    "how", "do", "does", "did", "i", "you", "your", "we", "my", "me", "a", "an", "the", "is", "are", "was", "be", "to", "of", "in", "on", "for", "from",
    "with", "can", "could", "would", "should", "it", "this", "that", "what", "when", "where", "which", "who", "and", "or", "if", "there", "about", "as", "at", "by",
    // ru
    "как", "что", "где", "когда", "какой", "какая", "какие", "какое", "кто", "ли", "можно", "нужно", "и", "в", "на", "с", "к", "по", "за", "о", "об", "для", "от",
    "из", "у", "я", "мы", "вы", "мне", "вам", "ты", "это", "эта", "этот", "эти", "есть", "бы", "же", "или", "не", "то", "там", "тут",
    // uk
    "як", "що", "де", "коли", "який", "яка", "які", "хто", "чи", "можна", "потрібно", "із", "до", "про", "від", "ми", "ви", "мені", "вам", "ти", "це", "ця", "цей",
    "ці", "є", "б", "або",
    // es
    "como", "cómo", "que", "qué", "donde", "dónde", "cuando", "cuándo", "cual", "cuál", "quien", "quién", "se", "puede", "puedo", "el", "la", "los", "las", "un",
    "una", "de", "del", "con", "por", "para", "es", "son", "y", "o", "si", "hay", "mi", "te", "tu",
    // de
    "wie", "was", "wo", "wann", "welche", "welcher", "wer", "kann", "ich", "du", "sie", "wir", "mein", "der", "die", "das", "den", "dem", "ein", "eine", "einen",
    "und", "oder", "ist", "sind", "zu", "von", "mit", "für", "auf", "bei", "gibt",
    // it
    "come", "che", "cosa", "dove", "quando", "quale", "chi", "posso", "può", "si", "il", "lo", "gli", "le", "di", "in", "per", "è", "sono", "e", "se", "ci",
];

/// Вопрос без служебных слов и однобуквенных обрывков — для поиска по словам.
/// Если ничего не осталось, возвращается сам вопрос.
pub fn content_words(question: &str) -> String {
    let kept: Vec<String> = question
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.chars().count() >= 2 && !STOP_WORDS.contains(&w.as_str()))
        .collect();
    if kept.is_empty() {
        question.trim().to_string()
    } else {
        kept.join(" ")
    }
}

/// Вопрос без регистра и знаков препинания: «Как выиграть бой?» и «как выиграть
/// бой» — один и тот же вопрос.
fn folded(text: &str) -> String {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Запись, одна из формулировок которой совпала с вопросом слово в слово.
/// Кнопки-вопросы берутся из формулировок каталога, поэтому нажатие на кнопку
/// обязано давать ответ этой записи, а не зависеть от модели: без неё подбор по
/// словам умеет только предлагать, и кнопка вела бы к тем же кнопкам.
pub fn exact_match(question: &str, entries: &[AskAnswer]) -> Option<usize> {
    let asked = folded(question);
    if asked.is_empty() {
        return None;
    }
    entries.iter().position(|e| index_phrases(e).iter().any(|p| folded(p) == asked))
}

/// Вопрос для списка в окне: напечатанный на языке читателя, тема, страницы и
/// формулировки, по которым окно подсказывает вопрос, пока человек печатает.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskQuestion {
    pub key: String,
    pub question: String,
    pub topic: String,
    pub pages: Vec<String>,
    pub phrases: Vec<String>,
}

/// Включённые записи, у которых есть вопрос на этом языке (или по цепочке откатов),
/// в порядке каталога. Черновики в список не попадают: на кнопку нужен ответ.
pub fn public_questions(entries: &[AskAnswer], lang: Lang) -> Vec<AskQuestion> {
    entries
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|e| {
            let question = e.question.in_lang(lang).filter(|q| !q.is_empty())?;
            Some(AskQuestion {
                key: e.key.clone(),
                question,
                topic: e.topic.clone(),
                pages: e.pages.clone(),
                phrases: index_phrases(e),
            })
        })
        .collect()
}

/// Что сделать с вопросом.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// Ответить этой записью (индекс в списке записей).
    Answer(usize),
    /// Уверенности нет: предложить эти записи кнопками, лучшая первой.
    Suggest(Vec<usize>),
    /// Ничего подходящего: вопрос не про каталог.
    Nothing,
}

/// Пороги решения. Вынесены в структуру, чтобы замер перебирал их тем же кодом,
/// который работает на сайте, а не копией.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub answer_level: f32,
    pub answer_gap: f32,
    pub rescue: f32,
    pub candidate_floor: f32,
}

impl Thresholds {
    pub const DEFAULT: Thresholds = Thresholds {
        answer_level: ANSWER_LEVEL,
        answer_gap: ANSWER_GAP,
        rescue: RESCUE,
        candidate_floor: CANDIDATE_FLOOR,
    };
}

/// Решение по сходству вопроса с каждой записью (`levels[i]` — лучшая из её
/// формулировок). `lexical_top` — запись, которую первой поставил поиск по
/// словам; если она совпала с лучшей по смыслу, пограничный случай отвечается.
pub fn decide(levels: &[f32], lexical_top: Option<usize>) -> Decision {
    decide_with(levels, lexical_top, &Thresholds::DEFAULT)
}

pub fn decide_with(levels: &[f32], lexical_top: Option<usize>, th: &Thresholds) -> Decision {
    let mut order: Vec<usize> = (0..levels.len()).filter(|&i| levels[i].is_finite()).collect();
    order.sort_by(|&a, &b| levels[b].partial_cmp(&levels[a]).unwrap_or(std::cmp::Ordering::Equal));
    let Some(&top) = order.first() else {
        return Decision::Nothing;
    };
    let level = levels[top];
    let gap = order.get(1).map(|&s| level - levels[s]).unwrap_or(level);
    let strong = level >= th.answer_level && gap >= th.answer_gap;
    let rescued = lexical_top == Some(top) && level >= th.answer_level - th.rescue;
    if strong || rescued {
        return Decision::Answer(top);
    }
    if level >= th.candidate_floor {
        return Decision::Suggest(order.into_iter().take(CANDIDATES).collect());
    }
    Decision::Nothing
}

/// Решение без модели: поиск по словам не знает «насколько уверен», поэтому
/// только предлагает.
pub fn decide_lexical(ranked: &[usize]) -> Decision {
    if ranked.is_empty() {
        Decision::Nothing
    } else {
        Decision::Suggest(ranked.iter().copied().take(CANDIDATES).collect())
    }
}

// ── Ответ ручки ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedLink {
    pub href: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedAnswer {
    pub key: String,
    /// Если задан, клиент собирает ответ сам из данных сайта.
    pub handler: Option<String>,
    pub text: String,
    pub links: Vec<RoutedLink>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedCandidate {
    pub key: String,
    pub question: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskRoute {
    /// `answer`, `suggest`, `unanswered` (вопрос известен, ответа ещё нет) или `none`.
    pub decision: &'static str,
    pub answer: Option<RoutedAnswer>,
    pub candidates: Vec<RoutedCandidate>,
}

impl AskRoute {
    pub fn none() -> Self {
        AskRoute { decision: "none", answer: None, candidates: Vec::new() }
    }

    /// Вопрос узнан, но запись выключена: ответа на сайте ещё нет. Лучше сказать
    /// это, чем отдать вопрос соседней записи («сколько делается работа» →
    /// «цен на сайте нет»).
    pub fn unanswered(key: &str) -> Self {
        AskRoute {
            decision: "unanswered",
            answer: Some(RoutedAnswer { key: key.to_string(), handler: None, text: String::new(), links: Vec::new() }),
            candidates: Vec::new(),
        }
    }

    pub fn answer(a: &AskAnswer, lang: Lang) -> Self {
        let links = a
            .links
            .iter()
            .filter_map(|l| l.label.in_lang(lang).map(|label| RoutedLink { href: l.href.clone(), label }))
            .collect();
        AskRoute {
            decision: "answer",
            answer: Some(RoutedAnswer {
                key: a.key.clone(),
                handler: a.handler.clone(),
                text: a.answer.in_lang(lang).unwrap_or_default(),
                links,
            }),
            candidates: Vec::new(),
        }
    }

    pub fn suggest(entries: &[&AskAnswer], lang: Lang) -> Self {
        let candidates: Vec<RoutedCandidate> = entries
            .iter()
            .filter_map(|a| a.question.in_lang(lang).map(|question| RoutedCandidate { key: a.key.clone(), question }))
            .collect();
        if candidates.is_empty() {
            return AskRoute::none();
        }
        AskRoute { decision: "suggest", answer: None, candidates }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(en: &str, ru: &str) -> LangTexts {
        LangTexts { en: en.into(), ru: ru.into(), ..Default::default() }
    }

    #[test]
    fn topic_and_pages_are_checked_and_tidied() {
        let mut a = ready();
        a.topic = "  Order ".into();
        a.pages = vec!["Work".into(), "work".into(), "play".into()];
        let a = normalize(a).unwrap();
        assert_eq!(a.topic, "order");
        assert_eq!(a.pages, vec!["work", "play"]);

        let mut none = ready();
        none.topic = "".into();
        assert_eq!(normalize(none).unwrap().topic, TOPIC_DEFAULT);

        let mut bad = ready();
        bad.topic = "cooking".into();
        assert!(normalize(bad).is_err());
        let mut bad = ready();
        bad.pages = vec!["kitchen".into()];
        assert!(normalize(bad).is_err());
    }

    #[test]
    fn the_public_list_has_only_switched_on_entries_with_a_question() {
        let on = ready();
        let mut off = ready();
        off.key = "draft".into();
        off.enabled = false;
        let list = public_questions(&[on, off], Lang::Ru);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].key, "lead_time");
        assert!(list[0].phrases.len() >= 3);
    }

    #[test]
    fn exact_phrase_matches_without_case_and_punctuation() {
        let entries = vec![ready()];
        assert_eq!(exact_match("как быстро?", &entries), Some(0));
        assert_eq!(exact_match("  How long does it TAKE ", &entries), Some(0));
        assert_eq!(exact_match("как быстро делается", &entries), None);
        assert_eq!(exact_match("?!", &entries), None);
    }

    fn ready() -> AskAnswer {
        AskAnswer {
            key: "lead_time".into(),
            enabled: true,
            phrases: vec!["How long does it take?".into(), "Сколько делается работа".into(), "Как быстро".into()],
            question: texts("How long does it take?", "Сколько делается работа?"),
            answer: texts("Weeks.", "Недели."),
            ..Default::default()
        }
    }

    #[test]
    fn a_complete_entry_passes_and_is_tidied() {
        let mut a = ready();
        a.key = "  Lead_Time ".into();
        a.phrases.push("how  long   does it take?".into());
        let a = normalize(a).unwrap();
        assert_eq!(a.key, "lead_time");
        assert_eq!(a.phrases.len(), 3, "the repeat, differing only in spaces and case, is gone");
    }

    #[test]
    fn a_bad_key_is_refused() {
        for key in ["", "A", "1abc", "has space", "дом"] {
            let mut a = ready();
            a.key = key.into();
            assert!(normalize(a).is_err(), "{key:?}");
        }
    }

    #[test]
    fn a_switched_on_entry_needs_its_words() {
        let mut few = ready();
        few.phrases.truncate(2);
        assert!(normalize(few).is_err());

        let mut no_ru = ready();
        no_ru.answer.ru.clear();
        assert!(normalize(no_ru).is_err());

        let mut no_question = ready();
        no_question.question.en.clear();
        assert!(normalize(no_question).is_err());

        // Черновик может лежать пустым.
        let mut draft = ready();
        draft.enabled = false;
        draft.answer = LangTexts::default();
        assert!(normalize(draft).is_ok());
    }

    #[test]
    fn a_data_answer_carries_no_text_but_a_known_handler() {
        let mut a = ready();
        a.answer = LangTexts::default();
        a.handler = Some("count".into());
        assert!(normalize(a.clone()).is_ok());
        a.handler = Some("weather".into());
        assert!(normalize(a).is_err());
    }

    #[test]
    fn links_must_stay_on_the_site_or_use_https() {
        let link = |href: &str| AskLinkDef { href: href.into(), label: texts("Page", "Страница") };
        for ok in ["/acquire", "https://example.com/a"] {
            let mut a = ready();
            a.links = vec![link(ok)];
            assert!(normalize(a).is_ok(), "{ok}");
        }
        for bad in ["//evil.example", "http://example.com", "javascript:alert(1)", "/a b", ""] {
            let mut a = ready();
            a.links = vec![link(bad)];
            assert!(normalize(a).is_err(), "{bad:?}");
        }
        let mut no_ru = ready();
        no_ru.links = vec![AskLinkDef { href: "/a".into(), label: texts("Page", "") }];
        assert!(normalize(no_ru).is_err());
    }

    #[test]
    fn index_phrases_add_the_printed_questions_without_repeats() {
        let p = index_phrases(&ready());
        assert_eq!(p.len(), 4, "{p:?}");
        assert!(p.iter().any(|x| x == "Сколько делается работа?"));
    }

    #[test]
    fn the_word_search_gets_the_content_of_the_question_only() {
        assert_eq!(content_words("How do I unsubscribe from the newsletter?"), "unsubscribe newsletter");
        assert_eq!(content_words("Как отписаться от рассылки?"), "отписаться рассылки");
        assert_eq!(content_words("Wie melde ich mich vom Newsletter ab?"), "melde mich vom newsletter ab");
        // Одни служебные слова: ищем по самому вопросу, чем вовсе без поиска.
        assert_eq!(content_words("how do I"), "how do I");
        assert_eq!(content_words("  ?  "), "?");
    }

    #[test]
    fn a_clear_winner_is_answered() {
        assert_eq!(decide(&[0.93, 0.85, 0.80], None), Decision::Answer(0));
    }

    #[test]
    fn a_close_second_is_not_answered_but_offered() {
        // Выше порога, но вторая запись отстаёт всего на 0,005.
        match decide(&[0.90, 0.895, 0.85], None) {
            Decision::Suggest(v) => assert_eq!(v, vec![0, 1, 2]),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_word_search_waives_the_gap_only_when_it_agrees() {
        // Две близкие записи выше порога: без согласия поиска — кнопки, с согласием — ответ.
        let levels = [0.90, 0.895, 0.80];
        assert!(matches!(decide(&levels, None), Decision::Suggest(_)));
        assert!(matches!(decide(&levels, Some(1)), Decision::Suggest(_)));
        assert_eq!(decide(&levels, Some(0)), Decision::Answer(0));
        // Ниже порога согласие ничего не спасает: понижение порога замером не подтвердилось.
        assert!(matches!(decide(&[0.87, 0.70], Some(0)), Decision::Suggest(_)));
    }

    #[test]
    fn a_lowered_threshold_rescues_a_borderline_match() {
        let th = Thresholds { rescue: 0.03, ..Thresholds::DEFAULT };
        let levels = [0.87, 0.80, 0.70];
        assert_eq!(decide_with(&levels, Some(0), &th), Decision::Answer(0));
        assert!(matches!(decide_with(&levels, Some(1), &th), Decision::Suggest(_)));
        assert!(matches!(decide_with(&levels, None, &th), Decision::Suggest(_)));
    }

    #[test]
    fn a_question_about_nothing_in_the_catalog_gets_nothing() {
        assert_eq!(decide(&[0.80, 0.79], Some(0)), Decision::Nothing);
        assert_eq!(decide(&[0.835, 0.70], None), Decision::Nothing);
        assert_eq!(decide(&[], None), Decision::Nothing);
        assert_eq!(decide(&[f32::NAN], None), Decision::Nothing);
    }

    #[test]
    fn a_catalog_of_one_still_decides() {
        assert_eq!(decide(&[0.95], None), Decision::Answer(0));
        assert_eq!(decide(&[0.86], None), Decision::Suggest(vec![0]));
    }

    #[test]
    fn without_the_model_only_suggestions_are_made() {
        assert_eq!(decide_lexical(&[]), Decision::Nothing);
        assert_eq!(decide_lexical(&[4, 2, 7, 1]), Decision::Suggest(vec![4, 2, 7]));
    }

    #[test]
    fn the_reader_gets_their_language_with_a_fallback() {
        let mut a = ready();
        a.answer.uk = "Тижні.".into();
        a.links = vec![AskLinkDef { href: "/acquire".into(), label: texts("How to acquire", "Как приобрести") }];
        let uk = AskRoute::answer(&a, Lang::Uk);
        assert_eq!(uk.answer.as_ref().unwrap().text, "Тижні.");
        // У украинца ссылка без украинской подписи берёт английскую, не русскую.
        assert_eq!(uk.answer.as_ref().unwrap().links[0].label, "How to acquire");
        let de = AskRoute::answer(&a, Lang::De);
        assert_eq!(de.answer.as_ref().unwrap().text, "Weeks.");
    }

    #[test]
    fn an_unanswered_question_is_named_but_carries_no_text() {
        let r = AskRoute::unanswered("lead_time");
        assert_eq!(r.decision, "unanswered");
        assert_eq!(r.answer.as_ref().unwrap().key, "lead_time");
        assert!(r.answer.as_ref().unwrap().text.is_empty());
    }

    #[test]
    fn suggestions_skip_entries_without_a_question_in_any_language() {
        let mut blank = ready();
        blank.question = LangTexts::default();
        let route = AskRoute::suggest(&[&blank], Lang::En);
        assert_eq!(route.decision, "none");
        let route = AskRoute::suggest(&[&ready(), &blank], Lang::Ru);
        assert_eq!(route.decision, "suggest");
        assert_eq!(route.candidates.len(), 1);
    }
}
