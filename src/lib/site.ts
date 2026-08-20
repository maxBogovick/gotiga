// Public site origin used to build absolute URLs in SEO output (sitemap.xml, OG
// tags). Kept in sync with the `Sitemap:` line in static/robots.txt.
export const SITE_URL = 'https://ritunia.com';

// Statement of rights for photographs. Figurine ImageObjects point `license` here
// and `acquireLicensePage` at the #request fragment — Google Images needs both
// URLs to be real, crawlable pages (a fabricated link is worse than omitting them).
export const IMAGE_RIGHTS_PATH = '/rights';
export const PRIVACY_PATH = '/privacy';
export const ACQUIRE_PATH = '/acquire';
export const IMPRESSIONS_PATH = '/impressions';

// Public, prerendered routes with real HTML. Listed in sitemap.xml and (the
// subset that belongs in a prose map) in llms.txt. /commission is omitted on
// purpose: it reads a ?source query param so it cannot be prerendered.
export const SITEMAP_STATIC_ROUTES = [
    '/',
    '/figurines',
    '/gazette',
    '/workshop',
    '/author',
    '/upcoming',
    ACQUIRE_PATH,
    IMPRESSIONS_PATH,
    PRIVACY_PATH,
    IMAGE_RIGHTS_PATH,
] as const;

/** Turn a media path (possibly relative) into an absolute URL under SITE_URL. */
export function toAbsoluteUrl(path: string | null | undefined): string | null {
    if (!path) return null;
    const value = path.trim();
    if (!value) return null;
    if (value.startsWith('http://') || value.startsWith('https://')) return value;
    return `${SITE_URL}${value.startsWith('/') ? '' : '/'}${value}`;
}
