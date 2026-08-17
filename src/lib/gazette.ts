import type { GazetteCutting, GazetteKind, GazetteLeaf } from '$lib/types/api';
import type { Lang, TranslationKey } from '$lib/i18n';

export const GAZETTE_KIND_KEY: Record<GazetteKind, TranslationKey> = {
  arrival: 'gazetteKind_arrival',
  collage: 'gazetteKind_collage',
  showing: 'gazetteKind_showing',
  guest_story: 'gazetteKind_guest_story',
  tale: 'gazetteKind_tale',
  note: 'gazetteKind_note',
  world: 'gazetteKind_world',
};

const NAMED_ENTITIES: Record<string, string> = {
  amp: '&',
  lt: '<',
  gt: '>',
  quot: '"',
  apos: "'",
  nbsp: ' ',
  rsquo: '\u2019',
  lsquo: '\u2018',
  rdquo: '\u201D',
  ldquo: '\u201C',
  mdash: '\u2014',
  ndash: '\u2013',
  hellip: '\u2026',
};

/** RSS titles often arrive with `&#8217;` still encoded. House copy should not. */
export function decodeEntities(s: string): string {
  if (!s.includes('&')) return s;
  return s.replace(/&(#x[0-9a-fA-F]+|#\d+|[a-zA-Z][a-zA-Z0-9]+);/g, (full, ent: string) => {
    if (ent[0] === '#') {
      const hex = ent[1] === 'x' || ent[1] === 'X';
      const code = hex ? parseInt(ent.slice(2), 16) : parseInt(ent.slice(1), 10);
      if (Number.isFinite(code) && code > 0 && code <= 0x10ffff) {
        try {
          return String.fromCodePoint(code);
        } catch {
          return full;
        }
      }
      return full;
    }
    return NAMED_ENTITIES[ent.toLowerCase()] ?? full;
  });
}

/** Pick the language the visitor is reading, falling back to English. */
export function leafCopy(leaf: GazetteLeaf, lang: Lang): { title: string; dek: string; body: string } {
  const ru = lang === 'ru';
  const title = (ru && leaf.titleRu.trim() ? leaf.titleRu : leaf.titleEn).trim();
  const dek = (ru && leaf.dekRu?.trim() ? leaf.dekRu : leaf.dekEn)?.trim() ?? '';
  const body = (ru && leaf.bodyRu?.trim() ? leaf.bodyRu : leaf.bodyEn)?.trim() ?? '';
  return {
    title: decodeEntities(title),
    dek: decodeEntities(dek),
    body: decodeEntities(body),
  };
}

export interface GazetteSlip {
  id: string;
  title: string;
}

/**
 * Slips for the hero plate: house leaves first, then world cuttings
 * (pinned first, as the API already sorts them). Enough items that the
 * square can actually turn; a single slip would sit still.
 */
export function plateSlips(
  leaves: GazetteLeaf[],
  cuttings: GazetteCutting[],
  lang: Lang,
  max = 6,
): GazetteSlip[] {
  const out: GazetteSlip[] = [];
  for (const leaf of leaves) {
    const title = leafCopy(leaf, lang).title;
    if (!title) continue;
    out.push({ id: `leaf-${leaf.id}`, title });
    if (out.length >= max) return out;
  }
  for (const cut of cuttings) {
    const title = decodeEntities(cut.title.trim());
    if (!title) continue;
    out.push({ id: `cut-${cut.id}`, title });
    if (out.length >= max) break;
  }
  return out;
}

export interface GazetteTemplateFill {
  titleEn: string;
  titleRu: string;
  dekEn: string;
  dekRu: string;
}

/** House-voice starters. The keeper edits after the template is laid down. */
export function fillTemplate(kind: GazetteKind, name: string): GazetteTemplateFill {
  const n = name.trim() || 'a work';
  switch (kind) {
    case 'arrival':
      return {
        titleEn: `Laid out today: ${n}`,
        titleRu: `Сегодня на стол легла «${n}»`,
        dekEn: 'A new work has been set on the table. Come look while the dust still settles.',
        dekRu: 'Новая работа уже в доме. Кто зайдёт — увидит её первой.',
      };
    case 'collage':
      return {
        titleEn: `A collage has been laid beside the works: ${n}`,
        titleRu: `Рядом с работами появился коллаж: «${n}»`,
        dekEn: 'A small arrangement, to be looked at here.',
        dekRu: 'Небольшая композиция — можно посмотреть здесь.',
      };
    case 'showing':
      return {
        titleEn: `The house will open ${n} to the first glance`,
        titleRu: `Дом откроет «${n}» для первого взгляда`,
        dekEn: 'Those who are here will see it first.',
        dekRu: 'Кто будет в доме — увидит первым.',
      };
    case 'guest_story':
      return {
        titleEn: `A guest will speak of ${n}`,
        titleRu: `Гость расскажет о работе «${n}»`,
        dekEn: 'A visitor’s account of this piece is being set down.',
        dekRu: 'Готовится рассказ гостя об этой работе автора.',
      };
    case 'tale':
      return {
        titleEn: n === 'a work' ? 'A short tale about such figures has been set down' : `A short tale beside ${n}`,
        titleRu: n === 'a work' ? 'Появился маленький рассказ про такие фигурки' : `Маленький рассказ рядом с «${n}»`,
        dekEn: 'Come read it in the quiet of the house.',
        dekRu: 'Заходите, оцените — он лежит среди листков.',
      };
    case 'world':
      return {
        titleEn: n,
        titleRu: n,
        dekEn: 'A cutting the keeper pinned from the world.',
        dekRu: 'Вырезка, которую хранитель приколол со стола мира.',
      };
    default:
      return { titleEn: '', titleRu: '', dekEn: '', dekRu: '' };
  }
}

export function leafHref(leaf: GazetteLeaf, source?: string): string {
  const base = `/gazette/${leaf.slug}`;
  return source ? `${base}?src=${encodeURIComponent(source)}` : base;
}

export function workHref(leaf: GazetteLeaf, source?: string): string | null {
  if (!leaf.figurineId) return null;
  const handle = leaf.figurineSlug || leaf.figurineId;
  const base = `/figurines/${handle}`;
  return source ? `${base}?src=${encodeURIComponent(source)}` : base;
}

export function quietDate(iso: string | null | undefined, lang: Lang): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleDateString(lang === 'ru' ? 'ru-RU' : 'en-GB', {
    day: 'numeric',
    month: 'long',
  });
}
