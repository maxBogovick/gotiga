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
 * The one tale the room shows large.
 *
 * `pinned` is the keeper's choice and wins; otherwise the tale standing first
 * on the shelf takes the place. Never random — the room is prerendered, and a
 * random pick would be frozen into the build anyway.
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
 * How tall a spine stands on the shelf.
 *
 * Books are not the same height, and a shelf of identical spines reads as a
 * table of contents. Derived from the slug so it is the same for every visitor
 * and survives prerendering — a random height would shuffle on every build.
 */
export function spineHeight(slug: string, min = 244, max = 340): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < slug.length; i++) {
    h ^= slug.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return min + (h % (max - min + 1));
}
