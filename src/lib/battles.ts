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
  AbilityShape,
  BattleAbilitySnap,
  BattleAction,
  BattleHoldKind,
  BattleUnit,
  RiderStat,
  AbilityTrigger,
  AbilityVerb,
  BattleBadgeShape,
  BattleCard,
  BattleCardKind,
  BattleChannel,
  BattleEvent,
  BattleRace,
  SaveBattleCardRequest,
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
  BattleGround,
  BattleTile,
  BattleField,
  BattleLayout,
  CardAbility,
  CardTrait,
  FreeMark,
  FreeMarkAlign,
  FreeSlot,
  SheetBand,
  SheetRow,
  SheetShow,
  SheetSlot,
  SliceFit,
  SliceKind,
  SliceOrnament,
  SlicePiece,
  SlicePieces,
  SlicePlace,
  SliceSide,
  SliceSlot,
  SliceTurn,
} from "$lib/types/api";
import { fontStack } from "$lib/fonts";
import type { Lang, TranslationKey } from "$lib/i18n";

export const TIERS = [1, 2, 3, 4, 5] as const;

/** The two coins. `dust` settles on its own; `feed` is given by hand. */
export type Coin = "dust" | "feed";

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
/**
 * Где стоят значки стоимости и силы — в долях КАРТЫ, не окна (см.
 * `.badges-layer` в `BattleCard.svelte`).
 *
 * Числа выглядят необязательными, но они не выбраны заново: это ровно те
 * места, куда значки попадали, пока их считали в долях окна с отступом
 * 5cqi, — на карте 5 : 7 с нулевыми врезками. Дом не переехал.
 */
export const DEFAULT_COST_X = 14;
export const DEFAULT_COST_Y = 12;
export const DEFAULT_POWER_X = 86;
export const DEFAULT_POWER_Y = 88;
/** Где встают имя и приписка в `freeform` до первого перетаскивания — имя под
 *  фотографией, приписка в пустом развороте у подножия, там, где его обычно
 *  оставляет готовая иллюстрация в духе сертификата. Зеркало в `battles.rs`,
 *  менять вместе. */
export const DEFAULT_FREE_NAME_X = 50;
export const DEFAULT_FREE_NAME_Y = 60;
export const DEFAULT_FREE_LORE_X = 50;
export const DEFAULT_FREE_LORE_Y = 84;

export const SLICE_SLOTS: SliceSlot[] = [
  "corner",
  "sideH",
  "sideV",
  "cornerExtra",
  "sideMidH",
  "sideMidV",
];
export const SLICE_FITS: SliceFit[] = ["stretch", "contain", "cover", "tile"];
export const SLICE_TURNS: SliceTurn[] = ["mirror", "rotate", "none"];
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
  corner: "corner",
  sideH: "edgeH",
  sideV: "edgeV",
  cornerExtra: "corner",
  sideMidH: "midH",
  sideMidV: "midV",
};

export const SLICE_KINDS: SliceKind[] = [
  "corner",
  "edgeH",
  "edgeV",
  "midH",
  "midV",
];

/** Which copies a shape has. */
export const KIND_SIDES: Record<SliceKind, SliceSide[]> = {
  corner: ["tl", "tr", "bl", "br"],
  edgeH: ["top", "bottom"],
  edgeV: ["left", "right"],
  midH: ["top", "bottom"],
  midV: ["left", "right"],
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
  {
    anchorX: "left" | "right";
    anchorY: "top" | "bottom";
    gripX: "left" | "right";
    gripY: "top" | "bottom";
  }
> = {
  tl: { anchorX: "left", anchorY: "top", gripX: "right", gripY: "bottom" },
  tr: { anchorX: "right", anchorY: "top", gripX: "left", gripY: "bottom" },
  bl: { anchorX: "left", anchorY: "bottom", gripX: "right", gripY: "top" },
  br: { anchorX: "right", anchorY: "bottom", gripX: "left", gripY: "top" },
  // An edge has one anchored side and one free run. Along the run the anchor is
  // the end the run is measured from, so a nudge reads the same on both edges.
  top: { anchorX: "left", anchorY: "top", gripX: "right", gripY: "bottom" },
  bottom: { anchorX: "left", anchorY: "bottom", gripX: "right", gripY: "top" },
  left: { anchorX: "left", anchorY: "top", gripX: "right", gripY: "bottom" },
  right: { anchorX: "right", anchorY: "top", gripX: "left", gripY: "bottom" },
};

/** Which way a drag of `dx`/`dy` counts on this copy. Derived from the axes
 *  above rather than written out again: a sign that disagreed with the grip's
 *  own corner would make the grip run away from the pointer. */
export function sliceSigns(side: SliceSide) {
  const axes = SLICE_SIDE_AXES[side];
  return {
    nudgeX: axes.anchorX === "left" ? 1 : -1,
    nudgeY: axes.anchorY === "top" ? 1 : -1,
    growX: axes.gripX === "right" ? 1 : -1,
    growY: axes.gripY === "bottom" ? 1 : -1,
  };
}

export type SliceResizeX = "left" | "right";
export type SliceResizeY = "top" | "bottom";

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
): Pick<SlicePlace, "growX" | "growY" | "nudgeX" | "nudgeY"> {
  const x = xEdge
    ? resizeAxis(kind, side, "x", xEdge, dx)
    : { grow: 0, nudge: 0 };
  const y = yEdge
    ? resizeAxis(kind, side, "y", yEdge, dy)
    : { grow: 0, nudge: 0 };
  return { growX: x.grow, nudgeX: x.nudge, growY: y.grow, nudgeY: y.nudge };
}

function resizeAxis(
  kind: SliceKind,
  side: SliceSide,
  axis: "x" | "y",
  edge: SliceResizeX | SliceResizeY,
  d: number,
): { grow: number; nudge: number } {
  const axes = SLICE_SIDE_AXES[side];
  const sign = sliceSigns(side);
  const grip = axis === "x" ? axes.gripX : axes.gripY;
  const fromGrip = edge === grip;
  const growSign = axis === "x" ? sign.growX : sign.growY;
  const nudgeSign = axis === "x" ? sign.nudgeX : sign.nudgeY;

  // A medallion is centred on its edge: width/height grows both ways from
  // the midpoint, so planting the far side means the centre has to walk
  // with the pointer at half speed.
  const centred =
    (kind === "midH" && axis === "x") || (kind === "midV" && axis === "y");
  if (centred) {
    const start = axis === "x" ? "left" : "top";
    return edge === start
      ? { grow: -d, nudge: d / 2 }
      : { grow: d, nudge: d / 2 };
  }

  // An edge's LENGTH is taken off both ends. Half the delta goes to grow
  // and half to the shift, so the held end moves 1:1 and the far one stays.
  const dual =
    (kind === "edgeH" && axis === "x") || (kind === "edgeV" && axis === "y");
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

function piece(
  kind: SliceKind,
  layer: number,
  fit: SliceFit = "stretch",
): SlicePiece {
  const places: Partial<Record<SliceSide, SlicePlace>> = {};
  for (const side of KIND_SIDES[kind]) places[side] = place();
  return { layer, fit, turn: "mirror", linked: true, places };
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
    corner: piece("corner", 2),
    sideH: piece("edgeH", 3),
    sideV: piece("edgeV", 3),
    cornerExtra: piece("corner", 5, "contain"),
    sideMidH: piece("midH", 5, "contain"),
    sideMidV: piece("midV", 5, "contain"),
  };
}

/** A flourish the keeper just added: a picture, a shape, and the accents'
 *  own habits — laid in whole, above the assembly. */
export function newOrnament(
  image: string,
  kind: SliceKind = "corner",
): SliceOrnament {
  return {
    id: crypto.randomUUID(),
    image,
    kind,
    ...piece(kind, 5, "contain"),
  };
}

function span(v: unknown): number | null {
  return typeof v === "number" && Number.isFinite(v)
    ? Math.min(SLICE_GROW_MAX, Math.max(-SLICE_GROW_MAX, v))
    : null;
}

function placesOf(given: SlicePiece | undefined, kind: SliceKind) {
  const places: Partial<Record<SliceSide, SlicePlace>> = {};
  for (const side of KIND_SIDES[kind]) {
    const had = given?.places?.[side];
    places[side] =
      had && typeof had === "object"
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
function settle(
  given: SlicePiece | undefined,
  kind: SliceKind,
  base: SlicePiece,
): SlicePiece {
  if (!given || typeof given !== "object")
    return { ...base, places: placesOf(undefined, kind) };
  const layer = Number(given.layer);
  return {
    layer:
      Number.isFinite(layer) && layer >= 1 && layer <= SLICE_LAYERS
        ? Math.round(layer)
        : base.layer,
    fit: SLICE_FITS.includes(given.fit) ? given.fit : base.fit,
    turn: SLICE_TURNS.includes(given.turn) ? given.turn : base.turn,
    linked: typeof given.linked === "boolean" ? given.linked : true,
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
      const kind = SLICE_KINDS.includes(one.kind) ? one.kind : "corner";
      return {
        ...one,
        kind,
        ...settle(one, kind, defaultSlices().cornerExtra),
      };
    });
  return {
    ...frame,
    slices,
    ornaments,
    // Опись и два множителя — по той же причине, что и детали: стол правит
    // живой объект, а рамка, сохранённая до описи, несёт пустой список и ноли.
    // Дополняется здесь один раз, а не в каждом месте, которое их читает.
    sheet: normalizeSheet(frame.sheet),
    typeScale: frame.typeScale || 1,
    inkFade: frame.inkFade || 1,
  };
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
    }[slot] ?? ""
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

export function carving(frame: BattleFrame, showEmpty = false): CarvedPiece[] {
  const out: CarvedPiece[] = [];
  for (const slot of SLICE_SLOTS) {
    const image = slotArt(frame, slot);
    // Пустой слот на СТОЛЕ показывается пунктиром и берётся в руку, а на полке
    // не существует вовсе.
    //
    // Без этого пустая рама — карта, на которой нечего нажать: место у детали
    // есть, а детали нет, и человек жмёт на край и попадает в воздух. Пунктир
    // не «подсказка», а сама деталь: та же коробка, тот же `data-piece`, тот же
    // захват — просто в ней пока ничего не нарисовано.
    if (!image && !showEmpty) continue;
    out.push({
      id: slot,
      kind: SLICE_KIND[slot],
      image,
      piece: pieceOf(frame, slot),
    });
  }
  for (const one of frame.ornaments ?? []) {
    const image = one?.image?.trim();
    if (!one?.id || !image) continue;
    const kind = SLICE_KINDS.includes(one.kind) ? one.kind : "corner";
    out.push({
      id: one.id,
      kind,
      image,
      piece: settle(one, kind, defaultSlices().cornerExtra),
    });
  }
  return out;
}

/** What shape a piece is, named slot or added flourish alike. */
export function kindOf(frame: BattleFrame, id: string): SliceKind | null {
  if ((SLICE_SLOTS as string[]).includes(id))
    return SLICE_KIND[id as SliceSlot];
  const found = frame.ornaments?.find((one) => one.id === id);
  if (!found) return null;
  return SLICE_KINDS.includes(found.kind) ? found.kind : "corner";
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

function boxOf(
  frame: BattleFrame,
  kind: SliceKind,
  side: SliceSide,
  at: SlicePlace,
): string {
  const top = frame.insetTop || 0;
  const right = frame.insetRight || 0;
  const bottom = frame.insetBottom || 0;
  const left = frame.insetLeft || 0;
  const { anchorX, anchorY } = SLICE_SIDE_AXES[side];
  // The band this copy starts from: its own two insets.
  const acrossX = anchorX === "left" ? left : right;
  const acrossY = anchorY === "top" ? top : bottom;
  if (kind === "corner") {
    return [
      `${anchorY}:${at.nudgeY}%`,
      `${anchorX}:${at.nudgeX}%`,
      `width:${acrossX + at.growX}%`,
      `height:${acrossY + at.growY}%`,
    ].join(";");
  }
  if (kind === "edgeH") {
    // `grow` along the run is taken off BOTH ends, so growing it reaches in
    // under the two corners — the join that was impossible while a band was
    // also a boundary.
    return [
      `${anchorY}:${at.nudgeY}%`,
      `height:${acrossY + at.growY}%`,
      `left:${left - at.growX + at.nudgeX}%`,
      `right:${right - at.growX - at.nudgeX}%`,
    ].join(";");
  }
  if (kind === "edgeV") {
    return [
      `${anchorX}:${at.nudgeX}%`,
      `width:${acrossX + at.growX}%`,
      `top:${top - at.growY + at.nudgeY}%`,
      `bottom:${bottom - at.growY - at.nudgeY}%`,
    ].join(";");
  }
  // A medallion: centred on its edge, and square to the band it rides on, so it
  // reads at the scale of the border rather than at whatever its file happens
  // to be. The centring translate is composed with the mirror below.
  const size = kind === "midH" ? acrossY : acrossX;
  return kind === "midH"
    ? [
        `${anchorY}:${at.nudgeY}%`,
        `left:calc(50% + ${at.nudgeX}%)`,
        `width:${size + at.growX}%`,
        `height:${size + at.growY}%`,
      ].join(";")
    : [
        `${anchorX}:${at.nudgeX}%`,
        `top:calc(50% + ${at.nudgeY}%)`,
        `width:${size + at.growX}%`,
        `height:${size + at.growY}%`,
      ].join(";");
}

/** How a copy is turned to reach its side. The FIRST copy of a piece — top-left
 *  corner, lintel, left side — is never turned; the rest are mirrored by
 *  default, so an asymmetric flourish stays right-side up wherever it lands. */
function turnOf(kind: SliceKind, side: SliceSide, turn: SliceTurn): string {
  const centring =
    kind === "midH"
      ? "translateX(-50%)"
      : kind === "midV"
        ? "translateY(-50%)"
        : "";
  const first = KIND_SIDES[kind][0];
  let face = "";
  if (side !== first) {
    if (turn === "rotate") {
      // Quarter turns run the way a corner round is drawn: clockwise from the
      // top-left, so the piece meets the same two edges it was cut against.
      face =
        kind === "corner"
          ? { tr: "rotate(90deg)", br: "rotate(180deg)", bl: "rotate(270deg)" }[
              side as "tr" | "br" | "bl"
            ]
          : "rotate(180deg)";
    } else if (turn === "mirror") {
      face =
        kind === "corner"
          ? { tr: "scaleX(-1)", bl: "scaleY(-1)", br: "scale(-1, -1)" }[
              side as "tr" | "bl" | "br"
            ]
          : KIND_SIDES[kind][0] === "top"
            ? "scaleY(-1)"
            : "scaleX(-1)";
    }
  }
  const both = [centring, face].filter(Boolean).join(" ");
  return both ? `transform:${both}` : "";
}

/** `background-size`/`-repeat`/`-position` for one fit. `tile` is the only one
 *  that has to know which way the band runs: a running vine repeats ALONG its
 *  edge and is scaled across it, never the other way round. */
function fitOf(kind: SliceKind, fit: SliceFit): string {
  if (fit === "contain")
    return "background-size:contain;background-position:center";
  if (fit === "cover")
    return "background-size:cover;background-position:center";
  if (fit === "tile") {
    // A tile has to start from the band's own anchor, or the repeat would begin
    // mid-picture.
    const along =
      kind === "edgeH" || kind === "midH"
        ? "background-size:auto 100%;background-repeat:repeat-x"
        : kind === "edgeV" || kind === "midV"
          ? "background-size:100% auto;background-repeat:repeat-y"
          : "background-size:auto;background-repeat:repeat";
    return `${along};background-position:left top`;
  }
  return "background-size:100% 100%;background-position:center";
}

/** Every copy of every piece, ready to render. A copy the keeper put out is
 *  simply not here — an accent over the lintel and nothing on the sill is one
 *  unticked box, not a second upload with half of it erased. */
export function carvedCopies(
  frame: BattleFrame,
  showEmpty = false,
): CarvedCopy[] {
  const out: CarvedCopy[] = [];
  for (const { id, kind, image, piece: settled } of carving(frame, showEmpty)) {
    // Пустая деталь: коробка на месте, картинки нет. Пунктир рисуется здесь же,
    // а не классом, потому что и всё остальное про эту копию — строка стиля, и
    // второе место, где решается, как копия выглядит, однажды разошлось бы.
    const paint = image
      ? `background-image:url("${cssUrl(image)}");${fitOf(kind, settled.fit)};background-repeat:no-repeat`
      : "outline:1px dashed rgba(52,37,28,0.35);outline-offset:-2px;background:rgba(52,37,28,0.04)";
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
      ]
        .filter(Boolean)
        .join(";");
      out.push({ id, side, layer: settled.layer, style });
    }
  }
  return out;
}

/* ── Опись ────────────────────────────────────────────────────────────────
   Что печатается на карте, в какой полосе и в каком порядке.

   Список И ЕСТЬ порядок — то же правило, что у списка деталей рамки. Полоса
   и порядок, а не координаты: у текста длина меняется от языка и от карты, и
   свободно поставленное имя столкнётся с соседкой на первом же длинном
   названии. Координаты остались там, где стоит одна цифра, — у значков.     */

/** Все строки описи, в домашнем порядке. Порядок здесь — порядок на карте у
 *  рамки, которая описи не трогала. */
export const SHEET_SLOTS: SheetSlot[] = [
  "raceIcon",
  "race",
  "kind",
  "channel",
  "pips",
  "title",
  "rank",
  "traits",
  "effect",
  "lore",
  "health",
  "mana",
  "armor",
  "ward",
  "reach",
  "step",
  "mend",
  "stats",
  "cost",
  "power",
  "healthMark",
  "new",
  "costWord",
  "powerWord",
];

/**
 * Семь чисел паспорта. Строки описи, но не такие, как все: печатаются они
 * ОДНОЙ коробкой, а не семью — семь отдельных абзацев в колонке свойств это
 * семь строк высотой в карту. Коробку ставит первая из них, порядок внутри —
 * порядок описи, а сама коробка видна с той величины, с какой видно самое
 * щедрое из чисел (см. `statGroupShow`).
 */
export const SHEET_STATS: BodyStatField[] = [
  "health",
  "mana",
  "armor",
  "ward",
  "reach",
  "step",
  "mend",
];

export function isStatSlot(slot: SheetSlot): slot is BodyStatField {
  return (SHEET_STATS as string[]).includes(slot);
}

/** Ступени в том порядке, в каком их предлагают: от «нигде» к «везде», и
 *  особая пятая — «только в клетке» — последней, потому что она единственная
 *  говорит про потолок, а не про порог. */
export const SHEET_SHOWS: SheetShow[] = [
  "never",
  "large",
  "always",
  "cell",
  "cellOnly",
];

/**
 * Коробка паспорта видна с той величины, с какой видно самое щедрое из чисел в
 * ней. Иначе коробка, полная скрытых чисел, оставляла бы на карте свой отступ:
 * пустое место там, где по описи ничего не стоит.
 */
export function statGroupShow(rows: SheetRow[]): SheetShow {
  const has = (show: SheetShow) => rows.some((row) => row.show === show);
  // Не «самая старшая ступень», а самая ТЕСНАЯ, которая накрывает все: у
  // «только в клетке» потолок, а не порог, и по номеру в списке её не сложить
  // с остальными. Чего накрыть нельзя (лист и клетка без полки), накрывается с
  // запасом — лишний раз показанная пустая коробка честнее спрятанного числа.
  if (has("cell") || ((has("large") || has("always")) && has("cellOnly")))
    return "cell";
  if (has("always")) return "always";
  if (has("cellOnly")) return "cellOnly";
  if (has("large")) return "large";
  return "never";
}

/**
 * Где строка вообще может стоять.
 *
 * Не украшение и не вкусовщина: проза в шапке высотой в девять процентов
 * карты — это обрезанная проза, а метка «новая» и подписи под значками стоят
 * поверх карты и в потоке полос не стоят вовсе. Стол предлагает только то,
 * что имеет смысл, вместо того чтобы позволить выбрать заведомо сломанное.
 */
export const SHEET_SLOT_BANDS: Record<SheetSlot, SheetBand[]> = {
  raceIcon: ["head", "props", "foot"],
  race: ["head", "props", "foot"],
  kind: ["head", "props", "foot"],
  channel: ["head", "props", "foot"],
  pips: ["head", "props", "foot"],
  title: ["props", "head", "foot"],
  rank: ["props", "head", "foot"],
  traits: ["props"],
  effect: ["props"],
  lore: ["props"],
  health: ["props", "foot", "head"],
  mana: ["props", "foot", "head"],
  armor: ["props", "foot", "head"],
  ward: ["props", "foot", "head"],
  reach: ["props", "foot", "head"],
  step: ["props", "foot", "head"],
  mend: ["props", "foot", "head"],
  stats: ["foot", "head", "props"],
  cost: ["over"],
  power: ["over"],
  healthMark: ["over"],
  new: ["over", "head", "foot"],
  costWord: ["over"],
  powerWord: ["over"],
};

/**
 * Домашняя опись — буква в букву то, что карта печатала до описи, с одним
 * намеренным отличием: подписи под значками стоимости и силы сняты.
 *
 * Подпись висит под кружком в углу, а шапка отступает от кружка на его
 * ширину и про подпись ничего не знает — отсюда «СТОИМОСТЬ ДОМОВЫЕ · ТЕ…» на
 * каждой карте полки. Кружок с цифрой в углу карты понятен и без слова;
 * хранитель, которому слово нужно, включает его и ставит значок туда, где
 * слово помещается.
 */
/**
 * Домашняя опись — буква в букву то, что карта печатала до неё, и «буква в
 * букву» включает клетку боя: там всегда были рама, фотография и ИМЯ, и ничего
 * больше. Поэтому имя одно стоит на `cell`, а не на `always` — не потому, что
 * оно важнее, а потому, что так было.
 */
export function defaultSheet(): SheetRow[] {
  return SHEET_SLOTS.map((slot) => ({
    slot,
    show:
      slot === "costWord" || slot === "powerWord"
        ? "never"
        : slot === "healthMark"
          ? // Кружок здоровья — ТОЛЬКО в клетке боя, и «только» здесь
            // существенно: он встаёт ровно туда, где стоит стоимость, и
            // появляется ровно тогда, когда та исчезает. На полке цена и сила
            // напечатаны на бумаге и никуда не денутся; в клетке цена не
            // значит ничего, а здоровье — всё.
            "cellOnly"
          : slot === "title"
            ? "cell"
            : slot === "lore"
              ? "large"
              : "always",
    band: SHEET_SLOT_BANDS[slot][0],
  }));
}

/** Домашняя опись, но клетка боя показывает здоровье и силу. Не умолчание —
 *  заготовка: доска, на которой видно, кто ещё жив, это выбор хранителя, а
 *  `BATTLE-SCENE.md` держится обратного («числа только у раненых»). */
export function cellSheet(): SheetRow[] {
  return defaultSheet().map((row) =>
    row.slot === "health" || row.slot === "power"
      ? { ...row, show: "cell" }
      : row,
  );
}

/**
 * Опись, годная к отрисовке: незнакомое выброшено, повторы сняты, недостающее
 * дописано в домашнем виде и в домашнем месте.
 *
 * Дописывать обязательно, а не желательно: рамка, сохранённая до описи, несёт
 * пустой список, и «пустая опись = пустая карта» стёрла бы всю полку одним
 * сохранением. Пустое — это «как в доме», и поэтому переезда данных не
 * потребовалось.
 */
export function normalizeSheet(
  given: SheetRow[] | null | undefined,
): SheetRow[] {
  const seen = new Set<SheetSlot>();
  const rows: SheetRow[] = [];
  for (const row of given ?? []) {
    const slot = row?.slot as SheetSlot;
    if (!slot || !SHEET_SLOT_BANDS[slot] || seen.has(slot)) continue;
    seen.add(slot);
    const bands = SHEET_SLOT_BANDS[slot];
    rows.push({
      slot,
      show: SHEET_SHOWS.includes(row.show) ? row.show : "always",
      band: bands.includes(row.band) ? row.band : bands[0],
    });
  }
  if (!rows.length) return defaultSheet();
  // Строка, которой в сохранённой описи нет, встаёт на своё домашнее место —
  // не в конец. Иначе появление новой строки в доме переставляло бы карты у
  // всех, кто описи касался.
  for (const row of defaultSheet()) {
    if (seen.has(row.slot)) continue;
    const at = SHEET_SLOTS.indexOf(row.slot);
    const before = rows.findIndex((one) => SHEET_SLOTS.indexOf(one.slot) > at);
    if (before < 0) rows.push(row);
    else rows.splice(before, 0, row);
  }
  return rows;
}

/** Полосы в том порядке, в каком они стоят на карте. Ими же перечисляются
 *  ящики на столе, чтобы список и карта читались одинаково. */
export const SHEET_BANDS: SheetBand[] = ["head", "props", "foot", "over"];

/**
 * Переложить строку описи — в другую полосу, на другое место, или и то и
 * другое сразу. Одна пересадка на оба входа: мышь по карте и мышь по списку.
 *
 * Место названо СОСЕДОМ, а не номером, и это не вкусовщина. Номер значил бы
 * разное у двух входов: карта видит только напечатанное (у карты без черт
 * строки черт на ней нет), а список видит всю опись, — и «третье место»
 * оказывалось бы разным местом, смотря откуда несли. Сосед у обоих один и тот
 * же. `before: null` — в конец своей полосы.
 *
 * Список ПЕРЕСТАВЛЯЕТСЯ, а не пересобирается по полосам: порядок строк соседних
 * полос ничего не рисует, но он чей-то — перекладывать чужое, потому что тронули
 * своё, значит однажды сдвинуть то, чего никто не двигал.
 *
 * Полоса, в которой строке стоять нельзя, не берётся вовсе: `SHEET_SLOT_BANDS`
 * — не украшение, а забор от заведомо сломанного (проза в шапке высотой в
 * девять процентов карты это обрезанная проза).
 */
export function moveSheetRow(
  given: SheetRow[] | null | undefined,
  slot: SheetSlot,
  band: SheetBand,
  before: SheetSlot | null,
): SheetRow[] {
  const rows = normalizeSheet(given);
  const from = rows.findIndex((row) => row.slot === slot);
  if (from < 0) return rows;
  const allowed = SHEET_SLOT_BANDS[slot];
  const moved: SheetRow = {
    ...rows[from],
    band: allowed.includes(band) ? band : rows[from].band,
  };
  const rest = rows.filter((_, i) => i !== from);
  const out = rest.slice();
  const at = before
    ? rest.findIndex((row) => row.slot === before && row.band === moved.band)
    : -1;
  if (at >= 0) {
    out.splice(at, 0, moved);
    return out;
  }
  // В конец своей полосы — то есть сразу за её последней строкой, а не в конец
  // всего списка: там начинается чужая полоса.
  const inBand = rest.filter((row) => row.band === moved.band);
  if (!inBand.length) return [...rest, moved];
  out.splice(out.indexOf(inBand[inBand.length - 1]) + 1, 0, moved);
  return out;
}

/**
 * Печатает ли карта это число САМА, стоя в клетке боя.
 *
 * Спрашивает сцена, и спрашивает обязательно: доска рисует свои кружки поверх
 * карты, а карта рисует свои, и два отрисовщика одного числа — это не
 * «дублирование кода», это два кружка в одном углу. Ровно так и вышло: кружок
 * доски накрыл собой значок карты, и вся работа над значком была не видна.
 *
 * Слотом, а не тремя функциями: здоровье и сила устроены одинаково, и
 * `cellSaysHealth`, у которой не было ни одного вызова, ничему не помешала —
 * никто просто не спросил.
 *
 * В партии значок ставит `alive` даже на клетке шире полки (ступени ширины там
 * не решают — см. `byWidth` в `BattleCard`), поэтому здесь достаточно «строка
 * не снята», а не ступень `cell`/`cellOnly`.
 */
export function cellPrints(
  frame: BattleFrame,
  slot: SheetSlot,
): boolean {
  // На готовой иллюстрации значок, назначенный меткой, решает сам (§ 9.10).
  const kind: BadgeKind | null =
    slot === "healthMark" ? "health" : slot === "cost" || slot === "power" ? slot : null;
  const marked = kind ? freeBadgeShown(frame, kind) : null;
  if (marked != null) return marked;
  return sheetOf(frame).some(
    (row) => row.slot === slot && row.show !== "never",
  );
}

/** Опись этой рамы. Один вход для всех, кто её читает, — и карты, и стола. */
export function sheetOf(frame: Pick<BattleFrame, "sheet">): SheetRow[] {
  return normalizeSheet(frame.sheet);
}

/** Строки одной полосы, в порядке описи. */
export function sheetBand(rows: SheetRow[], band: SheetBand): SheetRow[] {
  return rows.filter((row) => row.band === band && row.show !== "never");
}

/** Показывается ли строка хоть где-нибудь. */
export function sheetShows(rows: SheetRow[], slot: SheetSlot): boolean {
  return rows.some((row) => row.slot === slot && row.show !== "never");
}

/**
 * Ширина, с которой строка «только крупно» появляется. То самое число, по
 * которому карта уже делит лист взятия и полку (`@container`, 280 px), —
 * второе, своё, развело бы предпросмотр и комнату.
 */
export const SHEET_LARGE_MIN = 281;

/**
 * Вторая ступень — порог полки. То же число, по которому карта делит себя сама
 * (`@container`, 161 px); второе, своё, развело бы предпросмотр и комнату
 * ровно так же, как развело бы первое.
 */
export const SHEET_SHELF_MIN = 161;

/**
 * Ширины, на которых карта стоит в комнате НА САМОМ ДЕЛЕ.
 *
 * Числа не круглые для красоты: это те самые 281 и 161, по которым карта делит
 * себя сама, взятые по обе стороны от каждого порога, — каждая ширина
 * ЗАВЕДОМО в своей полосе, а не на её краю. Один список на весь дом: стенд
 * «Лица карты» и стол резчика обязаны показывать одну и ту же карту, а два
 * списка разошлись бы на первом же подправленном числе, и один из двух
 * предпросмотров начал бы врать молча.
 *
 * От крупного к мелкому: рамку строят на листе взятия, где видно всё, и
 * проверяют на полке и в клетке, где видно не всё.
 */
export const CARD_WIDTHS = [400, 261, 140] as const;

/**
 * До какой ступени описи дотягивается карта такой ширины.
 *
 * Спрашивается у ШИРИНЫ, а не у величины стенда: карта делит себя сама, теми
 * же двумя порогами, и назвать полосу вторым способом значило бы завести
 * вторую правду о том, что на карте напечатано.
 */
export function widthShow(width: number): "large" | "always" | "cell" {
  if (width >= SHEET_LARGE_MIN) return "large";
  if (width >= SHEET_SHELF_MIN) return "always";
  return "cell";
}

/**
 * Какой высоты карта такой ширины. `aspect` — ширина к высоте (дом: 5 к 7),
 * поэтому высота делением, а не умножением; округляется, потому что показывают
 * её хранителю, а не считают из неё.
 */
export function cardTallAt(width: number, aspect: number): number {
  return Math.round(width / (aspect || DEFAULT_ASPECT));
}

/**
 * Сколько карта обязана уступить значкам стоимости и силы и метке «новая» —
 * в cqi, то есть в процентах ширины карты.
 *
 * До этого отступ был один и жёсткий (17cqi слева у шапки, 17cqi справа у
 * текста), и держался он на том, что значки стоят в двух углах по умолчанию.
 * Хранитель, оттащивший значок на четверть карты вправо, получал шапку под
 * значком и никакого способа это заметить, кроме как посмотреть. Считается из
 * того, где значок СТОИТ.
 *
 * Значок «в шапке», если его верхний край выше нижней границы шапки; иначе он
 * мешает тексту свойств, и уступает ему тот. Высота кружка задана в cqi, то
 * есть в долях ШИРИНЫ, поэтому в доли высоты переводится через отношение
 * сторон — иначе на квадратной карте отступа не хватило бы, а на узкой он
 * появлялся бы там, где значка нет.
 */
export function badgeReserve(
  frame: BattleFrame,
  opts: {
    isNew: boolean;
    costOn: boolean;
    powerOn: boolean;
    healthOn: boolean;
    /** Числа, которые значки печатают. Отступ считается по НАРИСОВАННОЙ
     *  ширине, а она зависит от числа: «1» уже «0» в полтора раза. */
    numbers: Record<BadgeKind, number | null | undefined>;
    costWord: boolean;
    powerWord: boolean;
    newOver: boolean;
  },
): {
  headLeft: number;
  headRight: number;
  bodyLeft: number;
  bodyRight: number;
} {
  let headLeft = 0;
  let headRight = 0;
  let bodyLeft = 0;
  let bodyRight = 0;
  if (frame.layout === "corners") {
    const aspect = frame.aspect || DEFAULT_ASPECT;
    const top = frame.insetTop || 0;
    const bottom = frame.insetBottom || 0;
    const headBottom =
      top + (frame.headerShare ?? DEFAULT_HEADER_SHARE) * (100 - top - bottom);
    // Величина значка входит в расчёт: увеличенный кружок и лезет дальше в
    // шапку, и просит больше отступа. Отступ, посчитанный по домашним 10.5cqi,
    // молча разошёлся бы с тем, что нарисовано, — ровно та же поломка, что
    // была у долей окна против долей карты.
    const worn: Record<BadgeKind, boolean> = {
      cost: opts.costOn,
      power: opts.powerOn,
      health: opts.healthOn,
    };
    const worded: Record<BadgeKind, boolean> = {
      cost: opts.costWord,
      power: opts.powerWord,
      // У здоровья подписи нет: его включают ровно там, где для слова уже
      // нет места.
      health: false,
    };
    const badges = BADGE_KINDS.filter((kind) => worn[kind]).map((kind) => ({
      ...badgeAt(frame, kind, opts.numbers[kind]),
      word: worded[kind],
      extent: badgeExtent(frame, kind, opts.numbers[kind]),
    }));
    for (const badge of badges) {
      const halfSize = badge.extent.w / 2;
      const half = (badge.extent.h / 2) * aspect;
      const inHead = badge.y - half <= headBottom;
      const reach = badge.word
        ? Math.max(BADGE_WORD_REACH, halfSize)
        : halfSize;
      const near = badge.x < 50;
      const room = near
        ? badge.x + reach - (frame.insetLeft || 0)
        : 100 - badge.x + reach - (frame.insetRight || 0);
      if (inHead) {
        if (near) headLeft = Math.max(headLeft, room);
        else headRight = Math.max(headRight, room);
      } else if (near) bodyLeft = Math.max(bodyLeft, room);
      else bodyRight = Math.max(bodyRight, room);
    }
  }
  // Метка «новая» лежит поверх правого края шапки. В потоке полосы (её можно
  // поставить и туда) она места не занимает и уступать ей нечего.
  if (opts.isNew && opts.newOver)
    headRight = Math.max(headRight, NEW_MARK_REACH);
  return {
    headLeft: Math.max(0, headLeft),
    headRight: Math.max(0, headRight),
    bodyLeft: Math.max(0, bodyLeft),
    bodyRight: Math.max(0, bodyRight),
  };
}

/**
 * Три значка, и все три устроены одинаково: место, форма, заливка, чернила,
 * величина, толщина. Таблица, а не тернарник на каждое поле, — при двух
 * значках это была мелкая неопрятность, при трёх стало бы шесть развилок,
 * каждую из которых можно забыть по отдельности.
 */
export type BadgeKind = "cost" | "power" | "health";

export const BADGE_KINDS: BadgeKind[] = ["cost", "power", "health"];

export const BADGE_FIELDS = {
  cost: {
    x: "costX",
    y: "costY",
    shape: "costShape",
    fill: "costFill",
    ink: "costInk",
    size: "costSize",
    weight: "costWeight",
    plate: "costPlate",
    homeX: DEFAULT_COST_X,
    homeY: DEFAULT_COST_Y,
  },
  power: {
    x: "powerX",
    y: "powerY",
    shape: "powerShape",
    fill: "powerFill",
    ink: "powerInk",
    size: "powerSize",
    weight: "powerWeight",
    plate: "powerPlate",
    homeX: DEFAULT_POWER_X,
    homeY: DEFAULT_POWER_Y,
  },
  // У здоровья поля СВОИ, но пустые они значат «как у стоимости» (см.
  // `BADGE_HOME` ниже). Сперва их не было вовсе — кружок здоровья не «походил
  // на» кружок стоимости, а и БЫЛ им, — и это было верно ровно до того дня,
  // когда хранителю понадобилось развести их: здоровье меняется в бою, а
  // стоимость напечатана навсегда, и одеть их одинаково — выбор, а не закон.
  // Откат к стоимости оставлен затем, что старый закон был не глуп: пока
  // хранитель молчит, два кружка остаются одним и разойтись не могут.
  health: {
    x: "healthX",
    y: "healthY",
    shape: "healthShape",
    fill: "healthFill",
    ink: "healthInk",
    size: "healthSize",
    weight: "healthWeight",
    plate: "healthPlate",
    homeX: DEFAULT_COST_X,
    homeY: DEFAULT_COST_Y,
  },
} as const satisfies Record<
  BadgeKind,
  {
    x: keyof BattleFrame;
    y: keyof BattleFrame;
    shape: keyof BattleFrame;
    fill: keyof BattleFrame;
    ink: keyof BattleFrame;
    size: keyof BattleFrame;
    weight: keyof BattleFrame;
    plate: keyof BattleFrame;
    homeX: number;
    homeY: number;
  }
>;

/**
 * Чей значок донашивает этот, пока ему не назначили своего.
 *
 * Одна запись, а не откат, повторённый в каждом из семи чтений: разойтись
 * семи копиям одного правила — вопрос времени, и разошлись бы они молча.
 */
export const BADGE_HOME: Partial<Record<BadgeKind, BadgeKind>> = {
  health: "cost",
};

type BadgeFieldKey =
  "x" | "y" | "shape" | "fill" | "ink" | "size" | "weight" | "plate";

function badgeRaw(
  frame: BattleFrame,
  kind: BadgeKind,
  key: BadgeFieldKey,
): unknown {
  return frame[BADGE_FIELDS[kind][key] as keyof BattleFrame];
}

/**
 * Строковое поле значка, с оглядкой на того, чей наряд он донашивает.
 *
 * Пустая строка у ЗДОРОВЬЯ значит «как у стоимости», и только потом уже пустая
 * строка стоимости значит «как в раме». Две пустоты на двух уровнях — не
 * путаница, а лестница: снял своё — вернулся к стоимости, снял и у стоимости —
 * вернулся к раме.
 */
export function badgeText(
  frame: BattleFrame,
  kind: BadgeKind,
  key: BadgeFieldKey,
): string {
  const own = ((badgeRaw(frame, kind, key) as string) ?? "").trim();
  if (own) return own;
  const home = BADGE_HOME[kind];
  return home ? ((badgeRaw(frame, home, key) as string) ?? "").trim() : "";
}

/** Числовое поле значка. Ноль — «не назначено», как у `typeScale`. */
export function badgeNum(
  frame: BattleFrame,
  kind: BadgeKind,
  key: "size" | "weight",
): number {
  const own = badgeRaw(frame, kind, key) as number;
  if (own) return own;
  const home = BADGE_HOME[kind];
  return home ? (badgeRaw(frame, home, key) as number) || 0 : 0;
}

/** Где значок стоит на самом деле — со своим местом, местом донашиваемого и
 *  домашним, в этом порядке, и с прижатием к карте. `null` здесь значит «не
 *  назначено», а ноль — верхний левый угол, поэтому проверка на `!= null`, а
 *  не на истинность. */
export function badgeAt(
  frame: BattleFrame,
  kind: BadgeKind,
  value: number | null | undefined,
): { x: number; y: number } {
  const keys = BADGE_FIELDS[kind];
  const axis = (key: "x" | "y", home: number) => {
    const own = badgeRaw(frame, kind, key) as number | null | undefined;
    if (own != null) return own;
    const under = BADGE_HOME[kind];
    if (under) {
      const worn = badgeRaw(frame, under, key) as number | null | undefined;
      if (worn != null) return worn;
    }
    return home;
  };
  return badgeSpot(
    axis("x", keys.homeX),
    axis("y", keys.homeY),
    frame.aspect || DEFAULT_ASPECT,
    badgeExtent(frame, kind, value),
  );
}

/**
 * Кружок значка, в cqi. Совпадает с `.corner` в `BattleCard.svelte`.
 *
 * Домашние 10.5, а не 13: при цифре кегля 6.6cqi тринадцать оставляли вокруг
 * числа поле пустоты почти в саму цифру шириной, а на клетке боя кружок съедал
 * четверть высоты карты. Число это читают и `badgeSpot`, и `badgeExtent`, и
 * `badgeReserve`, поэтому записано оно ровно дважды — здесь и в CSS, — и
 * правится только вместе.
 */
export const BADGE_SIZE = 10.5;

/**
 * Значок, прижатый к КАРТЕ.
 *
 * Место значка меряется в долях карты, и это ровно затем, чтобы его можно было
 * вынести на раму. Но карта непрозрачна и обрезает вышедшее за неё
 * (`.card { overflow: hidden }`), поэтому за её краем не «место, где значок
 * свисает», а срез: половина кружка просто исчезает, и хранитель видит
 * полукруг, которого не ставил. Границы считаются из самого кружка, и по
 * вертикали — через отношение сторон: 10.5cqi заданы в долях ШИРИНЫ.
 *
 * Прижимает и отрисовщик, и перетаскивание, и `badgeReserve`: место, которое
 * они поняли бы порознь, — это отступ под значок, стоящий не там.
 */
export function badgeSpot(
  x: number,
  y: number,
  aspect: number,
  extent: BadgeExtent = { w: BADGE_SIZE, h: BADGE_SIZE },
): { x: number; y: number } {
  const halfX = extent.w / 2;
  // Высота названа в cqi, то есть в долях ШИРИНЫ, и в доли высоты переводится
  // отношением сторон — иначе на узкой карте запас брался бы не оттуда.
  const halfY = (extent.h / 2) * (aspect || DEFAULT_ASPECT);
  return {
    x: Math.min(100 - halfX, Math.max(halfX, x)),
    y: Math.min(100 - halfY, Math.max(halfY, y)),
  };
}
/**
 * Цифра на залитом кружке.
 *
 * Заливку хранитель выбирает, цифру — нет, и это не экономия на поле, а
 * условие: два цвета, назначаемые порознь, рано или поздно совпадут, и на
 * карте окажется пустой кружок. Выбор идёт из ДВУХ красок самой рамы —
 * бумаги и чернил, — а не из чёрного с белым: чужая пара выдала бы значок
 * как приклеенный.
 *
 * Светлота считается по тем же весам, что и всюду (Rec. 709). Цвет, который
 * не удалось прочесть, считается тёмным: так значок выглядел до заливки, и
 * незнакомая запись не должна менять его молча.
 */
/**
 * Обе краски значка одной строкой инлайнового стиля — или ничего.
 *
 * Ничего — это важное состояние: нетронутый значок не должен получить НИ
 * ОДНОГО объявления, тогда домашние цвета остаются там, где им и место, — в
 * откатах `var(--badge-fill, …)`, где они у стоимости и у силы разные. Кто
 * попробует разрешить их здесь, тому придётся повторить эту разницу вторым
 * списком, и однажды списки разойдутся.
 *
 * Цифру назначает хранитель. Не назначил — карта выбирает её от заливки, и
 * только если заливка выбрана: без заливки цифра лежит на самой карте, и
 * `badgeInk` даёт её чернила.
 */
export function badgeStyle(
  frame: BattleFrame,
  kind: BadgeKind,
): string | undefined {
  const fill = badgeText(frame, kind, "fill");
  const ink = badgeText(frame, kind, "ink");
  const plate = badgePlate(frame, kind);
  const size = badgeScale(frame, kind);
  const weight = badgeWeight(frame, kind);
  const parts: string[] = [];
  if (plate) parts.push(`--badge-plate:url("${cssUrl(plate)}")`);
  // Заливка под жетоном не печатается, но и не забывается: сняли жетон —
  // выбранный цвет на месте. Молчит она в CSS, а не здесь, ровно как форма.
  if (fill) parts.push(`--badge-fill:${fill}`);
  if (ink) parts.push(`--badge-ink:${ink}`);
  // Пару к заливке карта угадывает, только когда цифра на заливке и лежит.
  else if (fill && !plate) parts.push(`--badge-ink:${badgeInk(fill, frame)}`);
  // На жетоне цифра лежит на чужой картинке, про которую ни одна краска рамы
  // ничего не обещает. Домашний откат тут не годится: у стоимости он — БУМАГА,
  // и светлая она ровно потому, что дома лежит на тёмном кружке чернил. Жетон
  // этот кружок убрал, и та же бумага стала белым по светлому. Чернила карты —
  // не угадывание, а честное умолчание: они читаются на большинстве бляшек, и
  // хранителю всё равно решать самому.
  else if (plate) parts.push(`--badge-ink:${frame.ink}`);
  if (size !== 1) parts.push(`--badge-size:${size}`);
  if (weight) parts.push(`--badge-weight:${weight}`);
  return parts.length ? parts.join(";") : undefined;
}

/**
 * Жетон значка — картинка, надетая вместо крашеной подложки, или пустая
 * строка.
 *
 * Читается одной функцией по той же причине, по которой значки читаются одной
 * таблицей: у здоровья своего жетона НЕТ, оно носит жетон стоимости, и это
 * должно быть сказано ровно один раз — в `BADGE_FIELDS`, а не тернарником в
 * каждом из трёх мест, где жетон нужен (отрисовщик, коробка, стол).
 */
export function badgePlate(frame: BattleFrame, kind: BadgeKind): string {
  return badgeText(frame, kind, "plate");
}

/** Форма значка. Пустая — «как у того, чей наряд донашиваем», а если и там
 *  пусто, то кружок: форма есть у всякого значка, её нельзя не иметь. */
export function badgeShape(
  frame: BattleFrame,
  kind: BadgeKind,
): BattleBadgeShape {
  return (badgeText(frame, kind, "shape") as BattleBadgeShape) || "circle";
}

/** Множитель величины значка. Ноль и мусор — «не назначено», как у `typeScale`:
 *  рамка, сохранённая до этой ручки, несёт ноль. */
export function badgeScale(frame: BattleFrame, kind: BadgeKind): number {
  return clampScale(
    badgeNum(frame, kind, "size"),
    BADGE_SCALE_MIN,
    BADGE_SCALE_MAX,
  );
}

/** Толщина цифры, или 0 — «как у карты». Округляется к своей ступени: между
 *  начертаниями шрифта промежутка нет, и дробное число обещало бы его. */
export function badgeWeight(frame: BattleFrame, kind: BadgeKind): number {
  const given = badgeNum(frame, kind, "weight");
  if (!Number.isFinite(given) || !given) return 0;
  return BADGE_WEIGHTS.reduce((best, w) =>
    Math.abs(w - given) < Math.abs(best - given) ? w : best,
  );
}

/**
 * Каким начертанием дом печатает цифру значка. Зеркало `--badge-weight` в
 * `.corner-num`: на монете в 10.5cqi светлое начертание тонет, и дом берёт
 * полужирное. Названо числом здесь потому, что по нему мерится ШИРИНА цифры, а
 * `badgeWeight` отдаёт ноль, когда хранитель начертания не назначал, — ноль
 * значит «как дома», и мерка обязана знать, что это за дом.
 */
export const BADGE_WEIGHT_HOME = 600;

export const BADGE_SCALE_MIN = 0.5;
export const BADGE_SCALE_MAX = 4;

/** Голая цифра — её собственная коробка, в cqi: кегль 6.6cqi при высоте строки 1. */
export const BADGE_BARE = 6.6;

/**
 * Знак рядом с цифрой значка и просвет между ними, в cqi.
 *
 * Зеркало `.corner-glyph` и `gap` у `.corner`, правятся только вместе. Знак
 * мерится ЦИФРОЙ, а не плашкой: 0.72 её кегля (`BADGE_BARE`), потому что
 * равняется он на прописную полосу числа, а не на середину коробки — Georgia
 * набирает старостильные цифры, и у «3» середина рисунка на 0.167em ниже
 * середины её коробки. Подробнее — там же, в CSS.
 */
export const BADGE_GLYPH = BADGE_BARE * 0.72;
export const BADGE_GLYPH_GAP = 1.5;

/** Боковые поля плашки и её наименьшая ширина, в cqi. Зеркало `padding` и
 *  `min-width` у `.corner`: правятся только вместе с ними. */
export const BADGE_PAD = 2.4;

/**
 * Ширина каждой цифры Georgia, в долях кегля.
 *
 * Замерено в браузере на той самой цепочке, которой набран значок
 * (`Georgia, 'Fraunces', serif`), двумя начертаниями — тем, которым дом печатает
 * число (полужирным), и светлым, на случай если хранитель назначил его сам.
 *
 * Таблица, а не одно число, потому что Georgia набирает СТАРОСТИЛЬНЫЕ цифры и
 * ширина у них разная: «1» — 0.49 кегля, «0» — 0.70, то есть в полтора раза
 * шире. Одним усреднённым числом мерка либо режет двузначное, либо отодвигает
 * однозначное от края карты на пустое место шириной почти в цифру — а это
 * ровно то, из-за чего значок не удавалось прижать к самому углу.
 */
const GEORGIA_FIGURE_BOLD = [
  0.7012, 0.4898, 0.6265, 0.6245, 0.6494, 0.5991, 0.648, 0.5542, 0.6763, 0.648,
];
const GEORGIA_FIGURE_PLAIN = [
  0.6138, 0.4297, 0.5586, 0.5518, 0.5649, 0.5283, 0.5659, 0.5024, 0.5962,
  0.5659,
];

/**
 * Сколько занимает САМО ЧИСЛО значка, в долях кегля.
 *
 * Спрашивается у числа, а не у количества знаков в нём: «11» и «00» — две
 * цифры в обоих, а шириной они отличаются на две пятых кегля.
 *
 * Начертание у Georgia настоящих всего два, и промежуточных она не рисует —
 * браузер берёт полужирное со ступени 600. Поэтому и здесь ступень одна.
 */
export function badgeFigures(
  value: number | null | undefined,
  weight: number,
): number {
  const table = weight >= 600 ? GEORGIA_FIGURE_BOLD : GEORGIA_FIGURE_PLAIN;
  const text = String(Math.abs(Math.round(Number(value) || 0)));
  let sum = 0;
  for (const ch of text) sum += table[Number(ch)] ?? table[0];
  return sum;
}

/** Что значок занимает по ширине и по высоте, в cqi. Двумя числами, а не
 *  одним: у кружка они равны, у цифры — нет, и одно число на двоих оставило бы
 *  по бокам цифры ровно то мёртвое поле, ради которого коробку и снимали. */
export type BadgeExtent = { w: number; h: number };

/**
 * Сколько значок ЗАНИМАЕТ на самом деле, в cqi.
 *
 * Без подложки занимает цифра, и мерить по кружку, которого не рисуют, значит
 * городить вокруг неё мёртвое поле — то самое, которое видно и в отступе
 * шапки, и в том, куда значок вообще пускают: кружок вдвое крупнее цифры не
 * подпускал её к краю карты на полкружка пустоты.
 *
 * Мерится по ЧИСЛУ, которое на значке напечатано, а не по двузначному с
 * запасом, как было. Запас был заведён затем, чтобы мерка не обрезала вторую
 * цифру, — но рисуется-то плашка по месту (`width: auto`), и на однозначном
 * числе мерка выходила на четыре с половиной cqi шире нарисованного: половина
 * этого с каждой стороны и была тем полем, из-за которого значок не удавалось
 * прижать к самому краю карты. Число значку известно; гадать про него не
 * нужно, и поэтому `value` — не необязательный довесок, а обязательный довод:
 * пусть лучше не соберётся тот, кто забыл его передать, чем разойдутся
 * отрисовщик, перетаскивание и отступ шапки.
 *
 * Прозрачная заливка коробку НЕ снимает: форму хранитель выбрал, и коробка —
 * это форма. Снимает её только «без формы», и правило целиком: нет формы — нет
 * коробки.
 */
export function badgeExtent(
  frame: BattleFrame,
  kind: BadgeKind,
  value: number | null | undefined,
): BadgeExtent {
  const shape = badgeShape(frame, kind);
  const scale = badgeScale(frame, kind);
  const marked = badgeWearsMark(frame, kind);
  // Кегль числа — тот же, что в CSS: 6.6cqi, помноженные на плотность рамы и
  // на величину значка.
  const font = BADGE_BARE * scale * clampScale(frame.typeScale, 0.75, 1.5);
  const figures =
    badgeFigures(value, badgeWeight(frame, kind) || BADGE_WEIGHT_HOME) * font;
  const withMark = marked ? (BADGE_GLYPH + BADGE_GLYPH_GAP) * scale : 0;
  // Жетон — это и есть нарисованная подложка, поэтому коробка у него та же,
  // что у формы, даже когда форма снята: «нет формы — нет коробки» сказано про
  // одинокую цифру, а под цифрой с жетоном коробка нарисована.
  //
  // И коробка эта КВАДРАТНАЯ, раз и навсегда: `.corner--plate` назначает
  // ширину числом, а не по месту, — жетон рисуют целиком, и растягивать чужую
  // картинку под трёхзначное число никто не станет. Мерка, выросшая вместе с
  // числом, отодвигала бы жетон от края карты тем дальше, чем больше на нём
  // напечатано, — при том что нарисован он всегда одинаково.
  if (badgePlate(frame, kind)) {
    return { w: BADGE_SIZE * scale, h: BADGE_SIZE * scale };
  }
  if (shape !== "none") {
    // Ширину держит либо содержимое с полями, либо наименьшая ширина кружка —
    // ровно как в CSS, где стоят `padding` и `min-width`. Со знаком коробка
    // ШИРЕ своей высоты: знак стоит рядом с цифрой, и квадратная мерка отдала
    // бы карте отступ под кружок там, где нарисована плашка.
    return {
      w: Math.max(
        BADGE_SIZE * scale,
        figures + withMark + BADGE_PAD * 2 * scale,
      ),
      h: BADGE_SIZE * scale,
    };
  }
  return {
    w: figures + withMark,
    h: marked ? Math.max(font, BADGE_GLYPH * scale) : font,
  };
}

/**
 * Носит ли значок свой знак.
 *
 * Не носит только под жетоном: жетон — чужой рисунок, и знак, положенный
 * рядом с цифрой внутри плашки, растянул бы этот рисунок ради места, которого
 * на нём не рисовали. Без формы — носит: там нет подложки, но есть цифра, а
 * знак заводили ради неё, а не ради подложки.
 *
 * Спрашивается ОДНОЙ функцией, потому что ответ нужен трижды и врозь: коробке
 * (`badgeExtent`), отрисовщику и отступу шапки. Разошедшись, они дали бы
 * отступ под плашку, которой нет.
 */
export function badgeWearsMark(frame: BattleFrame, kind: BadgeKind): boolean {
  // На готовой иллюстрации знак снимают руками (`glyph: false`) — там, где он
  // уже нарисован на картинке. По умолчанию он есть: пустой медальон с цифрой
  // не говорит, стоимость это или сила.
  return !badgePlate(frame, kind) && glyphMode(frame, kind) === "beside";
}

// ── Знак числа отдельно от числа ────────────────────────────────────────────
//
// На готовой картинке под знак часто нарисовано своё место — щит над цифрой,
// сердце сбоку, — и знак, приклеенный к цифре, туда не встаёт. Поэтому у
// знака три положения: нет его, рядом с цифрой (как везде на сайте) и
// отдельно — тогда у него своё место и своя величина, и таскают его сам по
// себе. Хранится в метке числа (`glyphX`/`glyphY`/`glyphSize`), а не в
// отдельной: знак принадлежит числу, и снятое число уносит свой знак.

export type GlyphSlot = BadgeKind | FreeStatSlot;
export type GlyphMode = "off" | "beside" | "apart";

/** Кегль отдельного знака при множителе 1, в cqi. */
export const FREE_GLYPH_BASE = 7;

export function glyphMode(frame: BattleFrame, slot: GlyphSlot): GlyphMode {
  const own = frame.freeMarks?.[slot];
  if (own?.glyph === false) return "off";
  if (isFreeform(frame) && finite(own?.glyphX) != null && finite(own?.glyphY) != null) {
    return "apart";
  }
  return "beside";
}

/** Где и какой величины стоит отдельный знак; `null` — он не отдельно. */
export function freeGlyphAt(
  frame: BattleFrame,
  slot: GlyphSlot,
): { x: number; y: number; size: number } | null {
  if (glyphMode(frame, slot) !== "apart") return null;
  const own = frame.freeMarks?.[slot] ?? {};
  const size = finite(own.glyphSize) || 1;
  return {
    x: own.glyphX as number,
    y: own.glyphY as number,
    size: Math.min(FREE_MARK_SIZE_MAX, Math.max(FREE_MARK_SIZE_MIN, size)),
  };
}

/**
 * Перевести знак в положение. Отделённый встаёт чуть левее числа — рядом с
 * тем, к чему относится, но уже своей вещью, которую видно и можно взять;
 * `from` — где число стоит сейчас.
 */
export function setGlyphMode(
  target: BattleFrame | FrameOverride,
  slot: GlyphSlot,
  mode: GlyphMode,
  from: { x: number; y: number },
): void {
  if (mode === "off") {
    setFreeMark(target, slot, { glyph: false });
    return;
  }
  if (mode === "beside") {
    setFreeMark(target, slot, { glyph: undefined, glyphX: undefined, glyphY: undefined, glyphSize: undefined });
    return;
  }
  const x = from.x > 15 ? from.x - 9 : from.x + 9;
  setFreeMark(target, slot, { glyph: undefined, glyphX: x, glyphY: from.y });
}
export const BADGE_WEIGHTS = [300, 400, 500, 600, 700, 800];

/**
 * Заливка, разобранная на цвет и на то, сколько его.
 *
 * Прозрачность живёт ВНУТРИ цвета (`#rrggbbaa`), а не вторым полем, и это то
 * же решение, что «прозрачная — это цвет»: заливка остаётся одной записью,
 * которую CSS понимает сам, и снятая заливка есть просто нулевая плотность.
 * Домашний цвет нужен затем, что у «как в раме» и у `transparent` своего цвета
 * нет, а ползунок плотности обязан от чего-то отталкиваться.
 */
export function fillParts(
  fill: string,
  house: string,
): { hex: string; alpha: number } {
  const v = (fill ?? "").trim();
  if (!v) return { hex: house, alpha: 100 };
  const eight = /^#([0-9a-f]{6})([0-9a-f]{2})$/i.exec(v);
  if (eight) {
    return {
      hex: `#${eight[1]}`,
      alpha: Math.round((parseInt(eight[2], 16) / 255) * 100),
    };
  }
  if (badgeUnfilled(v)) return { hex: house, alpha: 0 };
  return { hex: v, alpha: 100 };
}

/** Обратная сборка. Цвет, который не удалось бы дописать байтом (хранитель
 *  вписал слово), возвращается как есть: лучше полная заливка, чем запись,
 *  которой браузер не поймёт и нарисует чёрным. */
export function fillJoin(hex: string, alpha: number): string {
  const a = Math.round(Math.min(100, Math.max(0, alpha)));
  if (!/^#[0-9a-f]{6}$/i.test(hex)) return hex;
  if (a >= 100) return hex;
  return `${hex}${Math.round((a / 100) * 255)
    .toString(16)
    .padStart(2, "0")}`;
}

/**
 * Цифра, выбранная за хранителя, когда он её не выбрал.
 *
 * Это УМОЛЧАНИЕ и только оно. Пара к заливке угадывается верно, пока цифра
 * лежит на заливке; на снятой заливке она лежит на резьбе, на фотографии, на
 * чужой картинке — и там ни одна краска рамы ничего не обещает. Поэтому
 * `costInk`/`powerInk` перебивают этот выбор всегда.
 */
export function badgeInk(fill: string, frame: BattleFrame): string {
  // Без заливки цифра лежит уже не на кружке, а на самой карте — на бумаге, на
  // резьбе, на фотографии, — и печатается она тем же, чем печатается на карте
  // всё остальное. Светлоту тут спрашивать не у чего: заливки нет.
  if (badgeUnfilled(fill)) return frame.ink;
  return lightness(fill) > 0.55 ? frame.ink : frame.paper;
}

/**
 * «Без заливки» — это ЦВЕТ, а не пустое место и не третье поле.
 *
 * Пустая строка уже занята и значит «как в раме», поэтому снятая заливка
 * хранится словом CSS. Оно того стоит: отрисовщику про этот случай знать
 * нечего — `--badge-fill` принимает `transparent` как любой другой цвет, ни
 * одной ветки не прибавилось. Знать нужно ровно одному месту, `badgeInk`,
 * потому что цифре теперь нужна не пара к кружку, а краска карты.
 */
export const BADGE_FILL_NONE = "transparent";

/** Снята ли заливка. Кроме своего слова принимает `none` и запись с нулевой
 *  прозрачностью: цифра, ставшая невидимой из-за незнакомой записи, — самая
 *  дорогая из ошибок, которые тут возможны. */
export function badgeUnfilled(fill: string): boolean {
  const v = fill.trim().toLowerCase();
  return (
    v === "transparent" ||
    v === "none" ||
    /^#[0-9a-f]{6}00$/.test(v) ||
    /^#[0-9a-f]{3}0$/.test(v)
  );
}

/** Светлота цвета, 0..1. Понимает `#rgb` и `#rrggbb` — то, что даёт
 *  `<input type="color">`; всё прочее возвращает «тёмный». */
function lightness(color: string): number {
  const hex = color.trim().replace(/^#/, "");
  const full =
    hex.length === 3
      ? hex
          .split("")
          .map((c) => c + c)
          .join("")
      : hex.length === 6
        ? hex
        : // Плотность на светлоту не влияет: полупрозрачная краска лежит на том,
          // подо что её положили, и это уже не вопрос к самой краске.
          hex.length === 8
          ? hex.slice(0, 6)
          : "";
  if (!/^[0-9a-fA-F]{6}$/.test(full)) return 0;
  const [r, g, b] = [0, 2, 4].map(
    (i) => parseInt(full.slice(i, i + 2), 16) / 255,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Докуда достаёт значок с подписью. Слово длиннее кружка, и «СТОИМОСТЬ» —
 *  самое длинное из тех, что дом печатает. */
const BADGE_WORD_REACH = 14;
/** Метка «новая» в правом верхнем углу — её ширина с отступом. */
const NEW_MARK_REACH = 22;

/**
 * Наряд принёс картинку рамы, но не принёс окна.
 *
 * Тогда карта носит чужую картинку в врезках ранга, и содержимое вылезает
 * поверх резьбы — ровно то, что видно на третьей и четвёртой картах полки.
 * Молча это не чинится: где у ЭТОЙ картинки дыра, знает только тот, кто её
 * рисовал. Стол говорит об этом словами, а не догадывается.
 */
export function dressWindowMissing(
  dress: FrameOverride | null | undefined,
): boolean {
  if (!dress) return false;
  const picture =
    !!dress.frameImage?.trim() ||
    !!dress.cornerImage?.trim() ||
    !!dress.sideImageH?.trim() ||
    !!dress.sideImageV?.trim();
  if (!picture) return false;
  return (
    dress.insetTop === undefined &&
    dress.insetRight === undefined &&
    dress.insetBottom === undefined &&
    dress.insetLeft === undefined
  );
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
    tier,
    nameEn,
    nameRu,
    paper,
    ink,
    border,
    foil,
    frameImage: "",
    frameMode: "overlay",
    frameScaleX: 1,
    frameScaleY: 1,
    paperImage: "",
    backImage: "",
    cornerImage: "",
    sideImageH: "",
    sideImageV: "",
    cornerExtra: "",
    sideMidH: "",
    sideMidV: "",
    slices: defaultSlices(),
    ornaments: [],
    insetTop: 0,
    insetRight: 0,
    insetBottom: 0,
    insetLeft: 0,
    aspect: DEFAULT_ASPECT,
    headerShare: DEFAULT_HEADER_SHARE,
    artShare: DEFAULT_ART_SHARE,
    footShare: DEFAULT_FOOT_SHARE,
    titleFont: "",
    titleInk: "",
    layout: "corners",
    sheet: defaultSheet(),
    typeScale: 1,
    inkFade: 1,
    costX: DEFAULT_COST_X,
    costY: DEFAULT_COST_Y,
    powerX: DEFAULT_POWER_X,
    powerY: DEFAULT_POWER_Y,
    costShape: "circle",
    powerShape: "circle",
    costFill: "",
    powerFill: "",
    costInk: "",
    powerInk: "",
    costSize: 1,
    powerSize: 1,
    costWeight: 0,
    powerWeight: 0,
    costPlate: "",
    powerPlate: "",
    healthShape: "",
    healthFill: "",
    healthInk: "",
    healthPlate: "",
    healthSize: 0,
    healthWeight: 0,
    healthX: null,
    healthY: null,
    freeNameX: null,
    freeNameY: null,
    freeNameSize: 1,
    freeLoreX: null,
    freeLoreY: null,
    freeLoreSize: 1,
    freeLoreFont: "",
    freeLoreInk: "",
  };
}

/**
 * The same five frames the server hands out — kept here so a card still has a
 * dress when the frames request fails, and so the admin preview can paint
 * before anything has been saved. The server's `battles::default_frames` is the
 * original; change both together.
 */
export const DEFAULT_FRAMES: BattleFrame[] = [
  painted(1, "Humble", "Скромная", "#f8f1e7", "#34251c", "#d8c6b1", ""),
  painted(2, "Sturdy", "Крепкая", "#f3e9db", "#34251c", "#c3ad93", ""),
  painted(
    3,
    "Remembered",
    "Памятная",
    "#eeddc8",
    "#34251c",
    "#a8845f",
    "rgba(198,95,60,0.16)",
  ),
  painted(
    4,
    "Rare",
    "Редкая",
    "#e6cfb2",
    "#2a1a11",
    "#6f3b24",
    "rgba(198,95,60,0.28)",
  ),
  painted(
    5,
    "Epic",
    "Эпическая",
    "#3a2a1e",
    "#f3e4cd",
    "#c99a52",
    "rgba(214,178,110,0.42)",
  ),
];

export const LAYOUTS: BattleLayout[] = ["corners", "plaque"];
/** Порядок — это предложение, а не перечень.
 *
 *  «Собрана из частей» стоит первой, потому что это единственный способ, в
 *  котором раму ДЕЛАЮТ: два других надевают готовую картинку целиком. Первый в
 *  списке — то, с чего начинают, и новая рама начинается именно с него. */
export const FRAME_MODES: BattleFrameMode[] = [
  "sliced",
  "overlay",
  "behind",
  "freeform",
];
export const BADGE_SHAPES: BattleBadgeShape[] = [
  "circle",
  "square",
  "diamond",
  "hex",
  "shield",
  "none",
];

export function clampTier(tier: number): number {
  if (!Number.isFinite(tier)) return 1;
  return Math.min(5, Math.max(1, Math.round(tier)));
}

/** A card is never left undressed: an unknown rank falls back to its default. */
export function frameFor(
  tier: number,
  frames: BattleFrame[] | null | undefined,
): BattleFrame {
  const rank = clampTier(tier);
  return (
    frames?.find((f) => clampTier(f.tier) === rank) ?? DEFAULT_FRAMES[rank - 1]
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
export type FrameOverride = Partial<
  Omit<BattleFrame, "tier" | "nameEn" | "nameRu">
>;

/** A frame taken off and folded into a dress — carving, paint and window.
 *
 *  Rank and name stay with the dictionary. The roster (`sheet`) and the two
 *  type multipliers stay with Card Face on the five ranks: a race that wore a
 *  frozen copy of the roster would hide later Face edits on the shelf forever.
 *  Wearing a preset onto a RANK still copies the roster explicitly there. */
export function dressOf(frame: BattleFrame): FrameOverride {
  const {
    tier: _tier,
    nameEn: _nameEn,
    nameRu: _nameRu,
    sheet: _sheet,
    typeScale: _typeScale,
    inkFade: _inkFade,
    ...dress
  } = frame;
  return { ...dress };
}

/** Race dresses may still carry a roster snapshotted before Card Face was
 *  its own desk — strip it so the rank's sheet reaches the shelf. */
function dressCarving(patch: FrameOverride | null): FrameOverride | null {
  if (!patch) return null;
  const {
    sheet: _sheet,
    typeScale: _typeScale,
    inkFade: _inkFade,
    ...dress
  } = patch;
  return dress;
}

/** A broken or empty override is the same as none: the tier's own frame. */
export function parseFrameOverride(
  raw: string | null | undefined,
): FrameOverride | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as FrameOverride;
    return parsed && typeof parsed === "object" ? parsed : null;
  } catch {
    return null;
  }
}

/** A race's own dress per level of an owned copy: 5 slots, index 0 = level 1.
 *  Anything unparseable or short comes back as 5 empty slots, never fewer. */
export function parseLevelFrames(
  raw: string | null | undefined,
): (FrameOverride | null)[] {
  const empty: (FrameOverride | null)[] = [null, null, null, null, null];
  if (!raw) return empty;
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return empty;
    return empty.map((_, i) =>
      parsed[i] && typeof parsed[i] === "object" ? parsed[i] : null,
    );
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
function patchFrame(
  base: BattleFrame,
  patch: FrameOverride | null,
): BattleFrame {
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
 *   1. the tier's shared frame — carving AND roster (Card Face)
 *   2. the card's race, dressed for this level of an owned copy (carving only;
 *      a level nobody dressed keeps the tier's own)
 *   3. this one card's own `frameOverride` — carving only, same reason
 * Roster never comes from a dress: Face edits the five ranks, and a frozen
 * copy on a race or a card would hide those edits on the shelf forever.
 * The rank itself and its name are the dictionary's alone at every layer.
 */
export function frameForCard(
  card: Pick<BattleCard, "tier" | "frameOverride" | "raceLevelFrames">,
  frames: BattleFrame[] | null | undefined,
  level?: number | null,
): BattleFrame {
  let frame = frameFor(card.tier, frames);
  const levelFrames = parseLevelFrames(card.raceLevelFrames);
  frame = patchFrame(
    frame,
    dressCarving(levelFrames[clampTier(level ?? 1) - 1]),
  );
  frame = patchFrame(
    frame,
    dressCarving(parseFrameOverride(card.frameOverride)),
  );
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
  return frame.frameMode === "sliced" && isDressed(frame);
}

/** The picture lies on top and the card shows through the hole in it. */
export function isOverlaid(frame: BattleFrame): boolean {
  return isDressed(frame) && frame.frameMode !== "behind";
}

/** A whole ready-made illustration — paper, carving and window baked into one
 *  picture — under which the photograph sits full-bleed and over which the
 *  name and the lore stand wherever the keeper dragged them. Reuses the same
 *  cut-out carving `isOverlaid` already draws; what changes is `.content`,
 *  which stops being four measured bands and becomes one full-bleed picture
 *  plus two free-standing labels. */
export function isFreeform(frame: BattleFrame): boolean {
  return frame.frameMode === "freeform";
}

/** «Готовая карта»: картинка несёт всё, включая фотографию работы, и второй
 *  фотографии под ней не кладут. */
export function isBaked(frame: BattleFrame): boolean {
  return isFreeform(frame) && !!frame.artBaked;
}

// ── Метки поверх иллюстрации (`freeform`) ─────────────────────────────────
//
// Слова и числа карты, поставленные туда, где под них на картинке оставлено
// место. Одна таблица на все: место, величина, ширина строки, шрифт, чернила.
// Значки стоимости, силы и здоровья стоят в том же списке ради одного поля —
// `shown`: место и вид у них свои (§ 9.9), второй способ их ставить разошёлся
// бы с первым.

export const FREE_TEXT_SLOTS = [
  "title",
  "kind",
  "effect",
  "traits",
  "lore",
] as const satisfies readonly FreeSlot[];
export const FREE_STAT_SLOTS = [
  "mana",
  "armor",
  "ward",
  "reach",
  "step",
  "speed",
  "mend",
] as const satisfies readonly FreeSlot[];
export type FreeTextSlot = (typeof FREE_TEXT_SLOTS)[number];
export type FreeStatSlot = (typeof FREE_STAT_SLOTS)[number];
/** Метки, которые печатает сама карта. Значки рисует свой слой. */
export type FreePrintedSlot = FreeTextSlot | FreeStatSlot;
export const FREE_PRINTED_SLOTS: FreePrintedSlot[] = [
  ...FREE_TEXT_SLOTS,
  ...FREE_STAT_SLOTS,
];
/** Все места в порядке, в котором их перечисляет стол. Зеркало в
 *  `battles.rs` (`FREE_SLOTS`), менять вместе. */
export const FREE_SLOTS: FreeSlot[] = [
  ...FREE_TEXT_SLOTS,
  "cost",
  "power",
  "health",
  ...FREE_STAT_SLOTS,
];

export function isFreeStat(slot: FreeSlot): slot is FreeStatSlot {
  return (FREE_STAT_SLOTS as readonly FreeSlot[]).includes(slot);
}

export function isFreeBadge(slot: FreeSlot): slot is BadgeKind {
  return slot === "cost" || slot === "power" || slot === "health";
}

/** Имя места — для стола. У чисел это то же слово, что везде (`statLabel`). */
export function freeMarkLabel(slot: FreeSlot): TranslationKey {
  switch (slot) {
    case "title": return "adminBattlesTitle";
    case "kind": return "adminBattlesReadyKind";
    case "effect": return "adminBattlesEffect";
    case "traits": return "adminBattlesTraits";
    case "lore": return "adminBattlesLore";
    default: return statLabel(slot);
  }
}

/** Пределы. Зеркало в `battles.rs`, менять вместе. */
export const FREE_MARK_SIZE_MIN = 0.3;
export const FREE_MARK_SIZE_MAX = 4;
export const FREE_MARK_WIDTH_MIN = 10;
export const FREE_MARK_WIDTH_MAX = 100;

interface FreeHome {
  x: number;
  y: number;
  /** Ширина строки, % карты. Ноль — по содержимому (числа). */
  width: number;
  /** Кегль при множителе 1, в cqi. */
  base: number;
  italic: boolean;
  caps: boolean;
  shown: boolean;
}

/**
 * Где метка встаёт, пока её не двигали, и какой она величины. Имя и
 * приписка видны сразу — так было и до меток; остальное включают на столе.
 * Числа разведены по краям, чтобы включённые разом не легли друг на друга.
 */
const FREE_HOME: Record<FreePrintedSlot, FreeHome> = {
  title: { x: DEFAULT_FREE_NAME_X, y: DEFAULT_FREE_NAME_Y, width: 84, base: 7, italic: false, caps: false, shown: true },
  kind: { x: 50, y: 53, width: 74, base: 3.4, italic: false, caps: true, shown: false },
  effect: { x: 50, y: 72, width: 76, base: 4.2, italic: false, caps: false, shown: false },
  traits: { x: 50, y: 66, width: 76, base: 3.8, italic: false, caps: false, shown: false },
  lore: { x: DEFAULT_FREE_LORE_X, y: DEFAULT_FREE_LORE_Y, width: 76, base: 4.4, italic: true, caps: false, shown: true },
  mana: { x: 86, y: 12, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  armor: { x: 14, y: 30, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  ward: { x: 86, y: 30, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  reach: { x: 14, y: 46, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  step: { x: 86, y: 46, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  speed: { x: 14, y: 62, width: 0, base: 6.6, italic: false, caps: false, shown: false },
  mend: { x: 86, y: 62, width: 0, base: 6.6, italic: false, caps: false, shown: false },
};

/** Где метка встаёт, пока её не двигали. */
export function freeHomeAt(slot: FreePrintedSlot): { x: number; y: number } {
  return { x: FREE_HOME[slot].x, y: FREE_HOME[slot].y };
}

/** Чернила, которые читаются на картинке: светлые там, где она тёмная, и
 *  тёмные там, где светлая. Пара домашняя — бумага и текст сайта. */
export const READY_INK_DARK = "#34251c";
export const READY_INK_LIGHT = "#f4ead8";

/**
 * Какие чернила видны в этой точке картинки.
 *
 * Метку ставят на нарисованную плашку, и цвет текста «как у ранга» — это
 * цвет, мерянный по чужой картинке: тёмный текст на тёмной резьбе не виден,
 * и хранитель тыкал бы по карте наугад. Поэтому чернила выбирает сама
 * картинка: средняя светлота пятна вокруг метки (±6 % по ширине, ±3 % по
 * высоте) решает, светлые они или тёмные. Картинка уменьшается до 100 × 140
 * — для среднего по пятну этого довольно, и проход ничего не стоит.
 *
 * `null` — картинку прочитать не удалось (не загрузилась, чужой адрес без
 * разрешения на чтение): тогда остаются чернила ранга, а не догадка.
 */
export async function pictureInks(
  source: Blob | string,
): Promise<((x: number, y: number) => string) | null> {
  try {
    let picture: CanvasImageSource;
    if (typeof source === "string") {
      const image = new Image();
      image.decoding = "async";
      image.src = source;
      await image.decode();
      picture = image;
    } else {
      picture = await createImageBitmap(source);
    }
    const W = 100;
    const H = 140;
    const canvas = document.createElement("canvas");
    canvas.width = W;
    canvas.height = H;
    const pen = canvas.getContext("2d", { willReadFrequently: true });
    if (!pen) return null;
    pen.drawImage(picture, 0, 0, W, H);
    const pixels = pen.getImageData(0, 0, W, H).data;
    const linear = (c: number) => {
      const v = c / 255;
      return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    };
    return (x: number, y: number) => {
      const x0 = Math.max(0, Math.floor(((x - 6) / 100) * W));
      const x1 = Math.min(W - 1, Math.ceil(((x + 6) / 100) * W));
      const y0 = Math.max(0, Math.floor(((y - 3) / 100) * H));
      const y1 = Math.min(H - 1, Math.ceil(((y + 3) / 100) * H));
      let sum = 0;
      let count = 0;
      for (let row = y0; row <= y1; row++) {
        for (let col = x0; col <= x1; col++) {
          const at = (row * W + col) * 4;
          const alpha = pixels[at + 3] / 255;
          // Прозрачное читается как бумага сайта под картой — светлое.
          const lum =
            0.2126 * linear(pixels[at]) + 0.7152 * linear(pixels[at + 1]) + 0.0722 * linear(pixels[at + 2]);
          sum += lum * alpha + 0.87 * (1 - alpha);
          count++;
        }
      }
      // 0.18 — середина по контрасту между двумя чернилами, а не 0.5: глаз
      // мерит светлоту не линейно, и «серое на вид» лежит около пятой части.
      return count && sum / count < 0.18 ? READY_INK_LIGHT : READY_INK_DARK;
    };
  } catch {
    return null;
  }
}

/** Метка, разрешённая до конца: всё, что нужно, чтобы её нарисовать. */
export interface FreeMarkLook {
  x: number;
  y: number;
  size: number;
  width: number;
  align: FreeMarkAlign;
  /** Начертание, id из `SITE_FONTS`. Пусто — шрифт карты. */
  font: string;
  ink: string;
  bold: boolean;
  italic: boolean;
  caps: boolean;
  base: number;
  shown: boolean;
  /** Знак рядом с числом. Только у чисел; у слов всегда `false`. */
  glyph: boolean;
}

function finite(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/**
 * Как метка выглядит на этой раме.
 *
 * Лестница отката одна: своё поле метки → у имени и приписки прежние поля
 * (`freeName*`, `titleFont`/`titleInk`, `freeLore*`), которыми рама
 * пользовалась до меток → дом метки. Прежние поля только ЧИТАЮТСЯ: пишет
 * стол всегда в `freeMarks`, иначе у одной вещи было бы два дома.
 */
export function freeMarkOf(frame: BattleFrame, slot: FreePrintedSlot): FreeMarkLook {
  const own: FreeMark = frame.freeMarks?.[slot] ?? {};
  const home = FREE_HOME[slot];
  // У готовой карты прежних полей нет вовсе: они мерены по картинке ранга, а
  // на этой картинке место под имя нарисовано в другом месте и другим цветом.
  const legacy = isBaked(frame)
    ? { x: null, y: null, size: null, font: "", ink: "" }
    : slot === "title"
      ? {
          x: finite(frame.freeNameX),
          y: finite(frame.freeNameY),
          size: finite(frame.freeNameSize),
          font: frame.titleFont ?? "",
          ink: frame.titleInk ?? "",
        }
      : slot === "lore"
        ? {
            x: finite(frame.freeLoreX),
            y: finite(frame.freeLoreY),
            size: finite(frame.freeLoreSize),
            font: frame.freeLoreFont ?? "",
            ink: frame.freeLoreInk ?? "",
          }
        : { x: null, y: null, size: null, font: "", ink: "" };
  const size = finite(own.size) || legacy.size || 1;
  const width = finite(own.width) ?? home.width;
  return {
    x: finite(own.x) ?? legacy.x ?? home.x,
    y: finite(own.y) ?? legacy.y ?? home.y,
    size: Math.min(FREE_MARK_SIZE_MAX, Math.max(FREE_MARK_SIZE_MIN, size)),
    width: width ? Math.min(FREE_MARK_WIDTH_MAX, Math.max(FREE_MARK_WIDTH_MIN, width)) : 0,
    align: own.align ?? "center",
    font: own.font?.trim() || legacy.font.trim(),
    ink: own.ink?.trim() || legacy.ink.trim() || frame.ink,
    bold: own.bold ?? false,
    italic: own.italic ?? home.italic,
    caps: home.caps,
    base: home.base,
    shown: own.shown ?? home.shown,
    glyph: isFreeStat(slot) ? glyphMode(frame, slot) === "beside" : false,
  };
}

/** Стиль метки одной строкой — для карты, единственного её отрисовщика. */
export function freeMarkStyle(look: FreeMarkLook): string {
  return [
    `left:${look.x}%`,
    `top:${look.y}%`,
    look.width ? `width:${look.width}%` : "",
    `--fm-size:${(look.base * look.size).toFixed(3)}cqi`,
    `text-align:${look.align}`,
    `color:${look.ink}`,
    look.font ? `font-family:${fontStack(look.font)}` : "",
    `font-weight:${look.bold ? 700 : 400}`,
    `font-style:${look.italic ? "italic" : "normal"}`,
  ]
    .filter(Boolean)
    .join(";");
}

/**
 * Значок на готовой иллюстрации: назначенное меткой, иначе `null` — и тогда
 * решает опись, как решала до меток. Назначенное решает на любой величине
 * карты: место под число нарисовано на самой картинке.
 */
export function freeBadgeShown(frame: BattleFrame, kind: BadgeKind): boolean | null {
  if (!isFreeform(frame)) return null;
  return frame.freeMarks?.[kind]?.shown ?? null;
}

/** Записать в метку. Пустое (`undefined`) снимает поле — метка вернётся к
 *  откату, а не запомнит «ничего» как выбор. */
export function setFreeMark(
  target: BattleFrame | FrameOverride,
  slot: FreeSlot,
  patch: Partial<Record<keyof FreeMark, FreeMark[keyof FreeMark] | undefined>>,
): void {
  const marks = (target.freeMarks ??= {});
  const own = (marks[slot] ??= {}) as Record<string, unknown>;
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined) delete own[key];
    else own[key] = value;
  }
  if (!Object.keys(own).length) delete marks[slot];
}

/** Отношение сторон, которое считается «той же формой», что у карты. */
export const READY_ASPECT_SLACK = 0.03;

/**
 * Наряд «Готовой карты» для только что загруженной картинки.
 *
 * Прежний наряд готовой карты переживает замену картинки целиком — места,
 * шрифты, всё, что поставлено руками: картинку перерисовали, а место под
 * имя на ней то же. С чужого наряда (своя рамка, пресет) не берётся ничего.
 * Значки — голая цифра (`none`) со знаком рядом: подложка нарисована на
 * картинке, а что значит число, говорит знак (снимается в панели значка, если
 * на картинке нарисован и он). Места и чернила назначены
 * явно, а не взяты у ранга: ранговые мерены по чужой картинке, а голая цифра
 * в чернилах по умолчанию — бумажная, то есть светлая по светлому. `ink` —
 * чернила ранга, те же, в которых по умолчанию печатаются слова.
 */
export function readyCardDress(
  url: string,
  before: FrameOverride | null,
  ink: string,
  inkAt: ((x: number, y: number) => string) | null = null,
): FrameOverride {
  if (before?.frameMode === "freeform" && before.artBaked) {
    return { ...before, frameImage: url };
  }
  const dress: FrameOverride = {
    frameMode: "freeform",
    artBaked: true,
    frameImage: url,
    frameScaleX: 1,
    frameScaleY: 1,
    insetTop: 0,
    insetRight: 0,
    insetBottom: 0,
    insetLeft: 0,
    layout: "corners",
    costShape: "none",
    powerShape: "none",
    healthShape: "none",
    costInk: ink,
    powerInk: ink,
    healthInk: ink,
    costX: DEFAULT_COST_X,
    costY: DEFAULT_COST_Y,
    powerX: DEFAULT_POWER_X,
    powerY: DEFAULT_POWER_Y,
    healthX: DEFAULT_COST_X,
    healthY: DEFAULT_POWER_Y,
    freeMarks: {
      title: { shown: true },
      effect: { shown: true },
      lore: { shown: false },
      cost: { shown: true },
      power: { shown: true },
      health: { shown: true },
    },
  };
  if (inkAt) inkByPicture(dress, inkAt);
  return dress;
}

/**
 * Перекрасить слова и числа наряда под картинку — каждую метку и каждый
 * значок по тому месту, где они стоят СЕЙЧАС. Пишет поверх выбранного
 * руками: это кнопка «подобрать», а не умолчание.
 */
export function inkByPicture(
  dress: FrameOverride,
  inkAt: (x: number, y: number) => string,
): void {
  for (const slot of FREE_PRINTED_SLOTS) {
    const own = dress.freeMarks?.[slot];
    const home = FREE_HOME[slot];
    setFreeMark(dress, slot, { ink: inkAt(own?.x ?? home.x, own?.y ?? home.y) });
  }
  for (const kind of BADGE_KINDS) {
    const keys = BADGE_FIELDS[kind];
    const x = (dress[keys.x] as number | null | undefined) ?? keys.homeX;
    const y = (dress[keys.y] as number | null | undefined) ?? keys.homeY;
    (dress as Record<string, unknown>)[keys.ink] = inkAt(x, y);
  }
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
    "--paper-image": paperArt ? `url("${cssUrl(paperArt)}")` : "none",
    "--paper": frame.paper,
    "--ink": frame.ink,
    "--edge": frame.border,
    "--foil": frame.foil || "transparent",
    "--frame-image": image ? `url("${cssUrl(image)}")` : "none",
    // Множитель, не размер — та же причина, что у `--type-scale`: рамка не
    // назначает пиксели, а масштабирует уже нарисованную картинку вокруг её
    // же центра. `sliced` эти два не читает — читает грамматика CSS-класса
    // `.carving`, которого у нарезанной рамы просто нет.
    "--frame-scale-x": String(clampScale(frame.frameScaleX, BADGE_SCALE_MIN, BADGE_SCALE_MAX)),
    "--frame-scale-y": String(clampScale(frame.frameScaleY, BADGE_SCALE_MIN, BADGE_SCALE_MAX)),
    "--back-image": backArt ? `url("${cssUrl(backArt)}")` : "none",
    "--corner-image": cornerArt ? `url("${cssUrl(cornerArt)}")` : "none",
    "--side-image-h": sideHArt ? `url("${cssUrl(sideHArt)}")` : "none",
    "--side-image-v": sideVArt ? `url("${cssUrl(sideVArt)}")` : "none",
    "--corner-extra-image": cornerExtraArt
      ? `url("${cssUrl(cornerExtraArt)}")`
      : "none",
    "--side-mid-h-image": sideMidHArt
      ? `url("${cssUrl(sideMidHArt)}")`
      : "none",
    "--side-mid-v-image": sideMidVArt
      ? `url("${cssUrl(sideMidVArt)}")`
      : "none",
    // `freeform` читает те же врезки, что и `overlay`: готовая иллюстрация
    // несёт своё окно, но где именно оно вырезано в ЭТОЙ картинке, знает
    // только тот, кто её рисовал, — а не число, общее для всех.
    "--pad-top": `${frame.insetTop || 0}%`,
    "--pad-right": `${frame.insetRight || 0}%`,
    "--pad-bottom": `${frame.insetBottom || 0}%`,
    "--pad-left": `${frame.insetLeft || 0}%`,
    "--aspect": String(frame.aspect || DEFAULT_ASPECT),
    // The three measured bands. The properties band is not here on purpose: it
    // takes whatever these three leave, so it can never be squeezed to nothing
    // by three sliders that happen to add up.
    "--header-share": `${((frame.headerShare ?? DEFAULT_HEADER_SHARE) * 100).toFixed(1)}%`,
    "--art-share": `${((frame.artShare || DEFAULT_ART_SHARE) * 100).toFixed(1)}%`,
    "--foot-share": `${((frame.footShare ?? DEFAULT_FOOT_SHARE) * 100).toFixed(1)}%`,
    "--title-face": frame.titleFont ? fontStack(frame.titleFont) : "inherit",
    "--title-ink": frame.titleInk?.trim() || frame.ink,
    // Кегль и насыщенность — множители, а не размеры. Размеры карта считает
    // сама из своей ширины, и рамка, назначающая пиксели, отняла бы у неё
    // ровно то, ради чего она их считает.
    "--type-scale": String(clampScale(frame.typeScale, 0.75, 1.5)),
    "--ink-fade": String(clampScale(frame.inkFade, 0.5, 1.6)),
  };
}

/** Множитель рамки, приведённый к делу. Ноль и мусор — это «не назначено»,
 *  а не «стереть текст»: рамка, сохранённая до кегля, несёт ноль. Экспортирован
 *  ради стола рамок: тот же самый разбор нужен и там, где число не в CSS-строку
 *  идёт, а печатается словом рядом с ползунком, — второй, более наивный разбор
 *  там однажды упал бы на `undefined`, которого сервер до своей пересборки не
 *  посылал вовсе. */
export function clampScale(
  given: number | undefined,
  min: number,
  max: number,
): number {
  if (!Number.isFinite(given) || !given || given <= 0) return 1;
  return Math.min(max, Math.max(min, given as number));
}

/** The card's four insets — how far the window sits from each side of the
 *  photograph, in % of the card. */
export type InsetKey = "insetTop" | "insetRight" | "insetBottom" | "insetLeft";
export const INSET_MAX = 45;

const OPPOSITE_INSET: Record<InsetKey, InsetKey> = {
  insetTop: "insetBottom",
  insetBottom: "insetTop",
  insetLeft: "insetRight",
  insetRight: "insetLeft",
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
  const ru = (frame.nameRu ?? "").trim();
  const en = (frame.nameEn ?? "").trim();
  return (lang === "ru" ? ru || en : en || ru) || String(frame.tier);
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
  const s = own?.trim() ?? "";
  if (!s) return "";
  if (lang === "en" && mostlyCyrillic(s)) return "";
  return s;
}

/** Больше половины букв — кириллица. Такой текст в английском поле карта не
 *  печатает (`lineInLang`), и стол обязан сказать об этом словами. */
export function mostlyCyrillic(s: string): boolean {
  const letters = [...s].filter((ch) => /\p{L}/u.test(ch));
  if (!letters.length) return false;
  const cyr = letters.filter((ch) => /\p{Script=Cyrillic}/u.test(ch)).length;
  return cyr * 2 >= letters.length;
}

export function cardCopy(
  card: BattleCard,
  lang: Lang,
): { title: string; effect: string; lore: string } {
  const ru = lang === "ru";
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
  const ru = lang === "ru";
  const name = lineInLang(ru ? trait.nameRu : trait.nameEn, lang);
  const other = lineInLang(ru ? trait.nameEn : trait.nameRu, ru ? "en" : "ru");
  const text = lineInLang(ru ? trait.textRu : trait.textEn, lang);
  return { name, other: name && other && name !== other ? other : "", text };
}

/** The ability's own name in the reader's language. The verb is a dictionary
 *  word printed elsewhere — this is only what the keeper wrote on it. */
export function abilityCopy(
  ability: CardAbility,
  lang: Lang,
): { name: string } {
  const ru = lang === "ru";
  return { name: lineInLang(ru ? ability.nameRu : ability.nameEn, lang) };
}

/** The header band: what this is. Kind is a dictionary word, printed elsewhere;
 *  free `type` is no longer the header, and a digit in the field is not a type. */
export function headerCopy(
  card: BattleCard,
  lang: Lang,
): { race: string; type: string } {
  const ru = lang === "ru";
  const typeRaw = lineInLang(ru ? card.typeRu : card.typeEn, lang);
  const type = /^\d+$/.test(typeRaw) ? "" : typeRaw;
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

/** The floor `zoom` sanitizes to. A windowed card never asks for less than 1
 *  — the photograph must always cover the hole in the carving — but
 *  `freeform`'s picture sits full-bleed on its own paper, and there shrinking
 *  it on purpose (to see the whole photograph, matted by the illustration
 *  around it) is the point, not a mistake to clamp away. One shared floor,
 *  loose enough for both: the windowed aim only ever asks above 1 anyway, so
 *  loosening the sanitizer's floor doesn't hand it a value it would use. */
export const FOCAL_ZOOM_MIN = 0.2;

/** A card with a broken focus is centred, never blank. */
export function parseFocal(raw: string | null | undefined): Focal {
  if (!raw) return CENTRED;
  try {
    const parsed = JSON.parse(raw) as Partial<Focal>;
    const num = (v: unknown, fallback: number, lo: number, hi: number) =>
      typeof v === "number" && Number.isFinite(v)
        ? Math.min(hi, Math.max(lo, v))
        : fallback;
    return {
      x: num(parsed.x, 0.5, 0, 1),
      y: num(parsed.y, 0.5, 0, 1),
      zoom: num(parsed.zoom, 1, FOCAL_ZOOM_MIN, 3),
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
    out.push({ coin: "dust", amount: card.priceDust });
  }
  if (card.priceFeed != null && card.priceFeed > 0) {
    out.push({ coin: "feed", amount: card.priceFeed });
  }
  return out;
}

// ── Способности: имена, значки, потолок ─────────────────────────────────────
//
// Живут ЗДЕСЬ, а не в столе хозяина, по той же причине, по которой в общем
// месте живёт `BattleIcon`: этими словами и значками способность подписана и в
// админке, и на столе человека в студии, и второй такой список однажды
// разошёлся бы с первым — глагол, добавленный в доме, не появился бы у людей.
//
// Списки ЗАКРЫТЫ и повторяют `battles.rs`. Повторяются намеренно: сервер
// отбрасывает неизвестный глагол молча, а форма не должна давать его выбрать.

export const ABILITIES_MAX = 6;

// Ключ перевода рядом с самим значением, а не собранный из строки: тогда
// забытый в словаре глагол — ошибка компиляции, а не «battlesVerbFoo» на
// экране. Порядок записи здесь и есть порядок в списке.
export const VERB_LABELS = {
  damage: "battlesVerbDamage",
  dot: "battlesVerbDot",
  heal: "battlesVerbHeal",
  hot: "battlesVerbHot",
  shield: "battlesVerbShield",
  zone: "battlesVerbZone",
  bless: "battlesVerbBless",
  curse: "battlesVerbCurse",
  control: "battlesVerbControl",
  silence: "battlesVerbSilence",
  disarm: "battlesVerbDisarm",
  charm: "battlesVerbCharm",
  veil: "battlesVerbVeil",
  guard: "battlesVerbGuard",
  immune: "battlesVerbImmune",
  thorns: "battlesVerbThorns",
  move: "battlesVerbMove",
  summon: "battlesVerbSummon",
  sacrifice: "battlesVerbSacrifice",
  cleanse: "battlesVerbCleanse",
  dispel: "battlesVerbDispel",
  mana: "battlesVerbMana",
} as const satisfies Record<AbilityVerb, TranslationKey>;

export const SHAPE_LABELS = {
  self: "battlesShapeSelf",
  one: "battlesShapeOne",
  adjacent: "battlesShapeAdjacent",
  chain: "battlesShapeChain",
  line: "battlesShapeLine",
  radius: "battlesShapeRadius",
  side: "battlesShapeSide",
  cell: "battlesShapeCell",
} as const satisfies Record<AbilityShape, TranslationKey>;

export const TRIGGER_LABELS = {
  active: "battlesTriggerActive",
  onPlay: "battlesTriggerOnPlay",
  onHit: "battlesTriggerOnHit",
  onDamaged: "battlesTriggerOnDamaged",
  onDeath: "battlesTriggerOnDeath",
  turnStart: "battlesTriggerTurnStart",
  aura: "battlesTriggerAura",
  once: "battlesTriggerOnce",
} as const satisfies Record<AbilityTrigger, TranslationKey>;

/** Значки каналов. Формам и поводам такой таблицы не нужно: их значки
 *  названы теми же словами, что и сами значения, и разойтись им негде. */
export const CHANNEL_ICON = {
  physical: "sword",
  magic: "spark",
  pure: "pure",
  none: "nil",
} as const satisfies Record<BattleChannel, string>;

export const CHANNELS = Object.keys(CHANNEL_ICON) as BattleChannel[];

// Слова каналов — ПУБЛИЧНЫЕ, те же, которыми канал подписан на листе взятия.
// Админский набор повторял их слово в слово; двух словарей для четырёх слов
// быть не должно, и второй удалён.
export const CHANNEL_LABELS = {
  physical: "battlesChannelPhysical",
  magic: "battlesChannelMagic",
  pure: "battlesChannelPure",
  none: "battlesChannelNone",
} as const satisfies Record<BattleChannel, TranslationKey>;

/**
 * Значки глаголов.
 *
 * Таблица, а не совпадение имён, как у форм и поводов: пять глаголов носят
 * значок, который уже есть у чего-то другого и означает то же самое, —
 * «урон» это меч, «мана» это капля, — и рисовать им вторые такие же было бы
 * два рисунка одного предмета, которые однажды разойдутся.
 */
export const VERB_ICON = {
  damage: "sword",
  dot: "flame",
  heal: "sprig",
  hot: "bloom",
  shield: "shield",
  zone: "zone",
  bless: "bless",
  curse: "curse",
  control: "control",
  silence: "silence",
  disarm: "disarm",
  charm: "charm",
  veil: "veil",
  guard: "guard",
  immune: "immune",
  thorns: "thorns",
  move: "move",
  summon: "summon",
  sacrifice: "sacrifice",
  cleanse: "cleanse",
  dispel: "dispel",
  mana: "drop",
} as const satisfies Record<AbilityVerb, string>;

export const VERBS = Object.keys(VERB_LABELS) as AbilityVerb[];
export const SHAPES = Object.keys(SHAPE_LABELS) as AbilityShape[];
export const TRIGGERS = Object.keys(TRIGGER_LABELS) as AbilityTrigger[];

/** Только `chain` и `radius` несут число; у остальных поле нечего заполнять. */
export const shapeCarriesNumber = (shape: string) =>
  shape === "chain" || shape === "radius";

/** One dictionary word for the header: body, spell, or relic — never the free `type`. */
export function kindLabelKey(
  kind: BattleCardKind,
): "battlesKindUnit" | "battlesKindSpell" | "battlesKindRelic" {
  if (kind === "spell") return "battlesKindSpell";
  if (kind === "relic") return "battlesKindRelic";
  return "battlesKindUnit";
}

/**
 * The channel of the ordinary blow. Bodily is the default and stays silent;
 * anything else is a word the reader has not already assumed.
 */
export function channelLabelKey(
  channel: BattleChannel,
): "battlesChannelMagic" | "battlesChannelPure" | "battlesChannelNone" | null {
  if (channel === "magic") return "battlesChannelMagic";
  if (channel === "pure") return "battlesChannelPure";
  if (channel === "none") return "battlesChannelNone";
  return null;
}

export type BodyStatField =
  "health" | "mana" | "armor" | "ward" | "reach" | "step" | "mend";

/** i18n keys for the body passport — the scene already owns these words. */
export const BODY_STAT_LABELS = {
  health: "battlesHealthLabel",
  mana: "battlesManaLabel",
  armor: "battleStatArmour",
  ward: "battleStatWard",
  reach: "battleStatReach",
  step: "battleStatStep",
  mend: "battleStatMend",
} as const satisfies Record<BodyStatField, TranslationKey>;

/**
 * Число, у которого есть имя, есть и ЗНАК.
 *
 * Один список на весь дом — и это условие, а не опрятность. Знак стоит рядом с
 * числом в шести местах разом (лицо карты, лист взятия, разбор на сцене, три
 * плашки стола хранителя), и до этого списка он выбирался в каждом из них
 * по-своему: `icon="heart"` было вписано в разметку стола рукой, а карта и лист
 * не рисовали знака вовсе. Два места, называющие сердце для здоровья порознь,
 * — это два места, которые однажды назовут разное, и хранитель увидит на карте
 * одну картинку, а на своей плашке другую.
 *
 * Знак СОПРОВОЖДАЕТ слово, а не заменяет его, и это тоже решено не здесь:
 * сердце угадывают все, а «оберег» от «брони» по двум щитам не отличит никто
 * (см. `StatCell.svelte`). Единственное место, где знак стоит один, — кружок
 * значка: там нет слова и не было, туда не помещается даже цифра со словом.
 *
 * Имена — те, что понимает `BattleIcon`. Не путь к картинке: знаки рисуются
 * линиями в чернилах того места, где стоят, а не приходят файлом со склада,
 * который пришлось бы красить.
 */
export const STAT_MARKS = {
  health: "heart",
  mana: "drop",
  armor: "shield",
  ward: "ward",
  reach: "reach",
  step: "boot",
  mend: "sprig",
  /** Не в паспорте (`SHEET_STATS`), но на листе взятия стоит и печатается. */
  speed: "hourglass",
  cost: "coin",
  power: "sword",
} as const satisfies Record<BodyStatField | "speed" | BadgeKind, string>;

/** Слот, у которого есть знак. `healthMark` — не число, а кружок здоровья, и
 *  знак у него тот же, что у здоровья: это одно и то же число. */
export type MarkedStat = keyof typeof STAT_MARKS;

/**
 * Слово того же числа. `BODY_STAT_LABELS` называет семь из паспорта, а
 * стоимость, сила и скорость лежали по своим ключам и выписывались рукой в
 * каждом месте, где стояли рядом с паспортом, — тремя развилками там, где
 * нужен был один список. Ключи здесь буква в букву те же, что у `STAT_MARKS`:
 * у числа одно слово и один знак, и берутся они по одному имени.
 */
export const STAT_LABELS = {
  ...BODY_STAT_LABELS,
  speed: "battlesSpeedLabel",
  cost: "battlesCostLabel",
  power: "battlesPowerLabel",
} as const satisfies Record<MarkedStat, TranslationKey>;

/** Слово числа. Пара к `statMark`, и по той же причине функцией: `healthMark`
 *  — это здоровье под другим именем. */
export function statLabel(slot: MarkedStat | "healthMark"): TranslationKey {
  return STAT_LABELS[slot === "healthMark" ? "health" : slot];
}

// ── Чем заняться: намерения тела ─────────────────────────────────────────────
//
// Пока у тела был один способ действовать, выбора не существовало: подсветилось
// — ткнули. У карты, которая бьёт, лечит и проклинает на расстоянии, в одну и ту
// же чужую клетку ведут ТРИ разных дела, и «подсветилось» перестало отвечать на
// вопрос «что сейчас случится». Намерение — это один способ действовать вместе с
// уже готовым списком его целей.
//
// Цели не вычисляются. Каждая — готовое действие ИЗ `legalActions`, положенное
// в ящик под ключом тела, и отправляется оно назад неизменным: договор сцены
// («клиент не знает ни одного правила») держится ровно на этом.
//
// Правило здесь одно и только ради СЛОВ: `castingOf` — зеркало
// `AbilitySnapshot::casting` движка. Им ничего не играется; оно выбирает, какой
// значок нарисовать и какую чару показать спящей, когда её в законном списке
// нет. Та же терпимость и по той же причине, что у `HOUSE_RULES`, и тот же
// прецедент, что у `mendAsleep` в сцене: назвать отказ нельзя, не зная, что
// отказано. Разойдётся — спящая чара покажется лишней или не покажется вовсе;
// ни одного хода это не сделает.

/** Чем умение становится, когда его просят. Лечения здесь нет: оно играется
 *  `mend`, и игралось им до чар. Список — зеркало `Casting::of_verb` движка. */
export type Casting =
  | "harm"
  | "fester"
  | "knit"
  | "shield"
  | "bless"
  | "curse"
  | "bind"
  | "hush"
  | "disarm"
  | "sway"
  | "veil"
  | "guard"
  | "numb"
  | "thorns"
  | "shove"
  | "cleanse"
  | "dispel"
  | "coin"
  | "offer"
  | "zone"
  | "summon";

const CASTINGS: Record<string, Casting> = {
  damage: "harm",
  dot: "fester",
  hot: "knit",
  shield: "shield",
  bless: "bless",
  curse: "curse",
  control: "bind",
  silence: "hush",
  disarm: "disarm",
  charm: "sway",
  veil: "veil",
  guard: "guard",
  immune: "numb",
  thorns: "thorns",
  move: "shove",
  cleanse: "cleanse",
  dispel: "dispel",
  mana: "coin",
  sacrifice: "offer",
  zone: "zone",
  summon: "summon",
};

/** Чары, которые наводят на КЛЕТКУ, а не на тело. Форма `cell` у них не одна
 *  из многих, а единственная, какая у них бывает (§4). */
const SPOT_VERBS = ["zone", "summon"];

/**
 * Запрещённое §4 сочетание глагола и пригоршни: «✗ — запрещённые, не дорогие».
 *
 * Третье место, где это записано, — и терпимо оно по той же причине, что и всё
 * зеркало: играет правилом движок, отказывает словами стол, а здесь оно только
 * решает, показывать ли печать. Разойдётся — в веере окажется чара, которой
 * нечего делать; партию это не сдвинет.
 */
function forbiddenShape(verb: string, shape: string, radius: number): boolean {
  if (verb === "charm") return shape !== "one" && shape !== "self";
  if (verb === "control" || verb === "veil") {
    return shape === "side" || (shape === "radius" && radius >= 2);
  }
  return false;
}

/** Столько ходов отката значит НАВСЕГДА — зеркало `Unit::FOREVER`. */
export const FOREVER = 255;

/** Ключ умения: его `id`, а при пустом — место на карте. Зеркало `ability_key`
 *  движка — ключ приходит в действии, в откате и в имени всадника, и выдумать
 *  его второй раз по-своему значит потерять откат у безымянной чары. */
export function abilityKey(a: { id?: string | null }, index: number): string {
  const own = a.id?.trim();
  return own ? own : `#${index}`;
}

/** Чара, которую движок правда играет, — или ничего. */
export function castingOf(
  a: Pick<BattleAbilitySnap, "verb" | "trigger" | "amount" | "shape"> & {
    radius?: number;
    body?: unknown;
  },
): Casting | null {
  // Просят рукой `active` и `once`; остальные поводы случаются сами, и в веере
  // им места нет — обещать выбор там, где его нет, та же ложь, что показать
  // чару, которой не сыграть.
  if ((a.trigger !== "active" && a.trigger !== "once") || a.amount <= 0) return null;
  // Пригоршни считаются все восемь; запрещённые §4 сочетания не играются.
  if (forbiddenShape(a.verb, a.shape, a.radius ?? 0)) return null;
  // Призыв без тела не призыв, и движок его не играет.
  if (a.verb === "summon" && !a.body) return null;
  return CASTINGS[a.verb] ?? null;
}

/** Пригоршня, если она не «одно тело»: слово для подписи. Одно тело и «себе»
 *  не называются вовсе — это и так видно по тому, куда светит доска. */
export function reachWord(shape: string): TranslationKey | null {
  if (shape === "one" || shape === "self" || shape === "cell") return null;
  return SHAPE_LABELS[shape as AbilityShape] ?? null;
}

/** Слово и знак удержания. Один список на дом — по той же причине, по которой
 *  один список у чисел карты: оцепенение, названное на доске одним словом, а в
 *  журнале другим, читается как две разные вещи. */
export const HOLD_WORDS: Record<string, { word: TranslationKey; mark: string }> = {
  bound: { word: "battleHoldBound", mark: "control" },
  hushed: { word: "battleHoldHushed", mark: "silence" },
  disarmed: { word: "battleHoldDisarmed", mark: "disarm" },
  swayed: { word: "battleHoldSwayed", mark: "charm" },
  veiled: { word: "battleHoldVeiled", mark: "veil" },
  guarding: { word: "battleHoldGuarding", mark: "guard" },
  numb: { word: "battleHoldNumb", mark: "immune" },
  thorned: { word: "battleHoldThorned", mark: "thorns" },
  festering: { word: "battleHoldFestering", mark: "flame" },
  knitting: { word: "battleHoldKnitting", mark: "bloom" },
  rested: { word: "battleHoldRested", mark: "hourglass" },
};

/** Род удержания одним словом: в записи он то строка, то коробка с каналом
 *  внутри (`{numb: "magic"}`), и разворачивать это в каждом месте, где его
 *  показывают, значит забыть однажды. */
export function holdKind(kind: BattleHoldKind): string {
  return typeof kind === "string" ? kind : "numb";
}

/** Показатель всадника: его знак и его слово. Список ОДИН, как у чисел карты
 *  (`STAT_MARKS`), и по той же причине — уязвимость, названная порознь на
 *  карте и в веере, однажды будет названа по-разному. */
export const RIDER_STATS: Record<RiderStat, { mark: string; label: TranslationKey }> = {
  power: { mark: STAT_MARKS.power, label: STAT_LABELS.power },
  armor: { mark: STAT_MARKS.armor, label: STAT_LABELS.armor },
  ward: { mark: STAT_MARKS.ward, label: STAT_LABELS.ward },
  // Своего числа у уязвимости на карте нет — она живёт только всадником.
  vulnerable: { mark: "curse", label: "battleStatVulnerable" },
};

/** Слово всадника: своё имя той чары, которая его навела.
 *
 * Всадник носит КЛЮЧ умения, а не название: журнал переживает и сессию, и
 * перевод, и русское слово, впечатанное в запись, отдало бы английскому
 * читателю русский всадник. Слово подставляет комната — по ключу, ровно так же,
 * как она подставляет название карты по её слагу. Не нашлось — пусто, и
 * называть всадника останется его показателю. */
export function riderWord(
  status: { name: string },
  card: BattleCard | null | undefined,
  lang: Lang,
): string {
  const list = card?.abilities ?? [];
  for (let i = 0; i < list.length; i++) {
    if (abilityKey(list[i], i) !== status.name) continue;
    const own = (lang === "ru" ? list[i].nameRu : list[i].nameEn)?.trim();
    return own || "";
  }
  return "";
}

/** Есть ли этим что делать прямо сейчас — по телам или по клеткам. */
export const intentLive = (i: Intent) => i.aims.size > 0 || i.spots.size > 0;

/** Один способ действовать — и всё, что о нём надо сказать. */
export interface Intent {
  /** `blow` · `mend` · `cast:<ключ умения>`. */
  key: string;
  kind: "blow" | "mend" | "cast";
  casting: Casting | null;
  /** Своё имя чары на языке читателя. Пусто — слово берётся по глаголу. */
  name: string;
  word: TranslationKey;
  mark: string;
  /** Число, которое несёт: сила удара, сколько лечит, сколько снимает чара. */
  amount: number | null;
  mana: number;
  range: number | null;
  /** Показатель и срок всадника — только у проклятия и благословения. */
  stat: RiderStat | null;
  turns: number | null;
  /** Слово пригоршни — у тех, кто берёт больше одного тела. */
  reach: TranslationKey | null;
  /** Она же словом с карты: `BattleIcon` рисует пригоршни по этим именам, и
   *  второй таблицы «форма → значок» заводить не надо. */
  shape: string;
  /** Наводится на КЛЕТКУ, а не на тело. Нужно ровно затем, чтобы отказ звучал
   *  верно: у одной чары «некого», у другой «некуда», и это разные слова. */
  atSpot: boolean;
  /** Ширина круга или число звеньев: то же число, что на умении. */
  radius: number;
  /** Кого этим можно взять: тело → ГОТОВОЕ действие из `legalActions`. */
  aims: Map<number, BattleAction>;
  /** Куда этим можно ткнуть, если оно наводится на КЛЕТКУ: `x,y` → действие.
   *  Отдельным ящиком, а не общим: у клетки нет номера тела, а у тела нет
   *  клетки, пока оно не встало, — один ящик пришлось бы спрашивать дважды. */
  spots: Map<string, BattleAction>;
  /** Через сколько ходов вернётся. Читается с тела, а не считается. */
  asleep: number | null;
  /** Потрачено НАВСЕГДА: умение, которое просят один раз за партию. Отдельным
   *  признаком, а не числом ходов: «вернётся через 255» — это не срок, это
   *  «никогда», сказанное числом, которому человек поверит. */
  spent: boolean;
  /** Цена выше того, что есть. */
  dear: boolean;
}

/**
 * Чем это тело может заняться — по одному намерению на способ.
 *
 * Способ попадает сюда и тогда, когда им сейчас НЕЛЬЗЯ: тело с двумя чарами, из
 * которых одна спит, обязано показать обе, иначе спящая чара отличается от
 * несуществующей только памятью хранителя. Но ходить по ним нечем — `aims`
 * пуст, — и ни одно такое намерение не может превратиться в действие.
 */
export function intentsOf(
  unit: BattleUnit,
  card: BattleCard | null | undefined,
  legal: BattleAction[],
  mana: number,
  lang: Lang,
): Intent[] {
  const blank = (over: Partial<Intent> & Pick<Intent, "key" | "kind" | "word" | "mark">): Intent => ({
    casting: null,
    name: "",
    amount: null,
    mana: 0,
    range: null,
    stat: null,
    turns: null,
    reach: null,
    shape: "one",
    atSpot: false,
    radius: 0,
    aims: new Map(),
    spots: new Map(),
    asleep: null,
    spent: false,
    dear: false,
    ...over,
  });

  const out: Intent[] = [];
  const blow = blank({
    key: "blow",
    kind: "blow",
    word: "battleIntentBlow",
    mark: STAT_MARKS.power,
    amount: unit.power,
    range: unit.reach,
  });
  const mend = blank({
    key: "mend",
    kind: "mend",
    word: "battleIntentMend",
    mark: STAT_MARKS.mend,
    amount: unit.mend > 0 ? unit.mend : null,
    range: unit.reach,
  });

  // Удар и лечение тела — не умения, и спрашиваются они у тела.
  if (unit.card.strikes !== false && unit.power > 0) out.push(blow);
  let mendShown = unit.mend > 0;
  if (mendShown) out.push(mend);

  const left = new Map((unit.abilityCds ?? []).map((c) => [c.id, c.left]));
  const abilities = unit.card.abilities ?? [];
  // Своё имя чары лежит на КАРТЕ, а не в снимке тела: движок названий не
  // носит, и носить не должен — журнал переживает и перевод. Подставляется
  // оно по тому же ключу, по которому комната называет всадника.
  const named = new Map<string, string>();
  (card?.abilities ?? []).forEach((a, i) => {
    const own = (lang === "ru" ? a.nameRu : a.nameEn)?.trim();
    if (own) named.set(abilityKey(a, i), own);
  });
  for (let i = 0; i < abilities.length; i++) {
    const a = abilities[i];
    const key = abilityKey(a, i);
    const cd = left.get(key) ?? 0;
    const dear = a.manaCost > mana;

    // Лечение чарой — то же намерение, что лечение телом: движок играет его
    // одним действием (`mend`), и двух «лечить» в веере быть не может.
    if (a.verb === "heal" && (a.trigger === "active" || a.trigger === "once") && a.amount > 0) {
      if (a.shape !== "one" && a.shape !== "self") continue;
      if (!mendShown) {
        out.push(mend);
        mendShown = true;
      }
      mend.amount = mend.amount === null ? a.amount : Math.max(mend.amount, a.amount);
      mend.range = Math.max(mend.range ?? 0, a.range);
      mend.mana = mend.mana || a.manaCost;
      // Спит у лечения то из двух, что названо первым и ещё не проснулось.
      if (cd > 0 && mend.asleep === null) mend.asleep = cd;
      if (dear) mend.dear = true;
      continue;
    }

    const casting = castingOf(a);
    if (!casting) continue;
    out.push(
      blank({
        key: `cast:${key}`,
        kind: "cast",
        casting,
        name: named.get(key) ?? "",
        word: VERB_LABELS[a.verb as AbilityVerb] ?? "battlesVerbDamage",
        mark: VERB_ICON[a.verb as AbilityVerb] ?? "sword",
        amount: a.amount,
        mana: a.manaCost,
        range: a.range,
        stat: casting === "curse" || casting === "bless" ? (a.stat || "power") : null,
        turns: casting === "curse" || casting === "bless" ? Math.max(1, a.duration ?? 0) : null,
        reach: reachWord(a.shape),
        shape: a.shape,
        atSpot:
          casting === "zone" ||
          casting === "summon" ||
          (casting === "shove" && a.shape === "self"),
        radius: a.radius ?? 0,
        asleep: cd > 0 && cd < FOREVER ? cd : null,
        spent: cd >= FOREVER,
        dear,
      }),
    );
  }

  // Цели — из законного списка, как есть: каждая несёт ГОТОВОЕ действие, и
  // назад оно уходит неизменным.
  const by = new Map(out.map((i) => [i.key, i]));

  // Законное, которого тело за собой не знает, всё равно показывается, и это
  // единственная защита от того, что зеркало (`castingOf`) однажды разойдётся
  // с движком: список действий — истина, а перечень умений на карте только её
  // описание. Без этого карта, которой движок умеет больше, чем здесь
  // перечислено, молча теряла бы ход — тихо и навсегда.
  for (const action of legal) {
    if (typeof action === "string") continue;
    if ("attack" in action && action.attack.attacker === unit.id && !by.has("blow")) {
      out.unshift(blow);
      by.set("blow", blow);
    }
    if ("mend" in action && action.mend.healer === unit.id && !by.has("mend")) {
      out.push(mend);
      by.set("mend", mend);
    }
    if ("cast" in action && action.cast.caster === unit.id) {
      const key = `cast:${action.cast.ability}`;
      if (!by.has(key)) {
        const at = abilities.findIndex((a, i) => abilityKey(a, i) === action.cast.ability);
        const a = at >= 0 ? abilities[at] : null;
        const made = blank({
          key,
          kind: "cast",
          casting: a ? castingOf(a) : null,
          name: named.get(action.cast.ability) ?? "",
          word: (a && VERB_LABELS[a.verb as AbilityVerb]) || "battlesVerbDamage",
          mark: (a && VERB_ICON[a.verb as AbilityVerb]) || "sword",
          amount: a?.amount ?? null,
          mana: a?.manaCost ?? 0,
          range: a?.range ?? null,
        });
        out.push(made);
        by.set(key, made);
      }
    }
    // Цель у догнанного намерения та же самая: действие уже в руках.
    if ("attack" in action && action.attack.attacker === unit.id) {
      by.get("blow")?.aims.set(action.attack.target, action);
    } else if ("mend" in action && action.mend.healer === unit.id) {
      by.get("mend")?.aims.set(action.mend.target, action);
    } else if ("cast" in action && action.cast.caster === unit.id) {
      const intent = by.get(`cast:${action.cast.ability}`);
      const aim = action.cast.target;
      if ("unit" in aim) intent?.aims.set(aim.unit, action);
      else intent?.spots.set(`${aim.spot.x},${aim.spot.y}`, action);
    }
  }

  return out;
}

/**
 * Что берётся в руку само.
 *
 * Тем, чем есть что сделать, — а если таких несколько, то, чем бьют: это и был
 * прежний жест, и тело, которое умеет одно лишнее, не должно начинать ход
 * иначе, чем все остальные.
 */
export function firstToHand(intents: Intent[]): Intent | null {
  const live = intents.filter((i) => i.aims.size > 0 || i.spots.size > 0);
  if (!live.length) return null;
  return live.find((i) => i.kind === "blow") ?? live[0];
}

/**
 * Знак СТРОКИ ОПИСИ, или ничего.
 *
 * Опись знает и слова (имя, раса, байка), и числа, и знак есть только у
 * вторых. Спрашивать это здесь, а не перечислять числа в списке стола, —
 * потому что список слотов один (`SHEET_SLOTS`), и второй, живущий в админке
 * и помнящий, какие из них числа, разошёлся бы с ним на первой же новой
 * строке. `stats` — коробка паспорта, а не число: одного знака на семь чисел
 * не бывает.
 */
export function sheetSlotMark(slot: SheetSlot): string | null {
  if (slot === "healthMark") return statMark("health");
  return slot in STAT_MARKS ? STAT_MARKS[slot as MarkedStat] : null;
}

/** Знак числа. Отдельной функцией, а не чтением словаря на месте, потому что
 *  `healthMark` — это здоровье под другим именем, и разворачивать это в каждом
 *  из шести мест значит забыть однажды. */
export function statMark(slot: MarkedStat | "healthMark"): string {
  return STAT_MARKS[slot === "healthMark" ? "health" : slot];
}

/**
 * The numbers a person needs to see the body they will play. Zeros stay off
 * the paper, the same way `pricesOf` will not print a coin that is not a price.
 */
export function bodyPassport(
  card: Pick<BattleCard, BodyStatField>,
): { field: BodyStatField; value: number }[] {
  const rows: { field: BodyStatField; value: number }[] = [
    { field: "health", value: card.health },
    { field: "mana", value: card.mana },
    { field: "armor", value: card.armor },
    { field: "ward", value: card.ward },
    { field: "reach", value: card.reach },
    { field: "step", value: card.step },
    { field: "mend", value: card.mend },
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
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
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
    case "works_seen":
    case "works_liked":
    case "comments_left":
    case "bookings_done":
    case "orders_made":
      return "/figurines";
    case "tales_read":
      return "/tales";
    case "deck_laid":
      return "/battles/table";
    case "matches_finished":
    case "matches_won":
    case "challenges_won":
      return "/battles/etude";
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
  "blow",
  "spell",
  "mend",
  "arrive",
  "fall",
  "unseen",
];
export const GESTURE_WHOMS: GestureWhom[] = [
  "striker",
  "target",
  "flight",
  "beam",
  "field",
];
export const GESTURE_BODIES: GestureBody[] = [
  "none",
  "lunge",
  "flinch",
  "shiver",
  "sink",
  "rise",
  "swell",
  "bow",
  "draw",
  "recoil",
  "heave",
  "shudder",
  "sway",
  "loom",
  "kindle",
  "blanch",
  "wither",
];

/** Жесты света. Меняют `filter`, а не `transform`, — значит, их можно дать
 *  телу ВМЕСТЕ с движением, и они сложатся. Список нужен столу: он подсказывает
 *  хранителю, какой жест не отменит уже надетый. */
export const GESTURE_LIGHTS: GestureBody[] = ["kindle", "blanch", "wither"];
/** Замахи. Пишут `transform`, поэтому на одном теле живёт только один. */
export const GESTURE_MOVES: GestureBody[] = GESTURE_BODIES.filter(
  (b) => b !== "none" && !GESTURE_LIGHTS.includes(b),
);

export const isLight = (body: GestureBody) => GESTURE_LIGHTS.includes(body);
export const isMove = (body: GestureBody) =>
  body !== "none" && !GESTURE_LIGHTS.includes(body);

/** Полёт, луч и поле — рисунок без тела: двигать там некого. */
export const isBodiless = (whom: GestureWhom) =>
  whom === "flight" || whom === "beam" || whom === "field";

/** Полёт, луч и поле без картинки — слот под стрелу, не пустой жест. */
export const isSlot = (g: MotionGesture) => isBodiless(g.whom) && !g.image;

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
  const sm = last("striker", isMove);
  const tm = last("target", isMove);
  const sl = last("striker", isLight);
  const tl = last("target", isLight);
  return gestures.filter((g, i) => {
    if (g.whom !== "striker" && g.whom !== "target") return true;
    if (isMove(g.body)) return i === (g.whom === "striker" ? sm : tm);
    if (isLight(g.body)) return i === (g.whom === "striker" ? sl : tl);
    return true;
  });
}
export const GESTURE_TURNS: GestureTurn[] = ["none", "toTarget", "mirror"];
export const GESTURE_FADES: GestureFade[] = ["hold", "in", "out", "inOut"];

/** Потолок длительности. Тот же, что на сервере: ход хранителя из трёх
 *  действий обязан укладываться в две-три секунды, а этюд переигрывают. */
export const MOTION_MS_MAX = 1200;
export const MOTION_FRAMES_MAX = 24;
export const GESTURES_MAX = 12;
export const MOTIONS_MAX = 48;
/** Сколько нарядов держит ящик. Зеркало `PRESETS_MAX` в `battles.rs`. */
export const PRESETS_MAX = 64;
export const GESTURE_SIZE_MAX = 300;
export const GESTURE_NUDGE_MAX = 200;
export const GESTURE_LAYERS = 12;

/** Отношение сторон клетки на доске: 3 в ширину, 4 в высоту
 *  (`BATTLE-SCENE.md` §10.1). Нужно ровно затем, чтобы стрела летела под тем
 *  углом, под каким её видит глаз, а не под тем, какой у клеток в координатах
 *  движка: угол в клетках и угол на экране — разные числа. */
const CELL_TALL = 4 / 3;

export function newGesture(whom: GestureWhom = "striker"): MotionGesture {
  return {
    whom,
    body: isBodiless(whom) ? "none" : "lunge",
    image: "",
    frames: 1,
    size: 60,
    nudgeX: 0,
    nudgeY: 0,
    at: 0,
    dur: 300,
    turn: whom === "flight" ? "toTarget" : "none",
    fade: whom === "flight" ? "hold" : "inOut",
    layer: 5,
    strip: [],
  };
}

export function newMotion(occasion: MotionOccasion = "blow"): Motion {
  return {
    id:
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `m${Date.now().toString(36)}${Math.random().toString(36).slice(2, 8)}`,
    nameEn: "",
    nameRu: "",
    occasion,
    span: 0,
    gestures: [newGesture("striker")],
  };
}

function gesture(
  whom: GestureWhom,
  body: GestureBody,
  at: number,
  dur: number,
): MotionGesture {
  return { ...newGesture(whom), body, at, dur, fade: "hold" };
}

/** Слот под картинку. Без неё ничего не рисуется, но место живёт в записи. */
export function newSlot(
  whom: "flight" | "beam" | "field",
  at = 80,
  dur = 320,
): MotionGesture {
  return {
    ...newGesture(whom),
    at,
    dur,
    image: "",
    body: "none",
    size: whom === "flight" ? 45 : whom === "beam" ? 60 : 80,
    fade: "inOut",
  };
}

/**
 * Промах: удар не взял. Не тот же flinch, что у раны — иначе оберег дрожит
 * как раненый. Свет без движения: тело стоит, краска уходит.
 */
export const WARD_MOTION: Motion = {
  id: "house-ward",
  nameEn: "A ward",
  nameRu: "Оберег",
  occasion: "unseen",
  gestures: [gesture("target", "blanch", 0, 280)],
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
    id: "house-blow",
    nameEn: "A blow",
    nameRu: "Удар",
    occasion: "blow",
    gestures: [
      // Подача и возврат — ОДИН жест: 220 туда и 180 обратно живут в его
      // собственной кривой, а не в двух записях. Двумя записями хранитель
      // однажды сотрёт вторую и оставит тело поданным.
      gesture("striker", "lunge", 0, 400),
      gesture("target", "flinch", 220, 160),
    ],
  },
  {
    id: "house-mend",
    nameEn: "Mending",
    nameRu: "Лечение",
    occasion: "mend",
    gestures: [gesture("target", "rise", 0, 300)],
  },
  {
    id: "house-arrive",
    nameEn: "Taking the field",
    nameRu: "Выставление",
    occasion: "arrive",
    gestures: [gesture("striker", "swell", 0, 300)],
  },
  {
    id: "house-fall",
    nameEn: "Falling",
    nameRu: "Падение",
    occasion: "fall",
    gestures: [gesture("target", "sink", 0, 500)],
  },
  {
    id: "house-unseen",
    nameEn: "No author",
    nameRu: "Без автора",
    occasion: "unseen",
    gestures: [gesture("target", "shiver", 0, 160)],
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
  return Math.min(
    MOTION_MS_MAX,
    Math.max(motionBars(motion), motion.span || 0),
  );
}

export function motionTitle(motion: Motion, lang: Lang): string {
  const own = lang === "ru" ? motion.nameRu : motion.nameEn;
  return own || motion.nameRu || motion.nameEn || motion.id;
}

export function parseMotionWear(raw: string | null | undefined): MotionWear {
  if (!raw) return {};
  try {
    const found = JSON.parse(raw) as MotionWear;
    return found && typeof found === "object" ? found : {};
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
  if ("played" in event) return "arrive";
  if ("died" in event) return "fall";
  if ("healed" in event) return event.healed.by == null ? "unseen" : "mend";
  // Всадник и щит — чара по поводу, и иного повода у них нет: это ровно то, на
  // что хранитель надевает «наводит». Без автора — `unseen`, как у зоны и яда.
  if ("rider" in event) return event.rider.by == null ? "unseen" : "spell";
  if ("shielded" in event) return event.shielded.by == null ? "unseen" : "spell";
  if ("held" in event) return event.held.by == null ? "unseen" : "spell";
  if ("lifted" in event) return event.lifted.by == null ? "unseen" : "spell";
  // Опасная клетка — чара по поводу, даже когда гореть на ней ещё некому.
  if ("zoned" in event) return event.zoned.by == null ? "unseen" : "spell";
  if ("damaged" in event || "immune" in event) {
    const by = "damaged" in event ? event.damaged.by : event.immune.by;
    if (by == null) return "unseen";
    // `source` приходит в событии готовым словом — это не вывод правила, а
    // чтение того, что движок уже сказал.
    // Плеск — та же чара, только задевшая соседа: движение у них одно, и
    // разводить их значило бы просить у хранителя два наряда на один жест.
    const source = "damaged" in event ? event.damaged.source : "";
    return source === "ability" || source === "splash" ? "spell" : "blow";
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
  if (occasion === "spell") {
    const asBlow = found(own.blow) ?? found(kin.blow);
    if (asBlow) return asBlow;
  }

  const houseOccasion: MotionOccasion =
    occasion === "spell" ? "blow" : occasion;
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
  kind?: "scrap";
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
export type StruckKind = "bruise" | "ink";

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
  const src = hit.source ?? "attack";
  if (src === "dot" || src === "zone") return null;
  if (hit.channel === "physical") return "bruise";
  if (hit.channel === "magic" || hit.channel === "pure") return "ink";
  return null;
}

/** Когда движение касается цели: первый жест на ней, иначе сразу. */
export function motionContact(motion: Motion | null): number {
  const aimed = motion?.gestures.filter((g) => g.whom === "target") ?? [];
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
  const pictured = motion.gestures.filter(
    (g) => g.whom === "target" && g.image,
  );
  if (pictured.length) {
    const g = pictured.reduce((a, b) => ((a.at || 0) <= (b.at || 0) ? a : b));
    const frames = Math.max(1, g.frames || 1);
    const hit = frames > 1 ? 0.62 : 0.45;
    return Math.min(
      MOTION_MS_MAX,
      (g.at || 0) + Math.round((g.dur || 0) * hit),
    );
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

type PaperEdge = "top" | "right" | "bottom" | "left";

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
  const edges: PaperEdge[] = ["top", "right", "bottom", "left"];
  const bites: PaperBite[] = [];
  for (let i = 0; i < n; i++) {
    const edge = edges[Math.floor(rng() * 4)]!;
    const t = 0.18 + rng() * 0.64;
    const w = 0.07 + rng() * 0.05 + missing * 0.05;
    if (bites.some((b) => b.edge === edge && Math.abs(b.t - t) < 0.14))
      continue;
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
  const add = (x: number, y: number) =>
    pts.push(`${x.toFixed(2)}% ${y.toFixed(2)}%`);
  add(0, 0);
  for (const b of on("top")) {
    add((b.t - b.w) * 100, 0);
    add(b.t * 100, b.depth);
    add((b.t + b.w) * 100, 0);
  }
  add(100, 0);
  for (const b of on("right")) {
    add(100, (b.t - b.w) * 100);
    add(100 - b.depth, b.t * 100);
    add(100, (b.t + b.w) * 100);
  }
  add(100, 100);
  for (const b of [...on("bottom")].reverse()) {
    add((b.t + b.w) * 100, 100);
    add(b.t * 100, 100 - b.depth);
    add((b.t - b.w) * 100, 100);
  }
  add(0, 100);
  for (const b of [...on("left")].reverse()) {
    add(0, (b.t + b.w) * 100);
    add(b.depth, b.t * 100);
    add(0, (b.t - b.w) * 100);
  }
  return `polygon(${pts.join(",")})`;
}

/**
 * Сургуч на кружке здоровья: трещины и выщербы по тому, сколько его осталось.
 *
 * Заведено затем, что здоровье — единственное число карты, которое МЕНЯЕТСЯ, а
 * печаталось оно как отчеканенная навсегда монета: десять молча становилось
 * семью, и увидеть это можно было, только прочитав цифру. Сургуч читается
 * раньше цифры.
 *
 * Своих полей у здоровья нет и здесь: это правило ОТРИСОВЩИКА, а не ручка
 * хранителя. `BADGE_FIELDS.health` по-прежнему указывает на поля стоимости, и
 * дать сургучу настройку значило бы завести здоровью первое собственное поле —
 * то самое, из-за которого два значка однажды разошлись бы.
 *
 * Закон тот же, что у рваного края карты (`paperBites`): `seed` держит трещины
 * на месте, а число их растёт с уроном ПО ТОЙ ЖЕ последовательности, поэтому
 * лечение снимает те трещины, которые были, а не рисует новые. Совпадение это
 * не случайное — рвётся одна и та же бумага.
 *
 * Возвращает готовую строку стиля, а не куски: отрисовщик один, как у резьбы
 * (`carvedCopies`) и у движений (`stage`), и второй, собирающий то же самое из
 * частей, был бы предпросмотром, который однажды соврёт.
 */
export function sealWear(remain: number, seed: number): string | null {
  const missing = 1 - Math.max(0, Math.min(1, remain));
  if (missing < 0.02) return null;
  const rng = wearRng(seed + 101);
  const marks: string[] = [];
  // Трещин столько же, сколько выщербов у края карты, и по той же формуле:
  // кружок мельче карты, и четвёртая трещина на нём — уже не сургуч, а сетка.
  const cracks = Math.min(
    SEAL_CRACKS_MAX,
    1 + Math.floor(missing * SEAL_CRACKS_MAX),
  );
  for (let i = 0; i < cracks; i++) {
    // Каждая трещина ЦЕЛИКОМ вычерпывается из последовательности, включая
    // изломы, — иначе следующая забирала бы числа предыдущей и первая трещина
    // переезжала бы от одного удара к другому.
    const ang = rng() * Math.PI * 2;
    const jitters = [rng(), rng(), rng(), rng()];
    // Идёт от края внутрь: сургуч лопается от кромки, а не из середины.
    const reach = 0.34 + missing * 0.48;
    const pts: string[] = [];
    for (let k = 0; k <= 4; k++) {
      const t = k / 4;
      const r = SEAL_R * (1 - t * reach);
      const off = (jitters[k % 4]! - 0.5) * 0.3 * (1 - t);
      const a = ang + off;
      pts.push(
        `${(50 + Math.cos(a) * r).toFixed(1)},${(50 + Math.sin(a) * r).toFixed(1)}`,
      );
    }
    marks.push(
      `<polyline points="${pts.join(" ")}" fill="none" stroke="#2a1c14"` +
        // Толщина названа в СОТЫХ ДОЛЯХ кружка, а не в пикселях, и потому
        // держится на любой величине карты. Но и доля выбрана по самой мелкой:
        // на клетке боя кружок в двадцать пикселей, и волосок в два процента
        // его ширины там не рисуется вовсе — трещина, которой не видно ровно
        // там, где здоровье и меняется, не трещина.
        ` stroke-width="${(4.6 - i * 0.8).toFixed(1)}" stroke-linecap="round"` +
        ` stroke-opacity="${(0.5 + missing * 0.32).toFixed(2)}"/>`,
    );
    // Выщерб — только у сильно битого, и только у первых трещин: край
    // выкрошился там, где лопнуло раньше всего.
    if (missing > 0.45 && i < 2) {
      const w = 0.16 + rng() * 0.1;
      const p: string[] = [];
      for (const a of [ang - w, ang, ang + w]) {
        const r = a === ang ? SEAL_R * (1 - 0.16 - missing * 0.1) : SEAL_R;
        p.push(
          `${(50 + Math.cos(a) * r).toFixed(1)},${(50 + Math.sin(a) * r).toFixed(1)}`,
        );
      }
      marks.push(
        `<polygon points="${p.join(" ")}" fill="#2a1c14" fill-opacity="0.5"/>`,
      );
    }
  }
  // Остывший воск темнеет весь, а не только по трещинам. Печатается ПЕРВЫМ,
  // под трещинами: положенное поверх, оно размывало бы их собственный край.
  const dull =
    `<circle cx="50" cy="50" r="${SEAL_R}" fill="#2a1c14"` +
    ` fill-opacity="${(missing * 0.12).toFixed(3)}"/>`;
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" preserveAspectRatio="none">` +
    dull +
    marks.join("") +
    `</svg>`;
  // Кодируется целиком, а не одна решётка: в `data:`-ссылке `#` открывает
  // якорь и обрезает хвост картинки, а `<`/`>`/кавычка часть браузеров
  // принимает лишь по доброте.
  return `background-image:url("data:image/svg+xml,${encodeURIComponent(svg)}")`;
}

/** Трещин на кружке не больше трёх: четвёртая — уже не сургуч, а сетка. */
export const SEAL_CRACKS_MAX = 3;
/** Радиус сургучной печати в её собственных ста единицах. Не пятьдесят —
 *  трещина, дошедшая до самого края коробки, читается как царапина по бумаге
 *  вокруг значка, а не как лопнувший воск. */
const SEAL_R = 45;

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

const EMPTY_STAGE: Staged = { span: 0, striker: "", target: "", motes: [] };

/** Кривая подачи. Та же, что была написана в сцене до движка. */
const EASE = "cubic-bezier(0.2, 0.8, 0.25, 1)";

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
    if (!src.startsWith("blob:") && !src.startsWith("data:")) {
      img.crossOrigin = "anonymous";
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

function stripValueSat(
  r: number,
  g: number,
  b: number,
): { v: number; s: number } {
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
    const canvas = document.createElement("canvas");
    canvas.width = img.width;
    canvas.height = img.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas");
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
      ground[i] = isStripGround(data[o], data[o + 1], data[o + 2], data[o + 3])
        ? 1
        : 0;
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
        if (!isStripFringe(data[o], data[o + 1], data[o + 2], data[o + 3]))
          continue;
        let near = false;
        for (
          let yy = Math.max(0, y - fringeR);
          yy <= Math.min(h - 1, y + fringeR) && !near;
          yy++
        ) {
          for (
            let xx = Math.max(0, x - fringeR);
            xx <= Math.min(w - 1, x + fringeR);
            xx++
          ) {
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
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, "image/png"),
    );
    if (!blob) throw new Error("punch");
    const stem = file.name.replace(/\.[^.]+$/, "") || "strip";
    return new File([blob], `${stem}.png`, { type: "image/png" });
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
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, Math.round(sw));
    canvas.height = Math.max(1, Math.round(sh));
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("canvas");
    ctx.drawImage(img, i * sw, 0, sw, sh, 0, 0, canvas.width, canvas.height);
    out.push(canvas.toDataURL("image/png"));
  }
  return out;
}

/** Шесть картинок встык, без зазора, каждая со своей позой. */
export async function stitchMotionStrip(
  cells: StripCell[],
  count = STRIP_FRAMES,
): Promise<Blob> {
  const canvas = document.createElement("canvas");
  canvas.width = STRIP_SIDE * count;
  canvas.height = STRIP_SIDE;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("canvas");
  const imgs = await Promise.all(
    cells
      .slice(0, count)
      .map((c) => (c.src ? loadStripImage(c.src) : Promise.resolve(null))),
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
    canvas.toBlob(resolve, "image/png"),
  );
  if (!blob) throw new Error("strip");
  return blob;
}

function fadeName(fade: GestureFade): string {
  return fade === "hold" ? "" : `gotiga-fade-${fade}`;
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

  // Экранные координаты клеток. Разворот живёт здесь и только здесь. Вдоль
  // комнаты глубина идёт СПРАВА НАЛЕВО — своя половина слева, — и сцена
  // кладёт клетки в том же порядке (`spots` в `BattleScene`).
  const screen = (spot: StageSpot | null) =>
    spot
      ? { x: along ? spanX - 1 - spot.y : spot.x, y: along ? spot.x : spot.y }
      : null;
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

  const stir: Record<"striker" | "target", string[]> = {
    striker: [],
    target: [],
  };
  const motes: StagedMote[] = [];
  let key = 0;

  for (const g of motion.gestures) {
    const at = Math.max(0, g.at || 0);
    const dur = Math.max(0, g.dur || 0);

    if (
      g.body &&
      g.body !== "none" &&
      (g.whom === "striker" || g.whom === "target")
    ) {
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
    const spot = g.whom === "target" ? b : a;

    const parts: string[] = ["position:absolute"];
    const anims: string[] = [];

    if (g.whom === "beam") {
      // Луч — рисунок, растянутый от бьющего до цели: молния, нить, взгляд.
      // Начинается в середине бьющего, ширина — расстояние до середины цели
      // на ЭКРАНЕ (клетка 3:4, как у угла), поворот — вокруг левого края.
      // Высота — величина жеста в процентах клетки, как у всех.
      if (!a || !b) continue;
      const cx = ((a.x + 0.5) / spanX) * 100;
      const cy = ((a.y + 0.5) / spanY) * 100;
      const reach = Math.hypot(b.x - a.x, (b.y - a.y) * CELL_TALL);
      parts.push(
        `left:${cx.toFixed(3)}%`,
        `top:${(cy - h / 2).toFixed(3)}%`,
        `width:${((reach / spanX) * 100).toFixed(3)}%`,
        `height:${h.toFixed(3)}%`,
        "transform-origin:0 50%",
      );
    } else if (g.whom === "field") {
      parts.push("inset:0");
    } else if (!spot) {
      // Некому и не над кем: жест просто не выходит. Не ошибка — обычный урон
      // без автора.
      continue;
    } else {
      const cx = ((spot.x + 0.5) / spanX) * 100 + g.nudgeX / spanX;
      const cy = ((spot.y + 0.5) / spanY) * 100 + g.nudgeY / spanY;
      parts.push(
        `left:${(cx - w / 2).toFixed(3)}%`,
        `top:${(cy - h / 2).toFixed(3)}%`,
      );
      parts.push(`width:${w.toFixed(3)}%`, `height:${h.toFixed(3)}%`);
    }

    parts.push(`background-image:url("${cssUrl(g.image)}")`);
    parts.push("background-repeat:no-repeat");
    if (frames > 1) {
      parts.push(`background-size:${frames * 100}% 100%`);
      parts.push(`--strip-end:${stripEnd(frames).toFixed(4)}%`);
      anims.push(`gotiga-strip ${dur}ms steps(${frames}) ${lag(at)}ms both`);
    } else {
      parts.push("background-size:contain", "background-position:center");
    }

    const turn =
      g.whom === "beam" || g.turn === "toTarget"
        ? `${angle.toFixed(2)}deg`
        : g.turn === "mirror"
          ? "180deg"
          : "0deg";
    parts.push(`--turn:${turn}`);

    if (g.whom === "flight" && a && b) {
      // Перелёт задаётся в процентах СОБСТВЕННОЙ ширины рисунка: проценты в
      // `translate` меряются по самому элементу, а не по полю. Клетка по
      // экрану — это `100/size` его ширин, значит клетка пути — `10000/size`
      // процентов. Точно, а не приближённо.
      const own = size > 0 ? 10000 / size : 0;
      parts.push(`--mx:${((b.x - a.x) * own).toFixed(2)}%`);
      parts.push(`--my:${((b.y - a.y) * own).toFixed(2)}%`);
      anims.push(`gotiga-fly ${dur}ms ${EASE} ${lag(at)}ms both`);
    } else if (g.whom === "target" && frames === 1 && a && b) {
      // Одиночная картина на цели. Полоса уже несёт удар в кадрах; без полосы
      // рисунок иначе просто висит вторым портретом. Замах читается с той же
      // стороны, что подача, и свет на металле — тот же, что kindle: яркость
      // фотографии, не вспышка.
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const len = Math.max(1, Math.abs(dx) + Math.abs(dy));
      parts.push(`--lx:${((dx / len) * 32).toFixed(2)}%`);
      parts.push(`--ly:${((dy / len) * 32).toFixed(2)}%`);
      parts.push("transform-origin:50% 38%");
      anims.push(`gotiga-cleave ${dur}ms ${EASE} ${lag(at)}ms both`);
    }

    const fading = fadeName(g.fade);
    if (fading) anims.push(`${fading} ${dur}ms linear ${lag(at)}ms both`);
    // До своего мгновения рисунка нет. `both` заполняет и время до начала
    // нулевым кадром, и рисунок без проявления (`hold`, `out`) иначе стоял бы
    // на клетке с первой миллисекунды: вспышка на цели раньше, чем к ней
    // полетел шар. Ожидание — своя анимация длиной `at` без заливки: пока она
    // идёт, рисунок скрыт, кончилась — виден. Остановка времени сдвигает и её.
    if (at > 0) anims.push(`gotiga-wait ${at}ms linear ${lag(0)}ms`);

    const layer = Math.max(1, Math.min(GESTURE_LAYERS, g.layer || 1));
    parts.push(`z-index:${layer}`);
    // Поворот ставится ВСЕГДА, а не только когда анимации нет: полоса кадров
    // не трогает `transform`, и рисунок с ней терял бы свой угол. Перелёт свой
    // поворот несёт внутри собственных кадров и потому эту строку перебивает.
    parts.push("transform:rotate(var(--turn))");
    if (anims.length) parts.push(`animation:${anims.join(",")}`);
    if (frozen && anims.length) parts.push("animation-play-state:paused");

    motes.push({ key: `${motion.id}-${key++}`, layer, style: parts.join(";") });
  }

  const dress = (who: "striker" | "target") =>
    stir[who].length
      ? `--lx:${lx.toFixed(2)}%;--ly:${ly.toFixed(2)}%;animation:${stir[who].join(",")}` +
        (frozen ? ";animation-play-state:paused" : "")
      : "";

  return {
    span: motionSpan(motion),
    striker: dress("striker"),
    target: dress("target"),
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
  axe: "/battles/motion/axe.png",
  sword: "/battles/motion/sword.png",
  mace: "/battles/motion/mace.png",
} as const;

/** Молнии мага. Рисует `tools/spell_strips.py`: разряд — луч (`beam`),
 *  растянутый от середины мага до середины цели; на цели остаётся подпалина
 *  — отдельная полоса, потому что луч тянется вдоль, а подпалина обязана
 *  оставаться круглой. Цветов пять, рисунок один; чёрная — не цвет того же
 *  света, а его изнанка, и кадров у неё восемь, а не шесть. */
type LightningTone = "" | "red" | "violet" | "green" | "black";

const lightningArt = (tone: LightningTone) => {
  const suffix = tone ? `-${tone}` : "";
  return {
    beam: `/battles/motion/lightning${suffix}.png`,
    scorch: `/battles/motion/scorch${suffix}.png`,
  };
};

/** Светлая молния: маг собирается и затепливается, разряд идёт от него к
 *  цели, цель содрогается, на ней остаётся подпалина. */
function brightLightning(
  tone: LightningTone,
  nameEn: string,
  nameRu: string,
): (typeof STOCK_MOTIONS)[number] {
  const art = lightningArt(tone);
  return {
    nameEn,
    nameRu,
    occasion: "blow",
    gestures: [
      gesture("striker", "sway", 0, 460),
      gesture("striker", "kindle", 0, 600),
      {
        ...newGesture("beam"),
        image: art.beam,
        frames: STRIP_FRAMES,
        size: 70,
        at: 160,
        dur: 420,
        fade: "hold",
        layer: 9,
      },
      { ...strikeArt(art.scorch, 230, 520), size: 90, fade: "out" },
      gesture("target", "kindle", 230, 360),
      gesture("target", "shudder", 230, 300),
    ],
  };
}

/** Остальная магия. Рисует `tools/magic_strips.py`; полосы клеток там
 *  нарисованы в настоящих пропорциях клетки 3:4 и сохранены сжатыми, поэтому
 *  круглое ложится на карту круглым. */
const MAGIC_ART = {
  fireball: "/battles/motion/fireball.png",
  fireburst: "/battles/motion/fireburst.png",
  ravens: "/battles/motion/ravens.png",
  ravensStrike: "/battles/motion/ravens-strike.png",
  iceLances: "/battles/motion/ice-lances.png",
  frost: "/battles/motion/frost.png",
  soulVortex: "/battles/motion/soul-vortex.png",
  soulStream: "/battles/motion/soul-stream.png",
  soulGlow: "/battles/motion/soul-glow.png",
  poisonGlob: "/battles/motion/poison-glob.png",
  poisonCloud: "/battles/motion/poison-cloud.png",
  ghostArm: "/battles/motion/ghost-arm.png",
  ghostHand: "/battles/motion/ghost-hand.png",
} as const;

/** Тяжёлые удары оружием. Рисует `tools/melee_strips.py`: одна полоса на
 *  цели — замах (три кадра), удар, и что от него осталось (четыре). Меч,
 *  секира и булава не поворачиваются к цели: у разреза нет верха и низа, а
 *  секира, перевёрнутая вверх ногами, рубила бы снизу. Кулак поворачивается. */
const MELEE_ART = {
  sword: "/battles/motion/sword-cut.png",
  axe: "/battles/motion/axe-chop.png",
  fist: "/battles/motion/fist-blow.png",
  mace: "/battles/motion/mace-crush.png",
} as const;

/** Магия посильнее и моменты боя, которые не удары: лечение, выход карты,
 *  гибель, наложенный щит. Рисует `tools/arcana_strips.py`. Гибель лежит в
 *  коробке ровно 1.3 клетки: прожжённое там высчитано по карте внутри неё. */
const ARCANA_ART = {
  meteor: "/battles/motion/meteor.png",
  chains: "/battles/motion/chains.png",
  reaper: "/battles/motion/reaper.png",
  shadowSpikes: "/battles/motion/shadow-spikes.png",
  bats: "/battles/motion/bats.png",
  batsSwarm: "/battles/motion/bats-swarm.png",
  healing: "/battles/motion/healing.png",
  summoning: "/battles/motion/summoning.png",
  death: "/battles/motion/death.png",
  ward: "/battles/motion/ward.png",
} as const;
const DEATH_SIZE = 130;

/** Рисунок, который летит от бьющего к цели. */
const flying = (
  image: string,
  frames: number,
  size: number,
  at: number,
  dur: number,
  extra: Partial<MotionGesture> = {},
): MotionGesture => ({
  ...newGesture("flight"),
  image,
  frames,
  size,
  at,
  dur,
  fade: "hold",
  layer: 9,
  ...extra,
});

/** Рисунок на цели (или на бьющем) — полоса кадров на его клетке. */
const lying = (
  whom: "striker" | "target",
  image: string,
  frames: number,
  size: number,
  at: number,
  dur: number,
  extra: Partial<MotionGesture> = {},
): MotionGesture => ({
  ...newGesture(whom),
  body: "none",
  image,
  frames,
  size,
  at,
  dur,
  fade: "hold",
  layer: 8,
  ...extra,
});

/** Рисунок, растянутый от бьющего до цели. */
const reaching = (
  image: string,
  frames: number,
  size: number,
  at: number,
  dur: number,
): MotionGesture => ({
  ...newGesture("beam"),
  image,
  frames,
  size,
  at,
  dur,
  fade: "hold",
  layer: 9,
});

/** Чёрная молния: маг не затепливается, а темнеет и вырастает; разряд
 *  длиннее (восемь кадров) и толще, цель не вспыхивает, а меркнет. */
function blackLightning(): (typeof STOCK_MOTIONS)[number] {
  const art = lightningArt("black");
  return {
    nameEn: "Black lightning",
    nameRu: "Чёрная молния",
    occasion: "blow",
    gestures: [
      gesture("striker", "loom", 0, 520),
      gesture("striker", "wither", 0, 760),
      {
        ...newGesture("beam"),
        image: art.beam,
        frames: 8,
        size: 80,
        at: 200,
        dur: 600,
        fade: "hold",
        layer: 9,
      },
      { ...strikeArt(art.scorch, 300, 620), size: 100, fade: "out" },
      gesture("target", "wither", 300, 520),
      gesture("target", "shudder", 300, 360),
    ],
  };
}

function strikeArt(image: string, at: number, dur: number): MotionGesture {
  return {
    ...newGesture("target"),
    body: "none",
    image,
    frames: STRIP_FRAMES,
    size: 118,
    at,
    dur,
    fade: "inOut",
    layer: 8,
  };
}

export const STOCK_MOTIONS: {
  nameEn: string;
  nameRu: string;
  occasion: MotionOccasion;
  gestures: MotionGesture[];
}[] = [
  {
    nameEn: "An axe",
    nameRu: "Секира",
    occasion: "blow",
    gestures: [
      gesture("striker", "heave", 0, 600),
      strikeArt(STRIKE_STRIPS.axe, 180, 480),
      gesture("target", "recoil", 420, 280),
    ],
  },
  {
    nameEn: "A sword",
    nameRu: "Меч",
    occasion: "blow",
    gestures: [
      gesture("striker", "lunge", 0, 500),
      strikeArt(STRIKE_STRIPS.sword, 140, 440),
      gesture("target", "flinch", 340, 200),
    ],
  },
  {
    nameEn: "A mace",
    nameRu: "Булава",
    occasion: "blow",
    gestures: [
      gesture("striker", "heave", 0, 620),
      strikeArt(STRIKE_STRIPS.mace, 200, 500),
      gesture("target", "shudder", 440, 280),
    ],
  },
  {
    // Клинок проходит по диагонали, за ним серп света; разрез горит,
    // брызжет искрами и темнеет.
    nameEn: "A sword slash",
    nameRu: "Рассечение мечом",
    occasion: "blow",
    gestures: [
      gesture("striker", "lunge", 0, 500),
      lying("target", MELEE_ART.sword, 8, 135, 120, 640),
      gesture("target", "recoil", 360, 260),
    ],
  },
  {
    // Секира падает сверху и врубается; расщелина, трещины, щепки, пыль.
    nameEn: "An axe chop",
    nameRu: "Удар секирой",
    occasion: "blow",
    gestures: [
      gesture("striker", "heave", 0, 620),
      lying("target", MELEE_ART.axe, 8, 140, 160, 680),
      gesture("target", "shudder", 415, 320),
    ],
  },
  {
    // Латная перчатка в профиль, костяшки раскалены; линии удара, две
    // ударные волны, кратер с отпечатком четырёх костяшек. Единственный из
    // ударов оружием, что поворачивается к цели: кулак в профиль обязан
    // прийти оттуда, где стоит бьющий.
    nameEn: "A fist blow",
    nameRu: "Удар кулаком",
    occasion: "blow",
    gestures: [
      gesture("striker", "lunge", 0, 480),
      lying("target", MELEE_ART.fist, 8, 150, 120, 620, { turn: "toTarget" }),
      gesture("target", "recoil", 350, 300),
    ],
  },
  {
    // Булава по дуге; кратер, искры металла, обломки, пыль.
    nameEn: "A mace crush",
    nameRu: "Удар булавой",
    occasion: "blow",
    gestures: [
      gesture("striker", "heave", 0, 640),
      lying("target", MELEE_ART.mace, 8, 140, 180, 680),
      gesture("target", "shudder", 435, 340),
    ],
  },
  {
    nameEn: "A heavy blow",
    nameRu: "Тяжёлый удар",
    occasion: "blow",
    gestures: [
      gesture("striker", "heave", 0, 520),
      // Один замах на цель: recoil и shudder оба пишут transform, и второй
      // убивал первый. Отдача — то, чем тяжёлый удар читается.
      gesture("target", "recoil", 320, 280),
    ],
  },
  {
    nameEn: "A shot",
    nameRu: "Выстрел",
    occasion: "blow",
    gestures: [
      gesture("striker", "draw", 0, 420),
      // Слот: без картинки ничего не летит, но место уже есть — кладут стрелу,
      // а не заводят жест. След удара в комнате рисуется отдельно.
      newSlot("flight", 80, 340),
      gesture("target", "flinch", 400, 160),
    ],
  },
  // Молнии. Повод `blow` — так маг бьёт обычным ударом; на чару их
  // надевают тем же ящиком.
  brightLightning("", "Lightning", "Молния"),
  brightLightning("red", "Red lightning", "Красная молния"),
  brightLightning("violet", "Violet lightning", "Фиолетовая молния"),
  brightLightning("green", "Green lightning", "Зелёная молния"),
  blackLightning(),
  {
    // Шар летит с пылающим хвостом; на цели вспыхивает пламя, карта
    // обугливается и тлеет.
    nameEn: "A fireball",
    nameRu: "Огненный шар",
    occasion: "blow",
    gestures: [
      gesture("striker", "draw", 0, 380),
      gesture("striker", "kindle", 0, 520),
      // Гаснет в миг удара: с `hold` шар оставался висеть на цели поверх
      // взрыва (полёт лежит слоем выше) до конца движения.
      flying(MAGIC_ART.fireball, 6, 95, 150, 340, { fade: "inOut" }),
      lying("target", MAGIC_ART.fireburst, 8, 130, 470, 640),
      gesture("target", "kindle", 480, 400),
      gesture("target", "recoil", 480, 300),
    ],
  },
  {
    // Вороны летят стаей, не поворачиваясь: нарисованы со спины, и стая,
    // развёрнутая к цели слева, летела бы вверх ногами. Налетают, бьют,
    // разлетаются; падают перья.
    nameEn: "A flock of ravens",
    nameRu: "Стая воронов",
    occasion: "blow",
    gestures: [
      gesture("striker", "rise", 0, 420),
      gesture("striker", "wither", 0, 500),
      flying(MAGIC_ART.ravens, 6, 85, 120, 420, { turn: "none", fade: "inOut" }),
      lying("target", MAGIC_ART.ravensStrike, 8, 140, 480, 700),
      gesture("target", "blanch", 540, 500),
      gesture("target", "shiver", 540, 300),
    ],
  },
  {
    // Залп из трёх сосулек; по цели расползается иней, трескается и
    // осыпается.
    nameEn: "Ice lances",
    nameRu: "Ледяные копья",
    occasion: "blow",
    gestures: [
      gesture("striker", "draw", 0, 400),
      gesture("striker", "blanch", 0, 480),
      flying(MAGIC_ART.iceLances, 6, 75, 140, 300, { fade: "inOut" }),
      lying("target", MAGIC_ART.frost, 8, 130, 420, 720),
      gesture("target", "blanch", 430, 600),
      gesture("target", "flinch", 430, 220),
    ],
  },
  {
    // Над целью раскрывается воронка, из карты к магу тянется струя, цель
    // меркнет, маг разгорается.
    nameEn: "Soul drain",
    nameRu: "Похищение души",
    occasion: "blow",
    gestures: [
      lying("target", MAGIC_ART.soulVortex, 8, 125, 0, 900, { layer: 7 }),
      gesture("target", "wither", 120, 760),
      gesture("target", "shiver", 160, 320),
      reaching(MAGIC_ART.soulStream, 8, 55, 260, 640),
      lying("striker", MAGIC_ART.soulGlow, 6, 120, 700, 460),
      gesture("striker", "kindle", 720, 440),
      gesture("striker", "rise", 720, 420),
    ],
  },
  {
    // Капля яда летит и лопается над целью; облако расползается, капает,
    // рассеивается медленно.
    nameEn: "Poison mist",
    nameRu: "Ядовитый туман",
    occasion: "blow",
    gestures: [
      gesture("striker", "sway", 0, 440),
      flying(MAGIC_ART.poisonGlob, 6, 48, 120, 340, { fade: "inOut" }),
      lying("target", MAGIC_ART.poisonCloud, 8, 155, 440, 760),
      gesture("target", "wither", 470, 640),
      gesture("target", "shiver", 480, 280),
    ],
  },
  {
    // От мага тянется призрачная рука и сжимает цель; по карте идут
    // трещины, рука рассыпается дымом. Кисть повёрнута к цели вместе с рукой.
    nameEn: "A ghostly hand",
    nameRu: "Призрачная рука",
    occasion: "blow",
    gestures: [
      gesture("striker", "loom", 0, 500),
      gesture("striker", "blanch", 0, 800),
      reaching(MAGIC_ART.ghostArm, 8, 45, 100, 820),
      lying("target", MAGIC_ART.ghostHand, 8, 125, 140, 820, { turn: "toTarget", layer: 10 }),
      gesture("target", "blanch", 480, 460),
      gesture("target", "shudder", 520, 360),
    ],
  },
  {
    // Маг поднимает руки — сверху падает пылающий камень; взрыв, оплавленный
    // кратер, обломки, столб дыма.
    nameEn: "A meteor",
    nameRu: "Метеор",
    occasion: "blow",
    gestures: [
      gesture("striker", "rise", 0, 420),
      gesture("striker", "kindle", 0, 520),
      lying("target", ARCANA_ART.meteor, 8, 165, 120, 720),
      gesture("target", "kindle", 390, 400),
      gesture("target", "shudder", 390, 340),
    ],
  },
  {
    // Из разлома на карте вырываются цепи, сковывают её крест-накрест,
    // затягиваются и уходят обратно. Чара: к оцепенению, немоте, разоружению.
    nameEn: "Chains of the abyss",
    nameRu: "Цепи из бездны",
    occasion: "spell",
    gestures: [
      gesture("striker", "loom", 0, 500),
      gesture("striker", "wither", 0, 600),
      lying("target", ARCANA_ART.chains, 8, 140, 100, 800),
      gesture("target", "shudder", 480, 360),
    ],
  },
  {
    // Призрачная коса проходит серпом холодного света; душа выдёргивается
    // из карты и уходит обратно.
    nameEn: "The reaper's scythe",
    nameRu: "Коса жнеца",
    occasion: "blow",
    gestures: [
      gesture("striker", "sway", 0, 460),
      gesture("striker", "blanch", 0, 600),
      lying("target", ARCANA_ART.reaper, 8, 150, 120, 760),
      gesture("target", "blanch", 380, 520),
      gesture("target", "shiver", 380, 260),
    ],
  },
  {
    // Из лужи тени под картой вырастают обсидиановые шипы и рассыпаются.
    nameEn: "Shadow spikes",
    nameRu: "Теневые шипы",
    occasion: "blow",
    gestures: [
      gesture("striker", "loom", 0, 420),
      gesture("striker", "wither", 0, 520),
      lying("target", ARCANA_ART.shadowSpikes, 8, 145, 100, 720),
      gesture("target", "shudder", 290, 340),
    ],
  },
  {
    // Рой летучих мышей: летит стаей, кружит над целью, кусает, рассеивается.
    nameEn: "A swarm of bats",
    nameRu: "Рой летучих мышей",
    occasion: "blow",
    gestures: [
      gesture("striker", "rise", 0, 400),
      gesture("striker", "wither", 0, 500),
      flying(ARCANA_ART.bats, 6, 95, 120, 380, { turn: "none", fade: "inOut" }),
      lying("target", ARCANA_ART.batsSwarm, 8, 150, 440, 700),
      gesture("target", "blanch", 520, 500),
      gesture("target", "shiver", 520, 300),
    ],
  },
  {
    // Столб золотого света, руны и перья, трещины затягиваются золотом.
    nameEn: "Healing light",
    nameRu: "Исцеление",
    occasion: "mend",
    gestures: [
      gesture("striker", "bow", 0, 400),
      gesture("striker", "kindle", 0, 520),
      lying("target", ARCANA_ART.healing, 8, 140, 80, 900),
      gesture("target", "kindle", 320, 600),
      gesture("target", "rise", 320, 400),
    ],
  },
  {
    // Огненный круг призыва рисует себя, встаёт столп пламени — и в нём
    // появляется карта. Повод `arrive` ложится на саму выходящую карту.
    nameEn: "Rising from the flame",
    nameRu: "Явление из пламени",
    occasion: "arrive",
    gestures: [
      lying("striker", ARCANA_ART.summoning, 8, 145, 0, 900),
      gesture("striker", "swell", 300, 320),
    ],
  },
  {
    // Карта горит с краёв, обугленное рассыпается пеплом, вверх уходит душа.
    // Саму карту гасит `sink` — полоса кладёт ожог поверх.
    nameEn: "Burning away",
    nameRu: "Гибель в огне",
    occasion: "fall",
    gestures: [
      lying("target", ARCANA_ART.death, 8, DEATH_SIZE, 0, 1000),
      gesture("target", "sink", 150, 850),
    ],
  },
  {
    // Рунический щит распускается перед картой, удар расходится по нему
    // рябью, щит ложится на карту. Чара — так играется наложенный щит.
    nameEn: "A rune shield",
    nameRu: "Рунический щит",
    occasion: "spell",
    gestures: [
      gesture("striker", "kindle", 0, 500),
      lying("target", ARCANA_ART.ward, 8, 150, 60, 900),
      gesture("target", "kindle", 300, 500),
    ],
  },
  {
    nameEn: "A charm",
    nameRu: "Чара",
    occasion: "spell",
    gestures: [
      gesture("striker", "sway", 0, 460),
      gesture("striker", "kindle", 0, 460),
      newSlot("field", 200, 400),
      gesture("target", "kindle", 380, 320),
      gesture("target", "shiver", 380, 200),
    ],
  },
  {
    nameEn: "A curse",
    nameRu: "Проклятие",
    occasion: "spell",
    gestures: [
      gesture("striker", "loom", 0, 420),
      newSlot("field", 180, 400),
      gesture("target", "wither", 340, 340),
      gesture("target", "shudder", 340, 280),
    ],
  },
  {
    nameEn: "The evil eye",
    nameRu: "Сглаз",
    occasion: "blow",
    gestures: [
      gesture("striker", "sway", 0, 380),
      gesture("target", "blanch", 260, 380),
    ],
  },
  {
    nameEn: "Tending",
    nameRu: "Врачевание",
    occasion: "mend",
    gestures: [
      gesture("striker", "bow", 0, 380),
      gesture("target", "kindle", 200, 340),
      gesture("target", "rise", 200, 340),
    ],
  },
  {
    nameEn: "Stepping out",
    nameRu: "Явление",
    occasion: "arrive",
    gestures: [
      gesture("striker", "swell", 0, 340),
      gesture("striker", "kindle", 60, 320),
    ],
  },
  {
    nameEn: "Guttering out",
    nameRu: "Угасание",
    occasion: "fall",
    gestures: [
      gesture("target", "sink", 0, 560),
      gesture("target", "blanch", 0, 560),
    ],
  },
  {
    nameEn: "Poison",
    nameRu: "Яд",
    occasion: "unseen",
    gestures: [
      gesture("target", "wither", 0, 320),
      gesture("target", "shiver", 0, 200),
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
  breakthrough: false,
};

/** Одно отличие правил от домашних: чем сказать и какое при нём число. */
export interface RuleApart {
  key: TranslationKey;
  /** Число, которое ставится рядом со словами. `null` — правило без числа. */
  amount: number | null;
}

/**
 * Что держит игрока в ЭТОМ бою — не отличия, а всё, обо что можно удариться.
 *
 * Не противоречит соседке и не отменяет её правила: `rulesApart` печатается
 * на полке НЕПРОШЕННОЙ, и свод из десяти ручек там не сообщал бы ничего.
 * Этот список читают, только когда за ним пришли, — и тогда молчать о
 * домашнем правиле нельзя вдвойне: домашнее не названо нигде больше, а
 * кусается оно точно так же, как чужое. Ровно на этом обжигается первый
 * круг: он домашний, поэтому отличием не был никогда.
 *
 * Названо только то, обо что можно удариться, — ручка, стоящая в положении
 * «ничего не делает», молчит.
 */
export function rulesInForce(rules: BattleRules | null | undefined): RuleApart[] {
  if (!rules) return [];
  const out: RuleApart[] = [];
  const say = (key: TranslationKey, amount: number | null = null) =>
    out.push({ key, amount });

  // Порядок — по тому, обо что ударяются раньше, а не по полю в структуре.
  if (rules.openingAttacks < 255) say("battleRuleOpening", rules.openingAttacks);
  if (rules.actsPerTurn < 255) say("battleRuleActs", rules.actsPerTurn);
  say(rules.walkSpendsTurn ? "battleRuleWalkSpends" : "battleRuleWalkFree");
  if (rules.retaliation) say("battleRuleRetaliation");
  if (rules.idleToll > 0) say("battleRuleIdleToll", rules.idleToll);
  if (rules.escalationFrom > 0) say("battleRuleEscalation", rules.escalationFrom);
  if (rules.pointBlankPower < 100) say("battleRulePointBlank", rules.pointBlankPower);
  if (rules.longShotPower < 100) say("battleRuleLongShot", rules.longShotPower);
  if (rules.secondSideCoin > 0) say("battleRuleCoin", rules.secondSideCoin);
  if (rules.breakthrough) say("battleRuleBreakthrough");
  say("battleRuleRounds", rules.maxRounds);

  return out;
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
  const say = (key: TranslationKey, amount: number | null = null) =>
    out.push({ key, amount });

  if (rules.walkSpendsTurn !== HOUSE_RULES.walkSpendsTurn) {
    say(rules.walkSpendsTurn ? "battleRuleWalkSpends" : "battleRuleWalkFree");
  }
  if (rules.retaliation !== HOUSE_RULES.retaliation) {
    say(
      rules.retaliation ? "battleRuleRetaliation" : "battleRuleNoRetaliation",
    );
  }
  // 255 — «сколько угодно», то есть каждое тело по разу. Число рядом с этим
  // словом было бы враньём, поэтому и потолок называется, только когда он есть.
  if (
    rules.actsPerTurn !== HOUSE_RULES.actsPerTurn &&
    rules.actsPerTurn < 255
  ) {
    say("battleRuleActs", rules.actsPerTurn);
  }
  if (rules.openingAttacks !== HOUSE_RULES.openingAttacks) {
    say(
      rules.openingAttacks >= 255
        ? "battleRuleOpeningFree"
        : "battleRuleOpening",
      rules.openingAttacks >= 255 ? null : rules.openingAttacks,
    );
  }
  if (rules.idleToll !== HOUSE_RULES.idleToll) {
    say(
      rules.idleToll === 0 ? "battleRuleNoIdleToll" : "battleRuleIdleToll",
      rules.idleToll === 0 ? null : rules.idleToll,
    );
  }
  if (
    rules.escalationFrom !== HOUSE_RULES.escalationFrom &&
    rules.escalationFrom > 0
  ) {
    say("battleRuleEscalation", rules.escalationFrom);
  }
  if (rules.maxRounds !== HOUSE_RULES.maxRounds)
    say("battleRuleRounds", rules.maxRounds);
  if (rules.secondSideCoin !== HOUSE_RULES.secondSideCoin) {
    say("battleRuleCoin", rules.secondSideCoin);
  }
  if (rules.pointBlankPower !== HOUSE_RULES.pointBlankPower) {
    say(
      rules.pointBlankPower >= 100
        ? "battleRuleNoPointBlank"
        : "battleRulePointBlank",
      rules.pointBlankPower >= 100 ? null : rules.pointBlankPower,
    );
  }
  if (rules.longShotPower !== HOUSE_RULES.longShotPower) {
    say(
      rules.longShotPower === 0 ? "battleRuleNoLongShot" : "battleRuleLongShot",
      rules.longShotPower === 0 ? null : rules.longShotPower,
    );
  }
  // Записанное без поля — «выключено»: так его и играет сервер.
  if (!!rules.breakthrough !== !!HOUSE_RULES.breakthrough) {
    say(rules.breakthrough ? "battleRuleBreakthrough" : "battleRuleNoBreakthrough");
  }
  return out;
}

/** Порядок, в котором местность называется словами и лежит в ящике стола:
 *  сперва то, что не пускает, потом то, что держит и ранит, потом то, что
 *  даёт. */
export const GROUNDS: BattleGround[] = ['wall', 'ravine', 'pit', 'mire', 'cover', 'hill', 'spring'];

/** Слово к земле: что она делает, а не как называется. */
export const GROUND_KEY: Record<BattleGround, TranslationKey> = {
  wall: 'battleGroundWall',
  ravine: 'battleGroundRavine',
  pit: 'battleGroundPit',
  mire: 'battleGroundMire',
  cover: 'battleGroundCover',
  hill: 'battleGroundHill',
  spring: 'battleGroundSpring',
};

/** Что земля делает с тем, кто на неё идёт: запрет, опасность, подмога. По
 *  этому, а не по рисунку, выбирается знак — три формы читаются с одного
 *  взгляда, семь значков не различит никто. */
export type GroundKind = 'block' | 'hazard' | 'boon';
export const GROUND_KIND: Record<BattleGround, GroundKind> = {
  wall: 'block',
  ravine: 'block',
  pit: 'hazard',
  mire: 'hazard',
  cover: 'boon',
  hill: 'boon',
  spring: 'boon',
};

/** Правило земли коротко — для легенды рядом с полем, где строка узкая. */
export const GROUND_SHORT: Record<BattleGround, TranslationKey> = {
  wall: 'battleGroundShortWall',
  ravine: 'battleGroundShortRavine',
  pit: 'battleGroundShortPit',
  mire: 'battleGroundShortMire',
  cover: 'battleGroundShortCover',
  hill: 'battleGroundShortHill',
  spring: 'battleGroundShortSpring',
};

/** Имя земли — одно слово: для стола и для подписи клетки. */
export const GROUND_NAME: Record<BattleGround, TranslationKey> = {
  wall: 'battleGroundNameWall',
  ravine: 'battleGroundNameRavine',
  pit: 'battleGroundNamePit',
  mire: 'battleGroundNameMire',
  cover: 'battleGroundNameCover',
  hill: 'battleGroundNameHill',
  spring: 'battleGroundNameSpring',
};

/** Поле, на котором играется всё, что не назвало своего. */
export const DEFAULT_FIELD: BattleField = { width: 3, depth: 3 };

/** Какие величины поля бывают — зеркало `Field::normalized` в движке. */
export const FIELD_WIDTHS = [3, 4] as const;
export const FIELD_DEPTHS = [3, 4, 5] as const;

/** Сколько клеток местности держит этюд. Зеркало `battles::TERRAIN_MAX`. */
export const TERRAIN_MAX = 9;

/** На этой земле не стоят — и сервер не примет тело над ней. */
export const UNSTANDABLE: ReadonlySet<BattleGround> = new Set(['wall', 'ravine']);

/**
 * Какая местность лежит на этом поле — по строке на род, а не на клетку:
 * три стены объясняются одной фразой. Печатается и на полке (местность —
 * всегда отличие от ровного поля), и на листке правил в бою.
 */
export function terrainLines(terrain: BattleTile[] | null | undefined): RuleApart[] {
  const have = new Set((terrain ?? []).map((t) => t.ground));
  return GROUNDS.filter((g) => have.has(g)).map((g) => ({ key: GROUND_KEY[g], amount: null }));
}

/** Пустая карта — манекен, на котором примеряют раму.
 *
 *  Лежала в админке; студии нужна та же, и второй её копии быть не должно:
 *  карта прирастает полями, и забытый близнец — это поле, которого на манекене
 *  нет, то есть предпросмотр, который однажды соврёт.
 */
/** Пустое тело своей карты — ровно тот запрос, каким карту сохраняет и стол
 *  хозяина. Домовые поля (цена, слуг, наряд) сюда НЕ кладутся вовсе: их
 *  назначает дом, и место для них на странице было бы обещанием, которого
 *  сервер не выполнит. */
export function emptyCardRequest(): SaveBattleCardRequest {
  return {
    status: "draft",
    tier: 1,
    raceId: null,
    typeEn: null,
    typeRu: null,
    titleEn: "",
    titleRu: "",
    effectEn: null,
    effectRu: null,
    loreEn: null,
    loreRu: null,
    cost: 1,
    power: 1,
    health: 3,
    mana: 0,
    traits: [],
    kind: "unit",
    armor: 0,
    ward: 0,
    attackChannel: "physical",
    reach: 1,
    step: 1,
    speed: 3,
    mend: 0,
    abilities: [],
    artUrl: null,
    artFocal: null,
  };
}

/** Тело работы, показанное КАРТОЙ. Отрисовщик один на весь дом, и кормить его
 *  надо тем же, чем кормят полку, — поэтому запрос раскладывается в карту, а
 *  не рисуется вторым способом. */
export function cardFromRequest(
  req: SaveBattleCardRequest,
  races: BattleRace[] = [],
): BattleCard {
  const race = races.find((r) => r.id === req.raceId) ?? null;
  return {
    ...emptyBattleCard(),
    ...req,
    // У работы слуга нет и быть не может: его назначает дом при утверждении.
    slug: "",
    raceId: req.raceId ?? null,
    raceNameEn: race?.nameEn ?? null,
    raceNameRu: race?.nameRu ?? null,
    raceIconUrl: race?.iconUrl ?? null,
    raceLevelFrames: race?.levelFrames ?? null,
  };
}

export function emptyBattleCard(): BattleCard {
  return {
    id: "",
    slug: "",
    status: "draft",
    tier: 1,
    raceId: null,
    raceNameEn: null,
    raceNameRu: null,
    raceIconUrl: null,
    raceLevelFrames: null,
    typeEn: null,
    typeRu: null,
    titleEn: "",
    titleRu: "",
    effectEn: null,
    effectRu: null,
    loreEn: null,
    loreRu: null,
    cost: 1,
    power: 1,
    health: 0,
    mana: 0,
    traits: [],
    kind: "unit",
    armor: 0,
    ward: 0,
    attackChannel: "physical",
    reach: 1,
    step: 1,
    speed: 3,
    mend: 0,
    abilities: [],
    budgetPoints: null,
    balanceIndex: null,
    rulesVersion: 1,
    priceDust: null,
    priceFeed: null,
    levelPriceDust: null,
    lendable: false,
    creditName: null,
    editionSize: null,
    minted: 0,
    artUrl: null,
    artUrlOverride: null,
    artFocal: null,
    frameOverride: null,
    motionWear: null,
    shelfOrder: null,
    figurineId: null,
    figurineName: null,
    figurineSlug: null,
    createdAt: "",
    updatedAt: "",
  };
}

/** Отказ стола приходит СЛОВОМ (`deck:notYours`), а не текстом: текст живёт
 *  здесь, на двух языках, а сервер, который его сочиняет, сочиняет его на
 *  одном. Незнакомое слово не молчит — комната говорит общее.
 *
 *  Лежит в одном месте, потому что спрашивают его ДВОЕ: стол колоды, когда её
 *  сохраняют, и этюд, когда с нею начинают партию. Второй не спрашивал вовсе —
 *  колода, собранная из карт, которых у гостя больше нет, отвечала гостю
 *  «ход потерян» на четырёхсотый ответ сервера, и починить её по этим словам
 *  было нельзя.
 */
export function deckFaultLine(
  e: unknown,
  tr: (key: TranslationKey) => string,
  fallback: TranslationKey,
): string {
  const word = String(e).match(/deck:(\w+)/)?.[1];
  if (!word) return tr(fallback);
  const key =
    `battlesDeckFault${word[0].toUpperCase()}${word.slice(1)}` as TranslationKey;
  const said = tr(key);
  return said && said !== key ? said : tr(fallback);
}
