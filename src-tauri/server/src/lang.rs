//! The six languages of the site, for the text the server writes itself:
//! letters, notes in Telegram, the line that goes into someone's conversation.
//!
//! Text kept in the database has its own columns (`title_en`, `title_ru`,
//! `title_uk`, …); this is only about what the server composes. One enum, so a
//! new language is one line here and a compile error at every place that
//! has to write something for it — instead of a `bool` that quietly says
//! "Russian, or else English".

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    En,
    Ru,
    Uk,
    Es,
    De,
    It,
}

impl Lang {
    /// Every language, in the order the database columns are written.
    pub const ALL: [Lang; 6] = [Lang::En, Lang::Ru, Lang::Uk, Lang::Es, Lang::De, Lang::It];

    /// An exact code as stored in the database: `en`, `ru`, `uk`, `es`, `de`
    /// or `it`.
    pub fn parse(raw: &str) -> Option<Lang> {
        match raw.trim() {
            "en" => Some(Lang::En),
            "ru" => Some(Lang::Ru),
            "uk" => Some(Lang::Uk),
            "es" => Some(Lang::Es),
            "de" => Some(Lang::De),
            "it" => Some(Lang::It),
            _ => None,
        }
    }

    /// A language tag as browsers and Telegram send it (`uk`, `uk-UA`,
    /// `de-AT`): the part before the dash decides. Anything else is English —
    /// the world language, the one nobody is surprised by.
    pub fn from_tag(raw: Option<&str>) -> Lang {
        let tag = raw.unwrap_or("").trim().to_ascii_lowercase();
        let base = tag.split(['-', '_']).next().unwrap_or("");
        Lang::parse(base).unwrap_or(Lang::En)
    }

    /// The code that goes into the database and over the wire.
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Ru => "ru",
            Lang::Uk => "uk",
            Lang::Es => "es",
            Lang::De => "de",
            Lang::It => "it",
        }
    }

    /// One text per language, in the column order: en, ru, uk, es, de, it.
    pub fn pick<'a>(
        self,
        en: &'a str,
        ru: &'a str,
        uk: &'a str,
        es: &'a str,
        de: &'a str,
        it: &'a str,
    ) -> &'a str {
        match self {
            Lang::En => en,
            Lang::Ru => ru,
            Lang::Uk => uk,
            Lang::Es => es,
            Lang::De => de,
            Lang::It => it,
        }
    }

    /// The order in which a stored text is looked for when the reader's own
    /// language is empty: own, then English, Russian and Ukrainian (a Russian
    /// reader gets English first, a Ukrainian or Spanish one gets English
    /// before Russian: an untranslated text should not be Russian by
    /// default), then the rest.
    pub fn fallbacks(self) -> [Lang; 6] {
        let head = match self {
            Lang::Ru => [Lang::Ru, Lang::En, Lang::Uk],
            Lang::Uk => [Lang::Uk, Lang::En, Lang::Ru],
            _ => [Lang::En, Lang::Ru, Lang::Uk],
        };
        let mut out = [self; 6];
        let mut n = 1;
        for l in head.into_iter().chain(Lang::ALL) {
            if !out[..n].contains(&l) {
                out[n] = l;
                n += 1;
            }
        }
        out
    }
}

/// The first non-empty text for this language, trying the others in the order
/// `Lang::fallbacks` gives. Blank strings do not count.
pub fn in_lang(
    lang: Lang,
    en: Option<&str>,
    ru: Option<&str>,
    uk: Option<&str>,
    es: Option<&str>,
    de: Option<&str>,
    it: Option<&str>,
) -> Option<String> {
    for l in lang.fallbacks() {
        let side = match l {
            Lang::En => en,
            Lang::Ru => ru,
            Lang::Uk => uk,
            Lang::Es => es,
            Lang::De => de,
            Lang::It => it,
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
        assert_eq!(Lang::from_tag(Some("de-AT")), Lang::De);
        assert_eq!(Lang::from_tag(Some("es-MX")), Lang::Es);
        assert_eq!(Lang::from_tag(Some("IT")), Lang::It);
        assert_eq!(Lang::from_tag(Some("fr")), Lang::En);
        assert_eq!(Lang::from_tag(None), Lang::En);
    }

    #[test]
    fn only_exact_codes_parse() {
        assert_eq!(Lang::parse("uk"), Some(Lang::Uk));
        assert_eq!(Lang::parse(" ru "), Some(Lang::Ru));
        assert_eq!(Lang::parse("de"), Some(Lang::De));
        assert_eq!(Lang::parse("ua"), None);
        assert_eq!(Lang::parse(""), None);
        for l in Lang::ALL {
            assert_eq!(Lang::parse(l.code()), Some(l));
        }
    }

    #[test]
    fn every_language_falls_back_through_all_six_once() {
        for l in Lang::ALL {
            let order = l.fallbacks();
            assert_eq!(order[0], l);
            for other in Lang::ALL {
                assert_eq!(order.iter().filter(|x| **x == other).count(), 1, "{l:?}");
            }
        }
        assert_eq!(
            Lang::It.fallbacks()[..4],
            [Lang::It, Lang::En, Lang::Ru, Lang::Uk]
        );
    }

    #[test]
    fn a_ukrainian_reader_is_shown_english_before_russian() {
        let text = in_lang(Lang::Uk, Some("tale"), Some("байка"), None, None, None, None);
        assert_eq!(text.as_deref(), Some("tale"));
        let own = in_lang(Lang::Uk, Some("tale"), Some("байка"), Some("оповідь"), None, None, None);
        assert_eq!(own.as_deref(), Some("оповідь"));
        let only_ru = in_lang(Lang::Uk, Some("  "), Some("байка"), None, None, None, None);
        assert_eq!(only_ru.as_deref(), Some("байка"));
    }

    #[test]
    fn a_german_reader_gets_german_then_english() {
        let own = in_lang(Lang::De, Some("tale"), Some("байка"), None, Some("cuento"), Some("Märchen"), None);
        assert_eq!(own.as_deref(), Some("Märchen"));
        let en = in_lang(Lang::De, Some("tale"), Some("байка"), None, Some("cuento"), None, None);
        assert_eq!(en.as_deref(), Some("tale"));
        let es = in_lang(Lang::En, None, None, None, Some("cuento"), None, None);
        assert_eq!(es.as_deref(), Some("cuento"));
    }
}
