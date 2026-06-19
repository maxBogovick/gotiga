import { api } from '$lib/api';
import { SITE_URL } from '$lib/site';
import type { FigurineListItem } from '$lib/types/api';

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

export async function GET() {
    let figurines: FigurineListItem[] = [];
    try {
        const all = await api.getAllFigurines();
        // In-progress pieces have no public detail page worth indexing.
        figurines = all.filter((f) => f.status !== 'in_progress');
    } catch {
        figurines = [];
    }

    const entries = [
        ...STATIC_ROUTES.map((path) => ({ loc: `${SITE_URL}${path}`, lastmod: undefined as string | undefined })),
        ...figurines.map((f) => ({
            loc: `${SITE_URL}/figurines/${f.id}`,
            lastmod: isoDay(f.createdAt),
        })),
    ];

    const body =
        `<?xml version="1.0" encoding="UTF-8"?>\n` +
        `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n` +
        entries
            .map(({ loc, lastmod }) =>
                `  <url><loc>${xmlEscape(loc)}</loc>${lastmod ? `<lastmod>${lastmod}</lastmod>` : ''}</url>`
            )
            .join('\n') +
        `\n</urlset>\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'application/xml' },
    });
}
