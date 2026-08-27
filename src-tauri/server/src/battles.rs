//! Скромные эпические битвы — pure rules for the shelf of cards.
//!
//! No database here. The service layer uses these to clamp what the keeper
//! types, to keep a card's rank and its owner's level from drifting out of
//! range, and to hand the room a set of frames even before the keeper has
//! opened the design panel once.
//!
//! Two ranges that look alike and mean different things:
//!   * `tier`  — the card's rank, 1..5. A property of the card.
//!   * `level` — the state of one person's copy, 1..5. A property of owning it.

use crate::slug::slugify;
use serde::{Deserialize, Serialize};

pub const CARD_STATUSES: &[&str] = &["draft", "published", "retired"];

pub const TIER_MIN: i16 = 1;
pub const TIER_MAX: i16 = 5;
pub const LEVEL_MIN: i16 = 1;
pub const LEVEL_MAX: i16 = 5;

pub const TITLE_MAX: usize = 80;
/// Room for a list of named abilities, not just one line: a dressed frame with
/// a parchment panel is written to be read, and 160 characters could not hold
/// even three of them. What will not fit on the shelf is clamped by the
/// renderer, not refused by the table.
pub const EFFECT_MAX: usize = 400;
pub const LORE_MAX: usize = 400;

pub const COST_MAX: i16 = 20;
pub const POWER_MAX: i16 = 99;
pub const STAT_MAX: i16 = 99;

/// A card is read at a glance. Past about this many named properties it stops
/// being a card and becomes a page, and the properties band cannot hold them.
pub const TRAITS_MAX: usize = 8;
pub const TRAIT_NAME_MAX: usize = 60;
pub const TRAIT_TEXT_MAX: usize = 200;
pub const RACE_NAME_MAX: usize = 60;
pub const TYPE_MAX: usize = 40;

/// The shelf is handed out whole. It is a shelf, not a feed — the same reason
/// the tales room refuses pagination.
pub const SHELF_CARDS: i64 = 500;

/// Doors of the battles API. A card must not take one for a slug.
const RESERVED_SLUGS: &[&str] = &["cards", "frames", "me", "buy", "wallet", "new"];

pub fn valid_status(status: &str) -> bool {
    CARD_STATUSES.contains(&status)
}

pub fn clamp_tier(tier: i16) -> i16 {
    tier.clamp(TIER_MIN, TIER_MAX)
}

pub fn clamp_level(level: i16) -> i16 {
    level.clamp(LEVEL_MIN, LEVEL_MAX)
}

pub fn clamp_cost(cost: i16) -> i16 {
    cost.clamp(0, COST_MAX)
}

pub fn clamp_power(power: i16) -> i16 {
    power.clamp(0, POWER_MAX)
}

pub fn clamp_stat(stat: i16) -> i16 {
    stat.clamp(0, STAT_MAX)
}

fn clamp_text(raw: Option<&str>, max: usize) -> Option<String> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(max).collect())
}

pub fn clamp_title(raw: &str) -> String {
    raw.trim().chars().take(TITLE_MAX).collect()
}

pub fn clamp_effect(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, EFFECT_MAX)
}

pub fn clamp_lore(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, LORE_MAX)
}

pub fn clamp_type(raw: Option<&str>) -> Option<String> {
    clamp_text(raw, TYPE_MAX)
}

/// One named property of a card: "Вихрь Души (Wind of Soul): каждое третье
/// заклинание создаёт копию эффекта." Both names are kept because the card
/// shows them together, the way the keeper's own drawing does.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardTrait {
    #[serde(default)]
    pub name_en: String,
    #[serde(default)]
    pub name_ru: String,
    #[serde(default)]
    pub text_en: String,
    #[serde(default)]
    pub text_ru: String,
}

/// The properties of a card, as they will be stored.
///
/// A property with no name at all is dropped rather than written: an empty row
/// left behind by the editor is not something the keeper meant to say. Text is
/// cut to fit instead of refused, as everywhere else on this card.
pub fn normalize_traits(traits: &[CardTrait]) -> Option<String> {
    let cleaned: Vec<CardTrait> = traits
        .iter()
        .map(|t| CardTrait {
            name_en: t.name_en.trim().chars().take(TRAIT_NAME_MAX).collect(),
            name_ru: t.name_ru.trim().chars().take(TRAIT_NAME_MAX).collect(),
            text_en: t.text_en.trim().chars().take(TRAIT_TEXT_MAX).collect(),
            text_ru: t.text_ru.trim().chars().take(TRAIT_TEXT_MAX).collect(),
        })
        .filter(|t| !t.name_en.is_empty() || !t.name_ru.is_empty())
        .take(TRAITS_MAX)
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    serde_json::to_string(&cleaned).ok()
}

/// Read back what was stored. A column that will not parse yields no properties
/// rather than failing the whole card — the rest of it is still worth showing.
pub fn read_traits(raw: Option<&str>) -> Vec<CardTrait> {
    raw.and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default()
}

/// A price of `None` means "not to be had for this coin", which is not zero.
/// Negative is nonsense either way, so it becomes "not for this coin" too.
pub fn clamp_price(raw: Option<i32>) -> Option<i32> {
    raw.filter(|p| *p >= 0)
}

pub fn unique_slug(preferred: Option<&str>, title: &str, taken: &[String]) -> String {
    let mut base = preferred
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(slugify)
        .unwrap_or_default();
    if base.is_empty() {
        base = slugify(title);
    }
    if base.is_empty() {
        base = "card".into();
    }
    if RESERVED_SLUGS.contains(&base.as_str()) {
        base = format!("card-{base}");
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
    format!("{base}-{}", uuid::Uuid::new_v4().simple())
}

/// How the photograph sits inside the frame: a point to centre on and how close
/// to stand. Stored as JSON text, the same shape the figurine keyhole uses.
/// Anything unparseable becomes `None` — a card with a broken focus is centred,
/// never blank.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ArtFocal {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
}

pub fn normalize_focal(raw: Option<&str>) -> Option<String> {
    let parsed: ArtFocal = serde_json::from_str(raw?.trim()).ok()?;
    let focal = ArtFocal {
        x: parsed.x.clamp(0.0, 1.0),
        y: parsed.y.clamp(0.0, 1.0),
        zoom: parsed.zoom.clamp(1.0, 3.0),
    };
    serde_json::to_string(&focal).ok()
}

/// A single card's exception to the tier's shared frame — "wear a picture of
/// your own" without touching the frame every other card of that rank still
/// wears. Every field optional and `None` when absent, unlike `BattleFrame`
/// itself: this is a patch, not a whole dressing, and a field left out here
/// means "keep the tier's own", not "keep it empty".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<f32>,
}

/// `None` for "no override at all" — a broken or empty patch is the same as
/// none, never a patch that blanks the tier's own picture.
pub fn normalize_frame_override(raw: Option<&str>) -> Option<String> {
    let parsed: FrameOverride = serde_json::from_str(raw?.trim()).ok()?;
    let frame_image = parsed
        .frame_image
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let frame_mode = parsed
        .frame_mode
        .filter(|m| valid_frame_mode(m));
    let aspect = parsed.aspect.map(|a| a.clamp(0.3, 2.0));
    if frame_image.is_none() && frame_mode.is_none() && aspect.is_none() {
        return None;
    }
    serde_json::to_string(&FrameOverride {
        frame_image,
        frame_mode,
        aspect,
    })
    .ok()
}

/// How a rank is dressed. Frames are the design, cards are the data — a rule
/// every card-authoring tool converges on, and the reason the keeper edits five
/// frames instead of forty cards.
///
/// Two ways to dress a card, and a frame may use either. PAINTED: paper, ink,
/// border and foil, drawn by the renderer. DRESSED: a photograph of a real
/// frame laid under the whole card, with the card's content set inside it.
///
/// The photograph goes BEHIND everything rather than over it. An overlay would
/// need a transparent window, and this house re-encodes every uploaded image to
/// JPEG, which has no alpha — an overlay frame would arrive with its window
/// filled in. Laid behind, an ordinary photograph works.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFrame {
    pub tier: i16,
    pub name_en: String,
    pub name_ru: String,
    /// The card's paper. Still the ground under a photograph that fails to load.
    pub paper: String,
    /// Ink for title and stats.
    pub ink: String,
    pub border: String,
    /// Colour of the single slow sweep across the face. Empty = no foil at all,
    /// which is what a humble card should be.
    #[serde(default)]
    pub foil: String,
    /// A picture of a frame. Empty = painted.
    #[serde(default)]
    pub frame_image: String,
    /// `overlay` — the picture lies ON TOP of the card and the card shows through
    ///             the hole in it. What a cut-out frame with transparency wants:
    ///             the carving's inner edge overlaps the photograph, and a window
    ///             set a little wrong is hidden rather than exposed.
    /// `behind`  — the picture is the card's ground. For a frame with no hole in
    ///             it, where an overlay would simply cover everything.
    #[serde(default)]
    pub frame_mode: String,
    /// A texture for the card's ground — the paper the content is written on.
    /// A cut-out frame has nothing behind it but this. Empty = flat `paper`.
    #[serde(default)]
    pub paper_image: String,
    /// Where the card's content sits inside that photograph, as a percentage of
    /// the card on each side. A carved frame has thick sides; this is how the
    /// keeper says where the window actually is.
    #[serde(default)]
    pub inset_top: f32,
    #[serde(default)]
    pub inset_right: f32,
    #[serde(default)]
    pub inset_bottom: f32,
    #[serde(default)]
    pub inset_left: f32,
    /// Card width ÷ height. A dressed card is often squarer than a bare one.
    #[serde(default)]
    pub aspect: f32,
    /// The card is four bands: header, photograph, properties, footer. Three of
    /// them are given a share of the content's height; the properties band takes
    /// whatever is left, because it is the one that has to hold a paragraph.
    ///
    /// `Option` rather than a bare number, unlike the photograph's share: zero is
    /// a real choice here — a bare card wants no header and no footer — so
    /// "absent" and "set to nothing" cannot be the same value. Absent means a
    /// frame saved before the card had bands, and it takes the default;
    /// `Some(0.0)` means the keeper dragged the slider to the floor.
    /// Always `Some` after `normalize_frames`, so the client only ever sees a
    /// number.
    #[serde(default)]
    pub header_share: Option<f32>,
    /// How much of the content the work's photograph takes, 0..1. Zero is not a
    /// choice here — a card with no picture band has nowhere to look — so zero
    /// keeps meaning "not set".
    #[serde(default)]
    pub art_share: f32,
    #[serde(default)]
    pub foot_share: Option<f32>,
    /// Face for the name, by id from the site's own font list. Empty = the
    /// card's ordinary serif.
    #[serde(default)]
    pub title_font: String,
    /// Ink for the name alone, when the frame wants it apart. Empty = `ink`.
    #[serde(default)]
    pub title_ink: String,
    /// `corners` — cost and power ride the corners, as on a card held in a hand.
    /// `plaque`  — the name stands on a plate under the picture and the stats
    ///             are read as a line, as on a card standing in a case.
    #[serde(default)]
    pub layout: String,
}

pub const LAYOUTS: &[&str] = &["corners", "plaque"];
pub const FRAME_MODES: &[&str] = &["overlay", "behind"];

pub fn valid_layout(layout: &str) -> bool {
    LAYOUTS.contains(&layout)
}

pub fn valid_frame_mode(mode: &str) -> bool {
    FRAME_MODES.contains(&mode)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFrames {
    pub frames: Vec<BattleFrame>,
}

impl Default for BattleFrames {
    fn default() -> Self {
        Self {
            frames: default_frames(),
        }
    }
}

/// A card that is not dressed in a photograph. 5 : 7, the ratio of a card held
/// in a hand.
pub const DEFAULT_ASPECT: f32 = 5.0 / 7.0;
pub const DEFAULT_ART_SHARE: f32 = 0.44;
pub const DEFAULT_HEADER_SHARE: f32 = 0.09;
pub const DEFAULT_FOOT_SHARE: f32 = 0.10;

fn painted(
    tier: i16,
    name_en: &str,
    name_ru: &str,
    paper: &str,
    ink: &str,
    border: &str,
    foil: &str,
) -> BattleFrame {
    BattleFrame {
        tier,
        name_en: name_en.into(),
        name_ru: name_ru.into(),
        paper: paper.into(),
        ink: ink.into(),
        border: border.into(),
        foil: foil.into(),
        frame_image: String::new(),
        frame_mode: "overlay".into(),
        paper_image: String::new(),
        inset_top: 0.0,
        inset_right: 0.0,
        inset_bottom: 0.0,
        inset_left: 0.0,
        aspect: DEFAULT_ASPECT,
        header_share: Some(DEFAULT_HEADER_SHARE),
        art_share: DEFAULT_ART_SHARE,
        foot_share: Some(DEFAULT_FOOT_SHARE),
        title_font: String::new(),
        title_ink: String::new(),
        layout: "corners".into(),
    }
}

/// The house palette, five steps deeper. Nothing here glows: rank shows as
/// darker paper and a heavier border, not as a brighter colour.
pub fn default_frames() -> Vec<BattleFrame> {
    vec![
        painted(1, "Humble", "Скромная", "#f8f1e7", "#34251c", "#d8c6b1", ""),
        painted(2, "Sturdy", "Крепкая", "#f3e9db", "#34251c", "#c3ad93", ""),
        painted(3, "Remembered", "Памятная", "#eeddc8", "#34251c", "#a8845f", "rgba(198,95,60,0.16)"),
        painted(4, "Rare", "Редкая", "#e6cfb2", "#2a1a11", "#6f3b24", "rgba(198,95,60,0.28)"),
        painted(5, "Epic", "Эпическая", "#3a2a1e", "#f3e4cd", "#c99a52", "rgba(214,178,110,0.42)"),
    ]
}

/// Keep exactly five frames, one per rank, whatever the keeper saved. A missing
/// rank falls back to its default rather than leaving a card undressed, and a
/// number that would fold the card in half is pulled back into range instead of
/// being refused — the keeper is dragging sliders, not filling in a form.
///
/// Zero is read as "not set" for the two ratios, so frames saved before a card
/// could wear a photograph keep working without a data migration.
pub fn normalize_frames(mut saved: Vec<BattleFrame>) -> Vec<BattleFrame> {
    default_frames()
        .into_iter()
        .map(|fallback| {
            match saved
                .iter()
                .position(|f| clamp_tier(f.tier) == fallback.tier)
            {
                Some(at) => {
                    let mut found = saved.remove(at);
                    found.tier = fallback.tier;
                    if found.name_en.trim().is_empty() {
                        found.name_en = fallback.name_en;
                    }
                    if found.name_ru.trim().is_empty() {
                        found.name_ru = fallback.name_ru;
                    }
                    if found.paper.trim().is_empty() {
                        found.paper = fallback.paper;
                    }
                    if found.ink.trim().is_empty() {
                        found.ink = fallback.ink;
                    }
                    if found.border.trim().is_empty() {
                        found.border = fallback.border;
                    }
                    if !valid_layout(&found.layout) {
                        found.layout = fallback.layout;
                    }
                    if !valid_frame_mode(&found.frame_mode) {
                        // A frame saved before cards could wear a cut-out has no
                        // mode, and back then a picture was always the card's
                        // ground. Defaulting it to `overlay` would lay that solid
                        // picture over the card and hide everything on it — so an
                        // unset mode on a frame that already HAS a picture means
                        // `behind`, whatever the default for a fresh frame is.
                        found.frame_mode = if found.frame_image.trim().is_empty() {
                            fallback.frame_mode
                        } else {
                            "behind".into()
                        };
                    }
                    found.frame_image = found.frame_image.trim().to_string();
                    found.paper_image = found.paper_image.trim().to_string();
                    found.title_font = found.title_font.trim().to_string();
                    found.title_ink = found.title_ink.trim().to_string();
                    found.aspect = if found.aspect > 0.0 {
                        found.aspect.clamp(0.45, 1.4)
                    } else {
                        fallback.aspect
                    };
                    found.art_share = if found.art_share > 0.0 {
                        found.art_share.clamp(0.12, 0.85)
                    } else {
                        fallback.art_share
                    };
                    let mut header = clamp_band(found.header_share, fallback.header_share);
                    let mut foot = clamp_band(found.foot_share, fallback.foot_share);
                    // Capped together with the photograph so the properties band
                    // always keeps room; scaled in proportion rather than
                    // truncated, so the keeper's balance between them survives.
                    let taken = header + found.art_share + foot;
                    if taken > 0.9 {
                        let scale = 0.9 / taken;
                        header *= scale;
                        foot *= scale;
                        found.art_share *= scale;
                    }
                    found.header_share = Some(header);
                    found.foot_share = Some(foot);
                    let (top, bottom) = clamp_pair(found.inset_top, found.inset_bottom);
                    let (left, right) = clamp_pair(found.inset_left, found.inset_right);
                    found.inset_top = top;
                    found.inset_bottom = bottom;
                    found.inset_left = left;
                    found.inset_right = right;
                    found
                }
                None => fallback,
            }
        })
        .collect()
}

/// One band's share of the card. Absent takes the default; present is honoured,
/// including zero. A number that is not a number falls back rather than poisoning
/// the layout with NaN.
fn clamp_band(given: Option<f32>, fallback: Option<f32>) -> f32 {
    let fallback = fallback.unwrap_or(0.0);
    match given {
        Some(v) if v.is_finite() => v.clamp(0.0, 0.3),
        Some(_) => fallback,
        None => fallback,
    }
}

/// Two insets facing each other. Each is capped on its own, and together they
/// are never allowed to close the window they are supposed to open.
fn clamp_pair(a: f32, b: f32) -> (f32, f32) {
    const MOST: f32 = 45.0;
    const TOGETHER: f32 = 85.0;
    let a = if a.is_finite() { a.clamp(0.0, MOST) } else { 0.0 };
    let b = if b.is_finite() { b.clamp(0.0, MOST) } else { 0.0 };
    if a + b <= TOGETHER {
        return (a, b);
    }
    let scale = TOGETHER / (a + b);
    (a * scale, b * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_avoids_api_doors() {
        assert_eq!(unique_slug(Some("cards"), "anything", &[]), "card-cards");
    }

    #[test]
    fn slug_walks_past_taken() {
        let taken = vec!["raven".to_string(), "raven-2".to_string()];
        assert_eq!(unique_slug(None, "Raven", &taken), "raven-3");
    }

    #[test]
    fn effect_is_cut_not_refused() {
        let long = "ы".repeat(400);
        assert_eq!(clamp_effect(Some(&long)).unwrap().chars().count(), EFFECT_MAX);
    }

    #[test]
    fn blank_text_becomes_none() {
        assert!(clamp_effect(Some("   ")).is_none());
    }

    #[test]
    fn missing_price_is_not_free() {
        assert_eq!(clamp_price(Some(-5)), None);
        assert_eq!(clamp_price(Some(0)), Some(0));
    }

    #[test]
    fn focal_is_clamped_and_broken_focus_is_dropped() {
        let out = normalize_focal(Some(r#"{"x":2.0,"y":-1.0,"zoom":9.0}"#)).unwrap();
        let back: ArtFocal = serde_json::from_str(&out).unwrap();
        assert_eq!((back.x, back.y, back.zoom), (1.0, 0.0, 3.0));
        assert!(normalize_focal(Some("not json")).is_none());
    }

    #[test]
    fn frames_always_come_back_five_deep() {
        let mut mine = painted(3, "", "Своя", "#000000", "", "#111111", "");
        mine.ink = String::new();
        let frames = normalize_frames(vec![mine]);
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[2].paper, "#000000");
        assert_eq!(frames[2].name_ru, "Своя");
        // Blanks fall back rather than shipping an undressed rank.
        assert_eq!(frames[2].name_en, "Remembered");
        assert!(!frames[2].ink.is_empty());
    }

    #[test]
    fn a_frame_saved_before_photographs_keeps_working() {
        // Exactly what an older `battle_frames` setting deserialises to: every
        // new field at its zero value. Zero ratios must read as "not set".
        let old = serde_json::json!({
            "tier": 2, "nameEn": "Sturdy", "nameRu": "Крепкая",
            "paper": "#f3e9db", "ink": "#34251c", "border": "#c3ad93", "foil": ""
        });
        let frame: BattleFrame = serde_json::from_value(old).unwrap();
        let frames = normalize_frames(vec![frame]);
        assert_eq!(frames[1].aspect, DEFAULT_ASPECT);
        assert_eq!(frames[1].art_share, DEFAULT_ART_SHARE);
        // Absent, so the default — a frame written before cards had bands must
        // not come back with no header at all.
        assert_eq!(frames[1].header_share, Some(DEFAULT_HEADER_SHARE));
        assert_eq!(frames[1].foot_share, Some(DEFAULT_FOOT_SHARE));
        assert_eq!(frames[1].layout, "corners");
        // An empty mode is not a mode; a frame must know how it is worn.
        assert_eq!(frames[1].frame_mode, "overlay");
    }

    #[test]
    fn the_properties_band_always_keeps_room() {
        let mut mine = painted(2, "a", "а", "#fff", "#000", "#ccc", "");
        mine.header_share = Some(0.3);
        mine.art_share = 0.85;
        mine.foot_share = Some(0.3);
        let frames = normalize_frames(vec![mine]);
        let f = &frames[1];
        let taken = f.header_share.unwrap() + f.art_share + f.foot_share.unwrap();
        assert!(taken <= 0.9001, "three bands took {taken}");
        // Scaled in proportion, not truncated: the keeper's balance survives.
        assert!(f.art_share > f.header_share.unwrap());
    }

    #[test]
    fn a_header_may_be_nothing_at_all() {
        let mut mine = painted(1, "a", "а", "#fff", "#000", "#ccc", "");
        mine.header_share = Some(0.0);
        mine.foot_share = Some(0.0);
        let frames = normalize_frames(vec![mine]);
        // Zero is a choice here, unlike the photograph's share, where it means
        // "not set" — a card with no picture band would have nowhere to look.
        assert_eq!(frames[0].header_share, Some(0.0));
        assert_eq!(frames[0].foot_share, Some(0.0));
        assert_eq!(frames[0].art_share, DEFAULT_ART_SHARE);
    }

    #[test]
    fn traits_drop_the_nameless_and_cut_the_long() {
        let long = "я".repeat(400);
        let traits = vec![
            CardTrait { name_en: "Wind".into(), name_ru: "Вихрь".into(), text_en: long.clone(), text_ru: "х".into() },
            CardTrait { name_en: "  ".into(), name_ru: "".into(), text_en: "orphan".into(), text_ru: "".into() },
        ];
        let stored = normalize_traits(&traits).unwrap();
        let back = read_traits(Some(&stored));
        assert_eq!(back.len(), 1, "a nameless property is not a property");
        assert_eq!(back[0].text_en.chars().count(), TRAIT_TEXT_MAX);
        assert!(read_traits(Some("not json")).is_empty());
        assert!(normalize_traits(&[]).is_none());
    }

    #[test]
    fn an_old_picture_frame_is_still_worn_behind() {
        // Saved when a picture could only be the card's ground. Read as an
        // overlay it would cover the whole card, so it must stay behind.
        let old = serde_json::json!({
            "tier": 4, "nameEn": "Rare", "nameRu": "Редкая",
            "paper": "#e6cfb2", "ink": "#2a1a11", "border": "#6f3b24", "foil": "",
            "frameImage": "/static/images/preview/carved.jpg",
            "insetTop": 8.0, "insetRight": 9.0, "insetBottom": 8.0, "insetLeft": 9.0,
            "aspect": 0.8, "artShare": 0.5, "layout": "plaque"
        });
        let frame: BattleFrame = serde_json::from_value(old).unwrap();
        let frames = normalize_frames(vec![frame]);
        assert_eq!(frames[3].frame_mode, "behind");
        // A fresh frame with no picture keeps the modern default.
        assert_eq!(frames[0].frame_mode, "overlay");
    }

    #[test]
    fn insets_can_never_close_the_window() {
        let mut mine = painted(1, "a", "а", "#fff", "#000", "#ccc", "");
        mine.inset_top = 90.0;
        mine.inset_bottom = 90.0;
        mine.aspect = 99.0;
        mine.art_share = -3.0;
        mine.layout = "nonsense".into();
        let frames = normalize_frames(vec![mine]);
        assert!(frames[0].inset_top + frames[0].inset_bottom <= 85.0);
        assert!(frames[0].inset_top > 0.0);
        assert_eq!(frames[0].aspect, 1.4);
        // A negative share is not "set", so it falls back rather than clamping.
        assert_eq!(frames[0].art_share, DEFAULT_ART_SHARE);
        assert_eq!(frames[0].layout, "corners");
    }
}
