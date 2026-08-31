//! Cabinet gazette — pure helpers for house leaves and world-desk RSS.
//!
//! No database here. The service layer uses `leaf_is_live`, `cutting_is_live`,
//! slug uniqueness, kind/status checks, and `parse_feed` so the blotter never
//! auto-publishes a raw headline as if the house had written it. World cuttings
//! sit in a private inbox until the keeper pins them.

use crate::slug::slugify;
use chrono::{DateTime, NaiveDate, Utc};

pub const LEAF_KINDS: &[&str] = &[
    "arrival",
    "collage",
    "showing",
    "guest_story",
    "tale",
    "note",
    "world",
    "sketch",
];
pub const IMAGE_URLS_MAX: usize = 8;
pub const LEAF_STATUSES: &[&str] = &["draft", "scheduled", "published", "archived"];

pub const TITLE_MAX: usize = 200;
pub const DEK_MAX: usize = 500;
pub const BODY_MAX: usize = 12_000;
pub const EXCERPT_MAX: usize = 280;
pub const HOME_LEAVES: i64 = 4;
pub const HOME_CUTTINGS: i64 = 10;
/// A year volume of the room — enough for a blotter, not a catalogue dump.
pub const ROOM_LEAVES: i64 = 200;
pub const ROOM_CUTTINGS: i64 = 80;
pub const WORK_LEAVES: i64 = 8;
/// The shelf of tall tales, handed out whole — it is a shelf, not a feed.
pub const SHELF_TALES: i64 = 500;

pub const MARK_KEYS: &[&str] = &[
    "pillar", "hive", "boom", "quill", "lens", "shard", "coil", "letter",
];

pub fn valid_mark_key(key: &str) -> bool {
    MARK_KEYS.contains(&key)
}

/// A quiet house stamp for a new desk, guessed from the feed's name or url.
pub fn guess_mark_key(title: &str, url: &str) -> String {
    let hay = format!("{title} {url}").to_ascii_lowercase();
    if hay.contains("colossal") {
        "pillar".into()
    } else if hay.contains("hyperallergic") {
        "hive".into()
    } else if hay.contains("designboom") {
        "boom".into()
    } else {
        "letter".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFeedItem {
    pub guid: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub published_at: Option<DateTime<Utc>>,
}

pub fn valid_kind(kind: &str) -> bool {
    LEAF_KINDS.contains(&kind)
}

/// A day, a span, or nothing. Only sketch leaves keep a date; others clear it.
pub fn normalize_expected(
    kind: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<(Option<NaiveDate>, Option<NaiveDate>), String> {
    if kind != "sketch" {
        return Ok((None, None));
    }
    let parse = |raw: Option<&str>| -> Result<Option<NaiveDate>, String> {
        match raw.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(None),
            Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(Some)
                .map_err(|_| "Invalid expected date".into()),
        }
    };
    let mut start = parse(from)?;
    let mut end = parse(to)?;
    if start.is_none() && end.is_none() {
        return Ok((None, None));
    }
    if start.is_none() {
        start = end;
    }
    if end.is_none() {
        end = start;
    }
    if start > end {
        return Err("expectedFrom must be on or before expectedTo".into());
    }
    Ok((start, end))
}

/// Cover first, then the rest. Empty strings dropped. At most IMAGE_URLS_MAX.
pub fn normalize_image_urls(cover: Option<&str>, urls: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let push = |out: &mut Vec<String>, raw: &str| {
        let t = raw.trim();
        if t.is_empty() || out.iter().any(|u| u == t) || out.len() >= IMAGE_URLS_MAX {
            return;
        }
        out.push(t.to_string());
    };
    for u in urls {
        push(&mut out, u);
    }
    if out.is_empty() {
        if let Some(c) = cover {
            push(&mut out, c);
        }
    }
    out
}

pub fn valid_status(status: &str) -> bool {
    LEAF_STATUSES.contains(&status)
}

/// A leaf is on the public blotter when it is published, or scheduled and due.
pub fn leaf_is_live(status: &str, scheduled_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match status {
        "published" => true,
        "scheduled" => scheduled_at.map(|t| t <= now).unwrap_or(false),
        _ => false,
    }
}

/// A world cutting is on the public blotter only when the keeper pinned it
/// and has not set it aside. Unpinned cuttings stay in the private desk.
pub fn cutting_is_live(pinned: bool, dismissed: bool) -> bool {
    pinned && !dismissed
}

pub fn clamp_title(s: &str) -> String {
    clip_chars(s.trim(), TITLE_MAX)
}

pub fn clamp_dek(s: Option<&str>) -> Option<String> {
    let t = s.map(str::trim).filter(|v| !v.is_empty())?;
    Some(clip_chars(t, DEK_MAX))
}

pub fn clamp_body(s: Option<&str>) -> Option<String> {
    let t = s.map(str::trim).filter(|v| !v.is_empty())?;
    Some(clip_chars(t, BODY_MAX))
}

fn clip_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

/// Build a unique slug from an optional override or the English title.
/// `taken` is the set of slugs already used by *other* leaves.
pub fn unique_slug(preferred: Option<&str>, title_en: &str, taken: &[String]) -> String {
    let mut base = preferred
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify)
        .unwrap_or_default();
    if base.is_empty() {
        base = slugify(title_en);
    }
    if base.is_empty() {
        base = "leaf".into();
    }
    // Four-digit slugs are year volumes (`/gazette/2026`). `home`/`room`/`for-work`
    // are public API doors. A leaf must not take those.
    if is_year_slug(&base)
        || matches!(
            base.as_str(),
            "home" | "room" | "blotter" | "for-work" | "watch"
        )
    {
        base = format!("leaf-{base}");
    }
    if !taken.iter().any(|s| s == &base) {
        return base;
    }
    for n in 2..200 {
        let candidate = format!("{base}-{n}");
        if !taken.iter().any(|s| s == &candidate) {
            return candidate;
        }
    }
    format!("{base}-x")
}

pub fn is_year_slug(s: &str) -> bool {
    s.len() == 4 && s.bytes().all(|b| b.is_ascii_digit())
}

/// RSS 2.0 `<item>` and Atom `<entry>`. Titles/summaries are plain text;
/// HTML is stripped so a cutting can sit on the blotter as a newspaper clip.
pub fn parse_feed(xml: &str) -> Vec<ParsedFeedItem> {
    let mut items = parse_rss_items(xml);
    if items.is_empty() {
        items = parse_atom_entries(xml);
    }
    items
        .into_iter()
        .filter(|i| !i.title.is_empty() && !i.url.is_empty())
        .take(40)
        .collect()
}

fn parse_rss_items(xml: &str) -> Vec<ParsedFeedItem> {
    blocks_between(xml, "<item", "</item>")
        .into_iter()
        .filter_map(|block| {
            let title = tag_text(block, "title")?;
            let url = tag_text(block, "link")
                .or_else(|| tag_text(block, "guid"))
                .unwrap_or_default();
            let guid = tag_text(block, "guid").unwrap_or_else(|| url.clone());
            let raw_summary = tag_text(block, "description")
                .or_else(|| tag_text(block, "content:encoded"))
                .unwrap_or_default();
            Some(ParsedFeedItem {
                guid: guid.trim().to_string(),
                title: collapse_ws(&strip_tags(&title)),
                url: url.trim().to_string(),
                summary: excerpt(&raw_summary, EXCERPT_MAX),
                published_at: tag_text(block, "pubDate")
                    .or_else(|| tag_text(block, "dc:date"))
                    .and_then(|s| parse_date(&s)),
            })
        })
        .collect()
}

fn parse_atom_entries(xml: &str) -> Vec<ParsedFeedItem> {
    blocks_between(xml, "<entry", "</entry>")
        .into_iter()
        .filter_map(|block| {
            let title = tag_text(block, "title")?;
            let url = atom_link(block)
                .or_else(|| tag_text(block, "id"))
                .unwrap_or_default();
            let guid = tag_text(block, "id").unwrap_or_else(|| url.clone());
            let raw_summary = tag_text(block, "summary")
                .or_else(|| tag_text(block, "content"))
                .unwrap_or_default();
            Some(ParsedFeedItem {
                guid: guid.trim().to_string(),
                title: collapse_ws(&strip_tags(&title)),
                url: url.trim().to_string(),
                summary: excerpt(&raw_summary, EXCERPT_MAX),
                published_at: tag_text(block, "updated")
                    .or_else(|| tag_text(block, "published"))
                    .and_then(|s| parse_date(&s)),
            })
        })
        .collect()
}

fn atom_link(block: &str) -> Option<String> {
    // Prefer rel="alternate", else the first href.
    let mut first = None;
    let lower = block.to_ascii_lowercase();
    let mut search_from = 0;
    while let Some(rel) = lower[search_from..].find("<link") {
        let start = search_from + rel;
        let end = block[start..].find('>').map(|i| start + i + 1)?;
        let tag = &block[start..end];
        let href = attr(tag, "href")?;
        let rel_val = attr(tag, "rel").unwrap_or_default();
        if rel_val.is_empty() || rel_val == "alternate" {
            return Some(href);
        }
        if first.is_none() {
            first = Some(href);
        }
        search_from = end;
    }
    first
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=");
    let lower = tag.to_ascii_lowercase();
    let i = lower.find(&needle)?;
    let rest = tag[i + needle.len()..].trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = rest.get(1..)?;
    let end = inner.find(quote)?;
    Some(decode_entities(&inner[..end]).trim().to_string())
}

fn blocks_between<'a>(xml: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let lower = xml.to_ascii_lowercase();
    let open_l = open.to_ascii_lowercase();
    let close_l = close.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = lower[from..].find(&open_l) {
        let start = from + i;
        let after_open = start + open.len();
        let Some(gt) = xml[after_open..].find('>') else {
            break;
        };
        let content_start = after_open + gt + 1;
        let Some(j) = lower[content_start..].find(&close_l) else {
            break;
        };
        out.push(&xml[content_start..content_start + j]);
        from = content_start + j + close.len();
    }
    out
}

fn tag_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let lower = block.to_ascii_lowercase();
    let open_l = open.to_ascii_lowercase();
    let close_l = close.to_ascii_lowercase();
    let i = lower.find(&open_l)?;
    let after = i + open.len();
    let gt = block[after..].find('>')?;
    let content_start = after + gt + 1;
    let j = lower[content_start..].find(&close_l)?;
    let raw = &block[content_start..content_start + j];
    let text = decode_entities(&strip_cdata(raw)).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

fn strip_cdata(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("<![CDATA[") {
        out.push_str(&rest[..i]);
        rest = &rest[i + 9..];
        if let Some(j) = rest.find("]]>") {
            out.push_str(&rest[..j]);
            rest = &rest[j + 3..];
        } else {
            out.push_str(rest);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    decode_numeric(
        &s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
            .replace("&rsquo;", "\u{2019}")
            .replace("&lsquo;", "\u{2018}")
            .replace("&rdquo;", "\u{201D}")
            .replace("&ldquo;", "\u{201C}")
            .replace("&mdash;", "\u{2014}")
            .replace("&ndash;", "\u{2013}")
            .replace("&hellip;", "\u{2026}"),
    )
}

fn decode_numeric(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' && i + 2 < bytes.len() && bytes[i + 1] == b'#' {
            let hex = bytes[i + 2] == b'x' || bytes[i + 2] == b'X';
            let digits_start = if hex { i + 3 } else { i + 2 };
            let mut j = digits_start;
            while j < bytes.len() {
                let c = bytes[j];
                let ok = if hex {
                    c.is_ascii_hexdigit()
                } else {
                    c.is_ascii_digit()
                };
                if !ok {
                    break;
                }
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b';' && j > digits_start {
                let num = &s[digits_start..j];
                let parsed = if hex {
                    u32::from_str_radix(num, 16)
                } else {
                    num.parse::<u32>()
                };
                if let Ok(code) = parsed {
                    if let Some(ch) = char::from_u32(code) {
                        out.push(ch);
                        i = j + 1;
                        continue;
                    }
                }
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

pub fn excerpt(s: &str, max: usize) -> String {
    let t = collapse_ws(&strip_tags(s));
    if t.chars().count() <= max {
        return t;
    }
    let mut out: String = t.chars().take(max.saturating_sub(1)).collect();
    if let Some(i) = out.rfind(' ') {
        out.truncate(i);
    }
    out.push('…');
    out
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    DateTime::parse_from_rfc2822(s)
        .or_else(|_| DateTime::parse_from_rfc3339(s))
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn live_when_published_or_due() {
        let now = Utc.with_ymd_and_hms(2026, 8, 17, 12, 0, 0).unwrap();
        assert!(leaf_is_live("published", None, now));
        assert!(!leaf_is_live("draft", None, now));
        assert!(!leaf_is_live("archived", None, now));
        let due = Utc.with_ymd_and_hms(2026, 8, 17, 11, 0, 0).unwrap();
        let later = Utc.with_ymd_and_hms(2026, 8, 17, 13, 0, 0).unwrap();
        assert!(leaf_is_live("scheduled", Some(due), now));
        assert!(!leaf_is_live("scheduled", Some(later), now));
        assert!(!leaf_is_live("scheduled", None, now));
    }

    #[test]
    fn cutting_live_only_when_pinned() {
        assert!(cutting_is_live(true, false));
        assert!(!cutting_is_live(false, false));
        assert!(!cutting_is_live(true, true));
        assert!(!cutting_is_live(false, true));
    }

    #[test]
    fn sketch_is_a_kind() {
        assert!(valid_kind("sketch"));
        assert!(!valid_kind("news"));
    }

    #[test]
    fn image_urls_cover_and_cap() {
        let many: Vec<String> = (0..12).map(|i| format!("/img/{i}.jpg")).collect();
        let out = normalize_image_urls(Some("/cover.jpg"), &many);
        assert_eq!(out.len(), IMAGE_URLS_MAX);
        assert_eq!(out[0], "/img/0.jpg");
        let from_cover = normalize_image_urls(Some(" /face.jpg "), &[]);
        assert_eq!(from_cover, vec!["/face.jpg".to_string()]);
        let empty = normalize_image_urls(Some("  "), &[" ".into()]);
        assert!(empty.is_empty());
    }

    #[test]
    fn expected_dates_day_range_and_clear() {
        use chrono::NaiveDate;
        let day = NaiveDate::from_ymd_opt(2026, 3, 12).unwrap();
        assert_eq!(
            normalize_expected("sketch", Some("2026-03-12"), None).unwrap(),
            (Some(day), Some(day))
        );
        let end = NaiveDate::from_ymd_opt(2026, 5, 4).unwrap();
        assert_eq!(
            normalize_expected("sketch", Some("2026-03-12"), Some("2026-05-04")).unwrap(),
            (Some(day), Some(end))
        );
        assert!(normalize_expected("sketch", Some("2026-05-04"), Some("2026-03-12")).is_err());
        assert_eq!(
            normalize_expected("arrival", Some("2026-03-12"), Some("2026-05-04")).unwrap(),
            (None, None)
        );
        assert_eq!(
            normalize_expected("sketch", None, None).unwrap(),
            (None, None)
        );
    }

    #[test]
    fn slug_avoids_taken() {
        let taken = vec!["laid-out-today".into(), "laid-out-today-2".into()];
        assert_eq!(
            unique_slug(None, "Laid out today", &taken),
            "laid-out-today-3"
        );
        assert_eq!(unique_slug(Some("custom"), "X", &[]), "custom");
        assert_eq!(unique_slug(Some("2026"), "X", &[]), "leaf-2026");
        assert_eq!(unique_slug(Some("watch"), "X", &[]), "leaf-watch");
        assert_eq!(unique_slug(Some("room"), "X", &[]), "leaf-room");
        assert!(is_year_slug("2026"));
        assert!(!is_year_slug("leaf-2026"));
    }

    #[test]
    fn parses_rss_item() {
        let xml = r#"<?xml version="1.0"?>
        <rss><channel>
          <item>
            <title>A bronze bird</title>
            <link>https://example.com/bird</link>
            <guid>https://example.com/bird</guid>
            <description><![CDATA[<p>Quiet wings in a vitrine.</p>]]></description>
            <pubDate>Mon, 17 Aug 2026 10:00:00 GMT</pubDate>
          </item>
        </channel></rss>"#;
        let items = parse_feed(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "A bronze bird");
        assert_eq!(items[0].url, "https://example.com/bird");
        assert_eq!(items[0].summary, "Quiet wings in a vitrine.");
        assert!(items[0].published_at.is_some());
    }

    #[test]
    fn parses_atom_entry() {
        let xml = r#"<?xml version="1.0"?>
        <feed xmlns="http://www.w3.org/2005/Atom">
          <entry>
            <id>urn:ex:1</id>
            <title>Clay dust</title>
            <link rel="alternate" href="https://example.com/clay"/>
            <summary>Hands at the bench.</summary>
            <updated>2026-08-17T10:00:00Z</updated>
          </entry>
        </feed>"#;
        let items = parse_feed(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Clay dust");
        assert_eq!(items[0].url, "https://example.com/clay");
        assert_eq!(items[0].guid, "urn:ex:1");
    }

    #[test]
    fn excerpt_strips_and_clips() {
        let long = "word ".repeat(80);
        let e = excerpt(&format!("<p>{long}</p>"), 40);
        assert!(e.ends_with('…'));
        assert!(e.chars().count() <= 40);
        assert!(!e.contains('<'));
    }

    #[test]
    fn decodes_numeric_title_entities() {
        let xml = r#"<?xml version="1.0"?>
        <rss><channel>
          <item>
            <title>What&#8217;s in the cabinet</title>
            <link>https://example.com/apos</link>
            <guid>https://example.com/apos</guid>
          </item>
        </channel></rss>"#;
        let items = parse_feed(xml);
        assert_eq!(items[0].title, "What’s in the cabinet");
    }

    #[test]
    fn guesses_house_stamps() {
        assert_eq!(
            guess_mark_key("Colossal", "https://www.thisiscolossal.com/feed/"),
            "pillar"
        );
        assert_eq!(
            guess_mark_key("Hyperallergic", "https://hyperallergic.com/feed/"),
            "hive"
        );
        assert_eq!(
            guess_mark_key("Designboom", "https://www.designboom.com/feed/"),
            "boom"
        );
        assert_eq!(
            guess_mark_key("A quiet journal", "https://example.com/rss"),
            "letter"
        );
    }
}
