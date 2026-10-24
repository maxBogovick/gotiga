//! Telegram-каналы по языкам: слова постов и сборка поста о релизе.
//!
//! Без базы и без сети, как `telegram.rs`: здесь только то, что можно
//! проверить тестом. Какие каналы заведены и что в них уже вышло, знает
//! `services`.
//!
//! Каналов по одному на язык: английский, немецкий, испанский, итальянский
//! (русский и украинский тоже поддержаны — их называет окружение). Каждый
//! канал читает только свой язык, поэтому всё, что в него идёт, пишется на
//! нём. Нет перевода — выходит текст на другом языке по цепочке откатов
//! сайта (`Lang::fallbacks`), и это сказано в первой строке поста
//! (`untranslated`): канал на немецком не должен молча получать английский
//! текст под видом немецкого.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::lang::Lang;
use crate::telegram::esc;

/// Слово на языке из шести: порядок тот же, что у колонок в базе —
/// en, ru, uk, es, de, it.
pub fn by(lang: Lang, words: [&'static str; 6]) -> &'static str {
    words[match lang {
        Lang::En => 0,
        Lang::Ru => 1,
        Lang::Uk => 2,
        Lang::Es => 3,
        Lang::De => 4,
        Lang::It => 5,
    }]
}

/// Первый непустой текст по цепочке откатов и язык, на котором он написан.
/// Колонки в порядке en, ru, uk, es, de, it.
pub fn pick_text(lang: Lang, cols: [Option<&str>; 6]) -> Option<(String, Lang)> {
    for l in lang.fallbacks() {
        let side = match l {
            Lang::En => cols[0],
            Lang::Ru => cols[1],
            Lang::Uk => cols[2],
            Lang::Es => cols[3],
            Lang::De => cols[4],
            Lang::It => cols[5],
        };
        if let Some(s) = side.map(str::trim).filter(|s| !s.is_empty()) {
            return Some((s.to_string(), l));
        }
    }
    None
}

/// То же, но в каждом языке сначала первая колонка, потом вторая: у карты
/// это «приписка, а без неё эффект» — свой язык с эффектом раньше чужой
/// приписки.
pub fn pick_text_or(
    lang: Lang,
    first: [Option<&str>; 6],
    second: [Option<&str>; 6],
) -> Option<(String, Lang)> {
    for l in lang.fallbacks() {
        let at = |cols: [Option<&str>; 6]| {
            pick_text(l, cols).filter(|(_, from)| *from == l).map(|(t, _)| t)
        };
        if let Some(t) = at(first).or_else(|| at(second)) {
            return Some((t, l));
        }
    }
    None
}

/// Слово рода и, если часть текста написана не на языке канала, пометка об
/// этом. `written_in` — языки, на которых написано то, что вошло в пост.
pub fn labelled(lang: Lang, label: &str, written_in: &[Lang]) -> String {
    match written_in.iter().find(|l| **l != lang) {
        Some(from) => format!("{label} · {}", untranslated(lang, *from)),
        None => label.to_string(),
    }
}

/// Что объявляется: от этого зависят слово рода, метка и кнопка.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Work,
    Tale,
    GuestStory,
    Sketch,
    Showing,
    Collage,
    Arrival,
    /// Прочие листы вестника.
    Leaf,
    Card,
    EtudeSolo,
    /// Встреча со своей колодой.
    EtudeDeck,
}

/// К какому списку релизного поста относится вещь.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Works,
    Tales,
    Game,
    /// Листы вестника, не байки: в релизный пост не идут.
    Other,
}

impl Sort {
    pub fn group(self) -> Group {
        match self {
            Sort::Work => Group::Works,
            Sort::Tale | Sort::GuestStory => Group::Tales,
            Sort::Card | Sort::EtudeSolo | Sort::EtudeDeck => Group::Game,
            _ => Group::Other,
        }
    }
}

/// Род вещи словами, метка для поиска по каналу и текст кнопки.
pub struct Words {
    pub label: &'static str,
    pub tag: &'static str,
    pub button: &'static str,
}

const OPEN_SITE: [&str; 6] = [
    "Open on the site",
    "Открыть на сайте",
    "Відкрити на сайті",
    "Abrir en la web",
    "Auf der Seite öffnen",
    "Apri sul sito",
];

pub fn words(lang: Lang, sort: Sort) -> Words {
    let (label, tag, button) = match sort {
        Sort::Work => (
            ["New work", "Новая работа", "Нова робота", "Nueva obra", "Neues Werk", "Nuova opera"],
            ["work", "работа", "робота", "obra", "werk", "opera"],
            [
                "View on the site",
                "Смотреть на сайте",
                "Дивитися на сайті",
                "Ver en la web",
                "Auf der Seite ansehen",
                "Vedi sul sito",
            ],
        ),
        Sort::Tale => (
            [
                "New tale",
                "Новая байка",
                "Нова небилиця",
                "Nuevo cuento",
                "Neue Geschichte",
                "Nuovo racconto",
            ],
            ["tale", "байка", "небилиця", "cuento", "geschichte", "racconto"],
            ["Read", "Читать", "Читати", "Leer", "Lesen", "Leggi"],
        ),
        Sort::GuestStory => (
            [
                "Guest story",
                "Гостевая история",
                "Гостьова історія",
                "Historia de un visitante",
                "Gastgeschichte",
                "Storia di un ospite",
            ],
            ["story", "история", "історія", "historia", "gastgeschichte", "storia"],
            ["Read", "Читать", "Читати", "Leer", "Lesen", "Leggi"],
        ),
        Sort::Sketch => (
            [
                "Sketch from the workshop",
                "Эскиз из мастерской",
                "Ескіз із майстерні",
                "Boceto del taller",
                "Skizze aus der Werkstatt",
                "Schizzo dal laboratorio",
            ],
            ["sketch", "эскиз", "ескіз", "boceto", "skizze", "schizzo"],
            OPEN_SITE,
        ),
        Sort::Showing => (
            ["Showing", "Показ", "Показ", "Exposición", "Besichtigung", "Visione"],
            ["showing", "показ", "показ", "exposición", "besichtigung", "visione"],
            OPEN_SITE,
        ),
        Sort::Collage => (
            ["Collage", "Коллаж", "Колаж", "Collage", "Collage", "Collage"],
            ["collage", "коллаж", "колаж", "collage", "collage", "collage"],
            OPEN_SITE,
        ),
        Sort::Arrival => (
            [
                "New arrival",
                "Новое поступление",
                "Нове надходження",
                "Nueva llegada",
                "Neuzugang",
                "Nuovo arrivo",
            ],
            ["arrival", "поступление", "надходження", "llegada", "neuzugang", "arrivo"],
            OPEN_SITE,
        ),
        Sort::Leaf => (
            [
                "Gazette leaf",
                "Лист вестника",
                "Аркуш вісника",
                "Hoja de la gaceta",
                "Zeitungsblatt",
                "Foglio della gazzetta",
            ],
            ["gazette", "вестник", "вісник", "gaceta", "zeitung", "gazzetta"],
            OPEN_SITE,
        ),
        Sort::Card => (
            [
                "New card in the game",
                "Новая карта в игре",
                "Нова карта в грі",
                "Nueva carta en el juego",
                "Neue Karte im Spiel",
                "Nuova carta nel gioco",
            ],
            ["card", "карта", "карта", "carta", "karte", "carta"],
            [
                "Open the card",
                "Открыть карту",
                "Відкрити карту",
                "Abrir la carta",
                "Karte öffnen",
                "Apri la carta",
            ],
        ),
        Sort::EtudeSolo => (
            [
                "New etude in the game",
                "Новый этюд в игре",
                "Новий етюд у грі",
                "Nuevo desafío en el juego",
                "Neue Etüde im Spiel",
                "Nuova étude nel gioco",
            ],
            ["etude", "этюд", "етюд", "desafío", "etüde", "étude"],
            ["Play", "Сыграть", "Зіграти", "Jugar", "Spielen", "Gioca"],
        ),
        Sort::EtudeDeck => (
            [
                "New encounter in the game",
                "Новая встреча в игре",
                "Нова зустріч у грі",
                "Nuevo encuentro en el juego",
                "Neue Begegnung im Spiel",
                "Nuovo incontro nel gioco",
            ],
            ["etude", "этюд", "етюд", "desafío", "etüde", "étude"],
            ["Play", "Сыграть", "Зіграти", "Jugar", "Spielen", "Gioca"],
        ),
    };
    Words {
        label: by(lang, label),
        tag: by(lang, tag),
        button: by(lang, button),
    }
}

/// Пометка «текст на другом языке». Первый довод — язык канала, второй —
/// язык, на котором текст написан.
pub fn untranslated(channel: Lang, written_in: Lang) -> &'static str {
    let names: [&'static str; 6] = match channel {
        Lang::En => ["In English", "In Russian", "In Ukrainian", "In Spanish", "In German", "In Italian"],
        Lang::Ru => [
            "На английском",
            "На русском",
            "На украинском",
            "На испанском",
            "На немецком",
            "На итальянском",
        ],
        Lang::Uk => [
            "Англійською",
            "Російською",
            "Українською",
            "Іспанською",
            "Німецькою",
            "Італійською",
        ],
        Lang::Es => ["En inglés", "En ruso", "En ucraniano", "En español", "En alemán", "En italiano"],
        Lang::De => [
            "Auf Englisch",
            "Auf Russisch",
            "Auf Ukrainisch",
            "Auf Spanisch",
            "Auf Deutsch",
            "Auf Italienisch",
        ],
        Lang::It => [
            "In inglese",
            "In russo",
            "In ucraino",
            "In spagnolo",
            "In tedesco",
            "In italiano",
        ],
    };
    by(written_in, names)
}

// ============================================================
// ЗАМЕТКИ РЕЛИЗА
// ============================================================

/// Сколько пунктов каждого рода и какой длины пост принимает. Пост о
/// релизе читают в ленте, а не изучают: больше шести строк — уже список
/// изменений, а не новость.
pub const NOTES_PER_KIND: usize = 6;
pub const NOTE_MAX_CHARS: usize = 220;

/// Пункты релиза на одном языке.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangNotes {
    #[serde(default)]
    pub new: Vec<String>,
    #[serde(default)]
    pub better: Vec<String>,
}

impl LangNotes {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.better.is_empty()
    }

    /// Пустые строки уходят, лишние пробелы схлопываются, длинное и
    /// лишнее обрезается: правит человек, и сервер не должен верить форме.
    pub fn cleaned(&self) -> LangNotes {
        let clean = |items: &[String]| -> Vec<String> {
            items
                .iter()
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|s| !s.is_empty())
                .map(|s| s.chars().take(NOTE_MAX_CHARS).collect::<String>())
                .take(NOTES_PER_KIND)
                .collect()
        };
        LangNotes {
            new: clean(&self.new),
            better: clean(&self.better),
        }
    }
}

/// Пункты по языкам: ключ — код языка.
pub type Notes = BTreeMap<String, LangNotes>;

pub fn clean_notes(notes: &Notes) -> Notes {
    notes
        .iter()
        .filter_map(|(code, n)| {
            let lang = Lang::parse(code)?;
            let n = n.cleaned();
            (!n.is_empty()).then(|| (lang.code().to_string(), n))
        })
        .collect()
}

/// Один блок файла заметок.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseBlock {
    pub id: String,
    pub notes: Notes,
}

/// Разобрать файл заметок. Новейший релиз стоит первым.
///
/// ```text
/// ## v.14 · 2026-10-22
/// ### en
/// new: Spanish, German and Italian are on the site
/// better: Pages open faster
/// ### de
/// new: …
/// ```
///
/// Разбор терпимый: пустые строки, строки не из этих четырёх видов и пункты
/// вне языка молча пропускаются. Файл пишет человек, и из-за лишней строки
/// релиз не должен остаться без поста.
pub fn parse_release_notes(src: &str) -> Vec<ReleaseBlock> {
    let mut blocks: Vec<ReleaseBlock> = Vec::new();
    let mut lang: Option<Lang> = None;
    for raw in src.lines() {
        let line = raw.trim();
        if let Some(rest) = line.strip_prefix("### ") {
            lang = Lang::parse(&rest.to_ascii_lowercase());
        } else if let Some(rest) = line.strip_prefix("## ") {
            let id = rest
                .split(|c: char| c.is_whitespace() || c == '·')
                .find(|s| !s.is_empty())
                .unwrap_or("")
                .to_string();
            lang = None;
            if !id.is_empty() {
                blocks.push(ReleaseBlock { id, notes: Notes::new() });
            }
        } else if let (Some(block), Some(l)) = (blocks.last_mut(), lang) {
            let entry = block.notes.entry(l.code().to_string()).or_default();
            if let Some(text) = line.strip_prefix("new:") {
                entry.new.push(text.trim().to_string());
            } else if let Some(text) = line.strip_prefix("better:") {
                entry.better.push(text.trim().to_string());
            }
        }
    }
    for block in &mut blocks {
        block.notes = clean_notes(&block.notes);
    }
    blocks
}

/// Пункты для канала: свои, а нет своих — по цепочке откатов. Возвращает
/// язык, на котором они написаны.
pub fn notes_for(lang: Lang, notes: &Notes) -> Option<(LangNotes, Lang)> {
    for l in lang.fallbacks() {
        if let Some(n) = notes.get(l.code()).filter(|n| !n.is_empty()) {
            return Some((n.clone(), l));
        }
    }
    None
}

// ============================================================
// ПОСТ О РЕЛИЗЕ
// ============================================================

/// Вещь в списке поста: название на языке канала и куда ведёт.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NewItem {
    pub title: String,
    pub url: String,
}

/// Что вышло на сайте с прошлого релиза, по спискам.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ReleaseNews {
    pub works: Vec<NewItem>,
    pub tales: Vec<NewItem>,
    pub game: Vec<NewItem>,
}

impl ReleaseNews {
    pub fn is_empty(&self) -> bool {
        self.works.is_empty() && self.tales.is_empty() && self.game.is_empty()
    }

    /// Раздел сайта, где новое видно лучше всего: тот список, в котором
    /// больше записей. Ничего нового — главная.
    pub fn section(&self) -> &'static str {
        let (w, t, g) = (self.works.len(), self.tales.len(), self.game.len());
        if w == 0 && t == 0 && g == 0 {
            "/"
        } else if w >= t && w >= g {
            "/figurines"
        } else if t >= g {
            "/tales"
        } else {
            "/battles/etude"
        }
    }
}

/// Сколько вещей одного списка называется в посте; остальные — числом.
const LIST_SHOWN: usize = 8;

pub fn release_button(lang: Lang) -> &'static str {
    by(
        lang,
        [
            "See what's new",
            "Смотреть, что нового",
            "Дивитися, що нового",
            "Ver las novedades",
            "Neues ansehen",
            "Scopri le novità",
        ],
    )
}

fn linked_list(items: &[NewItem]) -> String {
    let mut out: Vec<String> = items
        .iter()
        .take(LIST_SHOWN)
        .map(|i| format!("<a href=\"{}\">{}</a>", esc(&i.url), esc(&i.title)))
        .collect();
    if items.len() > LIST_SHOWN {
        out.push(format!("+{}", items.len() - LIST_SHOWN));
    }
    out.join(", ")
}

/// Текст поста о релизе для канала одного языка.
///
/// ```text
/// ✦ Site update · v.14 · Auf Englisch   ← пометка только без перевода
///
/// Neu                                    ← жирным
/// • пункт
///
/// Besser
/// • пункт
///
/// Neue Werke: Название, Название
/// Neu im Spiel: Название
///
/// #update
/// ```
///
/// Тон тот же, что у остального канала: без восклицаний, цен и «успейте»
/// (CLAUDE.md § 1). `None` — писать нечего: ни пунктов, ни нового.
pub fn release_caption(lang: Lang, id: &str, notes: &Notes, news: &ReleaseNews) -> Option<String> {
    let own = notes_for(lang, notes);
    if own.is_none() && news.is_empty() {
        return None;
    }

    let mut head = format!(
        "✦ {} · {}",
        by(
            lang,
            [
                "Site update",
                "Обновление сайта",
                "Оновлення сайту",
                "Novedades del sitio",
                "Update der Seite",
                "Novità dal sito"
            ]
        ),
        id
    );
    if let Some((_, written_in)) = &own {
        if *written_in != lang {
            head.push_str(" · ");
            head.push_str(untranslated(lang, *written_in));
        }
    }
    let mut out = esc(&head);

    if let Some((n, _)) = &own {
        let section = |title: &'static str, items: &[String], out: &mut String| {
            if items.is_empty() {
                return;
            }
            out.push_str(&format!("\n\n<b>{}</b>", esc(title)));
            for item in items {
                out.push_str(&format!("\n• {}", esc(item)));
            }
        };
        section(
            by(lang, ["New", "Новое", "Нове", "Novedades", "Neu", "Novità"]),
            &n.new,
            &mut out,
        );
        section(
            by(lang, ["Better", "Лучше", "Краще", "Mejoras", "Besser", "Miglioramenti"]),
            &n.better,
            &mut out,
        );
    }

    if !news.is_empty() {
        out.push('\n');
        let lists = [
            (
                by(
                    lang,
                    [
                        "New works",
                        "Новые работы",
                        "Нові роботи",
                        "Obras nuevas",
                        "Neue Werke",
                        "Nuove opere",
                    ],
                ),
                &news.works,
            ),
            (
                by(
                    lang,
                    [
                        "New tales",
                        "Новые байки",
                        "Нові небилиці",
                        "Cuentos nuevos",
                        "Neue Geschichten",
                        "Nuovi racconti",
                    ],
                ),
                &news.tales,
            ),
            (
                by(
                    lang,
                    [
                        "New in the game",
                        "Новое в игре",
                        "Нове в грі",
                        "Novedades del juego",
                        "Neu im Spiel",
                        "Novità nel gioco",
                    ],
                ),
                &news.game,
            ),
        ];
        for (title, items) in lists {
            if !items.is_empty() {
                out.push_str(&format!("\n{}: {}", esc(title), linked_list(items)));
            }
        }
    }

    let tag = by(
        lang,
        ["update", "обновление", "оновлення", "novedades", "update", "novità"],
    );
    out.push_str(&format!("\n\n#{tag}"));
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(title: &str) -> NewItem {
        NewItem {
            title: title.into(),
            url: format!("https://x.test/{title}?src=telegram&lang=de"),
        }
    }

    #[test]
    fn the_notes_file_is_read_newest_first_and_tolerates_stray_lines() {
        let src = "intro line\n\n## v.14 · 2026-10-22\nstray\n### en\nnew:  Spanish  and German \nbetter: Faster pages\n\n### DE\nnew: Spanisch\n### xx\nnew: lost\n\n## v.13 · 2026-10-01\n### en\nnew: Old\n";
        let blocks = parse_release_notes(src);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].id, "v.14");
        assert_eq!(blocks[0].notes["en"].new, vec!["Spanish and German"]);
        assert_eq!(blocks[0].notes["en"].better, vec!["Faster pages"]);
        assert_eq!(blocks[0].notes["de"].new, vec!["Spanisch"]);
        assert!(!blocks[0].notes.contains_key("xx"));
        assert_eq!(blocks[1].id, "v.13");
    }

    #[test]
    fn notes_are_trimmed_and_limited() {
        let n = LangNotes {
            new: (0..10).map(|i| format!("  item {i}  ")).chain([" ".to_string()]).collect(),
            better: vec!["x".repeat(500)],
        };
        let c = n.cleaned();
        assert_eq!(c.new.len(), NOTES_PER_KIND);
        assert_eq!(c.new[0], "item 0");
        assert_eq!(c.better[0].chars().count(), NOTE_MAX_CHARS);
    }

    #[test]
    fn a_channel_without_its_own_notes_gets_english_and_says_so() {
        let mut notes = Notes::new();
        notes.insert(
            "en".into(),
            LangNotes { new: vec!["Spanish is on the site".into()], better: vec![] },
        );
        let de = release_caption(Lang::De, "v.14", &notes, &ReleaseNews::default()).unwrap();
        assert!(de.starts_with("✦ Update der Seite · v.14 · Auf Englisch"), "{de}");
        assert!(de.contains("<b>Neu</b>\n• Spanish is on the site"), "{de}");
        let en = release_caption(Lang::En, "v.14", &notes, &ReleaseNews::default()).unwrap();
        assert!(!en.contains("In English"), "{en}");

        notes.insert("de".into(), LangNotes { new: vec!["Spanisch ist da".into()], better: vec![] });
        let de = release_caption(Lang::De, "v.14", &notes, &ReleaseNews::default()).unwrap();
        assert!(!de.contains("Auf Englisch"), "{de}");
        assert!(de.contains("• Spanisch ist da"));
    }

    #[test]
    fn nothing_to_say_means_no_post() {
        assert!(release_caption(Lang::En, "v.1", &Notes::new(), &ReleaseNews::default()).is_none());
        let news = ReleaseNews { works: vec![item("Raven")], ..Default::default() };
        let text = release_caption(Lang::En, "v.1", &Notes::new(), &news).unwrap();
        assert!(text.contains("New works: <a href=\"https://x.test/Raven?src=telegram&amp;lang=de\">Raven</a>"), "{text}");
        assert!(text.ends_with("#update"));
    }

    #[test]
    fn a_long_list_is_cut_to_a_number() {
        let works: Vec<NewItem> = (0..11).map(|i| item(&format!("W{i}"))).collect();
        let news = ReleaseNews { works, ..Default::default() };
        let text = release_caption(Lang::En, "v.1", &Notes::new(), &news).unwrap();
        assert!(text.contains("W7</a>, +3"), "{text}");
        assert!(!text.contains("W8"));
    }

    #[test]
    fn titles_and_notes_are_escaped() {
        let mut notes = Notes::new();
        notes.insert("en".into(), LangNotes { new: vec!["A <b> & B".into()], better: vec![] });
        let text = release_caption(Lang::En, "v.1", &notes, &ReleaseNews::default()).unwrap();
        assert!(text.contains("A &lt;b&gt; &amp; B"), "{text}");
    }

    #[test]
    fn the_section_is_the_one_with_most_news() {
        let one = vec![item("a")];
        let two = vec![item("a"), item("b")];
        let none = ReleaseNews::default();
        assert_eq!(none.section(), "/");
        assert_eq!(ReleaseNews { works: two.clone(), tales: one.clone(), game: vec![] }.section(), "/figurines");
        assert_eq!(ReleaseNews { works: vec![], tales: two.clone(), game: one.clone() }.section(), "/tales");
        assert_eq!(ReleaseNews { works: vec![], tales: one, game: two }.section(), "/battles/etude");
    }

    #[test]
    fn text_falls_back_through_the_site_chain() {
        let got = pick_text(Lang::De, [Some("en"), Some("ru"), None, None, None, None]).unwrap();
        assert_eq!(got, ("en".to_string(), Lang::En));
        let own = pick_text(Lang::De, [Some("en"), None, None, None, Some(" de "), None]).unwrap();
        assert_eq!(own, ("de".to_string(), Lang::De));
        assert!(pick_text(Lang::It, [None, None, None, None, None, None]).is_none());
    }

    #[test]
    fn a_pair_of_columns_prefers_the_channels_own_language() {
        // Свой язык: приписки нет, но есть эффект — берётся эффект, а не
        // английская приписка.
        let got = pick_text_or(
            Lang::De,
            [Some("lore en"), None, None, None, None, None],
            [None, None, None, None, Some("Effekt"), None],
        )
        .unwrap();
        assert_eq!(got, ("Effekt".to_string(), Lang::De));
        let fallback = pick_text_or(
            Lang::De,
            [Some("lore en"), None, None, None, None, None],
            [None, None, None, None, None, None],
        )
        .unwrap();
        assert_eq!(fallback, ("lore en".to_string(), Lang::En));
    }

    #[test]
    fn the_label_names_a_foreign_text_only() {
        assert_eq!(labelled(Lang::De, "Neue Karte", &[Lang::De, Lang::De]), "Neue Karte");
        assert_eq!(
            labelled(Lang::De, "Neue Karte", &[Lang::De, Lang::En]),
            "Neue Karte · Auf Englisch"
        );
    }

    #[test]
    fn every_language_has_every_word() {
        for l in Lang::ALL {
            for s in [
                Sort::Work,
                Sort::Tale,
                Sort::GuestStory,
                Sort::Sketch,
                Sort::Showing,
                Sort::Collage,
                Sort::Arrival,
                Sort::Leaf,
                Sort::Card,
                Sort::EtudeSolo,
                Sort::EtudeDeck,
            ] {
                let w = words(l, s);
                assert!(!w.label.is_empty() && !w.tag.is_empty() && !w.button.is_empty(), "{l:?} {s:?}");
            }
            assert!(!release_button(l).is_empty());
        }
    }
}
