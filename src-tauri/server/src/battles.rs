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
/// An effect longer than this breaks the frame before it breaks the balance.
pub const EFFECT_MAX: usize = 160;
pub const LORE_MAX: usize = 400;

pub const COST_MAX: i16 = 20;
pub const POWER_MAX: i16 = 99;

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

/// One rank's dress. Frames are the design, cards are the data — a rule every
/// card-authoring tool converges on, and the reason the keeper edits five
/// frames instead of forty cards.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BattleFrame {
    pub tier: i16,
    pub name_en: String,
    pub name_ru: String,
    /// The card's paper.
    pub paper: String,
    /// Ink for title and stats.
    pub ink: String,
    pub border: String,
    /// Colour of the single slow sweep across the face. Empty = no foil at all,
    /// which is what a humble card should be.
    #[serde(default)]
    pub foil: String,
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

/// The house palette, five steps deeper. Nothing here glows: rank shows as
/// darker paper and a heavier border, not as a brighter colour.
pub fn default_frames() -> Vec<BattleFrame> {
    vec![
        BattleFrame {
            tier: 1,
            name_en: "Humble".into(),
            name_ru: "Скромная".into(),
            paper: "#f8f1e7".into(),
            ink: "#34251c".into(),
            border: "#d8c6b1".into(),
            foil: String::new(),
        },
        BattleFrame {
            tier: 2,
            name_en: "Sturdy".into(),
            name_ru: "Крепкая".into(),
            paper: "#f3e9db".into(),
            ink: "#34251c".into(),
            border: "#c3ad93".into(),
            foil: String::new(),
        },
        BattleFrame {
            tier: 3,
            name_en: "Remembered".into(),
            name_ru: "Памятная".into(),
            paper: "#eeddc8".into(),
            ink: "#34251c".into(),
            border: "#a8845f".into(),
            foil: "rgba(198,95,60,0.16)".into(),
        },
        BattleFrame {
            tier: 4,
            name_en: "Rare".into(),
            name_ru: "Редкая".into(),
            paper: "#e6cfb2".into(),
            ink: "#2a1a11".into(),
            border: "#6f3b24".into(),
            foil: "rgba(198,95,60,0.28)".into(),
        },
        BattleFrame {
            tier: 5,
            name_en: "Epic".into(),
            name_ru: "Эпическая".into(),
            paper: "#3a2a1e".into(),
            ink: "#f3e4cd".into(),
            border: "#c99a52".into(),
            foil: "rgba(214,178,110,0.42)".into(),
        },
    ]
}

/// Keep exactly five frames, one per rank, whatever the keeper saved. A missing
/// rank falls back to its default rather than leaving a card undressed.
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
                    found
                }
                None => fallback,
            }
        })
        .collect()
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
        let saved = vec![BattleFrame {
            tier: 3,
            name_en: String::new(),
            name_ru: "Своя".into(),
            paper: "#000000".into(),
            ink: String::new(),
            border: "#111111".into(),
            foil: String::new(),
        }];
        let frames = normalize_frames(saved);
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[2].paper, "#000000");
        assert_eq!(frames[2].name_ru, "Своя");
        // Blanks fall back rather than shipping an undressed rank.
        assert_eq!(frames[2].name_en, "Remembered");
        assert!(!frames[2].ink.is_empty());
    }
}
