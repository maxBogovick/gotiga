//! Cutting a sheet of frame parts into separate pictures.
//!
//! A generator hands back the parts of a frame on one sheet — corners, bars,
//! ornaments, each captioned with a letter. This finds the separate objects on
//! that sheet and returns every one of them cropped to its own artwork, with a
//! transparent ground.
//!
//! The background comes off two different ways, and the sheet decides which:
//!
//!   * a sheet that carries a real alpha channel is taken at its word,
//!     soft edges and all;
//!   * an opaque sheet (white ground, or a painted checkerboard) has its
//!     background found as everything pale and drained that REACHES THE EDGE
//!     OF THE CANVAS. The connectivity is the point: a white highlight inside
//!     a gem does not reach the edge, so it stays part of the artwork instead
//!     of becoming a hole.
//!
//! The naive route — `alpha = 1 - lightness` — is wrong here. It makes gold and
//! the glint on a stone half-transparent, which is exactly what the frame was
//! drawn for.
//!
//! Every number below was measured on real sheets, not chosen. Where a default
//! sits inside a narrow gap, the comment says how narrow.
//!
//! The same cut exists a second time, as `tools/slice_sheet.py` — a standalone
//! tool that works over a folder on disk with no server and no database. That
//! duplication is deliberate, and its cost is that the two can drift: the
//! thresholds here and there were measured on the same sheets, and changing one
//! without the other makes the tool disagree with what the keeper sees.

use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};

/// A sheet is a working document, not an exhibit: 12 MP is a 4000x3000 sheet,
/// far past anything a generator produces, and it bounds the label buffer at a
/// harmless 48 MB.
pub const SHEET_MAX_PIXELS: u64 = 12_000_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SliceSettings {
    /// Alpha above this is artwork; below it is leftover glow. Not zero on
    /// purpose — see [`mask_from_alpha`].
    pub alpha_threshold: u8,
    /// Background is lighter than this (0..1).
    pub bg_value: f32,
    /// ...and more drained than this (0..1).
    pub bg_sat: f32,
    /// Across how wide a gap two pieces are glued back into one object.
    pub merge_gap: u32,
    /// Smaller than this is grit, not an object.
    pub min_area: u32,
    /// Captions are no taller than this.
    pub text_max_h: u32,
    /// ...and carry less gold and gemstone than this share.
    pub text_color: f32,
    /// Keep captions as objects instead of setting them aside.
    pub keep_text: bool,
    /// Margin left around each crop.
    pub pad: u32,
    /// Edge softening, in pixels. Only used when the sheet has no alpha of
    /// its own; a sheet that does keeps its own edge exactly.
    pub feather: u32,
    /// How far colour is stretched out under the transparent edge.
    pub bleed: u32,
}

impl Default for SliceSettings {
    fn default() -> Self {
        Self {
            alpha_threshold: 24,
            bg_value: 0.62,
            bg_sat: 0.20,
            merge_gap: 4,
            min_area: 64,
            text_max_h: 30,
            text_color: 0.10,
            keep_text: false,
            pad: 2,
            feather: 1,
            bleed: 3,
        }
    }
}

/// Which of the two routes the sheet took. Worth reporting: it explains every
/// other number the keeper is looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaskSource {
    /// The sheet's own alpha channel.
    Alpha,
    /// Pale, drained and touching the canvas edge.
    Background,
}

/// What a part looks like it is for. A guess from proportions alone, offered
/// so the keeper corrects rather than types — never trusted by anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RoleGuess {
    Corner,
    SideH,
    SideV,
    Accent,
}

pub struct SlicedPart {
    /// 1-based, in reading order. Stable for a given [`SliceSettings`] — which
    /// is why a commit has to carry back the settings its proposal was made
    /// with, or the numbers point at different things.
    pub index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    /// Set aside as a caption. Still returned, and still numbered: the only
    /// way to check the cut is to see everything it found, including what it
    /// decided to throw away.
    pub is_text: bool,
    pub role: RoleGuess,
    pub image: RgbaImage,
}

pub struct Sliced {
    pub width: u32,
    pub height: u32,
    pub source: MaskSource,
    pub parts: Vec<SlicedPart>,
}

#[derive(Debug)]
pub enum SliceError {
    Decode(String),
    TooLarge { pixels: u64 },
    Empty,
}

impl std::fmt::Display for SliceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SliceError::Decode(e) => write!(f, "Invalid image file: {}", e),
            SliceError::TooLarge { pixels } => write!(
                f,
                "Sheet is too large: {} pixels, limit is {}",
                pixels, SHEET_MAX_PIXELS
            ),
            SliceError::Empty => write!(f, "Nothing found on the sheet"),
        }
    }
}

// ── window sums ────────────────────────────────────────────────────────────
// Growing a mask and softening an edge are the same summed-area table twice.
// A whole morphology crate would earn nothing here.

fn integral(mask: &[bool], w: usize, h: usize) -> Vec<u32> {
    let mut out = vec![0u32; (w + 1) * (h + 1)];
    for y in 0..h {
        let mut row = 0u32;
        for x in 0..w {
            row += mask[y * w + x] as u32;
            out[(y + 1) * (w + 1) + x + 1] = out[y * (w + 1) + x + 1] + row;
        }
    }
    out
}

/// Grow a mask by `r` in every direction. Used to glue an object's stray
/// pieces back together across a gap — by shape, not by bounding box, so a
/// distant neighbour is left alone.
fn dilate(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    if r == 0 {
        return mask.to_vec();
    }
    let sums = integral(mask, w, h);
    let mut out = vec![false; w * h];
    for y in 0..h {
        let y0 = y.saturating_sub(r);
        let y1 = (y + r + 1).min(h);
        for x in 0..w {
            let x0 = x.saturating_sub(r);
            let x1 = (x + r + 1).min(w);
            let s = sums[y1 * (w + 1) + x1] + sums[y0 * (w + 1) + x0]
                - sums[y0 * (w + 1) + x1]
                - sums[y1 * (w + 1) + x0];
            out[y * w + x] = s > 0;
        }
    }
    out
}

/// Box blur of a mask, as a 0..1 field. The soft ramp along a cut edge.
fn blur_mask(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<f32> {
    if r == 0 {
        return mask.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect();
    }
    let sums = integral(mask, w, h);
    let area = ((2 * r + 1) * (2 * r + 1)) as f32;
    let mut out = vec![0.0f32; w * h];
    for y in 0..h {
        // Clamped rather than padded, but divided by the FULL window: a pixel
        // on the crop's own border is genuinely half-surrounded by nothing,
        // and pretending otherwise would harden the edge we are softening.
        let y0 = y.saturating_sub(r);
        let y1 = (y + r + 1).min(h);
        for x in 0..w {
            let x0 = x.saturating_sub(r);
            let x1 = (x + r + 1).min(w);
            let s = sums[y1 * (w + 1) + x1] + sums[y0 * (w + 1) + x0]
                - sums[y0 * (w + 1) + x1]
                - sums[y1 * (w + 1) + x0];
            out[y * w + x] = s as f32 / area;
        }
    }
    out
}

// ── connected areas ────────────────────────────────────────────────────────

struct DisjointSet {
    parent: Vec<u32>,
}

impl DisjointSet {
    fn new() -> Self {
        Self { parent: vec![0] } // slot 0 is "no label" and never a member
    }

    fn make(&mut self) -> u32 {
        let id = self.parent.len() as u32;
        self.parent.push(id);
        id
    }

    fn find(&mut self, mut x: u32) -> u32 {
        while self.parent[x as usize] != x {
            let grand = self.parent[self.parent[x as usize] as usize];
            self.parent[x as usize] = grand;
            x = grand;
        }
        x
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra.max(rb) as usize] = ra.min(rb);
        }
    }
}

struct Areas {
    labels: Vec<u32>,
    count: u32,
}

/// Label by row runs rather than by pixel: a row holds a handful of runs, not
/// a thousand pixels, so the merging work is proportional to the drawing and
/// not to the canvas. Diagonal touches count as connected.
fn connected_areas(mask: &[bool], w: usize, h: usize) -> Areas {
    let mut set = DisjointSet::new();
    let mut labels = vec![0u32; w * h];
    let mut prev: Vec<(usize, usize, u32)> = Vec::new();
    let mut cur: Vec<(usize, usize, u32)> = Vec::new();

    for y in 0..h {
        cur.clear();
        let row = &mask[y * w..(y + 1) * w];
        let mut x = 0;
        while x < w {
            if !row[x] {
                x += 1;
                continue;
            }
            let start = x;
            while x < w && row[x] {
                x += 1;
            }
            let end = x; // exclusive
            let mut label = 0u32;
            for &(ps, pe, pl) in prev.iter() {
                // +1 on both sides is the diagonal reach
                if ps < end + 1 && pe + 1 > start {
                    if label == 0 {
                        label = set.find(pl);
                    } else {
                        set.union(label, pl);
                        label = set.find(label);
                    }
                }
            }
            if label == 0 {
                label = set.make();
            }
            cur.push((start, end, label));
        }
        for &(s, e, l) in cur.iter() {
            labels[y * w + s..y * w + e].fill(l);
        }
        std::mem::swap(&mut prev, &mut cur);
    }

    let mut remap = vec![0u32; set.parent.len()];
    let mut count = 0u32;
    for value in labels.iter_mut() {
        if *value == 0 {
            continue;
        }
        let root = set.find(*value) as usize;
        if remap[root] == 0 {
            count += 1;
            remap[root] = count;
        }
        *value = remap[root];
    }
    Areas { labels, count }
}

// ── the background ─────────────────────────────────────────────────────────

/// The threshold is deliberately not zero.
///
/// A wide, barely-there glow surrounds the artwork — single-digit percents of
/// opacity. Through it the caption under a bar is CONNECTED to the bar, and
/// then "H1" ends up inside the cut. For a 9-slice the glow is harmful anyway:
/// stretched along a side, it shows as a seam.
fn mask_from_alpha(rgba: &RgbaImage, threshold: u8) -> Vec<bool> {
    rgba.pixels().map(|p| p.0[3] > threshold).collect()
}

/// Background is pale, drained, and connected to the canvas edge.
fn mask_from_background(rgba: &RgbaImage, bg_value: f32, bg_sat: f32) -> Vec<bool> {
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let pale: Vec<bool> = rgba
        .pixels()
        .map(|p| {
            let (v, s) = value_and_sat(p.0);
            v >= bg_value && s <= bg_sat
        })
        .collect();

    let areas = connected_areas(&pale, w, h);
    let mut outer = vec![false; areas.count as usize + 1];
    for x in 0..w {
        outer[areas.labels[x] as usize] = true;
        outer[areas.labels[(h - 1) * w + x] as usize] = true;
    }
    for y in 0..h {
        outer[areas.labels[y * w] as usize] = true;
        outer[areas.labels[y * w + w - 1] as usize] = true;
    }
    outer[0] = false;

    // Everything that is not outer background is artwork — holes fill
    // themselves, which is the whole reason for going through connectivity.
    areas
        .labels
        .iter()
        .map(|&l| !outer[l as usize])
        .collect()
}

fn value_and_sat(px: [u8; 4]) -> (f32, f32) {
    let r = px[0] as f32 / 255.0;
    let g = px[1] as f32 / 255.0;
    let b = px[2] as f32 / 255.0;
    let v = r.max(g).max(b);
    let lo = r.min(g).min(b);
    let s = if v > 0.0 { (v - lo) / v } else { 0.0 };
    (v, s)
}

/// Gold and gemstone: saturated AND light.
///
/// The lightness bar is not decoration. A dark brown caption is saturated too,
/// and its feathering into a white ground passes a saturation test on its own.
/// But feathering desaturates as it lightens: at v=0.6 it is down to s≈0.2,
/// where gold is still at 0.45.
fn is_colourful(px: [u8; 4]) -> bool {
    let (v, s) = value_and_sat(px);
    s > 0.35 && v > 0.60
}

// ── the cut ────────────────────────────────────────────────────────────────

pub fn slice(data: &[u8], settings: &SliceSettings) -> Result<Sliced, SliceError> {
    let decoded =
        image::load_from_memory(data).map_err(|e| SliceError::Decode(e.to_string()))?;
    let pixels = decoded.width() as u64 * decoded.height() as u64;
    if pixels > SHEET_MAX_PIXELS {
        return Err(SliceError::TooLarge { pixels });
    }
    let rgba = decoded.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return Err(SliceError::Empty);
    }

    // Sampled, like the frame upload does: on a sheet the answer is the same
    // after a few thousand pixels, and 37 is a stride that cannot fall into
    // step with a row width.
    let sampled = rgba.pixels().step_by(37).count().max(1);
    let translucent = rgba.pixels().step_by(37).filter(|p| p.0[3] < 250).count();
    let source = if translucent * 100 > sampled {
        MaskSource::Alpha
    } else {
        MaskSource::Background
    };

    let mut mask = match source {
        MaskSource::Alpha => mask_from_alpha(&rgba, settings.alpha_threshold),
        MaskSource::Background => mask_from_background(&rgba, settings.bg_value, settings.bg_sat),
    };

    // Grit (JPEG noise, shreds of a painted checkerboard) goes before anything
    // else: left in, it glues neighbours together when the mask grows.
    let areas = connected_areas(&mask, w, h);
    if areas.count == 0 {
        return Err(SliceError::Empty);
    }
    let mut area_of = vec![0u32; areas.count as usize + 1];
    for &l in areas.labels.iter() {
        area_of[l as usize] += 1;
    }
    let mut swept = false;
    for (bit, &label) in mask.iter_mut().zip(areas.labels.iter()) {
        let area = area_of[label as usize];
        if label != 0 && area < settings.min_area {
            *bit = false;
            swept = true;
        }
    }
    let areas = if swept {
        connected_areas(&mask, w, h)
    } else {
        areas
    };
    if areas.count == 0 {
        return Err(SliceError::Empty);
    }

    // Captions come off BEFORE the merge. Otherwise a letter lying against a
    // bar grows together with it and ends up inside the cut — and by then it
    // cannot be thrown away, because it has become part of the object.
    //
    // A caption is short and colourless. Height beats brightness here: a
    // letter is thin, and half its pixels are the feathering into a pale
    // ground, which is light. Gold and gemstone, though, it has none.
    //
    // The 30 px default is a measurement, not a principle: on real sheets the
    // captions run to 27 and the thinnest genuine bar is 33. The gap is
    // narrow, which is why everything set aside is still returned and still
    // numbered — a wrong call is fixed by keeping that number, not by
    // rewriting the rule.
    let stats = area_stats(&areas, &rgba, w, h);
    let mut text_mask = vec![false; w * h];
    let mut any_text = false;
    if !settings.keep_text {
        let mut is_caption = vec![false; areas.count as usize + 1];
        for label in 1..=areas.count as usize {
            let st = &stats[label];
            if st.height <= settings.text_max_h && st.colour_share() < settings.text_color {
                is_caption[label] = true;
                any_text = true;
            }
        }
        if any_text {
            for i in 0..text_mask.len() {
                text_mask[i] = is_caption[areas.labels[i] as usize];
            }
        }
    }

    let art_mask: Vec<bool> = (0..mask.len()).map(|i| mask[i] && !text_mask[i]).collect();
    let grown = dilate(&art_mask, w, h, settings.merge_gap as usize);
    let art_groups = connected_areas(&grown, w, h);
    let mut groups: Vec<u32> = (0..w * h)
        .map(|i| if art_mask[i] { art_groups.labels[i] } else { 0 })
        .collect();
    let mut group_count = art_groups.count;
    let mut group_is_text = vec![false; group_count as usize + 1];

    if any_text {
        // Letters are merged among themselves, so a word reaches the keeper
        // whole instead of as a pile of glyphs.
        let grown = dilate(&text_mask, w, h, settings.merge_gap as usize);
        let text_groups = connected_areas(&grown, w, h);
        group_is_text.resize((group_count + text_groups.count) as usize + 1, true);
        for i in 0..w * h {
            if text_mask[i] && text_groups.labels[i] != 0 {
                groups[i] = text_groups.labels[i] + group_count;
            }
        }
        group_count += text_groups.count;
    }

    let mut boxes: Vec<Option<Bounds>> = vec![None; group_count as usize + 1];
    for y in 0..h {
        for x in 0..w {
            let g = groups[y * w + x] as usize;
            if g == 0 {
                continue;
            }
            match &mut boxes[g] {
                Some(b) => b.add(x as u32, y as u32),
                None => boxes[g] = Some(Bounds::at(x as u32, y as u32)),
            }
        }
    }

    let present: Vec<(u32, Bounds)> = (1..=group_count)
        .filter_map(|g| boxes[g as usize].map(|b| (g, b)))
        .collect();
    if present.is_empty() {
        return Err(SliceError::Empty);
    }
    let order = reading_order(&present);

    let mut parts = Vec::with_capacity(order.len());
    for (position, &slot) in order.iter().enumerate() {
        let (group, bounds) = present[slot];
        let image = cut_out(&rgba, &groups, group, &bounds, source, settings);
        parts.push(SlicedPart {
            index: position as u32 + 1,
            x: bounds.x0,
            y: bounds.y0,
            width: bounds.width(),
            height: bounds.height(),
            is_text: group_is_text[group as usize],
            role: guess_role(bounds.width(), bounds.height()),
            image,
        });
    }

    Ok(Sliced {
        width: rgba.width(),
        height: rgba.height(),
        source,
        parts,
    })
}

#[derive(Clone, Copy)]
struct Bounds {
    x0: u32,
    y0: u32,
    x1: u32, // inclusive
    y1: u32,
}

impl Bounds {
    fn at(x: u32, y: u32) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        }
    }
    fn add(&mut self, x: u32, y: u32) {
        self.x0 = self.x0.min(x);
        self.y0 = self.y0.min(y);
        self.x1 = self.x1.max(x);
        self.y1 = self.y1.max(y);
    }
    fn width(&self) -> u32 {
        self.x1 - self.x0 + 1
    }
    fn height(&self) -> u32 {
        self.y1 - self.y0 + 1
    }
}

struct AreaStat {
    height: u32,
    pixels: u32,
    colourful: u32,
}

impl AreaStat {
    fn colour_share(&self) -> f32 {
        if self.pixels == 0 {
            0.0
        } else {
            self.colourful as f32 / self.pixels as f32
        }
    }
}

/// Height and colour share for every area, in one pass. Asking area by area
/// would re-read the whole sheet each time, and on a hundred letters that is
/// minutes rather than milliseconds.
fn area_stats(areas: &Areas, rgba: &RgbaImage, w: usize, h: usize) -> Vec<AreaStat> {
    let mut top = vec![u32::MAX; areas.count as usize + 1];
    let mut bottom = vec![0u32; areas.count as usize + 1];
    let mut stats: Vec<AreaStat> = (0..=areas.count)
        .map(|_| AreaStat {
            height: 0,
            pixels: 0,
            colourful: 0,
        })
        .collect();
    let raw = rgba.as_raw();
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let l = areas.labels[i] as usize;
            if l == 0 {
                continue;
            }
            top[l] = top[l].min(y as u32);
            bottom[l] = bottom[l].max(y as u32);
            stats[l].pixels += 1;
            let px = [raw[i * 4], raw[i * 4 + 1], raw[i * 4 + 2], raw[i * 4 + 3]];
            if is_colourful(px) {
                stats[l].colourful += 1;
            }
        }
    }
    for l in 1..=areas.count as usize {
        if top[l] != u32::MAX {
            stats[l].height = bottom[l] - top[l] + 1;
        }
    }
    stats
}

/// Left to right, top to bottom — but by rows, not by one coordinate. Half of
/// the SMALLER of the two heights: otherwise one long bar crossing the sheet
/// from top to bottom drags its neighbours into separate rows.
fn reading_order(present: &[(u32, Bounds)]) -> Vec<usize> {
    let mut sorted: Vec<usize> = (0..present.len()).collect();
    sorted.sort_by_key(|&i| (present[i].1.y0, present[i].1.x0));

    let mut result = Vec::with_capacity(present.len());
    let mut row: Vec<usize> = Vec::new();
    let (mut top, mut bottom) = (0u32, 0u32);
    for &i in sorted.iter() {
        let b = present[i].1;
        if !row.is_empty() {
            let own = b.height();
            let band = bottom - top + 1;
            let slack = own.min(band) as f32 * 0.5;
            if b.y0 as f32 >= (bottom + 1) as f32 - slack {
                row.sort_by_key(|&j| present[j].1.x0);
                result.append(&mut row);
            }
        }
        if row.is_empty() {
            top = b.y0;
            bottom = b.y1;
        } else {
            top = top.min(b.y0);
            bottom = bottom.max(b.y1);
        }
        row.push(i);
    }
    row.sort_by_key(|&j| present[j].1.x0);
    result.append(&mut row);
    result
}

/// From proportions only. A bar that is much wider than it is tall goes along
/// a horizontal side; the same shape stood on end goes up a vertical one.
/// Anything squarer is offered as an accent, because a corner is a decision
/// about a frame and not about an aspect ratio.
fn guess_role(width: u32, height: u32) -> RoleGuess {
    let (w, h) = (width as f32, height as f32);
    if w / h > 2.2 {
        RoleGuess::SideH
    } else if h / w > 2.2 {
        RoleGuess::SideV
    } else {
        RoleGuess::Accent
    }
}

fn cut_out(
    rgba: &RgbaImage,
    groups: &[u32],
    group: u32,
    bounds: &Bounds,
    source: MaskSource,
    settings: &SliceSettings,
) -> RgbaImage {
    let sheet_w = rgba.width();
    let sheet_h = rgba.height();
    let pad = settings.pad;
    let x0 = bounds.x0.saturating_sub(pad);
    let y0 = bounds.y0.saturating_sub(pad);
    let x1 = (bounds.x1 + pad + 1).min(sheet_w);
    let y1 = (bounds.y1 + pad + 1).min(sheet_h);
    let (cw, ch) = ((x1 - x0) as usize, (y1 - y0) as usize);

    let mut mask = vec![false; cw * ch];
    for y in 0..ch {
        for x in 0..cw {
            let sx = x0 as usize + x;
            let sy = y0 as usize + y;
            mask[y * cw + x] = groups[sy * sheet_w as usize + sx] == group;
        }
    }

    // A sheet with its own alpha keeps it exactly; only a sheet whose ground
    // we inferred gets a manufactured edge.
    let alpha: Vec<u8> = match source {
        MaskSource::Alpha => (0..cw * ch)
            .map(|i| {
                if mask[i] {
                    let (x, y) = (i % cw, i / cw);
                    rgba.get_pixel(x0 + x as u32, y0 + y as u32).0[3]
                } else {
                    0
                }
            })
            .collect(),
        MaskSource::Background => blur_mask(&mask, cw, ch, settings.feather as usize)
            .into_iter()
            .map(|a| (a * 255.0).round().clamp(0.0, 255.0) as u8)
            .collect(),
    };

    let mut rgb = vec![0u8; cw * ch * 3];
    for y in 0..ch {
        for x in 0..cw {
            let p = rgba.get_pixel(x0 + x as u32, y0 + y as u32).0;
            let i = (y * cw + x) * 3;
            rgb[i] = p[0];
            rgb[i + 1] = p[1];
            rgb[i + 2] = p[2];
        }
    }
    bleed_colour(&mut rgb, &alpha, cw, ch, settings.bleed as usize);

    let mut out = RgbaImage::new(cw as u32, ch as u32);
    for y in 0..ch {
        for x in 0..cw {
            let i = y * cw + x;
            out.put_pixel(
                x as u32,
                y as u32,
                image::Rgba([rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2], alpha[i]]),
            );
        }
    }
    out
}

/// Stretch colour out under the transparent edge.
///
/// Without it a fringe of the old ground rides along the outline: a browser
/// smoothing the picture down mixes in the colour of pixels nobody can see.
fn bleed_colour(rgb: &mut [u8], alpha: &[u8], w: usize, h: usize, rounds: usize) {
    let mut known: Vec<bool> = alpha.iter().map(|&a| a > 32).collect();
    for _ in 0..rounds {
        let mut next = known.clone();
        let mut changed = false;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if known[i] {
                    continue;
                }
                let (mut sum, mut count) = ([0u32; 3], 0u32);
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let n = ny as usize * w + nx as usize;
                        if !known[n] {
                            continue;
                        }
                        sum[0] += rgb[n * 3] as u32;
                        sum[1] += rgb[n * 3 + 1] as u32;
                        sum[2] += rgb[n * 3 + 2] as u32;
                        count += 1;
                    }
                }
                // Nothing known nearby yet — this pixel waits for the next
                // round, when its neighbours may have been filled in.
                let Some(known_count) = std::num::NonZeroU32::new(count) else {
                    continue;
                };
                for c in 0..3 {
                    rgb[i * 3 + c] = (sum[c] / known_count) as u8;
                }
                next[i] = true;
                changed = true;
            }
        }
        known = next;
        if !changed {
            break;
        }
    }
}

/// A rectangle drawn by hand on a finished piece, in fractions of that piece.
///
/// Fractions and not pixels, because the keeper draws on whatever size the
/// screen happened to show — a 220 px review thumbnail one time, the full
/// picture the next. A fraction means the same thing at both.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FractionRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Take one hand-drawn rectangle off a piece.
///
/// The rectangle is a rough aim, not the answer: what comes out is trimmed to
/// the artwork inside it, exactly the way every automatic cut is trimmed. So
/// the keeper can draw fast and wide, and two rectangles drawn around two
/// corners give two corners rather than two corners with margins.
///
/// `None` when the rectangle caught nothing but transparency — there is no
/// honest picture to save, and an empty file filed under a name is worse than
/// being told the rectangle missed.
pub fn crop_to_content(image: &RgbaImage, rect: &FractionRect, alpha_floor: u8) -> Option<RgbaImage> {
    let (w, h) = (image.width() as f32, image.height() as f32);
    let x0 = (rect.x.clamp(0.0, 1.0) * w).floor() as u32;
    let y0 = (rect.y.clamp(0.0, 1.0) * h).floor() as u32;
    let x1 = ((rect.x + rect.w).clamp(0.0, 1.0) * w).ceil() as u32;
    let y1 = ((rect.y + rect.h).clamp(0.0, 1.0) * h).ceil() as u32;
    let x1 = x1.min(image.width());
    let y1 = y1.min(image.height());
    if x1 <= x0 || y1 <= y0 {
        return None;
    }

    let mut bounds: Option<Bounds> = None;
    for y in y0..y1 {
        for x in x0..x1 {
            if image.get_pixel(x, y).0[3] > alpha_floor {
                match &mut bounds {
                    Some(b) => b.add(x, y),
                    None => bounds = Some(Bounds::at(x, y)),
                }
            }
        }
    }
    let b = bounds?;
    Some(
        image::imageops::crop_imm(image, b.x0, b.y0, b.width(), b.height()).to_image(),
    )
}

/// A part shrunk to fit a box, for review in the browser. Never enlarged: a
/// thumbnail that invents detail is worse than a small one.
pub fn thumbnail(image: &RgbaImage, box_px: u32) -> RgbaImage {
    if image.width() <= box_px && image.height() <= box_px {
        return image.clone();
    }
    DynamicImage::ImageRgba8(image.clone())
        .resize(box_px, box_px, FilterType::Lanczos3)
        .to_rgba8()
}

/// Lossy WebP with an alpha channel — the same format the frame-art upload
/// writes, and for the same reason: an ordinary JPEG rendition would fill in
/// the transparency that the whole cut exists to produce.
pub fn to_webp(image: &RgbaImage, quality: f32) -> Vec<u8> {
    webp::Encoder::from_rgba(image.as_raw(), image.width(), image.height())
        .encode(quality)
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png(image: &RgbaImage) -> Vec<u8> {
        let mut out = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut out, image::ImageFormat::Png)
            .expect("png encodes");
        out.into_inner()
    }

    fn fill(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, colour: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                image.put_pixel(xx, yy, Rgba(colour));
            }
        }
    }

    const GOLD: [u8; 4] = [200, 160, 56, 255];
    const GEM: [u8; 4] = [32, 80, 200, 255];
    const INK: [u8; 4] = [74, 58, 40, 255]; // the brown a caption is set in
    const PAPER: [u8; 4] = [255, 255, 255, 255];

    /// A sheet with a white ground: one bar, one square, one caption.
    fn opaque_sheet() -> RgbaImage {
        let mut sheet = RgbaImage::from_pixel(200, 120, Rgba(PAPER));
        fill(&mut sheet, 10, 10, 100, 14, GOLD); // wide — a horizontal side
        fill(&mut sheet, 150, 10, 30, 30, GEM); // square — an accent
        fill(&mut sheet, 40, 40, 20, 8, INK); // short and colourless — a caption
        sheet
    }

    #[test]
    fn finds_every_object_and_sets_the_caption_aside() {
        let cut = slice(&png(&opaque_sheet()), &SliceSettings::default()).unwrap();
        assert_eq!(cut.source, MaskSource::Background);
        assert_eq!(cut.parts.len(), 3, "bar, square and caption are all reported");

        let kept: Vec<_> = cut.parts.iter().filter(|p| !p.is_text).collect();
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].role, RoleGuess::SideH);
        assert_eq!(kept[1].role, RoleGuess::Accent);

        // The caption is set aside, not dropped: it keeps a number, so a wrong
        // call can be undone by asking for that number back.
        let caption = cut.parts.iter().find(|p| p.is_text).unwrap();
        assert_eq!((caption.width, caption.height), (20, 8));
    }

    #[test]
    fn a_captions_number_survives_it_being_kept() {
        let sheet = png(&opaque_sheet());
        let numbers: Vec<u32> = slice(&sheet, &SliceSettings::default())
            .unwrap()
            .parts
            .iter()
            .map(|p| p.index)
            .collect();
        let keeping = SliceSettings {
            keep_text: true,
            ..SliceSettings::default()
        };
        let kept: Vec<u32> = slice(&sheet, &keeping)
            .unwrap()
            .parts
            .iter()
            .map(|p| p.index)
            .collect();
        assert_eq!(numbers, kept);
    }

    #[test]
    fn the_ground_comes_off_and_the_artwork_stays() {
        let cut = slice(&png(&opaque_sheet()), &SliceSettings::default()).unwrap();
        let bar = &cut.parts[0].image;
        // Padding leaves a transparent margin; the middle is untouched gold.
        assert_eq!(bar.get_pixel(0, 0).0[3], 0, "the corner is see-through");
        let middle = bar.get_pixel(bar.width() / 2, bar.height() / 2);
        assert_eq!(middle.0[3], 255, "the artwork is solid");
        assert_eq!([middle.0[0], middle.0[1], middle.0[2]], [GOLD[0], GOLD[1], GOLD[2]]);
    }

    #[test]
    fn a_pale_hole_inside_the_artwork_is_not_a_hole() {
        // The reason the ground is found by connectivity and not by lightness:
        // a white glint inside a stone never reaches the canvas edge.
        let mut sheet = RgbaImage::from_pixel(120, 120, Rgba(PAPER));
        fill(&mut sheet, 20, 20, 60, 60, GEM);
        fill(&mut sheet, 45, 45, 10, 10, PAPER); // the glint
        let cut = slice(&png(&sheet), &SliceSettings::default()).unwrap();
        assert_eq!(cut.parts.len(), 1);
        let stone = &cut.parts[0].image;
        let glint = stone.get_pixel(stone.width() / 2, stone.height() / 2);
        assert_eq!(glint.0[3], 255, "the glint belongs to the stone");
        assert_eq!([glint.0[0], glint.0[1], glint.0[2]], [255, 255, 255]);
    }

    #[test]
    fn a_sheet_with_its_own_alpha_is_taken_at_its_word() {
        let mut sheet = RgbaImage::from_pixel(200, 80, Rgba([0, 0, 0, 0]));
        fill(&mut sheet, 10, 10, 60, 60, GEM);
        fill(&mut sheet, 120, 10, 60, 60, GOLD);
        let cut = slice(&png(&sheet), &SliceSettings::default()).unwrap();
        assert_eq!(cut.source, MaskSource::Alpha);
        assert_eq!(cut.parts.len(), 2);
        assert!(cut.parts.iter().all(|p| !p.is_text));
    }

    #[test]
    fn a_faint_glow_does_not_tie_a_caption_to_its_part() {
        // The whole reason the alpha threshold is not zero.
        let mut sheet = RgbaImage::from_pixel(200, 120, Rgba([0, 0, 0, 0]));
        fill(&mut sheet, 10, 10, 120, 30, GOLD);
        fill(&mut sheet, 60, 70, 20, 10, INK); // the caption
        for y in 40..70 {
            for x in 10..130 {
                sheet.put_pixel(x, y, Rgba([120, 100, 60, 8])); // the glow between them
            }
        }
        let cut = slice(&png(&sheet), &SliceSettings::default()).unwrap();
        assert_eq!(cut.parts.len(), 2, "the glow is ground, not a bridge");
        assert_eq!(cut.parts[0].height, 30, "the bar did not swallow the caption");
        assert!(cut.parts[1].is_text);
    }

    #[test]
    fn stray_pieces_of_one_object_are_glued_back() {
        // A spike detached from its stone by two pixels is one object, and a
        // distant neighbour is still two.
        let mut sheet = RgbaImage::from_pixel(240, 120, Rgba(PAPER));
        fill(&mut sheet, 20, 20, 40, 40, GEM);
        fill(&mut sheet, 62, 20, 20, 40, GEM); // two px away
        fill(&mut sheet, 160, 20, 40, 40, GOLD); // far off
        let cut = slice(&png(&sheet), &SliceSettings::default()).unwrap();
        assert_eq!(cut.parts.len(), 2);
        assert_eq!(cut.parts[0].width, 62, "the spike came back to the stone");
    }

    #[test]
    fn grit_is_swept_before_it_can_glue_neighbours() {
        let mut sheet = RgbaImage::from_pixel(200, 120, Rgba(PAPER));
        fill(&mut sheet, 20, 20, 40, 40, GEM);
        fill(&mut sheet, 120, 20, 40, 40, GOLD);
        fill(&mut sheet, 90, 38, 3, 3, INK); // a speck between them
        let cut = slice(&png(&sheet), &SliceSettings::default()).unwrap();
        assert_eq!(cut.parts.len(), 2, "the speck is not an object");
    }

    #[test]
    fn an_empty_sheet_says_so_instead_of_returning_nothing() {
        let sheet = RgbaImage::from_pixel(60, 60, Rgba(PAPER));
        assert!(matches!(
            slice(&png(&sheet), &SliceSettings::default()),
            Err(SliceError::Empty)
        ));
    }

    #[test]
    fn a_hand_drawn_rectangle_is_an_aim_and_not_the_answer() {
        // Two stones on one piece, the way two corners arrive glued. A rough
        // rectangle around the left one comes out tight around the stone.
        let mut piece = RgbaImage::from_pixel(200, 100, Rgba([0, 0, 0, 0]));
        fill(&mut piece, 20, 30, 40, 40, GEM);
        fill(&mut piece, 140, 30, 40, 40, GOLD);

        let left = crop_to_content(
            &piece,
            &FractionRect { x: 0.0, y: 0.0, w: 0.5, h: 1.0 },
            8,
        )
        .expect("the rectangle caught the stone");
        assert_eq!(left.dimensions(), (40, 40), "trimmed to the artwork, not to the aim");
        assert_eq!(left.get_pixel(0, 0).0, GEM);
    }

    #[test]
    fn a_rectangle_that_caught_only_air_says_so() {
        let mut piece = RgbaImage::from_pixel(100, 100, Rgba([0, 0, 0, 0]));
        fill(&mut piece, 70, 70, 20, 20, GEM);
        assert!(
            crop_to_content(&piece, &FractionRect { x: 0.0, y: 0.0, w: 0.4, h: 0.4 }, 8)
                .is_none()
        );
    }

    #[test]
    fn a_rectangle_reaching_past_the_edge_is_pulled_back_in() {
        let mut piece = RgbaImage::from_pixel(60, 60, Rgba([0, 0, 0, 0]));
        fill(&mut piece, 10, 10, 40, 40, GOLD);
        let all = crop_to_content(
            &piece,
            &FractionRect { x: -0.5, y: -0.5, w: 3.0, h: 3.0 },
            8,
        )
        .expect("clamped, not refused");
        assert_eq!(all.dimensions(), (40, 40));
    }

    #[test]
    fn settings_arrive_as_the_measured_defaults_when_json_omits_them() {
        let from_nothing: SliceSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(from_nothing, SliceSettings::default());
        let one_field: SliceSettings = serde_json::from_str(r#"{"mergeGap":12}"#).unwrap();
        assert_eq!(one_field.merge_gap, 12);
        assert_eq!(one_field.text_max_h, SliceSettings::default().text_max_h);
    }
}
