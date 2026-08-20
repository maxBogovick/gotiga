import { api } from '$lib/api';
import { SITE_URL, toAbsoluteUrl } from '$lib/site';
import { leafCopy, leafCoverUrl } from '$lib/gazette';

export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// Static snapshot for the web build. Production nginx proxies /gazette/feed.xml
// to the live Rust handler so a newly laid leaf appears without a redeploy.

const CHANNEL_TITLE = 'Ritunia — leaves of the cabinet';
const CHANNEL_DESCRIPTION =
  'Notes the house has set down: arrivals, tales, openings, and cuttings pinned from farther away.';

const ILLEGAL_XML = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/g;

function xmlEscape(s: string): string {
  return s.replace(ILLEGAL_XML, '').replace(/[<>&'"]/g, (c) =>
    ({ '<': '&lt;', '>': '&gt;', '&': '&amp;', "'": '&apos;', '"': '&quot;' })[c] ?? c,
  );
}

function rfc822(value: string | null | undefined): string | undefined {
  if (!value) return undefined;
  const t = new Date(value).getTime();
  return Number.isNaN(t) ? undefined : new Date(t).toUTCString();
}

export async function GET({ fetch }: { fetch: typeof globalThis.fetch }) {
  let itemsXml = '';
  let lastBuild = new Date().toUTCString();
  try {
    const page = await api.getGazettePage(1, 200, fetch);
    const first = page.items[0];
    lastBuild = rfc822(first?.publishedAt ?? first?.createdAt) ?? lastBuild;
    itemsXml = page.items
      .map((leaf) => {
        const copy = leafCopy(leaf, 'en');
        const link = `${SITE_URL}/gazette/${leaf.slug}`;
        const pubDate = rfc822(leaf.publishedAt ?? leaf.createdAt);
        const image = toAbsoluteUrl(leafCoverUrl(leaf) || null);
        return (
          `    <item>\n` +
          `      <title>${xmlEscape(copy.title)}</title>\n` +
          `      <link>${xmlEscape(link)}</link>\n` +
          `      <guid isPermaLink="true">${xmlEscape(link)}</guid>\n` +
          (pubDate ? `      <pubDate>${pubDate}</pubDate>\n` : '') +
          `      <description>${xmlEscape(copy.dek || copy.title)}</description>\n` +
          (image
            ? `      <enclosure url="${xmlEscape(image)}" type="image/jpeg" length="0" />\n`
            : '') +
          `    </item>`
        );
      })
      .join('\n');
  } catch {
    itemsXml = '';
  }

  const body =
    `<?xml version="1.0" encoding="UTF-8"?>\n` +
    `<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">\n` +
    `  <channel>\n` +
    `    <title>${xmlEscape(CHANNEL_TITLE)}</title>\n` +
    `    <link>${SITE_URL}/gazette</link>\n` +
    `    <atom:link href="${SITE_URL}/gazette/feed.xml" rel="self" type="application/rss+xml" />\n` +
    `    <description>${xmlEscape(CHANNEL_DESCRIPTION)}</description>\n` +
    `    <language>en</language>\n` +
    `    <lastBuildDate>${lastBuild}</lastBuildDate>\n` +
    `${itemsXml}\n` +
    `  </channel>\n` +
    `</rss>\n`;

  return new Response(body, {
    headers: { 'Content-Type': 'application/rss+xml; charset=utf-8' },
  });
}
