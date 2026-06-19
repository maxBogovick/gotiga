// Public site origin used to build absolute URLs in SEO output (sitemap.xml, OG
// tags). Kept in sync with the `Sitemap:` line in static/robots.txt.
export const SITE_URL = 'https://ritunia.com';

/** Turn a media path (possibly relative) into an absolute URL under SITE_URL. */
export function toAbsoluteUrl(path: string | null | undefined): string | null {
    if (!path) return null;
    const value = path.trim();
    if (!value) return null;
    if (value.startsWith('http://') || value.startsWith('https://')) return value;
    return `${SITE_URL}${value.startsWith('/') ? '' : '/'}${value}`;
}
