//! Transliterated URL slugs for figurines (desktop build).
//!
//! Mirrors `gotiga-server`'s `slug` module so the Tauri app generates the same
//! slugs as the web server. Figurine names are Cyrillic («Хранительница
//! порога»); a share-link URL wants Latin ASCII (`hranitelnica-poroga`).
//!
//! DRIFT WARNING: this is a verbatim copy of `src-tauri/server/src/slug.rs`.
//! Slugs are persisted, so both backends MUST produce identical output for the
//! same name — apply any change to the transliteration map or collapsing rules
//! to both files, and keep the unit tests below in sync.

/// Transliterate one lowercase Cyrillic char to its Latin equivalent.
fn translit_char(c: char) -> Option<&'static str> {
    Some(match c {
        'а' => "a", 'б' => "b", 'в' => "v", 'г' => "g", 'д' => "d",
        'е' => "e", 'ё' => "yo", 'ж' => "zh", 'з' => "z", 'и' => "i",
        'й' => "y", 'к' => "k", 'л' => "l", 'м' => "m", 'н' => "n",
        'о' => "o", 'п' => "p", 'р' => "r", 'с' => "s", 'т' => "t",
        'у' => "u", 'ф' => "f", 'х' => "h", 'ц' => "c", 'ч' => "ch",
        'ш' => "sh", 'щ' => "sch", 'ъ' => "", 'ы' => "y", 'ь' => "",
        'э' => "e", 'ю' => "yu", 'я' => "ya",
        'і' => "i", 'ї' => "yi", 'є' => "ye", 'ґ' => "g",
        _ => return None,
    })
}

/// Build a URL slug from a work's name. Empty string when the name has no usable
/// characters (callers fall back to the work's id).
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = true;

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

    // Keep in sync with src-tauri/server/src/slug.rs — both backends must agree.
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
