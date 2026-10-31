import { api } from '$lib/api';
import { SITE_URL, SITEMAP_STATIC_ROUTES, toAbsoluteUrl } from '$lib/site';
import type { Figurine, FigurineListItem } from '$lib/types/api';
import { LANGS, LANG_META } from '$lib/i18n';
import { stripLang, withLang } from '$lib/i18n/path';

// Prerendered to a static sitemap.xml in the web build (matches the Sitemap line in
// robots.txt). The dev/SPA profile has no SEO surface, so it's excluded.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// /admin is intentionally absent (also Disallowed in robots). /commission is omitted
// on purpose: it reads a ?source query param so it can't be prerendered, and listing
// a JS-only shell here would point crawlers at a thin page. Acquisition is /acquire.

/** Каждая публичная страница есть на шести языках, у каждого языка свой адрес
 *  (`/ru/…`, английский без приставки); `x-default` — английский. Ссылки
 *  взаимные: каждая версия называет все шесть, включая себя. Адреса с `?lang=`
 *  сюда не идут: у них тот же HTML и canonical на английскую страницу. */
function alternates(path: string): string {
    const links = LANGS.map(
        (l) => `<xhtml:link rel="alternate" hreflang="${LANG_META[l].locale.split('-')[0]}" href="${xmlEscape(`${SITE_URL}${withLang(path, l)}`)}"/>`,
    );
    links.push(`<xhtml:link rel="alternate" hreflang="x-default" href="${xmlEscape(`${SITE_URL}${stripLang(path)}`)}"/>`);
    return links.join('');
}

function xmlEscape(s: string): string {
    return s.replace(/[<>&'"]/g, (c) =>
        ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', "'": '&apos;', '"': '&quot;' })[c] ?? c
    );
}

// Guard against a malformed createdAt — an Invalid Date would throw from
// toISOString() and crash the whole prerender build.
function isoDay(value: string | null | undefined): string | undefined {
    if (!value) return undefined;
    const t = new Date(value).getTime();
    return Number.isNaN(t) ? undefined : new Date(t).toISOString().slice(0, 10);
}

// <image:caption> is extra text Google's image sitemap extension accepts alongside
// <image:loc> — one more place to say what the photo depicts, on top of the page's own
// alt text/caption/JSON-LD. Title + material reads like a museum label ("Threshold
// Guardian — polymer clay, resin") without duplicating the alt-text formula verbatim.
function imageCaption(name: string, material: string | null | undefined): string {
    const detail = material?.trim();
    return detail ? `${name} — ${detail}` : name;
}

function imageLoc(figurine: Figurine | null | undefined, fallback: string | null | undefined): string | null {
    const image =
        figurine?.images?.find((i) => i.imageType === 'face')?.url ??
        figurine?.images?.[0]?.url ??
        fallback ??
        null;
    const absolute = toAbsoluteUrl(image);
    if (!absolute?.startsWith('http')) return absolute;
    try {
        const parsed = new URL(absolute);
        if (parsed.pathname.startsWith('/static/') || parsed.pathname.startsWith('/images/')) {
            return `${SITE_URL}${parsed.pathname}${parsed.search}`;
        }
    } catch {
        return absolute;
    }
    return absolute;
}

export async function GET({ fetch }: { fetch: typeof globalThis.fetch }) {
    let figurines: FigurineListItem[] = [];
    const detailById = new Map<string, Figurine | null>();
    try {
        const all = await api.getAllFigurines(undefined, fetch);
        // In-progress pieces have no public detail page worth indexing.
        figurines = all.filter((f) => f.status !== 'in_progress');
        const details = await Promise.all(
            figurines.map((f) => api.getFigurine(f.id, fetch).catch(() => null))
        );
        details.forEach((detail, i) => detailById.set(figurines[i].id, detail));
    } catch {
        figurines = [];
    }

    let gazetteLeaves: { slug: string; lastmod?: string }[] = [];
    let taleLeaves: { slug: string; lastmod?: string }[] = [];
    let gazetteYears: number[] = [];
    try {
        const [gaz, room] = await Promise.all([
            api.getGazettePage(1, 200, fetch),
            api.getGazetteRoom(undefined, fetch),
        ]);
        // A tale answers on /tales/<slug>; /gazette/<slug> only redirects there,
        // so listing the gazette address would point the crawler at a 308.
        gazetteLeaves = gaz.items
            .filter((leaf) => leaf.kind !== 'tale')
            .map((leaf) => ({
                slug: leaf.slug,
                lastmod: isoDay(leaf.publishedAt ?? leaf.createdAt),
            }));
        taleLeaves = gaz.items
            .filter((leaf) => leaf.kind === 'tale')
            .map((leaf) => ({
                slug: leaf.slug,
                lastmod: isoDay(leaf.publishedAt ?? leaf.createdAt),
            }));
        gazetteYears = room.years;
    } catch {
        gazetteLeaves = [];
        taleLeaves = [];
        gazetteYears = [];
    }

    const entries: { loc: string; lastmod?: string; image?: string | null; imageTitle?: string; imageCaption?: string }[] = [
        ...SITEMAP_STATIC_ROUTES.map((path) => ({ loc: `${SITE_URL}${path}` })),
        ...figurines.map((f) => ({
            loc: `${SITE_URL}/figurines/${f.slug ?? f.id}`,
            lastmod: isoDay(f.createdAt),
            image: imageLoc(detailById.get(f.id), f.faceImageUrl),
            imageTitle: f.name,
            imageCaption: imageCaption(f.name, detailById.get(f.id)?.material),
        })),
        ...gazetteLeaves.map((leaf) => ({
            loc: `${SITE_URL}/gazette/${leaf.slug}`,
            lastmod: leaf.lastmod,
        })),
        ...gazetteYears.map((year) => ({
            loc: `${SITE_URL}/gazette/${year}`,
        })),
        ...taleLeaves.map((tale) => ({
            loc: `${SITE_URL}/tales/${tale.slug}`,
            lastmod: tale.lastmod,
        })),
    ];

    const body =
        `<?xml version="1.0" encoding="UTF-8"?>\n` +
        `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n` +
        entries
            .flatMap((entry) => LANGS.map((l) => ({ ...entry, path: entry.loc.slice(SITE_URL.length) || '/', l })))
            .map(({ loc, lastmod, image, imageTitle, imageCaption, path, l }) => {
                const imageXml = image
                    ? `<image:image><image:loc>${xmlEscape(image)}</image:loc>` +
                      (imageTitle ? `<image:title>${xmlEscape(imageTitle)}</image:title>` : '') +
                      (imageCaption ? `<image:caption>${xmlEscape(imageCaption)}</image:caption>` : '') +
                      `</image:image>`
                    : '';
                const here = `${SITE_URL}${withLang(path, l)}`;
                return `  <url><loc>${xmlEscape(here)}</loc>${lastmod ? `<lastmod>${lastmod}</lastmod>` : ''}${alternates(path)}${imageXml}</url>`;
            })
            .join('\n') +
        `\n</urlset>\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'application/xml' },
    });
}
