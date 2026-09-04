import { api } from '$lib/api';
import {
    SITE_URL,
    ACQUIRE_PATH,
    IMPRESSIONS_PATH,
    PRIVACY_PATH,
    IMAGE_RIGHTS_PATH,
} from '$lib/site';
import { leafCopy } from '$lib/gazette';
import type { FigurineListItem, GazetteLeaf } from '$lib/types/api';

// LLM-facing site index (https://llmstxt.org). Prerendered to a static /llms.txt in
// the web build so language-model crawlers get a clean, link-rich map of the archive
// without executing JS. The dev/SPA profile has no SEO surface, so it's excluded.
//
// THE ONLY /llms.txt in the house. There used to be a second, hand-written one at
// static/llms.txt — and it was the one that actually shipped: a static asset and a
// prerendered route both writing the same path is won by the asset, silently. So the
// live map below, with the works list, had never once reached the site, and the two
// files had to be kept in agreement by hand — badly, since only one was ever read.
// The static twin is gone; its `## Notes` moved down into this file. Do not add a
// second one back.
export const prerender = import.meta.env.VITE_BUILD_TARGET === 'web';

// Collapse whitespace/newlines so each work stays a single tidy bullet line.
function oneLine(s: string | null | undefined): string {
    return (s ?? '').replace(/\s+/g, ' ').trim();
}

export async function GET({ fetch }: { fetch: typeof globalThis.fetch }) {
    let figurines: FigurineListItem[] = [];
    try {
        const all = await api.getAllFigurines(undefined, fetch);
        figurines = all.filter((f) => f.status !== 'in_progress');
    } catch {
        figurines = [];
    }

    // The shelf of tall tales, listed the way the works are. A single link to
    // /tales tells a model that a room exists and nothing about what is in it;
    // this is the room's actual contents, in the keeper's own order, so an agent
    // answering "what does this figure remember?" can reach the right leaf in
    // one hop. Caught here rather than in api.getTales — a map missing one
    // section is still a map, whereas a prerendered EMPTY SHELF is a lie, which
    // is why the shelf's own load lets the failure through.
    let tales: GazetteLeaf[] = [];
    try {
        tales = await api.getTales(fetch);
    } catch {
        tales = [];
    }

    const taleLines = tales
        .map((tale) => {
            const copy = leafCopy(tale, 'en');
            const title = oneLine(copy.title);
            if (!title) return '';
            const line = `- [${title}](${SITE_URL}/tales/${tale.slug})`;
            const about = oneLine(tale.figurineName);
            const dek = oneLine(copy.dek);
            const tail = [dek, about ? `about: ${about}` : ''].filter(Boolean).join(' — ');
            return tail ? `${line}: ${tail}` : line;
        })
        .filter(Boolean)
        .join('\n');

    const works = figurines
        .map((f) => {
            // FigurineListItem carries no description, so synthesise a short descriptor
            // from the structured facets that are available.
            const facets = [f.series, f.technique, f.material, f.year ? String(f.year) : null]
                .map((v) => oneLine(v))
                .filter(Boolean)
                .join(', ');
            const line = `- [${oneLine(f.name)}](${SITE_URL}/figurines/${f.slug ?? f.id})`;
            return facets ? `${line}: ${facets}` : line;
        })
        .join('\n');

    const body =
        `# Ritunia\n\n` +
        `> One-of-a-kind gothic art dolls and miniature sculptures — sculpted, painted ` +
        `and finished entirely by hand by a single maker. Ritunia is a showcase archive, ` +
        `not a shop: each piece is documented like a museum specimen, with material, ` +
        `dimensions, year and provenance, and many of them have a tale of their own.\n\n` +
        `## About\n\n` +
        `- [The author](${SITE_URL}/author): who makes the figures and why.\n` +
        `- [The workshop](${SITE_URL}/workshop): tools, materials and process.\n` +
        `- [The cellar](${SITE_URL}/cellar): how this site itself is built — the five effects, the palette, the rules.\n` +
        `- [How to acquire a work](${SITE_URL}${ACQUIRE_PATH}): house rules for commissioning or reserving a piece. The petition form is a later step, not a public listing.\n` +
        `- [The photographs](${SITE_URL}${IMAGE_RIGHTS_PATH}): images are copyrighted; reuse needs written permission.\n` +
        `- [Privacy](${SITE_URL}${PRIVACY_PATH}): what personal data the site collects and why.\n\n` +
        `## Works\n\n` +
        (works || '- (the archive is currently quiet)') +
        `\n\n## Tall tales\n\n` +
        `Short stories about the works — what each figure remembers, told in its own ` +
        `voice. Each tale lives at /tales/{slug} and is linked from the page of the work ` +
        `it belongs to.\n\n` +
        (taleLines || '- (the shelf is currently bare)') +
        `\n\n## Browse\n\n` +
        `- [Full archive](${SITE_URL}/figurines): every available, reserved and sold piece.\n` +
        `- [Gazette](${SITE_URL}/gazette): notes from the house — arrivals, sketches, openings, cuttings.\n` +
        `- [Tall tales](${SITE_URL}/tales): short stories about the works — what each figure remembers, in its own words.\n` +
        `- [Gazette RSS](${SITE_URL}/gazette/feed.xml): machine-readable leaves of the cabinet.\n` +
        `- [Upcoming](${SITE_URL}/upcoming): figures currently in progress.\n` +
        `- [The shelf of cards](${SITE_URL}/battles): the same works seen from another side, as cards. Not a shop \u2014 nothing here is bought with money.\n` +
        `- [Book of Impressions](${SITE_URL}${IMPRESSIONS_PATH}): leave a quiet reaction to the exhibition.\n\n` +
        // Conventions a crawler cannot infer from the link list alone. Carried
        // over from the hand-written static/llms.txt this file replaced.
        `## Notes\n\n` +
        `- Every figurine has its own permanent page at /figurines/{slug} with a description, material, dimensions, year, and photography.\n` +
        `- Gazette leaves live at /gazette/{slug}. RSS: ${SITE_URL}/gazette/feed.xml\n` +
        `- A tale about a work lives at /tales/{slug}, and is linked from that work's own page. /gazette/{slug} redirects there for tales.\n` +
        `- Pieces are not sold off-the-shelf; acquisition is by commission or reservation, as described at ${ACQUIRE_PATH}.\n` +
        `- A card at /battles is one work seen from another side. The rooms behind it (a guest's own table, a match against the keeper) belong to one person and are deliberately not indexed.\n` +
        `- Full machine-readable list of figurine and gazette pages: ${SITE_URL}/sitemap.xml\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
}
