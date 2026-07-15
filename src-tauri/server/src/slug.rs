//! Transliterated URL slugs for figurines.
//!
//! Figurine names are Cyrillic (e.g. «Хранительница порога»); a share-link or
//! search-engine URL wants Latin ASCII (`hranitelnica-poroga`). `slugify`
//! transliterates Cyrillic → Latin (a GOST-ish map), lowercases, and collapses
//! everything else into single hyphens.
//!
//! DRIFT WARNING: this module is mirrored verbatim by the desktop build's
//! `src-tauri/src/slug.rs`. Slugs are persisted, so the web and desktop backends
//! MUST produce identical output for the same name — any change to the
//! transliteration map or collapsing rules here has to be applied to both files,
//! and the unit tests below kept in sync.

/// Transliterate one lowercase Cyrillic char to its Latin equivalent. Returns
/// `None` for characters that are not Cyrillic letters (the caller handles those
/// via the generic separator path).
fn translit_char(c: char) -> Option<&'static str> {
    Some(match c {
        'а' => "a", 'б' => "b", 'в' => "v", 'г' => "g", 'д' => "d",
        'е' => "e", 'ё' => "yo", 'ж' => "zh", 'з' => "z", 'и' => "i",
        'й' => "y", 'к' => "k", 'л' => "l", 'м' => "m", 'н' => "n",
        'о' => "o", 'п' => "p", 'р' => "r", 'с' => "s", 'т' => "t",
        'у' => "u", 'ф' => "f", 'х' => "h", 'ц' => "c", 'ч' => "ch",
        'ш' => "sh", 'щ' => "sch", 'ъ' => "", 'ы' => "y", 'ь' => "",
        'э' => "e", 'ю' => "yu", 'я' => "ya",
        // Ukrainian/other Cyrillic letters occasionally seen in names.
        'і' => "i", 'ї' => "yi", 'є' => "ye", 'ґ' => "g",
        _ => return None,
    })
}

/// Build a URL slug from a work's name. Cyrillic is transliterated to Latin,
/// ASCII letters/digits are kept, and any run of other characters (spaces,
/// punctuation, emoji) becomes a single `-`. Leading/trailing `-` are trimmed.
///
/// Returns an empty string when the name has no usable characters (e.g. it is
/// all punctuation); callers fall back to the work's UUID in that case.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = true; // true → suppresses a leading dash

    for c in name.chars() {
        let lower = c.to_lowercase().next().unwrap_or(c);
        if let Some(t) = translit_char(lower) {
            out.push_str(t);
            prev_dash = t.is_empty() && prev_dash;
        } else if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transliterates_cyrillic() {
        assert_eq!(slugify("Хранительница порога"), "hranitelnica-poroga");
        assert_eq!(slugify("Ворон"), "voron");
        assert_eq!(slugify("Чёрный ящик"), "chyornyy-yaschik");
    }

    #[test]
    fn keeps_ascii_and_collapses_separators() {
        assert_eq!(slugify("The  Raven —  Nevermore!"), "the-raven-nevermore");
        assert_eq!(slugify("Item #42"), "item-42");
    }

    #[test]
    fn empty_when_no_usable_chars() {
        assert_eq!(slugify("—!?"), "");
        assert_eq!(slugify(""), "");
    }
}
