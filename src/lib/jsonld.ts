/**
 * Escape a value for safe embedding inside an inline `<script type="application/ld+json">`.
 *
 * Every field folded into a page's JSON-LD is admin-authored (figurine name, altText,
 * brand, bio copy…), and a closing script tag inside any of it would end the block
 * early and let the rest be parsed as HTML. Escaping every `<` is enough — JSON has no
 * other way to spell one. (The literal tag is deliberately never written out, even in
 * comments referencing this function: an HTML tokenizer ends a script block on sight of
 * it, comment or not, which is exactly the flaw this guards against.)
 *
 * Shared by every route that emits JSON-LD from admin-controlled data, so a future
 * widening of the escaping rule (e.g. to also cover U+2028/U+2029 line separators) only
 * has to be made once.
 */
export function jsonLdSafe(value: unknown): string {
  return JSON.stringify(value).replaceAll('<', '\\u003c');
}
