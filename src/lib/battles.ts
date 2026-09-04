// Скромные эпические битвы — the shelf of cards.
//
// A card is a work of the house seen from another side. What the room shows
// first is not a price list but a shelf of faces: every card is shown, or a
// person cannot be interested in it. What you hold is marked under it, not
// by turning the others to the wall.
//
// Two ranges that look alike and are not the same thing:
//   * `tier`  — the card's rank, 1..5. A property of the card, set by the keeper.
//   * `level` — the state of one person's copy, 1..5. A property of owning it.
// Nothing on this page may quietly turn one into the other.

import type {
  BattleBadgeShape,
  BattleCard,
  BattleCardKind,
  BattleChannel,
  BattleEvent,
  GestureBody,
  GestureFade,
  GestureTurn,
  GestureWhom,
  Motion,
  MotionGesture,
  MotionOccasion,
  MotionWear,
  BattleFrame,
  BattleFrameMode,
  BattleRules,
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
import type { Lang, TranslationKey } from '$lib/i18n';

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
 * way `grow` counts — it is the inner corner of the box, the one the visible
 * size knob still sits on. The other three sides of that same box are the
 * same two numbers asked from a different edge; `sliceResizeDelta` reads
 * them off here so a sign can never disagree with the handle.
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

export type SliceResizeX = 'left' | 'right';
export type SliceResizeY = 'top' | 'bottom';

/**
 * How a drag on one or two edges of a copy's box turns into grow/nudge.
 *
 * The edge under the pointer follows it; the opposite edge stays planted.
 * That is what "resize from this border" means, and why a single inner-corner
 * handle was never enough: the other three sides of the same box had no way
 * to be asked.
 *
 * The numbers are still the held copy's, copied verbatim onto its linked
 * mates — the same rule a move already uses.
 */
export function sliceResizeDelta(
  kind: SliceKind,
  side: SliceSide,
  xEdge: SliceResizeX | null,
  yEdge: SliceResizeY | null,
  dx: number,
  dy: number,
): Pick<SlicePlace, 'growX' | 'growY' | 'nudgeX' | 'nudgeY'> {
  const x = xEdge ? resizeAxis(kind, side, 'x', xEdge, dx) : { grow: 0, nudge: 0 };
  const y = yEdge ? resizeAxis(kind, side, 'y', yEdge, dy) : { grow: 0, nudge: 0 };
  return { growX: x.grow, nudgeX: x.nudge, growY: y.grow, nudgeY: y.nudge };
}

function resizeAxis(
  kind: SliceKind,
  side: SliceSide,
  axis: 'x' | 'y',
  edge: SliceResizeX | SliceResizeY,
  d: number,
): { grow: number; nudge: number } {
  const axes = SLICE_SIDE_AXES[side];
  const sign = sliceSigns(side);
  const grip = axis === 'x' ? axes.gripX : axes.gripY;
  const fromGrip = edge === grip;
  const growSign = axis === 'x' ? sign.growX : sign.growY;
  const nudgeSign = axis === 'x' ? sign.nudgeX : sign.nudgeY;

  // A medallion is centred on its edge: width/height grows both ways from
  // the midpoint, so planting the far side means the centre has to walk
  // with the pointer at half speed.
  const centred =
    (kind === 'midH' && axis === 'x') || (kind === 'midV' && axis === 'y');
  if (centred) {
    const start = axis === 'x' ? 'left' : 'top';
    return edge === start ? { grow: -d, nudge: d / 2 } : { grow: d, nudge: d / 2 };
  }

  // An edge's LENGTH is taken off both ends. Half the delta goes to grow
  // and half to the shift, so the held end moves 1:1 and the far one stays.
  const dual =
    (kind === 'edgeH' && axis === 'x') || (kind === 'edgeV' && axis === 'y');
  if (dual) {
    return fromGrip
      ? { grow: d / 2, nudge: d / 2 }
      : { grow: -d / 2, nudge: d / 2 };
  }

  // Thickness, a corner, a medallion's band: size is measured from one
  // anchor, so the grip-side is grow alone (what the old handle wrote) and
  // the anchor-side keeps the inner edge planted by writing both.
  if (fromGrip) return { grow: d * growSign, nudge: 0 };
  return { grow: -d * growSign, nudge: d * nudgeSign };
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
 * The line written for this language, and no other.
 *
 * Gazette copy may fall across languages so a Russian-only leaf still reads
 * in English. A card on the shelf must not: an empty `titleEn` used to print
 * as Cyrillic on the English shelf, and a race named only «Шмаг» sat in
 * `nameEn` as if it were English. If the keeper has not written this
 * language, the card is silent in it.
 *
 * Cyrillic sitting in an English field (the old silent copy) is treated as
 * missing, not as English.
 */
function lineInLang(own: string | null | undefined, lang: Lang): string {
  const s = own?.trim() ?? '';
  if (!s) return '';
  if (lang === 'en' && mostlyCyrillic(s)) return '';
  return s;
}

function mostlyCyrillic(s: string): boolean {
  const letters = [...s].filter((ch) => /\p{L}/u.test(ch));
  if (!letters.length) return false;
  const cyr = letters.filter((ch) => /\p{Script=Cyrillic}/u.test(ch)).length;
  return cyr * 2 >= letters.length;
}

export function cardCopy(
  card: BattleCard,
  lang: Lang,
): { title: string; effect: string; lore: string } {
  const ru = lang === 'ru';
  return {
    title: lineInLang(ru ? card.titleRu : card.titleEn, lang),
    effect: lineInLang(ru ? card.effectRu : card.effectEn, lang),
    lore: lineInLang(ru ? card.loreRu : card.loreEn, lang),
  };
}

/** A property in the reader's language, with the other name kept alongside —
 *  the card shows both, the way the keeper's own drawing does. */
export function traitCopy(
  trait: CardTrait,
  lang: Lang,
): { name: string; other: string; text: string } {
  const ru = lang === 'ru';
  const name = lineInLang(ru ? trait.nameRu : trait.nameEn, lang);
  const other = lineInLang(ru ? trait.nameEn : trait.nameRu, ru ? 'en' : 'ru');
  const text = lineInLang(ru ? trait.textRu : trait.textEn, lang);
  return { name, other: name && other && name !== other ? other : '', text };
}

/** The header band: what this is. Kind is a dictionary word, printed elsewhere;
 *  free `type` is no longer the header, and a digit in the field is not a type. */
export function headerCopy(card: BattleCard, lang: Lang): { race: string; type: string } {
  const ru = lang === 'ru';
  const typeRaw = lineInLang(ru ? card.typeRu : card.typeEn, lang);
  const type = /^\d+$/.test(typeRaw) ? '' : typeRaw;
  return {
    race: lineInLang(ru ? card.raceNameRu : card.raceNameEn, lang),
    type,
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
 * and the room must not print "0" where it means "never". A stored `0` is the
 * same silence — Granny's raven feed was `0`, not `null`, and printing it
 * named a coin that is not a price.
 */
export function pricesOf(card: BattleCard): { coin: Coin; amount: number }[] {
  const out: { coin: Coin; amount: number }[] = [];
  if (card.priceDust != null && card.priceDust > 0) {
    out.push({ coin: 'dust', amount: card.priceDust });
  }
  if (card.priceFeed != null && card.priceFeed > 0) {
    out.push({ coin: 'feed', amount: card.priceFeed });
  }
  return out;
}

/** One dictionary word for the header: body, spell, or relic — never the free `type`. */
export function kindLabelKey(
  kind: BattleCardKind,
): 'battlesKindUnit' | 'battlesKindSpell' | 'battlesKindRelic' {
  if (kind === 'spell') return 'battlesKindSpell';
  if (kind === 'relic') return 'battlesKindRelic';
  return 'battlesKindUnit';
}

/**
 * The channel of the ordinary blow. Bodily is the default and stays silent;
 * anything else is a word the reader has not already assumed.
 */
export function channelLabelKey(
  channel: BattleChannel,
): 'battlesChannelMagic' | 'battlesChannelPure' | 'battlesChannelNone' | null {
  if (channel === 'magic') return 'battlesChannelMagic';
  if (channel === 'pure') return 'battlesChannelPure';
  if (channel === 'none') return 'battlesChannelNone';
  return null;
}

export type BodyStatField = 'health' | 'mana' | 'armor' | 'ward' | 'reach' | 'step' | 'mend';

/** i18n keys for the body passport — the scene already owns these words. */
export const BODY_STAT_LABELS = {
  health: 'battlesHealthLabel',
  mana: 'battlesManaLabel',
  armor: 'battleStatArmour',
  ward: 'battleStatWard',
  reach: 'battleStatReach',
  step: 'battleStatStep',
  mend: 'battleStatMend',
} as const satisfies Record<BodyStatField, TranslationKey>;

/**
 * The numbers a person needs to see the body they will play. Zeros stay off
 * the paper, the same way `pricesOf` will not print a coin that is not a price.
 */
export function bodyPassport(
  card: Pick<BattleCard, BodyStatField>,
): { field: BodyStatField; value: number }[] {
  const rows: { field: BodyStatField; value: number }[] = [
    { field: 'health', value: card.health },
    { field: 'mana', value: card.mana },
    { field: 'armor', value: card.armor },
    { field: 'ward', value: card.ward },
    { field: 'reach', value: card.reach },
    { field: 'step', value: card.step },
    { field: 'mend', value: card.mend },
  ];
  return rows.filter((row) => row.value);
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

/**
 * Куда вести человека, чтобы он закрыл это поручение.
 *
 * Выбирается по УСЛОВИЮ, а не по slug'у: поручения заводит хранитель, и адрес,
 * привязанный к имени, перестанет работать на первом же новом поручении — а
 * условий всего тринадцать, и они живут в коде.
 *
 * `null` — «здесь и есть»: карты берут и поднимают на самой полке, и ссылка на
 * страницу, на которой человек стоит, — это ссылка в никуда.
 */
export function errandHref(rule: string): string | null {
  switch (rule) {
    case 'works_seen':
    case 'works_liked':
    case 'comments_left':
    case 'bookings_done':
    case 'orders_made':
      return '/figurines';
    case 'tales_read':
      return '/tales';
    case 'deck_laid':
      return '/battles/table';
    case 'matches_finished':
    case 'matches_won':
    case 'challenges_won':
      return '/battles/etude';
    default:
      return null;
  }
}

// ── Движения ─────────────────────────────────────────────────────────────────
//
// Чем показывается удар, чара, выстрел и лечение. ТЗ — `BATTLE-MOTION.md`.
//
// Здесь ОДИН отрисовщик на всё: `stage()` возвращает и стиль каждого рисунка, и
// стиль каждого шевелящегося тела, готовыми строками. Сцена и стол хранителя
// делают из этого один `{#each}` и два `style=` — ровно как `carvedCopies()`
// для резьбы рамы, и по той же причине: второй отрисовщик — это превью,
// которое однажды соврёт.

export const MOTION_OCCASIONS: MotionOccasion[] = [
  'blow',
  'spell',
  'mend',
  'arrive',
  'fall',
  'unseen',
];
export const GESTURE_WHOMS: GestureWhom[] = ['striker', 'target', 'flight', 'field'];
export const GESTURE_BODIES: GestureBody[] = [
  'none',
  'lunge',
  'flinch',
  'shiver',
  'sink',
  'rise',
  'swell',
  'bow',
  'draw',
  'recoil',
  'heave',
  'shudder',
  'sway',
  'loom',
  'kindle',
  'blanch',
  'wither',
];

/** Жесты света. Меняют `filter`, а не `transform`, — значит, их можно дать
 *  телу ВМЕСТЕ с движением, и они сложатся. Список нужен столу: он подсказывает
 *  хранителю, какой жест не отменит уже надетый. */
export const GESTURE_LIGHTS: GestureBody[] = ['kindle', 'blanch', 'wither'];
/** Замахи. Пишут `transform`, поэтому на одном теле живёт только один. */
export const GESTURE_MOVES: GestureBody[] = GESTURE_BODIES.filter(
  (b) => b !== 'none' && !GESTURE_LIGHTS.includes(b),
);

export const isLight = (body: GestureBody) => GESTURE_LIGHTS.includes(body);
export const isMove = (body: GestureBody) =>
  body !== 'none' && !GESTURE_LIGHTS.includes(body);

/** Полёт и поле без картинки — слот под стрелу, не пустой жест. */
export const isSlot = (g: MotionGesture) =>
  (g.whom === 'flight' || g.whom === 'field') && !g.image;

/**
 * Два замаха на одном теле не сложатся — победит последний. Свет складывается
 * с замахом, но не со вторым светом. Стол и сервер держат одно правило.
 */
export function oneStirPerBody(gestures: MotionGesture[]): MotionGesture[] {
  const last = (whom: GestureWhom, pred: (b: GestureBody) => boolean) => {
    for (let i = gestures.length - 1; i >= 0; i--) {
      if (gestures[i].whom === whom && pred(gestures[i].body)) return i;
    }
    return -1;
  };
  const sm = last('striker', isMove);
  const tm = last('target', isMove);
  const sl = last('striker', isLight);
  const tl = last('target', isLight);
  return gestures.filter((g, i) => {
    if (g.whom !== 'striker' && g.whom !== 'target') return true;
    if (isMove(g.body)) return i === (g.whom === 'striker' ? sm : tm);
    if (isLight(g.body)) return i === (g.whom === 'striker' ? sl : tl);
    return true;
  });
}
export const GESTURE_TURNS: GestureTurn[] = ['none', 'toTarget', 'mirror'];
export const GESTURE_FADES: GestureFade[] = ['hold', 'in', 'out', 'inOut'];

/** Потолок длительности. Тот же, что на сервере: ход хранителя из трёх
 *  действий обязан укладываться в две-три секунды, а этюд переигрывают. */
export const MOTION_MS_MAX = 1200;
export const MOTION_FRAMES_MAX = 24;
export const GESTURES_MAX = 12;
export const MOTIONS_MAX = 48;
export const GESTURE_SIZE_MAX = 300;
export const GESTURE_NUDGE_MAX = 200;
export const GESTURE_LAYERS = 12;

/** Отношение сторон клетки на доске: 3 в ширину, 4 в высоту
 *  (`BATTLE-SCENE.md` §10.1). Нужно ровно затем, чтобы стрела летела под тем
 *  углом, под каким её видит глаз, а не под тем, какой у клеток в координатах
 *  движка: угол в клетках и угол на экране — разные числа. */
const CELL_TALL = 4 / 3;

export function newGesture(whom: GestureWhom = 'striker'): MotionGesture {
  return {
    whom,
    body: whom === 'flight' || whom === 'field' ? 'none' : 'lunge',
    image: '',
    frames: 1,
    size: 60,
    nudgeX: 0,
    nudgeY: 0,
    at: 0,
    dur: 300,
    turn: whom === 'flight' ? 'toTarget' : 'none',
    fade: whom === 'flight' ? 'hold' : 'inOut',
    layer: 5,
    strip: [],
  };
}

export function newMotion(occasion: MotionOccasion = 'blow'): Motion {
  return {
    id:
      typeof crypto !== 'undefined' && 'randomUUID' in crypto
        ? crypto.randomUUID()
        : `m${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`,
    nameEn: '',
    nameRu: '',
    occasion,
    span: 0,
    gestures: [newGesture('striker')],
  };
}

function gesture(
  whom: GestureWhom,
  body: GestureBody,
  at: number,
  dur: number,
): MotionGesture {
  return { ...newGesture(whom), body, at, dur, fade: 'hold' };
}

/** Слот под картинку. Без неё ничего не рисуется, но место живёт в записи. */
export function newSlot(whom: 'flight' | 'field', at = 80, dur = 320): MotionGesture {
  return {
    ...newGesture(whom),
    at,
    dur,
    image: '',
    body: 'none',
    size: whom === 'flight' ? 45 : 80,
    fade: 'inOut',
  };
}

/**
 * Промах: удар не взял. Не тот же flinch, что у раны — иначе оберег дрожит
 * как раненый. Свет без движения: тело стоит, краска уходит.
 */
export const WARD_MOTION: Motion = {
  id: 'house-ward',
  nameEn: 'A ward',
  nameRu: 'Оберег',
  occasion: 'unseen',
  gestures: [gesture('target', 'blanch', 0, 280)],
};

/**
 * Умолчания дома — и доказательство, что комната не изменилась.
 *
 * Числа сверены с `BATTLE-SCENE.md` §6 и с тем, что стояло в сцене до движка:
 * подача 220 + 180 = 400, вздрагивание 160 с 220-й миллисекунды, лечение 300,
 * выставление 300, падение 500. Совпадает до миллисекунды.
 */
export const DEFAULT_MOTIONS: Motion[] = [
  {
    id: 'house-blow',
    nameEn: 'A blow',
    nameRu: 'Удар',
    occasion: 'blow',
    gestures: [
      // Подача и возврат — ОДИН жест: 220 туда и 180 обратно живут в его
      // собственной кривой, а не в двух записях. Двумя записями хранитель
      // однажды сотрёт вторую и оставит тело поданным.
      gesture('striker', 'lunge', 0, 400),
      gesture('target', 'flinch', 220, 160),
    ],
  },
  {
    id: 'house-mend',
    nameEn: 'Mending',
    nameRu: 'Лечение',
    occasion: 'mend',
    gestures: [gesture('target', 'rise', 0, 300)],
  },
  {
    id: 'house-arrive',
    nameEn: 'Taking the field',
    nameRu: 'Выставление',
    occasion: 'arrive',
    gestures: [gesture('striker', 'swell', 0, 300)],
  },
  {
    id: 'house-fall',
    nameEn: 'Falling',
    nameRu: 'Падение',
    occasion: 'fall',
    gestures: [gesture('target', 'sink', 0, 500)],
  },
  {
    id: 'house-unseen',
    nameEn: 'No author',
    nameRu: 'Без автора',
    occasion: 'unseen',
    gestures: [gesture('target', 'shiver', 0, 160)],
  },
];

/** Конец последнего жеста, без тишины после него. */
export function motionBars(motion: Motion | null): number {
  if (!motion || !motion.gestures.length) return 0;
  return Math.min(
    MOTION_MS_MAX,
    Math.max(...motion.gestures.map((g) => (g.at || 0) + (g.dur || 0))),
  );
}

export function motionSpan(motion: Motion | null): number {
  if (!motion) return 0;
  return Math.min(MOTION_MS_MAX, Math.max(motionBars(motion), motion.span || 0));
}

export function motionTitle(motion: Motion, lang: Lang): string {
  const own = lang === 'ru' ? motion.nameRu : motion.nameEn;
  return own || motion.nameRu || motion.nameEn || motion.id;
}

export function parseMotionWear(raw: string | null | undefined): MotionWear {
  if (!raw) return {};
  try {
    const found = JSON.parse(raw) as MotionWear;
    return found && typeof found === 'object' ? found : {};
  } catch {
    return {};
  }
}

/** Пустой наряд — это отсутствие наряда, а не `{}` в базе. */
export function stringifyMotionWear(wear: MotionWear): string | null {
  const kept: MotionWear = {};
  for (const occasion of MOTION_OCCASIONS) {
    const id = wear[occasion]?.trim();
    if (id) kept[occasion] = id;
  }
  return Object.keys(kept).length ? JSON.stringify(kept) : null;
}

/**
 * Повод — ради чего играется движение. Читается ТОЛЬКО из события.
 *
 * Ни одного сравнения правил здесь нет и быть не должно (`BATTLE-SCENE.md`
 * §11.10): «стрелок» не выводится из дальности, потому что копейщик с
 * дальностью 2 не стреляет. Стрелок — это карта, которой хранитель надел на
 * повод `blow` движение с летящим жестом, и знать про это движку незачем.
 */
export function occasionOf(event: BattleEvent): MotionOccasion | null {
  if ('played' in event) return 'arrive';
  if ('died' in event) return 'fall';
  if ('healed' in event) return event.healed.by == null ? 'unseen' : 'mend';
  if ('damaged' in event || 'immune' in event) {
    const by = 'damaged' in event ? event.damaged.by : event.immune.by;
    if (by == null) return 'unseen';
    // `source` приходит в событии готовым словом — это не вывод правила, а
    // чтение того, что движок уже сказал.
    return 'damaged' in event && event.damaged.source === 'ability' ? 'spell' : 'blow';
  }
  return null;
}

/**
 * Какое движение играется на этом поводе у этой карты.
 *
 * Цепочка буква в букву та же, что у наряда (`frameForCard`): карта → раса →
 * дом. Вторую цепочку хранителю пришлось бы держать в голове отдельно.
 *
 * Имя, которого в своде нет, молча уступает умолчанию: свод и карты сохраняются
 * порознь, и движение, стёртое из ящика, не должно ронять карту.
 */
export function motionFor(
  occasion: MotionOccasion,
  card: BattleCard | null | undefined,
  motions: Motion[] | null | undefined,
): Motion | null {
  const drawer = motions?.length ? motions : [];
  const found = (id: string | undefined): Motion | null =>
    (id && drawer.find((m) => m.id === id)) || null;

  const own = parseMotionWear(card?.motionWear);
  const kin = parseMotionWear(card?.raceMotionWear);
  const chosen = found(own[occasion]) ?? found(kin[occasion]);
  if (chosen) return chosen;

  // Чара, которой карта не назвала, показывается ударом: у большинства карт
  // способность — это тот же замах, и заводить ей отдельную запись ради того,
  // чтобы она выглядела как удар, незачем.
  if (occasion === 'spell') {
    const asBlow = found(own.blow) ?? found(kin.blow);
    if (asBlow) return asBlow;
  }

  const houseOccasion: MotionOccasion = occasion === 'spell' ? 'blow' : occasion;
  return DEFAULT_MOTIONS.find((m) => m.occasion === houseOccasion) ?? null;
}

// ── Отрисовка движения ───────────────────────────────────────────────────────

/** Где на доске стоит клетка. Координаты движка, а не экрана: `along`
 *  разворачивает их здесь, в одном месте. */
export interface StageSpot {
  x: number;
  y: number;
}

export interface StagedMote {
  key: string;
  layer: number;
  /** Готовый инлайновый стиль: коробка, картинка, полоса, слой, поворот. */
  style: string;
  /** Обломок бумаги — не картинка со склада, а кусок самой карты. */
  kind?: 'scrap';
}

export interface Staged {
  /** Сколько всё это длится. Столько сцена и ждёт — не константу. */
  span: number;
  /** Стиль для тела бьющего. Пусто — оно не шевелится. */
  striker: string;
  target: string;
  motes: StagedMote[];
}

/** Синяк на фото или чернильная блоха. Не полоска здоровья: живёт только такт. */
export type StruckKind = 'bruise' | 'ink';

/**
 * Чем этот удар оставляет след на карте. Числа — из события, не из правил:
 * `remain` после переписи, `blow` = toHealth/max, `seed` — id тела, чтобы
 * обломок летел туда же при переигрывании.
 */
export interface HitWear {
  remain: number;
  blow: number;
  seed: number;
  channel: BattleChannel;
  /** Откуда удар. Яд и зона — не синяк и не бумага. */
  source?: string;
  /** Когда касается — та же миллисекунда, что `motionContact`. */
  at: number;
}

export function struckOf(hit: HitWear | null | undefined): StruckKind | null {
  if (!hit || hit.blow <= 0) return null;
  const src = hit.source ?? 'attack';
  if (src === 'dot' || src === 'zone') return null;
  if (hit.channel === 'physical') return 'bruise';
  if (hit.channel === 'magic' || hit.channel === 'pure') return 'ink';
  return null;
}

/** Когда движение касается цели: первый жест на ней, иначе сразу. */
export function motionContact(motion: Motion | null): number {
  const aimed = motion?.gestures.filter((g) => g.whom === 'target') ?? [];
  return aimed.length ? Math.min(...aimed.map((g) => g.at || 0)) : 0;
}

/**
 * Когда удар уже состоялся — синяк, обломок, перепись здоровья.
 *
 * Если на цели лежит полоса (секира, меч, булава), это не появление оружия, а
 * кадр удара: вторая половина полосы. Одиночная картина бьёт раньше — в
 * касании замаха, не в конце.
 */
export function motionWound(motion: Motion | null): number {
  if (!motion) return 0;
  const pictured = motion.gestures.filter((g) => g.whom === 'target' && g.image);
  if (pictured.length) {
    const g = pictured.reduce((a, b) => ((a.at || 0) <= (b.at || 0) ? a : b));
    const frames = Math.max(1, g.frames || 1);
    const hit = frames > 1 ? 0.62 : 0.45;
    return Math.min(MOTION_MS_MAX, (g.at || 0) + Math.round((g.dur || 0) * hit));
  }
  return motionContact(motion);
}

/** Детерминированный 0..1. Не Math.random: этюд переигрывают. */
function wearRng(seed: number): () => number {
  let a = (seed | 0) + 0x9e3779b9;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

type PaperEdge = 'top' | 'right' | 'bottom' | 'left';

export interface PaperBite {
  edge: PaperEdge;
  t: number;
  w: number;
  depth: number;
}

/** Выщербы края. Глубокие нарочно: на клетке 3 % — это два пикселя, их нет. */
export function paperBites(remain: number, seed: number): PaperBite[] {
  const missing = 1 - Math.max(0, Math.min(1, remain));
  if (missing < 0.02) return [];
  const rng = wearRng(seed);
  const n = Math.min(5, 1 + Math.floor(missing * 5));
  const depth = 11 + missing * 16;
  const edges: PaperEdge[] = ['top', 'right', 'bottom', 'left'];
  const bites: PaperBite[] = [];
  for (let i = 0; i < n; i++) {
    const edge = edges[Math.floor(rng() * 4)]!;
    const t = 0.18 + rng() * 0.64;
    const w = 0.07 + rng() * 0.05 + missing * 0.05;
    if (bites.some((b) => b.edge === edge && Math.abs(b.t - t) < 0.14)) continue;
    bites.push({ edge, t, w, depth });
  }
  return bites;
}

/**
 * Надорванный край карты. Доля оставшегося здоровья — сколько выщербов,
 * `seed` — где они стоят, чтобы лечение снимало те же, а не рисовало новые.
 * Целая карта — `null`, клипа нет.
 */
export function paperClip(remain: number, seed: number): string | null {
  const bites = paperBites(remain, seed);
  if (!bites.length) return null;
  const on = (edge: PaperEdge) =>
    bites.filter((b) => b.edge === edge).sort((a, b) => a.t - b.t);

  const pts: string[] = [];
  const add = (x: number, y: number) => pts.push(`${x.toFixed(2)}% ${y.toFixed(2)}%`);
  add(0, 0);
  for (const b of on('top')) {
    add((b.t - b.w) * 100, 0);
    add(b.t * 100, b.depth);
    add((b.t + b.w) * 100, 0);
  }
  add(100, 0);
  for (const b of on('right')) {
    add(100, (b.t - b.w) * 100);
    add(100 - b.depth, b.t * 100);
    add(100, (b.t + b.w) * 100);
  }
  add(100, 100);
  for (const b of [...on('bottom')].reverse()) {
    add((b.t + b.w) * 100, 100);
    add(b.t * 100, 100 - b.depth);
    add((b.t - b.w) * 100, 100);
  }
  add(0, 100);
  for (const b of [...on('left')].reverse()) {
    add(0, (b.t + b.w) * 100);
    add(b.depth, b.t * 100);
    add(0, (b.t - b.w) * 100);
  }
  return `polygon(${pts.join(',')})`;
}

/** Полёт одного обломка, в долях карты (`cqi`). */
export function scrapFlight(
  blow: number,
  remain: number,
  seed: number,
): { size: number; x: string; y: string; spin: string } {
  const b = Math.max(0.08, Math.min(1, blow));
  const r = Math.max(0, Math.min(1, remain));
  const rng = wearRng(seed + 17);
  const ang = rng() * Math.PI * 2;
  const dist = 36 + (1 - r) * 48;
  return {
    size: 20 + b * 22,
    x: `${(Math.cos(ang) * dist).toFixed(1)}cqi`,
    y: `${(Math.sin(ang) * dist).toFixed(1)}cqi`,
    spin: `${(rng() * 140 - 40).toFixed(0)}deg`,
  };
}

export interface ScrapFly {
  blow: number;
  remain: number;
  seed: number;
}

const EMPTY_STAGE: Staged = { span: 0, striker: '', target: '', motes: [] };

/** Кривая подачи. Та же, что была написана в сцене до движка. */
const EASE = 'cubic-bezier(0.2, 0.8, 0.25, 1)';

/**
 * Полоса кадров, посчитанная точно.
 *
 * `background-position-x: p%` при ширине картинки в `n` ширин коробки сдвигает
 * её на `(1 − n)·p` коробок, то есть кадр `k` стоит при `p = k/(n−1)`. А
 * `steps(n)` от нуля до `E` выдаёт значения `k·E/n`. Значит `E = 100·n/(n−1)`,
 * и никакое другое: с привычным «до 100%» кадры разъезжаются на всём, что
 * длиннее двух, и полоса из восьми показывает семь с половиной.
 */
function stripEnd(frames: number): number {
  return frames > 1 ? (100 * frames) / (frames - 1) : 0;
}

/** Сколько клеток в сборщике полосы. Дом рисует удар шестью кадрами; CSS
 *  делит картинку ровно на столько частей, и зазора между ними нет. */
export const STRIP_FRAMES = 6;
/** Сторона клетки при ширине полосы 1536 — под порогом ужимания 1600. */
export const STRIP_SIDE = 256;
export const STRIP_TURN_MAX = 180;
export const STRIP_SCALE_MAX = 250;
export const STRIP_POSE_MAX = 80;

/** Клетка сборщика: исходник и поза. Движок играет слепок; стол правит это. */
export type StripCell = {
  src: string | null;
  turn: number;
  size: number;
  x: number;
  y: number;
};

export function blankStripCell(): StripCell {
  return { src: null, turn: 0, size: 100, x: 0, y: 0 };
}

function loadStripImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    if (!src.startsWith('blob:') && !src.startsWith('data:')) {
      img.crossOrigin = 'anonymous';
    }
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(src));
    img.src = src;
  });
}

// Порог бумаги — те же числа, что `sheet.rs` (`bg_value` 0.62, `bg_sat` 0.20).
// Чёрное поле (Kling) — не яркость в альфу: золото тогда станет дыркой.
// Семя — почти чистый чёрный, дошедший до края; кайма 2 px съедает
// сглаживание (v≤14). Шире — тёмный шар последнего кадра уходит вместе с полем.
const STRIP_PALE_V = 0.62;
const STRIP_DARK_V = 2 / 255;
const STRIP_FRINGE_V = 14 / 255;
const STRIP_FRINGE_R = 2;
const STRIP_BG_SAT = 0.2;

function stripValueSat(r: number, g: number, b: number): { v: number; s: number } {
  const R = r / 255;
  const G = g / 255;
  const B = b / 255;
  const v = Math.max(R, G, B);
  const lo = Math.min(R, G, B);
  const s = v > 0 ? (v - lo) / v : 0;
  return { v, s };
}

function isStripGround(r: number, g: number, b: number, a: number): boolean {
  if (a < 250) return true;
  const { v, s } = stripValueSat(r, g, b);
  if (s > STRIP_BG_SAT) return false;
  return v >= STRIP_PALE_V || v <= STRIP_DARK_V;
}

function isStripFringe(r: number, g: number, b: number, a: number): boolean {
  if (a < 250) return true;
  const { v, s } = stripValueSat(r, g, b);
  return s <= STRIP_BG_SAT && v <= STRIP_FRINGE_V;
}

/**
 * Снять поле с готовой полосы так же, как разрез снимает бумагу с листа:
 * бледное (или почти чёрное) только если оно ДОХОДИТ ДО КРАЯ холста.
 * Блик внутри самоцвета края не касается и остаётся камнем.
 *
 * Полоса, у которой уже есть своя альфа, не трогается — как лист, который
 * разрез берёт на слово.
 */
export async function punchStripGround(file: File): Promise<File> {
  const src = URL.createObjectURL(file);
  try {
    const img = await loadStripImage(src);
    const canvas = document.createElement('canvas');
    canvas.width = img.width;
    canvas.height = img.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('canvas');
    ctx.drawImage(img, 0, 0);
    const pix = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const { data, width: w, height: h } = pix;
    const n = w * h;

    let sampled = 0;
    let translucent = 0;
    for (let i = 0; i < n; i += 37) {
      sampled += 1;
      if (data[i * 4 + 3] < 250) translucent += 1;
    }
    if (sampled > 0 && translucent * 100 > sampled) return file;

    const ground = new Uint8Array(n);
    for (let i = 0; i < n; i++) {
      const o = i * 4;
      ground[i] = isStripGround(data[o], data[o + 1], data[o + 2], data[o + 3]) ? 1 : 0;
    }

    const seen = new Uint8Array(n);
    const q = new Int32Array(n);
    let head = 0;
    let tail = 0;
    const push = (i: number) => {
      if (i < 0 || i >= n || seen[i] || !ground[i]) return;
      seen[i] = 1;
      q[tail++] = i;
    };
    for (let x = 0; x < w; x++) {
      push(x);
      push((h - 1) * w + x);
    }
    for (let y = 0; y < h; y++) {
      push(y * w);
      push(y * w + w - 1);
    }
    while (head < tail) {
      const i = q[head++];
      const x = i % w;
      const y = (i - x) / w;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          if (dx === 0 && dy === 0) continue;
          const nx = x + dx;
          const ny = y + dy;
          if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
          push(ny * w + nx);
        }
      }
    }
    const punched = seen.slice();
    const fringeR = STRIP_FRINGE_R;
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = y * w + x;
        if (punched[i]) continue;
        const o = i * 4;
        if (!isStripFringe(data[o], data[o + 1], data[o + 2], data[o + 3])) continue;
        let near = false;
        for (let yy = Math.max(0, y - fringeR); yy <= Math.min(h - 1, y + fringeR) && !near; yy++) {
          for (let xx = Math.max(0, x - fringeR); xx <= Math.min(w - 1, x + fringeR); xx++) {
            if (punched[yy * w + xx]) {
              near = true;
              break;
            }
          }
        }
        if (near) seen[i] = 1;
      }
    }
    for (let i = 0; i < n; i++) {
      if (seen[i]) data[i * 4 + 3] = 0;
    }
    ctx.putImageData(pix, 0, 0);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/png'));
    if (!blob) throw new Error('punch');
    const stem = file.name.replace(/\.[^.]+$/, '') || 'strip';
    return new File([blob], `${stem}.png`, { type: 'image/png' });
  } finally {
    URL.revokeObjectURL(src);
  }
}

/** Разрезать готовую полосу обратно на кадры — чтобы клетку можно было сменить. */
export async function splitMotionStrip(
  src: string,
  count = STRIP_FRAMES,
): Promise<string[]> {
  const img = await loadStripImage(src);
  const n = Math.max(2, count);
  const sw = img.width / n;
  const sh = img.height;
  const out: string[] = [];
  for (let i = 0; i < n; i++) {
    const canvas = document.createElement('canvas');
    canvas.width = Math.max(1, Math.round(sw));
    canvas.height = Math.max(1, Math.round(sh));
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('canvas');
    ctx.drawImage(img, i * sw, 0, sw, sh, 0, 0, canvas.width, canvas.height);
    out.push(canvas.toDataURL('image/png'));
  }
  return out;
}

/** Шесть картинок встык, без зазора, каждая со своей позой. */
export async function stitchMotionStrip(
  cells: StripCell[],
  count = STRIP_FRAMES,
): Promise<Blob> {
  const canvas = document.createElement('canvas');
  canvas.width = STRIP_SIDE * count;
  canvas.height = STRIP_SIDE;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('canvas');
  const imgs = await Promise.all(
    cells.slice(0, count).map((c) => (c.src ? loadStripImage(c.src) : Promise.resolve(null))),
  );
  for (let i = 0; i < count; i++) {
    const cell = cells[i];
    const img = imgs[i];
    if (!cell?.src || !img) continue;
    const fit = Math.min(STRIP_SIDE / img.width, STRIP_SIDE / img.height);
    const w = img.width * fit;
    const h = img.height * fit;
    ctx.save();
    ctx.beginPath();
    ctx.rect(i * STRIP_SIDE, 0, STRIP_SIDE, STRIP_SIDE);
    ctx.clip();
    ctx.translate(
      i * STRIP_SIDE + STRIP_SIDE / 2 + (cell.x / 100) * STRIP_SIDE,
      STRIP_SIDE / 2 + (cell.y / 100) * STRIP_SIDE,
    );
    ctx.rotate((cell.turn * Math.PI) / 180);
    const s = (cell.size || 100) / 100;
    ctx.scale(s, s);
    ctx.drawImage(img, -w / 2, -h / 2, w, h);
    ctx.restore();
  }
  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob(resolve, 'image/png'),
  );
  if (!blob) throw new Error('strip');
  return blob;
}

function fadeName(fade: GestureFade): string {
  return fade === 'hold' ? '' : `gotiga-fade-${fade}`;
}

/**
 * Всё, что надо нарисовать и пошевелить ради одного события.
 *
 * `from` — клетка бьющего, `to` — клетка цели; любая может отсутствовать (у
 * урона без автора бьющего нет). `along` — стол лежит вдоль комнаты, и тогда
 * ось глубины идёт по экрану вширь. `calm` — `prefers-reduced-motion`: не
 * украшение, а обязательство, и здесь оно означает пустую сцену, а не быструю.
 */
export function stage(
  motion: Motion | null,
  from: StageSpot | null,
  to: StageSpot | null,
  opts: {
    spanX: number;
    spanY: number;
    along: boolean;
    calm: boolean;
    hit?: HitWear | null;
    hold?: number | null;
  },
): Staged {
  if (!motion || opts.calm) return EMPTY_STAGE;
  const { spanX, spanY, along } = opts;

  // Остановленное время. Стол хранителя правит движение не проигрыванием, а
  // остановкой: «покажи 270-ю миллисекунду и держи». Считать вторую, статичную
  // раскладку было бы вторым отрисовщиком, а он однажды соврёт, — поэтому
  // остановка делается той же строкой `animation`: задержка каждого жеста
  // сдвигается на `-hold`, и всё ставится на паузу. Жест, до которого время
  // ещё не дошло, остаётся с положительной задержкой и показывает свой нулевой
  // кадр — это и есть `both`, и это верно.
  const held = opts.hold ?? null;
  const frozen = held !== null;
  const lag = (at: number) => (frozen ? at - (held as number) : at);

  // Экранные координаты клеток. Разворот живёт здесь и только здесь.
  const screen = (spot: StageSpot | null) =>
    spot ? { x: along ? spot.y : spot.x, y: along ? spot.x : spot.y } : null;
  const a = screen(from);
  const b = screen(to);

  // Направление подачи — в процентах от фигуры, как и было в сцене: треть
  // клетки в сторону цели. Единственная арифметика правил здесь — рисование.
  let lx = 0;
  let ly = 0;
  let angle = 0;
  if (a && b) {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const len = Math.max(1, Math.abs(dx) + Math.abs(dy));
    lx = (dx / len) * 33;
    ly = (dy / len) * 33;
    // Угол берётся на ЭКРАНЕ, а не в клетках: клетка 3:4, и стрела, повёрнутая
    // по координатам движка, летит мимо собственной цели.
    angle = (Math.atan2(dy * CELL_TALL, dx) * 180) / Math.PI;
  }

  const stir: Record<'striker' | 'target', string[]> = { striker: [], target: [] };
  const motes: StagedMote[] = [];
  let key = 0;

  for (const g of motion.gestures) {
    const at = Math.max(0, g.at || 0);
    const dur = Math.max(0, g.dur || 0);

    if (g.body && g.body !== 'none' && (g.whom === 'striker' || g.whom === 'target')) {
      // Несколько шевелений одного тела складываются в один список анимаций —
      // так их и записывает CSS. Если две из них двигают одно и то же, побеждает
      // последняя: это предсказуемо и это же видно в списке жестов.
      stir[g.whom].push(`gotiga-${g.body} ${dur}ms ${EASE} ${lag(at)}ms both`);
    }

    if (!g.image) continue;

    const frames = Math.max(1, Math.min(MOTION_FRAMES_MAX, g.frames || 1));
    const size = Math.max(0, g.size || 0);
    // Коробка рисунка в долях ПОЛЯ: клетка — это `100/spanX` его ширины и
    // `100/spanY` его высоты, и величина жеста задана в процентах клетки.
    const w = size / spanX;
    const h = size / spanY;
    const spot = g.whom === 'target' ? b : a;

    const parts: string[] = ['position:absolute'];
    const anims: string[] = [];

    if (g.whom === 'field') {
      parts.push('inset:0');
    } else if (!spot) {
      // Некому и не над кем: жест просто не выходит. Не ошибка — обычный урон
      // без автора.
      continue;
    } else {
      const cx = ((spot.x + 0.5) / spanX) * 100 + g.nudgeX / spanX;
      const cy = ((spot.y + 0.5) / spanY) * 100 + g.nudgeY / spanY;
      parts.push(`left:${(cx - w / 2).toFixed(3)}%`, `top:${(cy - h / 2).toFixed(3)}%`);
      parts.push(`width:${w.toFixed(3)}%`, `height:${h.toFixed(3)}%`);
    }

    parts.push(`background-image:url("${cssUrl(g.image)}")`);
    parts.push('background-repeat:no-repeat');
    if (frames > 1) {
      parts.push(`background-size:${frames * 100}% 100%`);
      parts.push(`--strip-end:${stripEnd(frames).toFixed(4)}%`);
      anims.push(`gotiga-strip ${dur}ms steps(${frames}) ${lag(at)}ms both`);
    } else {
      parts.push('background-size:contain', 'background-position:center');
    }

    const turn =
      g.turn === 'toTarget' ? `${angle.toFixed(2)}deg` : g.turn === 'mirror' ? '180deg' : '0deg';
    parts.push(`--turn:${turn}`);

    if (g.whom === 'flight' && a && b) {
      // Перелёт задаётся в процентах СОБСТВЕННОЙ ширины рисунка: проценты в
      // `translate` меряются по самому элементу, а не по полю. Клетка по
      // экрану — это `100/size` его ширин, значит клетка пути — `10000/size`
      // процентов. Точно, а не приближённо.
      const own = size > 0 ? 10000 / size : 0;
      parts.push(`--mx:${((b.x - a.x) * own).toFixed(2)}%`);
      parts.push(`--my:${((b.y - a.y) * own).toFixed(2)}%`);
      anims.push(`gotiga-fly ${dur}ms ${EASE} ${lag(at)}ms both`);
    } else if (g.whom === 'target' && frames === 1 && a && b) {
      // Одиночная картина на цели. Полоса уже несёт удар в кадрах; без полосы
      // рисунок иначе просто висит вторым портретом. Замах читается с той же
      // стороны, что подача, и свет на металле — тот же, что kindle: яркость
      // фотографии, не вспышка.
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len = Math.max(1, Math.abs(dx) + Math.abs(dy));
      parts.push(`--lx:${((dx / len) * 32).toFixed(2)}%`);
      parts.push(`--ly:${((dy / len) * 32).toFixed(2)}%`);
      parts.push('transform-origin:50% 38%');
      anims.push(`gotiga-cleave ${dur}ms ${EASE} ${lag(at)}ms both`);
    }

    const fading = fadeName(g.fade);
    if (fading) anims.push(`${fading} ${dur}ms linear ${lag(at)}ms both`);

    const layer = Math.max(1, Math.min(GESTURE_LAYERS, g.layer || 1));
    parts.push(`z-index:${layer}`);
    // Поворот ставится ВСЕГДА, а не только когда анимации нет: полоса кадров
    // не трогает `transform`, и рисунок с ней терял бы свой угол. Перелёт свой
    // поворот несёт внутри собственных кадров и потому эту строку перебивает.
    parts.push('transform:rotate(var(--turn))');
    if (anims.length) parts.push(`animation:${anims.join(',')}`);
    if (frozen && anims.length) parts.push('animation-play-state:paused');

    motes.push({ key: `${motion.id}-${key++}`, layer, style: parts.join(';') });
  }

  const dress = (who: 'striker' | 'target') =>
    stir[who].length
      ? `--lx:${lx.toFixed(2)}%;--ly:${ly.toFixed(2)}%;animation:${stir[who].join(',')}` +
        (frozen ? ';animation-play-state:paused' : '')
      : '';

  return {
    span: motionSpan(motion),
    striker: dress('striker'),
    target: dress('target'),
    motes,
  };
}

/**
 * Готовые движения — то, что хранитель берёт и переделывает, а не сочиняет
 * с нуля.
 *
 * Не умолчания и не пресеты: умолчание играется само, когда ничего не надето
 * (`DEFAULT_MOTIONS`), а это — заготовки. Стол кладёт копию в ящик, дальше она
 * обыкновенная запись со своим именем, и дом про неё больше ничего не знает.
 * Отсюда и `id`, который выдаётся при взятии, а не хранится здесь.
 *
 * Три ближних удара несут полосу со склада дома: без рисунка секира, меч и
 * булава — один и тот же замах. Остальные собраны без картинки, замахом и
 * светом, потому что выстрел и чару надо показать уже сегодня, а стрелу
 * хранитель кладёт сам.
 *
 * Чара и проклятие показывают, зачем свет отделён от движения: `sway` двигает
 * `transform`, `kindle` — `filter`, и потому они играют ОДНИМ телом
 * одновременно. Два движения так не сложились бы: победило бы последнее.
 */

/** Полосы ближнего удара. Дом нарисовал их сам; хранитель может заменить. */
export const STRIKE_STRIPS = {
  axe: '/battles/motion/axe.png',
  sword: '/battles/motion/sword.png',
  mace: '/battles/motion/mace.png',
} as const;

function strikeArt(image: string, at: number, dur: number): MotionGesture {
  return {
    ...newGesture('target'),
    body: 'none',
    image,
    frames: STRIP_FRAMES,
    size: 118,
    at,
    dur,
    fade: 'inOut',
    layer: 8,
  };
}

export const STOCK_MOTIONS: { nameEn: string; nameRu: string; occasion: MotionOccasion; gestures: MotionGesture[] }[] = [
  {
    nameEn: 'An axe',
    nameRu: 'Секира',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'heave', 0, 600),
      strikeArt(STRIKE_STRIPS.axe, 180, 480),
      gesture('target', 'recoil', 420, 280),
    ],
  },
  {
    nameEn: 'A sword',
    nameRu: 'Меч',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'lunge', 0, 500),
      strikeArt(STRIKE_STRIPS.sword, 140, 440),
      gesture('target', 'flinch', 340, 200),
    ],
  },
  {
    nameEn: 'A mace',
    nameRu: 'Булава',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'heave', 0, 620),
      strikeArt(STRIKE_STRIPS.mace, 200, 500),
      gesture('target', 'shudder', 440, 280),
    ],
  },
  {
    nameEn: 'A heavy blow',
    nameRu: 'Тяжёлый удар',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'heave', 0, 520),
      // Один замах на цель: recoil и shudder оба пишут transform, и второй
      // убивал первый. Отдача — то, чем тяжёлый удар читается.
      gesture('target', 'recoil', 320, 280),
    ],
  },
  {
    nameEn: 'A shot',
    nameRu: 'Выстрел',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'draw', 0, 420),
      // Слот: без картинки ничего не летит, но место уже есть — кладут стрелу,
      // а не заводят жест. След удара в комнате рисуется отдельно.
      newSlot('flight', 80, 340),
      gesture('target', 'flinch', 400, 160),
    ],
  },
  {
    nameEn: 'A charm',
    nameRu: 'Чара',
    occasion: 'spell',
    gestures: [
      gesture('striker', 'sway', 0, 460),
      gesture('striker', 'kindle', 0, 460),
      newSlot('field', 200, 400),
      gesture('target', 'kindle', 380, 320),
      gesture('target', 'shiver', 380, 200),
    ],
  },
  {
    nameEn: 'A curse',
    nameRu: 'Проклятие',
    occasion: 'spell',
    gestures: [
      gesture('striker', 'loom', 0, 420),
      newSlot('field', 180, 400),
      gesture('target', 'wither', 340, 340),
      gesture('target', 'shudder', 340, 280),
    ],
  },
  {
    nameEn: 'The evil eye',
    nameRu: 'Сглаз',
    occasion: 'blow',
    gestures: [
      gesture('striker', 'sway', 0, 380),
      gesture('target', 'blanch', 260, 380),
    ],
  },
  {
    nameEn: 'Tending',
    nameRu: 'Врачевание',
    occasion: 'mend',
    gestures: [
      gesture('striker', 'bow', 0, 380),
      gesture('target', 'kindle', 200, 340),
      gesture('target', 'rise', 200, 340),
    ],
  },
  {
    nameEn: 'Stepping out',
    nameRu: 'Явление',
    occasion: 'arrive',
    gestures: [
      gesture('striker', 'swell', 0, 340),
      gesture('striker', 'kindle', 60, 320),
    ],
  },
  {
    nameEn: 'Guttering out',
    nameRu: 'Угасание',
    occasion: 'fall',
    gestures: [
      gesture('target', 'sink', 0, 560),
      gesture('target', 'blanch', 0, 560),
    ],
  },
  {
    nameEn: 'Poison',
    nameRu: 'Яд',
    occasion: 'unseen',
    gestures: [
      gesture('target', 'wither', 0, 320),
      gesture('target', 'shiver', 0, 200),
    ],
  },
];

/** Заготовку берут копией: `id` рождается в этот миг, потому что на него сразу
 *  начинают показывать карта, раса и порядок в ящике. */
export function takeStock(index: number): Motion | null {
  const found = STOCK_MOTIONS[index];
  if (!found) return null;
  return {
    ...newMotion(found.occasion),
    nameEn: found.nameEn,
    nameRu: found.nameRu,
    gestures: found.gestures.map((g) => ({ ...g })),
  };
}

/** Умолчание дома — тоже копией: сами пять записей не правят, иначе комната
 *  перестанет быть доказательством, что такт не изменился. */
export function takeHouse(index: number): Motion | null {
  const found = DEFAULT_MOTIONS[index];
  if (!found) return null;
  return {
    ...newMotion(found.occasion),
    nameEn: found.nameEn,
    nameRu: found.nameRu,
    gestures: found.gestures.map((g) => ({ ...g })),
  };
}

// ── Правила испытания ────────────────────────────────────────────────────────
//
// Полка и сцена показывают, чем этот бой отличается от соседнего. Отличие
// считается сравнением с умолчаниями дома, и умолчания приходится держать
// здесь ЗЕРКАЛОМ того, что стоит в `battle_core::Rules::default()`.
//
// Зеркало терпимо ровно потому, что этими числами здесь ничего не играется:
// они выбирают, какую строчку сказать словами. Разъехавшееся зеркало показало
// бы лишнюю строку или промолчало о нужной — и не изменило бы в бою ничего,
// потому что бой считает сервер. Полагаться на него в счёте нельзя.

/** Умолчания дома. Зеркало `Rules::default()`; см. оговорку выше. */
export const HOUSE_RULES: BattleRules = {
  secondSideCoin: 1,
  openingAttacks: 1,
  walkSpendsTurn: false,
  retaliation: false,
  actsPerTurn: 255,
  escalationFrom: 0,
  idleToll: 1,
  maxRounds: 12,
  longShotPower: 25,
  pointBlankPower: 50,
};

/** Одно отличие правил от домашних: чем сказать и какое при нём число. */
export interface RuleApart {
  key: TranslationKey;
  /** Число, которое ставится рядом со словами. `null` — правило без числа. */
  amount: number | null;
}

/**
 * Чем эти правила отличаются от домашних.
 *
 * Названо только отличие, а не весь свод: этюд, у которого перечислены все
 * десять ручек, ничего не сообщает — читатель обязан помнить наизусть, какие
 * из них обычные. Разница же читается с одного взгляда и ровно затем и
 * показывается.
 */
export function rulesApart(rules: BattleRules | null | undefined): RuleApart[] {
  if (!rules) return [];
  const out: RuleApart[] = [];
  const say = (key: TranslationKey, amount: number | null = null) => out.push({ key, amount });

  if (rules.walkSpendsTurn !== HOUSE_RULES.walkSpendsTurn) {
    say(rules.walkSpendsTurn ? 'battleRuleWalkSpends' : 'battleRuleWalkFree');
  }
  if (rules.retaliation !== HOUSE_RULES.retaliation) {
    say(rules.retaliation ? 'battleRuleRetaliation' : 'battleRuleNoRetaliation');
  }
  // 255 — «сколько угодно», то есть каждое тело по разу. Число рядом с этим
  // словом было бы враньём, поэтому и потолок называется, только когда он есть.
  if (rules.actsPerTurn !== HOUSE_RULES.actsPerTurn && rules.actsPerTurn < 255) {
    say('battleRuleActs', rules.actsPerTurn);
  }
  if (rules.openingAttacks !== HOUSE_RULES.openingAttacks) {
    say(rules.openingAttacks >= 255 ? 'battleRuleOpeningFree' : 'battleRuleOpening',
      rules.openingAttacks >= 255 ? null : rules.openingAttacks);
  }
  if (rules.idleToll !== HOUSE_RULES.idleToll) {
    say(rules.idleToll === 0 ? 'battleRuleNoIdleToll' : 'battleRuleIdleToll',
      rules.idleToll === 0 ? null : rules.idleToll);
  }
  if (rules.escalationFrom !== HOUSE_RULES.escalationFrom && rules.escalationFrom > 0) {
    say('battleRuleEscalation', rules.escalationFrom);
  }
  if (rules.maxRounds !== HOUSE_RULES.maxRounds) say('battleRuleRounds', rules.maxRounds);
  if (rules.secondSideCoin !== HOUSE_RULES.secondSideCoin) {
    say('battleRuleCoin', rules.secondSideCoin);
  }
  if (rules.pointBlankPower !== HOUSE_RULES.pointBlankPower) {
    say(rules.pointBlankPower >= 100 ? 'battleRuleNoPointBlank' : 'battleRulePointBlank',
      rules.pointBlankPower >= 100 ? null : rules.pointBlankPower);
  }
  if (rules.longShotPower !== HOUSE_RULES.longShotPower) {
    say(rules.longShotPower === 0 ? 'battleRuleNoLongShot' : 'battleRuleLongShot',
      rules.longShotPower === 0 ? null : rules.longShotPower);
  }
  return out;
}
