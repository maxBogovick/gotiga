import type { GazetteCutting, GazetteKind, GazetteLeaf } from '$lib/types/api';
import type { Lang, TranslationKey } from '$lib/i18n';

export const TITLE_MAX = 200;
export const DEK_MAX = 500;
export const BODY_MAX = 12_000;

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
  const titleEn = (leaf.titleEn ?? '').trim();
  const titleRu = (leaf.titleRu ?? '').trim();
  const title = (ru && titleRu ? titleRu : titleEn) || titleRu;
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
  markKey?: string;
  markUrl?: string | null;
  letter?: string;
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
    out.push({
      id: `cut-${cut.id}`,
      title,
      markKey: cut.markKey,
      markUrl: cut.markUrl,
      letter: cut.sourceName,
    });
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

/** Starter copy for a new gazette note. The admin edits before publishing. */
export function fillTemplate(kind: GazetteKind, name: string): GazetteTemplateFill {
  const n = name.trim();
  switch (kind) {
    case 'arrival':
      return {
        titleEn: n || 'New work',
        titleRu: n || 'Новая работа',
        dekEn: n ? `${n} has been added to the catalogue.` : 'A new work has been added to the catalogue.',
        dekRu: n ? `«${n}» появилась в каталоге.` : 'Новая работа появилась в каталоге.',
      };
    case 'showing':
      return {
        titleEn: n ? `Showing: ${n}` : 'Upcoming showing',
        titleRu: n ? `Показ: ${n}` : 'Анонс показа',
        dekEn: 'Dates and details of the showing.',
        dekRu: 'Даты и детали показа.',
      };
    case 'collage':
    case 'guest_story':
    case 'tale':
    case 'note':
      return {
        titleEn: n ? n : '',
        titleRu: n ? n : '',
        dekEn: '',
        dekRu: '',
      };
    case 'world':
      return {
        titleEn: n,
        titleRu: n,
        dekEn: '',
        dekRu: '',
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

export function isGazetteYearSlug(slug: string): boolean {
  return /^\d{4}$/.test(slug);
}

/** API doors that live under /gazette/* and must never be treated as a leaf. */
export function isGazetteReservedSlug(slug: string): boolean {
  return slug === 'home' || slug === 'room' || slug === 'blotter' || slug === 'for-work';
}

export function leafWhen(leaf: GazetteLeaf): string {
  return leaf.publishedAt ?? leaf.scheduledAt ?? leaf.createdAt;
}

export function cuttingWhen(cut: GazetteCutting): string {
  return cut.publishedAt ?? cut.createdAt;
}

export function quietDate(iso: string | null | undefined, lang: Lang): string {
  if (!iso) return '';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleDateString(lang === 'ru' ? 'ru-RU' : 'en-GB', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  });
}

export function monthLabel(iso: string, lang: Lang): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return '';
  return d.toLocaleDateString(lang === 'ru' ? 'ru-RU' : 'en-GB', {
    month: 'long',
    year: 'numeric',
  });
}

export function roomDateline(
  leaves: GazetteLeaf[],
  cuttings: GazetteCutting[],
  year: number,
  lang: Lang,
): string {
  let newest = 0;
  let iso = '';
  for (const leaf of leaves) {
    const t = new Date(leafWhen(leaf)).getTime();
    if (Number.isFinite(t) && t > newest) {
      newest = t;
      iso = leafWhen(leaf);
    }
  }
  for (const cut of cuttings) {
    const stamp = cuttingWhen(cut);
    const t = new Date(stamp).getTime();
    if (Number.isFinite(t) && t > newest) {
      newest = t;
      iso = stamp;
    }
  }
  if (!iso) return String(year);
  return monthLabel(iso, lang);
}

export function neighborTitle(n: { titleEn: string; titleRu: string }, lang: Lang): string {
  const raw = (lang === 'ru' && n.titleRu.trim() ? n.titleRu : n.titleEn).trim();
  return decodeEntities(raw);
}

export function yearHref(year: number, latestYear: number | undefined): string {
  if (latestYear != null && year === latestYear) return '/gazette';
  return `/gazette/${year}`;
}

const KIND_ORDER: GazetteKind[] = [
  'arrival',
  'collage',
  'showing',
  'guest_story',
  'tale',
  'note',
  'world',
];

export interface GazetteMonthBand<T> {
  iso: string;
  items: T[];
}

function monthKey(iso: string): string | null {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`;
}

function groupByMonth<T>(items: T[], when: (item: T) => string): GazetteMonthBand<T>[] {
  const bands: GazetteMonthBand<T>[] = [];
  const index = new Map<string, number>();
  for (const item of items) {
    const iso = when(item);
    const key = monthKey(iso);
    if (!key) continue;
    const at = index.get(key);
    if (at == null) {
      index.set(key, bands.length);
      bands.push({ iso, items: [item] });
    } else {
      bands[at].items.push(item);
    }
  }
  return bands;
}

export function groupCuttingsByMonth(cuttings: GazetteCutting[]): GazetteMonthBand<GazetteCutting>[] {
  return groupByMonth(cuttings, cuttingWhen);
}

export interface GazetteKindBand {
  kind: GazetteKind;
  items: GazetteLeaf[];
}

export interface GazetteHouseMonth {
  iso: string;
  kinds: GazetteKindBand[];
}

/** Month bands; kind subheads only when a month holds more than one kind. */
export function groupLeavesByMonthAndKind(leaves: GazetteLeaf[]): GazetteHouseMonth[] {
  return groupByMonth(leaves, leafWhen).map((band) => {
    const buckets = new Map<GazetteKind, GazetteLeaf[]>();
    for (const leaf of band.items) {
      const list = buckets.get(leaf.kind) ?? [];
      list.push(leaf);
      buckets.set(leaf.kind, list);
    }
    const first = band.items[0];
    if (!first) return { iso: band.iso, kinds: [] };
    const kinds: GazetteKindBand[] =
      buckets.size <= 1
        ? [{ kind: first.kind, items: band.items }]
        : KIND_ORDER.filter((k) => buckets.has(k)).map((kind) => ({
            kind,
            items: buckets.get(kind) ?? [],
          }));
    return { iso: band.iso, kinds };
  });
}
