import { api } from '$lib/api';
import { SITE_URL, toAbsoluteUrl } from '$lib/site';
import type { Figurine, FigurineListItem } from '$lib/types/api';

// Prerendered to a static sitemap.xml in the web build (matches the Sitemap line in
// robots.txt). The Tauri build has no server and no SEO surface, so it's excluded.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// Public, prerendered routes with real, indexable HTML. /admin is intentionally
// absent (also Disallowed in robots). /commission is omitted on purpose: it reads a
// ?source query param so it can't be prerendered, and listing a JS-only shell here
// would point crawlers at a thin page.
const STATIC_ROUTES = ['/', '/figurines', '/upcoming', '/workshop', '/author'];

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

export async function GET() {
    let figurines: FigurineListItem[] = [];
    const detailById = new Map<string, Figurine | null>();
    try {
        const all = await api.getAllFigurines();
        // In-progress pieces have no public detail page worth indexing.
        figurines = all.filter((f) => f.status !== 'in_progress');
        const details = await Promise.all(
            figurines.map((f) => api.getFigurine(f.id).catch(() => null))
        );
        details.forEach((detail, i) => detailById.set(figurines[i].id, detail));
    } catch {
        figurines = [];
    }

    const entries: { loc: string; lastmod?: string; image?: string | null }[] = [
        ...STATIC_ROUTES.map((path) => ({ loc: `${SITE_URL}${path}` })),
        ...figurines.map((f) => ({
            loc: `${SITE_URL}/figurines/${f.slug ?? f.id}`,
            lastmod: isoDay(f.createdAt),
            image: imageLoc(detailById.get(f.id), f.faceImageUrl),
        })),
    ];

    const body =
        `<?xml version="1.0" encoding="UTF-8"?>\n` +
        `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:image="http://www.google.com/schemas/sitemap-image/1.1">\n` +
        entries
            .map(({ loc, lastmod, image }) => {
                const imageXml = image ? `<image:image><image:loc>${xmlEscape(image)}</image:loc></image:image>` : '';
                return `  <url><loc>${xmlEscape(loc)}</loc>${lastmod ? `<lastmod>${lastmod}</lastmod>` : ''}${imageXml}</url>`;
            })
            .join('\n') +
        `\n</urlset>\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'application/xml' },
    });
}
