//! The three languages of the site, for the text the server writes itself:
//! letters, notes in Telegram, the line that goes into someone's conversation.
//!
//! Text kept in the database has its own columns (`title_en`, `title_ru`,
//! `title_uk`); this is only about what the server composes. One enum, so a
//! fourth language is one line here and a compile error at every place that
//! has to write something for it — instead of a `bool` that quietly says
//! "Russian, or else English".

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ru,
    Uk,
}

impl Lang {
    /// An exact code as stored in the database: `en`, `ru` or `uk`.
    pub fn parse(raw: &str) -> Option<Lang> {
        match raw.trim() {
            "en" => Some(Lang::En),
            "ru" => Some(Lang::Ru),
            "uk" => Some(Lang::Uk),
            _ => None,
        }
    }

    /// A language tag as browsers and Telegram send it (`uk`, `uk-UA`,
    /// `ru-RU`): the part before the dash decides. Anything else is English —
    /// the world language, the one nobody is surprised by.
    pub fn from_tag(raw: Option<&str>) -> Lang {
        let tag = raw.unwrap_or("").trim().to_ascii_lowercase();
        let base = tag.split(['-', '_']).next().unwrap_or("");
        match base {
            "uk" => Lang::Uk,
            "ru" => Lang::Ru,
            _ => Lang::En,
        }
    }

    /// The code that goes into the database and over the wire.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
            Lang::Uk => "uk",
        }
    }

    /// One text per language.
    pub fn pick<'a>(self, en: &'a str, ru: &'a str, uk: &'a str) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Ru => ru,
            Lang::Uk => uk,
        }
    }

    /// The order in which a stored text is looked for when the reader's own
    /// language is empty: Ukrainian readers get English before Russian.
    pub fn fallbacks(self) -> [Lang; 3] {
        match self {
            Lang::En => [Lang::En, Lang::Ru, Lang::Uk],
            Lang::Ru => [Lang::Ru, Lang::En, Lang::Uk],
            Lang::Uk => [Lang::Uk, Lang::En, Lang::Ru],
        }
    }
}

/// The first non-empty text for this language, trying the others in the order
/// `Lang::fallbacks` gives. Blank strings do not count.
pub fn in_lang(
    lang: Lang,
    en: Option<&str>,
    ru: Option<&str>,
    uk: Option<&str>,
) -> Option<String> {
    for l in lang.fallbacks() {
        let side = match l {
            Lang::En => en,
            Lang::Ru => ru,
            Lang::Uk => uk,
        };
        if let Some(s) = side.map(str::trim).filter(|s| !s.is_empty()) {
            return Some(s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_browser_tag_picks_by_its_first_part() {
        assert_eq!(Lang::from_tag(Some("uk-UA")), Lang::Uk);
        assert_eq!(Lang::from_tag(Some("ru_RU")), Lang::Ru);
        assert_eq!(Lang::from_tag(Some("en-GB")), Lang::En);
        assert_eq!(Lang::from_tag(Some("de")), Lang::En);
        assert_eq!(Lang::from_tag(None), Lang::En);
    }

    #[test]
    fn only_exact_codes_parse() {
        assert_eq!(Lang::parse("uk"), Some(Lang::Uk));
        assert_eq!(Lang::parse(" ru "), Some(Lang::Ru));
        assert_eq!(Lang::parse("ua"), None);
        assert_eq!(Lang::parse(""), None);
    }

    #[test]
    fn a_ukrainian_reader_is_shown_english_before_russian() {
        let text = in_lang(Lang::Uk, Some("tale"), Some("байка"), None);
        assert_eq!(text.as_deref(), Some("tale"));
        let own = in_lang(Lang::Uk, Some("tale"), Some("байка"), Some("оповідь"));
        assert_eq!(own.as_deref(), Some("оповідь"));
        let only_ru = in_lang(Lang::Uk, Some("  "), Some("байка"), None);
        assert_eq!(only_ru.as_deref(), Some("байка"));
    }
}
