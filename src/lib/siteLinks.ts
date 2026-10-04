// Links to this house, written into prose by hand.
//
// A tale is plain text (see `renderTale`), so a link in it is whatever the
// author pasted: `https://ritunia.com/figurines/…`, with or without the scheme,
// with or without `www`. Printed as typed, it is a raw address in the middle of
// a sentence. Here it is recognised, told which room it leads to, and handed a
// name — the work's, the tale's, the room's — so the page can print that
// instead.
//
// Only this house. A link elsewhere stays text: the prose is not a place to
// send readers away from, and there is nothing of a stranger's page to show.

import { SITE_URL } from '$lib/site';
import { isGazetteReservedSlug, isGazetteYearSlug, leafCoverUrl } from '$lib/gazette';
import type { GazetteLeaf } from '$lib/types/api';

export type SiteRoom = 'work' | 'tale' | 'leaf' | 'page';

export type SiteRef = {
  room: SiteRoom;
  /** Slug or id for a work, slug for a tale or leaf, the path for a page. */
  handle: string;
  /** Where the link goes: always relative, so it stays inside the SPA. */
  href: string;
  /** The address as a reader would read it, for a link nothing could name. */
  bare: string;
};

const HOST = new URL(SITE_URL).host.replace(/^www\./, '');
const HOST_RE = HOST.replace(/\./g, '\\.');

/**
 * Not preceded by a word, `@`, a dot or a slash: `mail@ritunia.com` is an
 * address, and `notritunia.com` is somebody else.
 */
export const SITE_LINK_RE = new RegExp(
  `(?<![\\w@./-])` +
    // A copy of the house running on this machine: the author copies the
    // address from the browser the tale is written in. Only with the scheme — the bare
    // word "localhost" in a sentence is a word.
    `(?:https?:\\/\\/(?:localhost|127\\.0\\.0\\.1)(?::\\d+)?|(?:https?:\\/\\/)?(?:www\\.)?${HOST_RE})` +
    `(?:\\/[^\\s<>"«»*]*)?`,
  'gi',
);

/** Punctuation that closes the sentence, not the address. */
const TRAILING = /[.,;:!?…)\]»"'’”]+$/;

/** Split a raw match into the address and the punctuation that followed it. */
export function trimLink(raw: string): { link: string; tail: string } {
  const tail = raw.match(TRAILING)?.[0] ?? '';
  return { link: tail ? raw.slice(0, -tail.length) : raw, tail };
}

export function parseSiteLink(link: string): SiteRef | null {
  let url: URL;
  try {
    url = new URL(/^https?:\/\//i.test(link) ? link : `https://${link}`);
  } catch {
    return null;
  }
  const path = url.pathname.replace(/\/+$/, '') || '/';
  const [room, handle, ...rest] = path.split('/').filter(Boolean);

  // The reader's own trip is counted like every other link out of a tale
  // (`workHref(leaf, 'tale')`) — unless the author already said where from.
  if (!url.searchParams.has('src')) url.searchParams.set('src', 'tale');
  const href = `${path}${url.search}${url.hash}`;
  const bare = `${HOST}${path === '/' ? '' : path}`;

  const one = handle && rest.length === 0 ? decodeURIComponent(handle) : '';
  if (room === 'figurines' && one) return { room: 'work', handle: one, href, bare };
  if (room === 'tales' && one) return { room: 'tale', handle: one, href, bare };
  if (room === 'gazette' && one && !isGazetteReservedSlug(one) && !isGazetteYearSlug(one)) {
    return { room: 'leaf', handle: one, href, bare };
  }
  // The bare domain names the house, it does not point into it: "выложила на
  // ritunia.com" stays the words the author wrote.
  if (path === '/') return null;
  return { room: 'page', handle: path, href, bare };
}

export function siteRefKey(ref: Pick<SiteRef, 'room' | 'handle'>): string {
  return `${ref.room}:${ref.handle}`;
}

/** Every link to this house in a piece of text, deduplicated by what it names. */
export function siteRefsIn(text: string | null | undefined): SiteRef[] {
  const out = new Map<string, SiteRef>();
  for (const m of (text ?? '').matchAll(SITE_LINK_RE)) {
    const ref = parseSiteLink(trimLink(m[0]).link);
    if (ref) out.set(siteRefKey(ref), ref);
  }
  return [...out.values()];
}

/**
 * What a link names, fetched once, when the tale is loaded.
 *
 * In the loader rather than on the page: the tale is prerendered, and a name
 * that arrives after mount would be a raw address in the HTML that search
 * engines read and a flicker for everyone else.
 */
export type SiteRefInfo =
  | { room: 'work'; id: string; name: string; dek: string; image: string }
  | {
      room: 'tale' | 'leaf';
      id: string;
      figurineId: string | null;
      leaf: Pick<GazetteLeaf, 'titleEn' | 'titleRu' | 'dekEn' | 'dekRu' | 'bodyEn' | 'bodyRu'>;
      image: string;
    };

type Fetchers = {
  getFigurine: (handle: string) => Promise<{
    id: string;
    name: string;
    shortText: string | null;
    images: { imageType: string; url: string }[];
  } | null>;
  getGazetteLeaf: (slug: string) => Promise<GazetteLeaf>;
};

/**
 * Names for every reference. A link that cannot be named — deleted work,
 * mistyped slug, the house unreachable — is simply absent from the map and
 * prints as the address it is. A tale is never refused over its links.
 */
export async function resolveSiteRefs(
  refs: SiteRef[],
  api: Fetchers,
): Promise<Record<string, SiteRefInfo>> {
  const out: Record<string, SiteRefInfo> = {};
  await Promise.all(
    refs.map(async (ref) => {
      try {
        if (ref.room === 'work') {
          const f = await api.getFigurine(ref.handle);
          if (!f) return;
          const face = f.images.find((i) => i.imageType === 'face') ?? f.images[0];
          out[siteRefKey(ref)] = {
            room: 'work',
            id: f.id,
            name: f.name,
            dek: (f.shortText ?? '').trim(),
            image: face?.url ?? '',
          };
        } else if (ref.room === 'tale' || ref.room === 'leaf') {
          const leaf = await api.getGazetteLeaf(ref.handle);
          out[siteRefKey(ref)] = {
            room: ref.room,
            id: leaf.id,
            figurineId: leaf.figurineId,
            leaf: {
              titleEn: leaf.titleEn,
              titleRu: leaf.titleRu,
              dekEn: leaf.dekEn,
              dekRu: leaf.dekRu,
              bodyEn: null,
              bodyRu: null,
            },
            image: leafCoverUrl(leaf),
          };
        }
      } catch {
        // Left unnamed on purpose; see above.
      }
    }),
  );
  return out;
}

/** The rooms that have a name of their own in the menu. */
export const PAGE_LABELS: Record<string, string> = {
  '/figurines': 'navArchive',
  '/tales': 'navTales',
  '/gazette': 'navGazette',
  '/author': 'navAuthor',
  '/workshop': 'navWorkshop',
  '/upcoming': 'navUpcoming',
  '/acquire': 'navAcquire',
  '/impressions': 'navImpressions',
  '/battles': 'navBattles',
  '/studio': 'navStudio',
};
