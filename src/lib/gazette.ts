import { figurineHref } from '$lib/figurineHref';
import type { FigurineListItem, GazetteCutting, GazetteKind, GazetteLeaf } from '$lib/types/api';
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
  sketch: 'gazetteKind_sketch',
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
  // Fall back to whichever language was actually written, in both directions.
  // The reader's own language wins; an empty one must not blank the page. This
  // used to be asymmetric — a leaf written only in Russian showed nothing at
  // all to an English reader, while the reverse read fine.
  const dek = (ru ? leaf.dekRu?.trim() || leaf.dekEn?.trim() : leaf.dekEn?.trim() || leaf.dekRu?.trim()) ?? '';
  const body = (ru ? leaf.bodyRu?.trim() || leaf.bodyEn?.trim() : leaf.bodyEn?.trim() || leaf.bodyRu?.trim()) ?? '';
  return {
    title: decodeEntities(title),
    dek: decodeEntities(dek),
    body: decodeEntities(body),
  };
}

export type GazetteSlipKind = 'leaf' | 'cut' | 'work';

export interface GazetteSlip {
  id: string;
  title: string;
  href: string;
  kind: GazetteSlipKind;
  markKey?: string;
  markUrl?: string | null;
  letter?: string;
  dateLabel?: string;
  imageUrl?: string | null;
  external?: boolean;
}

export type GazettePlateWork = Pick<
  FigurineListItem,
  'id' | 'slug' | 'name' | 'createdAt' | 'faceImageUrl' | 'thumbUrl'
>;

function workSlip(fig: GazettePlateWork, lang: Lang): GazetteSlip | null {
  const title = fig.name.trim();
  if (!title) return null;
  const imageUrl = (fig.thumbUrl ?? fig.faceImageUrl)?.trim() || null;
  return {
    id: `work-${fig.id}`,
    title,
    href: figurineHref(fig, 'home_plate'),
    kind: 'work',
    dateLabel: quietDate(fig.createdAt, lang),
    imageUrl,
  };
}

/**
 * Slips for the hero plate: the newest work is always mixed in, then house
 * leaves, then world cuttings (pinned first, as the API already sorts them).
 * Each slip opens its own leaf, cutting, or figurine — never the gazette index.
 */
export function plateSlips(
  leaves: GazetteLeaf[],
  cuttings: GazetteCutting[],
  lang: Lang,
  latestWork?: GazettePlateWork | null,
  max = 6,
): GazetteSlip[] {
  const mixed = latestWork ? workSlip(latestWork, lang) : null;
  const skipArrivalId = mixed ? latestWork?.id : null;
  const budget = mixed ? Math.max(0, max - 1) : max;
  const out: GazetteSlip[] = [];

  for (const leaf of leaves) {
    if (out.length >= budget) break;
    if (skipArrivalId && leaf.figurineId === skipArrivalId && leaf.kind === 'arrival') continue;
    const title = leafCopy(leaf, lang).title;
    if (!title) continue;
    const cover = leafCoverUrl(leaf);
    out.push({
      id: `leaf-${leaf.id}`,
      title,
      href: leafHref(leaf, 'home_plate'),
      kind: 'leaf',
      dateLabel: quietDate(leafWhen(leaf), lang),
      imageUrl: cover || null,
    });
  }
  for (const cut of cuttings) {
    if (out.length >= budget) break;
    const title = decodeEntities(cut.title.trim());
    if (!title) continue;
    const href = cut.url.trim();
    if (!href) continue;
    out.push({
      id: `cut-${cut.id}`,
      title,
      href,
      kind: 'cut',
      external: true,
      markKey: cut.markKey,
      markUrl: cut.markUrl,
      letter: cut.sourceName,
    });
  }

  if (mixed) out.unshift(mixed);
  return out;
}

export interface GazetteTemplateFill {
  titleEn: string;
  titleRu: string;
  dekEn: string;
  dekRu: string;
}

function dateOnlyIso(value: string): string {
  return /^\d{4}-\d{2}-\d{2}$/.test(value) ? `${value}T12:00:00` : value;
}

/** Venue and dates for a showing leaf, in the language of the dek. */
export function showingDateline(
  startsAt: string | null | undefined,
  endsAt: string | null | undefined,
  venue: string | null | undefined,
  lang: Lang,
): string {
  const start = startsAt?.trim() ? quietDate(dateOnlyIso(startsAt.trim()), lang) : '';
  const end = endsAt?.trim() ? quietDate(dateOnlyIso(endsAt.trim()), lang) : '';
  const range = start && end && start !== end ? `${start} — ${end}` : start || end;
  const place = venue?.trim() ?? '';
  return [place, range].filter(Boolean).join('. ');
}

/** Distinct face / detail URLs to pick a frame for a gazette slip. */
export function workFrameUrls(
  fig: {
    faceImageUrl?: string | null;
    detailImageUrl?: string | null;
    faceImageLargeUrl?: string | null;
    detailImageLargeUrl?: string | null;
  },
  extra?: { images?: { url: string }[] } | null,
): string[] {
  const out: string[] = [];
  const add = (u?: string | null) => {
    const v = u?.trim();
    if (v && !out.includes(v)) out.push(v);
  };
  add(fig.faceImageUrl);
  add(fig.detailImageUrl);
  add(fig.faceImageLargeUrl);
  add(fig.detailImageLargeUrl);
  for (const img of extra?.images ?? []) add(img.url);
  return out;
}

export const SKETCH_MAX = 8;

/** Sketches and bench photos for a work still taking shape. Process steps first. */
export function sketchUrlsFromWork(fig: {
  processSteps?: { imageUrl?: string | null; stepType?: string }[];
  images?: { url: string }[];
  faceImageUrl?: string | null;
  detailImageUrl?: string | null;
}): string[] {
  const out: string[] = [];
  const add = (u?: string | null) => {
    const v = u?.trim();
    if (v && !out.includes(v) && out.length < SKETCH_MAX) out.push(v);
  };
  const steps = fig.processSteps ?? [];
  for (const s of steps.filter((s) => s.stepType === 'sketch')) add(s.imageUrl);
  for (const s of steps.filter((s) => s.stepType !== 'sketch')) add(s.imageUrl);
  for (const img of fig.images ?? []) add(img.url);
  add(fig.faceImageUrl);
  add(fig.detailImageUrl);
  return out;
}

export function leafImageList(leaf: { imageUrl?: string | null; imageUrls?: string[] | null }): string[] {
  if (leaf.imageUrls?.length) return leaf.imageUrls.filter((u) => !!u?.trim());
  const cover = leaf.imageUrl?.trim();
  return cover ? [cover] : [];
}

export function leafCoverUrl(leaf: { imageUrl?: string | null; imageUrls?: string[] | null }): string {
  return leafImageList(leaf)[0] ?? '';
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
    case 'sketch':
      return {
        titleEn: n || 'In the making',
        titleRu: n || 'В работе',
        dekEn: n ? `${n} is still in the making.` : 'Sketches from the workshop.',
        dekRu: n ? `«${n}» ещё в работе.` : 'Наброски из мастерской.',
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
  // The gazette announces a tale; the shelf is where it lives. Routing that
  // choice through this one function is what keeps the vestnik, the hall and
  // every work page pointing at the room without each of them knowing about it.
  const base = leaf.kind === 'tale' ? `/tales/${leaf.slug}` : `/gazette/${leaf.slug}`;
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
  return slug === 'home' || slug === 'room' || slug === 'blotter' || slug === 'for-work' || slug === 'watch';
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

function dateOnly(iso: string): string {
  return iso.slice(0, 10);
}

function todayStamp(now = new Date()): string {
  const y = now.getFullYear();
  const m = String(now.getMonth() + 1).padStart(2, '0');
  const d = String(now.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

function quietDay(ymd: string, lang: Lang): string {
  return quietDate(`${ymd}T12:00:00`, lang);
}

/** Whispered window for a sketch leaf. Empty when the whole span is already past. */
export function expectedWhen(
  from: string | null | undefined,
  to: string | null | undefined,
  lang: Lang,
  around: (date: string) => string,
  range: (a: string, b: string) => string,
  now = new Date(),
): string {
  const start = dateOnly(from || to || '');
  const end = dateOnly(to || from || '');
  if (!start || !end) return '';
  if (end < todayStamp(now)) return '';
  if (start === end) return around(quietDay(start, lang));
  return range(quietDay(start, lang), quietDay(end, lang));
}

export function expectedWhisper(
  leaf: { kind: string; expectedFrom?: string | null; expectedTo?: string | null },
  lang: Lang,
  around: (date: string) => string,
  range: (a: string, b: string) => string,
  now = new Date(),
): string {
  if (leaf.kind !== 'sketch') return '';
  return expectedWhen(leaf.expectedFrom, leaf.expectedTo, lang, around, range, now);
}

export function sketchLaidOut(leaf: { kind: string; figurineStatus?: string | null }): boolean {
  return leaf.kind === 'sketch' && leaf.figurineStatus === 'available';
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
  'sketch',
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
