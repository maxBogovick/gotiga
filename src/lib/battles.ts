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

import type { BattleCard, BattleFrame } from '$lib/types/api';
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
export const DEFAULT_FRAMES: BattleFrame[] = [
  { tier: 1, nameEn: 'Humble',      nameRu: 'Скромная',   paper: '#f8f1e7', ink: '#34251c', border: '#d8c6b1', foil: '' },
  { tier: 2, nameEn: 'Sturdy',      nameRu: 'Крепкая',    paper: '#f3e9db', ink: '#34251c', border: '#c3ad93', foil: '' },
  { tier: 3, nameEn: 'Remembered',  nameRu: 'Памятная',   paper: '#eeddc8', ink: '#34251c', border: '#a8845f', foil: 'rgba(198,95,60,0.16)' },
  { tier: 4, nameEn: 'Rare',        nameRu: 'Редкая',     paper: '#e6cfb2', ink: '#2a1a11', border: '#6f3b24', foil: 'rgba(198,95,60,0.28)' },
  { tier: 5, nameEn: 'Epic',        nameRu: 'Эпическая',  paper: '#3a2a1e', ink: '#f3e4cd', border: '#c99a52', foil: 'rgba(214,178,110,0.42)' },
];

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
