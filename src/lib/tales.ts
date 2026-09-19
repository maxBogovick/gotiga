// The shelf of tall tales.
//
// A tale is a gazette leaf of kind `tale` — same table, same admin plumbing.
// What is its own is the room: an address of its own, a shelf arranged by
// hand instead of by date, and prose broken into paragraphs and ornaments.

import type { GazetteLeaf } from '$lib/types/api';

export const TALE_KIND = 'tale';

/** The gazette may announce a tale; this is where the tale itself lives. */
export function taleHref(leaf: { slug: string }, source?: string): string {
  const base = `/tales/${leaf.slug}`;
  return source ? `${base}?src=${encodeURIComponent(source)}` : base;
}

export function isTale(leaf: { kind: string }): boolean {
  return leaf.kind === TALE_KIND;
}

/**
 * The tale that stands first in the arcade.
 *
 * `pinned` is the keeper's choice and wins; otherwise the tale standing first
 * on the shelf takes the place. Never random — the room is prerendered, and a
 * random pick would be frozen into the build anyway.
 *
 * It used to be the one tale shown large. The arches are all one size now —
 * an arcade whose spans differ is not an arcade — so the keeper's pin decides
 * who stands first and nothing else.
 */
export function leadTale(tales: GazetteLeaf[]): GazetteLeaf | null {
  return tales.find((t) => t.pinned) ?? tales[0] ?? null;
}

export type TaleBlock = { kind: 'p'; text: string } | { kind: 'ornament' };

/** A lone ✦ on its own line: a turn in the tale, not just a new paragraph. */
export const ORNAMENT = '✦';

function isOrnament(line: string): boolean {
  return line === ORNAMENT || line === '*' || line === '***';
}

/**
 * Prose into blocks, read one line at a time. A blank line ends a paragraph;
 * a line that is nothing but the ornament ends one too and becomes a turn.
 *
 * Line by line rather than paragraph by paragraph on purpose: an ornament
 * written without blank lines around it sits *inside* a paragraph chunk, and
 * splitting on blank lines first would hoist it above the prose it divides.
 *
 * Deliberately not markdown and deliberately not HTML: the body is written by
 * one person in one house and rendered as text, so nothing here can carry
 * markup into the page — which is why the reading room needs no sanitizer.
 */
export function renderTale(body: string | null | undefined): TaleBlock[] {
  const blocks: TaleBlock[] = [];
  let held: string[] = [];

  const flush = () => {
    const text = held.join(' ').replace(/\s+/g, ' ').trim();
    if (text) blocks.push({ kind: 'p', text });
    held = [];
  };

  for (const raw of (body ?? '').replace(/\r\n?/g, '\n').split('\n')) {
    const line = raw.trim();
    if (!line) {
      flush();
    } else if (isOrnament(line)) {
      flush();
      // Two ornaments in a row divide nothing between them.
      if (blocks[blocks.length - 1]?.kind !== 'ornament') blocks.push({ kind: 'ornament' });
    } else {
      held.push(line);
    }
  }
  flush();

  // An ornament at either end has nothing on one side of it.
  while (blocks.length && blocks[0].kind === 'ornament') blocks.shift();
  while (blocks.length && blocks[blocks.length - 1].kind === 'ornament') blocks.pop();
  return blocks;
}

/**
 * How long the tale takes to read, in minutes.
 *
 * Counted here rather than stored: the shelf payload already carries the whole
 * body, so a column on the leaf would be a second source for one number that is
 * derived from the first — and the two would part company the first time a tale
 * is edited. 170 words a minute is unhurried reading: these are read at the pace
 * of a tall tale, not of a notice.
 */
export function readingMinutes(body: string | null | undefined): number {
  const words = (body ?? '').match(/[\p{L}\p{N}]+/gu)?.length ?? 0;
  // Zero words is zero minutes, and the caller prints nothing. A floor of one
  // would put "1 min" under a leaf that has no prose in it yet — a number that
  // lies quietly, which is worse than no number.
  if (!words) return 0;
  return Math.max(1, Math.round(words / 170));
}

/**
 * Which of the three Russian forms the minute count takes.
 *
 * No language is asked for on purpose. English has one form and spells all
 * three keys the same, so picking by the Russian rule lands on the right string
 * in both languages, and a `lang` argument here would be a branch that never
 * branches.
 */
export function minutesKey(mins: number): 'talesMinuteOne' | 'talesMinuteFew' | 'talesMinuteMany' {
  const ten = mins % 10;
  const hundred = mins % 100;
  if (ten === 1 && hundred !== 11) return 'talesMinuteOne';
  if (ten >= 2 && ten <= 4 && (hundred < 12 || hundred > 14)) return 'talesMinuteFew';
  return 'talesMinuteMany';
}

/**
 * The name the tale's photograph carries across a navigation.
 *
 * Deliberately the work's own name (`figurine-{id}`) rather than the tale's:
 * the shelf, the tale and the work all print the same photograph, and one name
 * across the whole chain gives the morph on every one of those trips instead of
 * on one of them. A tale with no work behind it gets a name of its own — its
 * cover still has somewhere to travel.
 */
export function taleMorph(leaf: { id: string; figurineId?: string | null }): string {
  return leaf.figurineId ? `figurine-${leaf.figurineId}` : `tale-${leaf.id}`;
}

/**
 * The same names for a whole shelf, with every repeat dropped.
 *
 * Two tales about one work is ordinary, and the name is the work's, so both
 * would carry it. Two identical `view-transition-name`s on one rendered page
 * abort the transition entirely — not just for those two, for the page — so
 * both are dropped rather than one kept: a morph that picks between two
 * identical names is worse than no morph.
 *
 * Keyed by leaf id, and a tale absent from the map simply travels without a
 * name.
 */
export function taleMorphNames(tales: GazetteLeaf[]): Map<string, string> {
  const count = new Map<string, number>();
  for (const tale of tales) {
    const name = taleMorph(tale);
    count.set(name, (count.get(name) ?? 0) + 1);
  }
  const out = new Map<string, string>();
  for (const tale of tales) {
    const name = taleMorph(tale);
    if (count.get(name) === 1) out.set(tale.id, name);
  }
  return out;
}
