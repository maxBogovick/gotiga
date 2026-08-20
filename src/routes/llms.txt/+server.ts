import { api } from '$lib/api';
import {
    SITE_URL,
    ACQUIRE_PATH,
    IMPRESSIONS_PATH,
    PRIVACY_PATH,
    IMAGE_RIGHTS_PATH,
} from '$lib/site';
import type { FigurineListItem } from '$lib/types/api';

// LLM-facing site index (https://llmstxt.org). Prerendered to a static /llms.txt in
// the web build so language-model crawlers get a clean, link-rich map of the archive
// without executing JS. The dev/SPA profile has no SEO surface, so it's excluded.
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
        `> An author's cabinet of gothic, handmade miniatures and art dolls. ` +
        `Ritunia is a showcase archive — not a shop — where each piece is a unique, ` +
        `hand-sculpted and hand-painted figure with its own story.\n\n` +
        `## About\n\n` +
        `- [The author](${SITE_URL}/author): who makes the figures and why.\n` +
        `- [The workshop](${SITE_URL}/workshop): tools, materials and process.\n` +
        `- [How to acquire a work](${SITE_URL}${ACQUIRE_PATH}): house rules for commissioning or reserving a piece. The petition form is a later step, not a public listing.\n` +
        `- [The photographs](${SITE_URL}${IMAGE_RIGHTS_PATH}): images are copyrighted; reuse needs written permission.\n` +
        `- [Privacy](${SITE_URL}${PRIVACY_PATH}): what personal data the site collects and why.\n\n` +
        `## Works\n\n` +
        (works || '- (the archive is currently quiet)') +
        `\n\n## Browse\n\n` +
        `- [Full archive](${SITE_URL}/figurines): every available, reserved and sold piece.\n` +
        `- [Gazette](${SITE_URL}/gazette): notes from the house — arrivals, sketches, openings, cuttings.\n` +
        `- [Gazette RSS](${SITE_URL}/gazette/feed.xml): machine-readable leaves of the cabinet.\n` +
        `- [Upcoming](${SITE_URL}/upcoming): figures currently in progress.\n` +
        `- [Book of Impressions](${SITE_URL}${IMPRESSIONS_PATH}): leave a quiet reaction to the exhibition.\n`;

    return new Response(body, {
        headers: { 'Content-Type': 'text/plain; charset=utf-8' },
    });
}
