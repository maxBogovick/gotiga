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
  BattleBadgeShape,
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
export const DEFAULT_COST_X = 10;
export const DEFAULT_COST_Y = 9;
export const DEFAULT_POWER_X = 90;
export const DEFAULT_POWER_Y = 91;

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
    backImage: '',
    cornerImage: '',
    sideImageH: '',
    sideImageV: '',
    cornerExtra: '',
    sideMidH: '',
    sideMidV: '',
    insetTop: 0, insetRight: 0, insetBottom: 0, insetLeft: 0,
    aspect: DEFAULT_ASPECT,
    headerShare: DEFAULT_HEADER_SHARE,
    artShare: DEFAULT_ART_SHARE,
    footShare: DEFAULT_FOOT_SHARE,
    titleFont: '',
    titleInk: '',
    layout: 'corners',
    costX: DEFAULT_COST_X,
    costY: DEFAULT_COST_Y,
    powerX: DEFAULT_POWER_X,
    powerY: DEFAULT_POWER_Y,
    costShape: 'circle',
    powerShape: 'circle',
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
export const FRAME_MODES: BattleFrameMode[] = ['overlay', 'behind', 'sliced'];
export const BADGE_SHAPES: BattleBadgeShape[] = ['circle', 'square', 'diamond', 'hex', 'shield'];

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
 *  this one card, without touching what every other card of that rank wears.
 *  The four insets are here too, not just the picture: a race's own frame per
 *  level wears a picture the tier's insets were never tuned for, so where its
 *  window actually sits has to travel with it. */
export interface FrameOverride {
  frameImage?: string;
  frameMode?: BattleFrameMode;
  aspect?: number;
  insetTop?: number;
  insetRight?: number;
  insetBottom?: number;
  insetLeft?: number;
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

/** A race's own dress per level of an owned copy: 5 slots, index 0 = level 1.
 *  Anything unparseable or short comes back as 5 empty slots, never fewer. */
export function parseLevelFrames(raw: string | null | undefined): (FrameOverride | null)[] {
  const empty: (FrameOverride | null)[] = [null, null, null, null, null];
  if (!raw) return empty;
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return empty;
    return empty.map((_, i) => (parsed[i] && typeof parsed[i] === 'object' ? parsed[i] : null));
  } catch {
    return empty;
  }
}

/** Patch a base frame's picture and window from an override, the one merge
 *  every layer of dress uses. */
function patchFrame(base: BattleFrame, patch: FrameOverride | null): BattleFrame {
  if (!patch) return base;
  return {
    ...base,
    frameImage: patch.frameImage ?? base.frameImage,
    frameMode: patch.frameMode ?? base.frameMode,
    aspect: patch.aspect ?? base.aspect,
    insetTop: patch.insetTop ?? base.insetTop,
    insetRight: patch.insetRight ?? base.insetRight,
    insetBottom: patch.insetBottom ?? base.insetBottom,
    insetLeft: patch.insetLeft ?? base.insetLeft,
  };
}

/**
 * The frame this ONE card actually wears, built in three layers:
 *   1. the tier's shared frame
 *   2. the card's race, dressed for this level of an owned copy (if the race
 *      set one for that level — a level nobody dressed keeps the tier's own)
 *   3. this one card's own `frameOverride` — the keeper's most specific
 *      exception, "wear a picture of your own", wins over both of the above
 * Kept to `frameImage`/`frameMode`/`aspect` at every layer, so the rest of the
 * tier's design (paper, ink, bands, insets) always belongs to the dictionary,
 * never to a race or a single card.
 */
export function frameForCard(
  card: Pick<BattleCard, 'tier' | 'frameOverride' | 'raceLevelFrames'>,
  frames: BattleFrame[] | null | undefined,
  level?: number | null,
): BattleFrame {
  let frame = frameFor(card.tier, frames);
  const levelFrames = parseLevelFrames(card.raceLevelFrames);
  frame = patchFrame(frame, levelFrames[clampTier(level ?? 1) - 1]);
  frame = patchFrame(frame, parseFrameOverride(card.frameOverride));
  return frame;
}

/**
 * A URL going into `url("…")`. The keeper types this into the admin, so it is
 * not hostile input — but a stray quote would still end the url() early and let
 * whatever follows be read as CSS, which is a bug either way.
 */
function cssUrl(raw: string): string {
  return raw.replace(/["'()\\\s]/g, encodeURIComponent);
}

/** A dressed frame is one that wears a picture, worn either way round — a
 *  single stretched photograph, or built from a corner and two side
 *  pictures instead. */
export function isDressed(frame: BattleFrame): boolean {
  return (
    !!frame.frameImage?.trim() ||
    !!frame.cornerImage?.trim() ||
    !!frame.sideImageH?.trim() ||
    !!frame.sideImageV?.trim()
  );
}

/** Built from a corner and two side pictures rather than one stretched whole. */
export function isSliced(frame: BattleFrame): boolean {
  return frame.frameMode === 'sliced' && isDressed(frame);
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
  const backArt = frame.backImage?.trim();
  const cornerArt = frame.cornerImage?.trim();
  const sideHArt = frame.sideImageH?.trim();
  const sideVArt = frame.sideImageV?.trim();
  const cornerExtraArt = frame.cornerExtra?.trim();
  const sideMidHArt = frame.sideMidH?.trim();
  const sideMidVArt = frame.sideMidV?.trim();
  return {
    '--paper-image': paperArt ? `url("${cssUrl(paperArt)}")` : 'none',
    '--paper': frame.paper,
    '--ink': frame.ink,
    '--edge': frame.border,
    '--foil': frame.foil || 'transparent',
    '--frame-image': image ? `url("${cssUrl(image)}")` : 'none',
    '--back-image': backArt ? `url("${cssUrl(backArt)}")` : 'none',
    '--corner-image': cornerArt ? `url("${cssUrl(cornerArt)}")` : 'none',
    '--side-image-h': sideHArt ? `url("${cssUrl(sideHArt)}")` : 'none',
    '--side-image-v': sideVArt ? `url("${cssUrl(sideVArt)}")` : 'none',
    '--corner-extra-image': cornerExtraArt ? `url("${cssUrl(cornerExtraArt)}")` : 'none',
    '--side-mid-h-image': sideMidHArt ? `url("${cssUrl(sideMidHArt)}")` : 'none',
    '--side-mid-v-image': sideMidVArt ? `url("${cssUrl(sideMidVArt)}")` : 'none',
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

/** The card's four insets — how far the window sits from each side of the
 *  photograph, in % of the card. */
export type InsetKey = 'insetTop' | 'insetRight' | 'insetBottom' | 'insetLeft';
export const INSET_MAX = 45;

const OPPOSITE_INSET: Record<InsetKey, InsetKey> = {
  insetTop: 'insetBottom',
  insetBottom: 'insetTop',
  insetLeft: 'insetRight',
  insetRight: 'insetLeft',
};

/**
 * Grows `kind` by `delta` and, when `mirror` (the default), the side facing
 * it by the same amount — a tier's two parallel sides are edited as a pair,
 * not four independent numbers, so narrowing the left always narrows the
 * right by the same amount instead of leaving the keeper to match them by
 * eye. Shared by the on-card drag handles and the Frames tab's own sliders,
 * so both ways of setting an inset agree. Each side is clamped on its own.
 *
 * A race's own frame per level does NOT mirror: its window was cut into one
 * particular picture, off-centre as that picture happens to be — the herbs
 * hanging along the top of a frame take more room than the moss along its
 * foot, and forcing the two to move together would put the fit out of the
 * keeper's reach.
 */
export function applyInsetDelta(
  target: BattleFrame | FrameOverride,
  kind: InsetKey,
  delta: number,
  mirror = true,
): void {
  const current = target[kind] ?? 0;
  target[kind] = Math.min(INSET_MAX, Math.max(0, current + delta));
  if (!mirror) return;
  const opposite = OPPOSITE_INSET[kind];
  const currentOpposite = target[opposite] ?? 0;
  target[opposite] = Math.min(INSET_MAX, Math.max(0, currentOpposite + delta));
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
