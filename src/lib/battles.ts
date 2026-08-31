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
  SliceFit,
  SliceKind,
  SliceOrnament,
  SlicePiece,
  SlicePieces,
  SlicePlace,
  SliceSide,
  SliceSlot,
  SliceTurn,
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

export const SLICE_SLOTS: SliceSlot[] = [
  'corner', 'sideH', 'sideV', 'cornerExtra', 'sideMidH', 'sideMidV',
];
export const SLICE_FITS: SliceFit[] = ['stretch', 'contain', 'cover', 'tile'];
export const SLICE_TURNS: SliceTurn[] = ['mirror', 'rotate', 'none'];
/** How far past its band a copy may reach, in % of the card — wide enough for
 *  a corner to swallow a whole edge band, short of a second card face. */
export const SLICE_GROW_MAX = 40;
/** How many layers the carving has. Wide enough that the list of pieces can
 *  give every one its own: two pieces on one layer fall back to the order they
 *  happen to be written in, which is an order nobody chose and nobody sees. */
export const SLICE_LAYERS = 24;

function place(): SlicePlace {
  return { growX: 0, growY: 0, nudgeX: 0, nudgeY: 0, shown: true };
}

/** Which shape each named slot is. The six were the five shapes all along —
 *  writing it down is what lets an added ornament pick one. */
export const SLICE_KIND: Record<SliceSlot, SliceKind> = {
  corner: 'corner',
  sideH: 'edgeH',
  sideV: 'edgeV',
  cornerExtra: 'corner',
  sideMidH: 'midH',
  sideMidV: 'midV',
};

export const SLICE_KINDS: SliceKind[] = ['corner', 'edgeH', 'edgeV', 'midH', 'midV'];

/** Which copies a shape has. */
export const KIND_SIDES: Record<SliceKind, SliceSide[]> = {
  corner: ['tl', 'tr', 'bl', 'br'],
  edgeH: ['top', 'bottom'],
  edgeV: ['left', 'right'],
  midH: ['top', 'bottom'],
  midV: ['left', 'right'],
};

/** Which copies each named slot has — read through its shape, so the two can
 *  never disagree. */
export const SLICE_SIDES: Record<SliceSlot, SliceSide[]> = {
  corner: KIND_SIDES.corner,
  sideH: KIND_SIDES.edgeH,
  sideV: KIND_SIDES.edgeV,
  cornerExtra: KIND_SIDES.corner,
  sideMidH: KIND_SIDES.midH,
  sideMidV: KIND_SIDES.midV,
};

/**
 * Which two sides of the card a copy hangs off, and which corner of its own box
 * faces the card's inside.
 *
 * ONE table, because everything directional is read off it and two would drift:
 * the anchor decides which way `nudge` counts (always inward, so mirrored
 * copies move together rather than apart), and the grip corner decides which
 * way `grow` counts — and it is also literally where the size grip is drawn.
 */
export const SLICE_SIDE_AXES: Record<
  SliceSide,
  { anchorX: 'left' | 'right'; anchorY: 'top' | 'bottom'; gripX: 'left' | 'right'; gripY: 'top' | 'bottom' }
> = {
  tl: { anchorX: 'left', anchorY: 'top', gripX: 'right', gripY: 'bottom' },
  tr: { anchorX: 'right', anchorY: 'top', gripX: 'left', gripY: 'bottom' },
  bl: { anchorX: 'left', anchorY: 'bottom', gripX: 'right', gripY: 'top' },
  br: { anchorX: 'right', anchorY: 'bottom', gripX: 'left', gripY: 'top' },
  // An edge has one anchored side and one free run. Along the run the anchor is
  // the end the run is measured from, so a nudge reads the same on both edges.
  top: { anchorX: 'left', anchorY: 'top', gripX: 'right', gripY: 'bottom' },
  bottom: { anchorX: 'left', anchorY: 'bottom', gripX: 'right', gripY: 'top' },
  left: { anchorX: 'left', anchorY: 'top', gripX: 'right', gripY: 'bottom' },
  right: { anchorX: 'right', anchorY: 'top', gripX: 'left', gripY: 'bottom' },
};

/** Which way a drag of `dx`/`dy` counts on this copy. Derived from the axes
 *  above rather than written out again: a sign that disagreed with the grip's
 *  own corner would make the grip run away from the pointer. */
export function sliceSigns(side: SliceSide) {
  const axes = SLICE_SIDE_AXES[side];
  return {
    nudgeX: axes.anchorX === 'left' ? 1 : -1,
    nudgeY: axes.anchorY === 'top' ? 1 : -1,
    growX: axes.gripX === 'right' ? 1 : -1,
    growY: axes.gripY === 'bottom' ? 1 : -1,
  };
}

function piece(kind: SliceKind, layer: number, fit: SliceFit = 'stretch'): SlicePiece {
  const places: Partial<Record<SliceSide, SlicePlace>> = {};
  for (const side of KIND_SIDES[kind]) places[side] = place();
  return { layer, fit, turn: 'mirror', linked: true, places };
}

/**
 * The placement every slot has always had, written down as numbers. The two
 * base edges paint OVER the corners because that is the order the pieces stood
 * in the markup before any of this was adjustable; the three accents sit above
 * both and are laid in whole rather than stretched, which is what makes an
 * accent an accent. Every copy starts on its own band, lit, and linked to its
 * mates, so a frame saved before pieces could overlap renders exactly as it did.
 *
 * The server's `battles::default_slices` is the original; change both together.
 */
export function defaultSlices(): SlicePieces {
  return {
    corner: piece('corner', 2),
    sideH: piece('edgeH', 3),
    sideV: piece('edgeV', 3),
    cornerExtra: piece('corner', 5, 'contain'),
    sideMidH: piece('midH', 5, 'contain'),
    sideMidV: piece('midV', 5, 'contain'),
  };
}

/** A flourish the keeper just added: a picture, a shape, and the accents'
 *  own habits — laid in whole, above the assembly. */
export function newOrnament(image: string, kind: SliceKind = 'corner'): SliceOrnament {
  return {
    id: crypto.randomUUID(),
    image,
    kind,
    ...piece(kind, 5, 'contain'),
  };
}

function span(v: unknown): number | null {
  return typeof v === 'number' && Number.isFinite(v)
    ? Math.min(SLICE_GROW_MAX, Math.max(-SLICE_GROW_MAX, v))
    : null;
}

function placesOf(given: SlicePiece | undefined, kind: SliceKind) {
  const places: Partial<Record<SliceSide, SlicePlace>> = {};
  for (const side of KIND_SIDES[kind]) {
    const had = given?.places?.[side];
    places[side] = had && typeof had === 'object'
      ? {
          growX: span(had.growX) ?? 0,
          growY: span(had.growY) ?? 0,
          nudgeX: span(had.nudgeX) ?? 0,
          nudgeY: span(had.nudgeY) ?? 0,
          shown: had.shown !== false,
        }
      : place();
  }
  return places;
}

/** One piece's picture settings and the placement of each of its copies, held
 *  to the same ranges the server holds them to — the admin's preview paints a
 *  frame that has not been saved yet, and the two must agree on what is seen. */
function settle(given: SlicePiece | undefined, kind: SliceKind, base: SlicePiece): SlicePiece {
  if (!given || typeof given !== 'object') return { ...base, places: placesOf(undefined, kind) };
  const layer = Number(given.layer);
  return {
    layer: Number.isFinite(layer) && layer >= 1 && layer <= SLICE_LAYERS
      ? Math.round(layer)
      : base.layer,
    fit: SLICE_FITS.includes(given.fit) ? given.fit : base.fit,
    turn: SLICE_TURNS.includes(given.turn) ? given.turn : base.turn,
    linked: typeof given.linked === 'boolean' ? given.linked : true,
    places: placesOf(given, kind),
  };
}

export function pieceOf(frame: BattleFrame, slot: SliceSlot): SlicePiece {
  return settle(frame.slices?.[slot], SLICE_KIND[slot], defaultSlices()[slot]);
}

/**
 * The same frame with a complete placement on it. `bind:` in the keeper's desk
 * needs real objects to write into, and a frame saved before its pieces could
 * overlap carries none — reading through `pieceOf` is enough to RENDER one, but
 * not to edit one.
 */
export function completeSlices(frame: BattleFrame): BattleFrame {
  const slices = {} as SlicePieces;
  for (const slot of SLICE_SLOTS) slices[slot] = pieceOf(frame, slot);
  const ornaments = (frame.ornaments ?? [])
    .filter((one) => one && one.id && one.image?.trim())
    .map((one) => {
      const kind = SLICE_KINDS.includes(one.kind) ? one.kind : 'corner';
      return { ...one, kind, ...settle(one, kind, defaultSlices().cornerExtra) };
    });
  return { ...frame, slices, ornaments };
}

/** Which upload each named slot draws. A slot with no picture is never placed
 *  and never taken in hand: there would be nothing on the card to see move. */
export function slotArt(frame: BattleFrame, slot: SliceSlot): string {
  return (
    {
      corner: frame.cornerImage,
      sideH: frame.sideImageH,
      sideV: frame.sideImageV,
      cornerExtra: frame.cornerExtra,
      sideMidH: frame.sideMidH,
      sideMidV: frame.sideMidV,
    }[slot] ?? ''
  ).trim();
}

/** Everything a `sliced` frame is built from, named slots and added flourishes
 *  alike, in one list. ONE list is the point: the shelf, the preview and the
 *  keeper's drag all read it, so an ornament is never a second kind of thing
 *  that some of them know about and others do not. */
export interface CarvedPiece {
  /** A named slot, or an ornament's own id. */
  id: string;
  kind: SliceKind;
  image: string;
  piece: SlicePiece;
}

export function carving(frame: BattleFrame): CarvedPiece[] {
  const out: CarvedPiece[] = [];
  for (const slot of SLICE_SLOTS) {
    const image = slotArt(frame, slot);
    if (!image) continue;
    out.push({ id: slot, kind: SLICE_KIND[slot], image, piece: pieceOf(frame, slot) });
  }
  for (const one of frame.ornaments ?? []) {
    const image = one?.image?.trim();
    if (!one?.id || !image) continue;
    const kind = SLICE_KINDS.includes(one.kind) ? one.kind : 'corner';
    out.push({ id: one.id, kind, image, piece: settle(one, kind, defaultSlices().cornerExtra) });
  }
  return out;
}

/** What shape a piece is, named slot or added flourish alike. */
export function kindOf(frame: BattleFrame, id: string): SliceKind | null {
  if ((SLICE_SLOTS as string[]).includes(id)) return SLICE_KIND[id as SliceSlot];
  const found = frame.ornaments?.find((one) => one.id === id);
  if (!found) return null;
  return SLICE_KINDS.includes(found.kind) ? found.kind : 'corner';
}

/** The live piece with this id ON THIS FRAME OBJECT, made if the frame was
 *  saved before pieces could be placed. What a drag writes into — `pieceOf`
 *  returns a reading, this returns the thing itself. */
export function livePiece(frame: BattleFrame, id: string): SlicePiece | null {
  if ((SLICE_SLOTS as string[]).includes(id)) {
    const slot = id as SliceSlot;
    if (!frame.slices) frame.slices = defaultSlices();
    if (!frame.slices[slot]) frame.slices[slot] = pieceOf(frame, slot);
    return frame.slices[slot];
  }
  return frame.ornaments?.find((one) => one.id === id) ?? null;
}

/** How each copy of a piece is drawn, as an inline style.
 *
 * Written here and not as sixteen CSS rules, which is what it was: the six
 * named slots are five shapes between them, an added ornament is one of the
 * same five, and a second renderer for the added ones would be a preview that
 * eventually lies. Percentages throughout, so `left`/`right`/`width` measure
 * against the card's width and `top`/`bottom`/`height` against its height —
 * exactly what the four insets already mean.
 */
export interface CarvedCopy {
  id: string;
  side: SliceSide;
  /** Kept beside the style so a pick-through can sort by it without asking the
   *  frame a second question about a piece it already resolved. */
  layer: number;
  style: string;
}

function boxOf(frame: BattleFrame, kind: SliceKind, side: SliceSide, at: SlicePlace): string {
  const top = frame.insetTop || 0;
  const right = frame.insetRight || 0;
  const bottom = frame.insetBottom || 0;
  const left = frame.insetLeft || 0;
  const { anchorX, anchorY } = SLICE_SIDE_AXES[side];
  // The band this copy starts from: its own two insets.
  const acrossX = anchorX === 'left' ? left : right;
  const acrossY = anchorY === 'top' ? top : bottom;
  if (kind === 'corner') {
    return [
      `${anchorY}:${at.nudgeY}%`,
      `${anchorX}:${at.nudgeX}%`,
      `width:${acrossX + at.growX}%`,
      `height:${acrossY + at.growY}%`,
    ].join(';');
  }
  if (kind === 'edgeH') {
    // `grow` along the run is taken off BOTH ends, so growing it reaches in
    // under the two corners — the join that was impossible while a band was
    // also a boundary.
    return [
      `${anchorY}:${at.nudgeY}%`,
      `height:${acrossY + at.growY}%`,
      `left:${left - at.growX + at.nudgeX}%`,
      `right:${right - at.growX - at.nudgeX}%`,
    ].join(';');
  }
  if (kind === 'edgeV') {
    return [
      `${anchorX}:${at.nudgeX}%`,
      `width:${acrossX + at.growX}%`,
      `top:${top - at.growY + at.nudgeY}%`,
      `bottom:${bottom - at.growY - at.nudgeY}%`,
    ].join(';');
  }
  // A medallion: centred on its edge, and square to the band it rides on, so it
  // reads at the scale of the border rather than at whatever its file happens
  // to be. The centring translate is composed with the mirror below.
  const size = kind === 'midH' ? acrossY : acrossX;
  return kind === 'midH'
    ? [
        `${anchorY}:${at.nudgeY}%`,
        `left:calc(50% + ${at.nudgeX}%)`,
        `width:${size + at.growX}%`,
        `height:${size + at.growY}%`,
      ].join(';')
    : [
        `${anchorX}:${at.nudgeX}%`,
        `top:calc(50% + ${at.nudgeY}%)`,
        `width:${size + at.growX}%`,
        `height:${size + at.growY}%`,
      ].join(';');
}

/** How a copy is turned to reach its side. The FIRST copy of a piece — top-left
 *  corner, lintel, left side — is never turned; the rest are mirrored by
 *  default, so an asymmetric flourish stays right-side up wherever it lands. */
function turnOf(kind: SliceKind, side: SliceSide, turn: SliceTurn): string {
  const centring = kind === 'midH' ? 'translateX(-50%)' : kind === 'midV' ? 'translateY(-50%)' : '';
  const first = KIND_SIDES[kind][0];
  let face = '';
  if (side !== first) {
    if (turn === 'rotate') {
      // Quarter turns run the way a corner round is drawn: clockwise from the
      // top-left, so the piece meets the same two edges it was cut against.
      face = kind === 'corner'
        ? { tr: 'rotate(90deg)', br: 'rotate(180deg)', bl: 'rotate(270deg)' }[side as 'tr' | 'br' | 'bl']
        : 'rotate(180deg)';
    } else if (turn === 'mirror') {
      face = kind === 'corner'
        ? { tr: 'scaleX(-1)', bl: 'scaleY(-1)', br: 'scale(-1, -1)' }[side as 'tr' | 'bl' | 'br']
        : KIND_SIDES[kind][0] === 'top' ? 'scaleY(-1)' : 'scaleX(-1)';
    }
  }
  const both = [centring, face].filter(Boolean).join(' ');
  return both ? `transform:${both}` : '';
}

/** `background-size`/`-repeat`/`-position` for one fit. `tile` is the only one
 *  that has to know which way the band runs: a running vine repeats ALONG its
 *  edge and is scaled across it, never the other way round. */
function fitOf(kind: SliceKind, fit: SliceFit): string {
  if (fit === 'contain') return 'background-size:contain;background-position:center';
  if (fit === 'cover') return 'background-size:cover;background-position:center';
  if (fit === 'tile') {
    // A tile has to start from the band's own anchor, or the repeat would begin
    // mid-picture.
    const along =
      kind === 'edgeH' || kind === 'midH' ? 'background-size:auto 100%;background-repeat:repeat-x'
      : kind === 'edgeV' || kind === 'midV' ? 'background-size:100% auto;background-repeat:repeat-y'
      : 'background-size:auto;background-repeat:repeat';
    return `${along};background-position:left top`;
  }
  return 'background-size:100% 100%;background-position:center';
}

/** Every copy of every piece, ready to render. A copy the keeper put out is
 *  simply not here — an accent over the lintel and nothing on the sill is one
 *  unticked box, not a second upload with half of it erased. */
export function carvedCopies(frame: BattleFrame): CarvedCopy[] {
  const out: CarvedCopy[] = [];
  for (const { id, kind, image, piece: settled } of carving(frame)) {
    const picture = `background-image:url("${cssUrl(image)}")`;
    const paint = `${picture};${fitOf(kind, settled.fit)};background-repeat:no-repeat`;
    for (const side of KIND_SIDES[kind]) {
      const at = settled.places[side];
      if (!at || at.shown === false) continue;
      const style = [
        boxOf(frame, kind, side, at),
        // `fitOf` may set its own repeat; the plain `no-repeat` above it is the
        // default and is overridden by whatever `fitOf` wrote after it.
        paint,
        `z-index:${settled.layer}`,
        turnOf(kind, side, settled.turn),
      ].filter(Boolean).join(';');
      out.push({ id, side, layer: settled.layer, style });
    }
  }
  return out;
}

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
    slices: defaultSlices(),
    ornaments: [],
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

/** A dress worn instead of the tier's own — by one card, or by one level of
 *  a race's copies. Any part of a frame's design may travel: a keeper who
 *  saved a whole frame as a preset means the whole frame, ornaments and paper
 *  and bands together, not just its photograph. What never travels is which
 *  rank a card belongs to or what that rank is called: those are the
 *  dictionary's, and a dress that could rename a rank would be a sixth rank
 *  wearing a disguise.
 *
 *  Every field is optional and only what is present is worn, so a dress made
 *  the old way — a picture and the four insets around its window — still means
 *  exactly what it meant when it was saved. */
export type FrameOverride = Partial<Omit<BattleFrame, 'tier' | 'nameEn' | 'nameRu'>>;

/** A frame taken off and folded into a dress — everything but its rank and
 *  its name, which belong to the dictionary it came from. */
export function dressOf(frame: BattleFrame): FrameOverride {
  const { tier: _tier, nameEn: _nameEn, nameRu: _nameRu, ...dress } = frame;
  return { ...dress };
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

/** Put a dress on a frame — the one merge every layer uses.
 *
 *  Whatever the dress actually names is worn, and nothing else is disturbed:
 *  an old dress naming only a picture leaves the rank's paper, bands and
 *  badges exactly where they were, while a whole frame saved as a preset
 *  replaces all of them. An empty string is a choice, not an absence — a
 *  sliced dress says "no single photograph" by naming `frameImage: ''`, and
 *  reading that as "unset" would leave the rank's old picture underneath. */
function patchFrame(base: BattleFrame, patch: FrameOverride | null): BattleFrame {
  if (!patch) return base;
  const worn = { ...base };
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined || value === null) continue;
    (worn as Record<string, unknown>)[key] = value;
  }
  // Never travels, whatever an old or hand-written dress happens to carry.
  worn.tier = base.tier;
  worn.nameEn = base.nameEn;
  worn.nameRu = base.nameRu;
  return worn;
}

/**
 * The frame this ONE card actually wears, built in three layers:
 *   1. the tier's shared frame
 *   2. the card's race, dressed for this level of an owned copy (if the race
 *      set one for that level — a level nobody dressed keeps the tier's own)
 *   3. this one card's own `frameOverride` — the keeper's most specific
 *      exception, "wear a picture of your own", wins over both of the above
 * A layer wears only what it names — usually a picture and its window, and,
 * when the keeper took a whole saved frame out of the drawer, the whole design.
 * The rank itself and its name are the dictionary's alone at every layer.
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
    !!frame.sideImageV?.trim() ||
    // A frame can be nothing but flourishes, and it is still dressed.
    (frame.ornaments ?? []).some((one) => !!one?.image?.trim())
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
