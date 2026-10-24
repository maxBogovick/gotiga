//! Готовые обложки книг небылиц.
//!
//! Картинки встроены в сервер и кладутся на склад при запуске, как
//! снаряжение штурма (`siege.rs`): миграция (`UPDATE gazette_leaves SET
//! book_cover = '/static/covers/…'`) ссылается на эти имена, а сами файлы
//! приезжают с выкладкой, и загружать их руками не нужно. Обрезку до 2:3 и
//! перевод в WebP делает `tools/tale_covers/prepare.py`.

use std::path::Path;

/// Имя файла на складе (`uploads/covers/`) и байты.
pub const COVER_ART: [(&str, &[u8]); 11] = [
    ("book-gb.webp", include_bytes!("../assets/covers/book-gb.webp")),
    ("book-ispolnyu-vse-zhelaniya.webp", include_bytes!("../assets/covers/book-ispolnyu-vse-zhelaniya.webp")),
    ("book-lechu.webp", include_bytes!("../assets/covers/book-lechu.webp")),
    ("book-mir-ne-takoy-kak-tebe.webp", include_bytes!("../assets/covers/book-mir-ne-takoy-kak-tebe.webp")),
    ("book-molodost-bystree-tvoih.webp", include_bytes!("../assets/covers/book-molodost-bystree-tvoih.webp")),
    ("book-ne-odinoko-grustno.webp", include_bytes!("../assets/covers/book-ne-odinoko-grustno.webp")),
    ("book-otmeryalschik-ot-boga.webp", include_bytes!("../assets/covers/book-otmeryalschik-ot-boga.webp")),
    ("book-the-forester-and-his-unforgettable-memory.webp", include_bytes!("../assets/covers/book-the-forester-and-his-unforgettable-memory.webp")),
    ("book-ubit-a-potom-razorvat.webp", include_bytes!("../assets/covers/book-ubit-a-potom-razorvat.webp")),
    ("book-vse-kak-u-nas.webp", include_bytes!("../assets/covers/book-vse-kak-u-nas.webp")),
    ("book-ya-veryu.webp", include_bytes!("../assets/covers/book-ya-veryu.webp")),
];

/// Положить обложки на склад. Пишет только недостающее или отличающееся:
/// перерисованная обложка приезжает со следующей выкладкой.
pub fn lay_cover_art(upload_dir: &Path) -> std::io::Result<usize> {
    let dir = upload_dir.join("covers");
    std::fs::create_dir_all(&dir)?;
    let mut written = 0;
    for (name, bytes) in COVER_ART {
        let path = dir.join(name);
        if std::fs::read(&path).ok().as_deref() != Some(bytes) {
            std::fs::write(&path, bytes)?;
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_are_laid_once_and_relaid_when_changed() {
        let dir = std::env::temp_dir().join(format!("gotiga-covers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(lay_cover_art(&dir).unwrap(), COVER_ART.len(), "пустой склад — все");
        assert_eq!(lay_cover_art(&dir).unwrap(), 0, "уже лежат — не трогать");
        std::fs::write(dir.join("covers").join(COVER_ART[0].0), b"old").unwrap();
        assert_eq!(lay_cover_art(&dir).unwrap(), 1, "отличается — переложить");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_cover_is_a_webp_with_a_slug_name() {
        for (name, bytes) in COVER_ART {
            assert!(name.starts_with("book-") && name.ends_with(".webp"), "{name}");
            assert_eq!(&bytes[..4], b"RIFF", "{name}");
            assert_eq!(&bytes[8..12], b"WEBP", "{name}");
        }
    }
}
