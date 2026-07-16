import type { Figurine, FigurineImage } from '$lib/types/api';
import type { TranslationKey } from '$lib/i18n';

/** i18n strings the formula below is built from — see altLabelsFrom() below. */
export type AltTextLabels = {
  face: string;
  detail: string;
  full: string;
  default: string;
  /** Trailing context phrase; brand token already substituted by t(). */
  context: string;
};

/**
 * Builds the AltTextLabels bundle from a `$t`-like translate function. Every caller of
 * formatFigurineAlt (the gallery, the admin auto-fill button, the JSON-LD caption, the
 * passport hero) needs this exact same five-key object — pulling it through one shared
 * function keeps them from drifting independently out of sync with each other.
 */
export function altLabelsFrom(translate: (key: TranslationKey) => string): AltTextLabels {
  return {
    face: translate('altTypeFace'),
    detail: translate('altTypeDetail'),
    full: translate('altTypeFull'),
    default: translate('altTypeDefault'),
    context: translate('altContext'),
  };
}

/**
 * Where `image` sits among its same-`imageType` siblings in `images` — the
 * disambiguation formatFigurineAlt needs so multiple photos of the same type don't
 * get identical alt text. `undefined` when there's only one (or the image isn't in
 * the list at all), meaning no "(n/total)" suffix is needed.
 */
export function siblingPosition(
  images: FigurineImage[],
  image: FigurineImage,
): { index: number; total: number } | undefined {
  const sameType = images.filter((i) => i.imageType === image.imageType);
  if (sameType.length <= 1) return undefined;
  const index = sameType.indexOf(image);
  return index === -1 ? undefined : { index, total: sameType.length };
}

// Google's image-SEO guidance: alt text should read as a natural description in the
// 50-125 character range — long enough to be specific, short enough to stay a label
// rather than a paragraph.
const MAX_LEN = 125;
const MIN_LEN = 50;

function typePhrase(labels: AltTextLabels, imageType: FigurineImage['imageType'] | null | undefined): string {
  switch (imageType) {
    case 'face': return labels.face;
    case 'detail': return labels.detail;
    case 'full': return labels.full;
    default: return labels.default;
  }
}

// Never cut mid-word: back up to the last space inside the budget.
function clampToMax(text: string): string {
  if (text.length <= MAX_LEN) return text;
  const cut = text.slice(0, MAX_LEN);
  const lastSpace = cut.lastIndexOf(' ');
  return (lastSpace > MIN_LEN ? cut.slice(0, lastSpace) : cut).trimEnd();
}

/**
 * Compose a fallback alt text following the formula
 * [type] + [subject] + [material/detail] + [context] — used wherever an admin
 * hasn't written a custom `altText` for a photo (see `img.altText ??` call sites).
 *
 * `position` disambiguates multiple images of the same `imageType` on one figurine
 * (e.g. three 'detail' shots): without it every one of them would get identical alt
 * text, which is exactly what Google's per-image alt guidance says not to do.
 */
export function formatFigurineAlt(
  figurine: Pick<Figurine, 'name' | 'material' | 'technique'>,
  imageType: FigurineImage['imageType'] | null | undefined,
  labels: AltTextLabels,
  position?: { index: number; total: number },
): string {
  const type = typePhrase(labels, imageType);
  const subject = figurine.name?.trim() ?? '';
  const detail = figurine.material?.trim() || figurine.technique?.trim() || '';
  // No imageType exclusion here on purpose: the admin UI keeps at most one 'face'
  // image, but nothing at the data layer enforces that (no DB uniqueness constraint),
  // so two same-typed images — 'face' included — must still be disambiguated or they
  // get byte-identical alt text, defeating the whole point of `position`.
  const variant = position && position.total > 1
    ? ` (${position.index + 1}/${position.total})`
    : '';

  const base = (subject ? `${type} — ${subject}` : type) + variant;
  const withDetail = detail ? `${base}, ${detail}` : base;
  const full = `${withDetail}, ${labels.context}`;

  if (full.length <= MAX_LEN) return full;
  if (withDetail.length <= MAX_LEN) return withDetail;
  return clampToMax(base.length <= MAX_LEN ? base : full);
}
