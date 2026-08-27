// Скромные эпические битвы — the shelf of cards.
//
// A card is a work of the house seen from another side. What the room shows
// first is not a price list but a shelf: cards you have stand face up, the rest
// lie face down in dust with what they would cost written on the back.
//
// Two ranges that look alike and are not the same thing:
//   * `tier`  — the card's rank, 1..5. A property of the card, set by the keeper.
//   * `level` — the state of one person's copy, 1..5. A property of owning it.
// Nothing on this page may quietly turn one into the other.

import type {
  BattleCard,
  BattleFrame,
  BattleFrameMode,
  BattleLayout,
  CardTrait,
} from '$lib/types/api';
import { fontStack } from '$lib/fonts';
import type { Lang } from '$lib/i18n';

export const TIERS = [1, 2, 3, 4, 5] as const;

/** The two coins. `dust` settles on its own; `feed` is given by hand. */
export type Coin = 'dust' | 'feed';

/**
 * The same five frames the server hands out — kept here so a card still has a
 * dress when the frames request fails, and so the admin preview can paint
 * before anything has been saved. The server's `battles::default_frames` is the
 * original; change both together.
 */
/** A bare card: 5 : 7, the ratio of a card held in a hand. */
export const DEFAULT_ASPECT = 5 / 7;
export const DEFAULT_ART_SHARE = 0.44;
export const DEFAULT_HEADER_SHARE = 0.09;
export const DEFAULT_FOOT_SHARE = 0.1;

function painted(
  tier: number,
  nameEn: string,
  nameRu: string,
  paper: string,
  ink: string,
  border: string,
  foil: string,
): BattleFrame {
  return {
    tier, nameEn, nameRu, paper, ink, border, foil,
    frameImage: '',
    frameMode: 'overlay',
    paperImage: '',
    insetTop: 0, insetRight: 0, insetBottom: 0, insetLeft: 0,
    aspect: DEFAULT_ASPECT,
    headerShare: DEFAULT_HEADER_SHARE,
    artShare: DEFAULT_ART_SHARE,
    footShare: DEFAULT_FOOT_SHARE,
    titleFont: '',
    titleInk: '',
    layout: 'corners',
  };
}

/**
 * The same five frames the server hands out — kept here so a card still has a
 * dress when the frames request fails, and so the admin preview can paint
 * before anything has been saved. The server's `battles::default_frames` is the
 * original; change both together.
 */
export const DEFAULT_FRAMES: BattleFrame[] = [
  painted(1, 'Humble',     'Скромная',  '#f8f1e7', '#34251c', '#d8c6b1', ''),
  painted(2, 'Sturdy',     'Крепкая',   '#f3e9db', '#34251c', '#c3ad93', ''),
  painted(3, 'Remembered', 'Памятная',  '#eeddc8', '#34251c', '#a8845f', 'rgba(198,95,60,0.16)'),
  painted(4, 'Rare',       'Редкая',    '#e6cfb2', '#2a1a11', '#6f3b24', 'rgba(198,95,60,0.28)'),
  painted(5, 'Epic',       'Эпическая', '#3a2a1e', '#f3e4cd', '#c99a52', 'rgba(214,178,110,0.42)'),
];

export const LAYOUTS: BattleLayout[] = ['corners', 'plaque'];
export const FRAME_MODES: BattleFrameMode[] = ['overlay', 'behind'];

export function clampTier(tier: number): number {
  if (!Number.isFinite(tier)) return 1;
  return Math.min(5, Math.max(1, Math.round(tier)));
}

/** A card is never left undressed: an unknown rank falls back to its default. */
export function frameFor(tier: number, frames: BattleFrame[] | null | undefined): BattleFrame {
  const rank = clampTier(tier);
  return (
    frames?.find((f) => clampTier(f.tier) === rank) ??
    DEFAULT_FRAMES[rank - 1]
  );
}

/** A card's own exception to its tier's shared frame — a picture just for
 *  this one card, without touching what every other card of that rank wears. */
export interface FrameOverride {
  frameImage?: string;
  frameMode?: BattleFrameMode;
  aspect?: number;
}

/** A broken or empty override is the same as none: the tier's own frame. */
export function parseFrameOverride(raw: string | null | undefined): FrameOverride | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as FrameOverride;
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * The frame this ONE card actually wears: the tier's shared frame, with the
 * card's own `frameOverride` patched on top where it sets something. This is
 * the exception the user asked the frame-click to offer — "pick the tier, or
 * wear a picture of your own" — kept to `frameImage`/`frameMode`/`aspect`
 * only, so the rest of the tier's design (paper, ink, bands, insets) still
 * belongs to the dictionary, not the card.
 */
export function frameForCard(
  card: Pick<BattleCard, 'tier' | 'frameOverride'>,
  frames: BattleFrame[] | null | undefined,
): BattleFrame {
  const base = frameFor(card.tier, frames);
  const override = parseFrameOverride(card.frameOverride);
  if (!override) return base;
  return {
    ...base,
    frameImage: override.frameImage ?? base.frameImage,
    frameMode: override.frameMode ?? base.frameMode,
    aspect: override.aspect ?? base.aspect,
  };
}

/**
 * A URL going into `url("…")`. The keeper types this into the admin, so it is
 * not hostile input — but a stray quote would still end the url() early and let
 * whatever follows be read as CSS, which is a bug either way.
 */
function cssUrl(raw: string): string {
  return raw.replace(/["'()\\\s]/g, encodeURIComponent);
}

/** A dressed frame is one that wears a picture, worn either way round. */
export function isDressed(frame: BattleFrame): boolean {
  return !!frame.frameImage?.trim();
}

/** The picture lies on top and the card shows through the hole in it. */
export function isOverlaid(frame: BattleFrame): boolean {
  return isDressed(frame) && frame.frameMode !== 'behind';
}

/**
 * Every value the card's CSS reads, in one place.
 *
 * Written as custom properties rather than inline rules so the stylesheet still
 * owns the whole design: it can switch the whole effect off under a media query,
 * which an inline style cannot be talked out of.
 */
export function frameVars(frame: BattleFrame): Record<string, string> {
  const image = frame.frameImage?.trim();
  const paperArt = frame.paperImage?.trim();
  return {
    '--paper-image': paperArt ? `url("${cssUrl(paperArt)}")` : 'none',
    '--paper': frame.paper,
    '--ink': frame.ink,
    '--edge': frame.border,
    '--foil': frame.foil || 'transparent',
    '--frame-image': image ? `url("${cssUrl(image)}")` : 'none',
    '--pad-top': `${frame.insetTop || 0}%`,
    '--pad-right': `${frame.insetRight || 0}%`,
    '--pad-bottom': `${frame.insetBottom || 0}%`,
    '--pad-left': `${frame.insetLeft || 0}%`,
    '--aspect': String(frame.aspect || DEFAULT_ASPECT),
    // The three measured bands. The properties band is not here on purpose: it
    // takes whatever these three leave, so it can never be squeezed to nothing
    // by three sliders that happen to add up.
    '--header-share': `${((frame.headerShare ?? DEFAULT_HEADER_SHARE) * 100).toFixed(1)}%`,
    '--art-share': `${((frame.artShare || DEFAULT_ART_SHARE) * 100).toFixed(1)}%`,
    '--foot-share': `${((frame.footShare ?? DEFAULT_FOOT_SHARE) * 100).toFixed(1)}%`,
    '--title-face': frame.titleFont ? fontStack(frame.titleFont) : 'inherit',
    '--title-ink': frame.titleInk?.trim() || frame.ink,
  };
}

export function frameName(frame: BattleFrame, lang: Lang): string {
  const ru = (frame.nameRu ?? '').trim();
  const en = (frame.nameEn ?? '').trim();
  return (lang === 'ru' ? ru || en : en || ru) || String(frame.tier);
}

/**
 * The reader's own language wins, and an empty one never blanks the card —
 * the same two-way fallback the gazette uses, for the same reason: a card
 * written only in Russian must still read as something in English.
 */
export function cardCopy(
  card: BattleCard,
  lang: Lang,
): { title: string; effect: string; lore: string } {
  const ru = lang === 'ru';
  const pick = (a: string | null | undefined, b: string | null | undefined) =>
    ((ru ? a?.trim() || b?.trim() : b?.trim() || a?.trim()) ?? '');
  return {
    title: pick(card.titleRu, card.titleEn),
    effect: pick(card.effectRu, card.effectEn),
    lore: pick(card.loreRu, card.loreEn),
  };
}

/** A property in the reader's language, with the other name kept alongside —
 *  the card shows both, the way the keeper's own drawing does. */
export function traitCopy(
  trait: CardTrait,
  lang: Lang,
): { name: string; other: string; text: string } {
  const ru = lang === 'ru';
  const name = (ru ? trait.nameRu : trait.nameEn)?.trim() || '';
  const other = (ru ? trait.nameEn : trait.nameRu)?.trim() || '';
  const text = ((ru ? trait.textRu?.trim() || trait.textEn?.trim() : trait.textEn?.trim() || trait.textRu?.trim()) ?? '');
  return { name: name || other, other: name && other && name !== other ? other : '', text };
}

/** The header band: what this is, and what kind of thing it is. */
export function headerCopy(card: BattleCard, lang: Lang): { race: string; type: string } {
  const ru = lang === 'ru';
  const pick = (a: string | null | undefined, b: string | null | undefined) =>
    ((ru ? a?.trim() || b?.trim() : b?.trim() || a?.trim()) ?? '');
  return {
    race: pick(card.raceNameRu, card.raceNameEn),
    type: pick(card.typeRu, card.typeEn),
  };
}

export interface Focal {
  x: number;
  y: number;
  zoom: number;
}

const CENTRED: Focal = { x: 0.5, y: 0.5, zoom: 1 };

/** A card with a broken focus is centred, never blank. */
export function parseFocal(raw: string | null | undefined): Focal {
  if (!raw) return CENTRED;
  try {
    const parsed = JSON.parse(raw) as Partial<Focal>;
    const num = (v: unknown, fallback: number, lo: number, hi: number) =>
      typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : fallback;
    return {
      x: num(parsed.x, 0.5, 0, 1),
      y: num(parsed.y, 0.5, 0, 1),
      zoom: num(parsed.zoom, 1, 1, 3),
    };
  } catch {
    return CENTRED;
  }
}

/** `object-position` and `scale` for the picture inside the frame. */
export function focalStyle(raw: string | null | undefined): string {
  const { x, y, zoom } = parseFocal(raw);
  return `object-position:${(x * 100).toFixed(1)}% ${(y * 100).toFixed(1)}%;transform:scale(${zoom});`;
}

/**
 * What a card costs, in the coins it can actually be had for.
 *
 * `null` is not zero: it means this card is not to be had for that coin at all,
 * and the room must not print "0" where it means "never".
 */
export function pricesOf(card: BattleCard): { coin: Coin; amount: number }[] {
  const out: { coin: Coin; amount: number }[] = [];
  if (card.priceDust != null) out.push({ coin: 'dust', amount: card.priceDust });
  if (card.priceFeed != null) out.push({ coin: 'feed', amount: card.priceFeed });
  return out;
}

/**
 * Each card gets its own transition name so the shelf can morph a card into its
 * larger self. Two elements sharing a name abort the whole transition, so this
 * must not be rendered twice on one page — the same rule the archive follows
 * for `figurine-{id}`.
 */
export function cardTransitionName(card: BattleCard): string {
  return `battle-card-${card.id}`;
}

/** Where the work behind the card lives, if it still has one. */
export function workHref(card: BattleCard): string | null {
  const handle = card.figurineSlug || card.figurineId;
  return handle ? `/figurines/${handle}` : null;
}

/**
 * A native file picker as a promise, so a click on the card itself — art,
 * a race icon, a card's own frame picture — can `await` the choice instead
 * of juggling an `<input>` element's own callback.
 */
export function pickImageFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'image/*';
    input.onchange = () => resolve(input.files?.[0] ?? null);
    // No file chosen (the dialog was cancelled) never fires `change`, so the
    // promise is left to resolve later rather than hanging forever — the
    // caller simply never gets a result for that attempt, same as a click
    // that never happened.
    input.click();
  });
}
